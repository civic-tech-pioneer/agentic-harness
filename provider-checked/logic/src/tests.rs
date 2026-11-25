//! The provider's scenarios against a fake host: no network, no host. The scripted judge answers as
//! dev's `Play::Raw` does: a `{status, headers}` item, the body bytes, then `ended`.

use super::*;
use exports::mods::claude_code::settings::SettingsError;

const K: &str = KERNEL;

#[derive(Clone, Default)]
struct Fake {
    kv: BTreeMap<String, Vec<u8>>,
    /// Every host call but the state's, in order, as text.
    log: Vec<String>,
    /// The items pushed on own streams: (stream, item).
    pushed: Vec<(String, Vec<u8>)>,
    launches: Vec<String>,
    /// The judge requests: (judge stream, request JSON).
    requests: Vec<(String, String)>,
    timers: BTreeMap<String, u64>,
    own_n: u32,
    claude_n: u32,
    judge_n: u32,
    full: bool,
    refuse_timer: bool,
    refuse_judge: bool,
    start_err: Option<StartError>,
    open_err: bool,
}

impl Host for Fake {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.kv.get(key).cloned()
    }
    fn put(&mut self, key: &str, value: &[u8]) {
        self.kv.insert(key.to_string(), value.to_vec());
    }
    fn delete(&mut self, key: &str) {
        self.kv.remove(key);
    }
    fn open(&mut self) -> Result<String, StreamError> {
        if self.open_err {
            return Err(StreamError::Full);
        }
        self.own_n += 1;
        self.log.push("open".into());
        Ok(format!("own{}", self.own_n))
    }
    fn push(&mut self, s: &str, item: &[u8]) -> Result<(), StreamError> {
        if self.full {
            return Err(StreamError::Full);
        }
        self.pushed.push((s.to_string(), item.to_vec()));
        Ok(())
    }
    fn close(&mut self, s: &str, outcome: StreamOutcome) {
        self.log
            .push(format!("close {s} {}", outcome_name(&outcome)));
    }
    fn cancel(&mut self, s: &str) {
        self.log.push(format!("cancel {s}"));
    }
    fn resume(&mut self, s: &str) {
        self.log.push(format!("resume {s}"));
    }
    fn claude_start(&mut self, launch: &str) -> Result<String, StartError> {
        if let Some(e) = self.start_err.take() {
            return Err(e);
        }
        self.claude_n += 1;
        self.launches.push(launch.to_string());
        self.log.push("claude-start".into());
        Ok(format!("cl{}", self.claude_n))
    }
    fn say(&mut self, s: &str, text: &str) -> Result<(), SessionError> {
        self.log.push(format!("say {s} {text}"));
        Ok(())
    }
    fn answer(&mut self, s: &str, ask: &str, d: &Decision) -> Result<(), SessionError> {
        let d = match d {
            Decision::Allow(None) => "allow".to_string(),
            Decision::Allow(Some(x)) => format!("allow:{x}"),
            Decision::Deny(m) => format!("deny:{m}"),
            Decision::None => "none".to_string(),
        };
        self.log.push(format!("answer {s} {ask} {d}"));
        Ok(())
    }
    fn interrupt(&mut self, s: &str) -> Result<(), SessionError> {
        self.log.push(format!("interrupt {s}"));
        Ok(())
    }
    fn compact(&mut self, s: &str) -> Result<(), SessionError> {
        self.log.push(format!("compact {s}"));
        Ok(())
    }
    fn judge_start(&mut self, request: &str) -> Result<String, StartError> {
        if self.refuse_judge {
            return Err(StartError::Destination);
        }
        self.judge_n += 1;
        let j = format!("j{}", self.judge_n);
        self.requests.push((j.clone(), request.to_string()));
        self.log.push("judge-start".into());
        Ok(j)
    }
    fn after(&mut self, id: &str, delay_ms: u64) -> Result<(), TimerError> {
        if self.refuse_timer {
            return Err(TimerError::TooMany);
        }
        self.timers.insert(id.to_string(), delay_ms);
        Ok(())
    }
    fn cancel_timer(&mut self, id: &str) {
        self.timers.remove(id);
        self.log.push(format!("cancel-timer {id}"));
    }
}

impl Fake {
    /// The log lines that start with `prefix`.
    fn calls(&self, prefix: &str) -> Vec<String> {
        self.log
            .iter()
            .filter(|l| l.starts_with(prefix))
            .cloned()
            .collect()
    }
    /// The items pushed on `own`, as `type:on[:outcome]` (a `checking` or `verdict`) or `type`.
    fn items(&self, own: &str) -> Vec<String> {
        self.pushed
            .iter()
            .filter(|(s, _)| s == own)
            .map(|(_, b)| {
                let v: Value = serde_json::from_slice(b).unwrap();
                let t = v["type"].as_str().unwrap();
                match (v["on"].as_str(), v["outcome"].as_str()) {
                    (Some(on), Some(o)) => format!("{t}:{on}:{o}"),
                    (Some(on), None) => format!("{t}:{on}"),
                    _ => t.to_string(),
                }
            })
            .collect()
    }
    /// The verdict items pushed on `own`.
    fn verdicts(&self, own: &str) -> Vec<Value> {
        self.pushed
            .iter()
            .filter(|(s, _)| s == own)
            .map(|(_, b)| serde_json::from_slice::<Value>(b).unwrap())
            .filter(|v| v["type"] == "verdict")
            .collect()
    }
    /// The state text of the judge request `i` (an input's text, or the tool's name).
    fn asked(&self, i: usize) -> String {
        let r: Value = serde_json::from_str(&self.requests[i].1).unwrap();
        let b: Value = serde_json::from_str(r["body"].as_str().unwrap()).unwrap();
        let st = &b["state"];
        st["text"]
            .as_str()
            .or(st["tool"].as_str())
            .unwrap()
            .to_string()
    }
}

