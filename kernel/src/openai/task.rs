//! The `task` function: an OpenAI root delegates a first turn to a named child session
//! (TECHNICAL 9.7; S134-4). Admission is `delegation::admit` (S134-3); this file starts the child,
//! keeps the association, and resolves the root's call exactly once.
//!
//! Two decisions, inside the spec:
//!
//! 1. **Where the pending association lives.** In records already there, with no new key: the
//!    child's record holds `task_call` (the root's call id) while its delegated first turn is
//!    outstanding, the root's loop state holds the open call as `CallState::Child {session}` in
//!    `loop/<root>` `calls`, and the root's record lists the child in `children`. All three are
//!    written together with the indexed `child` entry in the call that starts the child, so the
//!    association has no stream, takes no request or job slot, and is bounded by the live cap
//!    (`subagent-limits.live`, at most 20, so by `children` too), the active index and the owner's
//!    session and byte quotas. `resolve_task` clears all three once: `task_call` is taken first,
//!    so a second end of the same child (a second turn) finds nothing to resolve, and a root that
//!    no longer holds the matching `Child` call (aborted, restarted) makes it a no-op. A root or
//!    child of an older boot is an inactive relationship and resolves nothing (R4).
//!
//! 2. **How the child's first request is started inside an items call.** It never is: the root's
//!    `response.completed` arrives with `held` set and the call may still be dropped, so the child
//!    is created with its first turn `running`, its user message in the transcript and wire, and
//!    its loop `ready {wait: n}`: a place in the FIFO that this call queued (9.5 Slots, the
//!    deputy's r4 and r5 rulings). The end of that same call sends it (`after_call`), under the
//!    owner's origin, with a request slot free; else the child's turn stops `idle {stopped
//!    {limit: busy}}` and the root's call gets `busy` with the child's id. The FIFO route is used
//!    in every call that starts a child (also an owner's `answer`), so one path serves them all
//!    and a start never sends before its writes. A call that may be another owner's (`foreign`:
//!    a drained job's item) starts no child: the output is `{"err":{"code":"busy"}}`.
//!
//! 3. **Stopping (S134-6).** `stop_children` runs inside the root's interrupt, cancel or delete,
//!    before the root's calls are aborted: it cuts each live child's `task_call` first (so its end
//!    resolves nothing and the root's call gets the cleanup's `aborted`), then interrupts or
//!    cancels it with a `Work` of its own, whose pushes leave before the root's (`Work::stopped`).
//!    A child's own cancel or delete resolves the root's call `aborted` (`abort_task`). A restart
//!    stops no child: the root's calls are aborted by its R4 step, each child takes its own, and
//!    its `task_call` goes with it.
//!
//! One-call bound (17): a call that starts children writes the root, and for each `task` call it
//! dispatches one new child record, its first turn and its FIFO place. A response can hold at most
//! as many accepted `task` calls as the root's live cap (at most 20: the rest are refused at
//! admission without touching anything). A call that resolves a call touches the root, the one
//! child it ends and their streams; the root's pushes go on the root's own owner stream
//! (`Work::peer`), and the bytes each record owns are attributed to it (`peer_bytes`).

use crate::delegation::admit_with_skills;
use crate::error::{KernelError, Problem, Refusal};
use crate::openai::run::{Cx, Launch, LoopState};
use crate::openai::skill;
use crate::openai::tools::CallState;
use crate::profile::{Backend, Profile};
use crate::queue::check_input;
use crate::record::{Record, Stop, Turn, ID_BYTES};
use crate::sessions::{add_signed, check_slash, Host, Kernel, Work};
use crate::state::State;
use crate::transcript::NewEntry;
use crate::{active, Change};
use serde_json::{json, Value};

/// The name of the built-in function (9.7).
pub(crate) const TASK: &str = "task";

/// The most bytes of a `task` output (the normal tool-output bound, 9.4).
const OUTPUT_BYTES: usize = 64 * 1024;

/// The definition a root's requests advertise, after its toolsets' tools (9.5).
pub(crate) fn definition(delegates: &[String]) -> Value {
    json!({
        "type": "function",
        "name": TASK,
        "description": format!(
            "Delegate one self-contained piece of work to a named agent profile. The profile runs \
             it as its own session and its final answer is the result. Profiles: {}.",
            delegates.join(", ")
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "profile": {"type": "string", "description": "the name of the profile to run"},
                "input": {"type": "string", "description": "the work, as the profile's first message"}
            },
            "required": ["profile", "input"],
            "additionalProperties": false
        },
        "strict": false
    })
}

