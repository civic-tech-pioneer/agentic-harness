//! The checked Claude Code provider (TECHNICAL 2.2, 10 "Checked"). Only the kernel may call it.
//!
//! It starts `claude` with `pre-tool-check`, forwards every item of that stream to its own stream (the
//! kernel's), and judges each input, `check` and `ask` with TypeSafe System One (`judge.rs`) before
//! the input reaches `claude` or the check is answered. No call waits for the judge: a judge call
//! spans calls, driven by the judge stream's items, its `ended` and a deadline timer.
//!
//! Every call gets a fresh instance, so every session lives in the state interface:
//!
//! | Key | Value |
//! |---|---|
//! | `s/<own stream>` | the session (`Session`): the `claude` stream, `timeout-s`, `initialized`, the next input number, the input queue (`launch` first), whether its head is being judged, the in-flight judge calls by `on` (kind, request, attempts, judge stream, status, a held answer), the run of denials and the last 50 verdicts, and the judge's excerpt (`said`, `agent`: the person's latest input that reached `claude` and the assistant's latest text, 2 KiB each) |
//! | `c/<claude stream>` | the own stream, to find the session from a `claude` item |
//! | `j/<judge stream>` | `<own stream>\n<on>`, to find the call from a judge item |
//! | `jb/<judge stream>` | the body bytes of the judge's answer so far |
//! | `settings` | `{"unsafe-at":…,"safe-at":…}` |
//!
//! Timers are `judge/<own stream>/<on>`. A judge call is final when its entry leaves `calls`: the
//! verdict, the deadline or a refusal removes it, cancels its timer and judge stream and deletes
//! its `j/` and `jb/` keys, so anything that arrives later finds nothing and changes nothing.
//!
//! Choices the spec leaves open:
//! - A launch with no input text has nothing to judge: no judge call for `launch`.
//! - `interrupt` and the breaker drop every queued input and the one being judged, but keep the
//!   `launch` entry (as `say: false`) while its verdict is out: no input passes before it (case 50).
//! - After the breaker fires its run and window start again from zero.
//! - A verdict item that cannot be pushed because the own stream is full defers the item call
//!   (10.7); in a timer call it is retried 1 s later; in `ended` and `say` calls, which cannot
//!   defer, it is dropped (the denial itself still goes out).
wit_bindgen::generate!({ path: "../wit", world: "claude-code-checked", generate_all });

mod judge;

use exports::mods::claude_code::sessions::Started;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use watane::host::claude_types::{BadLaunch, Decision, SessionError, StartError};
use watane::host::streams::{Outcome as StreamOutcome, SinkOutcome, StreamError};
use watane::host::timers::TimerError;

/// The single kernel identity for this build (TECHNICAL 2.2). The release
/// builder pins the publishing channel before compiling the provider.
const KERNEL: &str = match option_env!("AGENT_KERNEL_IDENTITY") {
    Some(identity) => identity,
    None => "local/agent",
};

/// The only identity that passes: an exact byte comparison.
fn is_kernel(identity: &str) -> bool {
    identity.as_bytes() == KERNEL.as_bytes()
}

const DEFAULT_TIMEOUT_S: u64 = 60;
const MAX_ATTEMPTS: u32 = 3;
const BREAKER_RUN: u32 = 3;
const BREAKER_WINDOW: usize = 50;
const BREAKER_COUNT: usize = 10;
const MAX_BODY: usize = 1 << 20;
const DEFAULT_UNSAFE_AT: f64 = 0.7;
const DEFAULT_SAFE_AT: f64 = 0.3;
/// The bound of each text of the judge's excerpt, in bytes (D1.2): two texts, so at most 4 KiB.
const EXCERPT_MAX: usize = 2048;

/// The first `EXCERPT_MAX` bytes of `t`, cut at a character boundary.
fn head(t: &str) -> String {
    let mut end = t.len().min(EXCERPT_MAX);
    while !t.is_char_boundary(end) {
        end -= 1;
    }
    t[..end].to_string()
}

/// The last `EXCERPT_MAX` bytes of `t`, cut at a character boundary.
fn tail(t: &str) -> String {
    let mut start = t.len().saturating_sub(EXCERPT_MAX);
    while !t.is_char_boundary(start) {
        start += 1;
    }
    t[start..].to_string()
}

/// The text blocks of a `claude` `assistant` line, joined; none when it has no text.
fn assistant_text(v: &Value) -> Option<String> {
    let text = v
        .get("message")?
        .get("content")?
        .as_array()?
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.is_empty()).then_some(text)
}