/// An item call as the host makes it: a deferred call is rolled back whole.
fn deliver(f: &mut Fake, s: &str, items: &[&[u8]]) -> SinkOutcome {
    let snap = f.clone();
    let items: Vec<Vec<u8>> = items.iter().map(|i| i.to_vec()).collect();
    let r = items_with(f, s, &items);
    if matches!(r, SinkOutcome::Defer) {
        *f = snap;
        f.log.push("defer".into());
    }
    r
}

fn from_claude(f: &mut Fake, claude: &str, item: Value) -> SinkOutcome {
    deliver(f, claude, &[item.to_string().as_bytes()])
}

/// The scripted judge answers stream `j` with a status and a body, then ends it normally.
fn judge_says(f: &mut Fake, j: &str, status: u64, body: &str) {
    let head = json!({"status": status, "headers": {}}).to_string();
    deliver(f, j, &[head.as_bytes(), body.as_bytes()]);
    ended_with(f, j, &StreamOutcome::Closed);
}

fn answers(harm: f64, intent: f64) -> String {
    json!({"model": "jev-1.14.0", "answers": {"unsafe": {"type": "noul", "noul": harm}, "off_task": {"type": "noul", "noul": intent}}})
        .to_string()
}

fn judge_p(f: &mut Fake, j: &str, p: f64) {
    judge_says(f, j, 200, &answers(p, 0.0));
}

fn judge_safe(f: &mut Fake, j: &str) {
    judge_p(f, j, 0.0);
}

fn judge_unsafe(f: &mut Fake, j: &str) {
    judge_p(f, j, 0.9);
}

/// The judge stream of the latest request.
fn last_j(f: &Fake) -> String {
    f.requests.last().unwrap().0.clone()
}

fn start(f: &mut Fake, launch: &str, input: &str) -> (String, String) {
    let r = start_with(f, K, launch, input).unwrap();
    (r.stream, format!("cl{}", f.claude_n))
}

fn initialized(f: &mut Fake, claude: &str) {
    from_claude(f, claude, json!({"type": "initialized", "workspace": "w"}));
}

/// A session of the launch `{}` and the input `"launch text"`, initialized.
fn ready_session(f: &mut Fake) -> (String, String) {
    let (own, claude) = start(f, "{}", "launch text");
    initialized(f, &claude);
    (own, claude)
}

fn check(id: &str) -> Value {
    json!({"type": "check", "id": id, "tool_use_id": "t", "tool": "Bash", "input": {"command": "ls"}})
}

fn ask(id: &str) -> Value {
    json!({"type": "ask", "id": id, "tool_use_id": "t", "tool": "Write", "input": {}, "expires-at": 1})
}

// --- AC1 ------------------------------------------------------------------------------------

#[test]
fn ac1_every_function_refuses_a_caller_that_is_not_the_kernel_and_calls_nothing() {
    let mut f = Fake::default();
    let (own, _) = start(&mut f, "{}", "x");
    let before = f.clone();
    for who in [
        "local/agents",
        "page",
        "host",
        "",
        "local/agent ",
        "Local/agent",
    ] {
        assert!(matches!(
            start_with(&mut f, who, "{}", "x"),
            Err(StartError::NotOwner)
        ));
        assert!(matches!(
            say_with(&mut f, who, &own, "x"),
            Err(SessionError::NotOwner)
        ));
        assert!(matches!(
            answer_with(&mut f, who, &own, "a", Decision::None),
            Err(SessionError::NotOwner)
        ));
        assert!(matches!(
            interrupt_with(&mut f, who, &own),
            Err(SessionError::NotOwner)
        ));
        assert!(matches!(
            compact_with(&mut f, who, &own),
            Err(SessionError::NotOwner)
        ));
    }
    assert_eq!(f.log, before.log);
    assert_eq!(f.kv, before.kv);
    assert!(f.requests.is_empty() && f.pushed.is_empty());
}

#[test]
fn ac1_only_the_kernel_identity_passes() {
    let expected = option_env!("AGENT_KERNEL_IDENTITY").unwrap_or("local/agent");
    assert!(is_kernel(expected));
    assert!(!is_kernel(&format!("{expected}s")));
    assert!(!is_kernel(""));
}

// --- AC2 and case 37 ------------------------------------------------------------------------

