//! The session calls of TECHNICAL 2.3 over host state: `start`, `say`, `answer`, the queue, `read`,
//! `rename`, `delete`, `list`, and the provider's events (9.3, 9.4). Restart is not here (R1-R6).
//!
//! Every call checks all its refusals before its first write (7, "A call commits whole"), keeps
//! a client key only when the call changed something (9.1), and reads and writes only what its
//! inputs name: no listing of keys.

use crate::active::{self, Slots};
use crate::adapters::claude::{facts, unread_init, ClaudeAdapter, Fact};
use crate::adapters::{self, Kind};
use crate::broker;
use crate::check::check;
use crate::error::{KernelError, Refusal};
use crate::launch::launch;
use crate::openai::run::{Cx, REQUEST};
use crate::openai::tools::JOB;
use crate::overlay::{apply_overlay, parse_overlay, Overlay};
use crate::profile::{parse_profile, Backend};
use crate::queue::{check_input, Queue, Queued};
use crate::quota::{segment, Change, Limits, Metered, Past, Usage};
use crate::record::{
    check_key, check_title, Allow, Deadline, OpenAsk, PendingAnswer, Record, Stop, Turn,
    ALLOW_BYTES, ID_BYTES, OPEN_ASKS,
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
const PAGE_ORIGIN: &str = "page-origin";
/// The deadline of the 10-minute provider release (9.3, 9.5), and its delay.
const IDLE: &str = "idle";
const IDLE_MS: u64 = 600_000;
/// The profile a page gets on its first run (9.1): Claude's defaults, nothing more.
const SCRATCH: &str = "scratch";
/// The built-in `scratch` (9.1): no built-in tool, mode `manual`, its asks sent to the owner (S36).
const SCRATCH_PROFILE: &str = r#"{"backend":"claude","partial":true,"ask":true}"#;
/// The identity of the console's page (18.1).
const PAGE: &str = "page";

/// What the kernel needs of the host (2.1): the generation, the clock, ids, timers, requests.
pub trait Host {
    /// `host-info.generation()` (2.1, G1).
    fn generation(&mut self) -> u64;
    /// Milliseconds.
    fn now(&mut self) -> u64;
    /// A fresh session id, at most 64 bytes.
    fn new_id(&mut self) -> String;
    /// The kernel never passes more than `MAX_TIMER_MS` (2.1, "The kernel's side of the seam").
    fn timer_after(&mut self, id: &str, delay_ms: u64);
    fn timer_cancel(&mut self, id: &str);
    /// The host-resolved identity of this top-level timer firing (2.1).
    fn timer_firing_origin(&mut self) -> Option<String> {
        None
    }
    /// What goes to the provider: recorded, never run.
    fn request(&mut self, r: Request);
    /// `streams.open()`: a new owner stream (9.4).
    fn open_stream(&mut self) -> Result<String, StreamError>;
    /// The provider's `start` (2.2): the provider stream of a new Claude session and whether it
    /// is `direct`; `Err(cause)` when it fails or none is linked.
    fn start_provider(&mut self, launch: &str, input: &str) -> Result<Started, String>;
    /// The consumer's `cancel` of a stream the kernel holds.
    fn cancel_stream(&mut self, stream: &str);
    /// The producer's `push` on a stream the kernel opened (an owner stream) (3, 2.1).
    fn push(&mut self, stream: &str, value: Value) -> Result<(), StreamError>;
    /// The producer's `close` of a stream the kernel opened, with its outcome.
    fn close_stream(&mut self, stream: &str, outcome: Outcome);
    /// The consumer's `resume` of a stream the kernel holds; it does nothing for one that is not deferred (3).
    fn resume_stream(&mut self, stream: &str);
    /// `https-stream.start` (2.1, 5): the request's stream, or the host's `start-error` as text.
    fn https_start(&mut self, request: &str) -> Result<String, String> {
        let _ = request;
        Err("not linked yet".into())
    }
    /// `agent-tools.open` (6): used by S40-4. The error is the `tools-error` case in kebab-case.
    fn tools_open(&mut self, toolsets: &[String]) -> Result<String, String> {
        let _ = toolsets;
        Err("not linked yet".into())
    }
    fn tools_describe(&mut self, handle: &str) -> Result<Vec<Tool>, String> {
        let _ = handle;
        Err("not linked yet".into())
    }
    fn tools_call(
        &mut self,
        handle: &str,
        key: &str,
        tool: &str,
        args: &str,
    ) -> Result<String, String> {
        let _ = (handle, key, tool, args);
        Err("not linked yet".into())
    }
    fn tools_close(&mut self, handle: &str) {
        let _ = handle;
    }
}

/// One tool of a toolset, as `agent-tools.describe` gives it (6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: String,
    pub parallel: bool,
    pub idempotent: bool,
}

/// What the provider's `start` answers (2.2): the stream, and whether it is the host's own (10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    pub stream: String,
    pub direct: bool,
}

/// The text a turn's input is sent as (2.1, "The kernel's calls on a provider stream"): the
/// `text` of its ACP text blocks joined with a blank line, other blocks left out. Input that is
/// not a JSON array of blocks is the text itself.
pub fn input_text(input: &str) -> String {
    match serde_json::from_str::<Value>(input) {
        Ok(Value::Array(blocks)) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => input.to_string(),
    }
}

/// The longest delay the kernel asks of `timers.after`: 30 days (7).
pub const MAX_TIMER_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// host 19's `stream-error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamError {
    NotYours,
    UnknownStream,
    /// A `push` past the stream's window (3, "In flight"), or `open` past 32 streams.
    Full,
    TooLarge,
}

impl StreamError {
    /// The host's code, as the WIT enum spells it.
    pub fn code(&self) -> &'static str {
        match self {
            StreamError::NotYours => "not-yours",
            StreamError::UnknownStream => "unknown-stream",
            StreamError::Full => "full",
            StreamError::TooLarge => "too-large",
        }
    }

    fn cause(&self) -> &'static str {
        match self {
            StreamError::NotYours => "this call carries no live connection",
            StreamError::UnknownStream => "the stream is gone",
            StreamError::Full => "the client holds all its streams",
            StreamError::TooLarge => "the item is too large",
        }
    }
}

/// The `outcome` a producer's `close` passes (2.1): the three the kernel uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Closed,
    Replaced,
    Lagging,
}