/// The root's compiled profile as admission compares it: what the loop kept at `start` (9.2).
fn root_profile(ls: &LoopState) -> Profile {
    let p = &ls.policy;
    Profile {
        backend: Some("openai".into()),
        model: Some(ls.model.clone()),
        toolsets: p.toolsets.clone(),
        delegates: p.delegates.clone(),
        allow: p.allow.clone(),
        deny: p.deny.clone(),
        mode: p.mode.clone(),
        ask: p.ask,
        pre_tool_check: p.pre_tool_check.clone(),
        subagent_limits: p.subagent_limits.clone(),
        max_turns: p.max_turns,
        max_tool_errors: p.max_tool_errors,
        max_tokens: p.max_tokens,
        ask_expiry: p.ask_expiry,
        persist: p.persist,
        ..Profile::default()
    }
}

fn refused_out(refusals: &[Refusal]) -> String {
    json!({"err": {"code": "refused", "refusals": refusals}}).to_string()
}

fn code_out(code: &str, session: Option<&str>) -> String {
    match session {
        Some(s) => json!({"err": {"code": code, "session": s}}),
        None => json!({"err": {"code": code}}),
    }
    .to_string()
}

/// `{"ok":{"session","message"}}` with the message cut so the whole output stays within the
/// tool-output bound and valid JSON (9.7).
fn ok_out(session: &str, text: &str) -> String {
    let wrap = |m: &str| json!({"ok": {"session": session, "message": m}}).to_string();
    let full = wrap(text);
    if full.len() <= OUTPUT_BYTES {
        return full;
    }
    let mark = format!(" [cut by watane: {} bytes]", text.len());
    let mut end = text.len().min(OUTPUT_BYTES);
    loop {
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let out = wrap(&format!("{}{mark}", &text[..end]));
        if out.len() <= OUTPUT_BYTES || end == 0 {
            return out;
        }
        end = end.saturating_sub((out.len() - OUTPUT_BYTES).max(1));
    }
}