#[test]
fn ac2_start_sets_pre_tool_check_keeps_the_rest_and_judges_nothing() {
    let mut f = Fake::default();
    let r = start_with(&mut f, K, r#"{"model":"m","tools":["Read"]}"#, "hi").unwrap();
    assert_eq!((r.stream.as_str(), r.direct), ("own1", false));
    let launch: Value = serde_json::from_str(&f.launches[0]).unwrap();
    assert_eq!(
        launch,
        json!({"model": "m", "tools": ["Read"], "pre-tool-check": {"timeout-s": 60}})
    );
    // case 37: start returns before any verdict, with no judge request, no say and no item.
    assert_eq!(f.log, ["claude-start", "open"]);
    assert!(f.requests.is_empty() && f.pushed.is_empty());
}

#[test]
fn ac2_a_given_timeout_is_kept() {
    let mut f = Fake::default();
    start(&mut f, r#"{"pre-tool-check":{"timeout-s":30}}"#, "hi");
    let launch: Value = serde_json::from_str(&f.launches[0]).unwrap();
    assert_eq!(launch["pre-tool-check"], json!({"timeout-s": 30}));
}

#[test]
fn ac2_a_launch_that_is_not_an_object_is_bad_launch() {
    for launch in ["[]", "\"x\"", "7", "null", "{", ""] {
        let mut f = Fake::default();
        match start_with(&mut f, K, launch, "x") {
            Err(StartError::BadLaunch(b)) => assert_eq!(b.field, "launch"),
            _ => panic!("expected bad-launch for {launch:?}"),
        }
        assert!(f.log.is_empty(), "{launch:?}");
    }
}

#[test]
fn ac2_a_host_start_error_passes_and_a_failed_open_cancels_claude() {
    let mut f = Fake {
        start_err: Some(StartError::Full),
        ..Fake::default()
    };
    assert!(matches!(
        start_with(&mut f, K, "{}", "x"),
        Err(StartError::Full)
    ));
    assert!(f.log.is_empty());
    let mut f = Fake {
        open_err: true,
        ..Fake::default()
    };
    assert!(matches!(
        start_with(&mut f, K, "{}", "x"),
        Err(StartError::BadLaunch(_))
    ));
    assert_eq!(f.log, ["claude-start", "cancel cl1"]);
}

#[test]
fn case37_no_request_before_initialized() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "launch text");
    say_with(&mut f, K, &own, "second").unwrap();
    from_claude(
        &mut f,
        &claude,
        json!({"type": "consent-pending", "what": "w"}),
    );
    assert!(f.requests.is_empty());
    assert_eq!(f.items(&own), ["consent-pending"]);
    initialized(&mut f, &claude);
    assert_eq!(f.requests.len(), 1);
}

// --- AC3 ------------------------------------------------------------------------------------

#[test]
fn ac3_inputs_wait_for_initialized_then_are_judged_one_at_a_time_in_order() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "launch text");
    say_with(&mut f, K, &own, "second").unwrap();
    say_with(&mut f, K, &own, "third").unwrap();
    assert!(f.requests.is_empty());
    initialized(&mut f, &claude);
    assert_eq!(f.requests.len(), 1);
    assert_eq!(f.asked(0), "launch text");
    assert!(
        f.calls("say ").is_empty(),
        "no input before its safe verdict"
    );
    judge_safe(&mut f, "j1");
    assert_eq!(f.calls("say "), ["say cl1 launch text"]);
    assert_eq!(f.requests.len(), 2);
    assert_eq!(f.asked(1), "second");
    judge_safe(&mut f, "j2");
    assert_eq!(f.asked(2), "third");
    judge_safe(&mut f, "j3");
    assert_eq!(
        f.calls("say "),
        ["say cl1 launch text", "say cl1 second", "say cl1 third"]
    );
    assert_eq!(
        f.items(&own),
        [
            "initialized",
            "checking:launch",
            "verdict:launch:safe",
            "checking:input/2",
            "verdict:input/2:safe",
            "checking:input/3",
            "verdict:input/3:safe",
        ]
    );
}

#[test]
fn case43_a_second_initialized_changes_nothing() {
    let mut f = Fake::default();
    let (_, claude) = ready_session(&mut f);
    let before = (f.requests.len(), f.timers.clone(), f.kv.clone());
    initialized(&mut f, &claude);
    assert_eq!((f.requests.len(), f.timers.clone(), f.kv.clone()), before);
}

#[test]
fn ac3_say_after_initialized_is_judged_at_once_when_nothing_is_pending() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    assert!(f.requests.is_empty(), "no launch input: nothing to judge");
    say_with(&mut f, K, &own, "now").unwrap();
    assert_eq!(f.asked(0), "now");
    assert_eq!(f.items(&own).last().unwrap(), "checking:input/2");
}

#[test]
fn ac3_an_input_is_read_as_the_kernel_reads_it() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    say_with(
        &mut f,
        K,
        &own,
        r#"[{"type":"text","text":"a"},{"type":"text","text":"b"}]"#,
    )
    .unwrap();
    assert_eq!(f.asked(0), "a\n\nb");
    assert!(matches!(
        say_with(&mut f, K, &own, "  /compact"),
        Err(SessionError::SlashCommand)
    ));
    assert!(matches!(
        say_with(&mut f, K, "nope", "x"),
        Err(SessionError::UnknownStream)
    ));
}

// --- AC4 and case 38 ------------------------------------------------------------------------

#[test]
fn ac4_an_unsafe_launch_ends_the_session_after_its_verdict_item() {
    for p in [0.9, 0.5] {
        let mut f = Fake::default();
        let (own, _) = ready_session(&mut f);
        say_with(&mut f, K, &own, "later").unwrap();
        judge_p(&mut f, "j1", p);
        assert!(f.calls("say ").is_empty());
        assert_eq!(f.requests.len(), 1, "the queued input is never judged");
        assert_eq!(
            f.items(&own).last().unwrap().as_str(),
            if p > 0.7 {
                "verdict:launch:unsafe"
            } else {
                "verdict:launch:unsure"
            }
        );
        let tail = &f.log[f.log.len() - 2..];
        assert_eq!(tail, ["cancel cl1", format!("close {own} closed").as_str()]);
        assert!(f.kv.is_empty(), "{:?}", f.kv.keys());
        assert!(f.timers.is_empty());
    }
}

#[test]
fn case38_an_unsafe_later_input_is_dropped_and_the_stream_stays() {
    let mut f = Fake::default();
    let (own, _) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    say_with(&mut f, K, &own, "bad").unwrap();
    judge_unsafe(&mut f, "j2");
    say_with(&mut f, K, &own, "good").unwrap();
    judge_safe(&mut f, "j3");
    assert_eq!(f.calls("say "), ["say cl1 launch text", "say cl1 good"]);
    assert!(f.calls("cancel cl").is_empty());
    assert!(f.calls("close").is_empty());
    let v = f.verdicts(&own);
    assert_eq!(v[1]["on"], "input/2");
    assert_eq!(v[1]["outcome"], "unsafe");
}