/// Every host call the provider makes, so the logic runs natively against a fake.
trait Host {
    fn get(&mut self, key: &str) -> Option<Vec<u8>>;
    fn put(&mut self, key: &str, value: &[u8]);
    fn delete(&mut self, key: &str);
    fn open(&mut self) -> Result<String, StreamError>;
    fn push(&mut self, s: &str, item: &[u8]) -> Result<(), StreamError>;
    fn close(&mut self, s: &str, outcome: StreamOutcome);
    fn cancel(&mut self, s: &str);
    fn resume(&mut self, s: &str);
    fn claude_start(&mut self, launch: &str) -> Result<String, StartError>;
    fn say(&mut self, s: &str, text: &str) -> Result<(), SessionError>;
    fn answer(&mut self, s: &str, ask: &str, d: &Decision) -> Result<(), SessionError>;
    fn interrupt(&mut self, s: &str) -> Result<(), SessionError>;
    fn compact(&mut self, s: &str) -> Result<(), SessionError>;
    fn judge_start(&mut self, request: &str) -> Result<String, StartError>;
    fn after(&mut self, id: &str, delay_ms: u64) -> Result<(), TimerError>;
    fn cancel_timer(&mut self, id: &str);
}

struct Real;

impl Host for Real {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        watane::host::state::get(key)
    }
    fn put(&mut self, key: &str, value: &[u8]) {
        watane::host::state::put(key, value)
    }
    fn delete(&mut self, key: &str) {
        watane::host::state::delete(key)
    }
    fn open(&mut self) -> Result<String, StreamError> {
        watane::host::streams::open()
    }
    fn push(&mut self, s: &str, item: &[u8]) -> Result<(), StreamError> {
        watane::host::streams::push(s, item)
    }
    fn close(&mut self, s: &str, outcome: StreamOutcome) {
        watane::host::streams::close(s, &outcome)
    }
    fn cancel(&mut self, s: &str) {
        watane::host::streams::cancel(s)
    }
    fn resume(&mut self, s: &str) {
        let _ = watane::host::streams::resume(s);
    }
    fn claude_start(&mut self, launch: &str) -> Result<String, StartError> {
        watane::host::claude_code::start(launch)
    }
    fn say(&mut self, s: &str, text: &str) -> Result<(), SessionError> {
        watane::host::claude_session::say(s, text)
    }
    fn answer(&mut self, s: &str, ask: &str, d: &Decision) -> Result<(), SessionError> {
        watane::host::claude_session::answer(s, ask, d)
    }
    fn interrupt(&mut self, s: &str) -> Result<(), SessionError> {
        watane::host::claude_session::interrupt(s)
    }
    fn compact(&mut self, s: &str) -> Result<(), SessionError> {
        watane::host::claude_session::compact(s)
    }
    fn judge_start(&mut self, request: &str) -> Result<String, StartError> {
        watane::host::https_stream::start(request)
    }
    fn after(&mut self, id: &str, delay_ms: u64) -> Result<(), TimerError> {
        watane::host::timers::after(id, delay_ms)
    }
    fn cancel_timer(&mut self, id: &str) {
        watane::host::timers::cancel(id)
    }
}

// ---------------------------------------------------------------------------------------------
// Small readers shared with the unchecked provider

/// The text of an input, as the kernel reads it: the `text` of the ACP text blocks of a JSON
/// array joined with a blank line (none when there is none); any other input is its own text
/// (none when empty).
fn input_text(input: &str) -> Option<String> {
    let text = match serde_json::from_str::<Value>(input) {
        Ok(Value::Array(blocks)) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => input.to_string(),
    };
    (!text.is_empty()).then_some(text)
}

fn stream_error_case(e: &StreamError) -> &'static str {
    match e {
        StreamError::NotYours => "not-yours",
        StreamError::UnknownStream => "unknown-stream",
        StreamError::Full => "full",
        StreamError::TooLarge => "too-large",
    }
}

fn start_error_case(e: &StartError) -> &'static str {
    match e {
        StartError::BadLaunch(_) => "bad-launch",
        StartError::Destination => "destination",
        StartError::ForbiddenHeader(_) => "forbidden-header",
        StartError::TooLarge => "too-large",
        StartError::Full => "full",
        StartError::NotOwner => "not-owner",
        StartError::NotDirect => "not-direct",
    }
}

fn bad_launch(field: &str, reason: &str) -> StartError {
    StartError::BadLaunch(BadLaunch {
        field: field.to_string(),
        reason: reason.to_string(),
    })
}

