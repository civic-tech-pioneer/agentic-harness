//! A session's transcript in state chunks: TECHNICAL 9.1 (`entries/<id>/<n>`), 9.4 (entries, `part`, `read`, replay).

use crate::adapters::Entry;
use crate::error::{KernelError, Refusal};
use crate::state::State;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CHUNK_BYTES: usize = 64 * 1024;
pub const CHUNK_ENTRIES: usize = 256;
pub const ENTRY_BYTES: usize = 256 * 1024;
pub const PAGE_BYTES: usize = 2 * 1024 * 1024;
pub const PAGE_ENTRIES: u32 = 256;
pub const REPLAY_ENTRIES: u64 = 128;
pub const MAX_CHUNKS: usize = 16_384;

const KINDS: [&str; 9] = [
    "message",
    "tool",
    "ask",
    "allow",
    "verdict",
    "turn",
    "compaction",
    "unknown",
    "part",
];

/// The kinds `append` takes: whole entries (9.4); `part` is made by `append` itself.
const APPEND_KINDS: [&str; 8] = [
    "message",
    "tool",
    "ask",
    "allow",
    "verdict",
    "turn",
    "compaction",
    "unknown",
];

fn refused(field: &str, reason: &str) -> KernelError {
    KernelError::Refused(vec![Refusal {
        field: field.to_string(),
        backend: String::new(),
        model: String::new(),
        reason: reason.to_string(),
    }])
}

/// The transcript fields of the session record (9.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Meta {
    /// The first index of each chunk, ascending.
    pub chunks: Vec<u64>,
    /// The high-water index.
    pub last: u64,
}

/// An entry to append: the parts of 9.4 without `v`, `i`, `at`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewEntry {
    pub kind: String,
    pub uuid: Option<String>,
    pub body: Value,
}

impl From<Entry> for NewEntry {
    fn from(e: Entry) -> Self {
        let kind = match serde_json::to_value(e.kind) {
            Ok(Value::String(s)) => s,
            _ => "unknown".to_string(),
        };
        NewEntry {
            kind,
            uuid: e.uuid,
            body: e.body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<String>,
    pub last: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Replay {
    Entries(Vec<String>),
    Behind { through: u64 },
}

/// The key of the chunk at position `n` of `meta.chunks`.
pub fn chunk_key(session: &str, n: usize) -> String {
    format!("entries/{session}/{n}")
}

type Cache = Option<(usize, Vec<String>)>;

pub struct Transcript<'a, S: State> {
    state: &'a mut S,
    session: String,
    meta: &'a mut Meta,
}

impl<'a, S: State> Transcript<'a, S> {
    pub fn new(state: &'a mut S, session: &str, meta: &'a mut Meta) -> Self {
        Transcript {
            state,
            session: session.to_string(),
            meta,
        }
    }

    /// Stores the entry; returns the index of the whole entry (its last part's for a large one).
    pub fn append(&mut self, entry: &NewEntry, at: u64) -> Result<u64, KernelError> {
        if !APPEND_KINDS.contains(&entry.kind.as_str()) {
            return Err(refused(
                "kind",
                "kind must be one of message, tool, ask, allow, verdict, turn, compaction, unknown",
            ));
        }
        let first = self.meta.last + 1;
        let (lines, last) = build_lines(entry, at, first);
        let mut chunks = self.meta.chunks.clone();
        let mut writes: Vec<(usize, String)> = Vec::new();
        // The chunk being filled: (position, value, entry count).
        let mut cur: Option<(usize, String, usize)> = None;
        let mut loaded = false;
        for (k, line) in lines.iter().enumerate() {
            let i = first + k as u64;
            if cur.is_none() && !loaded && line.len() <= CHUNK_BYTES && !chunks.is_empty() {
                loaded = true;
                let pos = chunks.len() - 1;
                if let Some(v) = self.state.get(&chunk_key(&self.session, pos)) {
                    let v = String::from_utf8_lossy(&v).into_owned();
                    let count = v.matches('\n').count() + 1;
                    cur = Some((pos, v, count));
                }
            }
            let fits = match &cur {
                Some((_, v, count)) => {
                    line.len() <= CHUNK_BYTES
                        && *count < CHUNK_ENTRIES
                        && (*count > 1 || v.len() <= CHUNK_BYTES)
                        && v.len() + 1 + line.len() <= CHUNK_BYTES
                }
                None => false,
            };
            if fits {
                if let Some((_, v, count)) = cur.as_mut() {
                    v.push('\n');
                    v.push_str(line);
                    *count += 1;
                }
            } else {
                if let Some((pos, v, _)) = cur.take() {
                    writes.push((pos, v));
                }
                if chunks.len() >= MAX_CHUNKS {
                    return Err(KernelError::Quota);
                }
                chunks.push(i);
                cur = Some((chunks.len() - 1, line.clone(), 1));
            }
        }
        if let Some((pos, v, _)) = cur {
            writes.push((pos, v));
        }
        for (pos, v) in writes {
            self.state
                .put(&chunk_key(&self.session, pos), v.into_bytes())
                .map_err(|e| {
                    KernelError::Backend(format!("state refused a chunk: {} bytes", e.0))
                })?;
        }
        self.meta.chunks = chunks;
        self.meta.last = last;
        Ok(last)
    }

    /// A page of entries, ascending (9.4 `read`).
    pub fn read(&mut self, after: u64, before: Option<u64>, max: u32) -> Result<Page, KernelError> {
        if max == 0 {
            return Err(refused("max", "max must be at least 1"));
        }
        let max = max.min(PAGE_ENTRIES) as usize;
        let last = self.meta.last;
        let mut cache: Cache = None;
        let hi = match before {
            Some(b) => b.saturating_sub(1).min(last),
            None => last,
        };
        let lo = after.saturating_add(1);
        let mut out: Vec<String> = Vec::new();
        if lo > hi || self.meta.chunks.is_empty() {
            return Ok(Page { entries: out, last });
        }
        let mut bytes = 0usize;
        if before.is_none() {
            let mut c = self.chunk_of(lo);
            'fwd: while c < self.meta.chunks.len() {
                let first = self.meta.chunks[c];
                self.load(c, &mut cache);
                let lines = cache.as_ref().map(|x| x.1.as_slice()).unwrap_or(&[]);
                for (j, line) in lines.iter().enumerate() {
                    let i = first + j as u64;
                    if i < lo {
                        continue;
                    }
                    if i > hi {
                        break 'fwd;
                    }
                    let s = render(line, i);
                    if !out.is_empty() && bytes + s.len() > PAGE_BYTES {
                        break 'fwd;
                    }
                    bytes += s.len();
                    out.push(s);
                    if out.len() == max {
                        break 'fwd;
                    }
                }
                c += 1;
            }
        } else {
            let mut c = self.chunk_of(hi);
            'back: loop {
                let first = self.meta.chunks[c];
                self.load(c, &mut cache);
                let lines = cache.as_ref().map(|x| x.1.as_slice()).unwrap_or(&[]);
                for (j, line) in lines.iter().enumerate().rev() {
                    let i = first + j as u64;
                    if i > hi {
                        continue;
                    }
                    if i < lo {
                        break 'back;
                    }
                    let s = render(line, i);
                    if !out.is_empty() && bytes + s.len() > PAGE_BYTES {
                        break 'back;
                    }
                    bytes += s.len();
                    out.push(s);
                    if out.len() == max {
                        break 'back;
                    }
                }
                if c == 0 {
                    break;
                }
                c -= 1;
            }
            out.reverse();
        }
        Ok(Page { entries: out, last })
    }