impl<S: State, H: Host> Kernel<S, H> {
    /// Gives the pending `task` call `at` its outcome: an output (refused, busy, quota), or the
    /// call becomes `Child {session}` with the child started (9.7 "Admission" and "The function
    /// and its lifetime"). Everything that can refuse is checked before anything is written.
    pub(crate) fn start_task(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        at: usize,
    ) -> Result<(), KernelError> {
        let item = ls.calls[at].item.clone();
        let call_id = item["call_id"].as_str().unwrap_or_default().to_string();
        let model = ls.model.clone();
        let refusal = |field: &str, reason: &str| {
            vec![Problem::new(field, reason).refusal(Backend::Openai, &model)]
        };
        // Step 1: the root's access again, against the root's owner (a child's own is its root's).
        if !self.may_work(&rec.owner.clone(), rec) {
            let root = rec
                .parent
                .as_deref()
                .and_then(|id| Record::load(&mut self.state, id));
            let owner = root
                .as_ref()
                .map_or(rec.owner.as_str(), |r| r.owner.as_str());
            let profile = root
                .as_ref()
                .map_or(rec.profile.as_str(), |r| r.profile.as_str());
            let reason = match self.profile_access_error(owner, profile) {
                crate::error::KernelError::Refused(mut rs) => rs.remove(0).reason,
                _ => "this client may not use this profile".into(),
            };
            let out = refused_out(&refusal("profile", &reason));
            return self.end_call(rec, ls, cx, at, out);
        }
        let args: Value = item["arguments"]
            .as_str()
            .and_then(|a| serde_json::from_str(a).ok())
            .unwrap_or(Value::Null);
        let name = args["profile"].as_str().unwrap_or_default().to_string();
        // Steps 2 to 5.
        let live = ls
            .calls
            .iter()
            .filter(|c| matches!(c.st, CallState::Child { .. }))
            .count() as u32;
        let root = root_profile(ls);
        let revisions = if ls.skill_listing.is_empty() || rec.parent.is_some() {
            skill::Revisions::new()
        } else {
            skill::revisions(&mut self.state, &rec.id)
        };
        let admitted = admit_with_skills(
            &root,
            &model,
            rec.parent.is_some(),
            live,
            &name,
            |n| {
                // A changed offer is unadopted at admission, even while its old bytes remain.
                if crate::access::index(&mut self.state)
                    .iter()
                    .any(|row| row.name == n && row.stale)
                {
                    None
                } else {
                    self.profile_json(n)
                }
            },
            |s| {
                skill::revision(s)
                    .as_ref()
                    .is_some_and(|r| revisions.get(&s.name) == Some(r))
            },
        );
        let admitted = match admitted {
            Ok(profile) => profile,
            Err(rs) => return self.end_call(rec, ls, cx, at, refused_out(&rs)),
        };
        let Some(input) = args["input"].as_str() else {
            let out = refused_out(&refusal("input", "input must be a string"));
            return self.end_call(rec, ls, cx, at, out);
        };
        match check_input(input).and_then(|()| check_slash(input)) {
            Ok(()) => {}
            Err(KernelError::Refused(rs)) => {
                return self.end_call(rec, ls, cx, at, refused_out(&rs));
            }
            Err(_) => {
                let out = refused_out(&refusal("input", "too large"));
                return self.end_call(rec, ls, cx, at, out);
            }
        }
        // The origin and the caps: a call that may be another owner's starts no child, and a
        // request the call queued already counts toward the slots (9.5 Slots).
        let queued = self.queued.len() as u32;
        let room = cx.slots.request + queued < self.limits.request
            && cx.a.owner.request_at(cx.slots.boot) + queued < self.limits.owner_request
            && active::check_room(cx.slots).is_ok();
        if self.foreign || !room {
            return self.end_call(rec, ls, cx, at, code_out("busy", None));
        }
        if cx.past.is_past() || cx.a.owner.sessions >= self.limits.owner_sessions {
            return self.end_call(rec, ls, cx, at, code_out("quota", None));
        }
        let id = self.host.new_id();
        if id.len() > ID_BYTES {
            return self.end_call(rec, ls, cx, at, code_out("backend", None));
        }

        // The `child` entry first: stored, or the root's turn ends `stopped {quota}` and nothing
        // else is written (the caller stops the loop).
        let entry = NewEntry {
            kind: "child".into(),
            uuid: None,
            body: json!({"session": id, "toolCallId": call_id}),
        };
        if self
            .append_loose(rec, cx.past, entry, cx.now, cx.work)?
            .is_none()
        {
            return Ok(());
        }
        self.create_child(rec, cx, &id, &name, &call_id, input, admitted)?;
        rec.children.push(id.clone());
        ls.calls[at].st = CallState::Child { session: id };
        Ok(())
    }

    /// The child's record, first turn and FIFO place, with the owner's page, the active index
    /// and the accounts that follow (as `start` does them, 9.3), in one write of this call.
    #[allow(clippy::too_many_arguments)]
    fn create_child(
        &mut self,
        root: &Record,
        cx: &mut Cx,
        id: &str,
        profile: &str,
        call_id: &str,
        input: &str,
        mut admitted: Profile,
    ) -> Result<(), KernelError> {
        let (n, page_key, mut page) = self.owner_page(&root.owner, cx.a);
        let blocks = json!([{"type": "text", "text": input}]).to_string();
        let mut child = Record {
            id: id.to_string(),
            owner: root.owner.clone(),
            profile: profile.to_string(),
            backend: "openai".into(),
            boot: cx.slots.boot,
            created: cx.now,
            updated: cx.now,
            owner_page: n as u32,
            parent: Some(root.id.clone()),
            task_call: Some(call_id.to_string()),
            ..Record::default()
        };
        let before = self.state.change().session;
        let skills = std::mem::take(&mut admitted.skills);
        let mut cls = Self::loop_from_profile(admitted, &skills);
        skill::save(&mut self.state, id, skills)?;
        let mut work = Work::default();
        self.start_turn(
            &mut child,
            cx.past,
            blocks.clone(),
            false,
            None,
            cx.now,
            &mut work,
        )?;
        let pending = self.pending_for(&child, &cls, &blocks);
        cls.reset_turn();
        {
            let mut ccx = Cx {
                slots: &mut *cx.slots,
                a: &mut *cx.a,
                work: &mut work,
                past: cx.past,
                now: cx.now,
                dirty: false,
            };
            // The request waits in the FIFO whatever the call: the call's end sends it.
            self.go(
                &mut child,
                &mut cls,
                &mut ccx,
                pending,
                Some(Launch::Wait),
                0,
            )?;
            cx.dirty |= ccx.dirty;
        }
        self.loop_save(id, &cls)?;
        cx.dirty |= self.sync_active(&mut child, cx.slots, false)?;
        page.push(id.to_string());
        self.state
            .put(
                &page_key,
                serde_json::to_vec(&page).expect("a page serializes"),
            )
            .map_err(|t| KernelError::TooLarge(t.0 as u64))?;
        cx.a.add_session(n);
        cx.work.hand_on_stopped(&mut work);
        self.save_peer(&mut child, before)
    }

