//! The OpenAI loop (TECHNICAL 9.5, S40-3): a session on a `backend: "openai"` profile sends
//! Responses requests through `https-stream`, shows the reply as the Claude backend's events are
//! shown, keeps the finished response in the wire history, and answers errors, usage limits,
//! `interrupt`, `cancel`, `delete` and a restart as 9.5 says. Tool calls go to `openai_calls`
//! (`tools.rs`, S40-4).
//!
//! The hooks live in `sessions.rs`; every state change here is made by a call that already holds
//! the session's record and accounts (`Cx`), or by a wrapper below that reads them.

use crate::active::Slots;
use crate::adapters::responses;
use crate::error::KernelError;
use crate::openai::answer::{self, backoff_ms, Failure, Sse};
use crate::openai::tools::{Call, Policy, SetupError, ToolSet, JOB};
use crate::openai::wire::{user_message, BodyArgs, Wire, WireItem, COMPACT_THRESHOLD};
use crate::overlay::apply_overlay;
use crate::profile::{parse_profile, SystemPrompt};
use crate::queue::Queue;
use crate::quota::Past;
use crate::record::{Activity, Deadline, Record, Stop, Turn};
use crate::sessions::{
    delta_only, input_text, wire_update, Acct, Called, Host, Kernel, Push, SinkOutcome, Step,
    StreamError, StreamKey, Work,
};
use crate::state::State;
use crate::transcript::Transcript;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The `kind` of a request stream's `stream/<id>` (9.1).
pub(crate) const REQUEST: &str = "request";
const RETRY: &str = "retry";
const RESUME: &str = "resume";
const URL: &str = "https://api.openai.com/v1/responses";
/// The model's context window in tokens, as the `usage` item reports it (9.7).
pub const CONTEXT_WINDOW: u64 = 272_000;
/// The most bytes of the transcript's last assistant messages a summary restart carries (9.5).
pub const SUMMARY_BYTES: usize = 32 * 1024;
/// The most bytes of a non-2xx body the loop keeps to classify it (one host chunk, 9.5).
pub const ERROR_BODY: usize = 64 * 1024;

/// What the loop is doing (9.5). `Idle` is between turns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
pub enum Phase {
    #[default]
    Idle,
    /// Waiting to send: `wait` is the place in the FIFO (`slots/wait/<n>`), `attempt` the retry
    /// this request is.
    Ready {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wait: Option<u64>,
        #[serde(default)]
        attempt: u32,
    },
    Requesting {
        stream: String,
        #[serde(default)]
        attempt: u32,
    },
    Retrying {
        attempt: u32,
    },
    Paused,
    /// A step's calls wait for the owner's answers (9.5, `waiting`).
    Waiting,
    /// A step's jobs run (9.5, `tools`); `wait` is the place in the FIFO while pending calls
    /// wait for a kernel or owner job slot.
    Tools {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wait: Option<u64>,
    },
}

/// `loop/<id>` (9.1, 9.5).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct LoopState {
    #[serde(flatten)]
    pub phase: Phase,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Compiled once at `start`: the profile's prompt, then the overlay's append.
    pub instructions: String,
    /// The error text of the last failed request, kept when retries ran out (9.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// The turn ended `failed {too-large}`: the next input restarts from a summary.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub too_large: bool,
    /// Tokens and tool calls of the turn so far, for its `idle`.
    #[serde(skip_serializing_if = "is_zero")]
    pub turn_tokens: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub turn_calls: u64,
    /// What the loop reads of the profile, compiled once at `start` (9.6).
    #[serde(skip_serializing_if = "Policy::is_empty")]
    pub policy: Policy,
    /// The session's handle on `agent-tools` and its described tools (9.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolSet>,
    /// The calls of the step that have no output yet.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<Call>,
    /// The step's calls run one after another (a tool that is not `parallel`).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub serial: bool,
    /// Job streams of the session that have not ended.
    #[serde(skip_serializing_if = "is_zero32")]
    pub jobs: u32,
    /// The `output_item.done` items of the response under way: its output when `completed` carries
    /// none (dev 18.3).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Value>,
    /// The toolset consent the first request waits for (live; kept for `attach`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent: Option<String>,
    #[serde(flatten)]
    pub wire: Wire,
}

fn is_zero32(n: &u32) -> bool {
    *n == 0
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// `slots/wait/<n>` (9.1).
#[derive(Serialize, Deserialize)]
struct Waiting {
    session: String,
    boot: u64,
}

/// The call in hand: the slots and accounts it read, what it pushes, and the clock.
pub(crate) struct Cx<'a> {
    pub slots: &'a mut Slots,
    pub a: &'a mut Acct,
    pub work: &'a mut Work,
    pub past: Past,
    pub now: u64,
    /// `slots` changed and is not saved yet.
    pub dirty: bool,
}

/// What a turn adds to the wire before its request: a summary that starts the history anew
/// (`failed {too-large}`), and the input items.
#[derive(Debug, Clone, Default)]
pub(crate) struct Pending {
    summary: Option<String>,
    inputs: Vec<Value>,
}

/// How `launch` found the request: sent (before any write), waiting, or not sendable.
pub(crate) enum Launch {
    Wait,
    /// No request slot is free for a request the turn makes later, or the call may be another
    /// owner's: nothing is sent and the turn stops `busy` (9.5 Slots).
    Busy,
    Started(String),
    TooLarge,
    Host(String),
    /// The new toolset handle needs the opening call to commit.
    OpeningToolset,
    /// The toolsets wait for the participant's consent (9.5): the request is tried again.
    Consent(String),
    /// The toolsets cannot be opened: the turn ends `failed`.
    Failed(String),
}

/// What `start`, `say` and `interrupt` work out before their first write.
pub(crate) struct Prelaunch {
    pub ls: LoopState,
    go: Option<(Pending, Launch)>,
}

/// One item of a request stream, as the loop reads it.
enum Ev {
    Consent(String),
    Head {
        status: u16,
        sse: bool,
        cleared: bool,
    },
    BadHead,
    Data {
        bytes: Vec<u8>,
        status: u16,
    },
}

fn request_json(body: &str) -> String {
    let body = serde_json::to_string(body).expect("a string serializes");
    format!(
        "{{\"url\":\"{URL}\",\"method\":\"POST\",\"headers\":{{\"content-type\":\"application/json\",\"accept\":\"text/event-stream\"}},\"body\":{body},\"account\":\"openai\",\"idle-s\":90}}"
    )
}

fn consent_what(v: &Value) -> Option<String> {
    if v.get("type").and_then(Value::as_str) != Some("consent-pending") {
        return None;
    }
    Some(
        v.get("what")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    )
}

