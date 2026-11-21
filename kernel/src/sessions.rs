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
use crate::quota::{segment, Change, Limits, Metered, Usage};
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
    /// Opens a provider stream for a Claude session (3): its id, as the host's items will carry it.
    fn open_stream(&mut self) -> String;
    /// The consumer's `cancel` of a stream the kernel holds.
    fn cancel_stream(&mut self, stream: &str);
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
}

/// A host for tests: a settable clock and boot, ids `s1`, `s2`, ..., and a log of what was asked.
#[derive(Debug, Default)]
pub struct FakeHost {
    pub boot: u64,
    pub now: u64,
    pub next_id: u64,
    pub next_stream: u64,
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
}

/// Pushes and provider requests a call collects; requests leave only after the last write.
#[derive(Default)]
struct Work {
    pushes: Vec<Push>,
    requests: Vec<Request>,
    /// The owner or the account was past its quota when the call began (9.3 Quota).
    past: bool,
}

/// The owner's account and the kernel's, as read for a call and kept for its one write (9.3 Quota).
struct Acct {
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

fn stream_key(id: &str) -> String {
    format!("stream/{id}")
}

fn key_of(owner: &str, key: &str) -> String {
    format!("key/{}/{key}", segment(owner))
}

fn owner_page_key(owner: &str, n: u32) -> String {
    format!("owner/{}/{n}", segment(owner))
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
        let owner = Usage::load(&mut self.state, &owner_key);
        let account = Usage::load(&mut self.state, Usage::ACCOUNT);
        Acct {
            owner_key,
            owner0: owner.clone(),
            account0: account.clone(),
            owner,
            account,
        }
    }