    /// The replay decision of `attach` (9.4): the entries after `after`, or `Behind`.
    pub fn replay(&mut self, after: u64) -> Replay {
        let last = self.meta.last;
        let behind = Replay::Behind { through: last };
        if after >= last {
            return Replay::Entries(Vec::new());
        }
        if last - after > REPLAY_ENTRIES {
            return behind;
        }
        let mut cache: Cache = None;
        let after = self.adjust(after, &mut cache);
        let want = last - after;
        if want > REPLAY_ENTRIES {
            return behind;
        }
        match self.read(after, None, REPLAY_ENTRIES as u32) {
            Ok(page) if page.entries.len() as u64 == want => Replay::Entries(page.entries),
            _ => behind,
        }
    }

    /// For `replay` only: moves `after` back to before the first part of the group it falls in.
    /// `read` takes `after` as given, so forward paging always makes progress (9.4).
    fn adjust(&mut self, after: u64, cache: &mut Cache) -> u64 {
        if after == 0 || after >= self.meta.last || self.meta.chunks.is_empty() {
            return after;
        }
        let c = self.chunk_of(after);
        self.load(c, cache);
        let j = (after - self.meta.chunks[c]) as usize;
        let line = cache.as_ref().and_then(|x| x.1.get(j));
        let Some(Value::Object(o)) = line.and_then(|l| serde_json::from_str(l).ok()) else {
            return after;
        };
        if o.get("kind").and_then(Value::as_str) != Some("part") {
            return after;
        }
        let n = o
            .get("body")
            .and_then(|b| b.get("n"))
            .and_then(Value::as_u64);
        let m = o
            .get("body")
            .and_then(|b| b.get("m"))
            .and_then(Value::as_u64);
        match (n, m) {
            (Some(n), Some(m)) if n < m && n <= after => after - n,
            _ => after,
        }
    }

    /// The position in `meta.chunks` of the chunk that holds index `i`.
    fn chunk_of(&self, i: u64) -> usize {
        self.meta
            .chunks
            .partition_point(|&f| f <= i)
            .saturating_sub(1)
    }

    fn load(&mut self, c: usize, cache: &mut Cache) {
        if matches!(cache, Some((n, _)) if *n == c) {
            return;
        }
        let lines = match self.state.get(&chunk_key(&self.session, c)) {
            Some(v) => {
                let s = String::from_utf8_lossy(&v).into_owned();
                if s.is_empty() {
                    Vec::new()
                } else {
                    s.split('\n').map(str::to_string).collect()
                }
            }
            None => Vec::new(),
        };
        *cache = Some((c, lines));
    }
}