#[test]
fn case38_a_slow_judge_is_denied_by_the_timer_and_the_late_verdict_changes_nothing() {
    let mut f = Fake::default();
    let (own, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    from_claude(&mut f, &claude, check("c1"));
    assert_eq!(f.timers["judge/own1/c1"], 58_000);
    fired_with(&mut f, "judge/own1/c1");
    assert_eq!(
        f.calls("answer"),
        ["answer cl1 c1 deny:judge: no verdict before the deadline"]
    );
    assert_eq!(f.verdicts(&own).last().unwrap()["outcome"], "unsafe");
    let (log, pushed) = (f.log.len(), f.pushed.len());
    judge_safe(&mut f, "j2");
    assert_eq!((f.log.len(), f.pushed.len()), (log, pushed));
    fired_with(&mut f, "judge/own1/c1");
    assert_eq!((f.log.len(), f.pushed.len()), (log, pushed));
}

// --- AC5 and case 42 ------------------------------------------------------------------------

#[test]
fn ac5_a_check_is_answered_none_when_safe_and_deny_otherwise() {
    let mut f = Fake::default();
    let (own, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    from_claude(&mut f, &claude, check("c1"));
    assert!(f.calls("answer").is_empty(), "nothing before the verdict");
    judge_safe(&mut f, "j2");
    from_claude(&mut f, &claude, check("c2"));
    judge_says(&mut f, "j3", 200, &answers(0.5, 0.1));
    from_claude(&mut f, &claude, check("c3"));
    judge_says(&mut f, "j4", 200, &answers(0.1, 0.8));
    assert_eq!(
        f.calls("answer"),
        [
            "answer cl1 c1 none",
            "answer cl1 c2 deny:unsafe: 0.50; off_task: 0.10",
            "answer cl1 c3 deny:unsafe: 0.10; off_task: 0.80",
        ]
    );
    // The item reached the kernel's stream unchanged, then `checking`, then the verdict.
    let at = f.items(&own).iter().position(|i| i == "check").unwrap();
    assert_eq!(f.pushed[at].1, check("c1").to_string().as_bytes());
    assert_eq!(
        &f.items(&own)[at..at + 3],
        ["check", "checking:c1", "verdict:c1:safe"]
    );
}

#[test]
fn ac5_a_check_is_judged_with_its_tool_and_input() {
    let mut f = Fake::default();
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    let r: Value = serde_json::from_str(&f.requests[0].1).unwrap();
    let b: Value = serde_json::from_str(r["body"].as_str().unwrap()).unwrap();
    assert_eq!(
        b["state"],
        json!({"kind": "tool", "tool": "Bash", "input": {"command": "ls"}})
    );
}

#[test]
fn case42_an_ask_is_answered_only_deny_and_safe_leaves_it_unanswered() {
    let mut f = Fake::default();
    let (_, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    from_claude(&mut f, &claude, ask("a1"));
    judge_safe(&mut f, "j2");
    from_claude(&mut f, &claude, ask("a2"));
    judge_unsafe(&mut f, "j3");
    from_claude(&mut f, &claude, ask("a3"));
    judge_p(&mut f, "j4", 0.5);
    assert_eq!(
        f.calls("answer"),
        [
            "answer cl1 a2 deny:unsafe: 0.90; off_task: 0.00",
            "answer cl1 a3 deny:unsafe: 0.50; off_task: 0.00",
        ]
    );
}

#[test]
fn case42_the_owners_answer_to_a_pending_ask_waits_for_a_safe_verdict() {
    let mut f = Fake::default();
    let (own, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    from_claude(&mut f, &claude, ask("a1"));
    answer_with(
        &mut f,
        K,
        &own,
        "a1",
        Decision::Allow(Some("{\"x\":1}".into())),
    )
    .unwrap();
    assert!(f.calls("answer").is_empty());
    assert!(matches!(
        answer_with(&mut f, K, &own, "a1", Decision::Allow(None)),
        Err(SessionError::UnknownAsk)
    ));
    judge_safe(&mut f, "j2");
    assert_eq!(f.calls("answer"), ["answer cl1 a1 allow:{\"x\":1}"]);

    // After an unsafe verdict only the provider's deny goes; the later answer reaches the host.
    from_claude(&mut f, &claude, ask("a2"));
    answer_with(&mut f, K, &own, "a2", Decision::Allow(None)).unwrap();
    judge_unsafe(&mut f, "j3");
    assert_eq!(f.calls("answer").len(), 2);
    assert!(f.calls("answer")[1].starts_with("answer cl1 a2 deny:"));
    answer_with(&mut f, K, &own, "a2", Decision::Allow(None)).unwrap();
    assert_eq!(f.calls("answer")[2], "answer cl1 a2 allow");
}

#[test]
fn ac5_an_answer_for_an_unknown_id_is_forwarded_and_a_check_is_never_answered_by_the_kernel() {
    let mut f = Fake::default();
    let (own, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    answer_with(&mut f, K, &own, "zzz", Decision::Deny("no".into())).unwrap();
    assert_eq!(f.calls("answer"), ["answer cl1 zzz deny:no"]);
    from_claude(&mut f, &claude, check("c1"));
    assert!(matches!(
        answer_with(&mut f, K, &own, "c1", Decision::Deny("x".into())),
        Err(SessionError::UnknownAsk)
    ));
    assert_eq!(f.calls("answer").len(), 1);
}

// --- AC6, cases 44 and 50 -------------------------------------------------------------------

#[test]
fn ac6_a_judge_call_is_checking_request_timer() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, r#"{"pre-tool-check":{"timeout-s":30}}"#, "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    assert_eq!(f.items(&own), ["initialized", "check", "checking:c1"]);
    assert_eq!(f.timers["judge/own1/c1"], 28_000);
    let r: Value = serde_json::from_str(&f.requests[0].1).unwrap();
    assert_eq!(r["account"], "judge");
    assert_eq!(r["url"], format!("{}/v1/systemone", judge::ORIGIN));
    // The call's timer and stream are gone with its verdict.
    judge_safe(&mut f, "j1");
    assert!(f.timers.is_empty());
}

#[test]
fn case44_an_unsafe_or_unsure_body_is_one_request_and_a_late_end_changes_nothing() {
    for p in [0.9, 0.5] {
        let mut f = Fake::default();
        let (_, claude) = start(&mut f, "{}", "");
        initialized(&mut f, &claude);
        from_claude(&mut f, &claude, check("c1"));
        judge_p(&mut f, "j1", p);
        assert_eq!(f.requests.len(), 1);
        assert_eq!(f.calls("answer").len(), 1);
        assert!(f.calls("answer")[0].starts_with("answer cl1 c1 deny:"));
    }
}

#[test]
fn ac6_a_new_attempt_follows_an_ending_with_no_readable_verdict_at_most_three() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    judge_says(&mut f, "j1", 500, "oops");
    assert_eq!(f.requests.len(), 2);
    judge_says(&mut f, "j2", 200, "not json");
    assert_eq!(f.requests.len(), 3);
    assert!(f.calls("answer").is_empty());
    ended_with(&mut f, "j3", &StreamOutcome::Failed("boom".into()));
    assert_eq!(f.requests.len(), 3, "never a fourth");
    assert!(f.calls("answer")[0].starts_with("answer cl1 c1 deny:judge: no readable verdict"));
    assert_eq!(f.verdicts(&own)[0]["outcome"], "unsafe");
}

#[test]
fn ac6_a_readable_verdict_on_a_later_attempt_is_final() {
    let mut f = Fake::default();
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    ended_with(&mut f, "j1", &StreamOutcome::IdleTimeout);
    judge_safe(&mut f, "j2");
    assert_eq!(f.calls("answer"), ["answer cl1 c1 none"]);
    assert_eq!(f.requests.len(), 2);
}

#[test]
fn ac6_a_verdict_in_the_body_is_read_before_the_stream_ends() {
    let mut f = Fake::default();
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    let head = json!({"status": 200, "headers": {}}).to_string();
    let body = answers(0.0, 0.0);
    let (a, b) = body.as_bytes().split_at(10);
    deliver(&mut f, "j1", &[head.as_bytes(), a]);
    assert!(f.calls("answer").is_empty());
    deliver(&mut f, "j1", &[b]);
    assert_eq!(f.calls("answer"), ["answer cl1 c1 none"]);
}

#[test]
fn ac6_an_account_that_is_refused_is_unsafe_without_another_attempt() {
    let mut f = Fake::default();
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    ended_with(&mut f, "j1", &StreamOutcome::NoAccount);
    assert_eq!(f.requests.len(), 1);
    assert_eq!(f.calls("answer"), ["answer cl1 c1 deny:judge: no-account"]);
}

#[test]
fn case50_a_refused_timer_denies_that_on_at_once_and_makes_no_request() {
    let mut f = Fake {
        refuse_timer: true,
        ..Fake::default()
    };
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    assert!(f.requests.is_empty());
    assert_eq!(
        f.calls("answer"),
        ["answer cl1 c1 deny:judge: timer refused"]
    );
    assert_eq!(f.items(&own)[2..], ["checking:c1", "verdict:c1:unsafe"]);
}

#[test]
fn ac6_a_refused_request_denies_at_once() {
    let mut f = Fake {
        refuse_judge: true,
        ..Fake::default()
    };
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, ask("a1"));
    assert_eq!(
        f.calls("answer"),
        ["answer cl1 a1 deny:judge: request refused (destination)"]
    );
    assert!(f.timers.is_empty());
}

#[test]
fn case50_x_reaches_claude_only_after_the_launchs_safe_verdict() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "launch text");
    interrupt_with(&mut f, K, &own).unwrap();
    say_with(&mut f, K, &own, "X").unwrap();
    assert!(f.requests.is_empty());
    initialized(&mut f, &claude);
    assert_eq!(f.requests.len(), 1, "the launch is judged alone");
    assert_eq!(f.asked(0), "launch text");
    judge_safe(&mut f, "j1");
    // The launch's input was dropped by the interrupt: it says nothing; X follows.
    assert!(f.calls("say ").is_empty());
    assert_eq!(f.asked(1), "X");
    judge_safe(&mut f, "j2");
    assert_eq!(f.calls("say "), ["say cl1 X"]);
    assert_eq!(
        f.items(&own)
            .iter()
            .filter(|i| i.contains("input/2"))
            .count(),
        2
    );
}