/// The live `turn` push of a running turn with `activity` (9.3: live only).
fn running_push(rec: &Record, activity: Option<Activity>) -> Option<Push> {
    match rec.turn {
        Turn::Running { since, .. } => Some(Push::of(
            &rec.id,
            crate::sessions::turn_push(&Turn::Running { since, activity }),
        )),
        _ => None,
    }
}

fn failed(error: impl Into<String>) -> Stop {
    Stop::Failed {
        error: error.into(),
        cause: None,
    }
}

impl<S: State, H: Host> Kernel<S, H> {
    // ---- the loop state

    pub(crate) fn loop_load(&mut self, id: &str) -> Option<LoopState> {
        self.state
            .get(&format!("loop/{id}"))
            .and_then(|b| serde_json::from_slice(&b).ok())
    }

    pub(crate) fn loop_save(&mut self, id: &str, ls: &LoopState) -> Result<(), KernelError> {
        let bytes = serde_json::to_vec(ls).expect("a loop state serializes");
        self.state
            .put(&format!("loop/{id}"), bytes)
            .map_err(|t| KernelError::TooLarge(t.0 as u64))
    }

    /// The session's loop state, or a new one from its profile and overlay (9.5: `instructions`
    /// are compiled once, at `start`).
    pub(crate) fn loop_for(&mut self, rec: &Record) -> LoopState {
        if let Some(ls) = self.loop_load(&rec.id) {
            return ls;
        }
        let json = self.profile_json(&rec.profile).unwrap_or_default();
        let base = parse_profile(&json).0;
        let compiled = apply_overlay(&base, &rec.overlay.clone().unwrap_or_default()).0;
        LoopState {
            model: base.model.unwrap_or_default(),
            effort: base.effort,
            policy: Policy {
                toolsets: compiled.toolsets.clone(),
                allow: compiled.allow.clone(),
                deny: compiled.deny.clone(),
                mode: compiled.mode.clone(),
                ask_expiry: compiled.ask_expiry,
            },
            instructions: match compiled.system_prompt {
                Some(SystemPrompt::Replace(s) | SystemPrompt::Append(s)) => s,
                None => String::new(),
            },
            ..LoopState::default()
        }
    }

    // ---- the request

    fn pending_for(&mut self, rec: &Record, ls: &LoopState, input: &str) -> Pending {
        Pending {
            summary: ls.too_large.then(|| self.summary_text(rec)),
            inputs: vec![user_message(&input_text(input))],
        }
    }

    /// 9.5: the text of the transcript's last assistant messages, at most 32 KiB, newest kept.
    fn summary_text(&mut self, rec: &Record) -> String {
        let mut meta = rec.meta();
        let mut texts: Vec<String> = Vec::new();
        let mut bytes = 0usize;
        let mut before = Some(meta.last + 1);
        'pages: loop {
            let page = Transcript::new(&mut self.state, &rec.id, &mut meta).read(0, before, 256);
            let Ok(page) = page else { break };
            let Some(first) = page.entries.first() else {
                break;
            };
            let first_i = serde_json::from_str::<Value>(first)
                .ok()
                .and_then(|v| v["i"].as_u64())
                .unwrap_or(1);
            for line in page.entries.iter().rev() {
                let Ok(v) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if v["kind"] != "message" || v["body"]["role"] != "agent" {
                    continue;
                }
                let text: String = v["body"]["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c["text"].as_str())
                    .collect();
                if text.is_empty() {
                    continue;
                }
                bytes += text.len() + 2;
                texts.push(text);
                if bytes >= SUMMARY_BYTES {
                    break 'pages;
                }
            }
            if first_i <= 1 {
                break;
            }
            before = Some(first_i);
        }
        texts.reverse();
        let mut out = texts.join("\n\n");
        if out.len() > SUMMARY_BYTES {
            let mut cut = out.len() - SUMMARY_BYTES;
            while !out.is_char_boundary(cut) {
                cut += 1;
            }
            out = out[cut..].to_string();
        }
        out
    }

    /// The wire items the next request sends, with `pending` as it will be stored.
    fn preview(&mut self, rec: &Record, ls: &LoopState, pending: &Pending) -> Vec<WireItem> {
        let mut items = if pending.summary.is_some() {
            Vec::new()
        } else {
            ls.wire.active_items(&mut self.state, &rec.id)
        };
        let new = pending
            .summary
            .iter()
            .map(|s| user_message(s))
            .chain(pending.inputs.iter().cloned());
        for (i, item) in (ls.wire.next..).zip(new) {
            items.push(WireItem {
                i,
                step: ls.wire.steps,
                model: None,
                item,
            });
        }
        items
    }

    /// Builds the body and, when a request slot is free, starts the stream. Writes nothing (it
    /// sets only `ls.wire.trim` in memory), so a call can do it before its first write. In an
    /// items call (held) nothing is sent: the loop waits in the FIFO and the call's end sends it.
    fn launch(
        &mut self,
        rec: &Record,
        ls: &mut LoopState,
        slots: &Slots,
        a: &Acct,
        pending: &Pending,
        freeing: u32,
    ) -> Launch {
        if self.foreign {
            return Launch::Busy;
        }
        if self.held.is_some() {
            return Launch::Wait;
        }
        let room = slots.request.saturating_sub(freeing) < self.limits.request
            && a.owner.request_at(slots.boot).saturating_sub(freeing) < self.limits.owner_request;
        if !room {
            return Launch::Busy;
        }
        match self.tools_setup(ls) {
            Ok(()) => {}
            Err(SetupError::Opening) => return Launch::OpeningToolset,
            Err(SetupError::Consent(what)) => return Launch::Consent(what),
            Err(SetupError::Failed(e)) => return Launch::Failed(e),
        }
        if pending.summary.is_some() {
            ls.wire.trim = None;
        }
        let items = self.preview(rec, ls, pending);
        let reasoning = ls.effort.as_ref().map(|e| json!({ "effort": e }));
        let args = BodyArgs {
            session: &rec.id,
            model: &ls.model,
            instructions: &ls.instructions,
            tools: ls.tools.as_ref().map_or(&[], |t| &t.defs),
            reasoning: reasoning.as_ref(),
            compact_threshold: COMPACT_THRESHOLD,
        };
        let body = match ls.wire.body(&args, &items) {
            Ok(b) => b,
            Err(_) => return Launch::TooLarge,
        };
        match self.host.https_start(&request_json(&body)) {
            Ok(stream) => {
                self.opened = Some(stream.clone());
                Launch::Started(stream)
            }
            Err(e) => Launch::Host(e),
        }
    }

