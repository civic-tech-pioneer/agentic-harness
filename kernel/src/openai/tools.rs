//! The OpenAI loop's tool calls (TECHNICAL 9.5 `waiting` and `tools`, 9.6, 6; S40-4).
//!
//! A completed response's calls are decided by the broker (the profile's deny, ask and allow
//! rules), the allowed ones run as jobs of `agent-tools` within the job slots, and every call gets
//! exactly one output before the next request. The state lives in `loop/<id>`: `calls` holds the
//! calls of the step that have no output yet.

use crate::adapters::responses;
use crate::error::KernelError;
use crate::openai::run::{Cx, LoopState, Pending, Phase};
use crate::openai::skill::{self, SKILL};
use crate::openai::task::TASK;
use crate::profile::{PreToolCheck, Skill};
use crate::record::{Deadline, OpenAsk, Record, Stop, Turn, DEADLINES, OPEN_ASKS};
use crate::rules::{glob, Rules, Verdict};
use crate::sessions::{wire_update, Host, Kernel, Push, StreamKey, Tool};
use crate::state::State;
use crate::transcript::NewEntry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The `kind` of a job's `stream/<id>` (9.1).
pub(crate) const JOB: &str = "job";
/// The ask expiry of a profile that sets none, in seconds (9.6).
pub const ASK_EXPIRY_S: u64 = 600;

/// What the loop reads of the profile, compiled once at `start` with the instructions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Policy {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub toolsets: Vec<String>,
    /// Effective skill files pinned at admission. Older OpenAI loops had no skills.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Skill>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_expiry: Option<u64>,
    /// The turn's limits, compiled from the profile and overlay (9.7 "Limits").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_errors: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// What admission of a child compares (9.7): the compiled profile's delegation and the
    /// fields a child must match or narrow, kept so a later edit of the stored profile cannot
    /// widen what this session's children may be.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub delegates: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_limits: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_tool_check: Option<PreToolCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persist: Option<bool>,
}

impl Policy {
    pub fn is_empty(&self) -> bool {
        *self == Policy::default()
    }
}

/// What the loop keeps of one described tool: its name and the two annotations (6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub parallel: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub idempotent: bool,
}

/// The session's handle on `agent-tools` and the tools it described: `defs` is the request's
/// `tools`, kept so that every request of the session sends the same bytes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolSet {
    pub handle: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub described: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defs: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub info: Vec<ToolInfo>,
}

impl ToolSet {
    fn info(&self, name: &str) -> Option<&ToolInfo> {
        self.info.iter().find(|i| i.name == name)
    }
}

/// Where a call of the step stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "st", rename_all = "kebab-case")]
pub enum CallState {
    /// An open ask names the call (the ask's id is the call's `call_id`).
    Ask,
    /// Allowed, not started: waits for a job slot (9.5 `pending`).
    Pending,
    /// Running as a job on this stream.
    Job { stream: String },
    /// A `task` call whose delegated first turn runs in this child session (9.7): the call holds
    /// no stream and no slot.
    Child { session: String },
}

/// A `function_call` item of the step with no output yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub item: Value,
    #[serde(flatten)]
    pub st: CallState,
}

impl Call {
    pub(crate) fn id(&self) -> &str {
        self.item["call_id"].as_str().unwrap_or_default()
    }
}

/// Why a step's pending calls did not all start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blocked {
    No,
    /// The session holds its job slots, or all its streams.
    Session,
    /// The kernel or the owner holds its job slots: the session waits in the FIFO.
    Shared,
}

fn consent_pending(e: &str) -> Option<String> {
    e.strip_prefix("consent-pending(")
        .and_then(|r| r.strip_suffix(')'))
        .map(str::to_string)
}

/// Why a request could not use its toolset yet.
pub(crate) enum SetupError {
    Opening,
    Consent(String),
    Failed(String),
}

/// The rule names of the profile may hold `*` and `?`: each is replaced by the described tools
/// whose names it matches (the rest of the rule is kept).
fn expand(rules: &[String], names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for r in rules {
        let (head, rest) = match r.find('(') {
            Some(i) => r.split_at(i),
            None => (r.as_str(), ""),
        };
        if !head.contains(['*', '?']) {
            out.push(r.clone());
            continue;
        }
        let pat: Vec<char> = head.chars().collect();
        for n in names {
            let text: Vec<char> = n.chars().collect();
            if glob(&pat, &text) {
                out.push(format!("{n}{rest}"));
            }
        }
    }
    out
}