#[test]
fn case50_x_never_reaches_claude_after_an_unsafe_launch() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "launch text");
    interrupt_with(&mut f, K, &own).unwrap();
    say_with(&mut f, K, &own, "X").unwrap();
    initialized(&mut f, &claude);
    judge_unsafe(&mut f, "j1");
    assert!(f.calls("say ").is_empty());
    assert_eq!(f.requests.len(), 1);
}

// --- AC7 ------------------------------------------------------------------------------------

#[test]
fn case46_two_sessions_with_the_same_on_keep_separate_timers_and_verdicts() {
    let mut f = Fake::default();
    let (own1, cl1) = start(&mut f, "{}", "");
    let (own2, cl2) = start(&mut f, r#"{"pre-tool-check":{"timeout-s":20}}"#, "");
    initialized(&mut f, &cl1);
    initialized(&mut f, &cl2);
    from_claude(&mut f, &cl1, check("c1"));
    from_claude(&mut f, &cl2, check("c1"));
    assert_eq!(f.timers["judge/own1/c1"], 58_000);
    assert_eq!(f.timers["judge/own2/c1"], 18_000);
    judge_safe(&mut f, "j1");
    assert!(f.timers.contains_key("judge/own2/c1"));
    assert!(!f.timers.contains_key("judge/own1/c1"));
    fired_with(&mut f, "judge/own2/c1");
    assert_eq!(f.calls("answer")[0], "answer cl1 c1 none");
    assert!(f.calls("answer")[1].starts_with("answer cl2 c1 deny:"));
    assert_eq!(f.verdicts(&own1)[0]["outcome"], "safe");
    assert_eq!(f.verdicts(&own2)[0]["outcome"], "unsafe");
    // A verdict of the other judge stream is not read as this one's.
    judge_unsafe(&mut f, "j2");
    assert_eq!(f.verdicts(&own2).len(), 1);
}

// --- AC8 and case 47 ------------------------------------------------------------------------

#[test]
fn case47_interrupt_drops_inputs_waiting_for_initialized() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    say_with(&mut f, K, &own, "A").unwrap();
    say_with(&mut f, K, &own, "B").unwrap();
    interrupt_with(&mut f, K, &own).unwrap();
    assert_eq!(f.calls("interrupt"), ["interrupt cl1"]);
    initialized(&mut f, &claude);
    assert!(f.requests.is_empty());
    initialized(&mut f, &claude);
    assert!(f.requests.is_empty() && f.calls("say ").is_empty());
}