/// How the kernel reads why the `claude` stream ended (TECHNICAL 2.1): the outcome's name.
fn outcome_name(o: &StreamOutcome) -> String {
    match o {
        StreamOutcome::Closed => "closed".into(),
        StreamOutcome::Cancelled => "cancelled".into(),
        StreamOutcome::Lagging => "lagging".into(),
        StreamOutcome::Replaced => "replaced".into(),
        StreamOutcome::ConsumerFailed => "consumer-failed".into(),
        StreamOutcome::IdleTimeout => "idle-timeout".into(),
        StreamOutcome::Gone => "gone".into(),
        StreamOutcome::Mismatch(f) => format!("mismatch({f})"),
        StreamOutcome::NoBinary => "no-binary".into(),
        StreamOutcome::Version(v) => format!("version {} < {}", v.found, v.need),
        StreamOutcome::ConsentRefused(w) => format!("consent-refused({w})"),
        StreamOutcome::NoApp => "no-app".into(),
        StreamOutcome::NoAccount => "no-account".into(),
        StreamOutcome::Failed(m) => m.clone(),
    }
}

// ---------------------------------------------------------------------------------------------
// The session record

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Input,
    Check,
    Ask,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Input => "input",
            Kind::Check => "check",
            Kind::Ask => "ask",
        }
    }
    fn parse(s: &str) -> Option<Kind> {
        match s {
            "input" => Some(Kind::Input),
            "check" => Some(Kind::Check),
            "ask" => Some(Kind::Ask),
            _ => None,
        }
    }
}

/// An input waiting for its verdict; `launch` is the first. `say` is false once an `interrupt`
/// dropped it: its verdict is still read, but it sends nothing.
#[derive(Clone, Debug)]
struct Input {
    on: String,
    text: String,
    say: bool,
}

/// One judge call in flight.
#[derive(Clone, Debug)]
struct Call {
    kind: Kind,
    request: String,
    attempts: u32,
    jstream: Option<String>,
    status: Option<u64>,
    /// The kernel's answer to an ask whose verdict is pending (10, step 6).
    held: Option<Decision>,
}

struct Session {
    own: String,
    claude: String,
    timeout_s: u64,
    initialized: bool,
    next_n: u64,
    queue: Vec<Input>,
    judging: bool,
    calls: BTreeMap<String, Call>,
    run: u32,
    window: Vec<bool>,
    /// The person's latest input that reached `claude` (its start) and the assistant's latest text
    /// (its end): the judge's excerpt of the session (10 "Judge call", D1.2).
    said: String,
    agent: String,
    // Not stored: what the current call may do when its own stream is full.
    /// A full own stream is an `Err(Full)`: the caller defers or retries and discards the session.
    strict: bool,
    /// The call is rolled back when it defers, so a host call made before a later `Err(Full)` is harmless.
    rollback: bool,
    /// The session ended in this call: nothing is saved.
    dead: bool,
}

/// The own stream is full.
struct Full;

fn decision_json(d: &Decision) -> Value {
    match d {
        Decision::Allow(x) => json!({"allow": x}),
        Decision::Deny(m) => json!({"deny": m}),
        Decision::None => json!("none"),
    }
}

fn decision_from(v: &Value) -> Option<Decision> {
    if v.as_str() == Some("none") {
        return Some(Decision::None);
    }
    if let Some(a) = v.get("allow") {
        return Some(Decision::Allow(a.as_str().map(str::to_string)));
    }
    Some(Decision::Deny(v.get("deny")?.as_str()?.to_string()))
}

impl Session {
    /// The judge's excerpt: the person's latest request and the assistant's latest text, each
    /// bounded; empty parts are left out.
    fn excerpt(&self) -> Vec<(&'static str, &str)> {
        [
            ("user", self.said.as_str()),
            ("assistant", self.agent.as_str()),
        ]
        .into_iter()
        .filter(|(_, t)| !t.is_empty())
        .collect()
    }

    fn new(own: &str, claude: &str, timeout_s: u64, queue: Vec<Input>) -> Session {
        Session {
            own: own.to_string(),
            claude: claude.to_string(),
            timeout_s,
            initialized: false,
            next_n: 2,
            queue,
            judging: false,
            calls: BTreeMap::new(),
            run: 0,
            window: Vec::new(),
            said: String::new(),
            agent: String::new(),
            strict: false,
            rollback: false,
            dead: false,
        }
    }