    /// 9.3 "Past": the owner's bytes or the kernel's account at or over their caps.
    fn past(&self, a: &Acct) -> bool {
        a.owner.bytes >= self.limits.owner_bytes || a.account.bytes >= self.limits.account_bytes
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
        if c.owner > 0 && a.account.bytes > ceiling {
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
    /// close in every turn state, and a Claude turn nothing drives becomes `unknown`. Writes: at most its page (or `slots` and
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
        e: NewEntry,
        at: u64,
        work: &mut Work,
    ) -> Result<Option<u64>, KernelError> {
        if !work.past {
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
        if matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }) {
            let stopped = Turn::Idle {
                stop: Stop::Stopped {
                    limit: "quota".to_string(),
                },
                seconds: None,
                tools: None,
                tokens: None,
            };
            self.turn_to(rec, stopped, at, work)?;
            work.requests.push(Request::Interrupt {
                session: rec.id.clone(),
            });
        }
        Ok(None)
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
        // Past its quota only a change that ends an active turn is stored: one `turn` entry per
        // turn (9.3 Quota). An `activity` change alone is no change: `turn_to` pushes it.
        let mut bare = t.clone();
        if let Turn::Running { activity, .. } = &mut bare {
            *activity = None;
        }
        if work.past && bare != rec.turn && !(before && !t.is_active()) {
            return Ok(());
        }
        self.turn_to(rec, t, now, work)?;
        // Past its quota, or at the chunk cap, a session starts no queued input (9.3 Quota).
        if before && matches!(rec.turn, Turn::Idle { .. }) && !work.past && Self::room(rec).is_ok()
        {
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
            (None, None) => return self.unknown_entry(rec, r.raw, now, work),
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
            return self.unknown_entry(rec, r.raw, now, work);
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

    fn unknown_entry(
        &mut self,
        rec: &mut Record,
        raw: Value,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let e = adapters::unknown(raw).entry.expect("unknown has an entry");
        self.append_loose(rec, NewEntry::from(e), now, work)?;
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
        self.start_turn(&mut rec, input.to_string(), false, now, &mut work)?;
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
        let mut acct = self.accounts(caller);
        self.refuse_past(&acct)?;
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
                    None => self.start_turn(&mut rec, input.to_string(), resume, now, &mut work)?,
                    Some((oldest, n)) => {
                        self.start_turn(&mut rec, oldest.input, resume, now, &mut work)?;
                        q.save(&mut self.state, &rec.id)?;
                        work.pushes.push(Push::of(&rec.id, q.push_value()));
                        n
                    }
                };
                if open {
                    self.open_provider(&mut rec, &mut slots, &mut acct)?;
                }
                if !self.sync_active(&mut rec, &mut slots, was)? && open {
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
        let mut acct = self.accounts(caller);
        if d.for_session {
            self.refuse_past(&acct)?;
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
        self.seal(&mut rec)?;
        self.key_put("answer", caller, key, Some(session), &())?;
        self.finish(None, &mut acct)?;
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
        let mut acct = self.accounts(caller);
        if let Some(input) = replacement {
            check_input(input)?;
            if !self.may_use(caller, &rec.profile) {
                return Err(Self::revoked());
            }
            self.refuse_past(&acct)?;
        }
        let mut work = Work::default();
        self.settle(&mut rec, &mut slots, &mut work)?;
        let was = rec.turn.is_active();
        if replacement.is_none() && !was {
            self.finish(None, &mut acct)?;
            return Ok(Called {
                value: (),
                pushes: work.pushes,
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
            self.end_interrupted(&mut rec, now, &mut work)?;
            work.requests.push(Request::Interrupt {
                session: rec.id.clone(),
            });
        }
        let (open, resume) = plan;
        if let Some(input) = replacement {
            self.start_turn(&mut rec, input.to_string(), resume, now, &mut work)?;
        }
        if open {
            self.open_provider(&mut rec, &mut slots, &mut acct)?;
        }
        if !self.sync_active(&mut rec, &mut slots, was)? && open {
            slots.save(&mut self.state)?;
        }
        rec.updated = now;
        self.finish(Some(&mut rec), &mut acct)?;
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
        let mut acct = self.accounts(caller);
        let mut work = Work::default();
        let mut rec = self.session(caller, session, &mut slots, &mut work)?;
        let now = self.host.now();
        let was = rec.turn.is_active();
        let closed = self.close_all_with(&mut rec, "cancel", now, &mut work, false)?;
        if was {
            self.end_interrupted(&mut rec, now, &mut work)?;
        }
        // The host's `Cancel` ends the provider stream: its slot is free at once, and its later
        // `ended` finds no key (9.4).
        let freed = match self.live_provider(&rec) {
            Some(t) => self.release_provider(&mut rec, &t, &mut slots, &mut acct),
            None => false,
        };
        if !self.sync_active(&mut rec, &mut slots, was)? && freed {
            slots.save(&mut self.state)?;
        }
        work.requests.push(Request::Cancel {
            session: rec.id.clone(),
        });
        if was || closed {
            rec.updated = now;
        }
        self.finish(Some(&mut rec), &mut acct)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work),
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
            self.settle(&mut rec, slots, &mut Work::default())?;
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
        let mut work = Work::default();
        let mut rec = self.session(caller, session, &mut slots, &mut work)?;
        let mut q = Queue::load(&mut self.state, &rec.id);
        q.remove(n)?;
        q.save(&mut self.state, &rec.id)?;
        work.pushes.push(Push::of(&rec.id, q.push_value()));
        self.finish(Some(&mut rec), &mut acct)?;
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
        let rec = self.read_session(caller, session, &mut slots)?;
        let mut meta = rec.meta();
        Transcript::new(&mut self.state, &rec.id, &mut meta).read(after, before, max)
    }

    pub fn rename(&mut self, caller: &str, session: &str, title: &str) -> Result<(), KernelError> {
        check_title(title)?;
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let mut rec = self.load_owned(caller, session)?;
        self.refuse_past(&acct)?;
        self.settle(&mut rec, &mut slots, &mut Work::default())?;
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
        if rec.boot == slots.boot {
            for s in &rec.streams {
                let bytes = self.state.get(&stream_key(s));
                let kind = bytes.and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok());
                if kind.is_some_and(|k| k.kind == PROVIDER) {
                    slots.provider = slots.provider.saturating_sub(1);
                    acct.owner.close_provider(slots.boot);
                    slot_freed = true;
                }
            }
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
    /// session (9.4). An id with no key (never opened, ended, or of an older boot) changes
    /// nothing and sends nothing.
    pub fn provider_item(
        &mut self,
        stream: &str,
        event: ProviderEvent,
    ) -> Result<Called<()>, KernelError> {
        self.open_meter();
        let key = self
            .state
            .get(&stream_key(stream))
            .and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok());
        let Some(key) = key else {
            return Ok(Called {
                value: (),
                pushes: Vec::new(),
            });
        };
        let rec = Record::load(&mut self.state, &key.session)
            .ok_or_else(|| KernelError::UnknownSession(key.session.clone()))?;
        self.provider_on(rec, Some(stream), event)
    }

    /// One item of a provider's stream for a session, for tests: it goes by the session's live
    /// provider stream, or, with none, acts on the session directly.
    pub fn provider(
        &mut self,
        session: &str,
        event: ProviderEvent,
    ) -> Result<Called<()>, KernelError> {
        self.open_meter();
        let rec = Record::load(&mut self.state, session)
            .ok_or_else(|| KernelError::UnknownSession(session.to_string()))?;
        let stream = self.live_provider(&rec);
        self.provider_on(rec, stream.as_deref(), event)
    }

    /// The item's work, on the session it names; `stream` is the provider stream it came on.
    fn provider_on(
        &mut self,
        mut rec: Record,
        stream: Option<&str>,
        event: ProviderEvent,
    ) -> Result<Called<()>, KernelError> {
        let mut acct = self.accounts(&rec.owner);
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
        let mut work = Work {
            past: self.past(&acct),
            ..Work::default()
        };
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
        // 9.4: the stream's `ended` frees its slot and keys.
        let mut freed = false;
        if let (Some(t), Some(s)) = (ended, slots.as_mut()) {
            freed = self.release_provider(&mut rec, t, s, &mut acct);
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
        self.finish(changed.then_some(&mut rec), &mut acct)?;
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
        e: adapters::Entry,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let is_ask = e.kind == Kind::Ask;
        let body = e.body.clone();
        let decided = is_ask && body.get("decision").is_some_and(|d| !d.is_null());
        if is_ask && !decided && self.reused(rec, &body) {
            return self.unknown_entry(rec, body, now, work);
        }
        let id = body.get("id").and_then(Value::as_str);
        // An ask decision is stored as ever; any other entry is not past the quota (9.3).
        let stored = if decided {
            Some(self.append(rec, NewEntry::from(e), now)?)
        } else {
            self.append_loose(rec, NewEntry::from(e), now, work)?
        };
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
                let mut acct = self.accounts(&rec.owner);
                let stepped = self.step(&mut rec, slots, &mut done).is_ok();
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
