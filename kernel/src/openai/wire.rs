//! The wire history of an OpenAI session (TECHNICAL 9.5, 9.1 `wire/`; S40-1).
//!
//! Pure code over `State`: the items a session sends to the Responses API live in `wire/<id>/<n>`
//! chunks (`n` is the wire index of the chunk's first item), are repaired before each request,
//! trimmed by the body budget, and cut at the newest compaction item or summary (`active`).

use crate::error::KernelError;
use crate::state::State;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A body over this gets a new trim mark (9.5, body budget).
pub const BODY_TRIM_AT: usize = 896 * 1024;
/// Trimming goes on until a body is at most this.
pub const BODY_TRIM_TO: usize = 768 * 1024;
/// A body still over this after trimming is refused.
pub const BODY_MAX: usize = 1024 * 1024;
/// The most `compact_threshold` the kernel asks for, in tokens.
pub const COMPACT_THRESHOLD: u64 = 200_000;

/// A chunk holds at most this many bytes of items, or exactly one larger item.
const CHUNK_MAX: usize = 64 * 1024;
/// A tool output is cut to this many bytes when trimmed.
const OUTPUT_CUT: usize = 8 * 1024;
/// The trim mark lies at the first item of this many last steps.
const KEEP_STEPS: usize = 4;

/// One stored wire item: its index, the step that made it (0 for an input), the model that made it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireItem {
    pub i: u64,
    pub step: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub item: Value,
}

/// How far trimming went: `Reasoning` drops reasoning items, `Outputs` also cuts tool outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrimStage {
    Reasoning,
    Outputs,
}

/// The trim mark and how far trimming went. Items with a wire index below `mark` are trimmed;
/// of those, the ones below `through` (`through <= mark`) get the stage: in `Reasoning`, reasoning
/// items are left out; in `Outputs`, all reasoning before the mark is left out and tool outputs
/// are cut. Items from `through` to `mark` are sent whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Trim {
    pub mark: u64,
    pub through: u64,
    pub stage: TrimStage,
}

/// The wire part of `loop/<id>`: S40-3 embeds it in its loop state with `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Wire {
    pub next: u64,
    pub active: u64,
    pub wire_chunks: Vec<u64>,
    pub steps: u64,
    pub trim: Option<Trim>,
}

pub struct BodyArgs<'a> {
    pub session: &'a str,
    pub model: &'a str,
    pub instructions: &'a str,
    pub tools: &'a [Value],
    pub reasoning: Option<&'a Value>,
    pub compact_threshold: u64,
}

/// A body over `BODY_MAX` after trimming everything before the mark; carries its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyTooLarge(pub usize);

fn key(id: &str, first: u64) -> String {
    format!("wire/{id}/{first}")
}

fn is_type(item: &Value, t: &str) -> bool {
    item.get("type").and_then(Value::as_str) == Some(t)
}