#[test]
fn case47_a_verdict_after_interrupt_sends_nothing() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    say_with(&mut f, K, &own, "A").unwrap();
    say_with(&mut f, K, &own, "B").unwrap();
    interrupt_with(&mut f, K, &own).unwrap();
    assert!(f.timers.is_empty());
    let (log, pushed) = (f.log.len(), f.pushed.len());
    judge_safe(&mut f, "j1");
    assert_eq!((f.log.len(), f.pushed.len()), (log, pushed));
    assert_eq!(f.requests.len(), 1, "B is never judged");
    // A new input after the interrupt is judged and said.
    say_with(&mut f, K, &own, "C").unwrap();
    judge_safe(&mut f, "j2");
    assert_eq!(f.calls("say "), ["say cl1 C"]);
}

#[test]
fn ac8_interrupt_does_not_touch_the_judging_of_a_check() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    interrupt_with(&mut f, K, &own).unwrap();
    judge_safe(&mut f, "j1");
    assert_eq!(f.calls("answer"), ["answer cl1 c1 none"]);
}

// --- AC9 ------------------------------------------------------------------------------------

fn one_check(f: &mut Fake, claude: &str, id: &str, p: f64) {
    from_claude(f, claude, check(id));
    let j = last_j(f);
    judge_p(f, &j, p);
}

#[test]
fn ac9_three_denials_in_a_row_interrupt_the_session() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    one_check(&mut f, &claude, "c1", 0.9);
    one_check(&mut f, &claude, "c2", 0.9);
    one_check(&mut f, &claude, "c3", 0.0);
    one_check(&mut f, &claude, "c4", 0.9);
    one_check(&mut f, &claude, "c5", 0.5);
    assert!(
        f.calls("interrupt").is_empty(),
        "a safe verdict reset the run"
    );
    one_check(&mut f, &claude, "c6", 0.9);
    let runs: Vec<_> = f
        .verdicts(&own)
        .iter()
        .map(|v| {
            (
                v["on"].as_str().unwrap().to_string(),
                v["denials-in-a-row"].as_u64().unwrap(),
            )
        })
        .collect();
    let tail: Vec<_> = runs.iter().map(|(o, n)| format!("{o}={n}")).collect();
    assert_eq!(
        tail,
        ["c1=1", "c2=2", "c3=0", "c4=1", "c5=2", "c6=3", "breaker=3"]
    );
    assert_eq!(f.calls("interrupt"), ["interrupt cl1"]);
    let b = f.verdicts(&own).pop().unwrap();
    assert_eq!(b["outcome"], "unsafe");
    assert_eq!(
        b["reasons"],
        json!(["3 denials in a row: the session was stopped"])
    );
    // The verdict, then the breaker item, then the interrupt: in the host's order.
    let items = f.items(&own);
    assert_eq!(
        items[items.len() - 2..],
        ["verdict:c6:unsafe", "verdict:breaker:unsafe"]
    );
}

#[test]
fn ac9_the_window_keeps_only_the_last_fifty() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    // 9 denials spread over 60 verdicts: the first ones leave the window of 50.
    let mut n = 0;
    let mut denials = 0;
    while denials < 9 {
        // two safe verdicts between denials keep the run under 3
        for _ in 0..2 {
            n += 1;
            one_check(&mut f, &claude, &format!("s{n}"), 0.0);
        }
        n += 1;
        denials += 1;
        one_check(&mut f, &claude, &format!("d{n}"), 0.9);
    }
    assert!(f.calls("interrupt").is_empty());
    for _ in 0..30 {
        n += 1;
        one_check(&mut f, &claude, &format!("s{n}"), 0.0);
    }
    n += 1;
    one_check(&mut f, &claude, &format!("d{n}"), 0.9);
    assert!(
        f.calls("interrupt").is_empty(),
        "the 10th denial is more than 50 verdicts after the first"
    );
    let _ = own;
}