/// A stored line as a reader gets it: itself, or `unreadable` (9.4).
fn render(line: &str, pos: u64) -> String {
    let parsed: Option<Value> = serde_json::from_str(line).ok();
    if let Some(Value::Object(o)) = &parsed {
        let v_ok = o.get("v").and_then(Value::as_u64).is_some_and(|v| v <= 1);
        let kind_ok = o
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|k| KINDS.contains(&k));
        if v_ok && kind_ok {
            return line.to_string();
        }
    }
    let v = parsed
        .as_ref()
        .and_then(|p| p.get("v"))
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let at = match parsed
        .as_ref()
        .and_then(|p| p.get("at"))
        .and_then(Value::as_u64)
    {
        Some(a) => format!(r#","at":{a}"#),
        None => String::new(),
    };
    let raw = serde_json::to_string(line).unwrap_or_else(|_| "\"\"".to_string());
    format!(r#"{{"v":{v},"i":{pos},"kind":"unreadable"{at},"raw":{raw}}}"#)
}

fn entry_line(i: u64, e: &NewEntry, at: u64) -> String {
    let kind = serde_json::to_string(&e.kind).unwrap_or_default();
    let uuid = match &e.uuid {
        Some(u) => format!(
            r#","uuid":{}"#,
            serde_json::to_string(u).unwrap_or_default()
        ),
        None => String::new(),
    };
    let body = serde_json::to_string(&e.body).unwrap_or_else(|_| "null".to_string());
    format!(r#"{{"v":1,"i":{i},"kind":{kind},"at":{at}{uuid},"body":{body}}}"#)
}

fn escaped_len(c: char) -> usize {
    match c {
        '"' | '\\' | '\n' | '\r' | '\t' | '\u{8}' | '\u{c}' => 2,
        c if (c as u32) < 0x20 => 6,
        c => c.len_utf8(),
    }
}

/// The stored lines of an entry first indexed `first`, and the index of the whole entry.
fn build_lines(e: &NewEntry, at: u64, first: u64) -> (Vec<String>, u64) {
    let whole = entry_line(first, e, at);
    if whole.len() <= ENTRY_BYTES {
        return (vec![whole], first);
    }
    let part = |i: u64, n: usize, m: usize, text: &str| {
        let text = serde_json::to_string(text).unwrap_or_default();
        format!(
            r#"{{"v":1,"i":{i},"kind":"part","at":{at},"body":{{"n":{n},"m":{m},"text":{text}}}}}"#
        )
    };
    let overhead = part(u64::MAX, usize::MAX, usize::MAX, "").len();
    let budget = ENTRY_BYTES - overhead;
    // The part count grows with the digits of the last index; it only grows, so this ends.
    let mut m = 1usize;
    loop {
        let last = first + m as u64 - 1;
        let whole = entry_line(last, e, at);
        let mut slices: Vec<&str> = Vec::new();
        let (mut start, mut used) = (0usize, 0usize);
        for (pos, c) in whole.char_indices() {
            let w = escaped_len(c);
            if used + w > budget {
                slices.push(&whole[start..pos]);
                start = pos;
                used = 0;
            }
            used += w;
        }
        slices.push(&whole[start..]);
        if slices.len() == m {
            let count = slices.len();
            let lines = slices
                .iter()
                .enumerate()
                .map(|(k, t)| part(first + k as u64, k + 1, count, t))
                .collect();
            return (lines, first + count as u64 - 1);
        }
        m = slices.len();
    }
}

/// Joins the parts of 9.4 back into whole entries.
#[derive(Debug, Default)]
pub struct Joiner {
    /// The open group: the next `n` expected, `m`, the text so far.
    open: Option<(u64, u64, String)>,
}

impl Joiner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one entry string; yields a whole entry when one completes.
    pub fn push(&mut self, entry: &str) -> Option<Value> {
        let v: Value = match serde_json::from_str(entry) {
            Ok(v) => v,
            Err(_) => {
                self.open = None;
                return None;
            }
        };
        if v.get("kind").and_then(Value::as_str) != Some("part") {
            self.open = None;
            return Some(v);
        }
        let body = v.get("body");
        let n = body.and_then(|b| b.get("n")).and_then(Value::as_u64);
        let m = body.and_then(|b| b.get("m")).and_then(Value::as_u64);
        let text = body.and_then(|b| b.get("text")).and_then(Value::as_str);
        let (Some(n), Some(m), Some(text)) = (n, m, text) else {
            self.open = None;
            return None;
        };
        let mut group = match self.open.take() {
            Some((next, gm, acc)) if n == next && m == gm => (next, gm, acc),
            _ if n == 1 => (1, m, String::new()),
            _ => return None,
        };
        group.2.push_str(text);
        if n == m {
            return serde_json::from_str(&group.2).ok();
        }
        self.open = Some((n + 1, m, group.2));
        None
    }
}