fn errors_reached(ls: &LoopState) -> bool {
    ls.policy
        .max_tool_errors
        .is_some_and(|m| ls.tool_errors_in_a_row >= m)
}

fn error_text(why: &str) -> String {
    json!({ "error": why }).to_string()
}

/// A job's item as the call's output: `{"ok":v}` is `v` as JSON text, `{"err":{code,message}}` is
/// `{"error":"<code>: <message>"}`.
fn job_result(body: &[u8]) -> (String, bool) {
    let v: Option<Value> = serde_json::from_slice(body).ok();
    match v.as_ref().and_then(|v| v.as_object()) {
        Some(o) if o.contains_key("ok") => (o["ok"].to_string(), true),
        Some(o) if o.contains_key("err") => {
            let e = &o["err"];
            let part = |k: &str| e.get(k).and_then(Value::as_str).unwrap_or_default();
            (
                error_text(&format!("{}: {}", part("code"), part("message"))),
                false,
            )
        }
        _ => (error_text("the job's result is unreadable"), false),
    }
}

impl<S: State, H: Host> Kernel<S, H> {
    // ---- opening the toolsets

    /// The session's `agent-tools` handle and the tools it describes (AC1): opened and described
    /// at the first request, kept in the loop state, never again. A profile with no toolsets
    /// opens nothing.
    /// The described names must be distinct from active built-ins (9.7).
    pub(crate) fn tools_setup(
        &mut self,
        ls: &mut LoopState,
        task_active: bool,
    ) -> Result<(), SetupError> {
        if ls.policy.toolsets.is_empty() {
            return Ok(());
        }
        let fail = |e: String| match consent_pending(&e) {
            Some(what) => SetupError::Consent(what),
            None => SetupError::Failed(e),
        };
        let opening = ls.tools.is_none();
        if opening {
            let handle = self.host.tools_open(&ls.policy.toolsets).map_err(fail)?;
            ls.tools = Some(ToolSet {
                handle,
                ..ToolSet::default()
            });
        }
        let Some(set) = ls.tools.as_mut().filter(|t| !t.described) else {
            return self.check_collision(ls, task_active);
        };
        let tools = self.host.tools_describe(&set.handle).map_err(|e| {
            // Watane publishes the open only after this call commits.
            if opening && e == "unknown-handle" {
                SetupError::Opening
            } else {
                fail(e)
            }
        })?;
        set.described = true;
        set.defs = tools.iter().map(definition).collect();
        set.info = tools
            .into_iter()
            .map(|t| ToolInfo {
                name: t.name,
                parallel: t.parallel,
                idempotent: t.idempotent,
            })
            .collect();
        self.check_collision(ls, task_active)
    }

    /// Check both fresh and cached descriptions before sending a request (9.7).
    fn check_collision(&mut self, ls: &mut LoopState, task_active: bool) -> Result<(), SetupError> {
        let Some(set) = &ls.tools else {
            return Ok(());
        };
        let Some(name) = [
            task_active.then_some(TASK),
            (!ls.policy.skills.is_empty()).then_some(SKILL),
        ]
        .into_iter()
        .flatten()
        .find(|name| set.info(name).is_some()) else {
            return Ok(());
        };
        self.host.tools_close(&set.handle);
        ls.tools = None;
        Err(SetupError::Failed(format!(
            "toolsets: the function {name} collides with a built-in of the kernel"
        )))
    }

    /// Ends the session's handle (`cancel`, `delete`).
    pub(crate) fn close_tools(&mut self, ls: &LoopState) {
        if let Some(t) = &ls.tools {
            self.host.tools_close(&t.handle);
        }
    }

    // ---- the calls of a response