fn read_chunk(state: &mut impl State, id: &str, first: u64) -> Vec<WireItem> {
    state
        .get(&key(id, first))
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn size_of(w: &WireItem) -> usize {
    serde_json::to_string(w).map_or(0, |s| s.len())
}

impl Wire {
    /// Appends items made by `step` (0: an input) with `model`; a compaction item moves `active` (and deletes chunks wholly before it).
    pub fn append(
        &mut self,
        state: &mut impl State,
        id: &str,
        step: u64,
        model: Option<&str>,
        items: Vec<Value>,
    ) -> Result<(), KernelError> {
        self.append_inner(state, id, step, model, items, false)
    }

    fn append_inner(
        &mut self,
        state: &mut impl State,
        id: &str,
        step: u64,
        model: Option<&str>,
        items: Vec<Value>,
        restart: bool,
    ) -> Result<(), KernelError> {
        if items.is_empty() {
            return Ok(());
        }
        let mut chunks = self.wire_chunks.clone();
        let mut next = self.next;
        let mut cur_first = chunks.last().copied().unwrap_or(next);
        let mut cur = match chunks.last() {
            Some(&f) => read_chunk(state, id, f),
            None => Vec::new(),
        };
        let mut cur_size: usize = cur.iter().map(size_of).sum();
        let mut writes: Vec<(u64, Vec<WireItem>)> = Vec::new();
        let mut new_active = None;
        for item in items {
            let w = WireItem {
                i: next,
                step,
                model: model.map(str::to_string),
                item,
            };
            next += 1;
            if restart || is_type(&w.item, "compaction") {
                new_active = Some(w.i);
            }
            let sz = size_of(&w);
            if !cur.is_empty() && cur_size + sz > CHUNK_MAX {
                writes.push((cur_first, std::mem::take(&mut cur)));
                cur_size = 0;
            }
            if cur.is_empty() {
                cur_first = w.i;
                if chunks.last() != Some(&w.i) && !chunks.contains(&w.i) {
                    chunks.push(w.i);
                }
            }
            cur_size += sz;
            cur.push(w);
        }
        writes.push((cur_first, cur));
        for (first, chunk) in &writes {
            let bytes = serde_json::to_vec(chunk).unwrap_or_default();
            state
                .put(&key(id, *first), bytes)
                .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        }
        self.next = next;
        self.wire_chunks = chunks;
        if let Some(a) = new_active {
            self.active = a;
            self.trim = None;
            // Chunks wholly before `active` hold nothing a request sends.
            while self.wire_chunks.len() > 1 && self.wire_chunks[1] <= a {
                let first = self.wire_chunks.remove(0);
                state.delete(&key(id, first));
            }
        }
        Ok(())
    }

    /// The stored items from `active` on, reading no chunk before the one holding it.
    pub fn active_items(&self, state: &mut impl State, id: &str) -> Vec<WireItem> {
        let start = self
            .wire_chunks
            .iter()
            .rposition(|&f| f <= self.active)
            .unwrap_or(0);
        let mut out = Vec::new();
        for &first in self.wire_chunks.iter().skip(start) {
            out.extend(
                read_chunk(state, id, first)
                    .into_iter()
                    .filter(|w| w.i >= self.active),
            );
        }
        out
    }

    /// Appends one user message with `summary` and moves `active` to it (9.5: `failed {too-large}`, resume-lost).
    pub fn restart_from_summary(
        &mut self,
        state: &mut impl State,
        id: &str,
        summary: &str,
    ) -> Result<(), KernelError> {
        let step = self.steps;
        self.append_inner(state, id, step, None, vec![user_message(summary)], true)
    }

    pub fn delete_all(&mut self, state: &mut impl State, id: &str) {
        for &first in &self.wire_chunks {
            state.delete(&key(id, first));
        }
        *self = Wire::default();
    }

    /// Repair, trim by the budget (updating `self.trim`), and serialise the request body.
    pub fn body(&mut self, args: &BodyArgs, items: &[WireItem]) -> Result<String, BodyTooLarge> {
        let first = items.first().map_or(0, |w| w.i);
        if self.trim.as_ref().is_some_and(|t| t.mark < first) {
            self.trim = None;
        }
        let full = build(args, items, self.trim.as_ref());
        if full.len() <= BODY_TRIM_AT {
            return Ok(full);
        }
        let mark = trim_mark(items);
        let trimmed = |stage, through| {
            let t = Trim {
                mark,
                through,
                stage,
            };
            build(args, items, Some(&t)).len()
        };
        // The smallest `through` in 0..=mark for which the body fits, if any.
        let search = |stage| {
            if trimmed(stage, mark) > BODY_TRIM_TO {
                return None;
            }
            let (mut lo, mut hi) = (0, mark);
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if trimmed(stage, mid) <= BODY_TRIM_TO {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            Some(lo)
        };
        let (stage, through) = if let Some(t) = search(TrimStage::Reasoning) {
            (TrimStage::Reasoning, t)
        } else if let Some(t) = search(TrimStage::Outputs) {
            (TrimStage::Outputs, t)
        } else {
            (TrimStage::Outputs, mark)
        };
        let trim = Trim {
            mark,
            through,
            stage,
        };
        let body = build(args, items, Some(&trim));
        self.trim = Some(trim);
        if body.len() > BODY_MAX {
            return Err(BodyTooLarge(body.len()));
        }
        Ok(body)
    }
}

/// The wire index of the first item of the last `KEEP_STEPS` steps present.
fn trim_mark(items: &[WireItem]) -> u64 {
    let mut steps: Vec<u64> = items.iter().map(|w| w.step).collect();
    steps.sort_unstable();
    steps.dedup();
    let from = steps.len().saturating_sub(KEEP_STEPS);
    let lowest = steps.get(from).copied().unwrap_or(0);
    items.iter().find(|w| w.step >= lowest).map_or(0, |w| w.i)
}

/// The items with the trim applied: the same input always gives the same output, so a later
/// request extends an earlier one.
fn apply_trim(items: &[WireItem], trim: &Trim) -> Vec<WireItem> {
    let mut out = Vec::with_capacity(items.len());
    for w in items {
        if w.i >= trim.through {
            out.push(w.clone());
            continue;
        }
        if is_type(&w.item, "reasoning") {
            continue;
        }
        let mut w = w.clone();
        if trim.stage == TrimStage::Outputs && is_type(&w.item, "function_call_output") {
            if let Some(Value::String(s)) = w.item.get_mut("output") {
                if s.len() > OUTPUT_CUT {
                    let n = s.len();
                    let mut end = OUTPUT_CUT;
                    while !s.is_char_boundary(end) {
                        end -= 1;
                    }
                    s.truncate(end);
                    s.push_str(&format!(" [cut by watane: {n} bytes]"));
                }
            }
        }
        out.push(w);
    }
    out
}

fn build(args: &BodyArgs, items: &[WireItem], trim: Option<&Trim>) -> String {
    let input = match trim {
        Some(t) => repair(&apply_trim(items, t), args.model),
        None => repair(items, args.model),
    };
    let s = |v: &str| serde_json::to_string(v).unwrap_or_default();
    let j = |v: &Value| serde_json::to_string(v).unwrap_or_default();
    let mut b = format!(
        "{{\"model\":{},\"instructions\":{},\"input\":{}",
        s(args.model),
        s(args.instructions),
        j(&Value::Array(input))
    );
    if !args.tools.is_empty() {
        b.push_str(",\"tools\":");
        b.push_str(&j(&Value::Array(args.tools.to_vec())));
        b.push_str(",\"tool_choice\":\"auto\",\"parallel_tool_calls\":true");
    }
    if let Some(r) = args.reasoning {
        b.push_str(",\"reasoning\":");
        b.push_str(&j(r));
    }
    let threshold = args.compact_threshold.min(COMPACT_THRESHOLD);
    b.push_str(&format!(
        ",\"include\":[\"reasoning.encrypted_content\"],\"store\":false,\"stream\":true,\"prompt_cache_key\":{},\"context_management\":[{{\"type\":\"compaction\",\"compact_threshold\":{threshold}}}]}}",
        s(args.session)
    ));
    b
}

/// The items a request sends: a call with no output gets `aborted` right after the last call of
/// its step, an output with no call is dropped, call order is kept; reasoning is kept only with
/// `encrypted_content` and made by `model`, and is sent without its `id`.
pub fn repair(items: &[WireItem], model: &str) -> Vec<Value> {
    use std::collections::HashSet;
    let calls: HashSet<&str> = items
        .iter()
        .filter(|w| is_type(&w.item, "function_call"))
        .filter_map(|w| w.item.get("call_id").and_then(Value::as_str))
        .collect();
    let answered: HashSet<&str> = items
        .iter()
        .filter(|w| is_type(&w.item, "function_call_output"))
        .filter_map(|w| w.item.get("call_id").and_then(Value::as_str))
        .collect();
    // The position of the last call of each step.
    let last_call = |step: u64| {
        items
            .iter()
            .rposition(|w| w.step == step && is_type(&w.item, "function_call"))
    };
    let mut out = Vec::with_capacity(items.len());
    for (n, w) in items.iter().enumerate() {
        let item = &w.item;
        if is_type(item, "function_call_output") {
            let id = item.get("call_id").and_then(Value::as_str);
            if !id.is_some_and(|c| calls.contains(c)) {
                continue;
            }
        } else if is_type(item, "reasoning") {
            let enc = item
                .get("encrypted_content")
                .and_then(Value::as_str)
                .is_some_and(|e| !e.is_empty());
            if !enc || w.model.as_deref() != Some(model) {
                continue;
            }
            let mut item = item.clone();
            if let Some(o) = item.as_object_mut() {
                o.remove("id");
            }
            out.push(item);
            continue;
        }
        out.push(item.clone());
        if is_type(item, "function_call") && last_call(w.step) == Some(n) {
            // After the last call of the step: an aborted output for each open call of it.
            for c in items
                .iter()
                .filter(|c| c.step == w.step && is_type(&c.item, "function_call"))
            {
                if let Some(cid) = c.item.get("call_id").and_then(Value::as_str) {
                    if !answered.contains(cid) {
                        out.push(
                            json!({"type":"function_call_output","call_id":cid,"output":"aborted"}),
                        );
                    }
                }
            }
        }
    }
    out
}

pub fn user_message(text: &str) -> Value {
    json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]})
}