    fn load<H: Host>(h: &mut H, own: &str) -> Option<Session> {
        let v: Value = serde_json::from_slice(&h.get(&format!("s/{own}"))?).ok()?;
        let queue = v
            .get("queue")?
            .as_array()?
            .iter()
            .map(|e| {
                Some(Input {
                    on: e.get("on")?.as_str()?.to_string(),
                    text: e.get("text")?.as_str()?.to_string(),
                    say: e.get("say")?.as_bool()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let mut calls = BTreeMap::new();
        for (on, c) in v.get("calls")?.as_object()? {
            calls.insert(
                on.clone(),
                Call {
                    kind: Kind::parse(c.get("kind")?.as_str()?)?,
                    request: c.get("request")?.as_str()?.to_string(),
                    attempts: c.get("attempts")?.as_u64()? as u32,
                    jstream: c.get("jstream").and_then(Value::as_str).map(str::to_string),
                    status: c.get("status").and_then(Value::as_u64),
                    held: c.get("held").and_then(decision_from),
                },
            );
        }
        Some(Session {
            own: own.to_string(),
            claude: v.get("claude")?.as_str()?.to_string(),
            timeout_s: v.get("timeout-s")?.as_u64()?,
            initialized: v.get("initialized")?.as_bool()?,
            next_n: v.get("next-n")?.as_u64()?,
            queue,
            judging: v.get("judging")?.as_bool()?,
            calls,
            run: v.get("run")?.as_u64()? as u32,
            window: v
                .get("window")?
                .as_array()?
                .iter()
                .map(|b| b.as_bool())
                .collect::<Option<_>>()?,
            said: v
                .get("said")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            agent: v
                .get("agent")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            strict: false,
            rollback: false,
            dead: false,
        })
    }

    fn save<H: Host>(&self, h: &mut H) {
        if self.dead {
            return;
        }
        let calls: Map<String, Value> = self
            .calls
            .iter()
            .map(|(on, c)| {
                let mut o = json!({
                    "kind": c.kind.as_str(), "request": c.request, "attempts": c.attempts,
                    "jstream": c.jstream, "status": c.status,
                });
                if let Some(d) = &c.held {
                    o["held"] = decision_json(d);
                }
                (on.clone(), o)
            })
            .collect();
        let v = json!({
            "claude": self.claude, "timeout-s": self.timeout_s, "initialized": self.initialized,
            "next-n": self.next_n, "judging": self.judging, "run": self.run, "window": self.window,
            "queue": self.queue.iter()
                .map(|i| json!({"on": i.on, "text": i.text, "say": i.say}))
                .collect::<Vec<_>>(),
            "calls": calls, "said": self.said, "agent": self.agent,
        });
        h.put(&format!("s/{}", self.own), v.to_string().as_bytes());
    }
}

fn timer_id(own: &str, on: &str) -> String {
    format!("judge/{own}/{on}")
}

/// Pushes an item on the own stream. A full stream is `Err(Full)` only while the call is strict;
/// any other failure (the kernel's stream is gone) drops the item.
fn push<H: Host>(h: &mut H, s: &Session, v: &Value) -> Result<(), Full> {
    match h.push(&s.own, v.to_string().as_bytes()) {
        Err(StreamError::Full) if s.strict => Err(Full),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------------
// Judge calls

/// The checking item, the deadline timer and the first attempt (10, step 3).
fn start_judge<H: Host>(
    h: &mut H,
    s: &mut Session,
    on: &str,
    kind: Kind,
    state: Value,
) -> Result<(), Full> {
    push(h, s, &json!({"type": "checking", "on": on}))?;
    s.calls.insert(
        on.to_string(),
        Call {
            kind,
            request: judge::request(judge::ORIGIN, &state),
            attempts: 0,
            jstream: None,
            status: None,
            held: None,
        },
    );
    let ms = (s.timeout_s * 1000).saturating_sub(2000).max(1);
    if h.after(&timer_id(&s.own, on), ms).is_err() {
        return finish(
            h,
            s,
            on,
            judge::Verdict::unsafe_because("judge: timer refused"),
        );
    }
    attempt(h, s, on)
}

/// One `https-stream.start` of a call; a refusal is unsafe at once.
fn attempt<H: Host>(h: &mut H, s: &mut Session, on: &str) -> Result<(), Full> {
    let Some(call) = s.calls.get(on) else {
        return Ok(());
    };
    match h.judge_start(&call.request) {
        Ok(j) => {
            h.put(&format!("j/{j}"), format!("{}\n{on}", s.own).as_bytes());
            if let Some(call) = s.calls.get_mut(on) {
                call.attempts += 1;
                call.jstream = Some(j);
                call.status = None;
            }
            Ok(())
        }
        Err(e) => {
            let reason = format!("judge: request refused ({})", start_error_case(&e));
            finish(h, s, on, judge::Verdict::unsafe_because(&reason))
        }
    }
}

/// Removes the timer and the judge stream of a call that is over.
fn drop_call_io<H: Host>(h: &mut H, own: &str, on: &str, call: &Call) {
    h.cancel_timer(&timer_id(own, on));
    if let Some(j) = &call.jstream {
        h.delete(&format!("j/{j}"));
        h.delete(&format!("jb/{j}"));
        h.cancel(j);
    }
}

fn verdict_item(on: &str, v: &judge::Verdict, run: u32) -> Value {
    json!({"type": "verdict", "on": on, "outcome": v.outcome.as_str(),
           "reasons": v.reasons, "denials-in-a-row": run})
}

/// The final verdict of a call: the item, the action it decides, the breaker. A verdict for a
/// call that is no longer in flight changes nothing. `Err(Full)` leaves nothing done.
fn finish<H: Host>(h: &mut H, s: &mut Session, on: &str, v: judge::Verdict) -> Result<(), Full> {
    let Some(kind) = s.calls.get(on).map(|c| c.kind) else {
        return Ok(());
    };
    let safe = v.outcome == judge::Outcome::Safe;
    s.window.push(!safe);
    if s.window.len() > BREAKER_WINDOW {
        s.window.remove(0);
    }
    s.run = if safe { 0 } else { s.run + 1 };
    push(h, s, &verdict_item(on, &v, s.run))?;
    if !s.rollback {
        s.strict = false;
    }
    let call = s.calls.remove(on).expect("checked above");
    drop_call_io(h, &s.own, on, &call);
    let reasons = v.reasons.join("; ");
    match kind {
        Kind::Check => {
            let d = if safe {
                Decision::None
            } else {
                Decision::Deny(reasons)
            };
            let _ = h.answer(&s.claude, on, &d);
        }
        Kind::Ask => {
            // "Safe" never answers allow: only a deny, or the owner's own answer once held.
            let d = if safe {
                call.held
            } else {
                Some(Decision::Deny(reasons))
            };
            if let Some(d) = d {
                let _ = h.answer(&s.claude, on, &d);
            }
        }
        Kind::Input => {
            s.judging = false;
            if s.queue.first().is_none_or(|i| i.on != on) {
                return Ok(());
            }
            let input = s.queue.remove(0);
            if safe {
                if input.say {
                    let _ = h.say(&s.claude, &input.text);
                    s.said = head(&input.text);
                    s.agent.clear();
                }
            } else if input.on == "launch" {
                end_session(h, s, true, Some(StreamOutcome::Closed));
                return Ok(());
            }
        }
    }
    breaker(h, s);
    if kind == Kind::Input {
        pump(h, s)?;
    }
    Ok(())
}

/// 3 denials in a row or 10 of the last 50: interrupt the session and say why (10 "Breaker").
fn breaker<H: Host>(h: &mut H, s: &mut Session) {
    let by_run = s.run >= BREAKER_RUN;
    if !by_run && s.window.iter().filter(|d| **d).count() < BREAKER_COUNT {
        return;
    }
    let rule = if by_run {
        format!("{BREAKER_RUN} denials in a row")
    } else {
        format!("{BREAKER_COUNT} denials in the last {BREAKER_WINDOW} verdicts")
    };
    let item = json!({"type": "verdict", "on": "breaker", "outcome": "unsafe",
        "reasons": [format!("{rule}: the session was stopped")], "denials-in-a-row": s.run});
    let _ = h.push(&s.own, item.to_string().as_bytes());
    let _ = h.interrupt(&s.claude);
    drop_inputs(h, s);
    s.run = 0;
    s.window.clear();
}

/// Starts the judge call of the input at the head of the queue, once `claude` is initialized and
/// no input is being judged: one at a time, in order.
fn pump<H: Host>(h: &mut H, s: &mut Session) -> Result<(), Full> {
    if !s.initialized || s.judging {
        return Ok(());
    }
    let Some(head) = s.queue.first().cloned() else {
        return Ok(());
    };
    s.judging = true;
    let state = judge::input_state(&head.text, &s.excerpt());
    start_judge(h, s, &head.on, Kind::Input, state)
}

/// Drops every input waiting for a verdict or for `initialized` (10, step 8). The launch keeps its
/// place without its input: nothing passes before the launch's safe verdict.
fn drop_inputs<H: Host>(h: &mut H, s: &mut Session) {
    match s.queue.first_mut() {
        Some(head) if head.on == "launch" => {
            head.say = false;
            s.queue.truncate(1);
        }
        Some(head) => {
            if s.judging
                && let Some(call) = s.calls.remove(&head.on)
            {
                drop_call_io(h, &s.own, &head.on, &call);
            }
            s.queue.clear();
            s.judging = false;
        }
        None => {}
    }
}

/// Ends the session: every judge call, the index keys and the record go.
fn end_session<H: Host>(
    h: &mut H,
    s: &mut Session,
    cancel_claude: bool,
    close_own: Option<StreamOutcome>,
) {
    for (on, call) in std::mem::take(&mut s.calls) {
        drop_call_io(h, &s.own, &on, &call);
    }
    h.delete(&format!("s/{}", s.own));
    h.delete(&format!("c/{}", s.claude));
    if cancel_claude {
        h.cancel(&s.claude);
    }
    if let Some(o) = close_own {
        h.close(&s.own, o);
    }
    s.dead = true;
}

// ---------------------------------------------------------------------------------------------
// Settings

fn thresholds<H: Host>(h: &mut H) -> (f64, f64) {
    h.get("settings")
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| Some((v.get("unsafe-at")?.as_f64()?, v.get("safe-at")?.as_f64()?)))
        .unwrap_or((DEFAULT_UNSAFE_AT, DEFAULT_SAFE_AT))
}

fn settings_set<H: Host>(
    h: &mut H,
    who: &str,
    unsafe_at: f64,
    safe_at: f64,
) -> Result<(), exports::mods::claude_code::settings::SettingsError> {
    use exports::mods::claude_code::settings::SettingsError;
    if who != "page" {
        return Err(SettingsError::NotOwner);
    }
    if !(0.0 <= safe_at && safe_at < unsafe_at && unsafe_at <= 1.0) {
        return Err(SettingsError::Refused(
            "0 <= safe-at < unsafe-at <= 1 is required".into(),
        ));
    }
    h.put(
        "settings",
        json!({"unsafe-at": unsafe_at, "safe-at": safe_at})
            .to_string()
            .as_bytes(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// `sessions`

/// `start` after the owner check (10 "Checked", step 1).
fn start_with<H: Host>(
    h: &mut H,
    who: &str,
    launch: &str,
    input: &str,
) -> Result<Started, StartError> {
    if !is_kernel(who) {
        return Err(StartError::NotOwner);
    }
    let Ok(Value::Object(mut doc)) = serde_json::from_str::<Value>(launch) else {
        return Err(bad_launch("launch", "not a JSON object"));
    };
    let timeout_s = match doc
        .get("pre-tool-check")
        .and_then(|c| c.get("timeout-s"))
        .and_then(Value::as_u64)
    {
        Some(t) => t,
        None => {
            doc.insert(
                "pre-tool-check".into(),
                json!({"timeout-s": DEFAULT_TIMEOUT_S}),
            );
            DEFAULT_TIMEOUT_S
        }
    };
    let claude = h.claude_start(&Value::Object(doc).to_string())?;
    let own = match h.open() {
        Ok(own) => own,
        Err(e) => {
            h.cancel(&claude);
            return Err(bad_launch("stream", stream_error_case(&e)));
        }
    };
    let queue = input_text(input)
        .map(|text| Input {
            on: "launch".into(),
            text,
            say: true,
        })
        .into_iter()
        .collect();
    Session::new(&own, &claude, timeout_s, queue).save(h);
    h.put(&format!("c/{claude}"), own.as_bytes());
    Ok(Started {
        stream: own,
        direct: false,
    })
}

fn say_with<H: Host>(h: &mut H, who: &str, s: &str, content: &str) -> Result<(), SessionError> {
    if !is_kernel(who) {
        return Err(SessionError::NotOwner);
    }
    let mut sess = Session::load(h, s).ok_or(SessionError::UnknownStream)?;
    if content.trim_start().starts_with('/') {
        return Err(SessionError::SlashCommand);
    }
    if let Some(text) = input_text(content) {
        sess.queue.push(Input {
            on: format!("input/{}", sess.next_n),
            text,
            say: true,
        });
        sess.next_n += 1;
        let _ = pump(h, &mut sess);
        sess.save(h);
    }
    Ok(())
}

fn answer_with<H: Host>(
    h: &mut H,
    who: &str,
    s: &str,
    ask: &str,
    d: Decision,
) -> Result<(), SessionError> {
    if !is_kernel(who) {
        return Err(SessionError::NotOwner);
    }
    let mut sess = Session::load(h, s).ok_or(SessionError::UnknownStream)?;
    match sess.calls.get_mut(ask) {
        // A check is answered by the provider alone.
        Some(c) if c.kind == Kind::Check => Err(SessionError::UnknownAsk),
        Some(c) if c.kind == Kind::Ask => {
            if c.held.is_some() {
                return Err(SessionError::UnknownAsk);
            }
            c.held = Some(d);
            sess.save(h);
            Ok(())
        }
        // Anything else is the host's to decide (`unknown-ask`).
        _ => h.answer(&sess.claude, ask, &d),
    }
}

fn interrupt_with<H: Host>(h: &mut H, who: &str, s: &str) -> Result<(), SessionError> {
    if !is_kernel(who) {
        return Err(SessionError::NotOwner);
    }
    let mut sess = Session::load(h, s).ok_or(SessionError::UnknownStream)?;
    drop_inputs(h, &mut sess);
    sess.save(h);
    h.interrupt(&sess.claude)
}

fn compact_with<H: Host>(h: &mut H, who: &str, s: &str) -> Result<(), SessionError> {
    if !is_kernel(who) {
        return Err(SessionError::NotOwner);
    }
    let sess = Session::load(h, s).ok_or(SessionError::UnknownStream)?;
    h.compact(&sess.claude)
}

// ---------------------------------------------------------------------------------------------
// Items, endings and timers

/// `stream-sink.items`: a batch of the `claude` stream or of a judge stream.
fn items_with<H: Host>(h: &mut H, s: &str, items: &[Vec<u8>]) -> SinkOutcome {
    let result = if let Some(own) = h.get(&format!("c/{s}")) {
        claude_items(h, &String::from_utf8_lossy(&own), items)
    } else if let Some(key) = h.get(&format!("j/{s}")) {
        judge_items(h, s, &String::from_utf8_lossy(&key), items)
    } else {
        Ok(())
    };
    match result {
        Ok(()) => SinkOutcome::Taken,
        Err(Full) => SinkOutcome::Defer,
    }
}

/// Forwards every item unchanged, then reads it: `initialized`, `check`, `ask`.
fn claude_items<H: Host>(h: &mut H, own: &str, items: &[Vec<u8>]) -> Result<(), Full> {
    let Some(mut s) = Session::load(h, own) else {
        return Ok(());
    };
    s.strict = true;
    s.rollback = true;
    for body in items {
        if let Err(StreamError::Full) = h.push(own, body) {
            return Err(Full);
        }
        let Ok(v) = serde_json::from_slice::<Value>(body) else {
            continue;
        };
        let id = v.get("id").and_then(Value::as_str).unwrap_or("");
        match v.get("type").and_then(Value::as_str) {
            Some("initialized") if !s.initialized => {
                s.initialized = true;
                pump(h, &mut s)?;
            }
            Some(t @ ("check" | "ask")) if !id.is_empty() && !s.calls.contains_key(id) => {
                let kind = if t == "check" { Kind::Check } else { Kind::Ask };
                let tool = v.get("tool").and_then(Value::as_str).unwrap_or("");
                let input = v.get("input").cloned().unwrap_or(Value::Null);
                let state = judge::tool_state(tool, &input, &s.excerpt());
                start_judge(h, &mut s, id, kind, state)?;
            }
            Some("assistant") => {
                if let Some(t) = assistant_text(&v) {
                    s.agent = tail(&t);
                }
            }
            _ => {}
        }
        if s.dead {
            return Ok(());
        }
    }
    s.save(h);
    Ok(())
}

/// The items of a judge stream: `{status, headers}` first, then the body. The first readable
/// verdict is final.
fn judge_items<H: Host>(h: &mut H, j: &str, key: &str, items: &[Vec<u8>]) -> Result<(), Full> {
    let (own, on) = key.split_once('\n').unwrap_or((key, ""));
    let Some(mut s) = Session::load(h, own) else {
        h.cancel(j);
        return Ok(());
    };
    if s.calls.get(on).and_then(|c| c.jstream.as_deref()) != Some(j) {
        return Ok(());
    }
    s.strict = true;
    s.rollback = true;
    for item in items {
        let Some(call) = s.calls.get_mut(on) else {
            break;
        };
        let Some(status) = call.status else {
            call.status = Some(
                serde_json::from_slice::<Value>(item)
                    .ok()
                    .and_then(|v| v.get("status")?.as_u64())
                    .unwrap_or(0),
            );
            continue;
        };
        let mut body = h.get(&format!("jb/{j}")).unwrap_or_default();
        if body.len() + item.len() <= MAX_BODY {
            body.extend_from_slice(item);
            h.put(&format!("jb/{j}"), &body);
        }
        if let Some(answers) = judge::read(status, &body) {
            let (unsafe_at, safe_at) = thresholds(h);
            let verdict = judge::map(&answers, unsafe_at, safe_at);
            finish(h, &mut s, on, verdict)?;
            break;
        }
    }
    s.save(h);
    Ok(())
}

/// `stream-sink.ended`: the `claude` stream ended, or a judge stream did.
fn ended_with<H: Host>(h: &mut H, s: &str, outcome: &StreamOutcome) {
    if let Some(own) = h.get(&format!("c/{s}")) {
        let Some(mut sess) = Session::load(h, &String::from_utf8_lossy(&own)) else {
            return;
        };
        // Whatever ended `claude` ends the session; the kernel reads the cause (2.1).
        let close = match outcome {
            StreamOutcome::Closed => StreamOutcome::Closed,
            other => StreamOutcome::Failed(outcome_name(other)),
        };
        end_session(h, &mut sess, false, Some(close));
    } else if let Some(key) = h.get(&format!("j/{s}")) {
        let key = String::from_utf8_lossy(&key).into_owned();
        judge_ended(h, s, &key, outcome);
    }
}

/// A judge stream ended with no readable verdict: another attempt while there are attempts and the
/// deadline is ahead (its call is still in flight), else unsafe.
fn judge_ended<H: Host>(h: &mut H, j: &str, key: &str, outcome: &StreamOutcome) {
    let (own, on) = key.split_once('\n').unwrap_or((key, ""));
    let Some(mut s) = Session::load(h, own) else {
        return;
    };
    let Some(call) = s.calls.get_mut(on) else {
        return;
    };
    if call.jstream.as_deref() != Some(j) {
        return;
    }
    call.jstream = None;
    h.delete(&format!("j/{j}"));
    h.delete(&format!("jb/{j}"));
    let attempts = call.attempts;
    // Another attempt cannot help when the account is refused.
    let refused = matches!(
        outcome,
        StreamOutcome::NoAccount | StreamOutcome::ConsentRefused(_)
    );
    let _ = if attempts < MAX_ATTEMPTS && !refused {
        attempt(h, &mut s, on)
    } else {
        let reason = if refused {
            format!("judge: {}", outcome_name(outcome))
        } else {
            format!("judge: no readable verdict after {attempts} attempts")
        };
        finish(h, &mut s, on, judge::Verdict::unsafe_because(&reason))
    };
    s.save(h);
}

/// `timer-sink.fired`: the deadline of a judge call is an unsafe verdict.
fn fired_with<H: Host>(h: &mut H, id: &str) {
    let Some((own, on)) = id.strip_prefix("judge/").and_then(|r| r.split_once('/')) else {
        return;
    };
    let Some(mut s) = Session::load(h, own) else {
        return;
    };
    s.strict = true;
    let v = judge::Verdict::unsafe_because("judge: no verdict before the deadline");
    match finish(h, &mut s, on, v) {
        Ok(()) => s.save(h),
        // The own stream is full: the call stays and the deadline is tried again soon.
        Err(Full) => {
            let _ = h.after(id, 1000);
        }
    }
}

/// `stream-producer.ready`: the own stream has room again; resume what deferred.
fn ready_with<H: Host>(h: &mut H, own: &str) {
    let Some(s) = Session::load(h, own) else {
        return;
    };
    h.resume(&s.claude);
    for call in s.calls.values() {
        if let Some(j) = &call.jstream {
            h.resume(j);
        }
    }
}

/// `stream-producer.ended`: the kernel's stream ended, so the `claude` stream is cancelled.
fn producer_ended_with<H: Host>(h: &mut H, own: &str) {
    if let Some(mut s) = Session::load(h, own) {
        end_session(h, &mut s, true, None);
    }
}

// ---------------------------------------------------------------------------------------------
// The exports

struct Mod;

impl exports::mods::claude_code::sessions::Guest for Mod {
    fn start(launch: String, input: String) -> Result<Started, StartError> {
        start_with(
            &mut Real,
            &watane::host::caller::identity(),
            &launch,
            &input,
        )
    }

    fn say(s: String, content: String) -> Result<(), SessionError> {
        say_with(&mut Real, &watane::host::caller::identity(), &s, &content)
    }

    fn answer(s: String, ask: String, d: Decision) -> Result<(), SessionError> {
        answer_with(&mut Real, &watane::host::caller::identity(), &s, &ask, d)
    }

    fn interrupt(s: String) -> Result<(), SessionError> {
        interrupt_with(&mut Real, &watane::host::caller::identity(), &s)
    }

    fn compact(s: String) -> Result<(), SessionError> {
        compact_with(&mut Real, &watane::host::caller::identity(), &s)
    }
}

impl exports::mods::claude_code::settings::Guest for Mod {
    fn get() -> exports::mods::claude_code::settings::Current {
        let (unsafe_at, safe_at) = thresholds(&mut Real);
        exports::mods::claude_code::settings::Current {
            unsafe_at,
            safe_at,
            origin: judge::ORIGIN.to_string(),
        }
    }

    fn set(
        unsafe_at: f64,
        safe_at: f64,
    ) -> Result<(), exports::mods::claude_code::settings::SettingsError> {
        settings_set(
            &mut Real,
            &watane::host::caller::identity(),
            unsafe_at,
            safe_at,
        )
    }
}

impl exports::watane::host::stream_sink::Guest for Mod {
    fn items(s: String, items: Vec<watane::host::streams::Item>) -> SinkOutcome {
        let bodies: Vec<Vec<u8>> = items.into_iter().map(|i| i.body).collect();
        items_with(&mut Real, &s, &bodies)
    }

    fn ended(s: String, outcome: StreamOutcome) {
        ended_with(&mut Real, &s, &outcome)
    }
}

impl exports::watane::host::stream_producer::Guest for Mod {
    fn ready(s: String) {
        ready_with(&mut Real, &s)
    }

    fn ended(s: String, _outcome: StreamOutcome) {
        producer_ended_with(&mut Real, &s)
    }
}

impl exports::watane::host::timer_sink::Guest for Mod {
    fn fired(id: String) {
        fired_with(&mut Real, &id)
    }
}

export!(Mod);

#[cfg(test)]
mod tests;
