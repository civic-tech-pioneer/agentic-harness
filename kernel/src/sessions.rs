//! The session calls of TECHNICAL 2.3 over host state: `start`, `say`, `answer`, the queue, `read`,
//! `rename`, `delete`, `list`, and the provider's events (9.3, 9.4). Restart is not here (R1-R6).
//!
//! Every call checks all its refusals before its first write (7, "A call commits whole"), keeps
//! a client key only when the call changed something (9.1), and reads and writes only what its
//! inputs name: no listing of keys.

use crate::active::{self, Slots};
use crate::adapters::{self, Kind};
use crate::check::check;
use crate::error::{KernelError, Refusal};
use crate::profile::{parse_profile, Backend};
use crate::queue::{check_input, Queue, Queued};
use crate::record::{
    check_key, check_title, OpenAsk, PendingAnswer, Record, Stop, Turn, ALLOW_BYTES, ID_BYTES,
    OPEN_ASKS,
};
use crate::state::State;
use crate::transcript::{Joiner, NewEntry, Page, Transcript, MAX_CHUNKS};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The kernel's own timer for the restart pass (R2).
const RESTART: &str = "restart";

/// What the kernel needs of the host (2.1): the boot number, the clock, ids, timers, requests.
pub trait Host {
    /// `caller.boot()` (2.1).
    fn boot(&mut self) -> u64;
    /// Milliseconds.
    fn now(&mut self) -> u64;
    /// A fresh session id, at most 64 bytes.
    fn new_id(&mut self) -> String;
    fn timer_after(&mut self, id: &str, delay_ms: u64);
    fn timer_cancel(&mut self, id: &str);
    /// What goes to the provider: recorded, never run.
    fn request(&mut self, r: Request);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Turn {
        session: String,
        input: String,
        resume: bool,
    },
    /// The owner's `interrupt`: the host stops the turn the provider runs.
    Interrupt { session: String },
    /// The owner's `cancel`: the host ends the session's stream and handle.
    Cancel { session: String },
    Answer {
        session: String,
        ask: String,
        decision: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostCall {
    Boot,
    TimerAfter { id: String, delay_ms: u64 },
    TimerCancel { id: String },
    Request(Request),
}

/// A host for tests: a settable clock and boot, ids `s1`, `s2`, ..., and a log of what was asked.
#[derive(Debug, Default)]
pub struct FakeHost {
    pub boot: u64,
    pub now: u64,
    pub next_id: u64,
    pub calls: Vec<HostCall>,
}

impl FakeHost {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Host for FakeHost {
    fn boot(&mut self) -> u64 {
        self.calls.push(HostCall::Boot);
        self.boot
    }
    fn now(&mut self) -> u64 {
        self.now
    }
    fn new_id(&mut self) -> String {
        self.next_id += 1;
        format!("s{}", self.next_id)
    }
    fn timer_after(&mut self, id: &str, delay_ms: u64) {
        self.calls.push(HostCall::TimerAfter {
            id: id.to_string(),
            delay_ms,
        });
    }
    fn timer_cancel(&mut self, id: &str) {
        self.calls
            .push(HostCall::TimerCancel { id: id.to_string() });
    }
    fn request(&mut self, r: Request) {
        self.calls.push(HostCall::Request(r));
    }
}

/// What a provider reports for a session (an item call on its stream).
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // the interface is fixed: `Out` is not boxed
pub enum ProviderEvent {
    Out(adapters::Out),
    Turn(Turn),
    /// The provider's stream ended: every open or pending ask closes (9.8 rule 4).
    Ended,
}

/// A call's result and the `_meta.watane` pushes it caused, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Called<T> {
    pub value: T,
    pub pushes: Vec<Push>,
}

/// One `_meta.watane` push and the session it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Push {
    pub session: String,
    pub value: Value,
}

impl Push {
    fn of(session: &str, value: Value) -> Self {
        Push {
            session: session.to_string(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub profile: String,
    pub backend: String,
    /// The turn JSON of 9.3.
    pub turn: String,
    pub updated: u64,
}

pub struct Kernel<S: State, H: Host> {
    pub state: S,
    pub host: H,
}

/// Pushes and provider requests a call collects; requests leave only after the last write.
#[derive(Default)]
struct Work {
    pushes: Vec<Push>,
    requests: Vec<Request>,
}

/// The owner's first page of session ids holds at most this many (9.1).
const OWNER_PAGE: usize = 256;
/// A start or say adds at most a message (two parts) and a turn entry: chunk starts to keep free.
const CHUNK_MARGIN: usize = 4;

fn refused(field: &str, reason: &str) -> KernelError {
    KernelError::Refused(vec![Refusal {
        field: field.to_string(),
        backend: String::new(),
        model: String::new(),
        reason: reason.to_string(),
    }])
}

/// What `key/<owner>/<key>` holds: the first result, with the call and owner it belongs to, so a
/// key reused by another call kind, or met through an owner whose identity is a prefix, is never
/// taken as a replay nor overwritten.
#[derive(Serialize, Deserialize)]
struct Kept {
    call: String,
    owner: String,
    /// The session a `say` or `answer` acted on.
    #[serde(default)]
    session: Option<String>,
    value: Value,
}

/// The owner as one path segment: `%` and `/` escaped, so (owner, key) maps to one state key only.
fn segment(owner: &str) -> String {
    owner.replace('%', "%25").replace('/', "%2F")
}

fn key_of(owner: &str, key: &str) -> String {
    format!("key/{}/{key}", segment(owner))
}

fn owner_page_key(owner: &str) -> String {
    format!("owner/{}/0", segment(owner))
}

fn turn_push(t: &Turn) -> Value {
    json!({"type": "turn", "state": t.to_json()})
}

/// The `message` body of a user input: ACP content blocks as given, or the text as one block.
fn user_content(input: &str) -> Value {
    match serde_json::from_str::<Value>(input) {
        Ok(Value::Array(blocks)) => Value::Array(blocks),
        _ => json!([{"type": "text", "text": input}]),
    }
}

struct Decision {
    allow: bool,
    for_session: bool,
    reason: Option<String>,
    value: Value,
}

/// The four shapes of 2.3; anything else is one refusal `{field: "decision"}`.
fn parse_decision(s: &str) -> Result<Decision, KernelError> {
    let bad = |why: &str| refused("decision", why);
    let value: Value = serde_json::from_str(s).map_err(|_| bad("not valid JSON"))?;
    let Value::Object(o) = &value else {
        return Err(bad("not a JSON object"));
    };
    let t = |k: &str| o.get(k) == Some(&Value::Bool(true));
    let (allow, deny) = (o.contains_key("allow"), o.contains_key("deny"));
    if allow == deny {
        return Err(bad("exactly one of allow or deny"));
    }
    if allow {
        if !t("allow") {
            return Err(bad("allow is true"));
        }
        let allowed = |k: &str| matches!(k, "allow" | "for" | "input");
        if !o.keys().all(|k| allowed(k)) {
            return Err(bad("allow takes only for or input"));
        }
        let for_session = match o.get("for") {
            None => false,
            Some(Value::String(f)) if f == "session" => true,
            Some(_) => return Err(bad("for is \"session\"")),
        };
        match o.get("input") {
            None => {}
            Some(Value::Object(_)) if !for_session => {}
            Some(Value::Object(_)) => return Err(bad("for and input do not go together")),
            Some(_) => return Err(bad("input is an object")),
        }
        return Ok(Decision {
            allow: true,
            for_session,
            reason: None,
            value,
        });
    }
    if !t("deny") {
        return Err(bad("deny is true"));
    }
    if !o.keys().all(|k| k == "deny" || k == "reason") {
        return Err(bad("deny takes only reason"));
    }
    let reason = match o.get("reason") {
        None => None,
        Some(Value::String(r)) => Some(r.clone()),
        Some(_) => return Err(bad("reason is text")),
    };
    Ok(Decision {
        allow: false,
        for_session: false,
        reason,
        value,
    })
}

fn waiting(asks: Vec<String>, expires_at: Option<u64>) -> Turn {
    Turn::Waiting { asks, expires_at }
}

impl<S: State, H: Host> Kernel<S, H> {
    pub fn new(state: S, host: H) -> Self {
        Kernel { state, host }
    }

    /// Starts a top-level call: reads `slots` and the host's boot number (R1). When they differ,
    /// this call starts the restart pass: the new boot, the stream slot counts zeroed (`timers`
    /// stays), the cursor at page 0, and the timer `restart` with delay 0. It steps no session.
    fn begin(&mut self) -> Result<Slots, KernelError> {
        let mut slots = Slots::load(&mut self.state);
        let boot = self.host.boot();
        if boot != slots.boot {
            slots.boot = boot;
            slots.provider = 0;
            slots.request = 0;
            slots.job = 0;
            slots.restart = Some(0);
            slots.save(&mut self.state)?;
            self.host.timer_after(RESTART, 0);
        }
        Ok(slots)
    }

    /// Loads a session record for the caller: unknown, or another owner's, is an error. It does not
    /// step a stale record (`settle` does), so a call can check what it can before the step writes.
    fn load_owned(&mut self, caller: &str, id: &str) -> Result<Record, KernelError> {
        let rec = Record::load(&mut self.state, id)
            .ok_or_else(|| KernelError::UnknownSession(id.to_string()))?;
        if rec.owner != caller {
            return Err(KernelError::NotOwner);
        }
        Ok(rec)
    }

    /// R6: a record of an older boot does its restart step before the call acts on it.
    fn settle(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        if rec.boot < slots.boot {
            self.step(rec, slots, work)?;
        }
        Ok(())
    }

    fn session(
        &mut self,
        caller: &str,
        id: &str,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<Record, KernelError> {
        let mut rec = self.load_owned(caller, id)?;
        self.settle(&mut rec, slots, work)?;
        Ok(rec)
    }

    /// The restart step of one session (R4): its stale `stream/` and `live/` keys go, its boot
    /// (and its page entry's) becomes the current one, a Claude session's open and pending asks
    /// close in every turn state, and a Claude turn nothing drives becomes `unknown`. Writes: at most the record, its page (or `slots` and
    /// the page when it leaves the index), a decision entry per open ask and a `turn` entry (R3).
    // An OpenAI `running` or `waiting` session only gets the stale keys and the boot here: its rows
    // of the step table (9.5: requests, retries, tool jobs) come with stage D.
    fn step(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let now = self.host.now();
        rec.boot = slots.boot;
        let stale_streams = std::mem::take(&mut rec.streams);
        let driven = rec.backend == "claude"
            && matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. });
        // Every open or pending Claude ask closes, whatever the turn state; a driven turn ends
        // `unknown`. An OpenAI session's asks are the kernel's own: its `waiting` row is unchanged.
        if rec.backend == "claude" && self.close_all_with(rec, "restart", now, work, !driven)? {
            rec.updated = now;
        }
        if driven {
            self.turn_to(rec, Turn::Unknown, now, work)?;
            rec.updated = now;
            self.sync_active(rec, slots, true)?;
        } else if let Some(p) = rec.active_page {
            active::set_boot(&mut self.state, p, &rec.id, slots.boot)?;
        }
        rec.save(&mut self.state)?;
        for s in stale_streams {
            self.state.delete(&format!("stream/{s}"));
        }
        self.state.delete(&format!("live/{}", rec.id));
        Ok(())
    }

    /// The kept result of `call` for this key; a key kept by another call or owner is refused
    /// `{field: "key"}` and stays as it is.
    fn key_get<T: DeserializeOwned>(
        &mut self,
        call: &str,
        owner: &str,
        key: &str,
    ) -> Result<Option<(T, Option<String>)>, KernelError> {
        let Some(bytes) = self.state.get(&key_of(owner, key)) else {
            return Ok(None);
        };
        let used = || refused("key", "this key was used by another call");
        let kept: Kept = serde_json::from_slice(&bytes).map_err(|_| used())?;
        if kept.call != call || kept.owner != owner {
            return Err(used());
        }
        let session = kept.session;
        serde_json::from_value(kept.value)
            .map(|v| Some((v, session)))
            .map_err(|_| used())
    }

    /// A replay of a key kept for a session: the caller must own that session (a missing record
    /// is the caller's own deleted one), and the key must have acted on the session named.
    fn replay_for<T: DeserializeOwned>(
        &mut self,
        call: &str,
        caller: &str,
        key: &str,
        session: &str,
    ) -> Result<Option<T>, KernelError> {
        let Some((v, kept)) = self.key_get::<T>(call, caller, key)? else {
            return Ok(None);
        };
        if let Some(rec) = Record::load(&mut self.state, session) {
            if rec.owner != caller {
                return Err(KernelError::NotOwner);
            }
        }
        if kept.as_deref() != Some(session) {
            return Err(refused("key", "this key was used for another session"));
        }
        Ok(Some(v))
    }

    fn key_put<T: Serialize>(
        &mut self,
        call: &str,
        owner: &str,
        key: &str,
        session: Option<&str>,
        v: &T,
    ) -> Result<(), KernelError> {
        let kept = Kept {
            call: call.to_string(),
            owner: owner.to_string(),
            session: session.map(str::to_string),
            value: serde_json::to_value(v).expect("a key result serializes"),
        };
        let bytes = serde_json::to_vec(&kept).expect("a key result serializes");
        self.state
            .put(&key_of(owner, key), bytes)
            .map_err(|t| KernelError::TooLarge(t.0 as u64))
    }

    fn append(&mut self, rec: &mut Record, e: NewEntry, at: u64) -> Result<u64, KernelError> {
        let mut meta = rec.meta();
        let i = Transcript::new(&mut self.state, &rec.id, &mut meta).append(&e, at)?;
        rec.set_meta(&meta);
        Ok(i)
    }

    /// Stores a turn change: the record's turn and a `turn` entry, and pushes the turn. `activity`
    /// is live only (9.3): a change of it alone pushes and writes nothing.
    fn turn_to(
        &mut self,
        rec: &mut Record,
        t: Turn,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let mut stored = t.clone();
        if let Turn::Running { activity, .. } = &mut stored {
            *activity = None;
        }
        if stored == rec.turn {
            if t != stored {
                work.pushes.push(Push::of(&rec.id, turn_push(&t)));
            }
            return Ok(());
        }
        let entry = NewEntry {
            kind: "turn".into(),
            uuid: None,
            body: stored.entry_body(),
        };
        self.append(rec, entry, now)?;
        rec.turn = stored;
        work.pushes.push(Push::of(&rec.id, turn_push(&t)));
        Ok(())
    }

    /// A user message, the `running` turn and the request: what a started turn is (AC1).
    /// Returns the message's index.
    fn start_turn(
        &mut self,
        rec: &mut Record,
        input: String,
        resume: bool,
        now: u64,
        work: &mut Work,
    ) -> Result<u64, KernelError> {
        let message = NewEntry {
            kind: "message".into(),
            uuid: None,
            body: json!({"role": "user", "content": user_content(&input)}),
        };
        let n = self.append(rec, message, now)?;
        let running = Turn::Running {
            since: now,
            activity: None,
        };
        self.turn_to(rec, running, now, work)?;
        work.requests.push(Request::Turn {
            session: rec.id.clone(),
            input,
            resume,
        });
        Ok(n)
    }

    /// The oldest queued input starts a turn, as a turn that ended hands over (AC4).
    fn next_queued(
        &mut self,
        rec: &mut Record,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let mut q = Queue::load(&mut self.state, &rec.id);
        let Some(next) = q.take_oldest() else {
            return Ok(());
        };
        // 9.7, "Revoking": a revoked profile starts nothing; the queue is kept.
        if !self.may_use(&rec.owner, &rec.profile) {
            return Ok(());
        }
        self.start_turn(rec, next.input, false, now, work)?;
        q.save(&mut self.state, &rec.id)?;
        work.pushes.push(Push::of(&rec.id, q.push_value()));
        Ok(())
    }

    /// A turn change from the provider (or the kernel): ending with inputs queued starts the next.
    fn change_turn(
        &mut self,
        rec: &mut Record,
        t: Turn,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let before = rec.turn.is_active();
        self.turn_to(rec, t, now, work)?;
        if before && matches!(rec.turn, Turn::Idle { .. }) {
            self.next_queued(rec, now, work)?;
        }
        Ok(())
    }

    /// Takes an ask out of `open`; the turn follows when it was waiting on it.
    fn close_ask(
        &mut self,
        rec: &mut Record,
        id: &str,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        rec.open.retain(|o| o.id != id);
        if let Turn::Waiting { asks, .. } = rec.turn.clone() {
            let left: Vec<String> = asks.into_iter().filter(|a| a != id).collect();
            let t = if left.is_empty() {
                Turn::Running {
                    since: now,
                    activity: None,
                }
            } else {
                let expires = rec
                    .open
                    .iter()
                    .filter(|o| left.contains(&o.id))
                    .filter_map(|o| o.expires_at)
                    .min();
                waiting(left, expires)
            };
            self.turn_to(rec, t, now, work)?;
        }
        Ok(())
    }

    /// Writes the one decision entry of the ask at `i`: its entry repeated, with `decision`.
    fn write_decision(
        &mut self,
        rec: &mut Record,
        i: u64,
        decision: Value,
        now: u64,
    ) -> Result<(), KernelError> {
        let Some(mut body) = self.ask_body(rec, i) else {
            return Ok(());
        };
        let Some(obj) = body.as_object_mut() else {
            return Ok(());
        };
        obj.insert("decision".into(), decision);
        let entry = NewEntry {
            kind: "ask".into(),
            uuid: None,
            body,
        };
        self.append(rec, entry, now)?;
        Ok(())
    }

    /// The kernel's own closing step (`ended`, R4, `cancel`): each pending answer becomes its
    /// decision, each open ask or check `deny, by: kernel, reason`; both end empty.
    fn close_all(
        &mut self,
        rec: &mut Record,
        reason: &str,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        self.close_all_with(rec, reason, now, work, true)
            .map(|_| ())
    }

    /// `close_all`; `follow` lets a `waiting` turn follow (R4 sets its own turn). Whether any closed.
    fn close_all_with(
        &mut self,
        rec: &mut Record,
        reason: &str,
        now: u64,
        work: &mut Work,
        follow: bool,
    ) -> Result<bool, KernelError> {
        let any = !rec.open.is_empty() || !rec.pending_answers.is_empty();
        for o in std::mem::take(&mut rec.open) {
            rec.deadlines.retain(|d| d.what != format!("ask {}", o.id));
            let d = json!({"outcome": "deny", "by": "kernel", "reason": reason});
            self.write_decision(rec, o.i, d, now)?;
        }
        for (_, p) in std::mem::take(&mut rec.pending_answers) {
            self.write_decision(rec, p.i, p.decision(), now)?;
        }
        if any && follow && matches!(rec.turn, Turn::Waiting { .. }) {
            let t = Turn::Running {
                since: now,
                activity: None,
            };
            self.turn_to(rec, t, now, work)?;
        }
        Ok(any)
    }

    /// An `ask-resolved` the adapter checked (9.8 rule 4): the ask's one decision entry.
    fn resolve(
        &mut self,
        rec: &mut Record,
        r: adapters::Resolved,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let open = rec.open.iter().find(|o| o.id == r.id).cloned();
        let pending = rec.pending_answers.get(&r.id).cloned();
        let (i, kind) = match (&open, &pending) {
            (Some(o), _) => (o.i, o.kind.clone()),
            (None, Some(p)) => (p.i, p.kind.clone()),
            (None, None) => return self.unknown_entry(rec, r.raw, now),
        };
        let outcome = r.outcome.as_deref();
        let good = match r.by.as_str() {
            "answered" => match outcome {
                Some("deny") => true,
                Some("none") => kind == "check",
                Some("allow") => kind == "ask",
                _ => false,
            },
            "expired" | "cancelled" => outcome.is_none(),
            _ => false,
        };
        if !good {
            return self.unknown_entry(rec, r.raw, now);
        }
        let decision = match (r.by.as_str(), outcome) {
            ("answered", Some(o)) => match &pending {
                Some(p) if p.outcome == o => p.decision(),
                _ => json!({"outcome": o, "by": if r.direct { "host" } else { "checked" }}),
            },
            _ => json!({"outcome": "deny", "by": "host", "reason": r.by}),
        };
        self.write_decision(rec, i, decision, now)?;
        if open.is_some() {
            self.close_ask(rec, &r.id, now, work)
        } else {
            rec.pending_answers.remove(&r.id);
            Ok(())
        }
    }

    fn unknown_entry(&mut self, rec: &mut Record, raw: Value, now: u64) -> Result<(), KernelError> {
        let e = adapters::unknown(raw).entry.expect("unknown has an entry");
        self.append(rec, NewEntry::from(e), now)?;
        Ok(())
    }

    /// Moves the session in or out of `active/` as its end state asks (A2); one `slots` and one page.
    fn sync_active(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        was: bool,
    ) -> Result<(), KernelError> {
        let will = rec.turn.is_active();
        if !was && will {
            rec.active_page = Some(active::add(&mut self.state, slots, &rec.id, slots.boot)?);
        } else if was && !will {
            if let Some(p) = rec.active_page.take() {
                active::remove(&mut self.state, slots, &rec.id, p)?;
            }
        }
        Ok(())
    }

    fn flush(&mut self, work: Work) -> Vec<Push> {
        for r in work.requests {
            self.host.request(r);
        }
        work.pushes
    }

    /// 9.1, 9.7: whether `access/<owner>` names the profile; absent or unreadable allows none.
    fn may_use(&mut self, owner: &str, profile: &str) -> bool {
        self.state
            .get(&format!("access/{owner}"))
            .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
            .is_some_and(|names| names.iter().any(|n| n == profile))
    }

    fn revoked() -> KernelError {
        refused("profile", "this client may not use this profile")
    }

    fn room(rec: &Record) -> Result<(), KernelError> {
        if rec.chunks.len() + CHUNK_MARGIN > MAX_CHUNKS {
            return Err(KernelError::Quota);
        }
        Ok(())
    }

    pub fn start(
        &mut self,
        caller: &str,
        key: &str,
        profile: &str,
        overlay: &str,
        input: &str,
    ) -> Result<Called<String>, KernelError> {
        check_key(key)?;
        let mut slots = self.begin()?;
        if let Some((id, _)) = self.key_get::<String>("start", caller, key)? {
            return Ok(Called {
                value: id,
                pushes: Vec::new(),
            });
        }
        check_input(input)?;
        // 9.1: the profiles this caller may use; absent or unreadable allows none.
        if !self.may_use(caller, profile) {
            return Err(Self::revoked());
        }
        let Some(bytes) = self.state.get(&format!("profile/{profile}")) else {
            return Err(refused("profile", "no such profile"));
        };
        let json = String::from_utf8_lossy(&bytes).into_owned();
        let parsed = parse_profile(&json).0;
        let backend = parsed
            .backend
            .and_then(|b| b.parse::<Backend>().ok())
            .unwrap_or(Backend::Claude);
        // A Claude profile without a model runs the default one; an OpenAI one must name it (9.2).
        if backend == Backend::Openai && parsed.model.is_none() {
            return Err(refused("model", "an OpenAI profile needs a model"));
        }
        let model = parsed.model.unwrap_or_else(|| "default".to_string());
        let refusals = check(&json, overlay, backend, &model, None);
        if !refusals.is_empty() {
            return Err(KernelError::Refused(refusals));
        }
        active::check_room(&slots)?;
        let mut page = self
            .state
            .get(&owner_page_key(caller))
            .map_or_else(Vec::<String>::new, |b| {
                serde_json::from_slice(&b).unwrap_or_default()
            });
        if page.len() >= OWNER_PAGE {
            return Err(KernelError::Quota);
        }
        let id = self.host.new_id();
        if id.len() > ID_BYTES {
            return Err(KernelError::Backend(
                "the host's session id is too long".into(),
            ));
        }
        let now = self.host.now();
        let mut rec = Record {
            id: id.clone(),
            owner: caller.to_string(),
            key: key.to_string(),
            profile: profile.to_string(),
            backend: backend.as_str().to_string(),
            boot: slots.boot,
            created: now,
            updated: now,
            ..Record::default()
        };
        let mut work = Work::default();
        self.start_turn(&mut rec, input.to_string(), false, now, &mut work)?;
        self.sync_active(&mut rec, &mut slots, false)?;
        page.push(id.clone());
        self.state
            .put(
                &owner_page_key(caller),
                serde_json::to_vec(&page).expect("a page serializes"),
            )
            .map_err(|t| KernelError::TooLarge(t.0 as u64))?;
        rec.save(&mut self.state)?;
        self.key_put("start", caller, key, None, &id)?;
        Ok(Called {
            value: id,
            pushes: self.flush(work),
        })
    }

    pub fn say(
        &mut self,
        caller: &str,
        session: &str,
        key: &str,
        input: &str,
    ) -> Result<Called<u64>, KernelError> {
        check_key(key)?;
        let mut slots = self.begin()?;
        if let Some(n) = self.replay_for::<u64>("say", caller, key, session)? {
            return Ok(Called {
                value: n,
                pushes: Vec::new(),
            });
        }
        check_input(input)?;
        let mut rec = self.load_owned(caller, session)?;
        if !self.may_use(caller, &rec.profile) {
            return Err(Self::revoked());
        }
        let mut work = Work::default();
        self.settle(&mut rec, &mut slots, &mut work)?;
        // A pause with no time ends with the next `say` (9.3); one with a time queues until it
        // resumes on its own.
        let n = match rec.turn {
            Turn::Idle { .. } | Turn::Unknown | Turn::ResumeLost | Turn::Paused { until: None } => {
                let was = rec.turn.is_active();
                Self::room(&rec)?;
                if !was {
                    active::check_room(&slots)?;
                }
                // R5: `unknown` resumes the Claude session; with inputs queued, this one joins
                // the end and the oldest starts the turn.
                let resume = rec.turn == Turn::Unknown;
                let mut q = Queue::load(&mut self.state, &rec.id);
                // The oldest leaves before the new input joins, so 32 queued inputs never refuse; the 2 MiB of stored bytes still can (`queue-full`).
                let queued = if q.is_empty() {
                    None
                } else {
                    let oldest = q.take_oldest().expect("the queue holds inputs");
                    Some((oldest, q.push(input.to_string())?))
                };
                let now = self.host.now();
                let n = match queued {
                    None => self.start_turn(&mut rec, input.to_string(), resume, now, &mut work)?,
                    Some((oldest, n)) => {
                        self.start_turn(&mut rec, oldest.input, resume, now, &mut work)?;
                        q.save(&mut self.state, &rec.id)?;
                        work.pushes.push(Push::of(&rec.id, q.push_value()));
                        n
                    }
                };
                self.sync_active(&mut rec, &mut slots, was)?;
                rec.updated = now;
                rec.save(&mut self.state)?;
                n
            }
            _ => {
                let mut q = Queue::load(&mut self.state, &rec.id);
                let n = q.push(input.to_string())?;
                q.save(&mut self.state, &rec.id)?;
                work.pushes.push(Push::of(&rec.id, q.push_value()));
                n
            }
        };
        self.key_put("say", caller, key, Some(session), &n)?;
        Ok(Called {
            value: n,
            pushes: self.flush(work),
        })
    }

    /// The whole ask entry's body at index `i`, joined when it is stored in parts.
    fn ask_body(&mut self, rec: &Record, i: u64) -> Option<Value> {
        let mut meta = rec.meta();
        let mut tr = Transcript::new(&mut self.state, &rec.id, &mut meta);
        let line = tr.read(i.checked_sub(1)?, None, 1).ok()?.entries.pop()?;
        let v: Value = serde_json::from_str(&line).ok()?;
        let whole = if v.get("kind").and_then(Value::as_str) == Some("part") {
            let m = v.pointer("/body/m").and_then(Value::as_u64)?;
            let lines = tr
                .read(i.checked_sub(m)?, None, u32::try_from(m).ok()?)
                .ok()?
                .entries;
            let mut j = Joiner::new();
            lines.iter().filter_map(|l| j.push(l)).last()?
        } else {
            v
        };
        whole.get("body").cloned()
    }

    pub fn answer(
        &mut self,
        caller: &str,
        session: &str,
        key: &str,
        ask: &str,
        decision: &str,
    ) -> Result<Called<()>, KernelError> {
        check_key(key)?;
        let mut slots = self.begin()?;
        if self
            .replay_for::<()>("answer", caller, key, session)?
            .is_some()
        {
            return Ok(Called {
                value: (),
                pushes: Vec::new(),
            });
        }
        let mut rec = self.load_owned(caller, session)?;
        let d = parse_decision(decision)?;
        if d.allow && !self.may_use(caller, &rec.profile) {
            return Err(Self::revoked());
        }
        // The step comes after the refusals that do not depend on it; an ask it denies is refused.
        let mut work = Work::default();
        self.settle(&mut rec, &mut slots, &mut work)?;
        let open = rec
            .open
            .iter()
            .find(|o| o.id == ask && o.kind == "ask")
            .cloned()
            .ok_or_else(|| refused("ask", "no open ask with this id"))?;
        let unreadable = || refused("ask", "the ask entry is unreadable");
        let body = self.ask_body(&rec, open.i).ok_or_else(unreadable)?;
        if rec.pending_answers.len() >= OPEN_ASKS {
            return Err(KernelError::Busy(format!(
                "{OPEN_ASKS} pending answers per session"
            )));
        }
        let pending = PendingAnswer {
            i: open.i,
            kind: open.kind.clone(),
            outcome: if d.allow { "allow" } else { "deny" }.to_string(),
            by: "owner".to_string(),
            for_: d.for_session.then(|| "session".to_string()),
            reason: d.reason.clone(),
            call: open.call.clone(),
            input: None,
        };
        let size = serde_json::to_vec(&pending)
            .expect("an answer serializes")
            .len();
        if size > ALLOW_BYTES {
            return Err(KernelError::TooLarge(ALLOW_BYTES as u64));
        }
        let pending = PendingAnswer {
            input: d.value.get("input").cloned(),
            ..pending
        };
        if d.for_session {
            let Some(reach) = body.get("reach").and_then(Value::as_str) else {
                return Err(refused("decision", "this ask offers no session allow"));
            };
            rec.add_allow(Value::String(reach.to_string()))?;
        }
        let now = self.host.now();
        self.close_ask(&mut rec, ask, now, &mut work)?;
        rec.pending_answers.insert(ask.to_string(), pending);
        work.requests.push(Request::Answer {
            session: rec.id.clone(),
            ask: ask.to_string(),
            decision: d.value,
        });
        rec.updated = now;
        rec.save(&mut self.state)?;
        self.key_put("answer", caller, key, Some(session), &())?;
        Ok(Called {
            value: (),
            pushes: self.flush(work),
        })
    }

    /// 2.3 `interrupt`: ends an active turn `idle {interrupted}` (open asks stay: the host closes
    /// them, 9.3 R4), then starts the replacement, if given, as `say` on `idle` does.
    pub fn interrupt(
        &mut self,
        caller: &str,
        session: &str,
        replacement: Option<&str>,
    ) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut rec = self.load_owned(caller, session)?;
        if let Some(input) = replacement {
            check_input(input)?;
            if !self.may_use(caller, &rec.profile) {
                return Err(Self::revoked());
            }
        }
        let mut work = Work::default();
        self.settle(&mut rec, &mut slots, &mut work)?;
        let was = rec.turn.is_active();
        if replacement.is_none() && !was {
            return Ok(Called {
                value: (),
                pushes: work.pushes,
            });
        }
        if replacement.is_some() {
            Self::room(&rec)?;
            if !was {
                active::check_room(&slots)?;
            }
        }
        let now = self.host.now();
        if was {
            self.end_interrupted(&mut rec, now, &mut work)?;
            work.requests.push(Request::Interrupt {
                session: rec.id.clone(),
            });
        }
        if let Some(input) = replacement {
            let resume = rec.turn == Turn::Unknown;
            self.start_turn(&mut rec, input.to_string(), resume, now, &mut work)?;
        }
        self.sync_active(&mut rec, &mut slots, was)?;
        rec.updated = now;
        rec.save(&mut self.state)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work),
        })
    }

    fn end_interrupted(
        &mut self,
        rec: &mut Record,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let idle = Turn::Idle {
            stop: Stop::Interrupted,
            seconds: None,
            tools: None,
            tokens: None,
        };
        self.turn_to(rec, idle, now, work)
    }

    /// 2.3 `cancel`: every open or pending ask gets its one decision, an active turn ends
    /// `idle {interrupted}`, the queue is kept, the host ends the stream.
    // Children (9.3) are not built: nothing to cancel there yet.
    pub fn cancel(&mut self, caller: &str, session: &str) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut work = Work::default();
        let mut rec = self.session(caller, session, &mut slots, &mut work)?;
        let now = self.host.now();
        let was = rec.turn.is_active();
        let closed = self.close_all_with(&mut rec, "cancel", now, &mut work, false)?;
        if was {
            self.end_interrupted(&mut rec, now, &mut work)?;
        }
        self.sync_active(&mut rec, &mut slots, was)?;
        work.requests.push(Request::Cancel {
            session: rec.id.clone(),
        });
        if was || closed {
            rec.updated = now;
        }
        rec.save(&mut self.state)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work),
        })
    }

    pub fn queued(&mut self, caller: &str, session: &str) -> Result<Vec<Queued>, KernelError> {
        let mut slots = self.begin()?;
        let rec = self.session(caller, session, &mut slots, &mut Work::default())?;
        Ok(Queue::load(&mut self.state, &rec.id).inputs)
    }

    pub fn unqueue(
        &mut self,
        caller: &str,
        session: &str,
        n: u64,
    ) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut work = Work::default();
        let rec = self.session(caller, session, &mut slots, &mut work)?;
        let mut q = Queue::load(&mut self.state, &rec.id);
        q.remove(n)?;
        q.save(&mut self.state, &rec.id)?;
        work.pushes.push(Push::of(&rec.id, q.push_value()));
        Ok(Called {
            value: (),
            pushes: work.pushes,
        })
    }

    pub fn read(
        &mut self,
        caller: &str,
        session: &str,
        after: u64,
        before: Option<u64>,
        max: u32,
    ) -> Result<Page, KernelError> {
        let mut slots = self.begin()?;
        let rec = self.session(caller, session, &mut slots, &mut Work::default())?;
        let mut meta = rec.meta();
        Transcript::new(&mut self.state, &rec.id, &mut meta).read(after, before, max)
    }

    pub fn rename(&mut self, caller: &str, session: &str, title: &str) -> Result<(), KernelError> {
        check_title(title)?;
        let mut slots = self.begin()?;
        let mut rec = self.session(caller, session, &mut slots, &mut Work::default())?;
        rec.title = title.to_string();
        rec.save(&mut self.state)
    }

    pub fn delete(&mut self, caller: &str, session: &str) -> Result<(), KernelError> {
        let mut slots = self.begin()?;
        let rec = self.session(caller, session, &mut slots, &mut Work::default())?;
        let page_key = owner_page_key(caller);
        let page: Vec<String> = self
            .state
            .get(&page_key)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if let Some(p) = rec.active_page {
            active::remove(&mut self.state, &mut slots, &rec.id, p)?;
        }
        for n in 0..rec.chunks.len() {
            self.state.delete(&crate::transcript::chunk_key(&rec.id, n));
        }
        self.state.delete(&Queue::key_of(&rec.id));
        let page: Vec<String> = page.into_iter().filter(|i| *i != rec.id).collect();
        if page.is_empty() {
            self.state.delete(&page_key);
        } else {
            self.state
                .put(
                    &page_key,
                    serde_json::to_vec(&page).expect("a page serializes"),
                )
                .map_err(|t| KernelError::TooLarge(t.0 as u64))?;
        }
        self.state.delete(&Record::key_of(&rec.id));
        Ok(())
    }

    pub fn list(
        &mut self,
        caller: &str,
        after: Option<&str>,
        max: u32,
    ) -> Result<Vec<SessionInfo>, KernelError> {
        if max == 0 {
            return Err(refused("max", "max must be at least 1"));
        }
        let slots = self.begin()?;
        let ids: Vec<String> = self
            .state
            .get(&owner_page_key(caller))
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let from = match after {
            None => 0,
            Some(a) => ids.iter().position(|i| i == a).map_or(ids.len(), |p| p + 1),
        };
        let mut out = Vec::new();
        for id in ids.iter().skip(from) {
            if out.len() >= max.min(256) as usize {
                break;
            }
            if let Some(r) = Record::load(&mut self.state, id).filter(|r| r.owner == caller) {
                // R6: the turn the restart step will give, without writing it.
                let stale = r.boot < slots.boot
                    && r.backend == "claude"
                    && matches!(r.turn, Turn::Running { .. } | Turn::Waiting { .. });
                let turn = if stale { Turn::Unknown } else { r.turn };
                out.push(SessionInfo {
                    id: r.id,
                    title: r.title,
                    profile: r.profile,
                    backend: r.backend,
                    turn: turn.to_json().to_string(),
                    updated: r.updated,
                });
            }
        }
        Ok(out)
    }

    /// One item of a provider's stream: no R1, no owner (the kernel's own).
    pub fn provider(
        &mut self,
        session: &str,
        event: ProviderEvent,
    ) -> Result<Called<()>, KernelError> {
        let mut rec = Record::load(&mut self.state, session)
            .ok_or_else(|| KernelError::UnknownSession(session.to_string()))?;
        let before = rec.clone();
        let was = rec.turn.is_active();
        let mut slots = None;
        if !was && may_activate(&event) {
            let s = Slots::load(&mut self.state);
            active::check_room(&s)?;
            slots = Some(s);
        }
        let now = self.host.now();
        let mut work = Work::default();
        match event {
            ProviderEvent::Turn(t) => self.change_turn(&mut rec, t, now, &mut work)?,
            ProviderEvent::Ended => self.close_all(&mut rec, "ended", now, &mut work)?,
            ProviderEvent::Out(out) => {
                if let Some(e) = out.entry {
                    self.out_entry(&mut rec, e, now, &mut work)?;
                }
                if let Some(r) = out.resolved {
                    self.resolve(&mut rec, r, now, &mut work)?;
                }
                if out.idle {
                    let idle = Turn::Idle {
                        stop: Stop::Completed,
                        seconds: None,
                        tools: None,
                        tokens: None,
                    };
                    self.change_turn(&mut rec, idle, now, &mut work)?;
                }
            }
        }
        if was != rec.turn.is_active() {
            let mut s = match slots {
                Some(s) => s,
                None => Slots::load(&mut self.state),
            };
            self.sync_active(&mut rec, &mut s, was)?;
        }
        if rec != before {
            rec.updated = now;
            rec.save(&mut self.state)?;
        }
        Ok(Called {
            value: (),
            pushes: self.flush(work),
        })
    }

    /// Whether an ask item reuses the id of an open or pending ask, or shows another tool or input
    /// on the call of one (9.8): the record decides, not the adapter's memory.
    fn reused(&mut self, rec: &Record, body: &Value) -> bool {
        let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
        if rec.open.iter().any(|o| o.id == id) || rec.pending_answers.contains_key(id) {
            return true;
        }
        let call = body
            .get("toolCallId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if call.is_empty() {
            return false;
        }
        let at = rec
            .open
            .iter()
            .map(|o| (&o.call, o.i))
            .chain(rec.pending_answers.values().map(|p| (&p.call, p.i)))
            .find(|(c, _)| c.as_str() == call)
            .map(|(_, i)| i);
        let Some(other) = at.and_then(|i| self.ask_body(rec, i)) else {
            return false;
        };
        other.pointer("/toolCall/title") != body.pointer("/toolCall/title")
            || other.get("input") != body.get("input")
    }

    /// An entry from the provider; an `ask` opens (or, with a decision, closes) an ask.
    fn out_entry(
        &mut self,
        rec: &mut Record,
        e: adapters::Entry,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let is_ask = e.kind == Kind::Ask;
        let body = e.body.clone();
        if is_ask && !body.get("decision").is_some_and(|d| !d.is_null()) && self.reused(rec, &body)
        {
            return self.unknown_entry(rec, body, now);
        }
        let i = self.append(rec, NewEntry::from(e), now)?;
        let id = body.get("id").and_then(Value::as_str);
        let (true, Some(id)) = (is_ask, id) else {
            return Ok(());
        };
        if body.get("decision").is_some_and(|d| !d.is_null()) {
            return self.close_ask(rec, id, now, work);
        }
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("ask");
        if rec.open.len() + rec.pending_answers.len() >= OPEN_ASKS {
            // Past the cap: denied at once, so a closing call writes at most 64 decisions.
            let d = json!({"outcome": "deny", "by": "kernel", "reason": "busy"});
            self.write_decision(rec, i, d, now)?;
            work.requests.push(Request::Answer {
                session: rec.id.clone(),
                ask: id.to_string(),
                decision: json!({"deny": true, "reason": "busy"}),
            });
            return Ok(());
        }
        let mine = body.get("expires-at").and_then(Value::as_u64);
        rec.open.push(OpenAsk {
            id: id.to_string(),
            kind: kind.to_string(),
            i,
            call: body
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            expires_at: mine,
        });
        if kind == "ask" {
            let (mut asks, theirs) = match &rec.turn {
                Turn::Waiting { asks, expires_at } => (asks.clone(), *expires_at),
                _ => (Vec::new(), None),
            };
            asks.push(id.to_string());
            let expires = [mine, theirs].into_iter().flatten().min();
            self.turn_to(rec, waiting(asks, expires), now, work)?;
        }
        Ok(())
    }

    /// `timer-sink.fired`. Session timers and deadlines are not built yet; `restart` is R2.
    pub fn timer(&mut self, id: &str) -> Called<()> {
        let mut work = Work::default();
        if let Ok(mut slots) = self.begin() {
            if id == RESTART {
                // A firing that still fails (its page or `slots` cannot be written) re-arms nothing.
                let _ = self.restart_firing(&mut slots, &mut work);
            }
        }
        Called {
            value: (),
            pushes: work.pushes,
        }
    }

    /// One firing of the pass (R2): the next session of an older boot does its step, or the pass ends.
    fn restart_firing(&mut self, slots: &mut Slots, work: &mut Work) -> Result<(), KernelError> {
        let from = slots.restart;
        if from.is_none() {
            return Ok(());
        }
        let Some((page, entry)) = active::next_stale(&mut self.state, slots) else {
            slots.save(&mut self.state)?;
            self.host.timer_cancel(RESTART);
            return Ok(());
        };
        let mut saved = false;
        match Record::load(&mut self.state, &entry.id) {
            None => {
                // A listed session with no record: nothing to step; drop the entry.
                active::remove(&mut self.state, slots, &entry.id, page)?;
                saved = true;
            }
            Some(mut rec) if rec.boot < slots.boot => {
                rec.active_page = Some(page);
                let mut done = Work::default();
                if self.step(&mut rec, slots, &mut done).is_ok() {
                    work.pushes.append(&mut done.pushes);
                } else if rec.active_page.is_some() {
                    // A step that fails must not stall the pass: its entry counts as current, and
                    // the record, still of an older boot, does its step when a call reads it (R6).
                    active::set_boot(&mut self.state, page, &entry.id, slots.boot)?;
                }
                saved = rec.active_page.is_none();
            }
            Some(_) => active::set_boot(&mut self.state, page, &entry.id, slots.boot)?,
        }
        if !saved && slots.restart != from {
            slots.save(&mut self.state)?;
        }
        self.host.timer_after(RESTART, 0);
        Ok(())
    }
}

/// Whether the event could move an inactive session into an active turn.
fn may_activate(e: &ProviderEvent) -> bool {
    match e {
        ProviderEvent::Turn(t) => t.is_active(),
        ProviderEvent::Out(o) => o.entry.as_ref().is_some_and(|e| e.kind == Kind::Ask),
        ProviderEvent::Ended => false,
    }
}