    /// The calls of a completed response (9.5 `requesting`): the broker decides each one, the
    /// allowed ones wait for the asks and then run. Leaves the loop `waiting`, `tools`, or `ready`
    /// (every call has its output); the caller sends the next request in that case.
    pub(crate) fn openai_calls(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        calls: Vec<Value>,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        ls.calls.clear();
        let names: Vec<String> = ls
            .tools
            .iter()
            .flat_map(|t| t.info.iter().map(|i| i.name.clone()))
            .chain(delegating(rec, ls).then(|| TASK.to_string()))
            .chain((!ls.policy.skills.is_empty()).then(|| SKILL.to_string()))
            .collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let (rules, _) = Rules::compile(
            &expand(&ls.policy.allow, &names),
            &[],
            &expand(&ls.policy.deny, &names),
        );
        for item in calls {
            if errors_reached(ls) {
                // The cap was reached by an earlier call of this step: the rest is cleanup.
                self.put_aborted(rec, ls, cx, &item)?;
                continue;
            }
            let name = item["name"].as_str().unwrap_or_default().to_string();
            // A child's `task` call is refused at dispatch without asking anyone (9.7).
            if name == TASK && rec.parent.is_some() {
                ls.calls.push(Call {
                    item,
                    st: CallState::Pending,
                });
                continue;
            }
            if !builtin_task(rec, ls, &name)
                && !builtin_skill(ls, &name)
                && ls.tools.as_ref().and_then(|t| t.info(&name)).is_none()
            {
                // A session with no toolsets has nothing to run: its call is `aborted` (S40-3).
                let out = if name == SKILL {
                    error_text("the session's profile has no skills")
                } else if ls.tools.is_some() {
                    error_text("unknown tool")
                } else {
                    "aborted".to_string()
                };
                self.put_output(rec, ls, cx, &item, out, false)?;
                continue;
            }
            let profile = || error_text("denied by the profile");
            match rules.decide(&name, None) {
                Verdict::Deny => self.put_output(rec, ls, cx, &item, profile(), false)?,
                Verdict::Ask if ls.policy.mode.as_deref() == Some("dont-ask") => {
                    self.put_output(rec, ls, cx, &item, profile(), false)?;
                }
                Verdict::Allow => ls.calls.push(Call {
                    item,
                    st: CallState::Pending,
                }),
                Verdict::Ask => match self.open_ask(rec, cx, &item)? {
                    None => ls.calls.push(Call {
                        item,
                        st: CallState::Ask,
                    }),
                    Some(why) => {
                        let out = error_text(&format!("denied: {why}"));
                        self.put_output(rec, ls, cx, &item, out, false)?;
                    }
                },
            }
        }
        if rec.turn.is_active() && self.stop_on_errors(rec, ls, cx)? {
            return Ok(());
        }
        let info = ls.tools.clone().unwrap_or_default();
        ls.serial = !ls.calls.iter().all(|c| {
            let name = c.item["name"].as_str().unwrap_or_default();
            builtin_task(rec, ls, name)
                || builtin_skill(ls, name)
                || info.info(name).is_some_and(|i| i.parallel)
        });
        ls.phase = if ls.calls.iter().any(|c| c.st == CallState::Ask) {
            Phase::Waiting
        } else {
            Phase::Tools { wait: None }
        };
        if !rec.turn.is_active() {
            return Ok(());
        }
        self.settle_calls(rec, ls, cx, true)
    }

    /// Stores a call's one output: in the wire, and shown to the owner as the Responses adapter
    /// maps a job output (the tool call entry completes or fails).
    pub(crate) fn put_output(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        call: &Value,
        output: String,
        ok: bool,
    ) -> Result<(), KernelError> {
        self.count_output(ls, ok);
        self.store_output(rec, ls, cx, call, output, ok)
    }

    /// Counts one output of a call for `max-tool-errors` (9.7 "Limits"): a success resets the
    /// streak, any other output adds to it. True when the streak has reached the cap. Cleanup
    /// aborts (`put_aborted`) never come here.
    pub(crate) fn count_output(&mut self, ls: &mut LoopState, ok: bool) -> bool {
        if ok {
            ls.tool_errors_in_a_row = 0;
        } else {
            ls.tool_errors_in_a_row = ls.tool_errors_in_a_row.saturating_add(1);
        }
        errors_reached(ls)
    }

    /// The `aborted` output of an interrupt cleanup or a limit stop (9.5, 9.7): stored like any
    /// output but not counted toward `max-tool-errors`.
    pub(crate) fn put_aborted(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        call: &Value,
    ) -> Result<(), KernelError> {
        self.store_output(rec, ls, cx, call, "aborted".into(), false)
    }