/// What an items call answers the host (3, "Deferred batch"): `Defer` is a call rolled back whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkOutcome {
    Taken,
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
    /// A turn's input for the session's live provider `stream` (`direct`: the host's own, 2.1).
    Turn {
        session: String,
        stream: String,
        direct: bool,
        input: String,
        resume: bool,
    },
    /// The owner's `interrupt`: the host stops the turn the provider runs.
    Interrupt {
        session: String,
        stream: String,
        direct: bool,
    },
    /// The owner's `cancel`: the host ends the session's provider `stream` (empty when it holds
    /// none) and handle.
    Cancel { session: String, stream: String },
    Answer {
        session: String,
        stream: String,
        direct: bool,
        ask: String,
        decision: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostCall {
    Generation,
    TimerAfter {
        id: String,
        delay_ms: u64,
    },
    TimerCancel {
        id: String,
    },
    Request(Request),
    /// `streams.open()`: an owner stream.
    Open {
        stream: String,
    },
    /// The provider's `start`: a Claude session's provider stream.
    StartProvider {
        launch: String,
        input: String,
    },
    CancelStream {
        stream: String,
    },
    Push {
        stream: String,
        value: Value,
    },
    Close {
        stream: String,
        outcome: Outcome,
    },
    Resume {
        stream: String,
    },
    /// `https-stream.start`: a model request.
    HttpsStart {
        request: String,
    },
    ToolsOpen {
        toolsets: Vec<String>,
    },
    ToolsDescribe {
        handle: String,
    },
    ToolsCall {
        handle: String,
        key: String,
        tool: String,
        args: String,
    },
    ToolsClose {
        handle: String,
    },
}

/// A host for tests: a settable clock and generation, ids `s1`, `s2`, ..., and a log of what was asked.
#[derive(Debug, Default)]
pub struct FakeHost {
    pub generation: u64,
    pub now: u64,
    pub firing_origin: Option<String>,
    pub next_id: u64,
    pub next_stream: u64,
    pub calls: Vec<HostCall>,
    /// Streams whose window is full: a push to one fails `Full`.
    pub full: BTreeSet<String>,
    /// Streams that have ended: a push to one fails `UnknownStream`.
    pub gone: BTreeSet<String>,
    /// `open_stream` fails with this.
    pub open_error: Option<StreamError>,
    /// `start_provider` fails: no provider is linked.
    pub no_provider: bool,
    /// A synchronous launch refusal returned by the host.
    pub start_error: Option<String>,
    /// `start_provider` answers `direct: false` (a checked provider's stream, 10).
    pub not_direct: bool,
    /// `https_start` fails with this (the host's `start-error`).
    pub https_error: Option<String>,
    /// What `tools_describe` answers.
    pub tools: Vec<Tool>,
    /// A failure of `tools_describe` only.
    pub tools_describe_error: Option<String>,
    /// Every `tools_*` that can fail fails with this.
    pub tools_error: Option<String>,
    /// Handles `tools_open` answered: `h1`, `h2`, ...
    pub next_handle: u64,
}

impl FakeHost {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Host for FakeHost {
    fn generation(&mut self) -> u64 {
        self.calls.push(HostCall::Generation);
        self.generation
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
    fn timer_firing_origin(&mut self) -> Option<String> {
        self.firing_origin.clone()
    }
    fn request(&mut self, r: Request) {
        self.calls.push(HostCall::Request(r));
    }
    fn open_stream(&mut self) -> Result<String, StreamError> {
        if let Some(e) = self.open_error {
            return Err(e);
        }
        self.next_stream += 1;
        let stream = format!("t{}", self.next_stream);
        self.calls.push(HostCall::Open {
            stream: stream.clone(),
        });
        Ok(stream)
    }
    fn start_provider(&mut self, launch: &str, input: &str) -> Result<Started, String> {
        if let Some(cause) = &self.start_error {
            return Err(cause.clone());
        }
        if self.no_provider {
            return Err("no Claude Code provider is linked".to_string());
        }
        self.next_stream += 1;
        self.calls.push(HostCall::StartProvider {
            launch: launch.to_string(),
            input: input.to_string(),
        });
        Ok(Started {
            stream: format!("t{}", self.next_stream),
            direct: !self.not_direct,
        })
    }
    fn cancel_stream(&mut self, stream: &str) {
        self.calls.push(HostCall::CancelStream {
            stream: stream.to_string(),
        });
    }
    fn push(&mut self, stream: &str, value: Value) -> Result<(), StreamError> {
        if self.gone.contains(stream) {
            return Err(StreamError::UnknownStream);
        }
        if self.full.contains(stream) {
            return Err(StreamError::Full);
        }
        self.calls.push(HostCall::Push {
            stream: stream.to_string(),
            value,
        });
        Ok(())
    }
    fn close_stream(&mut self, stream: &str, outcome: Outcome) {
        self.calls.push(HostCall::Close {
            stream: stream.to_string(),
            outcome,
        });
    }
    fn resume_stream(&mut self, stream: &str) {
        self.calls.push(HostCall::Resume {
            stream: stream.to_string(),
        });
    }
    fn https_start(&mut self, request: &str) -> Result<String, String> {
        if let Some(cause) = &self.https_error {
            return Err(cause.clone());
        }
        self.next_stream += 1;
        self.calls.push(HostCall::HttpsStart {
            request: request.to_string(),
        });
        Ok(format!("t{}", self.next_stream))
    }
    fn tools_open(&mut self, toolsets: &[String]) -> Result<String, String> {
        if let Some(e) = &self.tools_error {
            return Err(e.clone());
        }
        self.next_handle += 1;
        self.calls.push(HostCall::ToolsOpen {
            toolsets: toolsets.to_vec(),
        });
        Ok(format!("h{}", self.next_handle))
    }
    fn tools_describe(&mut self, handle: &str) -> Result<Vec<Tool>, String> {
        if let Some(e) = self
            .tools_describe_error
            .as_ref()
            .or(self.tools_error.as_ref())
        {
            return Err(e.clone());
        }
        self.calls.push(HostCall::ToolsDescribe {
            handle: handle.to_string(),
        });
        Ok(self.tools.clone())
    }
    fn tools_call(
        &mut self,
        handle: &str,
        key: &str,
        tool: &str,
        args: &str,
    ) -> Result<String, String> {
        if let Some(e) = &self.tools_error {
            return Err(e.clone());
        }
        self.next_stream += 1;
        self.calls.push(HostCall::ToolsCall {
            handle: handle.to_string(),
            key: key.to_string(),
            tool: tool.to_string(),
            args: args.to_string(),
        });
        Ok(format!("t{}", self.next_stream))
    }
    fn tools_close(&mut self, handle: &str) {
        self.calls.push(HostCall::ToolsClose {
            handle: handle.to_string(),
        });
    }
}

/// What a provider reports for a session (an item call on its stream).
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // the interface is fixed: `Out` is not boxed
pub enum ProviderEvent {
    Out(adapters::Out),
    Turn(Turn),
    /// The workspace the stream's `initialized` item names (4.6): kept once in the record.
    Workspace(String),
    /// The launch is waiting for consent, or has initialized and cleared the wait.
    Consent(Option<String>),
    /// The Claude session id of `system/init` (9.8 "Adapter state"): the record's `claude-uuid`.
    ClaudeSession(String),
    /// A `system/init` and its `cwd` (9.3 `folder`): the first init decides the folder, kept
    /// when it is a plain absolute folder; a later init changes nothing.
    Folder(Option<String>),
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
    pub(crate) fn of(session: &str, value: Value) -> Self {
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
    pub workspace: Option<String>,
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
    pub(crate) opened: Option<String>,
    /// While an item call may still be dropped whole (3, "Deferred batch"), the host calls it
    /// makes outside `request` wait here until the call commits.
    pub(crate) held: Option<Vec<Effect>>,
    /// A request slot was freed, or a request waits in the FIFO: the call's end sends the oldest
    /// waiting one (9.5 Slots).
    pub(crate) drain_due: bool,
    /// The FIFO places of the model requests this call queued: the call's end sends them or stops
    /// their turns `busy`; no other call sends them (9.5 Slots, the deputy's r5 ruling).
    pub(crate) queued: Vec<u64>,
    /// The call may run under an origin other than the session owner's: the FIFO's drain, or an
    /// item or end of a job the drain started. It sends no model request (9.5 Slots).
    pub(crate) foreign: bool,
    /// Present only during a session timer call, including when the host gave no origin.
    timer_origin: Option<Option<String>>,
}

/// What one item of an items call came to (`item_step`).
pub(crate) enum Step {
    Defer,
    Done {
        pushes: Vec<Push>,
        requests: Vec<Request>,
        /// `(session, stream)` of an owner stream whose push failed other than `full`: it has ended.
        gone: Vec<(String, String)>,
    },
}

/// A host call an item call makes that must not happen when the call defers.
pub(crate) enum Effect {
    TimerAfter(String, u64),
    TimerCancel(String),
    Cancel(String),
}

/// Pushes and provider requests a call collects; requests leave only after the last write.
#[derive(Default)]
pub(crate) struct Work {
    pub(crate) pushes: Vec<Push>,
    pub(crate) requests: Vec<Request>,
    /// The index of the `message` or `tool` entry the call stored last (9.4, "The wire").
    pub(crate) stored: Option<u64>,
    /// The live owner stream of the session an item call acted on, as its record ends the call.
    pub(crate) live: Option<String>,
    /// A call that resolves a child's `task` call for its root (9.7) also works on the root: the
    /// root's pushes and its live owner stream, which are not this call's session's.
    pub(crate) peer: Option<(Option<String>, Vec<Push>)>,
    /// The pushes of the children a root's interrupt, cancel or delete stopped, each with its live
    /// owner stream: they leave before the root's own (9.7: cleanup is done before Stop).
    pub(crate) stopped: Vec<(Option<String>, Vec<Push>)>,
}

impl Work {
    /// The one rule for `stopped` (S134 r3 F3.1): a `Work` made inside a call for another record
    /// hands the children's updates it collected on to the call's own, so the caller's delivery
    /// (`flush`, `tail`, `deliver_stopped`) sends them; no path drops them.
    pub(crate) fn hand_on_stopped(&mut self, inner: &mut Work) {
        self.stopped.append(&mut inner.stopped);
    }
}

/// The owner's account and the kernel's, as read for a call and kept for its one write (9.3 Quota).
pub(crate) struct Acct {
    /// Past quota when the accounts were read; only `accounts` makes it.
    pub(crate) past: Past,
    owner_key: String,
    pub(crate) owner: Usage,
    account: Usage,
    owner0: Usage,
    account0: Usage,
}

impl Acct {
    /// One more session of the owner, on owner page `n` (9.1, 9.3).
    pub(crate) fn add_session(&mut self, n: usize) {
        if self.owner.pages.len() <= n {
            self.owner.pages.resize(n + 1, 0);
        }
        self.owner.pages[n] += 1;
        self.owner.sessions += 1;
        self.account.sessions += 1;
    }
}

pub(crate) fn add_signed(v: u64, d: i64) -> u64 {
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
pub(crate) struct StreamKey {
    pub(crate) session: String,
    pub(crate) kind: String,
    /// The last `seq` an items call took on this stream (2.1, case 7); 0 when none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub(crate) last: u64,
    /// A provider stream the host does not feed directly (`direct: false`, 10).
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) checked: bool,
    // Kept with the live stream cursor so an attaching page can see the pending question.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) consent: Option<String>,
    /// A request stream's head: the status, and whether the body is an event stream (9.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) head: Option<u16>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) sse: bool,
    /// A request stream whose response completed: its later items are of a finished request and
    /// are ignored (9.5 Items), deltas too.
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) done: bool,
    /// The body of a non-2xx response so far, its items joined (at most `ERROR_BODY` bytes): it
    /// is classified once it reads as JSON, or at `ended` (9.5).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) body: String,
    /// A job the FIFO's drain started, in a call that may be another owner's: its items and end
    /// run under that origin, so they send no model request (9.5 Slots).
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) drained: bool,
}

impl StreamKey {
    pub(crate) fn new(session: &str, kind: &str) -> Self {
        StreamKey {
            session: session.to_string(),
            kind: kind.to_string(),
            last: 0,
            checked: false,
            consent: None,
            head: None,
            sse: false,
            done: false,
            body: String::new(),
            drained: false,
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

const PROVIDER: &str = "provider";
const OWNER: &str = "owner";

fn stream_key(id: &str) -> String {
    format!("stream/{id}")
}

fn adapter_key(id: &str) -> String {
    format!("adapter/{id}")
}

fn key_of(owner: &str, key: &str) -> String {
    format!("key/{}/{key}", segment(owner))
}

fn owner_page_key(owner: &str, n: u32) -> String {
    format!("owner/{}/{n}", segment(owner))
}

/// The update of an `Out` that only forwards a delta: no entry, resolution, idle or usage.
pub(crate) fn delta_only(out: &adapters::Out) -> Option<Value> {
    let bare = out.entry.is_none()
        && out.resolved.is_none()
        && out.watane.is_none()
        && out.usage.is_none()
        && out.meter.is_none()
        && !out.idle;
    out.update.clone().filter(|_| bare)
}

/// An ACP update as it goes on the wire: `_meta.watane.at`, and `i` when it finishes an entry (9.4).
pub(crate) fn wire_update(mut update: Value, at: u64, i: Option<u64>) -> Value {
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

/// The live `entry` items of a stored entry (first indexed `first`) that has no ACP form (9.4): its
/// stored lines, so one over 256 KiB goes out as its parts; none for a `message` or a `tool`, which
/// go out as their finishing update.
fn entry_push(session: &str, first: u64, e: &NewEntry, at: u64, work: &mut Work) {
    let user = e.kind == "message" && e.body["role"] == "user";
    if matches!(e.kind.as_str(), "message" | "tool") && !user {
        return;
    }
    for line in crate::transcript::lines_of(e, at, first) {
        let entry: Value = serde_json::from_str(&line).expect("a stored line is JSON");
        work.pushes
            .push(Push::of(session, json!({"type": "entry", "entry": entry})));
    }
}

pub(crate) fn turn_push(t: &Turn) -> Value {
    json!({"type": "turn", "state": t.to_json()})
}

/// Whether `c` is white space for the slash rule: what JavaScript's `trim` removes plus Unicode
/// categories Z, Cc and Cf (host 19.3, 4.5 step 5).
fn slash_space(c: char) -> bool {
    let u = c as u32;
    c.is_whitespace()
        || matches!(u,
            0..=0x1F | 0x7F..=0x9F | 0xA0 | 0xAD | 0x600..=0x605 | 0x61C | 0x6DD | 0x70F
            | 0x890..=0x891 | 0x8E2 | 0x1680 | 0x180E | 0x2000..=0x200F | 0x2028..=0x202F
            | 0x205F..=0x2064 | 0x2066..=0x206F | 0x3000 | 0xFEFF | 0xFFF9..=0xFFFB
            | 0x110BD | 0x110CD | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A
            | 0xE0001 | 0xE0020..=0xE007F)
}

/// 9.3: an input whose first text block starts with `/` after white space is refused.
pub(crate) fn check_slash(input: &str) -> Result<(), KernelError> {
    let blocks = user_content(input);
    let text = blocks
        .as_array()
        .and_then(|b| b.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str());
    match text {
        Some(t) if t.trim_start_matches(slash_space).starts_with('/') => {
            Err(refused("input", "slash-command"))
        }
        _ => Ok(()),
    }
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

pub(crate) fn waiting(asks: Vec<String>, expires_at: Option<u64>) -> Turn {
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
            drain_due: false,
            queued: Vec::new(),
            foreign: false,
            timer_origin: None,
        }
    }

    /// Starts a top-level call: reads `slots` and the host's generation (R1). When they differ,
    /// this call starts the restart pass: the new boot, the stream slot counts zeroed (`timers`
    /// stays), the cursor at page 0, and the timer `restart` with delay 0. It steps no session.
    fn begin(&mut self) -> Result<Slots, KernelError> {
        self.open_meter();
        let mut slots = Slots::load(&mut self.state);
        let boot = self.host.generation();
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

    pub(crate) fn open_meter(&mut self) {
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
    pub(crate) fn accounts(&mut self, owner: &str) -> Acct {
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

    /// The session's live provider stream and whether it is `direct`: the first of its `streams`
    /// whose key says `provider`.
    fn live_target(&mut self, rec: &Record) -> Option<(String, bool)> {
        for s in &rec.streams {
            let Some(bytes) = self.state.get(&stream_key(s)) else {
                continue;
            };
            if let Ok(k) = serde_json::from_slice::<StreamKey>(&bytes) {
                if k.kind == PROVIDER {
                    return Some((s.clone(), !k.checked));
                }
            }
        }
        None
    }

    fn live_provider(&mut self, rec: &Record) -> Option<String> {
        self.live_target(rec).map(|t| t.0)
    }

    /// Where a request for the session's provider goes: its live stream, when it is a Claude
    /// session that holds one.
    fn target(&mut self, rec: &Record) -> Option<(String, bool)> {
        if rec.backend == "claude" {
            return self.live_target(rec);
        }
        None
    }

    /// A request that needs the session's live provider stream (2.1); none is sent without one.
    fn send_answer(&mut self, rec: &Record, work: &mut Work, ask: &str, decision: Value) {
        if let Some((stream, direct)) = self.target(rec) {
            work.requests.push(Request::Answer {
                session: rec.id.clone(),
                stream,
                direct,
                ask: ask.to_string(),
                decision,
            });
        }
    }

    fn send_interrupt(&mut self, rec: &Record, work: &mut Work) {
        if let Some((stream, direct)) = self.target(rec) {
            work.requests.push(Request::Interrupt {
                session: rec.id.clone(),
                stream,
                direct,
            });
        }
    }

    /// The profile's JSON: the stored one, or for `scratch` the built-in (9.1).
    pub(crate) fn profile_json(&mut self, name: &str) -> Option<String> {
        match self.state.get(&format!("profile/{name}")) {
            Some(b) => Some(String::from_utf8_lossy(&b).into_owned()),
            None => (name == SCRATCH).then(|| SCRATCH_PROFILE.to_string()),
        }
    }

    /// The launch of 4.1 for a session of profile `name`, `json`, with `overlay`.
    fn compile_launch(
        name: &str,
        json: &str,
        overlay: &Overlay,
        workspace: &str,
        resume: Option<&str>,
    ) -> String {
        let base = parse_profile(json).0;
        let compiled = apply_overlay(&base, overlay).0;
        launch(name, &compiled, !base.tools.is_empty(), workspace, resume)
    }

    /// The launch of a session that needs a new provider stream (9.5): its profile with the
    /// overlay its `start` took, in its workspace, resuming the Claude session it kept.
    fn launch_of(&mut self, rec: &Record, resume: bool) -> Result<String, KernelError> {
        let json = self
            .profile_json(&rec.profile)
            .ok_or_else(|| refused("profile", "no such profile"))?;
        // A later launch is checked as `start` is (9.2, 11, 14), against the refuse table of the
        // session's backend: the one its stream runs, whatever the edited profile now says.
        let overlay = rec.overlay.clone().unwrap_or_default();
        let backend = rec.backend.parse::<Backend>().unwrap_or(Backend::Claude);
        let model = parse_profile(&json)
            .0
            .model
            .unwrap_or_else(|| "default".to_string());
        let overlay_json = serde_json::to_string(&overlay).expect("an overlay serializes");
        let refusals = check(&json, &overlay_json, backend, &model, None);
        if !refusals.is_empty() {
            return Err(KernelError::Refused(refusals));
        }
        let workspace = rec
            .workspace
            .as_deref()
            .or(rec.requested_folder.as_deref())
            .unwrap_or(SCRATCH);
        let uuid = rec.claude_uuid.as_deref().filter(|_| resume);
        Ok(Self::compile_launch(
            &rec.profile,
            &json,
            &overlay,
            workspace,
            uuid,
        ))
    }

    /// The provider's `start` (2.2): the stream of a new Claude session, with the input of its
    /// first turn. It comes before the call's first write, so a provider that is not linked
    /// leaves nothing behind. A call that then fails cancels the stream (`guarded`).
    fn start_provider(&mut self, launch: &str, input: &str) -> Result<Started, KernelError> {
        let started = self
            .host
            .start_provider(launch, input)
            .map_err(KernelError::Backend)?;
        self.opened = Some(started.stream.clone());
        Ok(started)
    }

    /// Registers the session's provider stream `t`: its `stream/` key, the record's
    /// `streams`, and one slot of the kernel and of the owner. `slots` is saved by the caller.
    fn open_provider(
        &mut self,
        started: Started,
        rec: &mut Record,
        slots: &mut Slots,
        a: &mut Acct,
    ) -> Result<(), KernelError> {
        let key = StreamKey {
            session: rec.id.clone(),
            kind: PROVIDER.to_string(),
            last: 0,
            checked: !started.direct,
            consent: None,
            head: None,
            sse: false,
            done: false,
            body: String::new(),
            drained: false,
        };
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        self.state
            .put(&stream_key(&started.stream), bytes)
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        rec.add_stream(started.stream)?;
        rec.owed_idles = 0;
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
        self.drop_adapter(stream);
        rec.streams.retain(|s| s != stream);
        rec.owed_idles = 0;
        if rec.boot != slots.boot {
            return false;
        }
        slots.provider = slots.provider.saturating_sub(1);
        a.owner.close_provider(slots.boot);
        true
    }

    /// Deletes a stream's adapter state, when it has one (9.8 "Adapter state").
    fn drop_adapter(&mut self, stream: &str) {
        let key = adapter_key(stream);
        if self.state.get(&key).is_some() {
            self.state.delete(&key);
        }
    }

    /// `timers.after`, for at most `MAX_TIMER_MS`: a later deadline is armed at the limit, and
    /// that firing arms the rest (2.1, 7).
    fn set_timer(&mut self, id: &str, delay_ms: u64) {
        let delay_ms = delay_ms.min(MAX_TIMER_MS);
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

    pub(crate) fn cancel_provider(&mut self, stream: &str) {
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
    pub(crate) fn sync_timer(
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
                .profile_json(&rec.profile)
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
    pub(crate) fn finish_as(
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
        a: &mut Acct,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        if rec.boot < slots.boot {
            self.step(rec, a, slots, work)?;
        }
        Ok(())
    }

    fn session(
        &mut self,
        a: &mut Acct,
        caller: &str,
        id: &str,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<Record, KernelError> {
        let mut rec = self.load_owned(caller, id)?;
        self.settle(&mut rec, a, slots, work)?;
        Ok(rec)
    }

    /// The restart step of one session (R4): its stale `stream/` and `live/` keys go, its boot
    /// (and its page entry's) becomes the current one, a Claude session's open and pending asks
    /// close in every turn state, and a Claude turn nothing drives becomes `unknown`. Writes: at most its page (or `slots` and
    /// the page when it leaves the index), a decision entry per open ask and a `turn` entry (R3).
    // A running or waiting OpenAI turn becomes `unknown` with no model request, toolset
    // operation or consent (`openai_restart`, 9.5).
    fn step(
        &mut self,
        rec: &mut Record,
        a: &mut Acct,
        slots: &mut Slots,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let past = a.past;
        let now = self.host.now();
        rec.boot = slots.boot;
        let stale_streams = std::mem::take(&mut rec.streams);
        rec.live = None;
        let driven = rec.backend == "claude"
            && matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. });
        // Every open or pending Claude ask closes, whatever the turn state; a driven turn ends
        // `unknown`. An OpenAI session's asks are the kernel's own: `openai_restart` closes them.
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
            self.state.delete(&stream_key(&s));
            self.drop_adapter(&s);
        }
        self.state.delete(&format!("live/{}", rec.id));
        // R4: a child's delegated call belongs to the boot that opened it (9.7).
        rec.task_call = None;
        // R4 clears the root's live child map (9.3 `children`: live delegation only); the links
        // stay in each child's `parent` and the root's `child` entries. Cleared before the turn
        // stops, so the stop's cleanup touches no child (no recursive restart).
        rec.children.clear();
        // 9.5 "After a restart": a running or waiting OpenAI turn stops `unknown` and sends nothing;
        // a paused child loses its automatic resume (R4).
        if rec.backend == "openai"
            && rec.parent.is_some()
            && matches!(rec.turn, Turn::Paused { .. })
        {
            self.openai_restart_paused(rec, a, slots, work, now)?;
            self.sync_active(rec, slots, true)?;
        } else if rec.backend == "openai" && rec.turn.is_active() {
            self.openai_restart(rec, a, slots, work, now)?;
            if !rec.turn.is_active() {
                self.sync_active(rec, slots, true)?;
            }
        }
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

    /// `append`, and the entry goes out live as an `entry` item when it has no ACP form (9.4); a
    /// `message` or `tool` goes out as its finishing update.
    fn append_live(
        &mut self,
        rec: &mut Record,
        e: NewEntry,
        at: u64,
        work: &mut Work,
    ) -> Result<u64, KernelError> {
        let first = rec.meta().last + 1;
        let i = self.append(rec, e.clone(), at)?;
        entry_push(&rec.id, first, &e, at, work);
        Ok(i)
    }

    /// An entry the provider sends: stored unless the owner or the account is past its quota or the
    /// session is at its chunk cap, keeping the room for one `turn` entry (9.3 Quota, case 53). Not
    /// stored, the active turn ends `stopped {limit: quota}` (`None`).
    pub(crate) fn append_loose(
        &mut self,
        rec: &mut Record,
        past: Past,
        e: NewEntry,
        at: u64,
        work: &mut Work,
    ) -> Result<Option<u64>, KernelError> {
        if !past.is_past() {
            let mut meta = rec.meta();
            let first = meta.last + 1;
            let stored = Transcript::new(&mut self.state, &rec.id, &mut meta).append_within(
                &e,
                at,
                MAX_CHUNKS - 1,
            );
            match stored {
                Ok(i) => {
                    rec.set_meta(&meta);
                    entry_push(&rec.id, first, &e, at, work);
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
            self.send_interrupt(rec, work);
        }
        Ok(())
    }

    /// Stores a turn change: the record's turn and a `turn` entry, and pushes the turn. `activity`
    /// is live only (9.3): a change of it alone pushes and writes nothing.
    pub(crate) fn turn_to(
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
        // 9.3: an idle turn that ends a running one carries its whole seconds, rounded.
        if let (Turn::Running { since, .. }, Turn::Idle { seconds, .. }) = (&rec.turn, &mut stored)
        {
            seconds.get_or_insert(now.saturating_sub(*since).saturating_add(500) / 1000);
        }
        let t = if let Turn::Idle { .. } = t {
            stored.clone()
        } else {
            t
        };
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
            self.append_live(rec, entry, now, work)?;
        }
        rec.turn = stored;
        work.pushes.push(Push::of(&rec.id, turn_push(&t)));
        Ok(())
    }

    /// A user message, the `running` turn and the request: what a started turn is (AC1).
    /// `send`: the provider stream the input goes to as a request; none when the stream's own
    /// `start` carried it (10) or the session has no provider. Returns the message's index.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start_turn(
        &mut self,
        rec: &mut Record,
        past: Past,
        input: String,
        resume: bool,
        send: Option<(String, bool)>,
        now: u64,
        work: &mut Work,
    ) -> Result<u64, KernelError> {
        let message = NewEntry {
            kind: "message".into(),
            uuid: None,
            body: json!({"role": "user", "content": user_content(&input)}),
        };
        let n = self.append_live(rec, message, now, work)?;
        let running = Turn::Running {
            since: now,
            activity: None,
        };
        self.turn_to(rec, past, running, now, work)?;
        if let Some((stream, direct)) = send {
            work.requests.push(Request::Turn {
                session: rec.id.clone(),
                stream,
                direct,
                input,
                resume,
            });
        }
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
        if !self.may_work(&rec.owner.clone(), rec) {
            return Ok(());
        }
        let send = self.target(rec);
        self.start_turn(rec, past, next.input, false, send, now, work)?;
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
    pub(crate) fn close_ask(
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
    pub(crate) fn write_decision(
        &mut self,
        rec: &mut Record,
        past: Past,
        ask: (&str, &str),
        i: u64,
        decision: Value,
        now: u64,
        work: &mut Work,
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
        self.append_live(rec, entry, now, work)?;
        Ok(())
    }

    /// The kernel's own closing step (`ended`, R4, `cancel`): each pending answer becomes its
    /// decision, each open ask or check `deny, by: kernel, reason`; both end empty.
    /// `follow` lets a `waiting` turn follow (R4 sets its own turn). Whether any closed.
    pub(crate) fn close_all_with(
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
            self.write_decision(rec, past, (&o.id, &o.kind), o.i, d, now, work)?;
        }
        for (id, p) in std::mem::take(&mut rec.pending_answers) {
            self.write_decision(rec, past, (&id, &p.kind), p.i, p.decision(), now, work)?;
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
        self.write_decision(rec, past, (&r.id, &kind), i, decision, now, work)?;
        if open.is_some() {
            self.close_ask(rec, past, &r.id, now, work)
        } else {
            rec.pending_answers.remove(&r.id);
            Ok(())
        }
    }

    pub(crate) fn unknown_entry(
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
    pub(crate) fn sync_active(
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
    pub(crate) fn flush(&mut self, mut work: Work, live: Option<&str>) -> Vec<Push> {
        let mut all = Vec::new();
        for (l, pushes) in std::mem::take(&mut work.stopped) {
            self.deliver(&pushes, l.as_deref());
            all.extend(pushes);
        }
        self.deliver(&work.pushes, live);
        self.deliver_peer(&mut work);
        for r in work.requests {
            self.host.request(r);
        }
        all.extend(work.pushes);
        all
    }

    /// The pushes of children a stop of this call's session stopped go to each child's own live
    /// owner stream, before the session's own (its Stop comes last, 9.7; S134 r2 F2.2), then
    /// join the call's own.
    pub(crate) fn deliver_stopped(&mut self, work: &mut Work) {
        let mut theirs = Vec::new();
        for (l, pushes) in std::mem::take(&mut work.stopped) {
            self.deliver(&pushes, l.as_deref());
            theirs.extend(pushes);
        }
        work.pushes.splice(0..0, theirs);
    }

    /// The pushes a call made for its peer session (the root of a resolved `task` call) go to that
    /// session's live owner stream, then join the call's own.
    pub(crate) fn deliver_peer(&mut self, work: &mut Work) {
        if let Some((live, pushes)) = work.peer.take() {
            self.deliver(&pushes, live.as_deref());
            work.pushes.extend(pushes);
        }
    }

    /// The live owner stream of a session: `live/<session>` (9.1).
    pub(crate) fn live_stream(&mut self, session: &str) -> Option<String> {
        let bytes = self.state.get(&format!("live/{session}"))?;
        String::from_utf8(bytes).ok()
    }

    /// Pushes, in order, to the live stream of their session (`live`, from its record or its key).
    /// The first failure: the stream gets no more.
    pub(crate) fn send_live(&mut self, pushes: &[Push], live: Option<&str>) -> Option<StreamError> {
        let o = live?;
        pushes
            .iter()
            .find_map(|p| self.host.push(o, on_wire(&p.value)).err())
    }

    /// Outside an item call a full owner stream cannot be waited for: it is closed `lagging` (9.4).
    pub(crate) fn deliver(&mut self, pushes: &[Push], live: Option<&str>) {
        let (Some(o), Some(first)) = (live, pushes.first()) else {
            return;
        };
        if let Some(e) = self.send_live(pushes, live) {
            self.drop_failed(&first.session.clone(), o, e);
        }
    }

    /// A push to the live owner stream failed: `Full` closes it `lagging`; any other failure
    /// means it has ended meanwhile, so it is dropped and nothing is closed (2.1).
    fn drop_failed(&mut self, session: &str, stream: &str, e: StreamError) {
        let outcome = (e == StreamError::Full).then_some(Outcome::Lagging);
        self.drop_live(session, stream, outcome);
    }

    /// Closes the session's live owner stream (when `outcome` says so) and takes its keys and its
    /// place in `streams`, in one write of its own: its bytes go from the record and both accounts.
    fn drop_live(&mut self, session: &str, stream: &str, outcome: Option<Outcome>) {
        if let Some(outcome) = outcome {
            self.host.close_stream(stream, outcome);
        }
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
    /// The console's first run (S35): with no `access/page` at all, the page may use `scratch`.
    pub(crate) fn may_use(&mut self, owner: &str, profile: &str) -> bool {
        match self.state.get(&format!("access/{owner}")) {
            None => {
                owner == PAGE
                    && profile == SCRATCH
                    && self.state.get(&format!("profile/{SCRATCH}")).is_none()
            }
            Some(b) => serde_json::from_slice::<Vec<String>>(&b)
                .is_ok_and(|names| names.iter().any(|n| n == profile)),
        }
    }

    /// 9.7 "Revoking": whether a call that gives the session more work may go on. A root is
    /// checked against its owner's access; a child is checked as its root is, against the root's
    /// owner (it has no entry of its own), and a missing root is refused: fail closed.
    pub(crate) fn may_work(&mut self, owner: &str, rec: &Record) -> bool {
        match &rec.parent {
            None => self.may_use(owner, &rec.profile),
            Some(root) => match Record::load(&mut self.state, root) {
                Some(r) => self.may_use(&r.owner, &r.profile),
                None => false,
            },
        }
    }

    fn revoked() -> KernelError {
        refused("profile", "this client may not use this profile")
    }

    pub(crate) fn room(rec: &Record) -> Result<(), KernelError> {
        if rec.chunks.len() + CHUNK_MARGIN > MAX_CHUNKS {
            return Err(KernelError::Quota(format!(
                "{MAX_CHUNKS} transcript chunks per session"
            )));
        }
        Ok(())
    }

    /// 9.1: the first page of the owner's session ids with room, or the next one: its number,
    /// its key and its ids.
    pub(crate) fn owner_page(&mut self, caller: &str, acct: &Acct) -> (usize, String, Vec<String>) {
        let n = acct
            .owner
            .pages
            .iter()
            .position(|&c| (c as usize) < OWNER_PAGE)
            .unwrap_or(acct.owner.pages.len());
        let page_key = owner_page_key(caller, n as u32);
        let page: Vec<String> = if acct.owner.pages.get(n).is_some_and(|&c| c > 0) {
            self.state
                .get(&page_key)
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        (n, page_key, page)
    }

    pub fn start(
        &mut self,
        caller: &str,
        key: &str,
        profile: &str,
        overlay: &str,
        input: &str,
    ) -> Result<Called<String>, KernelError> {
        self.guarded(|k| k.start_call(caller, key, profile, overlay, input, None))
    }

    pub fn start_in_folder(
        &mut self,
        caller: &str,
        key: &str,
        profile: &str,
        overlay: &str,
        input: &str,
        folder: &str,
    ) -> Result<Called<String>, KernelError> {
        self.guarded(|k| k.start_call(caller, key, profile, overlay, input, Some(folder)))
    }

    fn start_call(
        &mut self,
        caller: &str,
        key: &str,
        profile: &str,
        overlay: &str,
        input: &str,
        folder: Option<&str>,
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
        check_slash(input)?;
        // 9.1: the profiles this caller may use; absent or unreadable allows none.
        if !self.may_use(caller, profile) {
            return Err(Self::revoked());
        }
        let Some(json) = self.profile_json(profile) else {
            return Err(refused("profile", "no such profile"));
        };
        let parsed = parse_profile(&json).0;
        let backend = parsed
            .backend
            .as_deref()
            .and_then(|b| b.parse::<Backend>().ok())
            .unwrap_or(Backend::Claude);
        // A Claude profile without a model runs the default one; an OpenAI one must name it (9.2).
        if backend == Backend::Openai && parsed.model.is_none() {
            return Err(refused("model", "an OpenAI profile needs a model"));
        }
        if let Some(folder) = folder {
            if backend != Backend::Claude || !folder.starts_with('/') {
                return Err(refused(
                    "folder",
                    "Claude Code needs an absolute folder path",
                ));
            }
            if folder.chars().any(char::is_control) {
                return Err(refused(
                    "folder",
                    "folder path contains a control character",
                ));
            }
            const MAX_FOLDER_BYTES: usize = 4096;
            if folder.len() > MAX_FOLDER_BYTES {
                return Err(KernelError::TooLarge(MAX_FOLDER_BYTES as u64));
            }
        }
        let model = parsed.model.as_deref().unwrap_or("default");
        let refusals = check(&json, overlay, backend, model, None);
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
        let (n, page_key, mut page) = self.owner_page(caller, &acct);
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
            requested_folder: folder.map(str::to_owned),
            ..Record::default()
        };
        // A later launch narrows as this one does (9.2).
        let narrowing = parse_overlay(overlay).0;
        rec.overlay = (narrowing != Overlay::default()).then(|| narrowing.clone());
        // The provider's `start` comes before the first write (2.1): none linked, nothing written.
        let started = match backend {
            Backend::Claude => {
                let launch = Self::compile_launch(
                    profile,
                    &json,
                    &narrowing,
                    folder.unwrap_or(SCRATCH),
                    None,
                );
                Some(self.start_provider(&launch, input)?)
            }
            _ => None,
        };
        // An OpenAI session's first request is sent, too, before the first write (9.5).
        let openai = match backend {
            Backend::Openai => {
                let compiled = apply_overlay(&parsed, &narrowing).0;
                Some(self.openai_start(&rec, input, &slots, &acct, compiled)?)
            }
            Backend::Claude => None,
        };
        let mut work = Work::default();
        let mut slots_dirty = false;
        // The provider's `start` carries the first turn's input (10): no `Request::Turn`.
        self.start_turn(
            &mut rec,
            past,
            input.to_string(),
            false,
            None,
            now,
            &mut work,
        )?;
        if let Some(t) = started {
            self.open_provider(t, &mut rec, &mut slots, &mut acct)?;
        }
        if let Some(p) = openai {
            let mut cx = Cx {
                slots: &mut slots,
                a: &mut acct,
                work: &mut work,
                past,
                now,
                dirty: false,
            };
            self.openai_finish(&mut rec, p, &mut cx)?;
            slots_dirty = cx.dirty;
            slots_dirty |= self.sync_timer(&rec, &mut slots, &mut acct, &[], now, false);
        }
        // The first turn is active: this saves `slots` with the slot taken.
        self.sync_active(&mut rec, &mut slots, false)?;
        if slots_dirty {
            slots.save(&mut self.state)?;
        }
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
        check_slash(input)?;
        let mut rec = self.load_owned(caller, session)?;
        if !self.may_work(caller, &rec) {
            return Err(Self::revoked());
        }
        let mut acct = self.accounts(caller);
        let past = acct.past;
        self.refuse_past(&acct)?;
        let mut work = Work::default();
        self.settle(&mut rec, &mut acct, &mut slots, &mut work)?;
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
                let turn_input = match &queued {
                    None => input,
                    Some((oldest, _)) => oldest.input.as_str(),
                };
                // A stream the turn needs is opened before the call's first write, with the
                // input the turn starts with (10).
                let started = match open {
                    true => {
                        let launch = self.launch_of(&rec, resume)?;
                        Some(self.start_provider(&launch, turn_input)?)
                    }
                    false => None,
                };
                let send = match started {
                    Some(_) => None,
                    None => self.target(&rec),
                };
                // An OpenAI turn sends its request before the first write (9.5).
                let openai = match rec.backend.as_str() {
                    "openai" => {
                        Some(self.openai_prelaunch(&rec, Some(turn_input), &slots, &acct, false)?)
                    }
                    _ => None,
                };
                let now = self.host.now();
                let n = match queued {
                    None => self.start_turn(
                        &mut rec,
                        past,
                        input.to_string(),
                        resume,
                        send,
                        now,
                        &mut work,
                    )?,
                    Some((oldest, n)) => {
                        self.start_turn(
                            &mut rec,
                            past,
                            oldest.input,
                            resume,
                            send,
                            now,
                            &mut work,
                        )?;
                        q.save(&mut self.state, &rec.id)?;
                        work.pushes.push(Push::of(&rec.id, q.push_value()));
                        n
                    }
                };
                if let Some(t) = started {
                    self.open_provider(t, &mut rec, &mut slots, &mut acct)?;
                }
                let mut sent = false;
                if let Some(p) = openai {
                    let mut cx = Cx {
                        slots: &mut slots,
                        a: &mut acct,
                        work: &mut work,
                        past,
                        now,
                        dirty: false,
                    };
                    self.openai_finish(&mut rec, p, &mut cx)?;
                    sent = cx.dirty;
                }
                let timed = self.idle_after(&mut rec, &mut slots, &mut acct, was, &before, now);
                if !self.sync_active(&mut rec, &mut slots, was)? && (open || timed || sent) {
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
        if d.allow && !self.may_work(caller, &rec) {
            return Err(Self::revoked());
        }
        let mut acct = self.accounts(caller);
        let past = acct.past;
        if d.for_session {
            self.refuse_past(&acct)?;
        }
        // The step comes after the refusals that do not depend on it; an ask it denies is refused.
        let mut work = Work::default();
        self.settle(&mut rec, &mut acct, &mut slots, &mut work)?;
        let open = rec
            .open
            .iter()
            .find(|o| o.id == ask && o.kind == "ask")
            .cloned()
            .ok_or_else(|| refused("ask", "no open ask with this id"))?;
        if rec.backend == "openai" {
            return self.openai_answer(caller, session, key, ask, d, rec, acct, slots, work);
        }
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
        let now = self.host.now();
        if d.for_session {
            // The allow grants only what the ask offered: the scope computed now must be the
            // reach the ask was opened with (9.6, 14; S36 r1).
            let offered = body.get("reach").and_then(Value::as_str);
            let Some((tool, key)) = Self::ask_scope(&rec, &body)
                .filter(|(t, k)| offered == Some(broker::render(t, k).as_str()))
            else {
                return Err(refused("decision", "this ask offers no session allow"));
            };
            if !rec.allows.iter().any(|a| a.tool == tool && a.key == key) {
                // The allow's id is the index of its entry, written before the ask closes.
                let id = rec.next_index.to_string();
                rec.add_allow(Allow {
                    id: id.clone(),
                    tool: tool.clone(),
                    key: key.clone(),
                })?;
                // Its push goes before the ask's close and any turn entry it causes, in
                // transcript order (S36 r1).
                let push = self.write_allow(&mut rec, &id, &tool, &key, true, now)?;
                work.pushes.push(push);
            }
        }
        self.close_ask(&mut rec, past, ask, now, &mut work)?;
        rec.pending_answers.insert(ask.to_string(), pending);
        let mut decision = d.value;
        if let Some(o) = decision.as_object_mut() {
            // The host never gets `for`: `claude` would keep a rule the kernel cannot take back.
            o.remove("for");
        }
        self.send_answer(&rec, &mut work, ask, decision);
        rec.updated = now;
        self.seal(&mut rec)?;
        self.key_put("answer", caller, key, Some(session), &())?;
        self.finish(None, &mut acct)?;
        Ok(Called {
            value: (),
            pushes: self.flush(work, rec.live.as_deref()),
        })
    }

    /// `answer` on an OpenAI ask (9.6): the kernel's own ask, so the owner's answer is its
    /// decision at once. The call runs, or gets the denial as its output, and when no ask of the
    /// step is left the loop goes on.
    #[allow(clippy::too_many_arguments)]
    fn openai_answer(
        &mut self,
        caller: &str,
        session: &str,
        key: &str,
        ask: &str,
        d: Decision,
        mut rec: Record,
        mut acct: Acct,
        mut slots: Slots,
        mut work: Work,
    ) -> Result<Called<()>, KernelError> {
        if d.for_session {
            // The tools of a toolset have no scope function: an OpenAI ask offers no reach.
            return Err(refused("decision", "this ask offers no session allow"));
        }
        let past = acct.past;
        let now = self.host.now();
        let before = rec.clone();
        let was = rec.turn.is_active();
        let mut ls = self
            .loop_load(&rec.id)
            .ok_or_else(|| refused("ask", "no open ask with this id"))?;
        let before_ls = ls.clone();
        let mut cx = Cx {
            slots: &mut slots,
            a: &mut acct,
            work: &mut work,
            past,
            now,
            dirty: false,
        };
        self.decide_ask(
            &mut rec,
            &mut ls,
            &mut cx,
            ask,
            d.allow,
            "owner",
            d.reason.as_deref(),
        )?;
        self.after_asks(&mut rec, &mut ls, &mut cx)?;
        let mut dirty = cx.dirty;
        self.settle_openai(
            &mut rec, &before, &mut slots, &mut acct, was, now, &mut dirty,
        )?;
        if dirty {
            slots.save(&mut self.state)?;
        }
        if ls != before_ls {
            self.loop_save(&rec.id, &ls)?;
        }
        rec.updated = now;
        self.seal(&mut rec)?;
        self.key_put("answer", caller, key, Some(session), &())?;
        self.finish(None, &mut acct)?;
        let pushes = self.flush(work, rec.live.as_deref());
        self.after_call();
        Ok(Called { value: (), pushes })
    }

    /// Writes the `allow` entry of a session allow given or taken back (9.4); its live push.
    fn write_allow(
        &mut self,
        rec: &mut Record,
        id: &str,
        tool: &str,
        key: &str,
        given: bool,
        now: u64,
    ) -> Result<Push, KernelError> {
        let reach = broker::render(tool, key);
        let body = json!({"id": id, "tool": tool, "key": key, "reach": reach, "given": given, "by": "owner"});
        let entry = NewEntry {
            kind: "allow".into(),
            uuid: None,
            body: body.clone(),
        };
        let i = self.append(rec, entry, now)?;
        let entry = json!({"v": 1, "i": i, "kind": "allow", "at": now, "body": body});
        Ok(Push::of(&rec.id, json!({"type": "entry", "entry": entry})))
    }

    /// 2.3 `take-back`: removes a session allow and writes its `allow` entry (9.6). It only
    /// narrows, so it reads no `access/` and is never refused `quota` (9.3 "Past").
    pub fn take_back(
        &mut self,
        caller: &str,
        session: &str,
        allow: &str,
    ) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut rec = self.load_owned(caller, session)?;
        let at = rec
            .allows
            .iter()
            .position(|a| a.id == allow)
            .ok_or_else(|| KernelError::UnknownAllow(allow.to_string()))?;
        let mut acct = self.accounts(caller);
        let mut work = Work::default();
        self.settle(&mut rec, &mut acct, &mut slots, &mut work)?;
        let taken = rec.allows.remove(at);
        let now = self.host.now();
        let push = self.write_allow(&mut rec, &taken.id, &taken.tool, &taken.key, false, now)?;
        work.pushes.push(push);
        rec.updated = now;
        self.seal(&mut rec)?;
        self.finish_as(None, &mut acct, true)?;
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
            check_slash(input)?;
            if !self.may_work(caller, &rec) {
                return Err(Self::revoked());
            }
            self.refuse_past(&acct)?;
        }
        let mut work = Work::default();
        self.settle(&mut rec, &mut acct, &mut slots, &mut work)?;
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
        let started = match (plan.0, replacement) {
            (true, Some(input)) => {
                let launch = self.launch_of(&rec, plan.1)?;
                Some(self.start_provider(&launch, input)?)
            }
            _ => None,
        };
        // An OpenAI replacement sends its request before the first write; with no replacement the
        // session's loop is stopped below (9.5).
        let mut openai = match rec.backend.as_str() {
            "openai" => Some(self.openai_prelaunch(&rec, replacement, &slots, &acct, true)?),
            _ => None,
        };
        // The stream the turn ran on, before a new one is registered.
        let live = self.target(&rec);
        let now = self.host.now();
        let mut sent = false;
        if was {
            // The calls and asks of an OpenAI loop end before its turn does (9.5).
            if let Some(p) = openai.as_mut() {
                let mut cx = Cx {
                    slots: &mut slots,
                    a: &mut acct,
                    work: &mut work,
                    past,
                    now,
                    dirty: false,
                };
                // 9.7: each live child ends `interrupted` before the root's calls are aborted.
                self.stop_children(&mut rec, &p.ls, &mut cx, false)?;
                self.openai_stop(&mut rec, &mut p.ls, &mut cx)?;
                sent = cx.dirty;
            }
            self.end_interrupted(&mut rec, past, now, &mut work)?;
            // 9.7: the owner interrupting a child's first turn resolves the root's call.
            if rec.task_call.is_some() {
                let mut cx = Cx {
                    slots: &mut slots,
                    a: &mut acct,
                    work: &mut work,
                    past,
                    now,
                    dirty: false,
                };
                self.end_task(&mut rec, &mut cx, None)?;
                sent |= cx.dirty;
            }
            if let Some((stream, direct)) = live.clone() {
                // `claude` ends the interrupted turn with an `idle` of its own, later (S36).
                rec.owed_idles = rec.owed_idles.saturating_add(1);
                work.requests.push(Request::Interrupt {
                    session: rec.id.clone(),
                    stream,
                    direct,
                });
            }
        }
        let (open, resume) = plan;
        if let Some(input) = replacement {
            let send = match started {
                Some(_) => None,
                None => live,
            };
            self.start_turn(
                &mut rec,
                past,
                input.to_string(),
                resume,
                send,
                now,
                &mut work,
            )?;
        }
        if let Some(t) = started {
            self.open_provider(t, &mut rec, &mut slots, &mut acct)?;
        }
        if let Some(p) = openai {
            let mut cx = Cx {
                slots: &mut slots,
                a: &mut acct,
                work: &mut work,
                past,
                now,
                dirty: false,
            };
            self.openai_finish(&mut rec, p, &mut cx)?;
            sent |= cx.dirty;
        }
        let timed = self.idle_after(&mut rec, &mut slots, &mut acct, was, &before, now);
        if !self.sync_active(&mut rec, &mut slots, was)? && (open || timed || sent) {
            slots.save(&mut self.state)?;
        }
        rec.updated = now;
        // 9.3 "Past": an interrupt without a replacement only ends the turn.
        self.finish_as(Some(&mut rec), &mut acct, replacement.is_none())?;
        let pushes = self.flush(work, rec.live.as_deref());
        self.after_call();
        Ok(Called { value: (), pushes })
    }

    pub(crate) fn end_interrupted(
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
    /// `idle {interrupted}`, the queue is kept, the host ends the stream. An OpenAI root also
    /// cancels each live child (9.7).
    pub fn cancel(&mut self, caller: &str, session: &str) -> Result<Called<()>, KernelError> {
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let past = acct.past;
        let mut work = Work::default();
        let mut rec = self.session(&mut acct, caller, session, &mut slots, &mut work)?;
        let now = self.host.now();
        {
            let mut cx = Cx {
                slots: &mut slots,
                a: &mut acct,
                work: &mut work,
                past,
                now,
                dirty: false,
            };
            self.cancel_rec(&mut rec, &mut cx)?;
        }
        // 9.3 "Past": a cancel closes the session's work, so it cannot fail on quota.
        self.finish_as(Some(&mut rec), &mut acct, true)?;
        let pushes = self.flush(work, rec.live.as_deref());
        self.after_call();
        Ok(Called { value: (), pushes })
    }

    /// The writes of a cancel on one session, the caller saving the record. A child's cancel also
    /// resolves its root's call `aborted` (9.7).
    pub(crate) fn cancel_rec(&mut self, rec: &mut Record, cx: &mut Cx) -> Result<(), KernelError> {
        let (past, now) = (cx.past, cx.now);
        let was = rec.turn.is_active();
        let closed = self.close_all_with(rec, past, "cancel", now, cx.work, false)?;
        // An OpenAI session's request streams and jobs end the same way, its calls get `aborted`
        // and its handle closes (9.5).
        if rec.backend == "openai" {
            if let Some(mut ls) = self.loop_load(&rec.id) {
                // The deadlines go below, with the timer they hold.
                let kept = rec.deadlines.clone();
                self.stop_children(rec, &ls, cx, true)?;
                self.openai_stop(rec, &mut ls, cx)?;
                rec.deadlines = kept;
                self.loop_save(&rec.id, &ls)?;
                self.close_tools(&ls);
            }
        }
        if was {
            self.end_interrupted(rec, past, now, cx.work)?;
        }
        if rec.task_call.is_some() {
            self.abort_task(rec, cx)?;
        }
        // The host's `Cancel` ends the provider stream: its slot is free at once, and its later
        // `ended` finds no key (9.4).
        let provider = self.live_provider(rec);
        let mut freed = cx.dirty
            || match &provider {
                Some(t) => self.release_provider(rec, t, cx.slots, cx.a),
                None => false,
            };
        // 9.3: no deadline outlives a cancel; its timer goes with them.
        let before = std::mem::take(&mut rec.deadlines);
        freed |= self.sync_timer(rec, cx.slots, cx.a, &before, now, false);
        if !self.sync_active(rec, cx.slots, was)? && freed {
            cx.slots.save(&mut self.state)?;
        }
        cx.work.requests.push(Request::Cancel {
            session: rec.id.clone(),
            stream: provider.unwrap_or_default(),
        });
        if was || closed {
            rec.updated = now;
        }
        Ok(())
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
            self.settle(&mut rec, &mut acct, slots, &mut Work::default())?;
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
        let mut rec = self.session(&mut acct, caller, session, &mut slots, &mut work)?;
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
        let page = Transcript::new(&mut self.state, &rec.id, &mut meta).read(after, before, max)?;
        if slots.drain_owed {
            self.drain_owed();
        }
        Ok(page)
    }

    pub fn rename(&mut self, caller: &str, session: &str, title: &str) -> Result<(), KernelError> {
        check_title(title)?;
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        let mut rec = self.load_owned(caller, session)?;
        self.refuse_past(&acct)?;
        self.settle(&mut rec, &mut acct, &mut slots, &mut Work::default())?;
        rec.title = title.to_string();
        self.finish(Some(&mut rec), &mut acct)
    }

    /// 2.3 `delete`: frees the session's bytes, as its record holds them, without reading its
    /// chunks; the kept `key/` results stay charged (9.3 Quota).
    pub fn delete(&mut self, caller: &str, session: &str) -> Result<(), KernelError> {
        let mut slots = self.begin()?;
        let mut acct = self.accounts(caller);
        // No restart step (9.3): a session can always be deleted.
        let mut rec = self.load_owned(caller, session)?;
        // 9.7: a live child's deletion resolves its root's call `aborted`; a root's deletion
        // cancels each live child (their records stay). Both are of this boot only.
        let mut work = Work::default();
        let mut stopped = false;
        if rec.boot == slots.boot {
            let now = self.host.now();
            let past = acct.past;
            let mut cx = Cx {
                slots: &mut slots,
                a: &mut acct,
                work: &mut work,
                past,
                now,
                dirty: false,
            };
            if rec.task_call.is_some() {
                self.abort_task(&mut rec, &mut cx)?;
            }
            if rec.backend == "openai" {
                if let Some(ls) = self.loop_load(&rec.id) {
                    let mut root = rec.clone();
                    self.stop_children(&mut root, &ls, &mut cx, true)?;
                }
            }
            stopped = cx.dirty;
        }
        // The record's bytes.
        let freed = rec.bytes as i64;
        let page_key = owner_page_key(caller, rec.owner_page);
        let page: Vec<String> = self
            .state
            .get(&page_key)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        // The slots of this boot's provider streams; a stale record's streams were counted zero.
        let mut slot_freed = stopped;
        let mut owners = Vec::new();
        let mut requests = Vec::new();
        let mut jobs = Vec::new();
        let mut provider = String::new();
        if rec.boot == slots.boot {
            for s in &rec.streams {
                let bytes = self.state.get(&stream_key(s));
                let kind = bytes.and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok());
                match kind.map(|k| k.kind) {
                    Some(k) if k == PROVIDER => {
                        provider = s.clone();
                        slots.provider = slots.provider.saturating_sub(1);
                        acct.owner.close_provider(slots.boot);
                        slot_freed = true;
                    }
                    Some(k) if k == OWNER => owners.push(s.clone()),
                    Some(k) if k == JOB => {
                        jobs.push(s.clone());
                        slots.job = slots.job.saturating_sub(1);
                        acct.owner.close_job(slots.boot);
                        self.drain_due = true;
                        slot_freed = true;
                    }
                    Some(k) if k == REQUEST => {
                        requests.push(s.clone());
                        self.drain_due = true;
                        slots.request = slots.request.saturating_sub(1);
                        acct.owner.close_request(slots.boot);
                        slot_freed = true;
                    }
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
        if rec.backend == "openai" {
            self.openai_delete(&rec.id);
        }
        for s in &rec.streams {
            self.state.delete_uncharged(&stream_key(s));
            self.state.delete_uncharged(&adapter_key(s));
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
            self.host.close_stream(&o, Outcome::Closed);
        }
        for r in requests.into_iter().chain(jobs) {
            self.host.cancel_stream(&r);
        }
        self.host.request(Request::Cancel {
            session: rec.id.clone(),
            stream: provider,
        });
        // What the stopped children and the resolved root left: their pushes and requests.
        self.flush(work, None);
        self.after_call();
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
                // A Claude or an OpenAI turn, `running` or `waiting`, becomes `unknown` (R4).
                let stale = r.boot < slots.boot
                    && (matches!(r.turn, Turn::Running { .. } | Turn::Waiting { .. })
                        || (r.parent.is_some() && matches!(r.turn, Turn::Paused { .. })));
                let turn = if stale { Turn::Unknown } else { r.turn };
                out.push(SessionInfo {
                    id: r.id,
                    title: r.title,
                    profile: r.profile,
                    backend: r.backend,
                    workspace: r
                        .workspace
                        .filter(|path| path.starts_with('/'))
                        .or(r.requested_folder),
                    turn: turn.to_json().to_string(),
                    updated: r.updated,
                });
            }
        }
        Ok(out)
    }

    /// The `stream/<id>` of a provider stream; `None` for an id with no key (never opened or
    /// ended) and for an owner stream's id, which is no provider item (9.1 `stream/<id>`).
    fn provider_key(&mut self, stream: &str) -> Option<StreamKey> {
        self.state
            .get(&stream_key(stream))
            .and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok())
            .filter(|k| k.kind == PROVIDER)
    }

    /// One item of a provider's stream, by its id: `stream/<id>` is read first and names the
    /// session (9.4). An id with no key (never opened or ended) changes nothing and sends
    /// nothing; an id of an older boot is no different to the code, and reaches it only if the
    /// host breaks R1 (items come on streams of this boot). When the session's live owner stream is full, the call is
    /// dropped whole and answers `Defer` (3, "Deferred batch"). It does not keep a `seq`
    /// (`provider_batch` does).
    pub fn provider_item(
        &mut self,
        stream: &str,
        event: ProviderEvent,
    ) -> Result<Called<SinkOutcome>, KernelError> {
        if matches!(event, ProviderEvent::Ended) {
            let done = self.provider_ended(stream)?;
            return Ok(Called {
                value: SinkOutcome::Taken,
                pushes: done.pushes,
            });
        }
        self.open_meter();
        let Some(key) = self.provider_key(stream) else {
            return Ok(Called {
                value: SinkOutcome::Taken,
                pushes: Vec::new(),
            });
        };
        self.hold();
        let step = self.item_step(stream, &key.session, event);
        self.finish_items(step)
    }

    /// `stream-sink.ended` of a provider stream: it cannot defer, so it is applied in full (asks
    /// closed, slot and key released, turn settled) and its pushes go to the live owner stream, or,
    /// when that is full, close it `lagging` (9.4, outside an item call).
    pub fn provider_ended(&mut self, stream: &str) -> Result<Called<()>, KernelError> {
        self.provider_ended_with(stream, None)
    }

    /// `provider_ended` with the cause the stream's outcome gives when it is anything but
    /// `closed` (2.1): an active turn ends `idle {failed {error: provider-ended, cause}}`.
    pub fn provider_ended_with(
        &mut self,
        stream: &str,
        cause: Option<String>,
    ) -> Result<Called<()>, KernelError> {
        self.open_meter();
        if let Some(key) = self.loop_key(stream) {
            return self.request_ended(stream, key, cause.as_deref());
        }
        let Some(key) = self.provider_key(stream) else {
            return Ok(Called {
                value: (),
                pushes: Vec::new(),
            });
        };
        let rec = Record::load(&mut self.state, &key.session)
            .ok_or_else(|| KernelError::UnknownSession(key.session.clone()))?;
        let work = self.provider_work(rec, Some(stream), ProviderEvent::Ended, cause)?;
        let live = work.live.clone();
        Ok(Called {
            value: (),
            pushes: self.flush(work, live.as_deref()),
        })
    }

    /// 2.1, case 7: a numbered batch of a provider's stream. Items at or below the last `seq`
    /// taken are dropped, the rest are taken in `seq` order, and the last `seq` is kept in
    /// `stream/<id>`, written once. When any item defers the whole batch does (nothing is kept,
    /// the host sends it again).
    pub fn provider_batch(
        &mut self,
        stream: &str,
        items: Vec<(u64, ProviderEvent)>,
    ) -> Result<Called<SinkOutcome>, KernelError> {
        self.open_meter();
        let Some(key) = self.provider_key(stream) else {
            return Ok(Called {
                value: SinkOutcome::Taken,
                pushes: Vec::new(),
            });
        };
        let mut fresh: Vec<_> = items.into_iter().filter(|(n, _)| *n > key.last).collect();
        fresh.sort_by_key(|(n, _)| *n);
        let Some(last) = fresh.last().map(|(n, _)| *n) else {
            return Ok(Called {
                value: SinkOutcome::Taken,
                pushes: Vec::new(),
            });
        };
        self.hold();
        let mut done = (Vec::new(), Vec::new(), Vec::new());
        let mut prev = key.last;
        for (seq, event) in fresh {
            if seq == prev {
                continue;
            }
            prev = seq;
            self.open_meter();
            match self.item_step(stream, &key.session, event) {
                Ok(Step::Done {
                    mut pushes,
                    mut requests,
                    gone,
                }) => {
                    done.0.append(&mut pushes);
                    done.1.append(&mut requests);
                    done.2.extend(gone);
                }
                other => return self.finish_items(other),
            }
        }
        let kept = self.take_seq(stream, last, None, key.consent);
        let (pushes, requests, gone) = done;
        self.finish_items(kept.map(|()| Step::Done {
            pushes,
            requests,
            gone,
        }))
    }

    /// 2.1, 9.8 "Adapter state": a numbered batch of a Claude provider stream's own lines. Each
    /// body goes through the stream's adapter, whose state `adapter/<stream>` keeps between calls
    /// (read once, written once, with the stream's `last`); the facts a line gives about the
    /// session (`initialized`, `system/init`) go to the record and give no entry. Each `Out` is
    /// acted on as `provider_batch` acts on a `ProviderEvent::Out`. A batch that defers or fails
    /// keeps nothing, the adapter's state included.
    pub fn provider_lines(
        &mut self,
        stream: &str,
        items: Vec<(u64, Vec<u8>)>,
    ) -> Result<Called<SinkOutcome>, KernelError> {
        self.open_meter();
        let taken = || {
            Ok(Called {
                value: SinkOutcome::Taken,
                pushes: Vec::new(),
            })
        };
        // A model request's stream (9.5): the OpenAI loop reads its items.
        if let Some(key) = self.loop_key(stream) {
            return self.request_lines(stream, key, items);
        }
        let Some(key) = self.provider_key(stream) else {
            return taken();
        };
        let mut fresh: Vec<_> = items.into_iter().filter(|(n, _)| *n > key.last).collect();
        fresh.sort_by_key(|(n, _)| *n);
        let Some(last) = fresh.last().map(|(n, _)| *n) else {
            return taken();
        };
        self.hold();
        let kept = self.state.get(&adapter_key(stream));
        let mut adapter = ClaudeAdapter::from_state(kept.as_deref(), !key.checked);
        let before = adapter.to_state();
        let mut consent = key.consent;
        let mut done = (Vec::new(), Vec::new(), Vec::new());
        let mut prev = key.last;
        for (seq, body) in fresh {
            if seq == prev {
                continue;
            }
            prev = seq;
            let facts = facts(&body);
            let events: Vec<ProviderEvent> = if facts.is_empty() {
                let at = self.host.now();
                let outs = adapter.line(&body, at);
                let unread = unread_init(&body).then_some(ProviderEvent::Folder(None));
                unread
                    .into_iter()
                    .chain(outs.into_iter().map(ProviderEvent::Out))
                    .collect()
            } else {
                facts
                    .into_iter()
                    .flat_map(|f| match f {
                        Fact::ConsentPending(what) => {
                            consent = Some(what.clone());
                            vec![ProviderEvent::Consent(Some(what))]
                        }
                        Fact::Workspace(id) => {
                            let clear = consent.take().map(|_| ProviderEvent::Consent(None));
                            [Some(ProviderEvent::Workspace(id)), clear]
                                .into_iter()
                                .flatten()
                                .collect()
                        }
                        Fact::Session(uuid) => vec![ProviderEvent::ClaudeSession(uuid)],
                        Fact::Folder(cwd) => vec![ProviderEvent::Folder(cwd)],
                    })
                    .collect()
            };
            for event in events {
                self.open_meter();
                match self.item_step(stream, &key.session, event) {
                    Ok(Step::Done {
                        mut pushes,
                        mut requests,
                        gone,
                    }) => {
                        done.0.append(&mut pushes);
                        done.1.append(&mut requests);
                        done.2.extend(gone);
                    }
                    other => return self.finish_items(other),
                }
            }
        }
        let after = adapter.to_state();
        let state = (after != before).then_some(after);
        let kept = self.take_seq(stream, last, state, consent);
        let (pushes, requests, gone) = done;
        self.finish_items(kept.map(|()| Step::Done {
            pushes,
            requests,
            gone,
        }))
    }

    /// Starts an items call: its writes and the host calls it makes outside `request` are held.
    pub(crate) fn hold(&mut self) {
        self.state.hold();
        self.held = Some(Vec::new());
    }

    /// Ends an items call: dropped whole on an error or a deferral, else committed, its requests
    /// sent and any owner stream that turned out to have ended dropped.
    pub(crate) fn finish_items(
        &mut self,
        step: Result<Step, KernelError>,
    ) -> Result<Called<SinkOutcome>, KernelError> {
        let (pushes, requests, gone) = match step {
            Err(e) => {
                self.drop_held();
                return Err(e);
            }
            Ok(Step::Defer) => {
                self.drop_held();
                return Ok(Called {
                    value: SinkOutcome::Defer,
                    pushes: Vec::new(),
                });
            }
            Ok(Step::Done {
                pushes,
                requests,
                gone,
            }) => (pushes, requests, gone),
        };
        self.commit_held();
        for r in requests {
            self.host.request(r);
        }
        let mut seen = BTreeSet::new();
        for (session, stream) in gone {
            if seen.insert(stream.clone()) {
                self.drop_live(&session, &stream, None);
            }
        }
        Ok(Called {
            value: SinkOutcome::Taken,
            pushes,
        })
    }

    /// Keeps `last` in `stream/<id>`, unless the batch ended the stream; its bytes go from the
    /// record and both accounts.
    fn take_seq(
        &mut self,
        stream: &str,
        last: u64,
        adapter: Option<Vec<u8>>,
        consent: Option<String>,
    ) -> Result<(), KernelError> {
        let Some(mut key) = self
            .state
            .get(&stream_key(stream))
            .and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok())
        else {
            return Ok(());
        };
        self.open_meter();
        key.last = last;
        key.consent = consent;
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        self.state
            .put(&stream_key(stream), bytes)
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        if let Some(a) = adapter {
            self.state
                .put(&adapter_key(stream), a)
                .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        }
        // P09, case 56: a batch whose writes changed no byte count (a text delta: `last` of the
        // same width, the adapter unchanged) charges nothing, so it reads no record or account.
        let c = self.state.change();
        if c.owner == 0 && c.session == 0 {
            self.state.close();
            return Ok(());
        }
        match Record::load(&mut self.state, &key.session) {
            Some(mut rec) => {
                let mut acct = self.accounts(&rec.owner);
                self.finish_as(Some(&mut rec), &mut acct, true)
            }
            None => {
                self.state.close();
                Ok(())
            }
        }
    }

    /// One item's work and its pushes to the session's live owner stream, as part of an items
    /// call: `Defer` when that stream is full. A stream that has ended is reported in `gone`.
    fn item_step(
        &mut self,
        stream: &str,
        session: &str,
        event: ProviderEvent,
    ) -> Result<Step, KernelError> {
        // 9.4, P09: an item that only forwards a delta reads `stream/<id>` and `live/<session>`.
        let (pushes, requests, live) = match &event {
            ProviderEvent::Out(out) if delta_only(out).is_some() => {
                let update = delta_only(out).expect("checked");
                let now = self.host.now();
                let push = Push::of(session, wire_update(update, now, None));
                (vec![push], Vec::new(), self.live_stream(session))
            }
            _ => {
                let rec = Record::load(&mut self.state, session)
                    .ok_or_else(|| KernelError::UnknownSession(session.to_string()))?;
                let work = self.provider_work(rec, Some(stream), event, None)?;
                (work.pushes, work.requests, work.live)
            }
        };
        let failed = self.send_live(&pushes, live.as_deref());
        if failed == Some(StreamError::Full) {
            return Ok(Step::Defer);
        }
        let gone = live
            .filter(|_| failed.is_some())
            .map(|o| (session.to_string(), o))
            .into_iter()
            .collect();
        Ok(Step::Done {
            pushes,
            requests,
            gone,
        })
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
        let work = self.provider_work(rec, stream.as_deref(), event, None)?;
        let live = work.live.clone();
        Ok(Called {
            value: (),
            pushes: self.flush(work, live.as_deref()),
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
        let mut rec = self.load_owned(caller, session)?;
        let stale = rec.boot < slots.boot;
        let mut work = Work::default();
        self.settle(&mut rec, &mut acct, &mut slots, &mut work)?;
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
        let o = self
            .host
            .open_stream()
            .map_err(|e| KernelError::Backend(format!("{}: {}", e.code(), e.cause())))?;
        let written = self.write_owner(&mut rec, &mut acct, &o, previous.as_deref());
        if let Err(e) = written {
            self.host.close_stream(&o, Outcome::Closed);
            return Err(e);
        }
        if let Some(p) = &previous {
            self.host.close_stream(p, Outcome::Replaced);
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
        let mut turn = rec.turn.clone();
        if rec.backend == "openai" {
            if let Turn::Running { activity, .. } = &mut turn {
                *activity = self.openai_activity(&rec);
            }
        }
        if rec.requested_folder.is_some() {
            if let Turn::Running { activity, .. } = &mut turn {
                *activity = self
                    .live_provider(&rec)
                    .and_then(|stream| self.provider_key(&stream))
                    .and_then(|key| key.consent)
                    .map(|what| crate::record::Activity::Consent { what });
            }
        }
        items.push(json!({"_meta": {"watane": turn_push(&turn)}}));
        items.push(json!({"_meta": {"watane": queue}}));
        if let Some(u) = &rec.usage {
            items.push(json!({"_meta": {"watane": u}}));
        }
        for v in items {
            if let Err(e) = self.host.push(&o, v) {
                self.drop_failed(&rec.id, &o, e);
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
            last: 0,
            checked: false,
            consent: None,
            head: None,
            sse: false,
            done: false,
            body: String::new(),
            drained: false,
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

    /// The item's work, on the session it names; `stream` is the provider stream it came on.
    /// Everything is written; the pushes and requests are the caller's.
    fn provider_work(
        &mut self,
        mut rec: Record,
        stream: Option<&str>,
        event: ProviderEvent,
        cause: Option<String>,
    ) -> Result<Work, KernelError> {
        let mut acct = self.accounts(&rec.owner);
        let past = acct.past;
        let before = rec.clone();
        let was = rec.turn.is_active();
        let mut ended = matches!(event, ProviderEvent::Ended)
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
            ProviderEvent::Consent(what) => {
                if let Turn::Running { since, .. } = rec.turn {
                    let activity = what.map(|what| crate::record::Activity::Consent { what });
                    work.pushes.push(Push::of(
                        &rec.id,
                        turn_push(&Turn::Running { since, activity }),
                    ));
                }
            }
            ProviderEvent::Turn(t) => self.change_turn(&mut rec, past, t, now, &mut work)?,
            ProviderEvent::Workspace(id) => match rec.workspace.as_deref() {
                None => rec.workspace = Some(id),
                Some(kept) if kept == id => {}
                Some(_) => {
                    let raw = json!({"type": "initialized", "workspace": id});
                    self.unknown_entry(&mut rec, past, raw, now, &mut work)?;
                }
            },
            ProviderEvent::ClaudeSession(uuid) => rec.claude_uuid = Some(uuid),
            ProviderEvent::Folder(cwd) => {
                if !rec.init_seen {
                    rec.init_seen = true;
                    if rec.folder.is_none() {
                        rec.folder = cwd.filter(|c| broker::plain_folder(c));
                    }
                }
            }
            ProviderEvent::Ended => {
                self.close_all_with(&mut rec, past, "ended", now, &mut work, false)?;
                if matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }) {
                    let failed = Turn::Idle {
                        stop: Stop::Failed {
                            error: "provider-ended".into(),
                            cause,
                        },
                        seconds: None,
                        tools: None,
                        tokens: None,
                    };
                    self.turn_to(&mut rec, past, failed, now, &mut work)?;
                }
            }
            ProviderEvent::Out(out) => {
                if let Some(on) = out.checking {
                    if let Turn::Running { since, .. } = rec.turn {
                        let activity =
                            Some(crate::record::Activity::Checking { on: judged_on(&on) });
                        work.pushes.push(Push::of(
                            &rec.id,
                            turn_push(&Turn::Running { since, activity }),
                        ));
                    }
                }
                if let Some(mut e) = out.entry {
                    let mut judged = None;
                    if e.kind == Kind::Verdict {
                        judged = self.verdict_entry(&rec, &mut e.body);
                    }
                    let verdict = e.kind == Kind::Verdict;
                    self.out_entry(&mut rec, past, e, now, &mut work)?;
                    if let (Some(terminal), true) = (judged, rec.turn.is_active()) {
                        let failed = Turn::Idle {
                            stop: Stop::Failed {
                                error: "judge".into(),
                                cause: None,
                            },
                            seconds: None,
                            tools: None,
                            tokens: None,
                        };
                        if terminal {
                            // The launch's denial closes the provider's session: a queued input
                            // stays queued for the next launch, never sent to it (S127 r2 F2.2).
                            self.turn_to(&mut rec, past, failed, now, &mut work)?;
                            // The stream is over now, as at its `ended` (which then finds no
                            // key): a `say` in between opens a new one (S127 r3 F3.1).
                            self.close_all_with(&mut rec, past, "ended", now, &mut work, false)?;
                            ended = stream;
                        } else {
                            // As any turn end: a queued input starts the next turn (r1 F1.3).
                            self.change_turn(&mut rec, past, failed, now, &mut work)?;
                        }
                    } else if let (true, Turn::Running { since, .. }) = (verdict, &rec.turn) {
                        // The `checking` activity ends with the verdict (9.3).
                        let t = Turn::Running {
                            since: *since,
                            activity: None,
                        };
                        work.pushes.push(Push::of(&rec.id, turn_push(&t)));
                    }
                }
                if let Some(u) = out.update {
                    let push = Push::of(&rec.id, wire_update(u, now, work.stored));
                    work.pushes.push(push);
                }
                if let Some(r) = out.resolved {
                    self.resolve(&mut rec, past, r, now, &mut work)?;
                }
                // A late `result` of an interrupted turn (its idle is still owed) says nothing
                // about the replacement turn's context (9.3, 9.7).
                if let (Some(m), 0) = (out.meter, rec.owed_idles) {
                    rec.usage = m["context-used"].is_u64().then(|| m.clone());
                    work.pushes.push(Push::of(&rec.id, m));
                }
                if out.idle && rec.owed_idles > 0 {
                    rec.owed_idles -= 1;
                } else if out.idle {
                    let idle = Turn::Idle {
                        stop: match out.limit {
                            Some(limit) => Stop::Stopped { limit },
                            None => Stop::Completed,
                        },
                        seconds: None,
                        tools: None,
                        tokens: None,
                    };
                    self.change_turn(&mut rec, past, idle, now, &mut work)?;
                }
            }
        }
        // 9.4: the stream's `ended` frees its slot and keys.
        if ended.is_some() && slots.is_none() {
            slots = Some(Slots::load(&mut self.state));
        }
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

    /// Maps a checked provider's verdict body for the entry (9.4): `on` of an input, `toolCallId`
    /// of the open ask or check it names. True when the verdict is an unsafe or unsure one on an
    /// input, which ends the turn (10).
    /// Fills a verdict entry's `on` and `toolCallId`; for a denied input, `Some(terminal)`:
    /// `true` for the launch, whose denial ends the provider's session (10 step 4).
    fn verdict_entry(&self, rec: &Record, body: &mut Value) -> Option<bool> {
        let on = body["on"].as_str().unwrap_or_default().to_string();
        let on_input = on == "launch" || on.starts_with("input/");
        let bad = body["outcome"] != "safe";
        if on_input {
            body["on"] = json!("input");
        } else if let Some(call) = rec
            .open
            .iter()
            .map(|o| (&o.id, &o.call))
            .chain(rec.pending_answers.iter().map(|(id, p)| (id, &p.call)))
            .find(|(id, _)| **id == on)
            .map(|(_, c)| c)
            .filter(|c| !c.is_empty())
        {
            body["toolCallId"] = json!(call);
        }
        (on_input && bad).then_some(on == "launch")
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

    /// 9.6: the (tool, scope key) of an ask, computed afresh from its own tool and input; never
    /// read from its `reach`.
    fn ask_scope(rec: &Record, body: &Value) -> Option<(String, String)> {
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("ask");
        if kind != "ask" || rec.owner.starts_with("agent:") {
            return None;
        }
        let tool = body.pointer("/toolCall/title").and_then(Value::as_str)?;
        let input = body.get("input").unwrap_or(&Value::Null);
        let path = body.get("path").and_then(Value::as_str);
        let key = broker::scope(tool, Self::allow_folder(rec, tool), input, path)?;
        Some((tool.to_string(), key))
    }

    /// A picked folder has no named source tree for a recursive file write grant.
    fn allow_folder<'a>(rec: &'a Record, tool: &str) -> Option<&'a str> {
        if rec.requested_folder.is_some()
            && matches!(tool, "Write" | "Edit" | "MultiEdit" | "NotebookEdit")
        {
            return None;
        }
        rec.folder.as_deref()
    }

    /// 9.6: the kernel sets the reach of a new ask, replacing or removing the one it came with.
    fn set_reach(&self, rec: &Record, body: &mut Value) {
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("ask");
        let tool = body.pointer("/toolCall/title").and_then(Value::as_str);
        let reach = tool.and_then(|t| {
            let input = body.get("input").unwrap_or(&Value::Null);
            let path = body.get("path").and_then(Value::as_str);
            broker::reach(&rec.owner, Self::allow_folder(rec, t), kind, t, input, path)
        });
        if let Some(o) = body.as_object_mut() {
            match reach {
                Some(r) => o.insert("reach".into(), Value::String(r)),
                None => o.remove("reach"),
            };
        }
    }

    /// An entry from the provider; an `ask` opens (or, with a decision, closes) an ask.
    pub(crate) fn out_entry(
        &mut self,
        rec: &mut Record,
        past: Past,
        mut e: adapters::Entry,
        now: u64,
        work: &mut Work,
    ) -> Result<(), KernelError> {
        let kind = e.kind;
        let is_ask = kind == Kind::Ask;
        if is_ask && !e.body.get("decision").is_some_and(|d| !d.is_null()) {
            self.set_reach(rec, &mut e.body);
        }
        let body = e.body.clone();
        let decided = is_ask && body.get("decision").is_some_and(|d| !d.is_null());
        if is_ask && !decided && self.reused(rec, &body) {
            return self.unknown_entry(rec, past, body, now, work);
        }
        let id = body.get("id").and_then(Value::as_str);
        // An ask decision is stored as ever; any other entry is not past the quota (9.3).
        let stored = if decided {
            // The decision is stored when the ask is open or pending, else not; past its quota
            // it is stored short.
            let at = rec
                .open
                .iter()
                .map(|o| (&o.id, &o.kind, o.i))
                .chain(rec.pending_answers.iter().map(|(id, p)| (id, &p.kind, p.i)))
                .find(|(oid, _, _)| Some(oid.as_str()) == id)
                .map(|(oid, k, i)| (oid.clone(), k.clone(), i));
            let Some((oid, k, ai)) = at else {
                // Not stored: past the quota an active turn ends as for any item not stored;
                // no deny, as the adapter's entry already carries its decision.
                if past.short(self.state.change().owner) {
                    return self.stop_at_quota(rec, past, now, work);
                }
                return Ok(());
            };
            if past.short(self.state.change().owner) {
                // Built from a fixed shape, so its size is bounded (9.3, 9.4).
                let Some(d) = short_decision(body.get("decision")) else {
                    return self.stop_at_quota(rec, past, now, work);
                };
                let short = NewEntry {
                    kind: "ask".into(),
                    uuid: None,
                    body: json!({"id": oid, "kind": k, "ask-i": ai, "decision": d}),
                };
                Some(self.append_live(rec, short, now, work)?)
            } else {
                Some(self.append_live(rec, NewEntry::from(e), now, work)?)
            }
        } else {
            self.append_loose(rec, past, NewEntry::from(e), now, work)?
        };
        if let (Some(i), Kind::Message | Kind::Tool) = (stored, kind) {
            work.stored = Some(i);
        }
        let Some(i) = stored else {
            // A new ask that is not stored is denied at once.
            if let (true, Some(id)) = (is_ask, id) {
                self.send_answer(rec, work, id, json!({"deny": true, "reason": "quota"}));
            }
            return Ok(());
        };
        let (true, Some(id)) = (is_ask, id) else {
            return Ok(());
        };
        if body.get("decision").is_some_and(|d| !d.is_null()) {
            // Closes the ask as an `ask-resolved` does: from `open` and from `pending-answers`.
            rec.pending_answers.remove(id);
            return self.close_ask(rec, past, id, now, work);
        }
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("ask");
        if rec.open.len() + rec.pending_answers.len() >= OPEN_ASKS {
            // Past the cap: denied at once, so a closing call writes at most 64 decisions.
            let d = json!({"outcome": "deny", "by": "kernel", "reason": "busy"});
            self.write_decision(rec, past, (id, kind), i, d, now, work)?;
            self.send_answer(rec, work, id, json!({"deny": true, "reason": "busy"}));
            return Ok(());
        }
        if kind == "ask" {
            if let Some((pending, decision)) = self.kernel_answer(rec, &body) {
                rec.pending_answers.insert(
                    id.to_string(),
                    PendingAnswer {
                        i,
                        kind: kind.to_string(),
                        call: body
                            .get("toolCallId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        outcome: pending.0.to_string(),
                        by: pending.1.to_string(),
                        for_: None,
                        reason: pending.2.map(str::to_string),
                        input: None,
                    },
                );
                self.send_answer(rec, work, id, decision);
                return Ok(());
            }
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

    /// 9.6: the kernel's own answer to a new Claude ask: a held session allow whose profile the
    /// owner may still use, else the "no owner stream" deny; none when the owner is to be asked.
    /// Returns `((outcome, by, reason), the request's decision)`.
    #[allow(clippy::type_complexity)]
    fn kernel_answer(
        &mut self,
        rec: &Record,
        body: &Value,
    ) -> Option<((&'static str, &'static str, Option<&'static str>), Value)> {
        let held = Self::ask_scope(rec, body)
            .is_some_and(|(t, k)| rec.allows.iter().any(|a| a.tool == t && a.key == k));
        if held && self.may_work(&rec.owner.clone(), rec) {
            return Some((("allow", "session-allow", None), json!({"allow": true})));
        }
        if rec.owner.starts_with("agent:") || rec.live.is_none() {
            return Some((
                ("deny", "kernel", Some("no-owner")),
                json!({"deny": true, "reason": "no-owner"}),
            ));
        }
        None
    }

    /// Called after an authenticated page call succeeds. Only that call can arm this timer.
    pub fn arm_page_origin(&mut self) {
        if self.state.get(PAGE_ORIGIN).is_none() {
            self.host.timer_after(PAGE_ORIGIN, 0);
        }
    }

    pub(crate) fn timer_carries_owner(&mut self, owner: &str) -> bool {
        let Some(origin) = &self.timer_origin else {
            return true;
        };
        if owner == "page" {
            return origin.as_ref().is_some_and(|origin| {
                self.state.get(PAGE_ORIGIN).as_deref() == Some(origin.as_bytes())
            });
        }
        origin.as_deref() == Some(owner)
    }

    /// `timer-sink.fired`: `restart` is R2, `s/<id>` a session's deadlines (9.3).
    pub fn timer(&mut self, id: &str) -> Called<()> {
        if id == PAGE_ORIGIN {
            // A page chain resolves to this installation. Even the host's fallback resolves
            // to this same installation, the owner of the reserved timer.
            if let Some(identity) = self.host.timer_firing_origin() {
                let _ = self.state.put(PAGE_ORIGIN, identity.into_bytes());
            }
            return Called {
                value: (),
                pushes: Vec::new(),
            };
        }
        let mut work = Work::default();
        if let Ok(mut slots) = self.begin() {
            if id == RESTART {
                // A firing that still fails (its page or `slots` cannot be written) re-arms nothing.
                let _ = self.restart_firing(&mut slots, &mut work);
            } else if let Some(session) = id.strip_prefix("s/") {
                // A firing is never an error: one that fails changes nothing more.
                self.timer_origin = Some(self.host.timer_firing_origin());
                let _ = self.session_firing(session, &mut slots, &mut work);
            }
        }
        let pushes = work.pushes;
        self.after_timer();
        self.timer_origin = None;
        Called { value: (), pushes }
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
        // The timer ID names the record whose immutable owner was bound by the authenticated
        // caller at session creation. The host's immediate caller for a firing is always host.
        let denied = rec.backend == "openai"
            && !rec.deadlines.is_empty()
            && !self.timer_carries_owner(&rec.owner);
        let mut acct = self.accounts(&rec.owner);
        let boot = rec.boot;
        if self.settle(&mut rec, &mut acct, slots, work).is_err() {
            self.lost_step(id, &mut acct);
            return Ok(());
        }
        let now = self.host.now();
        if !denied && !rec.deadlines.iter().any(|d| d.at <= now) {
            // Nothing due (an early firing, or a deadline over 30 days ahead, 7): the timer is set
            // again; a restart step `settle` ran is saved and charged (9.3 "Past"), else no key
            // and no count changes.
            if let Some(at) = next_due(&rec.deadlines, now) {
                self.set_timer(&format!("s/{id}"), at - now);
            }
            if rec.boot != boot {
                slots.save(&mut self.state)?;
                return self.finish_as(Some(&mut rec), &mut acct, true);
            }
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
        // 9.5: a `retry` or a `resume` that came due sends the request again.
        if denied {
            let was = rec.turn.is_active();
            self.openai_deny_timer(&mut rec, slots, &mut acct, work, now)?;
            self.sync_active(&mut rec, slots, was)?;
        } else if rec.backend == "openai" {
            let due = |what: &str| rec.deadlines.iter().any(|d| d.at <= now && d.what == what);
            let (retry, resume) = (due("retry"), due("resume"));
            if retry || resume {
                let was = rec.turn.is_active();
                rec.deadlines
                    .retain(|d| !(d.at <= now && (d.what == "retry" || d.what == "resume")));
                // A firing cannot fail (9.3): what the loop could not do stays undone.
                let _ = self.openai_fire(&mut rec, slots, &mut acct, work, now, (retry, resume));
                self.sync_active(&mut rec, slots, was)?;
            }
        }
        // 9.6: an `ask <id>` that came due denies the ask, reason `expired`.
        if !denied && rec.backend == "openai" {
            let ids: Vec<String> = rec
                .deadlines
                .iter()
                .filter(|d| d.at <= now)
                .filter_map(|d| d.what.strip_prefix("ask ").map(str::to_string))
                .collect();
            if !ids.is_empty() {
                let was = rec.turn.is_active();
                let _ = self.openai_expire(&mut rec, slots, &mut acct, work, now, &ids);
                self.sync_active(&mut rec, slots, was)?;
            }
        }
        // Due deadlines of later stages stay (and keep the count); the timer follows the rest.
        self.sync_timer(&rec, slots, &mut acct, &before, now, true);
        slots.save(&mut self.state)?;
        // A firing cannot fail (9.3 "Past"): what it wrote is charged, past the reserve too.
        self.finish_as(Some(&mut rec), &mut acct, true)?;
        // After the last write (7): the live owner stream gets the entry, or is dropped `lagging`.
        let own = std::mem::take(&mut work.pushes);
        self.deliver_stopped(work);
        self.deliver(&own, rec.live.as_deref());
        work.pushes.extend(own);
        self.deliver_peer(work);
        Ok(())
    }

    /// A firing whose restart step failed (9.3 R6): the session's timer is set again (for its next
    /// deadline, or, when only a due one is left, a retry after the idle delay), and what the step
    /// wrote is charged to the stored record and both accounts, as `restart_firing` does. The
    /// record stays of its older boot, so a later call does its step.
    fn lost_step(&mut self, id: &str, acct: &mut Acct) {
        let now = self.host.now();
        if let Some(mut held) = Record::load(&mut self.state, id) {
            let delay = next_due(&held.deadlines, now).map_or(IDLE_MS, |at| at - now);
            if !held.deadlines.is_empty()
                && (held.backend != "openai" || self.timer_carries_owner(&held.owner))
            {
                self.set_timer(&format!("s/{id}"), delay);
            }
            let lost = self.state.change().session - self.sealed;
            if lost != 0 {
                held.bytes = add_signed(held.bytes, lost);
                let _ = held.save(&mut self.state);
            }
        }
        let c = self.state.close();
        acct.owner.add(c.owner);
        acct.account.add(c.owner);
        let _ = self.save_accounts(acct);
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
                let stepped = self.step(&mut rec, &mut acct, slots, &mut done).is_ok();
                if stepped {
                    work.pushes.append(&mut done.pushes);
                    work.hand_on_stopped(&mut done);
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
        ProviderEvent::Consent(_)
        | ProviderEvent::Workspace(_)
        | ProviderEvent::ClaudeSession(_)
        | ProviderEvent::Folder(_)
        | ProviderEvent::Ended => false,
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

/// The activity's `on` of a provider's `checking` item: `input` for the launch and an input (10).
fn judged_on(on: &str) -> String {
    if on == "launch" || on.starts_with("input/") {
        "input".into()
    } else {
        on.into()
    }
}
