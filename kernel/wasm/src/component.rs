//! The glue: the crate's `Host` and `State` over host 19's imports, and the exports (AC4-AC6).
//! Each call builds a `Kernel` over the imports, runs the one crate call, and maps the result.

use crate::exports::mods::agent::sessions;
use crate::exports::watane::host::{stream_producer, stream_sink, timer_sink};
use crate::map;
use crate::mods::claude_code::sessions as provider;
use crate::watane::host::{caller, claude_session, host_info, state, streams, timers};
use agent_kernel::{
    Attached, Buffered, Host, Kernel, KernelError, Outcome, Request, Started, State, StreamError,
    TooLarge, MAX_VALUE,
};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

struct Export;

crate::export!(Export with_types_in crate);

/// `watane:host/state`.
struct WitState;

impl State for WitState {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        state::get(key)
    }
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        if value.len() > MAX_VALUE {
            return Err(TooLarge(MAX_VALUE));
        }
        state::put(key, &value);
        Ok(())
    }
    fn delete(&mut self, key: &str) {
        state::delete(key)
    }
}

/// One call's buffer (S32 R1, TECHNICAL 2.1): the kernel's state and the host's `new_id` share it,
/// so the id counter commits or drops with the call's other writes.
type Buf = Rc<RefCell<Buffered<WitState>>>;

struct Shared(Buf);

impl State for Shared {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.0.borrow_mut().get(key)
    }
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        self.0.borrow_mut().put(key, value)
    }
    fn delete(&mut self, key: &str) {
        self.0.borrow_mut().delete(key)
    }
}

/// The kernel's `Host` over `streams`, `timers`, `host-info`, the provider and `claude-session`.
struct WitHost(Buf);

const NEXT_ID: &str = "next-id";

impl Host for WitHost {
    fn generation(&mut self) -> u64 {
        host_info::generation()
    }
    fn now(&mut self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
    fn new_id(&mut self) -> String {
        let mut b = self.0.borrow_mut();
        let n = b
            .get(NEXT_ID)
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1);
        let _ = b.put(NEXT_ID, (n + 1).to_string().into_bytes());
        format!("s{n}")
    }
    fn timer_after(&mut self, id: &str, delay_ms: u64) {
        let _ = timers::after(id, delay_ms);
    }
    fn timer_cancel(&mut self, id: &str) {
        timers::cancel(id);
    }
    /// A direct stream takes the request on `claude-session`, a `direct: false` one on the
    /// provider; a `session-error` is ignored, the turn's end comes on the stream (2.1).
    fn request(&mut self, r: Request) {
        match r {
            Request::Turn {
                stream,
                direct,
                input,
                ..
            } => {
                let text = agent_kernel::input_text(&input);
                let _ = if direct {
                    claude_session::say(&stream, &text)
                } else {
                    provider::say(&stream, &text)
                };
            }
            Request::Interrupt { stream, direct, .. } => {
                let _ = if direct {
                    claude_session::interrupt(&stream)
                } else {
                    provider::interrupt(&stream)
                };
            }
            Request::Answer {
                stream,
                direct,
                ask,
                decision,
                ..
            } => {
                let d = map::decision(&decision);
                let _ = if direct {
                    claude_session::answer(&stream, &ask, &d)
                } else {
                    provider::answer(&stream, &ask, &d)
                };
            }
            Request::Cancel { stream, .. } => {
                if !stream.is_empty() {
                    streams::cancel(&stream);
                }
            }
        }
    }
    fn open_stream(&mut self) -> Result<String, StreamError> {
        streams::open().map_err(map::stream_error)
    }
    fn start_provider(&mut self, launch: &str, input: &str) -> Result<Started, String> {
        provider::start(launch, input)
            .map(|s| Started {
                stream: s.stream,
                direct: s.direct,
            })
            .map_err(map::start_error)
    }
    fn cancel_stream(&mut self, stream: &str) {
        streams::cancel(stream);
    }
    fn push(&mut self, stream: &str, value: Value) -> Result<(), StreamError> {
        let bytes = serde_json::to_vec(&value).map_err(|_| StreamError::TooLarge)?;
        streams::push(stream, &bytes).map_err(map::stream_error)
    }
    fn close_stream(&mut self, stream: &str, outcome: Outcome) {
        streams::close(stream, &map::outcome(outcome));
    }
    fn resume_stream(&mut self, stream: &str) {
        let _ = streams::resume(stream);
    }
}

/// Runs one crate call over a `Buffered` state: `Ok` commits its writes in order, `Err` drops them
/// (S32 R1). Host calls the crate makes go out as it makes them; it holds back those that follow
/// a failure (2.1).
fn run<T>(
    f: impl FnOnce(&mut Kernel<Shared, WitHost>) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    let buf: Buf = Rc::new(RefCell::new(Buffered::new(WitState)));
    let mut k = Kernel::new(Shared(buf.clone()), WitHost(buf.clone()));
    let r = f(&mut k);
    drop(k);
    let Ok(cell) = Rc::try_unwrap(buf) else {
        unreachable!("the kernel is dropped")
    };
    let b = cell.into_inner();
    if r.is_ok() {
        b.commit();
    } else {
        b.discard();
    }
    r
}