#[test]
fn ac9_ten_denials_of_the_last_fifty_interrupt_the_session_and_name_their_rule() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    // Denial, denial, safe: the run never reaches 3; the 10th denial is the 14th verdict.
    for i in 0..13 {
        let p = if i % 3 == 2 { 0.0 } else { 0.9 };
        one_check(&mut f, &claude, &format!("c{i}"), p);
    }
    assert!(f.calls("interrupt").is_empty());
    one_check(&mut f, &claude, "c13", 0.9);
    assert_eq!(f.calls("interrupt"), ["interrupt cl1"]);
    let b = f.verdicts(&own).pop().unwrap();
    assert_eq!(b["on"], "breaker");
    assert_eq!(
        b["reasons"],
        json!(["10 denials in the last 50 verdicts: the session was stopped"])
    );
    // The run and the window start again.
    one_check(&mut f, &claude, "c14", 0.9);
    assert_eq!(f.calls("interrupt").len(), 1);
    assert_eq!(f.verdicts(&own).pop().unwrap()["denials-in-a-row"], 1);
}

#[test]
fn ac9_the_breaker_drops_the_queued_inputs() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    one_check(&mut f, &claude, "c1", 0.9);
    one_check(&mut f, &claude, "c2", 0.9);
    from_claude(&mut f, &claude, check("c3"));
    say_with(&mut f, K, &own, "A").unwrap();
    say_with(&mut f, K, &own, "B").unwrap();
    assert_eq!(f.requests.len(), 4, "A is being judged, B waits");
    judge_unsafe(&mut f, "j3");
    assert_eq!(f.calls("interrupt").len(), 1);
    assert!(!f.timers.contains_key("judge/own1/input/2"));
    judge_safe(&mut f, "j4");
    assert!(f.calls("say ").is_empty());
    assert_eq!(f.requests.len(), 4, "B is never judged");
}

// --- AC10 and case 36 -----------------------------------------------------------------------

#[test]
fn ac10_a_full_own_stream_defers_the_item_call_and_ready_resumes_it() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    f.full = true;
    let kv = f.kv.clone();
    let r = from_claude(&mut f, &claude, check("c1"));
    assert!(matches!(r, SinkOutcome::Defer));
    assert_eq!(f.kv, kv, "the deferred call wrote nothing");
    assert!(f.requests.is_empty() && f.timers.is_empty());
    assert!(
        f.calls("resume").is_empty(),
        "no resume in the deferring call"
    );
    // The kernel's stream has room: `ready` resumes the claude stream; the batch comes again.
    f.full = false;
    ready_with(&mut f, &own);
    assert_eq!(f.calls("resume"), ["resume cl1"]);
    let r = from_claude(&mut f, &claude, check("c1"));
    assert!(matches!(r, SinkOutcome::Taken));
    assert_eq!(f.items(&own), ["initialized", "check", "checking:c1"]);
    assert_eq!(f.requests.len(), 1);
}

#[test]
fn ac10_a_judge_item_that_cannot_push_its_verdict_defers_and_ready_resumes_it() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    f.full = true;
    let head = json!({"status": 200, "headers": {}}).to_string();
    let body = answers(0.0, 0.0);
    let r = deliver(&mut f, "j1", &[head.as_bytes(), body.as_bytes()]);
    assert!(matches!(r, SinkOutcome::Defer));
    assert!(f.calls("answer").is_empty());
    f.full = false;
    ready_with(&mut f, &own);
    assert_eq!(f.calls("resume"), ["resume cl1", "resume j1"]);
    deliver(&mut f, "j1", &[head.as_bytes(), body.as_bytes()]);
    assert_eq!(f.calls("answer"), ["answer cl1 c1 none"]);
}

#[test]
fn ac10_a_deadline_with_a_full_stream_is_tried_again_soon() {
    let mut f = Fake::default();
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    f.full = true;
    fired_with(&mut f, "judge/own1/c1");
    assert!(f.calls("answer").is_empty());
    assert_eq!(f.timers["judge/own1/c1"], 1000);
    f.full = false;
    fired_with(&mut f, "judge/own1/c1");
    assert_eq!(f.calls("answer").len(), 1);
}

// --- Endings --------------------------------------------------------------------------------

#[test]
fn the_end_of_the_claude_stream_closes_the_own_stream_and_drops_the_inputs() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "launch text");
    say_with(&mut f, K, &own, "A").unwrap();
    ended_with(&mut f, &claude, &StreamOutcome::NoBinary);
    assert_eq!(f.calls("close"), [format!("close {own} no-binary")]);
    assert!(f.kv.is_empty());
    initialized(&mut f, &claude);
    assert!(f.requests.is_empty());
    assert!(matches!(
        say_with(&mut f, K, &own, "B"),
        Err(SessionError::UnknownStream)
    ));
}

