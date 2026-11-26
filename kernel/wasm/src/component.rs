//! The glue: the crate's `Host` and `State` over host 19's imports, and the exports (AC4-AC6).
//! Each call builds a `Kernel` over the imports, runs the one crate call, and maps the result.

use crate::exports::mods::agent::sessions;
use crate::exports::watane::host::{stream_producer, stream_sink, timer_sink};
use crate::map;
use crate::mods::claude_code::sessions as provider;
use crate::watane::host::{
    agent_tools, caller, claude_session, host_info, https_stream, state, streams, timers,
};
use agent_kernel::call::{self, CallKernel};
use agent_kernel::{
    Attached, Host, KernelError, Outcome, Request, Started, State, StreamError, TooLarge, Tool,
    MAX_VALUE,
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

/// The kernel's `Host` over `streams`, `timers`, `host-info`, the provider and `claude-session`.
/// Its `new_id` is never called: `call::run` keeps the counter in the call's buffer.
struct WitHost;

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
        unreachable!("call::run counts ids")
    }
    fn timer_after(&mut self, id: &str, delay_ms: u64) {
        let _ = timers::after(id, delay_ms);
    }
    fn timer_cancel(&mut self, id: &str) {
        timers::cancel(id);
    }
    fn timer_firing_origin(&mut self) -> Option<String> {
        timers::firing_origin()
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
    /// `https-stream.start`: an `Err` is the `start-error` as `map::start_error` names it.
    fn https_start(&mut self, request: &str) -> Result<String, String> {
        https_stream::start(request).map_err(map::start_error)
    }
    fn tools_open(&mut self, toolsets: &[String]) -> Result<String, String> {
        agent_tools::open(toolsets).map_err(map::tools_error)
    }
    fn tools_describe(&mut self, handle: &str) -> Result<Vec<Tool>, String> {
        agent_tools::describe(handle)
            .map(|v| {
                v.into_iter()
                    .map(|t| Tool {
                        name: t.name,
                        description: t.description,
                        input_schema: t.input_schema,
                        parallel: t.parallel,
                        idempotent: t.idempotent,
                    })
                    .collect()
            })
            .map_err(map::tools_error)
    }
    fn tools_call(
        &mut self,
        handle: &str,
        key: &str,
        tool: &str,
        args: &str,
    ) -> Result<String, String> {
        agent_tools::call(handle, key, tool, args).map_err(map::tools_error)
    }
    fn tools_close(&mut self, handle: &str) {
        agent_tools::close(handle)
    }
}

/// Runs one crate call (S32 R1, `call::run`): `Ok` commits its writes, `Err` drops them.
fn run<T>(
    f: impl FnOnce(&mut CallKernel<WitState, WitHost>) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    call::run(WitState, WitHost, f).0
}

/// 2.3: the owner is `caller.identity()`, never `who()`.
fn me() -> String {
    caller::identity()
}

fn run_client<T>(
    f: impl FnOnce(&mut CallKernel<WitState, WitHost>) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    run(|k| {
        let value = f(k)?;
        if me() == "page" {
            k.arm_page_origin();
        }
        Ok(value)
    })
}

impl sessions::Guest for Export {
    fn start(
        key: String,
        profile: String,
        overlay: String,
        input: String,
    ) -> Result<String, sessions::Error> {
        run_client(|k| k.start(&me(), &key, &profile, &overlay, &input))
            .map(|c| c.value)
            .map_err(map::error)
    }
    fn start_in_folder(
        key: String,
        profile: String,
        overlay: String,
        input: String,
        folder: String,
    ) -> Result<String, sessions::Error> {
        run_client(|k| k.start_in_folder(&me(), &key, &profile, &overlay, &input, &folder))
            .map(|c| c.value)
            .map_err(map::error)
    }

    /// 9.1, 9.2: by profile name, as `start` would check it.
    fn check(profile: String, overlay: String) -> Vec<sessions::Refusal> {
        run_client(|k| Ok(k.check_named(&me(), &profile, &overlay)))
            .unwrap_or_default()
            .into_iter()
            .map(map::refusal)
            .collect()
    }
    fn profiles() -> Vec<sessions::ProfileInfo> {
        run_client(|k| Ok(k.profiles(&me())))
            .unwrap_or_default()
            .into_iter()
            .map(|p| sessions::ProfileInfo {
                name: p.name,
                backend: p.backend,
                model: p.model,
            })
            .collect()
    }
    /// `caps` is not used in stage A: the crate's `attach` takes no capabilities.
    fn attach(
        session: String,
        after: u64,
        _caps: String,
    ) -> Result<streams::StreamId, sessions::Error> {
        match run_client(|k| k.attach(&me(), &session, after)) {
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
        run_client(|k| k.read(&me(), &session, after, before, max))
            .map(|p| sessions::Page {
                entries: p.entries,
                last: p.last,
            })
            .map_err(map::error)
    }
    fn say(session: String, key: String, input: String) -> Result<u64, sessions::Error> {
        run_client(|k| k.say(&me(), &session, &key, &input))
            .map(|c| c.value)
            .map_err(map::error)
    }
    fn answer(
        session: String,
        key: String,
        ask: String,
        decision: String,
    ) -> Result<(), sessions::Error> {
        run_client(|k| k.answer(&me(), &session, &key, &ask, &decision))
            .map(|_| ())
            .map_err(map::error)
    }
    fn interrupt(session: String, replacement: Option<String>) -> Result<(), sessions::Error> {
        run_client(|k| k.interrupt(&me(), &session, replacement.as_deref()))
            .map(|_| ())
            .map_err(map::error)
    }
    fn cancel(session: String) {
        let _ = run_client(|k| k.cancel(&me(), &session));
    }
    fn list(
        after: Option<String>,
        max: u32,
    ) -> Result<Vec<sessions::SessionInfo>, sessions::Error> {
        run_client(|k| k.list(&me(), after.as_deref(), max))
            .map(|v| {
                v.into_iter()
                    .map(|i| sessions::SessionInfo {
                        id: i.id,
                        title: i.title,
                        profile: i.profile,
                        backend: i.backend,
                        workspace: i.workspace,
                        turn: i.turn,
                        updated: i.updated,
                    })
                    .collect()
            })
            .map_err(map::error)
    }
    fn rename(session: String, title: String) -> Result<(), sessions::Error> {
        run_client(|k| k.rename(&me(), &session, &title)).map_err(map::error)
    }
    fn delete(session: String) -> Result<(), sessions::Error> {
        run_client(|k| k.delete(&me(), &session)).map_err(map::error)
    }
    fn queued(session: String) -> Result<Vec<sessions::QueuedInput>, sessions::Error> {
        run_client(|k| k.queued(&me(), &session))
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
        run_client(|k| k.unqueue(&me(), &session, n))
            .map(|_| ())
            .map_err(map::error)
    }
    fn take_back(session: String, allow: String) -> Result<(), sessions::Error> {
        run_client(|k| k.take_back(&me(), &session, &allow))
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
