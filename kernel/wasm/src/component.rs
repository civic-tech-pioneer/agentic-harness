//! The glue: the crate's `Host` and `State` over host 19's imports, and the exports (AC4-AC6).
//! Each call builds a `Kernel` over the imports, runs the one crate call, and maps the result.

use crate::exports::mods::agent::{probe, sessions};
use crate::exports::watane::host::{stream_producer, stream_sink, timer_sink};
use crate::map;
use crate::watane::host::{caller, state, streams, timers};
use agent_kernel::{
    Attached, Host, Kernel, KernelError, Outcome, Request, State, StreamError, TooLarge, MAX_VALUE,
};
use serde_json::Value;

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

/// The kernel's `Host` over `streams`, `timers` and the clock.
struct WitHost;

const NEXT_ID: &str = "next-id";

impl Host for WitHost {
    /// TECHNICAL 2.1, "Exact text": `host-info.generation()` is not in host 19 (watane S97): constant 1.
    fn generation(&mut self) -> u64 {
        1
    }
    fn now(&mut self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
    fn new_id(&mut self) -> String {
        let n = state::get(NEXT_ID)
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1);
        state::put(NEXT_ID, (n + 1).to_string().as_bytes());
        format!("s{n}")
    }
    fn timer_after(&mut self, id: &str, delay_ms: u64) {
        let _ = timers::after(id, delay_ms);
    }
    fn timer_cancel(&mut self, id: &str) {
        timers::cancel(id);
    }
    /// Unreachable in stage A: no provider stream exists to take a request.
    fn request(&mut self, _r: Request) {}
    fn open_stream(&mut self) -> Result<String, StreamError> {
        streams::open().map_err(map::stream_error)
    }
    fn start_provider(&mut self, _launch: &str, _input: &str) -> Result<String, String> {
        Err("no Claude Code provider is linked".to_string())
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

fn kernel() -> Kernel<WitState, WitHost> {
    Kernel::new(WitState, WitHost)
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
        kernel()
            .start(&me(), &key, &profile, &overlay, &input)
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
        match kernel().attach(&me(), &session, after) {
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
        kernel()
            .read(&me(), &session, after, before, max)
            .map(|p| sessions::Page {
                entries: p.entries,
                last: p.last,
            })
            .map_err(map::error)
    }
    fn say(session: String, key: String, input: String) -> Result<u64, sessions::Error> {
        kernel()
            .say(&me(), &session, &key, &input)
            .map(|c| c.value)
            .map_err(map::error)
    }
    fn answer(
        session: String,
        key: String,
        ask: String,
        decision: String,
    ) -> Result<(), sessions::Error> {
        kernel()
            .answer(&me(), &session, &key, &ask, &decision)
            .map(|_| ())
            .map_err(map::error)
    }
    fn interrupt(session: String, replacement: Option<String>) -> Result<(), sessions::Error> {
        kernel()
            .interrupt(&me(), &session, replacement.as_deref())
            .map(|_| ())
            .map_err(map::error)
    }
    fn cancel(session: String) {
        let _ = kernel().cancel(&me(), &session);
    }
    fn list(
        after: Option<String>,
        max: u32,
    ) -> Result<Vec<sessions::SessionInfo>, sessions::Error> {
        kernel()
            .list(&me(), after.as_deref(), max)
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
        kernel().rename(&me(), &session, &title).map_err(map::error)
    }
    fn delete(session: String) -> Result<(), sessions::Error> {
        kernel().delete(&me(), &session).map_err(map::error)
    }
    fn queued(session: String) -> Result<Vec<sessions::QueuedInput>, sessions::Error> {
        kernel()
            .queued(&me(), &session)
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
        kernel()
            .unqueue(&me(), &session, n)
            .map(|_| ())
            .map_err(map::error)
    }
    fn take_back(session: String, allow: String) -> Result<(), sessions::Error> {
        kernel()
            .take_back(&me(), &session, &allow)
            .map(|_| ())
            .map_err(map::error)
    }
}

impl probe::Guest for Export {
    fn start(key: String, count: u32, every_ms: u32) -> Result<String, sessions::Error> {
        kernel()
            .probe_start(&me(), &key, count, every_ms)
            .map(|c| c.value)
            .map_err(map::error)
    }
}

/// A kernel error in a sink call fails the call: the host rolls it back (3, "Failed batch").
fn failed(call: &str, e: KernelError) -> ! {
    panic!("{call}: {e}")
}

impl stream_sink::Guest for Export {
    fn items(s: String, items: Vec<streams::Item>) -> streams::SinkOutcome {
        let batch = items
            .into_iter()
            .map(|i| (i.seq, map::provider_event(&i.body)))
            .collect();
        match kernel().provider_batch(&s, batch) {
            Ok(c) => map::sink_outcome(c.value),
            Err(e) => failed("items", e),
        }
    }
    fn ended(s: String, _outcome: streams::Outcome) {
        if let Err(e) = kernel().provider_ended(&s) {
            failed("ended", e);
        }
    }
}

impl stream_producer::Guest for Export {
    fn ready(s: String) {
        kernel().stream_ready(&s);
    }
    fn ended(s: String, _outcome: streams::Outcome) {
        kernel().stream_ended(&s);
    }
}

impl timer_sink::Guest for Export {
    fn fired(id: String) {
        kernel().timer(&id);
    }
}