    /// Saves a record the call works on besides its own: the session bytes written since `before`
    /// are the record's, not the call's session's (9.3 "Bytes").
    fn save_peer(&mut self, rec: &mut Record, before: i64) -> Result<(), KernelError> {
        let delta = self.state.change().session - before;
        rec.bytes = add_signed(rec.bytes, delta);
        self.state.adjust(Change {
            owner: 0,
            session: -delta,
        });
        rec.save(&mut self.state)
    }

    /// Removes the call `at` and stores `out` as its one output (not a success).
    fn end_call(
        &mut self,
        rec: &mut Record,
        ls: &mut LoopState,
        cx: &mut Cx,
        at: usize,
        out: String,
    ) -> Result<(), KernelError> {
        let call = ls.calls.remove(at);
        self.put_output(rec, ls, cx, &call.item, out, false)
    }

    /// A child's turn ended: if it was the delegated first turn, the root's call gets its output
    /// (9.7). `text` is the final answer of a completed turn. Later turns resolve nothing.
    pub(crate) fn end_task(
        &mut self,
        rec: &mut Record,
        cx: &mut Cx,
        text: Option<&str>,
    ) -> Result<(), KernelError> {
        if rec.task_call.is_none() {
            return Ok(());
        }
        let Turn::Idle { stop, .. } = &rec.turn else {
            return Ok(());
        };
        let id = rec.id.as_str();
        let (output, ok) = match stop {
            Stop::Completed => match text.filter(|t| !t.is_empty()) {
                Some(t) => (ok_out(id, t), true),
                None => (code_out("no-result", Some(id)), false),
            },
            Stop::Interrupted => (code_out("interrupted", Some(id)), false),
            Stop::Stopped { limit } => (code_out(limit, Some(id)), false),
            Stop::Failed { .. } => (code_out("backend", Some(id)), false),
        };
        self.resolve_task(rec, output, ok, cx)
    }

    /// The root's call for `child` gets the `aborted` output (the child was cancelled or deleted,
    /// 9.7). It is not counted toward `max-tool-errors`, like every cleanup abort.
    pub(crate) fn abort_task(
        &mut self,
        child: &mut Record,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        self.resolve_with(child, None, cx)
    }

    /// The owner's interrupt, cancel or delete of a root stops each live child it holds in a
    /// `Child` call, before the root's own calls are aborted (9.7). An interrupt ends the child's
    /// turn `interrupted` and keeps its handle; a cancel (or the delete) cancels it and closes the
    /// handle. The child's association is cut first, so its end resolves nothing: the root's call
    /// gets the cleanup's `aborted`. The child records stay. At most the live cap of children are
    /// touched, each with its own streams; their pushes leave before the root's.
    pub(crate) fn stop_children(
        &mut self,
        root: &mut Record,
        ls: &LoopState,
        cx: &mut Cx,
        cancel: bool,
    ) -> Result<(), KernelError> {
        let ids: Vec<String> = ls
            .calls
            .iter()
            .filter_map(|c| match &c.st {
                CallState::Child { session } => Some(session.clone()),
                _ => None,
            })
            .collect();
        for id in ids {
            if !root.children.contains(&id) {
                continue;
            }
            root.children.retain(|c| *c != id);
            let Some(mut child) = Record::load(&mut self.state, &id) else {
                continue;
            };
            if child.parent.as_deref() != Some(root.id.as_str())
                || child.owner != root.owner
                || child.boot != cx.slots.boot
            {
                continue;
            }
            child.task_call = None;
            let before = self.state.change().session;
            let mut work = Work::default();
            let live = child.live.clone();
            {
                let mut ccx = Cx {
                    slots: &mut *cx.slots,
                    a: &mut *cx.a,
                    work: &mut work,
                    past: cx.past,
                    now: cx.now,
                    dirty: false,
                };
                if cancel {
                    self.cancel_rec(&mut child, &mut ccx)?;
                } else {
                    self.interrupt_rec(&mut child, &mut ccx)?;
                }
                cx.dirty |= ccx.dirty;
            }
            self.save_peer(&mut child, before)?;
            cx.work.hand_on_stopped(&mut work);
            cx.work.stopped.push((live, work.pushes));
            cx.work.requests.append(&mut work.requests);
        }
        Ok(())
    }