    fn store_output(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        call: &Value,
        output: String,
        ok: bool,
    ) -> Result<(), KernelError> {
        let id = call["call_id"].as_str().unwrap_or_default();
        let step = ls.wire.steps;
        let item = json!({"type": "function_call_output", "call_id": id, "output": output});
        ls.wire
            .append(&mut self.state, &rec.id, step, None, vec![item])?;
        for out in responses::job_output(call, &output, ok, cx.now) {
            if let Some(e) = out.entry {
                self.out_entry(rec, cx.past, e, cx.now, cx.work)?;
            }
            if let Some(u) = out.update {
                let push = Push::of(&rec.id, wire_update(u, cx.now, cx.work.stored));
                cx.work.pushes.push(push);
            }
        }
        Ok(())
    }

    // ---- asks (9.6)

    /// A call that asks the owner: the ask entry, the `open` record, the `ask <id>` deadline and
    /// the `waiting` turn. `Some(reason)` when the kernel denies it at once instead (its ask and
    /// decision entries are written; `quota` stores nothing).
    fn open_ask(
        &mut self,
        rec: &mut Record,
        cx: &mut Cx,
        item: &Value,
    ) -> Result<Option<&'static str>, KernelError> {
        let id = item["call_id"].as_str().unwrap_or_default().to_string();
        let name = item["name"].as_str().unwrap_or_default();
        let args = item["arguments"].as_str().unwrap_or_default();
        let input: Value =
            serde_json::from_str(args).unwrap_or_else(|_| Value::String(args.to_string()));
        let expiry = self.ask_expiry_s(rec);
        let expires_at = cx.now.saturating_add(expiry.saturating_mul(1000));
        let body = json!({
            "id": id,
            "kind": "ask",
            "toolCallId": id,
            "toolCall": {"toolCallId": id, "title": name, "rawInput": input},
            "input": input,
            "expires-at": expires_at,
        });
        let entry = NewEntry {
            kind: "ask".into(),
            uuid: None,
            body,
        };
        let Some(i) = self.append_loose(rec, cx.past, entry, cx.now, cx.work)? else {
            return Ok(Some("quota"));
        };
        let why = if rec.owner.starts_with("agent:") || rec.live.is_none() {
            Some("no-owner")
        } else if rec.open.len() + rec.pending_answers.len() >= OPEN_ASKS {
            Some("busy")
        } else if expiry == 0 {
            Some("expired")
        } else if self.timer_capped(rec, cx) || rec.deadlines.len() >= DEADLINES {
            Some("timers")
        } else {
            None
        };
        if let Some(why) = why {
            let d = json!({"outcome": "deny", "by": "kernel", "reason": why});
            self.write_decision(rec, cx.past, (&id, "ask"), i, d, cx.now, cx.work)?;
            return Ok(Some(why));
        }
        rec.open.push(OpenAsk {
            id: id.clone(),
            kind: "ask".into(),
            i,
            call: id.clone(),
            expires_at: Some(expires_at),
        });
        rec.add_deadline(Deadline {
            what: format!("ask {id}"),
            at: expires_at,
        })?;
        let (mut asks, theirs) = match &rec.turn {
            Turn::Waiting { asks, expires_at } => (asks.clone(), *expires_at),
            _ => (Vec::new(), None),
        };
        asks.push(id);
        let earliest = [Some(expires_at), theirs].into_iter().flatten().min();
        let waiting = Turn::Waiting {
            asks,
            expires_at: earliest,
        };
        self.turn_to(rec, cx.past, waiting, cx.now, cx.work)?;
        Ok(None)
    }

    fn ask_expiry_s(&mut self, rec: &Record) -> u64 {
        self.loop_load(&rec.id)
            .and_then(|ls| ls.policy.ask_expiry)
            .unwrap_or(ASK_EXPIRY_S)
    }

    /// The owner's answer, or a deadline, decides the ask on `id` (AC3): its one decision entry,
    /// the turn follows, and the call is allowed to run or gets the denial as its output. When
    /// the step has no ask left the allowed jobs start.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn decide_ask(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        id: &str,
        allow: bool,
        by: &str,
        reason: Option<&str>,
    ) -> Result<(), KernelError> {
        let Some(open) = rec.open.iter().find(|o| o.id == id).cloned() else {
            return Ok(());
        };
        let mut d = json!({"outcome": if allow { "allow" } else { "deny" }, "by": by});
        if let Some(r) = reason {
            d["reason"] = Value::String(r.to_string());
        }
        self.write_decision(rec, cx.past, (id, "ask"), open.i, d, cx.now, cx.work)?;
        rec.deadlines.retain(|d| d.what != format!("ask {id}"));
        self.close_ask(rec, cx.past, id, cx.now, cx.work)?;
        let Some(at) = ls
            .calls
            .iter()
            .position(|c| c.st == CallState::Ask && c.id() == id)
        else {
            return Ok(());
        };
        if allow {
            ls.calls[at].st = CallState::Pending;
        } else {
            let call = ls.calls.remove(at);
            let out = match reason {
                Some(r) => error_text(&format!("denied: {r}")),
                None => error_text("denied by the owner"),
            };
            self.put_output(rec, ls, cx, &call.item, out, false)?;
        }
        Ok(())
    }

    /// After asks were decided: the loop leaves `waiting` once none is open.
    pub(crate) fn after_asks(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        if !rec.turn.is_active() || self.stop_on_errors(rec, ls, cx)? {
            return Ok(());
        }
        if ls.phase == Phase::Waiting && !ls.calls.iter().any(|c| c.st == CallState::Ask) {
            ls.phase = Phase::Tools { wait: None };
        }
        self.settle_calls(rec, ls, cx, false)
    }

    /// A due `ask <id>` deadline: the ask is denied, reason `expired` (9.6).
    pub(crate) fn expire_asks(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        ids: &[String],
    ) -> Result<(), KernelError> {
        for id in ids {
            self.decide_ask(rec, ls, cx, id, false, "kernel", Some("expired"))?;
            // A deadline whose ask is already gone must not stay due.
            rec.deadlines.retain(|d| d.what != format!("ask {id}"));
        }
        self.after_asks(rec, ls, cx)
    }

    // ---- jobs (6, 9.5 Slots)

    /// With the loop in `tools`: starts the allowed calls as far as the job slots allow, in call
    /// order, and sends the next request when every call has its output. `initial`: the caller is
    /// `on_completed`, which queues the request itself.
    pub(crate) fn settle_calls(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        initial: bool,
    ) -> Result<(), KernelError> {
        let Phase::Tools { wait } = ls.phase else {
            return Ok(());
        };
        let blocked = self.start_jobs(rec, ls, cx)?;
        if !rec.turn.is_active() {
            // A call or a child entry not stored past the quota ended the turn (9.3): the loop
            // stops with the calls aborted.
            return self.openai_stop(rec, ls, cx);
        }
        if self.stop_on_errors(rec, ls, cx)? {
            return Ok(());
        }
        if ls.calls.is_empty() {
            ls.phase = Phase::Ready {
                wait: None,
                attempt: 0,
            };
            if !initial {
                self.go(rec, ls, cx, Pending::default(), None, 0)?;
            }
            return Ok(());
        }
        let wait = match (blocked, wait) {
            (Blocked::Shared, Some(n)) => Some(n),
            (Blocked::Shared, None) => Some(self.fifo_put(rec, cx)?),
            _ => None,
        };
        ls.phase = Phase::Tools { wait };
        Ok(())
    }

    fn start_jobs(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<Blocked, KernelError> {
        let handle = ls.tools.as_ref().map(|t| t.handle.clone());
        let mut i = 0;
        while i < ls.calls.len() {
            if ls.calls[i].st != CallState::Pending {
                i += 1;
                continue;
            }
            if ls.serial
                && ls
                    .calls
                    .iter()
                    .any(|c| matches!(c.st, CallState::Job { .. }))
            {
                return Ok(Blocked::No);
            }
            let name = ls.calls[i].item["name"].as_str().unwrap_or_default();
            if builtin_task(rec, ls, name) {
                // A `task` call holds no job slot or stream: the child is the work (9.7).
                let open = ls.calls.len();
                self.start_task(rec, ls, cx, i)?;
                if !rec.turn.is_active() || errors_reached(ls) {
                    return Ok(Blocked::No);
                }
                if ls.calls.len() == open {
                    i += 1;
                }
                continue;
            }
            if builtin_skill(ls, name) {
                let call = ls.calls.remove(i);
                let (output, ok) = match skill::read(&ls.policy.skills, &call.item) {
                    Ok(md) => (md.to_string(), true),
                    Err(reason) => (error_text(reason), false),
                };
                self.put_output(rec, ls, cx, &call.item, output, ok)?;
                if !rec.turn.is_active() || errors_reached(ls) {
                    return Ok(Blocked::No);
                }
                continue;
            }
            if ls.jobs >= self.limits.session_job || rec.stream_room().is_err() {
                return Ok(Blocked::Session);
            }
            let boot = cx.slots.boot;
            // Work already waiting in the FIFO goes first (9.5): a session that is not in it
            // queues behind.
            let queued = !matches!(ls.phase, Phase::Tools { wait: Some(_) })
                && cx.slots.head != cx.slots.tail;
            if queued
                || cx.slots.job >= self.limits.job
                || cx.a.owner.job_at(boot) >= self.limits.owner_job
            {
                return Ok(Blocked::Shared);
            }
            let item = ls.calls[i].item.clone();
            let field = |k: &str| item[k].as_str().unwrap_or_default().to_string();
            let key = format!("{}/{}", rec.id, field("call_id"));
            let started = match &handle {
                Some(h) => self
                    .host
                    .tools_call(h, &key, &field("name"), &field("arguments")),
                None => Err("no handle".to_string()),
            };
            match started {
                Ok(stream) => {
                    self.register_job(rec, cx, &stream)?;
                    ls.jobs += 1;
                    ls.calls[i].st = CallState::Job { stream };
                    i += 1;
                }
                Err(e) => {
                    let call = ls.calls.remove(i);
                    self.put_output(rec, ls, cx, &call.item, error_text(&e), false)?;
                    if errors_reached(ls) {
                        return Ok(Blocked::No);
                    }
                }
            }
        }
        Ok(Blocked::No)
    }

    fn register_job(
        &mut self,
        rec: &mut Record,
        cx: &mut Cx,
        stream: &str,
    ) -> Result<(), KernelError> {
        let mut key = StreamKey::new(&rec.id, JOB);
        key.drained = self.foreign;
        let bytes = serde_json::to_vec(&key).expect("a stream key serializes");
        self.state
            .put(&format!("stream/{stream}"), bytes)
            .map_err(|e| KernelError::TooLarge(e.0 as u64))?;
        rec.add_stream(stream.to_string())?;
        cx.slots.job += 1;
        cx.a.owner.open_job(cx.slots.boot);
        cx.dirty = true;
        Ok(())
    }

    /// Frees a job stream's slot and key; `cancel` ends the stream at the host too.
    pub(crate) fn release_job(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        stream: &str,
        cx: &mut Cx,
        cancel: bool,
    ) {
        self.state.delete(&format!("stream/{stream}"));
        if cancel {
            self.cancel_provider(stream);
        }
        rec.streams.retain(|s| s != stream);
        if rec.boot == cx.slots.boot {
            cx.slots.job = cx.slots.job.saturating_sub(1);
            cx.a.owner.close_job(cx.slots.boot);
            cx.dirty = true;
        }
        ls.jobs = ls.jobs.saturating_sub(1);
        self.drain_due = true;
    }

    /// One item of a job stream: the call's output (AC4).
    pub(crate) fn job_item(
        &mut self,
        stream: &str,
        session: &str,
        body: &[u8],
    ) -> Result<crate::sessions::Work, KernelError> {
        let on = |ls: &LoopState| ls.calls.iter().any(|c| c.on(stream));
        self.with_loop(session, on, |this, rec, ls, cx| {
            let Some(at) = ls.calls.iter().position(|c| c.on(stream)) else {
                return Ok(());
            };
            let call = ls.calls.remove(at);
            let (out, ok) = job_result(body);
            this.put_output(rec, ls, cx, &call.item, out, ok)?;
            this.after_output(rec, ls, cx)
        })
    }

    /// The end of a job stream (9.5): its slot is free; a call that got no item gets `aborted`.
    pub(crate) fn job_gone(
        &mut self,
        rec: &mut Record,
        ls: Option<&mut LoopState>,
        cx: &mut Cx,
        stream: &str,
    ) -> Result<(), KernelError> {
        let Some(ls) = ls else {
            self.release_job_only(rec, stream, cx);
            return Ok(());
        };
        self.release_job(rec, ls, stream, cx, false);
        if let Some(at) = ls.calls.iter().position(|c| c.on(stream)) {
            let call = ls.calls.remove(at);
            self.put_output(rec, ls, cx, &call.item, "aborted".into(), false)?;
        }
        self.after_output(rec, ls, cx)
    }

    fn release_job_only(&mut self, rec: &mut Record, stream: &str, cx: &mut Cx) {
        let mut none = LoopState::default();
        self.release_job(rec, &mut none, stream, cx, false);
    }

    pub(crate) fn after_output(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        if !rec.turn.is_active() {
            // An output not stored past the quota ended the turn (9.3): the loop stops.
            if matches!(ls.phase, Phase::Tools { .. } | Phase::Waiting) {
                self.openai_stop(rec, ls, cx)?;
            }
            return Ok(());
        }
        if !self.timer_carries_owner(&rec.owner) {
            return self.stop_for_timer_origin(rec, ls, cx);
        }
        if self.stop_on_errors(rec, ls, cx)? {
            return Ok(());
        }
        self.settle_calls(rec, ls, cx, false)
    }

    /// A place in the FIFO for a session whose pending calls wait for a kernel or owner job slot.
    fn fifo_put(&mut self, rec: &Record, cx: &mut Cx) -> Result<u64, KernelError> {
        self.fifo_entry(rec, cx)
    }

    /// 9.5: `interrupt`, `cancel` and a quota stop while `waiting` or `tools`: every job stream is
    /// cancelled, every open call gets `aborted`, every open ask is denied `by: kernel`.
    pub(crate) fn abort_calls(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        // A live child's call is closed with the others, so the child stops too: interrupt
        // semantics, as the cleanup of 9.5 (S134 r1 F1.1: a limit or quota stop of the root
        // leaves no child running). A child its root's interrupt, cancel or delete already
        // stopped has left `children` and is skipped.
        self.stop_children(rec, ls, cx, false)?;
        self.close_all_with(rec, cx.past, "interrupted", cx.now, cx.work, false)?;
        for s in rec.streams.clone() {
            let kind = self
                .state
                .get(&format!("stream/{s}"))
                .and_then(|b| serde_json::from_slice::<StreamKey>(&b).ok())
                .map(|k| k.kind);
            if kind.as_deref() == Some(JOB) {
                self.release_job(rec, ls, &s, cx, true);
            }
        }
        ls.jobs = 0;
        for call in std::mem::take(&mut ls.calls) {
            self.put_aborted(rec, ls, cx, &call.item)?;
        }
        Ok(())
    }

    /// 9.7 "Limits": when the failure streak reached `max-tool-errors`, the turn stops
    /// `stopped {limit: max-tool-errors}` with the other open and pending calls aborted by the
    /// interrupt cleanup (those aborts do not count). True when it stopped.
    fn stop_on_errors(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
    ) -> Result<bool, KernelError> {
        if !errors_reached(ls) {
            return Ok(false);
        }
        self.abort_calls(rec, ls, cx)?;
        if rec.turn.is_active() {
            let stop = Stop::Stopped {
                limit: "max-tool-errors".into(),
            };
            self.end_turn(rec, ls, cx, stop)?;
        } else {
            self.openai_stop(rec, ls, cx)?;
        }
        Ok(true)
    }
}

impl Call {
    fn on(&self, stream: &str) -> bool {
        matches!(&self.st, CallState::Job { stream: s } if s == stream)
    }
}

/// The session advertises the built-in `task`: a root that names delegates (9.7).
pub(crate) fn delegating(rec: &Record, ls: &LoopState) -> bool {
    rec.parent.is_none() && !ls.policy.delegates.is_empty()
}

/// A call named `task` is the kernel's built-in in a delegating root, and refused as one in a
/// child (9.7); anywhere else the name is free, and a toolset function of that name runs as any
/// described tool (S134 r2 F2.1).
pub(crate) fn builtin_task(rec: &Record, ls: &LoopState, name: &str) -> bool {
    name == TASK && (rec.parent.is_some() || delegating(rec, ls))
}

fn builtin_skill(ls: &LoopState, name: &str) -> bool {
    name == SKILL && !ls.policy.skills.is_empty()
}

/// One tool as the request lists it (AC1).
fn definition(t: &Tool) -> Value {
    let schema: Value = serde_json::from_str(&t.input_schema)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
    json!({
        "type": "function",
        "name": t.name,
        "description": t.description,
        "parameters": schema,
        "strict": false,
    })
}
