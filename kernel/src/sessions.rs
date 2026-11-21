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
use crate::record::{check_key, check_title, OpenAsk, Record, Stop, Turn, ID_BYTES};
use crate::state::State;
use crate::transcript::{Joiner, NewEntry, Page, Transcript, MAX_CHUNKS};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

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
}

/// A call's result and the `_meta.watane` pushes it caused, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Called<T> {
    pub value: T,
    pub pushes: Vec<Value>,
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
    pushes: Vec<Value>,
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

fn key_of(owner: &str, key: &str) -> String {
    format!("key/{owner}/{key}")
}

fn owner_page_key(owner: &str) -> String {
    format!("owner/{owner}/0")
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

    /// Starts a top-level call: reads `slots` (R1 goes here).
    fn begin(&mut self) -> Slots {
        Slots::load(&mut self.state)
    }

    /// Loads a session record for the caller: unknown, or another owner's, is an error (R6 goes here).
    fn session(&mut self, caller: &str, id: &str) -> Result<Record, KernelError> {
        let rec = Record::load(&mut self.state, id)
            .ok_or_else(|| KernelError::UnknownSession(id.to_string()))?;
        if rec.owner != caller {
            return Err(KernelError::NotOwner);
        }
        Ok(rec)
    }

    fn key_get<T: DeserializeOwned>(&mut self, owner: &str, key: &str) -> Option<T> {
        let bytes = self.state.get(&key_of(owner, key))?;
        serde_json::from_slice(&bytes).ok()
    }

    fn key_put<T: Serialize>(&mut self, owner: &str, key: &str, v: &T) -> Result<(), KernelError> {
        let bytes = serde_json::to_vec(v).expect("a key result serializes");
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
                work.pushes.push(turn_push(&t));
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
        work.pushes.push(turn_push(&t));
        Ok(())
    }

    /// A user message, the `running` turn and the request: what a started turn is (AC1).
    /// Returns the message's index.
    fn start_turn(
        &mut self,
        rec: &mut Record,
        input: String,
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
            resume: false,
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
        self.start_turn(rec, next.input, now, work)?;
        q.save(&mut self.state, &rec.id)?;
        work.pushes.push(q.push_value());
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
        if let Turn::Waiting { asks, expires_at } = rec.turn.clone() {
            let left: Vec<String> = asks.into_iter().filter(|a| a != id).collect();
            let t = if left.is_empty() {
                Turn::Running {
                    since: now,
                    activity: None,
                }
            } else {
                waiting(left, expires_at)
            };
            self.turn_to(rec, t, now, work)?;
        }
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

    fn flush(&mut self, work: Work) -> Vec<Value> {
        for r in work.requests {
            self.host.request(r);
        }
        work.pushes
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
        let mut slots = self.begin();
        if let Some(id) = self.key_get::<String>(caller, key) {
            return Ok(Called {
                value: id,
                pushes: Vec::new(),
            });
        }
        check_input(input)?;
        let Some(bytes) = self.state.get(&format!("profile/{profile}")) else {
            return Err(refused("profile", "no such profile"));
        };
        let json = String::from_utf8_lossy(&bytes).into_owned();
        let parsed = parse_profile(&json).0;
        let backend = parsed
            .backend
            .and_then(|b| b.parse::<Backend>().ok())
            .unwrap_or(Backend::Claude);
        // A profile without a model runs the backend's default one.
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
        self.start_turn(&mut rec, input.to_string(), now, &mut work)?;
        self.sync_active(&mut rec, &mut slots, false)?;
        page.push(id.clone());
        self.state
            .put(
                &owner_page_key(caller),
                serde_json::to_vec(&page).expect("a page serializes"),
            )
            .map_err(|t| KernelError::TooLarge(t.0 as u64))?;
        rec.save(&mut self.state)?;
        self.key_put(caller, key, &id)?;
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
        let mut slots = self.begin();
        if let Some(n) = self.key_get::<u64>(caller, key) {
            return Ok(Called {
                value: n,
                pushes: Vec::new(),
            });
        }
        check_input(input)?;
        let mut rec = self.session(caller, session)?;
        let mut work = Work::default();
        let n = match rec.turn {
            Turn::Idle { .. } | Turn::Unknown | Turn::ResumeLost => {
                Self::room(&rec)?;
                active::check_room(&slots)?;
                let now = self.host.now();
                let n = self.start_turn(&mut rec, input.to_string(), now, &mut work)?;
                self.sync_active(&mut rec, &mut slots, false)?;
                rec.updated = now;
                rec.save(&mut self.state)?;
                n
            }
            _ => {
                let mut q = Queue::load(&mut self.state, &rec.id);
                let n = q.push(input.to_string())?;
                q.save(&mut self.state, &rec.id)?;
                work.pushes.push(q.push_value());
                n
            }
        };
        self.key_put(caller, key, &n)?;
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
        self.begin();
        if self.key_get::<()>(caller, key).is_some() {
            return Ok(Called {
                value: (),
                pushes: Vec::new(),
            });
        }
        let mut rec = self.session(caller, session)?;
        let d = parse_decision(decision)?;
        let open = rec
            .open
            .iter()
            .find(|o| o.id == ask && o.kind == "ask")
            .cloned()
            .ok_or_else(|| refused("ask", "no open ask with this id"))?;
        let unreadable = || refused("ask", "the ask entry is unreadable");
        let mut body = self.ask_body(&rec, open.i).ok_or_else(unreadable)?;
        let Some(obj) = body.as_object_mut() else {
            return Err(unreadable());
        };
        if d.for_session {
            let Some(reach) = obj.get("reach").and_then(Value::as_str) else {
                return Err(refused("decision", "this ask offers no session allow"));
            };
            rec.add_allow(Value::String(reach.to_string()))?;
        }
        let mut dec = json!({"outcome": if d.allow { "allow" } else { "deny" }, "by": "owner"});
        if d.for_session {
            dec["for"] = json!("session");
        }
        if let Some(r) = &d.reason {
            dec["reason"] = json!(r);
        }
        obj.insert("decision".into(), dec);
        let now = self.host.now();
        let mut work = Work::default();
        let entry = NewEntry {
            kind: "ask".into(),
            uuid: None,
            body,
        };
        self.append(&mut rec, entry, now)?;
        self.close_ask(&mut rec, ask, now, &mut work)?;
        work.requests.push(Request::Answer {
            session: rec.id.clone(),
            ask: ask.to_string(),
            decision: d.value,
        });
        rec.updated = now;
        rec.save(&mut self.state)?;
        self.key_put(caller, key, &())?;
        Ok(Called {
            value: (),
            pushes: self.flush(work),
        })
    }

    pub fn queued(&mut self, caller: &str, session: &str) -> Result<Vec<Queued>, KernelError> {
        self.begin();
        let rec = self.session(caller, session)?;
        Ok(Queue::load(&mut self.state, &rec.id).inputs)
    }

    pub fn unqueue(
        &mut self,
        caller: &str,
        session: &str,
        n: u64,
    ) -> Result<Called<()>, KernelError> {
        self.begin();
        let rec = self.session(caller, session)?;
        let mut q = Queue::load(&mut self.state, &rec.id);
        q.remove(n)?;
        q.save(&mut self.state, &rec.id)?;
        Ok(Called {
            value: (),
            pushes: vec![q.push_value()],
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
        self.begin();
        let rec = self.session(caller, session)?;
        let mut meta = rec.meta();
        Transcript::new(&mut self.state, &rec.id, &mut meta).read(after, before, max)
    }

    pub fn rename(&mut self, caller: &str, session: &str, title: &str) -> Result<(), KernelError> {
        check_title(title)?;
        self.begin();
        let mut rec = self.session(caller, session)?;
        rec.title = title.to_string();
        rec.save(&mut self.state)
    }

    pub fn delete(&mut self, caller: &str, session: &str) -> Result<(), KernelError> {
        let mut slots = self.begin();
        let rec = self.session(caller, session)?;
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
        self.begin();
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
                out.push(SessionInfo {
                    id: r.id,
                    title: r.title,
                    profile: r.profile,
                    backend: r.backend,
                    turn: r.turn.to_json().to_string(),
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
            ProviderEvent::Out(out) => {
                if let Some(e) = out.entry {
                    self.out_entry(&mut rec, e, now, &mut work)?;
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
        let i = self.append(rec, NewEntry::from(e), now)?;
        let id = body.get("id").and_then(Value::as_str);
        let (true, Some(id)) = (is_ask, id) else {
            return Ok(());
        };
        if body.get("decision").is_some_and(|d| !d.is_null()) {
            return self.close_ask(rec, id, now, work);
        }
        if rec.open.iter().any(|o| o.id == id) {
            return Ok(());
        }
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("ask");
        rec.open.push(OpenAsk {
            id: id.to_string(),
            kind: kind.to_string(),
            i,
        });
        if kind == "ask" {
            let mine = body.get("expires-at").and_then(Value::as_u64);
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

    /// `timer-sink.fired`: session timers and deadlines are not built yet (R2 comes with S21-4).
    pub fn timer(&mut self, _id: &str) -> Called<()> {
        Called {
            value: (),
            pushes: Vec::new(),
        }
    }
}

/// Whether the event could move an inactive session into an active turn.
fn may_activate(e: &ProviderEvent) -> bool {
    match e {
        ProviderEvent::Turn(t) => t.is_active(),
        ProviderEvent::Out(o) => o.entry.as_ref().is_some_and(|e| e.kind == Kind::Ask),
    }
}