    /// An interrupt of one OpenAI session in the root's call: its turn ends `interrupted`, its
    /// loop is idle, its handle kept.
    fn interrupt_rec(&mut self, rec: &mut Record, cx: &mut Cx) -> Result<(), KernelError> {
        let was = rec.turn.is_active();
        if !was {
            return Ok(());
        }
        let before = rec.deadlines.clone();
        if let Some(mut ls) = self.loop_load(&rec.id) {
            self.openai_stop(rec, &mut ls, cx)?;
            self.loop_save(&rec.id, &ls)?;
        }
        self.end_interrupted(rec, cx.past, cx.now, cx.work)?;
        cx.dirty |= self.sync_timer(rec, cx.slots, cx.a, &before, cx.now, false);
        self.sync_active(rec, cx.slots, was)?;
        rec.updated = cx.now;
        Ok(())
    }

    /// Writes the root's output for `child`'s delegated call once and continues the root as a
    /// finished job does (`after_output`); a no-op when the root no longer has the matching
    /// `Child {session}` call. `ok` is true only for the `{"ok"}` envelope (9.7 "Limits").
    pub(crate) fn resolve_task(
        &mut self,
        child: &mut Record,
        output: String,
        ok: bool,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        self.resolve_with(child, Some((output, ok)), cx)
    }

    /// `resolve_task`, or with no output the cleanup's `aborted`.
    fn resolve_with(
        &mut self,
        child: &mut Record,
        output: Option<(String, bool)>,
        cx: &mut Cx,
    ) -> Result<(), KernelError> {
        // The association is consumed here, whatever follows: it produces one output at most.
        let (Some(call_id), Some(root_id)) = (child.task_call.take(), child.parent.clone()) else {
            return Ok(());
        };
        let Some(mut root) = Record::load(&mut self.state, &root_id) else {
            return Ok(());
        };
        // Live delegation belongs to the boot that opened it (R4): an older one resolves nothing.
        if root.boot != cx.slots.boot || child.boot != cx.slots.boot {
            return Ok(());
        }
        let Some(mut ls) = self.loop_load(&root_id) else {
            return Ok(());
        };
        let Some(at) = ls.calls.iter().position(|c| {
            c.id() == call_id
                && matches!(&c.st, CallState::Child { session } if *session == child.id)
        }) else {
            return Ok(());
        };
        let (before, before_ls, was) = (root.clone(), ls.clone(), root.turn.is_active());
        let mark = self.state.change().session;
        let mut work = Work::default();
        let mut dirty;
        {
            let mut rcx = Cx {
                slots: &mut *cx.slots,
                a: &mut *cx.a,
                work: &mut work,
                past: cx.past,
                now: cx.now,
                dirty: false,
            };
            let call = ls.calls.remove(at);
            root.children.retain(|c| *c != child.id);
            match output {
                Some((output, ok)) => {
                    self.put_output(&mut root, &mut ls, &mut rcx, &call.item, output, ok)?
                }
                None => self.put_aborted(&mut root, &mut ls, &mut rcx, &call.item)?,
            }
            self.after_output(&mut root, &mut ls, &mut rcx)?;
            dirty = rcx.dirty;
        }
        self.settle_openai(&mut root, &before, cx.slots, cx.a, was, cx.now, &mut dirty)?;
        cx.dirty |= dirty;
        if ls != before_ls {
            self.loop_save(&root_id, &ls)?;
        }
        if root != before {
            root.updated = cx.now;
        }
        self.save_peer(&mut root, mark)?;
        let (live, pushes) = cx.work.peer.take().unwrap_or((None, Vec::new()));
        let mut pushes = pushes;
        pushes.append(&mut work.pushes);
        cx.work.peer = Some((live.or(root.live.clone()), pushes));
        cx.work.hand_on_stopped(&mut work);
        cx.work.requests.append(&mut work.requests);
        Ok(())
    }
}