    /// What `start`, `say` and an `interrupt` replacement work out before their first write: the
    /// loop state, the input as wire items, and the request, sent at once. A host that refuses
    /// the request refuses the call (`backend`); no free request slot refuses it `busy`. `interrupting`: the request the
    /// session holds is about to be freed.
    pub(crate) fn openai_prelaunch(
        &mut self,
        rec: &Record,
        input: Option<&str>,
        slots: &Slots,
        a: &Acct,
        interrupting: bool,
    ) -> Result<Prelaunch, KernelError> {
        let mut ls = self.loop_for(rec);
        let Some(input) = input else {
            return Ok(Prelaunch { ls, go: None });
        };
        let pending = self.pending_for(rec, &ls, input);
        let freeing = u32::from(interrupting && matches!(ls.phase, Phase::Requesting { .. }));
        let launch = self.launch(rec, &mut ls, slots, a, &pending, freeing);
        match launch {
            Launch::Host(e) => return Err(KernelError::Backend(e)),
            // The deputy's r4 ruling (F4.2): a request of the owner's call that finds no slot
            // does not wait in the FIFO, whose drain may run in another origin's call; the
            // call is refused `busy` before its first write and the owner sends again.
            Launch::Wait | Launch::Busy => {
                return Err(KernelError::Busy(format!(
                    "no request slot is free ({} in all, {} per owner)",
                    self.limits.request, self.limits.owner_request
                )))
            }
            _ => {}
        }
        Ok(Prelaunch {
            ls,
            go: Some((pending, launch)),
        })
    }

    /// The end of a call that began a turn: stores the input in the wire and registers the
    /// request (or its place in the FIFO). Saves the loop state.
    pub(crate) fn openai_finish(
        &mut self,
        rec: &mut Record,
        p: Prelaunch,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        let Prelaunch { mut ls, go } = p;
        if let Some((pending, launch)) = go {
            ls.turn_tokens = 0;
            ls.turn_calls = 0;
            self.go(rec, &mut ls, cx, pending, Some(launch), 0)?;
        }
        self.loop_save(&rec.id, &ls)
    }

    /// Stores `pending` and takes the request's slot, or waits, or ends the turn (9.5 `ready`).
    pub(crate) fn go(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        pending: Pending,
        launched: Option<Launch>,
        attempt: u32,
    ) -> Result<(), KernelError> {
        let launch = match launched {
            Some(l) => l,
            None => self.launch(rec, ls, cx.slots, cx.a, &pending, 0),
        };
        if let Some(s) = &pending.summary {
            ls.wire.restart_from_summary(&mut self.state, &rec.id, s)?;
            ls.too_large = false;
        }
        if !pending.inputs.is_empty() {
            let step = ls.wire.steps;
            ls.wire
                .append(&mut self.state, &rec.id, step, None, pending.inputs)?;
        }
        if let Some(t) = &ls.tools {
            if rec.handle.as_deref() != Some(t.handle.as_str()) {
                rec.handle = Some(t.handle.clone());
            }
        }
        match launch {
            Launch::Started(stream) => {
                if ls.consent.take().is_some() {
                    let push = running_push(rec, None);
                    cx.work.pushes.extend(push);
                }
                self.register_request(rec, ls, cx, stream, attempt)
            }
            Launch::Wait => self.enqueue(rec, ls, cx, attempt),
            Launch::Busy => self.stop_busy(rec, ls, cx),
            Launch::TooLarge => {
                ls.too_large = true;
                self.end_turn(rec, ls, cx, failed("too-large"))
            }
            Launch::Host(e) | Launch::Failed(e) => {
                ls.last_error = Some(e.clone());
                self.end_turn(rec, ls, cx, failed(e))
            }
            Launch::OpeningToolset => self.retry(rec, ls, cx, attempt, "opening toolset".into()),
            Launch::Consent(what) => {
                ls.consent = Some(what.clone());
                self.retry(rec, ls, cx, attempt, format!("consent-pending({what})"))?;
                if matches!(ls.phase, Phase::Retrying { .. }) {
                    let push = running_push(rec, Some(Activity::Consent { what }));
                    cx.work.pushes.extend(push);
                }
                Ok(())
            }
        }
    }

    fn register_request(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        stream: String,
        attempt: u32,
    ) -> Result<(), KernelError> {
        let key = StreamKey::new(&rec.id, REQUEST);
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        self.state
            .put(&format!("stream/{stream}"), bytes)
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        rec.add_stream(stream.clone())?;
        cx.slots.request += 1;
        cx.a.owner.open_request(cx.slots.boot);
        cx.dirty = true;
        ls.phase = Phase::Requesting { stream, attempt };
        ls.items.clear();
        Ok(())
    }

    /// The request waits in the FIFO (9.5 Slots): `slots/wait/<n>` and `ready {wait}`.
    fn enqueue(
        &mut self,
        rec: &Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        attempt: u32,
    ) -> Result<(), KernelError> {
        let n = self.fifo_entry(rec, cx)?;
        self.queued.push(n);
        ls.phase = Phase::Ready {
            wait: Some(n),
            attempt,
        };
        Ok(())
    }

    /// Takes the place `slots.tail` in the FIFO for the session and asks the call's end to send
    /// the oldest waiting work.
    pub(crate) fn fifo_entry(&mut self, rec: &Record, cx: &mut Cx) -> Result<u64, KernelError> {
        let n = cx.slots.tail;
        let entry = Waiting {
            session: rec.id.clone(),
            boot: cx.slots.boot,
        };
        self.state
            .put(
                &format!("slots/wait/{n}"),
                serde_json::to_vec(&entry).expect("an entry serializes"),
            )
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        cx.slots.tail += 1;
        cx.dirty = true;
        self.drain_due = true;
        Ok(n)
    }

    /// Frees a request stream's slot and keys; `cancel` ends the stream at the host too.
    fn release_request(&mut self, rec: &mut Record, stream: &str, cx: &mut Cx, cancel: bool) {
        self.state.delete(&format!("stream/{stream}"));
        if cancel {
            self.cancel_provider(stream);
        }
        rec.streams.retain(|s| s != stream);
        if rec.boot == cx.slots.boot {
            cx.slots.request = cx.slots.request.saturating_sub(1);
            cx.a.owner.close_request(cx.slots.boot);
            cx.dirty = true;
        }
        self.drain_due = true;
    }

    // ---- the end of a turn, errors, pauses

    /// The turn ends `idle {stop}`; the oldest queued input then starts the next turn.
    fn end_turn(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        stop: Stop,
    ) -> Result<(), KernelError> {
        let tokens = (ls.turn_tokens > 0).then_some(ls.turn_tokens);
        let tools = (ls.turn_calls > 0).then_some(ls.turn_calls);
        ls.phase = Phase::Idle;
        ls.turn_tokens = 0;
        ls.turn_calls = 0;
        ls.consent = None;
        rec.deadlines
            .retain(|d| d.what != RETRY && d.what != RESUME);
        let idle = Turn::Idle {
            stop,
            seconds: None,
            tools,
            tokens,
        };
        self.turn_to(rec, cx.past, idle, cx.now, cx.work)?;
        if cx.past.is_past() || Self::room(rec).is_err() || !self.may_use(&rec.owner, &rec.profile)
        {
            return Ok(());
        }
        let mut q = Queue::load(&mut self.state, &rec.id);
        let Some(next) = q.take_oldest() else {
            return Ok(());
        };
        self.start_turn(
            rec,
            cx.past,
            next.input.clone(),
            false,
            None,
            cx.now,
            cx.work,
        )?;
        q.save(&mut self.state, &rec.id)?;
        cx.work.pushes.push(Push::of(&rec.id, q.push_value()));
        let pending = self.pending_for(rec, ls, &next.input);
        self.go(rec, ls, cx, pending, None, 0)
    }