/// 2.3: the owner is `caller.identity()`, never `who()`.
fn me() -> String {
    caller::identity()
}

impl sessions::Guest for Export {
    fn start(
        key: String,
        profile: String,
        overlay: String,
        input: String,
    ) -> Result<String, sessions::Error> {
        run(|k| k.start(&me(), &key, &profile, &overlay, &input))
            .map(|c| c.value)
            .map_err(map::error)
    }
    /// Stage A: the crate has no name-based check (it checks a profile document for `start`),
    /// so this answers one refusal that says so.
    fn check(_profile: String, _overlay: String) -> Vec<sessions::Refusal> {
        vec![sessions::Refusal {
            field: "profile".into(),
            backend: String::new(),
            model: String::new(),
            reason: "check is not available in stage A".into(),
        }]
    }
    /// `caps` is not used in stage A: the crate's `attach` takes no capabilities.
    fn attach(
        session: String,
        after: u64,
        _caps: String,
    ) -> Result<streams::StreamId, sessions::Error> {
        match run(|k| k.attach(&me(), &session, after)) {
            Ok(c) => match c.value {
                Attached::Live { stream } => Ok(stream),
                Attached::Behind { through } => Err(sessions::Error::Behind(through)),
            },
            Err(e) => Err(map::error(e)),
        }
    }
    fn read(
        session: String,
        after: u64,
        before: Option<u64>,
        max: u32,
    ) -> Result<sessions::Page, sessions::Error> {
        run(|k| k.read(&me(), &session, after, before, max))
            .map(|p| sessions::Page {
                entries: p.entries,
                last: p.last,
            })
            .map_err(map::error)
    }
    fn say(session: String, key: String, input: String) -> Result<u64, sessions::Error> {
        run(|k| k.say(&me(), &session, &key, &input))
            .map(|c| c.value)
            .map_err(map::error)
    }
    fn answer(
        session: String,
        key: String,
        ask: String,
        decision: String,
    ) -> Result<(), sessions::Error> {
        run(|k| k.answer(&me(), &session, &key, &ask, &decision))
            .map(|_| ())
            .map_err(map::error)
    }
    fn interrupt(session: String, replacement: Option<String>) -> Result<(), sessions::Error> {
        run(|k| k.interrupt(&me(), &session, replacement.as_deref()))
            .map(|_| ())
            .map_err(map::error)
    }
    fn cancel(session: String) {
        let _ = run(|k| k.cancel(&me(), &session));
    }
    fn list(
        after: Option<String>,
        max: u32,
    ) -> Result<Vec<sessions::SessionInfo>, sessions::Error> {
        run(|k| k.list(&me(), after.as_deref(), max))
            .map(|v| {
                v.into_iter()
                    .map(|i| sessions::SessionInfo {
                        id: i.id,
                        title: i.title,
                        profile: i.profile,
                        backend: i.backend,
                        turn: i.turn,
                        updated: i.updated,
                    })
                    .collect()
            })
            .map_err(map::error)
    }
    fn rename(session: String, title: String) -> Result<(), sessions::Error> {
        run(|k| k.rename(&me(), &session, &title)).map_err(map::error)
    }
    fn delete(session: String) -> Result<(), sessions::Error> {
        run(|k| k.delete(&me(), &session)).map_err(map::error)
    }
    fn queued(session: String) -> Result<Vec<sessions::QueuedInput>, sessions::Error> {
        run(|k| k.queued(&me(), &session))
            .map(|v| {
                v.into_iter()
                    .map(|q| sessions::QueuedInput {
                        n: q.n,
                        input: q.input,
                    })
                    .collect()
            })
            .map_err(map::error)
    }
    fn unqueue(session: String, n: u64) -> Result<(), sessions::Error> {
        run(|k| k.unqueue(&me(), &session, n))
            .map(|_| ())
            .map_err(map::error)
    }
    fn take_back(session: String, allow: String) -> Result<(), sessions::Error> {
        run(|k| k.take_back(&me(), &session, &allow))
            .map(|_| ())
            .map_err(map::error)
    }
}

/// A kernel error in a sink call fails the call: the host rolls it back (3, "Failed batch").
fn failed(call: &str, e: KernelError) -> ! {
    panic!("{call}: {e}")
}

impl stream_sink::Guest for Export {
    fn items(s: String, items: Vec<streams::Item>) -> streams::SinkOutcome {
        let batch = items.into_iter().map(|i| (i.seq, i.body)).collect();
        match run(|k| k.provider_lines(&s, batch)) {
            Ok(c) => map::sink_outcome(c.value),
            Err(e) => failed("items", e),
        }
    }
    fn ended(s: String, outcome: streams::Outcome) {
        let cause = map::ended_cause(&outcome);
        if let Err(e) = run(|k| k.provider_ended_with(&s, cause)) {
            failed("ended", e);
        }
    }
}

impl stream_producer::Guest for Export {
    fn ready(s: String) {
        let _ = run(|k| Ok(k.stream_ready(&s)));
    }
    fn ended(s: String, _outcome: streams::Outcome) {
        let _ = run(|k| Ok(k.stream_ended(&s)));
    }
}

impl timer_sink::Guest for Export {
    fn fired(id: String) {
        let _ = run(|k| Ok(k.timer(&id)));
    }
}