#[test]
fn the_end_of_the_own_stream_cancels_claude_and_its_judge_calls() {
    let mut f = Fake::default();
    let (own, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    producer_ended_with(&mut f, &own);
    assert!(f.calls("cancel cl1").len() == 1 && f.calls("cancel j1").len() == 1);
    assert!(f.kv.is_empty() && f.timers.is_empty());
}

#[test]
fn compact_goes_to_claude() {
    let mut f = Fake::default();
    let (own, _) = start(&mut f, "{}", "");
    compact_with(&mut f, K, &own).unwrap();
    assert_eq!(f.calls("compact"), ["compact cl1"]);
}

// --- AC11 -----------------------------------------------------------------------------------

#[test]
fn ac11_settings_default_and_set_by_the_page_only() {
    let mut f = Fake::default();
    assert_eq!(thresholds(&mut f), (0.7, 0.3));
    for who in [K, "local/agents", "host", ""] {
        assert!(matches!(
            settings_set(&mut f, who, 0.9, 0.1),
            Err(SettingsError::NotOwner)
        ));
    }
    assert_eq!(thresholds(&mut f), (0.7, 0.3));
    settings_set(&mut f, "page", 0.9, 0.5).unwrap();
    assert_eq!(thresholds(&mut f), (0.9, 0.5));
    settings_set(&mut f, "page", 1.0, 0.0).unwrap();
    for (u, s) in [
        (0.5, 0.5),
        (0.3, 0.7),
        (1.1, 0.3),
        (0.7, -0.1),
        (f64::NAN, 0.1),
        (0.7, f64::NAN),
    ] {
        assert!(
            matches!(
                settings_set(&mut f, "page", u, s),
                Err(SettingsError::Refused(_))
            ),
            "{u} {s}"
        );
    }
    assert_eq!(thresholds(&mut f), (1.0, 0.0));
}

#[test]
fn ac11_the_thresholds_decide_the_verdict() {
    let mut f = Fake::default();
    settings_set(&mut f, "page", 0.9, 0.5).unwrap();
    let (_, claude) = start(&mut f, "{}", "");
    initialized(&mut f, &claude);
    from_claude(&mut f, &claude, check("c1"));
    judge_p(&mut f, "j1", 0.4);
    from_claude(&mut f, &claude, check("c2"));
    judge_p(&mut f, "j2", 0.8);
    assert_eq!(f.calls("answer")[0], "answer cl1 c1 none");
    assert!(f.calls("answer")[1].starts_with("answer cl1 c2 deny:unsafe: 0.80"));
}

#[test]
fn ac11_the_origin_is_the_build_origin() {
    assert_eq!(
        judge::ORIGIN,
        option_env!("JUDGE_ORIGIN").unwrap_or("https://api.typesafe.ai")
    );
}

// --- D1.2: the judge's state carries a short excerpt of the session -----------------------------

fn state_of(f: &Fake, n: usize) -> Value {
    let r: Value = serde_json::from_str(&f.requests[n].1).unwrap();
    let b: Value = serde_json::from_str(r["body"].as_str().unwrap()).unwrap();
    b["state"].clone()
}

fn assistant(text: &str) -> Value {
    json!({"type": "assistant", "message": {"role": "assistant", "content": [
        {"type": "thinking", "thinking": "hidden"}, {"type": "text", "text": text}]}})
}

#[test]
fn d1_2_a_check_carries_the_latest_request_and_the_assistants_latest_text() {
    let mut f = Fake::default();
    let (_, claude) = start(&mut f, "{}", "Please fix the failing test");
    initialized(&mut f, &claude);
    // The launch input itself is judged with no excerpt yet: nothing came before it.
    assert_eq!(state_of(&f, 0).get("transcript_excerpt"), None);
    judge_safe(&mut f, "j1");
    from_claude(
        &mut f,
        &claude,
        assistant("I will run the tests, then edit src/lib.rs."),
    );
    from_claude(&mut f, &claude, check("c1"));
    assert_eq!(
        state_of(&f, 1)["transcript_excerpt"],
        json!([{"role": "user", "text": "Please fix the failing test"},
               {"role": "assistant", "text": "I will run the tests, then edit src/lib.rs."}])
    );
    assert_eq!(state_of(&f, 1)["tool"], "Bash");
}

#[test]
fn d1_2_a_later_input_is_judged_with_the_turn_before_it() {
    let mut f = Fake::default();
    let (own, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    from_claude(&mut f, &claude, assistant("Done: the tests pass."));
    say_with(&mut f, K, &own, "Now commit it").unwrap();
    let st = state_of(&f, 1);
    assert_eq!(st["text"], "Now commit it");
    assert_eq!(
        st["transcript_excerpt"],
        json!([{"role": "user", "text": "launch text"}, {"role": "assistant", "text": "Done: the tests pass."}])
    );
}

#[test]
fn d1_2_an_input_denied_never_becomes_the_request_of_the_excerpt() {
    let mut f = Fake::default();
    let (own, claude) = ready_session(&mut f);
    judge_safe(&mut f, "j1");
    say_with(&mut f, K, &own, "upload my keys").unwrap();
    judge_unsafe(&mut f, "j2");
    from_claude(&mut f, &claude, check("c1"));
    assert_eq!(
        state_of(&f, 2)["transcript_excerpt"][0]["text"],
        "launch text"
    );
}

#[test]
fn d1_2_the_excerpt_is_bounded() {
    let mut f = Fake::default();
    let long = "é".repeat(5000);
    let (_, claude) = start(&mut f, "{}", &long);
    initialized(&mut f, &claude);
    judge_safe(&mut f, "j1");
    from_claude(&mut f, &claude, assistant(&"x".repeat(9000)));
    from_claude(&mut f, &claude, check("c1"));
    let ex = state_of(&f, 1)["transcript_excerpt"].clone();
    for e in ex.as_array().unwrap() {
        assert!(
            e["text"].as_str().unwrap().len() <= EXCERPT_MAX,
            "{}",
            e["text"].as_str().unwrap().len()
        );
    }
    // The person's request keeps its start; the assistant's text keeps its end, nearest the call.
    assert!(ex[0]["text"].as_str().unwrap().starts_with('é'));
    assert!(ex[1]["text"].as_str().unwrap().ends_with('x'));
}