    /// A request the turn makes later (after its jobs, at a retry, a resume or the next queued
    /// input) finds no request slot, or comes in a call that may be another owner's: it does not
    /// wait where another owner's call could send it (the deputy's r5 ruling). The turn stops
    /// `idle {stopped {limit: busy}}`, its wire, outputs and queue kept, as after a restart; the
    /// owner's next message continues it in her own call.
    fn stop_busy(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        let tokens = (ls.turn_tokens > 0).then_some(ls.turn_tokens);
        let tools = (ls.turn_calls > 0).then_some(ls.turn_calls);
        self.openai_stop(rec, ls, cx)?;
        if !rec.turn.is_active() {
            return Ok(());
        }
        let idle = Turn::Idle {
            stop: Stop::Stopped {
                limit: "busy".into(),
            },
            seconds: None,
            tools,
            tokens,
        };
        self.turn_to(rec, cx.past, idle, cx.now, cx.work)
    }

    /// Whether a session holding no timer yet finds none free (9.3).
    pub(crate) fn timer_capped(&self, rec: &Record, cx: &Cx) -> bool {
        rec.deadlines.is_empty()
            && (cx.slots.timers >= self.limits.timers
                || cx.a.owner.timers >= self.limits.owner_timers)
    }

    /// A usage limit (9.5): `paused {until}` with a `resume` deadline, or with no time when the
    /// reset is unknown or no timer is free.
    fn pause(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        until: Option<u64>,
    ) -> Result<(), KernelError> {
        rec.deadlines
            .retain(|d| d.what != RETRY && d.what != RESUME);
        let at = until
            .map(|u| u.max(cx.now + 1000))
            .filter(|_| !self.timer_capped(rec, cx))
            .filter(|&at| {
                rec.add_deadline(Deadline {
                    what: RESUME.into(),
                    at,
                })
                .is_ok()
            });
        ls.phase = Phase::Paused;
        self.turn_to(rec, cx.past, Turn::Paused { until: at }, cx.now, cx.work)
    }

    /// A retryable error (9.5): the next attempt after its backoff, or a pause with no time when
    /// the attempts ran out; no timer free ends the turn `failed {timers}` (9.3).
    fn retry(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        failed_attempt: u32,
        msg: String,
    ) -> Result<(), KernelError> {
        ls.last_error = Some(msg);
        rec.deadlines
            .retain(|d| d.what != RETRY && d.what != RESUME);
        let attempt = failed_attempt + 1;
        let Some(ms) = backoff_ms(attempt) else {
            ls.phase = Phase::Paused;
            return self.turn_to(rec, cx.past, Turn::Paused { until: None }, cx.now, cx.work);
        };
        let deadline = Deadline {
            what: RETRY.into(),
            at: cx.now + ms,
        };
        if self.timer_capped(rec, cx) || rec.add_deadline(deadline).is_err() {
            return self.end_turn(rec, ls, cx, failed("timers"));
        }
        ls.phase = Phase::Retrying { attempt };
        Ok(())
    }

    fn apply_failure(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        f: Failure,
    ) -> Result<(), KernelError> {
        let attempt = match &ls.phase {
            Phase::Requesting { attempt, .. } => *attempt,
            _ => 0,
        };
        match f {
            Failure::UsageLimit { until } => self.pause(rec, ls, cx, until),
            Failure::Retryable(msg) => self.retry(rec, ls, cx, attempt, msg),
            Failure::Fatal(msg) => {
                ls.last_error = Some(msg.clone());
                self.end_turn(rec, ls, cx, failed(msg))
            }
        }
    }

    /// A response completed (9.5 `requesting`): its output goes to the wire, the owner gets
    /// `usage`, and the turn ends or the calls go on.
    fn on_completed(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        usage: answer::Usage,
        output: Vec<Value>,
    ) -> Result<(), KernelError> {
        // dev 18.3: a stream may carry the items only as `output_item.done` events.
        let streamed = std::mem::take(&mut ls.items);
        let output = if output.is_empty() {
            streamed.clone()
        } else {
            output
        };
        ls.wire.steps += 1;
        let step = ls.wire.steps;
        let model = ls.model.clone();
        ls.wire
            .append(&mut self.state, &rec.id, step, Some(&model), output.clone())?;
        ls.turn_tokens += usage.input_tokens + usage.output_tokens;
        ls.last_error = None;
        cx.work.pushes.push(Push::of(
            &rec.id,
            json!({
                "type": "usage",
                "context-used": usage.input_tokens,
                "context-window": CONTEXT_WINDOW,
                "compact-at": COMPACT_THRESHOLD,
            }),
        ));
        // A compaction that did not stream is marked now (9.5 "Compaction items").
        for c in output
            .iter()
            .filter(|i| i["type"] == "compaction" && !streamed.contains(i))
        {
            let event = json!({"type": "response.output_item.done", "item": c});
            for out in responses::event(&event, cx.now) {
                if let Some(e) = out.entry {
                    self.out_entry(rec, cx.past, e, cx.now, cx.work)?;
                }
            }
        }
        if !rec.turn.is_active() {
            // The marker not stored past the quota ended the turn `stopped {quota}` (9.3): the
            // loop stops and the completion does not end the turn again.
            return self.openai_stop(rec, ls, cx);
        }
        let calls: Vec<Value> = output
            .into_iter()
            .filter(|i| i["type"] == "function_call")
            .collect();
        if calls.is_empty() {
            return self.end_turn(rec, ls, cx, Stop::Completed);
        }
        ls.turn_calls += calls.len() as u64;
        if streamed.is_empty() {
            // Not shown while they streamed: the calls appear as pending now.
            for c in &calls {
                let event = json!({"type": "response.output_item.done", "item": c});
                for out in responses::event(&event, cx.now) {
                    if let Some(u) = out.update {
                        let push = Push::of(&rec.id, wire_update(u, cx.now, None));
                        cx.work.pushes.push(push);
                    }
                }
            }
        }
        self.openai_calls(rec, ls, calls, cx)?;
        if !rec.turn.is_active() && matches!(ls.phase, Phase::Waiting | Phase::Tools { .. }) {
            // A call or an ask not stored past the quota ended the turn (9.3): the loop stops.
            return self.openai_stop(rec, ls, cx);
        }
        if matches!(ls.phase, Phase::Ready { .. }) {
            let attempt = 0;
            self.enqueue(rec, ls, cx, attempt)?;
        }
        Ok(())
    }

