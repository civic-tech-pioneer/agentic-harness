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
use crate::quota::{segment, Change, Limits, Metered, Past, Usage};
use crate::record::{
    check_key, check_title, Deadline, OpenAsk, PendingAnswer, Record, Stop, Turn, ALLOW_BYTES,
    ID_BYTES, OPEN_ASKS,
};
use crate::state::State;
use crate::transcript::{Joiner, NewEntry, Page, Replay, Transcript, MAX_CHUNKS};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// When the session timer is next due (9.3): at the earliest deadline still in the future. A
/// firing handles every due deadline it can; one of a later stage (`ask`, `retry`, `resume`,
/// stage D) that is already due waits for its stage, so it arms no timer (one at delay 0 would
/// fire on every firing).
fn next_due(deadlines: &[Deadline], now: u64) -> Option<u64> {
    deadlines.iter().filter(|d| d.at > now).map(|d| d.at).min()
}

/// The kernel's own timer for the restart pass (R2).
const RESTART: &str = "restart";
/// The deadline of the 10-minute provider release (9.3, 9.5), and its delay.
const IDLE: &str = "idle";
const IDLE_MS: u64 = 600_000;

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
    /// Opens a provider stream for a Claude session (3): its id, as the host's items will carry it.
    fn open_stream(&mut self) -> String;
    /// The consumer's `cancel` of a stream the kernel holds.
    fn cancel_stream(&mut self, stream: &str);
    /// The producer's `push` on a stream the kernel opened (an owner stream): `Full` when it is at its window (3).
    fn push(&mut self, stream: &str, value: Value) -> Result<(), Full>;
    /// The producer's `close` of a stream the kernel opened, with its outcome.
    fn close_stream(&mut self, stream: &str, outcome: &str);
    /// The consumer's `resume` of a stream the kernel holds; it does nothing for one that is not deferred (3).
    fn resume_stream(&mut self, stream: &str);
}

/// A `push` past the stream's window (3, "In flight").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

/// What an item call answers the host (3, "Deferred batch"): `Defer` is a call rolled back whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Done,
    Defer,
}

/// What `attach` answers (9.4): the owner stream it opened, or that too much lies after `after`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attached {
    Live { stream: String },
    Behind { through: u64 },
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
    Open { stream: String },
    CancelStream { stream: String },
    Push { stream: String, value: Value },
    Close { stream: String, outcome: String },
    Resume { stream: String },
}

/// A host for tests: a settable clock and boot, ids `s1`, `s2`, ..., and a log of what was asked.
#[derive(Debug, Default)]
pub struct FakeHost {
    pub boot: u64,
    pub now: u64,
    pub next_id: u64,
    pub next_stream: u64,
    pub calls: Vec<HostCall>,
    /// Streams whose window is full: a push to one fails `Full`.
    pub full: BTreeSet<String>,
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
    fn open_stream(&mut self) -> String {
        self.next_stream += 1;
        let stream = format!("t{}", self.next_stream);
        self.calls.push(HostCall::Open {
            stream: stream.clone(),
        });
        stream
    }
    fn cancel_stream(&mut self, stream: &str) {
        self.calls.push(HostCall::CancelStream {
            stream: stream.to_string(),
        });
    }
    fn push(&mut self, stream: &str, value: Value) -> Result<(), Full> {
        if self.full.contains(stream) {
            return Err(Full);
        }
        self.calls.push(HostCall::Push {
            stream: stream.to_string(),
            value,
        });
        Ok(())
    }
    fn close_stream(&mut self, stream: &str, outcome: &str) {
        self.calls.push(HostCall::Close {
            stream: stream.to_string(),
            outcome: outcome.to_string(),
        });
    }
    fn resume_stream(&mut self, stream: &str) {
        self.calls.push(HostCall::Resume {
            stream: stream.to_string(),
        });
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

/// The refusal cause of the kernel's shared account: it names no other owner (14).
const ACCOUNT_CAUSE: &str = "this mod's agent sessions are at their limit";

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
    pub state: Metered<S>,
    pub host: H,
    /// The kernel's quota settings (9.3 Quota).
    pub limits: Limits,
    /// What of the call's change in session bytes the record in hand already holds (`seal`).
    sealed: i64,
    /// The provider stream the call in hand opened: a call that then fails cancels it (7).
    opened: Option<String>,
    /// While an item call may still be dropped whole (3, "Deferred batch"), the host calls it
    /// makes outside `request` wait here until the call commits.
    held: Option<Vec<Effect>>,
}

/// A host call an item call makes that must not happen when the call defers.
enum Effect {
    TimerAfter(String, u64),
    TimerCancel(String),
    Cancel(String),
}

/// Pushes and provider requests a call collects; requests leave only after the last write.
#[derive(Default)]
struct Work {
    pushes: Vec<Push>,
    requests: Vec<Request>,
    /// The index of the `message` or `tool` entry the call stored last (9.4, "The wire").
    stored: Option<u64>,
    /// The live owner stream of the session an item call acted on, as its record ends the call.
    live: Option<String>,
}

/// The owner's account and the kernel's, as read for a call and kept for its one write (9.3 Quota).
struct Acct {
    /// Past quota when the accounts were read; only `accounts` makes it.
    past: Past,
    owner_key: String,
    owner: Usage,
    account: Usage,
    owner0: Usage,
    account0: Usage,
}

fn add_signed(v: u64, d: i64) -> u64 {
    if d >= 0 {
        v.saturating_add(d as u64)
    } else {
        v.saturating_sub(d.unsigned_abs())
    }
}

/// The owner's first page of session ids holds at most this many (9.1).
const OWNER_PAGE: usize = 256;
/// A start or say adds at most a message (two parts) and a turn entry: chunk starts to keep free.
const CHUNK_MARGIN: usize = 4;

/// A cap for a `quota` cause: whole MiB as "64 MiB", any other size in bytes, so the text names the real cap.
fn size(bytes: u64) -> String {
    if bytes > 0 && bytes.is_multiple_of(1 << 20) {
        format!("{} MiB", bytes >> 20)
    } else {
        format!("{bytes} bytes")
    }
}

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

/// What `stream/<id>` holds (9.1): the session a stream serves and what kind of stream it is.
#[derive(Serialize, Deserialize)]
struct StreamKey {
    session: String,
    kind: String,
}

const PROVIDER: &str = "provider";
const OWNER: &str = "owner";

fn stream_key(id: &str) -> String {
    format!("stream/{id}")
}

fn key_of(owner: &str, key: &str) -> String {
    format!("key/{}/{key}", segment(owner))
}

fn owner_page_key(owner: &str, n: u32) -> String {
    format!("owner/{}/{n}", segment(owner))
}

/// The update of an `Out` that only forwards a delta: no entry, resolution, idle or usage.
fn delta_only(out: &adapters::Out) -> Option<Value> {
    let bare = out.entry.is_none()
        && out.resolved.is_none()
        && out.watane.is_none()
        && out.usage.is_none()
        && !out.idle;
    out.update.clone().filter(|_| bare)
}

/// An ACP update as it goes on the wire: `_meta.watane.at`, and `i` when it finishes an entry (9.4).
fn wire_update(mut update: Value, at: u64, i: Option<u64>) -> Value {
    if let Some(o) = update.as_object_mut() {
        let meta = o.entry("_meta").or_insert_with(|| json!({}));
        if !meta.is_object() {
            *meta = json!({});
        }
        let w = &mut meta["watane"];
        if !w.is_object() {
            *w = json!({});
        }
        w["at"] = json!(at);
        if let Some(i) = i {
            w["i"] = json!(i);
        }
    }
    update
}

/// A push as it goes on the owner stream (9.4 "The wire"): an ACP update as it is, the kernel's
/// own items (`turn`, `queue`) under `_meta.watane`.
fn on_wire(v: &Value) -> Value {
    if v.get("sessionUpdate").is_some() {
        return v.clone();
    }
    json!({"_meta": {"watane": v}})
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
        Kernel {
            state: Metered::new(state),
            host,
            limits: Limits::default(),
            sealed: 0,
            opened: None,
            held: None,
        }
    }