    // ---- items and the end of a request stream

    pub(crate) fn request_key(&mut self, stream: &str) -> Option<StreamKey> {
        self.loop_key(stream).filter(|k| k.kind == REQUEST)
    }

    /// The key of a stream the loop consumes: a model request or a tool job.
    pub(crate) fn loop_key(&mut self, stream: &str) -> Option<StreamKey> {
        self.state
            .get(&format!("stream/{stream}"))
            .and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok())
            .filter(|k| k.kind == REQUEST || k.kind == JOB)
    }

    /// `provider_lines` of a request stream (2.1 case 7, 9.5): the loop reads the head, each event
    /// and the consent notice.
    pub(crate) fn request_lines(
        &mut self,
        stream: &str,
        mut key: StreamKey,
        items: Vec<(u64, Vec<u8>)>,
    ) -> Result<Called<SinkOutcome>, KernelError> {
        let taken = || {
            Ok(Called {
                value: SinkOutcome::Taken,
                pushes: Vec::new(),
            })
        };
        let mut fresh: Vec<_> = items.into_iter().filter(|(n, _)| *n > key.last).collect();
        fresh.sort_by_key(|(n, _)| *n);
        let Some(last) = fresh.last().map(|(n, _)| *n) else {
            return taken();
        };
        self.foreign |= key.kind == JOB && key.drained;
        self.hold();
        let mut done = (Vec::new(), Vec::new(), Vec::new());
        let mut prev = key.last;
        for (seq, body) in fresh {
            if seq == prev {
                continue;
            }
            prev = seq;
            self.open_meter();
            let item = if key.kind == JOB {
                self.job_work(stream, &key, &body)
            } else {
                self.request_item(stream, &mut key, &body)
            };
            match item {
                Ok(Step::Done {
                    mut pushes,
                    mut requests,
                    gone,
                }) => {
                    done.0.append(&mut pushes);
                    done.1.append(&mut requests);
                    done.2.extend(gone);
                }
                other => {
                    let r = self.finish_items(other);
                    self.after_call();
                    return r;
                }
            }
            if self.state.get(&format!("stream/{stream}")).is_none() {
                break;
            }
        }
        let kept = self.take_request_seq(stream, key, last);
        let (pushes, requests, gone) = done;
        let r = self.finish_items(kept.map(|()| Step::Done {
            pushes,
            requests,
            gone,
        }));
        self.after_call();
        r
    }

    /// Keeps the stream's cursor, head and consent in `stream/<id>`, unless the batch ended it.
    fn take_request_seq(
        &mut self,
        stream: &str,
        mut key: StreamKey,
        last: u64,
    ) -> Result<(), KernelError> {
        let name = format!("stream/{stream}");
        if self.state.get(&name).is_none() {
            return Ok(());
        }
        self.open_meter();
        key.last = last;
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        self.state
            .put(&name, bytes)
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
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

    /// One item: a delta only forwards (reads `live/<session>` alone, P09); anything else is
    /// the loop's work on the session.
    fn request_item(
        &mut self,
        stream: &str,
        key: &mut StreamKey,
        body: &[u8],
    ) -> Result<Step, KernelError> {
        let session = key.session.clone();
        let parsed: Option<Value> = serde_json::from_slice(body).ok();
        let ev = if let Some(what) = parsed.as_ref().and_then(consent_what) {
            key.consent = Some(what.clone());
            Ev::Consent(what)
        } else if key.head.is_none() {
            match answer::head(body) {
                Some(h) => {
                    key.head = Some(h.status);
                    key.sse = h.sse;
                    Ev::Head {
                        status: h.status,
                        sse: h.sse,
                        cleared: key.consent.take().is_some(),
                    }
                }
                None => Ev::BadHead,
            }
        } else {
            let status = key.head.unwrap_or(0);
            if !(200..300).contains(&status) {
                // A non-2xx body is classified over its items joined (S40-2 `error_body`): once
                // they read as JSON, else at `ended`.
                if key.body.len() < ERROR_BODY {
                    let room = ERROR_BODY - key.body.len();
                    key.body
                        .push_str(&String::from_utf8_lossy(&body[..body.len().min(room)]));
                }
                if serde_json::from_str::<Value>(&key.body).is_err() {
                    return Ok(Step::Done {
                        pushes: Vec::new(),
                        requests: Vec::new(),
                        gone: Vec::new(),
                    });
                }
                Ev::Data {
                    bytes: key.body.clone().into_bytes(),
                    status,
                }
            } else {
                Ev::Data {
                    bytes: body.to_vec(),
                    status,
                }
            }
        };
        let now = self.host.now();
        // Items after `response.completed` are of a finished request (9.5 Items).
        if key.done {
            return Ok(Step::Done {
                pushes: Vec::new(),
                requests: Vec::new(),
                gone: Vec::new(),
            });
        }
        if let (Ev::Data { status, .. }, true) = (&ev, key.sse) {
            if (200..300).contains(status)
                && matches!(answer::sse(body, now), Sse::Completed { .. })
            {
                key.done = true;
            }
        }
        // A delta reads no record (P09).
        if let (Ev::Data { status, .. }, true) = (&ev, key.sse) {
            if (200..300).contains(status) {
                if let Sse::Event(v) = answer::sse(body, now) {
                    let outs = responses::event(&v, now);
                    let updates: Vec<Value> = outs.iter().filter_map(delta_only).collect();
                    if updates.len() == outs.len() {
                        let pushes: Vec<Push> = updates
                            .into_iter()
                            .map(|u| Push::of(&session, wire_update(u, now, None)))
                            .collect();
                        let live = self.live_stream(&session);
                        return Ok(self.tail(&session, pushes, live));
                    }
                }
            }
        }
        let work = self.openai_item(stream, &session, ev)?;
        let live = work.live.clone();
        Ok(self.tail(&session, work.pushes, live))
    }

    /// One item of a job stream: the call's output, then the same end as a request's item.
    fn job_work(
        &mut self,
        stream: &str,
        key: &StreamKey,
        body: &[u8],
    ) -> Result<Step, KernelError> {
        let work = self.job_item(stream, &key.session, body)?;
        let live = work.live.clone();
        Ok(self.tail(&key.session, work.pushes, live))
    }

    /// The end of an item's step, as `item_step` ends one: the pushes go to the live owner
    /// stream, and a full one defers the call.
    fn tail(&mut self, session: &str, pushes: Vec<Push>, live: Option<String>) -> Step {
        let failed = self.send_live(&pushes, live.as_deref());
        if failed == Some(StreamError::Full) {
            return Step::Defer;
        }
        let gone = live
            .filter(|_| failed.is_some())
            .map(|o| (session.to_string(), o))
            .into_iter()
            .collect();
        Step::Done {
            pushes,
            requests: Vec::new(),
            gone,
        }
    }

    /// The loop's work for one item of its current request stream.
    fn openai_item(&mut self, stream: &str, session: &str, ev: Ev) -> Result<Work, KernelError> {
        let current = |ls: &LoopState| matches!(&ls.phase, Phase::Requesting { stream: s, .. } if s == stream);
        self.with_loop(session, current, |this, rec, ls, cx| {
            this.on_event(rec, ls, cx, stream, ev)
        })
    }

    /// The frame of an item's work on a session's loop: the record, the loop state and the
    /// accounts are read, `act` runs when `gate` finds the loop in the state the item is for, and
    /// what changed is saved. Nothing is read past `gate` otherwise.
    pub(crate) fn with_loop(
        &mut self,
        session: &str,
        gate: impl FnOnce(&LoopState) -> bool,
        act: impl FnOnce(&mut Self, &mut Record, &mut LoopState, &mut Cx) -> Result<(), KernelError>,
    ) -> Result<Work, KernelError> {
        let mut rec = Record::load(&mut self.state, session)
            .ok_or_else(|| KernelError::UnknownSession(session.to_string()))?;
        let mut work = Work::default();
        let Some(mut ls) = self.loop_load(session) else {
            return Ok(work);
        };
        if !gate(&ls) {
            return Ok(work);
        }
        let mut acct = self.accounts(&rec.owner);
        let past = acct.past;
        let mut slots = Slots::load(&mut self.state);
        let (before, before_ls) = (rec.clone(), ls.clone());
        let was = rec.turn.is_active();
        let now = self.host.now();
        let mut cx = Cx {
            slots: &mut slots,
            a: &mut acct,
            work: &mut work,
            past,
            now,
            dirty: false,
        };
        act(self, &mut rec, &mut ls, &mut cx)?;
        let mut dirty = cx.dirty;
        self.settle_openai(
            &mut rec, &before, &mut slots, &mut acct, was, now, &mut dirty,
        )?;
        if dirty {
            slots.save(&mut self.state)?;
        }
        if ls != before_ls {
            self.loop_save(session, &ls)?;
        }
        let changed = rec != before;
        if changed {
            rec.updated = now;
        }
        self.finish_as(changed.then_some(&mut rec), &mut acct, true)?;
        work.live = rec.live.clone();
        Ok(work)
    }

    /// The timer and the active index follow what the step did to the record.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn settle_openai(
        &mut self,
        rec: &mut Record,
        before: &Record,
        slots: &mut Slots,
        a: &mut Acct,
        was: bool,
        now: u64,
        dirty: &mut bool,
    ) -> Result<(), KernelError> {
        if rec.deadlines != before.deadlines {
            *dirty |= self.sync_timer(rec, slots, a, &before.deadlines, now, false);
        }
        if was != rec.turn.is_active() {
            *dirty |= self.sync_active(rec, slots, was)?;
        }
        Ok(())
    }

    fn on_event(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        stream: &str,
        ev: Ev,
    ) -> Result<(), KernelError> {
        let running = running_push;
        match ev {
            Ev::Consent(what) => {
                let push = running(rec, Some(Activity::Consent { what }));
                cx.work.pushes.extend(push);
            }
            Ev::Head {
                status,
                sse,
                cleared,
            } => {
                if cleared {
                    let push = running(rec, None);
                    cx.work.pushes.extend(push);
                }
                if (200..300).contains(&status) && !sse {
                    let f = Failure::Fatal("the response is not an event stream".into());
                    self.release_request(rec, stream, cx, true);
                    self.apply_failure(rec, ls, cx, f)?;
                }
            }
            Ev::BadHead => {
                let f = Failure::Fatal("the stream's first item is not a response head".into());
                self.release_request(rec, stream, cx, true);
                self.apply_failure(rec, ls, cx, f)?;
            }
            Ev::Data { bytes, status } if !(200..300).contains(&status) => {
                let f = answer::error_body(status, &bytes, cx.now);
                self.release_request(rec, stream, cx, true);
                self.apply_failure(rec, ls, cx, f)?;
            }
            Ev::Data { bytes, .. } => {
                match answer::sse(&bytes, cx.now) {
                    Sse::Completed { usage, output } => {
                        self.on_completed(rec, ls, cx, usage, output)?;
                    }
                    Sse::Failed(f) => {
                        self.release_request(rec, stream, cx, true);
                        self.apply_failure(rec, ls, cx, f)?;
                    }
                    Sse::Unreadable(_) => {
                        let raw = Value::String(String::from_utf8_lossy(&bytes).into_owned());
                        self.unknown_entry(rec, cx.past, raw, cx.now, cx.work)?;
                    }
                    Sse::Event(_) | Sse::Item(_) => {
                        if let Sse::Item(item) = answer::sse(&bytes, cx.now) {
                            ls.items.push(item);
                        }
                        let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                        for out in responses::event(&v, cx.now) {
                            if let Some(e) = out.entry {
                                self.out_entry(rec, cx.past, e, cx.now, cx.work)?;
                            }
                            if let Some(u) = out.update {
                                let push =
                                    Push::of(&rec.id, wire_update(u, cx.now, cx.work.stored));
                                cx.work.pushes.push(push);
                            }
                        }
                        // An item not stored past the quota ended the turn: the loop stops (9.3).
                        if !rec.turn.is_active() && matches!(ls.phase, Phase::Requesting { .. }) {
                            self.openai_stop(rec, ls, cx)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// `stream-sink.ended` of a request stream (9.5).
    pub(crate) fn request_ended(
        &mut self,
        stream: &str,
        key: StreamKey,
        cause: Option<&str>,
    ) -> Result<Called<()>, KernelError> {
        self.foreign |= key.kind == JOB && key.drained;
        let r = self.openai_ended(stream, key, cause);
        self.after_call();
        r
    }

    fn openai_ended(
        &mut self,
        stream: &str,
        key: StreamKey,
        cause: Option<&str>,
    ) -> Result<Called<()>, KernelError> {
        let none = |this: &mut Self| {
            this.state.delete(&format!("stream/{stream}"));
            this.state.close();
            Ok(Called {
                value: (),
                pushes: Vec::new(),
            })
        };
        let Some(mut rec) = Record::load(&mut self.state, &key.session) else {
            return none(self);
        };
        let mut work = Work::default();
        let mut acct = self.accounts(&rec.owner);
        let past = acct.past;
        let mut slots = Slots::load(&mut self.state);
        let before = rec.clone();
        let was = rec.turn.is_active();
        let now = self.host.now();
        let mut ls = self.loop_load(&rec.id);
        let before_ls = ls.clone();
        let mut cx = Cx {
            slots: &mut slots,
            a: &mut acct,
            work: &mut work,
            past,
            now,
            dirty: false,
        };
        if key.kind == JOB {
            self.job_gone(&mut rec, ls.as_mut(), &mut cx, stream)?;
        } else {
            self.release_request(&mut rec, stream, &mut cx, false);
            if let Some(ls) = ls.as_mut() {
                let current =
                    matches!(&ls.phase, Phase::Requesting { stream: s, .. } if s == stream);
                if current {
                    let f = ended_failure(&key, cause, now);
                    self.apply_failure(&mut rec, ls, &mut cx, f)?;
                }
            }
        }
        let mut dirty = cx.dirty;
        self.settle_openai(
            &mut rec, &before, &mut slots, &mut acct, was, now, &mut dirty,
        )?;
        if dirty {
            slots.save(&mut self.state)?;
        }
        if let Some(ls) = ls.as_ref().filter(|l| Some(*l) != before_ls.as_ref()) {
            self.loop_save(&rec.id, ls)?;
        }
        let changed = rec != before;
        if changed {
            rec.updated = now;
        }
        self.finish_as(changed.then_some(&mut rec), &mut acct, true)?;
        let live = rec.live.clone();
        Ok(Called {
            value: (),
            pushes: self.flush(work, live.as_deref()),
        })
    }

    // ---- interrupt, cancel, delete

    /// Cancels every request stream of the session and frees their slots, aborts its calls and
    /// cancels its jobs (9.5 "any"): the loop is idle and no retry or resume is waiting.
    pub(crate) fn openai_stop(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        for s in rec.streams.clone() {
            if self.request_key(&s).is_some() {
                self.release_request(rec, &s, cx, true);
            }
        }
        if matches!(ls.phase, Phase::Tools { .. } | Phase::Waiting) || !ls.calls.is_empty() {
            self.abort_calls(rec, ls, cx)?;
        }
        ls.phase = Phase::Idle;
        ls.turn_tokens = 0;
        ls.turn_calls = 0;
        ls.consent = None;
        ls.items.clear();
        rec.deadlines
            .retain(|d| d.what != RETRY && d.what != RESUME);
        Ok(())
    }

    // ---- timers

    /// A `retry` or `resume` deadline came due (9.3 deadlines, 9.5).
    pub(crate) fn openai_fire(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        acct: &mut Acct,
        work: &mut Work,
        now: u64,
        due: (bool, bool),
    ) -> Result<(), KernelError> {
        let Some(mut ls) = self.loop_load(&rec.id) else {
            return Ok(());
        };
        let before_ls = ls.clone();
        let past = acct.past;
        let mut cx = Cx {
            slots,
            a: acct,
            work,
            past,
            now,
            dirty: false,
        };
        match (&ls.phase, due) {
            (Phase::Retrying { attempt }, (true, _)) => {
                let attempt = *attempt;
                self.go(rec, &mut ls, &mut cx, Pending::default(), None, attempt)?;
            }
            (Phase::Paused, (_, true)) if matches!(rec.turn, Turn::Paused { .. }) => {
                // 9.7 "Revoking": a profile the owner may no longer use sends nothing; the turn
                // stays paused with no time.
                if !self.may_use(&rec.owner, &rec.profile) {
                    let paused = Turn::Paused { until: None };
                    self.turn_to(rec, past, paused, now, cx.work)?;
                    return Ok(());
                }
                let running = Turn::Running {
                    since: now,
                    activity: None,
                };
                self.turn_to(rec, past, running, now, cx.work)?;
                self.go(rec, &mut ls, &mut cx, Pending::default(), None, 0)?;
            }
            _ => {}
        }
        if cx.dirty {
            slots.save(&mut self.state)?;
        }
        if ls != before_ls {
            self.loop_save(&rec.id, &ls)?;
        }
        Ok(())
    }

    /// `ask <id>` deadlines came due (9.6): each ask is denied, reason `expired`.
    pub(crate) fn openai_expire(
        &mut self,
        rec: &mut Record,
        slots: &mut Slots,
        acct: &mut Acct,
        work: &mut Work,
        now: u64,
        ids: &[String],
    ) -> Result<(), KernelError> {
        let Some(mut ls) = self.loop_load(&rec.id) else {
            return Ok(());
        };
        let before_ls = ls.clone();
        let past = acct.past;
        let mut cx = Cx {
            slots,
            a: acct,
            work,
            past,
            now,
            dirty: false,
        };
        self.expire_asks(rec, &mut ls, &mut cx, ids)?;
        if cx.dirty {
            slots.save(&mut self.state)?;
        }
        if ls != before_ls {
            self.loop_save(&rec.id, &ls)?;
        }
        Ok(())
    }

    // ---- restart

    /// R4 (9.5 "After a restart"; the deputy's cut, 2026-11-25, and its r4 ruling): a mid-turn
    /// turn, `running` (`requesting`, `ready`, `retrying` or `tools`) or `waiting` for a tool
    /// approval, stops and becomes `unknown`, not running, with the transcript up to its last
    /// finished message. Its open asks are denied `by: kernel`, reason `restart`; its open and
    /// pending calls get `aborted`; its `retry` and `ask` deadlines go. The step makes no model
    /// request, toolset operation or consent for it, under whatever origin runs the step. A
    /// `stopped {quota}` that recording the aborts gave stays. `paused` is unchanged. The owner's
    /// next `say` continues the session, in her own call.
    pub(crate) fn openai_restart(
        &mut self,
        rec: &mut Record,
        a: &mut Acct,
        slots: &mut Slots,
        work: &mut Work,
        now: u64,
    ) -> Result<(), KernelError> {
        if !matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }) {
            return Ok(());
        }
        let Some(mut ls) = self.loop_load(&rec.id) else {
            return Ok(());
        };
        let before = rec.deadlines.clone();
        let past = a.past;
        self.close_all_with(rec, past, "restart", now, work, false)?;
        let mut cx = Cx {
            slots,
            a,
            work,
            past,
            now,
            dirty: false,
        };
        self.openai_stop(rec, &mut ls, &mut cx)?;
        if rec.turn.is_active() {
            self.turn_to(rec, past, Turn::Unknown, now, cx.work)?;
        }
        let mut dirty = cx.dirty;
        self.loop_save(&rec.id, &ls)?;
        dirty |= self.sync_timer(rec, slots, a, &before, now, false);
        if dirty {
            slots.save(&mut self.state)?;
        }
        Ok(())
    }

    // ---- the FIFO

    /// Sends the oldest waiting requests whose session and owner are under their caps, while
    /// request slots are free (9.5 Slots). Each starts in a write of its own.
    pub(crate) fn after_call(&mut self) {
        if std::mem::take(&mut self.drain_due) {
            self.openai_drain();
        }
        self.queued.clear();
        self.foreign = false;
    }

    fn openai_drain(&mut self) {
        let mut slots = Slots::load(&mut self.state);
        // The requests this call queued go first: sent now, in the call that queued them, or
        // their turns stop `busy` (9.5 Slots); none is left for another call.
        for at in std::mem::take(&mut self.queued) {
            let name = format!("slots/wait/{at}");
            if let Some(bytes) = self.state.get(&name) {
                self.drain_one(&mut slots, &name, at, &bytes, true);
            }
        }
        let (mut n, tail) = (slots.head, slots.tail);
        let mut first_kept: Option<u64> = None;
        let mut changed = false;
        while n < tail && (slots.request < self.limits.request || slots.job < self.limits.job) {
            let at = n;
            n += 1;
            let name = format!("slots/wait/{at}");
            let Some(bytes) = self.state.get(&name) else {
                continue;
            };
            match self.drain_one(&mut slots, &name, at, &bytes, false) {
                Drain::Gone => changed = true,
                Drain::Kept => {
                    first_kept.get_or_insert(at);
                }
            }
        }
        // The rest of the FIFO is read only to find where it begins.
        while first_kept.is_none() && n < tail {
            if self.state.get(&format!("slots/wait/{n}")).is_some() {
                first_kept = Some(n);
            }
            n += 1;
        }
        let head = first_kept.unwrap_or(tail);
        if head != slots.head {
            slots.head = head;
            changed = true;
        }
        if changed {
            let _ = slots.save(&mut self.state);
        }
    }

    /// One FIFO entry: dropped when stale or sent now, or kept (its owner is at its cap). `mine`:
    /// a request this call queued. A request entry is sent only then, with a slot free; any other
    /// stops its turn `busy` (the call may be another owner's). Jobs are started with the call
    /// marked `foreign`, so the request after them is not sent in it.
    fn drain_one(
        &mut self,
        slots: &mut Slots,
        name: &str,
        at: u64,
        bytes: &[u8],
        mine: bool,
    ) -> Drain {
        let stale = |this: &mut Self| {
            this.state.delete(name);
            Drain::Gone
        };
        let Ok(w) = serde_json::from_slice::<Waiting>(bytes) else {
            return stale(self);
        };
        if w.boot != slots.boot {
            return stale(self);
        }
        let Some(mut rec) = Record::load(&mut self.state, &w.session) else {
            return stale(self);
        };
        let Some(mut ls) = self.loop_load(&w.session) else {
            return stale(self);
        };
        let (attempt, jobs) = match ls.phase {
            Phase::Ready {
                wait: Some(n),
                attempt,
            } if n == at => (attempt, false),
            Phase::Tools { wait: Some(n) } if n == at && !ls.calls.is_empty() => (0, true),
            _ => return stale(self),
        };
        // An entry whose kind of slot is all taken waits (9.5 Slots): another kind's still goes.
        let full = if jobs {
            slots.job >= self.limits.job
        } else {
            slots.request >= self.limits.request
        };
        if full && jobs {
            return Drain::Kept;
        }
        self.open_meter();
        let mut acct = self.accounts(&rec.owner);
        let at_cap = if jobs {
            acct.owner.job_at(slots.boot) >= self.limits.owner_job
        } else {
            acct.owner.request_at(slots.boot) >= self.limits.owner_request
        };
        if at_cap && jobs {
            self.state.close();
            return Drain::Kept;
        }
        // A request goes only in the call that queued it, with a slot free (launch checks it
        // again); otherwise its turn stops `busy`.
        let was_foreign = self.foreign;
        self.foreign = jobs || !mine;
        if !jobs {
            self.state.delete(name);
        }
        let mut work = Work::default();
        let (past, now) = (acct.past, self.host.now());
        let before = rec.clone();
        let was = rec.turn.is_active();
        let mut cx = Cx {
            slots,
            a: &mut acct,
            work: &mut work,
            past,
            now,
            dirty: false,
        };
        let sent = if jobs {
            self.settle_calls(&mut rec, &mut ls, &mut cx, false)
        } else {
            self.go(
                &mut rec,
                &mut ls,
                &mut cx,
                Pending::default(),
                None,
                attempt,
            )
        };
        self.foreign = was_foreign;
        let mut dirty = cx.dirty;
        // A session still waiting for job slots keeps its place; the rest is done with.
        let kept = jobs && ls.phase == (Phase::Tools { wait: Some(at) });
        if jobs && !kept {
            self.state.delete(name);
        }
        if sent.is_ok() {
            let _ = self.settle_openai(&mut rec, &before, slots, &mut acct, was, now, &mut dirty);
            let _ = self.loop_save(&rec.id, &ls);
        }
        let _ = slots.save(&mut self.state);
        if rec != before {
            rec.updated = now;
        }
        let _ = self.finish_as(Some(&mut rec), &mut acct, true);
        self.deliver(&work.pushes, rec.live.as_deref());
        if kept {
            Drain::Kept
        } else {
            Drain::Gone
        }
    }

    // ---- attach

    /// The activity a session's request shows an attaching page: its consent notice, if any.
    pub(crate) fn openai_activity(&mut self, rec: &Record) -> Option<Activity> {
        let ls = self.loop_load(&rec.id)?;
        if let (Phase::Retrying { .. }, Some(what)) = (&ls.phase, &ls.consent) {
            return Some(Activity::Consent { what: what.clone() });
        }
        let Phase::Requesting { stream, .. } = ls.phase else {
            return None;
        };
        let key = self.request_key(&stream)?;
        key.consent.map(|what| Activity::Consent { what })
    }

    /// `delete`: the loop state and the wire chunks go, uncharged (the record's `bytes` frees
    /// them); the request streams the session holds are the caller's.
    pub(crate) fn openai_delete(&mut self, id: &str) {
        if let Some(ls) = self.loop_load(id) {
            self.close_tools(&ls);
            for first in &ls.wire.wire_chunks {
                self.state.delete_uncharged(&format!("wire/{id}/{first}"));
            }
        }
        self.state.delete_uncharged(&format!("loop/{id}"));
    }
}

enum Drain {
    Gone,
    Kept,
}

/// How a request stream's end reads (9.5 Errors): a refused stream has its head's status; one
/// refused by the consent says so.
fn ended_failure(key: &StreamKey, cause: Option<&str>, now: u64) -> Failure {
    if let Some(c) = cause.filter(|c| c.starts_with("consent-refused(")) {
        return Failure::Fatal(c.to_string());
    }
    match key.head {
        Some(status) if !(200..300).contains(&status) => {
            answer::error_body(status, key.body.as_bytes(), now)
        }
        _ => answer::ended(cause, false)
            .unwrap_or_else(|| Failure::Fatal("the response ended before it completed".into())),
    }
}