    /// Starts a top-level call: reads `slots` and the host's boot number (R1). When they differ,
    /// this call starts the restart pass: the new boot, the stream slot counts zeroed (`timers`
    /// stays), the cursor at page 0, and the timer `restart` with delay 0. It steps no session.
    fn begin(&mut self) -> Result<Slots, KernelError> {
        self.open_meter();
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

    fn open_meter(&mut self) {
        self.state.open();
        self.sealed = 0;
    }

    /// Saves the record with `bytes` taking the change in its session keys so far.
    fn seal(&mut self, rec: &mut Record) -> Result<(), KernelError> {
        let c = self.state.change().session;
        rec.bytes = add_signed(rec.bytes, c - self.sealed);
        rec.save(&mut self.state)?;
        // Only a saved record holds the change (a failed restart step's is the firing's to keep).
        self.sealed = c;
        Ok(())
    }

    /// Reads the owner's account and the kernel's (9.1).
    fn accounts(&mut self, owner: &str) -> Acct {
        let owner_key = Usage::owner_key(owner);
        // 9.3 "Past": the owner's bytes or the kernel's account at or over their caps.
        let (past, owner, account) = Past::load(&mut self.state, &owner_key, &self.limits);
        Acct {
            past,
            owner_key,
            owner0: owner.clone(),
            account0: account.clone(),
            owner,
            account,
        }
    }

    /// The cap `past` found, named for the refusal.
    fn past_cause(&self, a: &Acct) -> Option<String> {
        if a.owner.bytes >= self.limits.owner_bytes {
            Some(format!("{} per owner", size(self.limits.owner_bytes)))
        } else if a.account.bytes >= self.limits.account_bytes {
            Some(ACCOUNT_CAUSE.to_string())
        } else {
            None
        }
    }

    fn refuse_past(&self, a: &Acct) -> Result<(), KernelError> {
        match self.past_cause(a) {
            Some(cause) => Err(KernelError::Quota(cause)),
            None => Ok(()),
        }
    }

    /// 9.5 "Slots": refuses `busy` when the kernel holds all its provider streams or the owner its share.
    fn check_provider(&self, slots: &Slots, a: &Acct) -> Result<(), KernelError> {
        if slots.provider >= self.limits.provider {
            return Err(KernelError::Busy(format!(
                "{} provider streams",
                self.limits.provider
            )));
        }
        if a.owner.provider_at(slots.boot) >= self.limits.owner_provider {
            return Err(KernelError::Busy(format!(
                "{} provider streams per owner",
                self.limits.owner_provider
            )));
        }
        Ok(())
    }

    /// The session's live provider stream: the first of its `streams` whose key says `provider`.
    fn live_provider(&mut self, rec: &Record) -> Option<String> {
        for s in &rec.streams {
            let Some(bytes) = self.state.get(&stream_key(s)) else {
                continue;
            };
            if serde_json::from_slice::<StreamKey>(&bytes).is_ok_and(|k| k.kind == PROVIDER) {
                return Some(s.clone());
            }
        }
        None
    }

    /// Opens the session's provider stream: the host's id, its `stream/` key, the record's
    /// `streams`, and one slot of the kernel and of the owner. `slots` is saved by the caller.
    fn open_provider(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        a: &mut Acct,
    ) -> Result<(), KernelError> {
        let t = self.host.open_stream();
        self.opened = Some(t.clone());
        let key = StreamKey {
            session: rec.id.clone(),
            kind: PROVIDER.to_string(),
        };
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        self.state
            .put(&stream_key(&t), bytes)
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        rec.add_stream(t)?;
        slots.provider += 1;
        a.owner.open_provider(slots.boot);
        Ok(())
    }

    /// Releases a provider stream the session holds: its key, its place in `streams`, and, when
    /// the stream is of this boot, one slot of the kernel and of the owner. Whether `slots`
    /// changed; the caller saves it.
    fn release_provider(
        &mut self,
        rec: &mut Record,
        stream: &str,
        slots: &mut Slots,
        a: &mut Acct,
    ) -> bool {
        self.state.delete(&stream_key(stream));
        rec.streams.retain(|s| s != stream);
        if rec.boot != slots.boot {
            return false;
        }
        slots.provider = slots.provider.saturating_sub(1);
        a.owner.close_provider(slots.boot);
        true
    }

    fn set_timer(&mut self, id: &str, delay_ms: u64) {
        match &mut self.held {
            Some(h) => h.push(Effect::TimerAfter(id.to_string(), delay_ms)),
            None => self.host.timer_after(id, delay_ms),
        }
    }

    fn clear_timer(&mut self, id: &str) {
        match &mut self.held {
            Some(h) => h.push(Effect::TimerCancel(id.to_string())),
            None => self.host.timer_cancel(id),
        }
    }

    fn cancel_provider(&mut self, stream: &str) {
        match &mut self.held {
            Some(h) => h.push(Effect::Cancel(stream.to_string())),
            None => self.host.cancel_stream(stream),
        }
    }

    /// The item call commits: its held writes and host calls go out, in the order they were made.
    fn commit_held(&mut self) {
        self.state.commit();
        for e in self.held.take().unwrap_or_default() {
            match e {
                Effect::TimerAfter(id, ms) => self.host.timer_after(&id, ms),
                Effect::TimerCancel(id) => self.host.timer_cancel(&id),
                Effect::Cancel(t) => self.host.cancel_stream(&t),
            }
        }
    }

    /// The item call is dropped whole: nothing it wrote or asked of the host remains.
    fn drop_held(&mut self) {
        self.state.discard();
        self.held = None;
    }

    /// Runs a call that may open a provider stream; when it fails the stream goes back (7).
    fn guarded<T>(
        &mut self,
        call: impl FnOnce(&mut Self) -> Result<T, KernelError>,
    ) -> Result<T, KernelError> {
        self.opened = None;
        let r = call(self);
        if let Some(t) = self.opened.take() {
            if r.is_err() {
                self.host.cancel_stream(&t);
            }
        }
        r
    }

    /// 9.3: the session's one timer `s/<id>` follows its deadlines. `before` is what they were when
    /// the call began; a change from none takes a timer slot, to none frees it (`fired`: the timer
    /// that fired needs no cancel, and is re-armed even when the deadlines did not change). The
    /// count follows the deadlines (AC8), the host timer only those this build handles
    /// (`next_due`). Whether `slots` changed.
    fn sync_timer(
        &mut self,
        rec: &Record,
        slots: &mut Slots,
        a: &mut Acct,
        before: &[Deadline],
        now: u64,
        fired: bool,
    ) -> bool {
        if rec.deadlines == before && !fired {
            return false;
        }
        let id = format!("s/{}", rec.id);
        let had = !before.is_empty();
        match next_due(&rec.deadlines, now) {
            Some(at) => self.set_timer(&id, at.saturating_sub(now)),
            None if had && !fired => self.clear_timer(&id),
            None => {}
        }
        match (had, rec.deadlines.is_empty()) {
            (false, false) => {
                slots.timers += 1;
                a.owner.timers += 1;
                true
            }
            (true, true) => {
                slots.timers = slots.timers.saturating_sub(1);
                a.owner.timers = a.owner.timers.saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    /// 9.5: a Claude turn that moved into `idle` while it holds a provider stream sets the `idle`
    /// deadline; a session needing a timer at a cap releases its stream at once. A turn that is
    /// active removes the deadline. Whether `slots` changed; the caller saves it.
    fn idle_after(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        a: &mut Acct,
        was: bool,
        before: &[Deadline],
        now: u64,
    ) -> bool {
        let mut changed = false;
        if rec.turn.is_active() {
            rec.deadlines.retain(|d| d.what != IDLE);
        } else if was && matches!(rec.turn, Turn::Idle { .. }) {
            if let Some(t) = self.live_provider(rec) {
                rec.deadlines.retain(|d| d.what != IDLE);
                let capped = before.is_empty()
                    && (slots.timers >= self.limits.timers
                        || a.owner.timers >= self.limits.owner_timers);
                let idle = Deadline {
                    what: IDLE.to_string(),
                    at: now + IDLE_MS,
                };
                if capped || rec.add_deadline(idle).is_err() {
                    self.cancel_provider(&t);
                    changed = self.release_provider(rec, &t, slots, a);
                }
            }
        }
        self.sync_timer(rec, slots, a, before, now, false) || changed
    }

    /// Whether a turn this call starts opens a provider stream (and then whether it resumes the
    /// Claude session), refusing `busy` before any write (9.5, R5). A live stream is used as it
    /// is, resuming only on `unknown` (R5). A turn that opens one, or an `unknown` one, resumes
    /// unless its turn is `resume-lost` or its profile sets `persist: false` (nothing was kept).
    fn plan_turn(
        &mut self,
        rec: &Record,
        slots: &Slots,
        a: &Acct,
    ) -> Result<(bool, bool), KernelError> {
        let open = rec.backend == "claude" && self.live_provider(rec).is_none();
        if !open && rec.turn != Turn::Unknown {
            return Ok((false, false));
        }
        if open {
            // Every refusal comes before the host opens the stream.
            rec.stream_room()?;
            self.check_provider(slots, a)?;
        }
        let resume = rec.turn != Turn::ResumeLost
            && self
                .state
                .get(&format!("profile/{}", rec.profile))
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .is_none_or(|j| parse_profile(&j).0.persist != Some(false));
        Ok((open, resume))
    }

    /// The end of a call that writes: the record (when given) takes the change in its session keys,
    /// both accounts take the change in the charged bytes, each written once and only when it changed.
    /// A call that would take the account past the reserve fails `quota` (the host commits nothing, 7).
    fn finish(&mut self, rec: Option<&mut Record>, a: &mut Acct) -> Result<(), KernelError> {
        self.finish_as(rec, a, false)
    }

    /// `finish`; `cannot_fail` is a call 9.3 lets past the ceiling (it charges what it wrote).
    fn finish_as(
        &mut self,
        rec: Option<&mut Record>,
        a: &mut Acct,
        cannot_fail: bool,
    ) -> Result<(), KernelError> {
        if let Some(rec) = rec {
            self.seal(rec)?;
        }
        let c = self.state.close();
        a.owner.add(c.owner);
        a.account.add(c.owner);
        let ceiling = self
            .limits
            .account_bytes
            .saturating_add(self.limits.reserve_bytes);
        if !cannot_fail && c.owner > 0 && a.account.bytes > ceiling {
            return Err(KernelError::Quota(ACCOUNT_CAUSE.to_string()));
        }
        self.save_accounts(a)
    }

    /// Writes each account that changed.
    fn save_accounts(&mut self, a: &Acct) -> Result<(), KernelError> {
        if a.owner != a.owner0 {
            a.owner.save(&mut self.state, &a.owner_key)?;
        }
        if a.account != a.account0 {
            a.account.save(&mut self.state, Usage::ACCOUNT)?;
        }
        Ok(())
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
        past: Past,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        if rec.boot < slots.boot {
            self.step(rec, past, slots, work)?;
        }
        Ok(())
    }

    fn session(
        &mut self,
        past: Past,
        caller: &str,
        id: &str,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<Record, KernelError> {
        let mut rec = self.load_owned(caller, id)?;
        self.settle(&mut rec, past, slots, work)?;
        Ok(rec)
    }

    /// The restart step of one session (R4): its stale `stream/` and `live/` keys go, its boot
    /// (and its page entry's) becomes the current one, a Claude session's open and pending asks
    /// close in every turn state, and a Claude turn nothing drives becomes `unknown`. Writes: at most its page (or `slots` and
    /// the page when it leaves the index), a decision entry per open ask and a `turn` entry (R3).
    // An OpenAI `running` or `waiting` session only gets the stale keys and the boot here: its rows
    // of the step table (9.5: requests, retries, tool jobs) come with stage D.
    fn step(
        &mut self,
        rec: &mut Record,
        past: Past,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let now = self.host.now();
        rec.boot = slots.boot;
        let stale_streams = std::mem::take(&mut rec.streams);
        rec.live = None;
        let driven = rec.backend == "claude"
            && matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. });
        // Every open or pending Claude ask closes, whatever the turn state; a driven turn ends
        // `unknown`. An OpenAI session's asks are the kernel's own: its `waiting` row is unchanged.
        if rec.backend == "claude"
            && self.close_all_with(rec, past, "restart", now, work, !driven)?
        {
            rec.updated = now;
        }
        if driven {
            self.turn_to(rec, past, Turn::Unknown, now, work)?;
            rec.updated = now;
            self.sync_active(rec, slots, true)?;
        } else if let Some(p) = rec.active_page {
            active::set_boot(&mut self.state, p, &rec.id, slots.boot)?;
        }
        for s in stale_streams {
            self.state.delete(&format!("stream/{s}"));
        }
        self.state.delete(&format!("live/{}", rec.id));
        self.seal(rec)
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

    /// An entry the provider sends: stored unless the owner or the account is past its quota or the
    /// session is at its chunk cap, keeping the room for one `turn` entry (9.3 Quota, case 53). Not
    /// stored, the active turn ends `stopped {limit: quota}` (`None`).
    fn append_loose(
        &mut self,
        rec: &mut Record,
        past: Past,
        e: NewEntry,
        at: u64,
        work: &mut Work,
    ) -> Result<Option<u64>, KernelError> {
        if !past.is_past() {
            let mut meta = rec.meta();
            let stored = Transcript::new(&mut self.state, &rec.id, &mut meta).append_within(
                &e,
                at,
                MAX_CHUNKS - 1,
            );
            match stored {
                Ok(i) => {
                    rec.set_meta(&meta);
                    return Ok(Some(i));
                }
                Err(KernelError::Quota(_)) => {}
                Err(e) => return Err(e),
            }
        }
        self.stop_at_quota(rec, past, at, work)?;
        Ok(None)
    }

    /// An item not stored past its quota ends an active turn `stopped {limit: quota}` and sends
    /// `Request::Interrupt` (9.3 "Any call when past").
    fn stop_at_quota(
        &mut self,
        rec: &mut Record,
        past: Past,
        at: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        if matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }) {
            let stopped = Turn::Idle {
                stop: Stop::Stopped {
                    limit: "quota".to_string(),
                },
                seconds: None,
                tools: None,
                tokens: None,
            };
            self.turn_to(rec, past, stopped, at, work)?;
            work.requests.push(Request::Interrupt {
                session: rec.id.clone(),
            });
        }
        Ok(())
    }

    /// Stores a turn change: the record's turn and a `turn` entry, and pushes the turn. `activity`
    /// is live only (9.3): a change of it alone pushes and writes nothing.
    fn turn_to(
        &mut self,
        rec: &mut Record,
        past: Past,
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
        // Past its quota only a change that ends an active turn is written: one `turn` entry per
        // turn (9.3 Quota). The record and the push follow either way.
        if !past.is_past() || (rec.turn.is_active() && !stored.is_active()) {
            let entry = NewEntry {
                kind: "turn".into(),
                uuid: None,
                body: stored.entry_body(),
            };
            self.append(rec, entry, now)?;
        }
        rec.turn = stored;
        work.pushes.push(Push::of(&rec.id, turn_push(&t)));
        Ok(())
    }

    /// A user message, the `running` turn and the request: what a started turn is (AC1).
    /// Returns the message's index.
    fn start_turn(
        &mut self,
        rec: &mut Record,
        past: Past,
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
        self.turn_to(rec, past, running, now, work)?;
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
        past: Past,
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
        self.start_turn(rec, past, next.input, false, now, work)?;
        q.save(&mut self.state, &rec.id)?;
        work.pushes.push(Push::of(&rec.id, q.push_value()));
        Ok(())
    }

    /// A turn change from the provider (or the kernel): ending with inputs queued starts the next.
    fn change_turn(
        &mut self,
        rec: &mut Record,
        past: Past,
        t: Turn,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let before = rec.turn.is_active();
        // Past its quota only a change that ends an active turn is taken: one `turn` entry per
        // turn (9.3 Quota). An `activity` change alone is no change: `turn_to` pushes it.
        let mut bare = t.clone();
        if let Turn::Running { activity, .. } = &mut bare {
            *activity = None;
        }
        if past.is_past() && bare != rec.turn && !(before && !t.is_active()) {
            return Ok(());
        }
        self.turn_to(rec, past, t, now, work)?;
        // Past its quota, or at the chunk cap, a session starts no queued input (9.3 Quota).
        if before
            && matches!(rec.turn, Turn::Idle { .. })
            && !past.is_past()
            && Self::room(rec).is_ok()
        {
            self.next_queued(rec, past, now, work)?;
        }
        Ok(())
    }

    /// Takes an ask out of `open`; the turn follows when it was waiting on it.
    fn close_ask(
        &mut self,
        rec: &mut Record,
        past: Past,
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
            self.turn_to(rec, past, t, now, work)?;
        }
        Ok(())
    }

    /// Writes the one decision entry of the ask at `i`: its entry repeated, with `decision`; past
    /// its quota, counted at this write, the short form from the record alone (9.4).
    #[allow(clippy::too_many_arguments)]
    fn write_decision(
        &mut self,
        rec: &mut Record,
        past: Past,
        ask: (&str, &str),
        i: u64,
        decision: Value,
        now: u64,
    ) -> Result<(), KernelError> {
        let body = if past.short(self.state.change().owner) {
            json!({"id": ask.0, "kind": ask.1, "ask-i": i, "decision": decision})
        } else {
            let Some(mut body) = self.ask_body(rec, i) else {
                return Ok(());
            };
            let Some(obj) = body.as_object_mut() else {
                return Ok(());
            };
            obj.insert("decision".into(), decision);
            body
        };
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
    /// `follow` lets a `waiting` turn follow (R4 sets its own turn). Whether any closed.
    fn close_all_with(
        &mut self,
        rec: &mut Record,
        past: Past,
        reason: &str,
        now: u64,
        work: &mut Work,
        follow: bool,
    ) -> Result<bool, KernelError> {
        let any = !rec.open.is_empty() || !rec.pending_answers.is_empty();
        for o in std::mem::take(&mut rec.open) {
            rec.deadlines.retain(|d| d.what != format!("ask {}", o.id));
            let d = json!({"outcome": "deny", "by": "kernel", "reason": reason});
            self.write_decision(rec, past, (&o.id, &o.kind), o.i, d, now)?;
        }
        for (id, p) in std::mem::take(&mut rec.pending_answers) {
            self.write_decision(rec, past, (&id, &p.kind), p.i, p.decision(), now)?;
        }
        if any && follow && matches!(rec.turn, Turn::Waiting { .. }) {
            let t = Turn::Running {
                since: now,
                activity: None,
            };
            self.turn_to(rec, past, t, now, work)?;
        }
        Ok(any)
    }

    /// An `ask-resolved` the adapter checked (9.8 rule 4): the ask's one decision entry.
    fn resolve(
        &mut self,
        rec: &mut Record,
        past: Past,
        r: adapters::Resolved,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let open = rec.open.iter().find(|o| o.id == r.id).cloned();
        let pending = rec.pending_answers.get(&r.id).cloned();
        let (i, kind) = match (&open, &pending) {
            (Some(o), _) => (o.i, o.kind.clone()),
            (None, Some(p)) => (p.i, p.kind.clone()),
            (None, None) => return self.unknown_entry(rec, past, r.raw, now, work),
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
            return self.unknown_entry(rec, past, r.raw, now, work);
        }
        let decision = match (r.by.as_str(), outcome) {
            ("answered", Some(o)) => match &pending {
                Some(p) if p.outcome == o => p.decision(),
                _ => json!({"outcome": o, "by": if r.direct { "host" } else { "checked" }}),
            },
            _ => json!({"outcome": "deny", "by": "host", "reason": r.by}),
        };
        self.write_decision(rec, past, (&r.id, &kind), i, decision, now)?;
        if open.is_some() {
            self.close_ask(rec, past, &r.id, now, work)
        } else {
            rec.pending_answers.remove(&r.id);
            Ok(())
        }
    }

    fn unknown_entry(
        &mut self,
        rec: &mut Record,
        past: Past,
        raw: Value,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let e = adapters::unknown(raw).entry.expect("unknown has an entry");
        self.append_loose(rec, past, NewEntry::from(e), now, work)?;
        Ok(())
    }

    /// Moves the session in or out of `active/` as its end state asks (A2); one `slots` and one page.
    /// Whether it saved `slots`.
    fn sync_active(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        was: bool,
    ) -> Result<bool, KernelError> {
        let will = rec.turn.is_active();
        if !was && will {
            rec.active_page = Some(active::add(&mut self.state, slots, &rec.id, slots.boot)?);
            return Ok(true);
        } else if was && !will {
            if let Some(p) = rec.active_page.take() {
                return active::remove(&mut self.state, slots, &rec.id, p);
            }
        }
        Ok(false)
    }

    /// The end of a call that did not defer: its pushes go to the session's live owner stream, then
    /// its requests leave (after the last write, 7).
    fn flush(&mut self, work: Work, live: Option<&str>) -> Vec<Push> {
        self.deliver(&work.pushes, live);
        for r in work.requests {
            self.host.request(r);
        }
        work.pushes
    }

    /// The live owner stream of a session: `live/<session>` (9.1).
    fn live_stream(&mut self, session: &str) -> Option<String> {
        let bytes = self.state.get(&format!("live/{session}"))?;
        String::from_utf8(bytes).ok()
    }

    /// Pushes, in order, to the live stream of their session (`live`, from its record or its key).
    /// Whether the stream was full: it gets no more.
    fn send_live(&mut self, pushes: &[Push], live: Option<&str>) -> bool {
        let Some(o) = live else {
            return false;
        };
        pushes
            .iter()
            .any(|p| self.host.push(o, on_wire(&p.value)).is_err())
    }

    /// Outside an item call a full owner stream cannot be waited for: it is closed `lagging` (9.4).
    fn deliver(&mut self, pushes: &[Push], live: Option<&str>) {
        let (Some(o), Some(first)) = (live, pushes.first()) else {
            return;
        };
        if self.send_live(pushes, live) {
            self.drop_live(&first.session.clone(), o, "lagging");
        }
    }

    /// Closes the session's live owner stream and takes its keys and its place in `streams`, in one
    /// write of its own: its bytes go from the record and both accounts.
    fn drop_live(&mut self, session: &str, stream: &str, outcome: &str) {
        self.host.close_stream(stream, outcome);
        self.open_meter();
        self.state.delete(&stream_key(stream));
        self.state.delete(&format!("live/{session}"));
        match Record::load(&mut self.state, session) {
            Some(mut rec) => {
                rec.streams.retain(|s| s != stream);
                rec.live = None;
                let provider = self.live_provider(&rec);
                let mut acct = self.accounts(&rec.owner);
                let _ = self.finish(Some(&mut rec), &mut acct);
                // The host's `ended` for the closed stream finds no key: resume here (3).
                if let Some(t) = provider {
                    self.host.resume_stream(&t);
                }
            }
            None => {
                self.state.close();
            }
        }
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
            return Err(KernelError::Quota(format!(
                "{MAX_CHUNKS} transcript chunks per session"
            )));
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
        self.guarded(|k| k.start_call(caller, key, profile, overlay, input))
    }

    fn start_call(
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
        let mut acct = self.accounts(caller);
        let past = acct.past;
        if acct.owner.sessions >= self.limits.owner_sessions {
            return Err(KernelError::Quota(format!(
                "{} sessions per owner",
                self.limits.owner_sessions
            )));
        }
        self.refuse_past(&acct)?;
        if backend == Backend::Claude {
            self.check_provider(&slots, &acct)?;
        }
        // 9.1: the first page with room, or the next one.
        let n = acct
            .owner
            .pages
            .iter()
            .position(|&c| (c as usize) < OWNER_PAGE)
            .unwrap_or(acct.owner.pages.len());
        let page_key = owner_page_key(caller, n as u32);
        let mut page: Vec<String> = if acct.owner.pages.get(n).is_some_and(|&c| c > 0) {
            self.state
                .get(&page_key)
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
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
            owner_page: n as u32,
            ..Record::default()
        };
        let mut work = Work::default();
        self.start_turn(&mut rec, past, input.to_string(), false, now, &mut work)?;
        if backend == Backend::Claude {
            self.open_provider(&mut rec, &mut slots, &mut acct)?;
        }
        // The first turn is active: this saves `slots` with the slot taken.
        self.sync_active(&mut rec, &mut slots, false)?;
        page.push(id.clone());
        self.state
            .put(
                &page_key,
                serde_json::to_vec(&page).expect("a page serializes"),
            )
            .map_err(|t| KernelError::TooLarge(t.0 as u64))?;
        self.seal(&mut rec)?;
        self.key_put("start", caller, key, None, &id)?;
        if acct.owner.pages.len() <= n {
            acct.owner.pages.resize(n + 1, 0);
        }
        acct.owner.pages[n] += 1;
        acct.owner.sessions += 1;
        acct.account.sessions += 1;
        self.finish(None, &mut acct)?;
        Ok(Called {
            value: id,
            pushes: self.flush(work, None),
        })
    }

    pub fn say(
        &mut self,
        caller: &str,
        session: &str,
        key: &str,
        input: &str,
    ) -> Result<Called<u64>, KernelError> {
        self.guarded(|k| k.say_call(caller, session, key, input))
    }

    fn say_call(
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
        let mut acct = self.accounts(caller);
        let past = acct.past;
        self.refuse_past(&acct)?;
        let mut work = Work::default();
        self.settle(&mut rec, past, &mut slots, &mut work)?;
        let before = rec.deadlines.clone();
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
                // the end and the oldest starts the turn. A turn with no live provider stream
                // opens one (9.5), and resumes the Claude session it had (9.4).
                let (open, resume) = self.plan_turn(&rec, &slots, &acct)?;
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
                    None => {
                        self.start_turn(&mut rec, past, input.to_string(), resume, now, &mut work)?
                    }
                    Some((oldest, n)) => {
                        self.start_turn(&mut rec, past, oldest.input, resume, now, &mut work)?;
                        q.save(&mut self.state, &rec.id)?;
                        work.pushes.push(Push::of(&rec.id, q.push_value()));
                        n
                    }
                };
                if open {
                    self.open_provider(&mut rec, &mut slots, &mut acct)?;
                }
                let timed = self.idle_after(&mut rec, &mut slots, &mut acct, was, &before, now);
                if !self.sync_active(&mut rec, &mut slots, was)? && (open || timed) {
                    slots.save(&mut self.state)?;
                }
                rec.updated = now;
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
        self.seal(&mut rec)?;
        self.key_put("say", caller, key, Some(session), &n)?;
        self.finish(None, &mut acct)?;
        Ok(Called {
            value: n,
            pushes: self.flush(work, rec.live.as_deref()),
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
        let mut acct = self.accounts(caller);
        let past = acct.past;
        if d.for_session {
            self.refuse_past(&acct)?;
        }
        // The step comes after the refusals that do not depend on it; an ask it denies is refused.
        let mut work = Work::default();
        self.settle(&mut rec, past, &mut slots, &mut work)?;
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
        self.close_ask(&mut rec, past, ask, now, &mut work)?;
        rec.pending_answers.insert(ask.to_string(), pending);
        work.requests.push(Request::Answer {
            session: rec.id.clone(),
            ask: ask.to_string(),
            decision: d.value,
        });
        rec.updated = now;
        self.seal(&mut rec)?;
        self.key_put("answer", caller, key, Some(session), &())?;
        self.finish(None, &mut acct)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work, rec.live.as_deref()),
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
        self.guarded(|k| k.interrupt_call(caller, session, replacement))
    }

    fn interrupt_call(
        &mut self,
        caller: &str,
        session: &str,
        replacement: Option<&str>,
    ) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut rec = self.load_owned(caller, session)?;
        let mut acct = self.accounts(caller);
        let past = acct.past;
        if let Some(input) = replacement {
            check_input(input)?;
            if !self.may_use(caller, &rec.profile) {
                return Err(Self::revoked());
            }
            self.refuse_past(&acct)?;
        }
        let mut work = Work::default();
        self.settle(&mut rec, past, &mut slots, &mut work)?;
        let was = rec.turn.is_active();
        let before = rec.deadlines.clone();
        if replacement.is_none() && !was {
            // 9.3 "Past": nothing to end, but the restart step it ran is charged, never refused.
            self.finish_as(None, &mut acct, true)?;
            return Ok(Called {
                value: (),
                pushes: self.flush(work, rec.live.as_deref()),
            });
        }
        let mut plan = (false, false);
        if replacement.is_some() {
            Self::room(&rec)?;
            if !was {
                active::check_room(&slots)?;
            }
            plan = self.plan_turn(&rec, &slots, &acct)?;
        }
        let now = self.host.now();
        if was {
            self.end_interrupted(&mut rec, past, now, &mut work)?;
            work.requests.push(Request::Interrupt {
                session: rec.id.clone(),
            });
        }
        let (open, resume) = plan;
        if let Some(input) = replacement {
            self.start_turn(&mut rec, past, input.to_string(), resume, now, &mut work)?;
        }
        if open {
            self.open_provider(&mut rec, &mut slots, &mut acct)?;
        }
        let timed = self.idle_after(&mut rec, &mut slots, &mut acct, was, &before, now);
        if !self.sync_active(&mut rec, &mut slots, was)? && (open || timed) {
            slots.save(&mut self.state)?;
        }
        rec.updated = now;
        // 9.3 "Past": an interrupt without a replacement only ends the turn.
        self.finish_as(Some(&mut rec), &mut acct, replacement.is_none())?;
        Ok(Called {
            value: (),
            pushes: self.flush(work, rec.live.as_deref()),
        })
    }

    fn end_interrupted(
        &mut self,
        rec: &mut Record,
        past: Past,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let idle = Turn::Idle {
            stop: Stop::Interrupted,
            seconds: None,
            tools: None,
            tokens: None,
        };
        self.turn_to(rec, past, idle, now, work)
    }

    /// 2.3 `cancel`: every open or pending ask gets its one decision, an active turn ends
    /// `idle {interrupted}`, the queue is kept, the host ends the stream.
    // Children (9.3) are not built: nothing to cancel there yet.
    pub fn cancel(&mut self, caller: &str, session: &str) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let past = acct.past;
        let mut work = Work::default();
        let mut rec = self.session(past, caller, session, &mut slots, &mut work)?;
        let now = self.host.now();
        let was = rec.turn.is_active();
        let closed = self.close_all_with(&mut rec, past, "cancel", now, &mut work, false)?;
        if was {
            self.end_interrupted(&mut rec, past, now, &mut work)?;
        }
        // The host's `Cancel` ends the provider stream: its slot is free at once, and its later
        // `ended` finds no key (9.4).
        let mut freed = match self.live_provider(&rec) {
            Some(t) => self.release_provider(&mut rec, &t, &mut slots, &mut acct),
            None => false,
        };
        // 9.3: no deadline outlives a cancel; its timer goes with them.
        let before = std::mem::take(&mut rec.deadlines);
        freed |= self.sync_timer(&rec, &mut slots, &mut acct, &before, now, false);
        if !self.sync_active(&mut rec, &mut slots, was)? && freed {
            slots.save(&mut self.state)?;
        }
        work.requests.push(Request::Cancel {
            session: rec.id.clone(),
        });
        if was || closed {
            rec.updated = now;
        }
        // 9.3 "Past": a cancel closes the session's work, so it cannot fail on quota.
        self.finish_as(Some(&mut rec), &mut acct, true)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work, rec.live.as_deref()),
        })
    }

    /// `session` for a call that writes only when the restart step runs (R6): the step is charged.
    fn read_session(
        &mut self,
        caller: &str,
        id: &str,
        slots: &mut Slots,
    ) -> Result<Record, KernelError> {
        let mut rec = self.load_owned(caller, id)?;
        if rec.boot < slots.boot {
            let mut acct = self.accounts(caller);
            let past = acct.past;
            self.settle(&mut rec, past, slots, &mut Work::default())?;
            self.finish(None, &mut acct)?;
        }
        Ok(rec)
    }

    pub fn queued(&mut self, caller: &str, session: &str) -> Result<Vec<Queued>, KernelError> {
        let mut slots = self.begin()?;
        let rec = self.read_session(caller, session, &mut slots)?;
        Ok(Queue::load(&mut self.state, &rec.id).inputs)
    }

    pub fn unqueue(
        &mut self,
        caller: &str,
        session: &str,
        n: u64,
    ) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let past = acct.past;
        let mut work = Work::default();
        let mut rec = self.session(past, caller, session, &mut slots, &mut work)?;
        let mut q = Queue::load(&mut self.state, &rec.id);
        q.remove(n)?;
        q.save(&mut self.state, &rec.id)?;
        work.pushes.push(Push::of(&rec.id, q.push_value()));
        self.finish(Some(&mut rec), &mut acct)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work, rec.live.as_deref()),
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
        let rec = self.read_session(caller, session, &mut slots)?;
        let mut meta = rec.meta();
        Transcript::new(&mut self.state, &rec.id, &mut meta).read(after, before, max)
    }

    pub fn rename(&mut self, caller: &str, session: &str, title: &str) -> Result<(), KernelError> {
        check_title(title)?;
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let past = acct.past;
        let mut rec = self.load_owned(caller, session)?;
        self.refuse_past(&acct)?;
        self.settle(&mut rec, past, &mut slots, &mut Work::default())?;
        rec.title = title.to_string();
        self.finish(Some(&mut rec), &mut acct)
    }

    /// 2.3 `delete`: frees the session's bytes, as its record holds them, without reading its
    /// chunks; the kept `key/` results stay charged (9.3 Quota).
    pub fn delete(&mut self, caller: &str, session: &str) -> Result<(), KernelError> {
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        // No restart step (9.3): a session can always be deleted.
        let rec = self.load_owned(caller, session)?;
        // The record's bytes.
        let freed = rec.bytes as i64;
        let page_key = owner_page_key(caller, rec.owner_page);
        let page: Vec<String> = self
            .state
            .get(&page_key)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        // The slots of this boot's provider streams; a stale record's streams were counted zero.
        let mut slot_freed = false;
        let mut owners = Vec::new();
        if rec.boot == slots.boot {
            for s in &rec.streams {
                let bytes = self.state.get(&stream_key(s));
                let kind = bytes.and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok());
                match kind.map(|k| k.kind) {
                    Some(k) if k == PROVIDER => {
                        slots.provider = slots.provider.saturating_sub(1);
                        acct.owner.close_provider(slots.boot);
                        slot_freed = true;
                    }
                    Some(k) if k == OWNER => owners.push(s.clone()),
                    _ => {}
                }
            }
        }
        // 9.3: a record that held a deadline held its timer.
        if !rec.deadlines.is_empty() {
            self.host.timer_cancel(&format!("s/{}", rec.id));
            slots.timers = slots.timers.saturating_sub(1);
            acct.owner.timers = acct.owner.timers.saturating_sub(1);
            slot_freed = true;
        }
        let mut saved = false;
        if let Some(p) = rec.active_page {
            saved = active::remove(&mut self.state, &mut slots, &rec.id, p)?;
        }
        if slot_freed && !saved {
            slots.save(&mut self.state)?;
        }
        for n in 0..rec.chunks.len() {
            self.state
                .delete_uncharged(&crate::transcript::chunk_key(&rec.id, n));
        }
        self.state.delete_uncharged(&Queue::key_of(&rec.id));
        for s in &rec.streams {
            self.state.delete_uncharged(&stream_key(s));
        }
        self.state.delete_uncharged(&format!("live/{}", rec.id));
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
        self.state.adjust(Change {
            owner: -freed,
            session: 0,
        });
        acct.owner.sessions = acct.owner.sessions.saturating_sub(1);
        acct.account.sessions = acct.account.sessions.saturating_sub(1);
        if let Some(c) = acct.owner.pages.get_mut(rec.owner_page as usize) {
            *c = c.saturating_sub(1);
        }
        self.finish(None, &mut acct)?;
        // After the last write, as `cancel` does: the host ends the stream and closes the asks.
        for o in owners {
            self.host.close_stream(&o, "closed");
        }
        self.host.request(Request::Cancel {
            session: rec.id.clone(),
        });
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
        let pages = Usage::load(&mut self.state, &Usage::owner_key(caller)).pages;
        let mut ids: Vec<String> = Vec::new();
        let mut found = after.is_none();
        // The pages in order; one that cannot hold the `after` id or what follows is read only to look for it.
        for (n, _) in pages.iter().enumerate().filter(|(_, c)| **c > 0) {
            if found && ids.len() >= max.min(256) as usize {
                break;
            }
            let mut page: Vec<String> = self
                .state
                .get(&owner_page_key(caller, n as u32))
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            if !found {
                match page.iter().position(|i| Some(i.as_str()) == after) {
                    Some(p) => {
                        found = true;
                        page.drain(..=p);
                    }
                    None => continue,
                }
            }
            ids.extend(page);
        }
        let mut out = Vec::new();
        for id in ids.iter() {
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

    /// One item of a provider's stream, by its id: `stream/<id>` is read first and names the
    /// session (9.4). An id with no key (never opened or ended) changes nothing and sends
    /// nothing; an id of an older boot is no different to the code, and reaches it only if the
    /// host breaks R1 (items come on streams of this boot). When the session's live owner stream is full, the call is
    /// dropped whole and answers `Defer` (3, "Deferred batch").
    pub fn provider_item(
        &mut self,
        stream: &str,
        event: ProviderEvent,
    ) -> Result<Called<Item>, KernelError> {
        self.open_meter();
        let key = self
            .state
            .get(&stream_key(stream))
            .and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok());
        // An owner stream's id is no provider item (9.1 `stream/<id>`).
        let Some(key) = key.filter(|k| k.kind == PROVIDER) else {
            return Ok(Called {
                value: Item::Done,
                pushes: Vec::new(),
            });
        };
        // 9.4, P09: an item that only forwards a delta reads `stream/<id>` and `live/<session>`.
        if let ProviderEvent::Out(out) = &event {
            if let Some(update) = delta_only(out) {
                let now = self.host.now();
                let push = Push::of(&key.session, wire_update(update, now, None));
                let pushes = vec![push];
                let live = self.live_stream(&key.session);
                if self.send_live(&pushes, live.as_deref()) {
                    return Ok(Called {
                        value: Item::Defer,
                        pushes: Vec::new(),
                    });
                }
                return Ok(Called {
                    value: Item::Done,
                    pushes,
                });
            }
        }
        let rec = Record::load(&mut self.state, &key.session)
            .ok_or_else(|| KernelError::UnknownSession(key.session.clone()))?;
        self.provider_on(rec, Some(stream), event, true)
    }

    /// One item of a provider's stream for a session, for tests: it goes by the session's live
    /// provider stream, or, with none, acts on the session directly. It cannot defer.
    pub fn provider(
        &mut self,
        session: &str,
        event: ProviderEvent,
    ) -> Result<Called<()>, KernelError> {
        self.open_meter();
        let rec = Record::load(&mut self.state, session)
            .ok_or_else(|| KernelError::UnknownSession(session.to_string()))?;
        let stream = self.live_provider(&rec);
        let c = self.provider_on(rec, stream.as_deref(), event, false)?;
        Ok(Called {
            value: (),
            pushes: c.pushes,
        })
    }

    /// 9.4 `attach`: the entries after `after`, then `synced`, the turn and the queue, go out on a
    /// new owner stream the kernel registers as the session's live one; an earlier one ends
    /// `replaced`. More than the replay window after `after` opens nothing (`Behind`).
    pub fn attach(
        &mut self,
        caller: &str,
        session: &str,
        after: u64,
    ) -> Result<Called<Attached>, KernelError> {
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let past = acct.past;
        let mut rec = self.load_owned(caller, session)?;
        let stale = rec.boot < slots.boot;
        let mut work = Work::default();
        self.settle(&mut rec, past, &mut slots, &mut work)?;
        let mut meta = rec.meta();
        let replay = Transcript::new(&mut self.state, &rec.id, &mut meta).replay(after);
        let lines = match replay {
            Replay::Entries(lines) => lines,
            Replay::Behind { through } => {
                // Only the restart step, when it ran, wrote anything.
                if stale {
                    self.finish(None, &mut acct)?;
                }
                return Ok(Called {
                    value: Attached::Behind { through },
                    pushes: self.flush(work, rec.live.as_deref()),
                });
            }
        };
        let previous = self.live_stream(&rec.id);
        match &previous {
            Some(p) if rec.streams.contains(p) => {}
            _ => rec.stream_room()?,
        }
        let queue = Queue::load(&mut self.state, &rec.id).push_value();
        let o = self.host.open_stream();
        let written = self.write_owner(&mut rec, &mut acct, &o, previous.as_deref());
        if let Err(e) = written {
            self.host.close_stream(&o, "closed");
            return Err(e);
        }
        if let Some(p) = &previous {
            self.host.close_stream(p, "replaced");
            // The key is gone, so the host's `ended` resumes nothing: resume here (3).
            if let Some(t) = self.live_provider(&rec) {
                self.host.resume_stream(&t);
            }
        }
        let mut items: Vec<Value> = lines
            .iter()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .map(|e| json!({"_meta": {"watane": {"type": "entry", "entry": e}}}))
            .collect();
        items.push(json!({"_meta": {"watane": {"type": "synced", "through": meta.last}}}));
        items.push(json!({"_meta": {"watane": turn_push(&rec.turn)}}));
        items.push(json!({"_meta": {"watane": queue}}));
        for v in items {
            if self.host.push(&o, v).is_err() {
                self.drop_live(&rec.id, &o, "lagging");
                break;
            }
        }
        // The restart step's requests, when it ran, leave after the last write (7).
        Ok(Called {
            value: Attached::Live { stream: o },
            pushes: self.flush(work, None),
        })
    }

    /// The writes of an `attach`: the new stream's keys and place in `streams`, and the end of the
    /// one it replaces (its key and its place), in the same commit (9.3, `streams`).
    fn write_owner(
        &mut self,
        rec: &mut Record,
        acct: &mut Acct,
        stream: &str,
        previous: Option<&str>,
    ) -> Result<(), KernelError> {
        if let Some(p) = previous {
            self.state.delete(&stream_key(p));
            rec.streams.retain(|s| s != p);
        }
        let key = StreamKey {
            session: rec.id.clone(),
            kind: OWNER.to_string(),
        };
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        let too_large = |e: crate::state::TooLarge| KernelError::TooLarge(e.0 as u64);
        self.state
            .put(&stream_key(stream), bytes)
            .map_err(too_large)?;
        self.state
            .put(&format!("live/{}", rec.id), stream.as_bytes().to_vec())
            .map_err(too_large)?;
        rec.add_stream(stream.to_string())?;
        rec.live = Some(stream.to_string());
        self.finish(Some(rec), acct)
    }

    /// The session of an owner stream, by `stream/<id>`; `None` for any other id or kind.
    fn owner_stream(&mut self, stream: &str) -> Option<String> {
        let bytes = self.state.get(&stream_key(stream))?;
        serde_json::from_slice::<StreamKey>(&bytes)
            .ok()
            .filter(|k| k.kind == OWNER)
            .map(|k| k.session)
    }

    /// `stream-producer.ready` of an owner stream: the session's provider stream, deferred or not,
    /// is resumed (3, "Resume"). The kernel keeps no record of a deferral.
    // Nothing maps a session to its provider stream but its record, so this reads that one record.
    pub fn stream_ready(&mut self, stream: &str) -> Called<()> {
        self.open_meter();
        if let Some(session) = self.owner_stream(stream) {
            if let Some(rec) = Record::load(&mut self.state, &session) {
                if let Some(t) = self.live_provider(&rec) {
                    self.host.resume_stream(&t);
                }
            }
        }
        Called {
            value: (),
            pushes: Vec::new(),
        }
    }

    /// `stream-producer.ended` of an owner stream: while it is still the session's live stream its
    /// keys and place in `streams` go, and the provider stream is resumed; later entries are stored
    /// and not pushed. A `replaced` stream has no key: nothing changes (3, "Forwarding").
    pub fn stream_ended(&mut self, stream: &str) -> Called<()> {
        self.open_meter();
        let done = Called {
            value: (),
            pushes: Vec::new(),
        };
        let Some(session) = self.owner_stream(stream) else {
            return done;
        };
        if self.live_stream(&session).as_deref() != Some(stream) {
            return done;
        }
        self.state.delete(&stream_key(stream));
        self.state.delete(&format!("live/{session}"));
        let mut provider = None;
        match Record::load(&mut self.state, &session) {
            Some(mut rec) => {
                rec.streams.retain(|s| s != stream);
                rec.live = None;
                provider = self.live_provider(&rec);
                let mut acct = self.accounts(&rec.owner);
                let _ = self.finish(Some(&mut rec), &mut acct);
            }
            None => {
                self.state.close();
            }
        }
        if let Some(t) = provider {
            self.host.resume_stream(&t);
        }
        done
    }

    /// `item`: the call may be dropped whole when a push fails `Full` (3); its writes and host
    /// calls are held until it commits.
    fn provider_on(
        &mut self,
        rec: Record,
        stream: Option<&str>,
        event: ProviderEvent,
        item: bool,
    ) -> Result<Called<Item>, KernelError> {
        if item {
            self.state.hold();
            self.held = Some(Vec::new());
        }
        let work = match self.provider_work(rec, stream, event) {
            Ok(w) => w,
            Err(e) => {
                self.drop_held();
                return Err(e);
            }
        };
        let live = work.live.clone();
        if !item {
            return Ok(Called {
                value: Item::Done,
                pushes: self.flush(work, live.as_deref()),
            });
        }
        if self.send_live(&work.pushes, live.as_deref()) {
            self.drop_held();
            return Ok(Called {
                value: Item::Defer,
                pushes: Vec::new(),
            });
        }
        self.commit_held();
        for r in work.requests {
            self.host.request(r);
        }
        Ok(Called {
            value: Item::Done,
            pushes: work.pushes,
        })
    }

    /// The item's work, on the session it names; `stream` is the provider stream it came on.
    /// Everything is written; the pushes and requests are the caller's.
    fn provider_work(
        &mut self,
        mut rec: Record,
        stream: Option<&str>,
        event: ProviderEvent,
    ) -> Result<Work, KernelError> {
        let mut acct = self.accounts(&rec.owner);
        let past = acct.past;
        let before = rec.clone();
        let was = rec.turn.is_active();
        let ended = matches!(event, ProviderEvent::Ended)
            .then_some(stream)
            .flatten();
        let mut slots = None;
        if (!was && may_activate(&event)) || ended.is_some() {
            let s = Slots::load(&mut self.state);
            if !was && may_activate(&event) {
                active::check_room(&s)?;
            }
            slots = Some(s);
        }
        let now = self.host.now();
        let mut work = Work::default();
        match event {
            ProviderEvent::Turn(t) => self.change_turn(&mut rec, past, t, now, &mut work)?,
            ProviderEvent::Ended => {
                self.close_all_with(&mut rec, past, "ended", now, &mut work, false)?;
                if matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }) {
                    let failed = Turn::Idle {
                        stop: Stop::Failed {
                            error: "provider-ended".into(),
                        },
                        seconds: None,
                        tools: None,
                        tokens: None,
                    };
                    self.turn_to(&mut rec, past, failed, now, &mut work)?;
                }
            }
            ProviderEvent::Out(out) => {
                if let Some(e) = out.entry {
                    self.out_entry(&mut rec, past, e, now, &mut work)?;
                }
                if let Some(u) = out.update {
                    let push = Push::of(&rec.id, wire_update(u, now, work.stored));
                    work.pushes.push(push);
                }
                if let Some(r) = out.resolved {
                    self.resolve(&mut rec, past, r, now, &mut work)?;
                }
                if out.idle {
                    let idle = Turn::Idle {
                        stop: Stop::Completed,
                        seconds: None,
                        tools: None,
                        tokens: None,
                    };
                    self.change_turn(&mut rec, past, idle, now, &mut work)?;
                }
            }
        }
        // 9.4: the stream's `ended` frees its slot and keys.
        let mut freed = false;
        if let (Some(t), Some(s)) = (ended, slots.as_mut()) {
            freed = self.release_provider(&mut rec, t, s, &mut acct);
        }
        let touch = if rec.turn.is_active() {
            rec.deadlines.iter().any(|d| d.what == IDLE)
        } else {
            was && matches!(rec.turn, Turn::Idle { .. })
        };
        if touch {
            let mut s = slots.take().unwrap_or_else(|| Slots::load(&mut self.state));
            freed |= self.idle_after(&mut rec, &mut s, &mut acct, was, &before.deadlines, now);
            slots = Some(s);
        }
        let mut saved = false;
        if was != rec.turn.is_active() {
            let mut s = match slots.take() {
                Some(s) => s,
                None => Slots::load(&mut self.state),
            };
            saved = self.sync_active(&mut rec, &mut s, was)?;
            slots = Some(s);
        }
        if freed && !saved {
            if let Some(s) = &slots {
                s.save(&mut self.state)?;
            }
        }
        let changed = rec != before;
        if changed {
            rec.updated = now;
        }
        // A stream's `ended`, and an item while past, cannot fail on quota (9.3 "Past"): their
        // writes are bounded (R4).
        self.finish_as(
            changed.then_some(&mut rec),
            &mut acct,
            ended.is_some() || past.is_past(),
        )?;
        work.live = rec.live.clone();
        Ok(work)
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
        let Some(i) = at else {
            return false;
        };
        // Fail closed: an ask on a call whose earlier entry cannot be read is not shown as an ask.
        let Some(other) = self.ask_body(rec, i) else {
            return true;
        };
        other.pointer("/toolCall/title") != body.pointer("/toolCall/title")
            || other.get("input") != body.get("input")
    }

    /// An entry from the provider; an `ask` opens (or, with a decision, closes) an ask.
    fn out_entry(
        &mut self,
        rec: &mut Record,
        past: Past,
        e: adapters::Entry,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let kind = e.kind;
        let is_ask = kind == Kind::Ask;
        let body = e.body.clone();
        let decided = is_ask && body.get("decision").is_some_and(|d| !d.is_null());
        if is_ask && !decided && self.reused(rec, &body) {
            return self.unknown_entry(rec, past, body, now, work);
        }
        let id = body.get("id").and_then(Value::as_str);
        // An ask decision is stored as ever; any other entry is not past the quota (9.3).
        let stored = if decided && past.short(self.state.change().owner) {
            // Past its quota the decision is stored short when the ask is open or pending, else not.
            let at = rec
                .open
                .iter()
                .map(|o| (&o.id, &o.kind, o.i))
                .chain(rec.pending_answers.iter().map(|(id, p)| (id, &p.kind, p.i)))
                .find(|(oid, _, _)| Some(oid.as_str()) == id)
                .map(|(oid, k, i)| (oid.clone(), k.clone(), i));
            let Some((oid, k, ai)) = at else {
                // Not stored: an active turn ends as for any item not stored; no deny, as the
                // adapter's entry already carries its decision.
                return self.stop_at_quota(rec, past, now, work);
            };
            // Built from a fixed shape, so its size is bounded (9.3, 9.4).
            let Some(d) = short_decision(body.get("decision")) else {
                return self.stop_at_quota(rec, past, now, work);
            };
            let short = NewEntry {
                kind: "ask".into(),
                uuid: None,
                body: json!({"id": oid, "kind": k, "ask-i": ai, "decision": d}),
            };
            Some(self.append(rec, short, now)?)
        } else if decided {
            Some(self.append(rec, NewEntry::from(e), now)?)
        } else {
            self.append_loose(rec, past, NewEntry::from(e), now, work)?
        };
        if let (Some(i), Kind::Message | Kind::Tool) = (stored, kind) {
            work.stored = Some(i);
        }
        let Some(i) = stored else {
            // A new ask that is not stored is denied at once.
            if let (true, Some(id)) = (is_ask, id) {
                work.requests.push(Request::Answer {
                    session: rec.id.clone(),
                    ask: id.to_string(),
                    decision: json!({"deny": true, "reason": "quota"}),
                });
            }
            return Ok(());
        };
        let (true, Some(id)) = (is_ask, id) else {
            return Ok(());
        };
        if body.get("decision").is_some_and(|d| !d.is_null()) {
            return self.close_ask(rec, past, id, now, work);
        }
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("ask");
        if rec.open.len() + rec.pending_answers.len() >= OPEN_ASKS {
            // Past the cap: denied at once, so a closing call writes at most 64 decisions.
            let d = json!({"outcome": "deny", "by": "kernel", "reason": "busy"});
            self.write_decision(rec, past, (id, kind), i, d, now)?;
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
            self.turn_to(rec, past, waiting(asks, expires), now, work)?;
        }
        Ok(())
    }

    /// `timer-sink.fired`: `restart` is R2, `s/<id>` a session's deadlines (9.3).
    pub fn timer(&mut self, id: &str) -> Called<()> {
        let mut work = Work::default();
        if let Ok(mut slots) = self.begin() {
            if id == RESTART {
                // A firing that still fails (its page or `slots` cannot be written) re-arms nothing.
                let _ = self.restart_firing(&mut slots, &mut work);
            } else if let Some(session) = id.strip_prefix("s/") {
                // A firing is never an error: one that fails changes nothing more.
                let _ = self.session_firing(session, &mut slots, &mut work);
            }
        }
        Called {
            value: (),
            pushes: work.pushes,
        }
    }

    /// One firing of a session's timer (9.3): every due deadline is handled (`idle` releases the
    /// provider stream, the others wait for their stages), then the timer follows the rest.
    fn session_firing(
        &mut self,
        id: &str,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let Some(mut rec) = Record::load(&mut self.state, id) else {
            return Ok(());
        };
        let mut acct = self.accounts(&rec.owner);
        let past = acct.past;
        self.settle(&mut rec, past, slots, work)?;
        let now = self.host.now();
        if !rec.deadlines.iter().any(|d| d.at <= now) {
            return Ok(());
        }
        let before = rec.deadlines.clone();
        if rec.deadlines.iter().any(|d| d.at <= now && d.what == IDLE) {
            if let Some(t) = self.live_provider(&rec) {
                self.cancel_provider(&t);
                self.release_provider(&mut rec, &t, slots, &mut acct);
            }
            rec.deadlines.retain(|d| !(d.at <= now && d.what == IDLE));
        }
        // Due deadlines of later stages stay (and keep the count); the timer follows the rest.
        self.sync_timer(&rec, slots, &mut acct, &before, now, true);
        slots.save(&mut self.state)?;
        self.finish(Some(&mut rec), &mut acct)
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
                let mut acct = self.accounts(&rec.owner);
                let past = acct.past;
                let stepped = self.step(&mut rec, past, slots, &mut done).is_ok();
                if stepped {
                    work.pushes.append(&mut done.pushes);
                }
                // The firing commits what the step wrote, failed or not: the stored record's
                // `bytes` and both accounts take it, past the reserve too (a firing cannot fail).
                let lost = self.state.change().session - self.sealed;
                if !stepped && lost != 0 {
                    if let Some(mut held) = Record::load(&mut self.state, &entry.id) {
                        held.bytes = add_signed(held.bytes, lost);
                        let _ = held.save(&mut self.state);
                    }
                }
                let c = self.state.close();
                acct.owner.add(c.owner);
                acct.account.add(c.owner);
                let _ = self.save_accounts(&acct);
                if !stepped && rec.active_page.is_some() {
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

/// The short form of an adapter's decision: `outcome`, `by` and `reason` (cut at 256 bytes) only.
/// `None` when `outcome` or `by` is not one of the known values, or `reason` is not a string.
fn short_decision(d: Option<&Value>) -> Option<Value> {
    let d = d?.as_object()?;
    let outcome = d.get("outcome")?.as_str()?;
    let by = d.get("by")?.as_str()?;
    if !["allow", "deny", "none"].contains(&outcome)
        || !["owner", "session-allow", "checked", "kernel", "host"].contains(&by)
    {
        return None;
    }
    let mut out = json!({"outcome": outcome, "by": by});
    match d.get("reason") {
        None | Some(Value::Null) => {}
        Some(Value::String(r)) => {
            let mut end = r.len().min(256);
            while !r.is_char_boundary(end) {
                end -= 1;
            }
            out["reason"] = Value::String(r[..end].to_string());
        }
        Some(_) => return None,
    }
    Some(out)
}
