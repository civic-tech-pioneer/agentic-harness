//! S40-4: the OpenAI loop's tool calls (TECHNICAL 9.5 `waiting` and `tools`, 9.6, 6): the toolsets
//! opened at the first request, the broker's decisions, asks, jobs within the job slots, restart,
//! interrupt and cancel.

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn profile(extra: &str) -> String {
    format!(
        r#"{{"backend":"openai","model":"gpt-5","system-prompt":{{"replace":"x"}},"toolsets":["notes"]{extra}}}"#
    )
}

fn tool(name: &str, parallel: bool, idempotent: bool) -> Tool {
    Tool {
        name: name.into(),
        description: format!("does {name}"),
        input_schema: r#"{"type":"object","properties":{"q":{"type":"string"}}}"#.into(),
        parallel,
        idempotent,
    }
}

/// A kernel with the profile `o` and the tools the host describes.
fn kernel_with(profile_json: &str, tools: Vec<Tool>) -> K {
    let mut st = MemState::new();
    st.put("profile/o", profile_json.as_bytes().to_vec())
        .unwrap();
    for who in ["alice", "bob", "carol"] {
        st.put(&format!("access/{who}"), br#"["o"]"#.to_vec())
            .unwrap();
    }
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        generation: 1,
        now: 1_000_000,
        tools,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

/// `allow: ["notes_*"]` over the tools `notes_a` (parallel) and `notes_b`.
fn kernel() -> K {
    kernel_with(
        &profile(r#","allow":["notes_*"]"#),
        vec![tool("notes_a", true, true), tool("notes_b", true, false)],
    )
}

fn starts(k: &K) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::HttpsStart { request } => Some(serde_json::from_str(request).unwrap()),
            _ => None,
        })
        .collect()
}

fn body_of(req: &Value) -> Value {
    serde_json::from_str(req["body"].as_str().unwrap()).unwrap()
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn lp(k: &mut K, id: &str) -> Value {
    serde_json::from_slice(&k.state.get(&format!("loop/{id}")).unwrap()).unwrap()
}

fn slots(k: &mut K) -> Slots {
    Slots::load(&mut k.state)
}

fn stream_of(k: &mut K, id: &str) -> String {
    lp(k, id)["stream"].as_str().unwrap().to_string()
}

fn sse_head() -> Vec<u8> {
    json!({"status": 200, "headers": {"content-type": "text/event-stream"}})
        .to_string()
        .into_bytes()
}

fn item_done(item: &Value) -> Vec<u8> {
    json!({"type":"response.output_item.done","output_index":0,"item":item})
        .to_string()
        .into_bytes()
}

fn completed(output: &[Value]) -> Vec<u8> {
    json!({"type":"response.completed","response":{"id":"r","usage":{"input_tokens":10,"output_tokens":5},"output":output}})
        .to_string()
        .into_bytes()
}

fn call_of(id: &str, name: &str, args: &str) -> Value {
    json!({"type":"function_call","id":"fc","call_id":id,"name":name,"arguments":args})
}

fn feed(k: &mut K, stream: &str, items: Vec<Vec<u8>>) {
    let last = k
        .state
        .get(&format!("stream/{stream}"))
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["last"].as_u64())
        .unwrap_or(0);
    let batch = items
        .into_iter()
        .enumerate()
        .map(|(n, b)| (last + 1 + n as u64, b))
        .collect();
    k.provider_lines(stream, batch).unwrap();
}

/// Starts a session of `owner` on `o`, optionally attached, and answers its first request with
/// `calls`, as `output` of `response.completed`.
fn session_with_calls(k: &mut K, owner: &str, key: &str, calls: &[Value]) -> String {
    let id = k.start(owner, key, "o", "{}", IN).unwrap().value;
    let s = stream_of(k, &id);
    feed(k, &s, vec![sse_head(), completed(calls)]);
    k.provider_ended_with(&s, None).unwrap();
    id
}

fn attached(k: &mut K, owner: &str, id: &str) {
    k.attach(owner, id, 0).unwrap();
}

fn calls_of(k: &K) -> Vec<(String, String, String, String)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::ToolsCall {
                handle,
                key,
                tool,
                args,
            } => Some((handle.clone(), key.clone(), tool.clone(), args.clone())),
            _ => None,
        })
        .collect()
}

/// The streams of the session's running jobs, in call order, with their call ids.
fn jobs(k: &mut K, id: &str) -> Vec<(String, String)> {
    lp(k, id)["calls"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|c| c["st"] == "job")
        .map(|c| {
            (
                c["item"]["call_id"].as_str().unwrap().to_string(),
                c["stream"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn job_item(k: &mut K, stream: &str, v: Value) {
    let last = k
        .state
        .get(&format!("stream/{stream}"))
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["last"].as_u64())
        .unwrap_or(0);
    k.provider_lines(stream, vec![(last + 1, v.to_string().into_bytes())])
        .unwrap();
}

fn job_ended(k: &mut K, stream: &str) {
    k.provider_ended_with(stream, None).unwrap();
}

/// The job answers `v` and ends.
fn finish(k: &mut K, stream: &str, v: Value) {
    job_item(k, stream, json!({ "ok": v }));
    job_ended(k, stream);
}

fn outputs(req: &Value) -> Vec<(String, String)> {
    body_of(req)["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["type"] == "function_call_output")
        .map(|i| {
            (
                i["call_id"].as_str().unwrap().to_string(),
                i["output"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn turn(k: &mut K, id: &str) -> Turn {
    rec(k, id).turn
}

fn read_all(k: &mut K, owner: &str, id: &str) -> Vec<Value> {
    let page = k.read(owner, id, 0, None, 1000).unwrap();
    page.entries
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn entries(k: &mut K, owner: &str, id: &str, kind: &str) -> Vec<Value> {
    read_all(k, owner, id)
        .into_iter()
        .filter(|e| e["kind"] == kind)
        .collect()
}

// ---- AC1: the toolsets

#[test]
fn ac1_the_first_request_opens_and_describes_the_toolsets_and_lists_the_tools() {
    let mut k = kernel();
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    let order: Vec<&HostCall> = k
        .host
        .calls
        .iter()
        .filter(|c| {
            matches!(
                c,
                HostCall::ToolsOpen { .. }
                    | HostCall::ToolsDescribe { .. }
                    | HostCall::HttpsStart { .. }
            )
        })
        .collect();
    assert!(matches!(order[0], HostCall::ToolsOpen { toolsets } if toolsets == &["notes"]));
    assert!(matches!(order[1], HostCall::ToolsDescribe { handle } if handle == "h1"));
    assert!(matches!(order[2], HostCall::HttpsStart { .. }));
    assert_eq!(order.len(), 3);
    assert_eq!(rec(&mut k, &id).handle.as_deref(), Some("h1"));
    let b = body_of(&starts(&k)[0]);
    assert_eq!(
        b["tools"],
        json!([
            {"type":"function","name":"notes_a","description":"does notes_a",
             "parameters":{"type":"object","properties":{"q":{"type":"string"}}},"strict":false},
            {"type":"function","name":"notes_b","description":"does notes_b",
             "parameters":{"type":"object","properties":{"q":{"type":"string"}}},"strict":false},
        ])
    );
    let first = starts(&k)[0]["body"].as_str().unwrap().to_string();
    let tools_bytes = |body: &str| {
        let a = body.find("\"tools\":").unwrap();
        let z = body.find(",\"tool_choice\"").unwrap();
        body[a..z].to_string()
    };
    // The same bytes in every later request, and nothing opened again.
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), completed(&[call_of("c1", "notes_a", "{}")])],
    );
    let j = jobs(&mut k, &id);
    finish(&mut k, &j[0].1, json!(1));
    let all = starts(&k);
    assert_eq!(all.len(), 2);
    assert_eq!(
        tools_bytes(all[1]["body"].as_str().unwrap()),
        tools_bytes(&first)
    );
    let opens = k
        .host
        .calls
        .iter()
        .filter(|c| {
            matches!(
                c,
                HostCall::ToolsOpen { .. } | HostCall::ToolsDescribe { .. }
            )
        })
        .count();
    assert_eq!(opens, 2);
}

#[test]
fn ac1_a_profile_with_no_toolsets_opens_nothing_and_sends_no_tools() {
    let mut k = kernel_with(
        r#"{"backend":"openai","model":"gpt-5"}"#,
        vec![tool("notes_a", true, true)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    assert!(!k.host.calls.iter().any(|c| matches!(
        c,
        HostCall::ToolsOpen { .. } | HostCall::ToolsDescribe { .. }
    )));
    let b = body_of(&starts(&k)[0]);
    assert!(b.get("tools").is_none());
    assert_eq!(rec(&mut k, &id).handle, None);
}

#[test]
fn ac1_consent_pending_shows_the_consent_and_retries_with_the_backoff() {
    let mut k = kernel();
    k.host.tools_error = Some("consent-pending(Let Notes' agents call: notes?)".into());
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    assert!(starts(&k).is_empty());
    assert_eq!(lp(&mut k, &id)["state"], "retrying");
    assert_eq!(lp(&mut k, &id)["attempt"], 1);
    assert_eq!(lp(&mut k, &id)["consent"], "Let Notes' agents call: notes?");
    let r = rec(&mut k, &id);
    assert_eq!(r.deadlines.len(), 1);
    assert_eq!(r.deadlines[0].what, "retry");
    assert_eq!(r.deadlines[0].at, 1_000_000 + 1000);
    assert!(matches!(r.turn, Turn::Running { .. }));
    // The turn pushed to the attached page shows the consent as its activity, and a page that
    // attaches meanwhile sees it too.
    k.host.calls.clear();
    k.attach("alice", &id, 0).unwrap();
    let shown = k.host.calls.iter().any(|c| {
        matches!(c, HostCall::Push { value, .. }
            if value["_meta"]["watane"]["type"] == "turn"
                && value["_meta"]["watane"]["state"]["activity"]
                    == json!({"kind":"consent","what":"Let Notes' agents call: notes?"}))
    });
    assert!(shown);
    // The consent is held: the retry sends the request, the activity clears.
    k.host.tools_error = None;
    k.host.now += 1000;
    k.timer(&format!("s/{id}"));
    assert_eq!(starts(&k).len(), 1);
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
    assert!(lp(&mut k, &id).get("consent").is_none());
    assert_eq!(rec(&mut k, &id).handle.as_deref(), Some("h1"));
}

#[test]
fn ac1_a_consent_that_stays_pending_backs_off_as_a_request_does() {
    let mut k = kernel();
    k.host.tools_error = Some("consent-pending(x)".into());
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    for want in [1u64, 2, 4] {
        let r = rec(&mut k, &id);
        assert_eq!(r.deadlines[0].at - k.host.now, want * 1000);
        k.host.now = r.deadlines[0].at;
        k.timer(&format!("s/{id}"));
    }
    assert!(starts(&k).is_empty());
}

#[test]
fn ac1_a_consent_refused_or_not_granted_ends_the_turn_failed() {
    for e in [
        "consent-refused(Let Notes' agents call: notes?)",
        "not-granted",
    ] {
        let mut k = kernel();
        k.host.tools_error = Some(e.into());
        let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
        assert!(starts(&k).is_empty());
        match turn(&mut k, &id) {
            Turn::Idle {
                stop: Stop::Failed { error, .. },
                ..
            } => assert_eq!(error, e),
            t => panic!("{t:?}"),
        }
        assert_eq!(lp(&mut k, &id)["state"], "idle");
    }
}

// ---- AC2: the broker

#[test]
fn ac2_an_allowed_call_runs_at_once_a_denied_one_runs_nothing() {
    let mut k = kernel_with(
        &profile(r#","allow":["notes_*"],"deny":["notes_b"]"#),
        vec![tool("notes_a", true, true), tool("notes_b", true, false)],
    );
    let id = session_with_calls(
        &mut k,
        "alice",
        "k",
        &[
            call_of("c1", "notes_a", r#"{"q":"x"}"#),
            call_of("c2", "notes_b", "{}"),
        ],
    );
    let ran = calls_of(&k);
    assert_eq!(ran.len(), 1);
    assert_eq!(
        ran[0],
        (
            "h1".into(),
            format!("{id}/c1"),
            "notes_a".into(),
            r#"{"q":"x"}"#.into()
        )
    );
    assert_eq!(lp(&mut k, &id)["state"], "tools");
    let j = jobs(&mut k, &id);
    assert_eq!(j.len(), 1);
    finish(&mut k, &j[0].1, json!("done"));
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 2);
    assert_eq!(
        outputs(&reqs[1]),
        [
            (
                "c2".to_string(),
                r#"{"error":"denied by the profile"}"#.to_string()
            ),
            ("c1".to_string(), r#""done""#.to_string()),
        ]
    );
}

#[test]
fn ac2_a_call_no_rule_names_asks_the_owner() {
    let mut k = kernel_with(
        &profile(r#","allow":["notes_a"]"#),
        vec![tool("notes_a", true, true), tool("notes_b", true, false)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            completed(&[
                call_of("c1", "notes_a", "{}"),
                call_of("c2", "notes_b", r#"{"q":"y"}"#),
            ]),
        ],
    );
    // The allowed call waits for the step's asks.
    assert!(calls_of(&k).is_empty());
    assert_eq!(lp(&mut k, &id)["state"], "waiting");
    let r = rec(&mut k, &id);
    assert_eq!(
        r.turn,
        Turn::Waiting {
            asks: vec!["c2".into()],
            expires_at: Some(1_000_000 + 600_000)
        }
    );
    assert_eq!(r.open.len(), 1);
    assert_eq!(r.open[0].kind, "ask");
    assert_eq!(r.open[0].call, "c2");
    assert_eq!(r.open[0].expires_at, Some(1_600_000));
    assert!(r
        .deadlines
        .iter()
        .any(|d| d.what == "ask c2" && d.at == 1_600_000));
    let ask = &entries(&mut k, "alice", &id, "ask")[0];
    assert_eq!(ask["body"]["toolCallId"], "c2");
    assert_eq!(ask["body"]["input"], json!({"q":"y"}));
    assert_eq!(ask["body"]["expires-at"], 1_600_000);
    assert!(ask["body"].get("reach").is_none());
}

#[test]
fn ac2_the_profiles_ask_expiry_sets_the_deadline() {
    let mut k = kernel_with(
        &profile(r#","ask-expiry":30"#),
        vec![tool("notes_a", true, true)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), completed(&[call_of("c1", "notes_a", "{}")])],
    );
    assert_eq!(rec(&mut k, &id).open[0].expires_at, Some(1_030_000));
}

#[test]
fn ac2_dont_ask_denies_what_no_rule_allows() {
    let mut k = kernel_with(
        &profile(r#","mode":"dont-ask","allow":["notes_a"]"#),
        vec![tool("notes_a", true, true), tool("notes_b", true, false)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), completed(&[call_of("c2", "notes_b", "{}")])],
    );
    assert!(rec(&mut k, &id).open.is_empty());
    assert_eq!(
        outputs(&starts(&k)[1]),
        [(
            "c2".to_string(),
            r#"{"error":"denied by the profile"}"#.to_string()
        )]
    );
}

#[test]
fn ac2_no_live_owner_stream_denies_at_once() {
    let mut k = kernel_with(&profile(""), vec![tool("notes_a", true, true)]);
    let id = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "notes_a", "{}")]);
    assert!(rec(&mut k, &id).open.is_empty());
    assert_eq!(
        outputs(&starts(&k)[1]),
        [(
            "c1".to_string(),
            r#"{"error":"denied: no-owner"}"#.to_string()
        )]
    );
    let asks = entries(&mut k, "alice", &id, "ask");
    assert_eq!(asks.len(), 2);
    assert_eq!(
        asks[1]["body"]["decision"],
        json!({"outcome":"deny","by":"kernel","reason":"no-owner"})
    );
}

#[test]
fn ac2_an_agent_owner_is_denied_at_once() {
    let mut k = kernel_with(&profile(""), vec![tool("notes_a", true, true)]);
    k.state
        .put("access/agent:h1", br#"["o"]"#.to_vec())
        .unwrap();
    let id = session_with_calls(&mut k, "agent:h1", "k", &[call_of("c1", "notes_a", "{}")]);
    assert!(rec(&mut k, &id).open.is_empty());
    assert_eq!(
        outputs(&starts(&k)[1])[0].1,
        r#"{"error":"denied: no-owner"}"#
    );
}

#[test]
fn ac2_an_ask_expiry_of_zero_denies_at_once_as_expired() {
    let mut k = kernel_with(
        &profile(r#","ask-expiry":0"#),
        vec![tool("notes_a", true, true)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), completed(&[call_of("c1", "notes_a", "{}")])],
    );
    let r = rec(&mut k, &id);
    assert!(r.open.is_empty());
    assert!(r.deadlines.is_empty());
    assert_eq!(
        outputs(&starts(&k)[1])[0].1,
        r#"{"error":"denied: expired"}"#
    );
    let asks = entries(&mut k, "alice", &id, "ask");
    assert_eq!(asks[1]["body"]["decision"]["reason"], "expired");
    assert_eq!(asks[1]["body"]["decision"]["by"], "kernel");
}

// ---- AC3: answers and deadlines

/// A session with an ask on `c1` (notes_a) and an allowed `c2` (notes_b).
fn asking() -> (K, String) {
    let mut k = kernel_with(
        &profile(r#","allow":["notes_b"]"#),
        vec![tool("notes_a", true, true), tool("notes_b", true, false)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            completed(&[
                call_of("c1", "notes_a", r#"{"q":"z"}"#),
                call_of("c2", "notes_b", "{}"),
            ]),
        ],
    );
    (k, id)
}

#[test]
fn ac3_allow_runs_the_call_and_starts_the_step_s_jobs() {
    let (mut k, id) = asking();
    k.answer("alice", &id, "a1", "c1", r#"{"allow":true}"#)
        .unwrap();
    let ran = calls_of(&k);
    assert_eq!(
        ran.iter().map(|c| c.2.as_str()).collect::<Vec<_>>(),
        ["notes_a", "notes_b"]
    );
    assert_eq!(lp(&mut k, &id)["state"], "tools");
    let r = rec(&mut k, &id);
    assert!(r.open.is_empty());
    assert!(r.deadlines.is_empty());
    assert!(matches!(r.turn, Turn::Running { .. }));
    let asks = entries(&mut k, "alice", &id, "ask");
    assert_eq!(asks.len(), 2);
    assert_eq!(
        asks[1]["body"]["decision"],
        json!({"outcome":"allow","by":"owner"})
    );
}

#[test]
fn ac3_deny_gives_the_reason_as_the_output() {
    let (mut k, id) = asking();
    k.answer("alice", &id, "a1", "c1", r#"{"deny":true,"reason":"r"}"#)
        .unwrap();
    // Only the allowed call ran.
    assert_eq!(calls_of(&k).len(), 1);
    let j = jobs(&mut k, &id);
    finish(&mut k, &j[0].1, json!(7));
    assert_eq!(
        outputs(&starts(&k)[1]),
        [
            ("c1".to_string(), r#"{"error":"denied: r"}"#.to_string()),
            ("c2".to_string(), "7".to_string()),
        ]
    );
    let asks = entries(&mut k, "alice", &id, "ask");
    assert_eq!(
        asks[1]["body"]["decision"],
        json!({"outcome":"deny","by":"owner","reason":"r"})
    );
}

#[test]
fn ac3_a_session_allow_is_refused_and_a_second_answer_too() {
    let (mut k, id) = asking();
    let e = k
        .answer(
            "alice",
            &id,
            "a1",
            "c1",
            r#"{"allow":true,"for":"session"}"#,
        )
        .unwrap_err();
    match e {
        KernelError::Refused(r) => assert_eq!(r[0].field, "decision"),
        e => panic!("{e:?}"),
    }
    // The ask stays open.
    assert_eq!(rec(&mut k, &id).open.len(), 1);
    k.answer("alice", &id, "a2", "c1", r#"{"allow":true}"#)
        .unwrap();
    let e = k
        .answer("alice", &id, "a3", "c1", r#"{"deny":true}"#)
        .unwrap_err();
    match e {
        KernelError::Refused(r) => assert_eq!(r[0].field, "ask"),
        e => panic!("{e:?}"),
    }
}

#[test]
fn ac3_the_deadline_denies_the_ask_as_expired() {
    let (mut k, id) = asking();
    k.host.now += 600_000;
    k.timer(&format!("s/{id}"));
    assert!(rec(&mut k, &id).open.is_empty());
    assert_eq!(calls_of(&k).len(), 1);
    let j = jobs(&mut k, &id);
    finish(&mut k, &j[0].1, json!(1));
    assert_eq!(
        outputs(&starts(&k)[1])[0],
        (
            "c1".to_string(),
            r#"{"error":"denied: expired"}"#.to_string()
        )
    );
    let asks = entries(&mut k, "alice", &id, "ask");
    assert_eq!(
        asks[1]["body"]["decision"],
        json!({"outcome":"deny","by":"kernel","reason":"expired"})
    );
}

#[test]
fn ac3_an_answer_after_the_expiry_is_refused() {
    let (mut k, id) = asking();
    k.host.now += 600_000;
    k.timer(&format!("s/{id}"));
    assert!(k
        .answer("alice", &id, "a1", "c1", r#"{"allow":true}"#)
        .is_err());
}

#[test]
fn ac3_the_step_waits_for_every_ask() {
    let mut k = kernel_with(&profile(""), vec![tool("notes_a", true, true)]);
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            completed(&[
                call_of("c1", "notes_a", "{}"),
                call_of("c2", "notes_a", "{}"),
            ]),
        ],
    );
    assert!(matches!(
        turn(&mut k, &id),
        Turn::Waiting { ref asks, .. } if asks == &["c1", "c2"]
    ));
    k.answer("alice", &id, "a1", "c1", r#"{"allow":true}"#)
        .unwrap();
    assert!(calls_of(&k).is_empty());
    assert!(matches!(
        turn(&mut k, &id),
        Turn::Waiting { ref asks, .. } if asks == &["c2"]
    ));
    k.answer("alice", &id, "a2", "c2", r#"{"allow":true}"#)
        .unwrap();
    assert_eq!(calls_of(&k).len(), 2);
}

// ---- AC4: jobs

#[test]
fn ac4_a_job_s_item_becomes_the_output_and_the_next_request_goes() {
    let mut k = kernel();
    let id = session_with_calls(
        &mut k,
        "alice",
        "k",
        &[
            call_of("c1", "notes_a", "{}"),
            call_of("c2", "notes_b", "{}"),
        ],
    );
    attached(&mut k, "alice", &id);
    let j = jobs(&mut k, &id);
    assert_eq!(j.len(), 2);
    let key: Value =
        serde_json::from_slice(&k.state.get(&format!("stream/{}", j[0].1)).unwrap()).unwrap();
    assert_eq!(key["kind"], "job");
    // Calls are one `tools_call` each, under the session's key.
    assert_eq!(calls_of(&k)[0].1, format!("{id}/c1"));
    job_item(&mut k, &j[0].1, json!({"ok": {"n": 1}}));
    assert_eq!(starts(&k).len(), 1);
    job_item(
        &mut k,
        &j[1].1,
        json!({"err": {"code": "unknown-tool", "message": "no such tool"}}),
    );
    // Both outputs are in: the loop is `ready` and the request is queued by the call's end.
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 2);
    assert_eq!(
        outputs(&reqs[1]),
        [
            ("c1".to_string(), r#"{"n":1}"#.to_string()),
            (
                "c2".to_string(),
                r#"{"error":"unknown-tool: no such tool"}"#.to_string()
            ),
        ]
    );
    // The owner sees the tool calls complete and fail.
    let tools = entries(&mut k, "alice", &id, "tool");
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["body"]["toolCallId"], "c1");
    assert_eq!(tools[0]["body"]["status"], "completed");
    assert_eq!(tools[1]["body"]["status"], "failed");
    // The slots are freed by the jobs' ends.
    assert_eq!(slots(&mut k).job, 2);
    job_ended(&mut k, &j[0].1);
    job_ended(&mut k, &j[1].1);
    assert_eq!(slots(&mut k).job, 0);
    assert_eq!(rec(&mut k, &id).streams.len(), 2);
}

#[test]
fn ac4_a_job_that_ends_with_no_item_is_aborted() {
    let mut k = kernel();
    let id = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "notes_a", "{}")]);
    let j = jobs(&mut k, &id);
    job_ended(&mut k, &j[0].1);
    assert_eq!(
        outputs(&starts(&k)[1]),
        [("c1".to_string(), "aborted".to_string())]
    );
    assert_eq!(slots(&mut k).job, 0);
}

#[test]
fn ac4_the_turn_counts_its_calls() {
    let mut k = kernel();
    let id = session_with_calls(
        &mut k,
        "alice",
        "k",
        &[
            call_of("c1", "notes_a", "{}"),
            call_of("c2", "notes_a", "{}"),
        ],
    );
    for (_, s) in jobs(&mut k, &id) {
        finish(&mut k, &s, json!(1));
    }
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            completed(&[
                json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"ok"}]}),
            ]),
        ],
    );
    match turn(&mut k, &id) {
        Turn::Idle {
            stop: Stop::Completed,
            tools,
            ..
        } => assert_eq!(tools, Some(2)),
        t => panic!("{t:?}"),
    }
}

#[test]
fn ac4_a_tool_the_host_does_not_describe_is_not_run() {
    let mut k = kernel();
    let id = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "nope", "{}")]);
    assert!(calls_of(&k).is_empty());
    assert_eq!(
        outputs(&starts(&k)[1]),
        [("c1".to_string(), r#"{"error":"unknown tool"}"#.to_string())]
    );
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
}

// ---- AC5, AC6: slots

#[test]
fn ac5_thirty_three_parallel_calls_run_in_waves_of_eight() {
    let mut k = kernel();
    let calls: Vec<Value> = (1..=33)
        .map(|n| call_of(&format!("c{n}"), "notes_a", "{}"))
        .collect();
    let id = session_with_calls(&mut k, "alice", "k", &calls);
    assert_eq!(calls_of(&k).len(), 8);
    assert_eq!(slots(&mut k).job, 8);
    // Another session's model request still starts.
    let before = starts(&k).len();
    k.start("bob", "k", "o", "{}", IN).unwrap();
    assert_eq!(starts(&k).len(), before + 1);
    let mut open = jobs(&mut k, &id);
    let mut done = 0;
    let mut order = Vec::new();
    while !open.is_empty() {
        assert!(open.len() <= 8);
        // The jobs' ends free the slots; the item alone does not.
        let (call, stream) = open.remove(0);
        order.push(call);
        finish(&mut k, &stream, json!(done));
        done += 1;
        assert!(slots(&mut k).job <= 8);
        open = jobs(&mut k, &id);
        let running = calls_of(&k).len() - done;
        assert!(running <= 8, "{running} open");
    }
    assert_eq!(done, 33);
    // Started in call order.
    let started: Vec<String> = calls_of(&k).iter().map(|c| c.1.clone()).collect();
    let want: Vec<String> = (1..=33).map(|n| format!("{id}/c{n}")).collect();
    assert_eq!(started, want);
    let reqs: Vec<Value> = starts(&k)
        .into_iter()
        .filter(|r| body_of(r)["prompt_cache_key"] == id.as_str())
        .collect();
    assert_eq!(reqs.len(), 2);
    assert_eq!(outputs(&reqs[1]).len(), 33);
    assert_eq!(slots(&mut k).job, 0);
}

#[test]
fn ac5_a_tool_that_is_not_parallel_runs_the_calls_one_after_another() {
    let mut k = kernel_with(
        &profile(r#","allow":["notes_*"]"#),
        vec![tool("notes_a", true, true), tool("notes_b", false, false)],
    );
    let id = session_with_calls(
        &mut k,
        "alice",
        "k",
        &[
            call_of("c1", "notes_a", "{}"),
            call_of("c2", "notes_b", "{}"),
            call_of("c3", "notes_a", "{}"),
        ],
    );
    for n in 1..=3 {
        let j = jobs(&mut k, &id);
        assert_eq!(j.len(), 1, "one at a time");
        assert_eq!(j[0].0, format!("c{n}"));
        assert_eq!(calls_of(&k).len(), n);
        finish(&mut k, &j[0].1, json!(n));
    }
    assert_eq!(outputs(&starts(&k)[1]).len(), 3);
}

#[test]
fn ac6_one_owner_holds_at_most_eight_job_slots_another_owner_still_starts() {
    let mut k = kernel();
    let calls: Vec<Value> = (1..=8)
        .map(|n| call_of(&format!("c{n}"), "notes_a", "{}"))
        .collect();
    let a1 = session_with_calls(&mut k, "alice", "k1", &calls);
    let a2 = session_with_calls(&mut k, "alice", "k2", &calls);
    let b = session_with_calls(&mut k, "bob", "k1", &calls);
    let started = |k: &K, id: &str| {
        calls_of(k)
            .iter()
            .filter(|c| c.1.starts_with(&format!("{id}/")))
            .count()
    };
    assert_eq!(started(&k, &a1), 8);
    assert_eq!(started(&k, &a2), 0);
    assert_eq!(started(&k, &b), 8);
    assert_eq!(slots(&mut k).job, 16);
    // Alice's second session waits in the FIFO; a job of her first frees the slot for it.
    assert_eq!(lp(&mut k, &a2)["state"], "tools");
    assert!(lp(&mut k, &a2)["wait"].as_u64().is_some());
    let j = jobs(&mut k, &a1);
    finish(&mut k, &j[0].1, json!(1));
    assert_eq!(started(&k, &a2), 1);
    assert_eq!(slots(&mut k).job, 16);
}

#[test]
fn ac6_the_kernel_s_sixteen_job_slots_hold_for_a_third_owner() {
    let mut k = kernel();
    let calls: Vec<Value> = (1..=8)
        .map(|n| call_of(&format!("c{n}"), "notes_a", "{}"))
        .collect();
    let _a = session_with_calls(&mut k, "alice", "k", &calls);
    let _b = session_with_calls(&mut k, "bob", "k", &calls);
    let c = session_with_calls(&mut k, "carol", "k", &calls);
    assert_eq!(calls_of(&k).len(), 16);
    assert_eq!(jobs(&mut k, &c).len(), 0);
    let j = jobs(&mut k, &_a);
    finish(&mut k, &j[0].1, json!(1));
    assert_eq!(jobs(&mut k, &c).len(), 1);
    assert_eq!(slots(&mut k).job, 16);
}

// ---- AC7: restart

#[test]
fn ac7_a_restart_runs_idempotent_jobs_again_and_aborts_the_others() {
    let mut k = kernel();
    // notes_a is idempotent, notes_b is not.
    let id = session_with_calls(
        &mut k,
        "alice",
        "k",
        &[
            call_of("c1", "notes_a", "{}"),
            call_of("c2", "notes_b", "{}"),
        ],
    );
    assert_eq!(calls_of(&k).len(), 2);
    k.host.generation = 2;
    k.read("alice", &id, 0, None, 10).unwrap();
    let calls = calls_of(&k);
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[2].1, format!("{id}/c1"));
    assert_eq!(lp(&mut k, &id)["state"], "tools");
    let j = jobs(&mut k, &id);
    assert_eq!(j.len(), 1);
    assert_eq!(slots(&mut k).job, 1);
    finish(&mut k, &j[0].1, json!("again"));
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 2);
    assert_eq!(
        outputs(&reqs[1]),
        [
            ("c2".to_string(), "aborted".to_string()),
            ("c1".to_string(), r#""again""#.to_string()),
        ]
    );
}

#[test]
fn ac7_a_restart_with_nothing_open_goes_ready_and_pending_calls_start() {
    let mut k = kernel();
    let calls: Vec<Value> = (1..=10)
        .map(|n| call_of(&format!("c{n}"), "notes_b", "{}"))
        .collect();
    let id = session_with_calls(&mut k, "alice", "k", &calls);
    assert_eq!(calls_of(&k).len(), 8);
    k.host.generation = 2;
    k.read("alice", &id, 0, None, 10).unwrap();
    // Eight idempotent... notes_b is not: its eight open jobs are aborted, the two pending start.
    let started = calls_of(&k).len();
    assert_eq!(started, 10);
    assert_eq!(jobs(&mut k, &id).len(), 2);
    for (_, s) in jobs(&mut k, &id) {
        finish(&mut k, &s, json!(0));
    }
    let outs = outputs(&starts(&k)[1]);
    assert_eq!(outs.len(), 10);
    assert_eq!(outs.iter().filter(|o| o.1 == "aborted").count(), 8);
}

#[test]
fn ac7_a_restart_with_every_job_aborted_goes_ready_and_sends_the_request() {
    let mut k = kernel();
    let id = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "notes_b", "{}")]);
    k.host.generation = 2;
    k.read("alice", &id, 0, None, 10).unwrap();
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 2);
    assert_eq!(
        outputs(&reqs[1]),
        [("c1".to_string(), "aborted".to_string())]
    );
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
    assert_eq!(slots(&mut k).job, 0);
}

#[test]
fn ac7_a_waiting_session_is_unchanged_by_a_restart() {
    let (mut k, id) = asking();
    k.host.generation = 2;
    let r = k.read("alice", &id, 0, None, 10);
    assert!(r.is_ok());
    assert_eq!(lp(&mut k, &id)["state"], "waiting");
    let r = rec(&mut k, &id);
    assert_eq!(r.open.len(), 1);
    assert!(r.deadlines.iter().any(|d| d.what == "ask c1"));
    // The owner can still answer.
    k.answer("alice", &id, "a1", "c1", r#"{"allow":true}"#)
        .unwrap();
    assert_eq!(lp(&mut k, &id)["state"], "tools");
}

// ---- AC8: interrupt and cancel

fn running_session() -> (K, String) {
    let mut k = kernel();
    let id = session_with_calls(
        &mut k,
        "alice",
        "k",
        &[
            call_of("c1", "notes_a", "{}"),
            call_of("c2", "notes_a", "{}"),
        ],
    );
    attached(&mut k, "alice", &id);
    (k, id)
}

#[test]
fn ac8_interrupt_cancels_the_jobs_and_aborts_the_calls() {
    let (mut k, id) = running_session();
    let j = jobs(&mut k, &id);
    assert_eq!(slots(&mut k).job, 2);
    k.host.calls.clear();
    k.interrupt("alice", &id, None).unwrap();
    for (_, s) in &j {
        assert!(k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::CancelStream { stream } if stream == s)));
    }
    assert_eq!(slots(&mut k).job, 0);
    assert!(rec(&mut k, &id).streams.is_empty() || rec(&mut k, &id).streams.len() == 1);
    assert!(matches!(
        turn(&mut k, &id),
        Turn::Idle {
            stop: Stop::Interrupted,
            ..
        }
    ));
    assert_eq!(lp(&mut k, &id)["state"], "idle");
    // The handle stays; the calls were answered `aborted` in the wire and shown failed.
    assert!(!k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::ToolsClose { .. })));
    let tools = entries(&mut k, "alice", &id, "tool");
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().all(|t| t["body"]["status"] == "failed"));
    // The next input sends the history with the aborted outputs paired.
    k.say("alice", &id, "s1", IN).unwrap();
    let reqs = starts(&k);
    assert_eq!(
        outputs(reqs.last().unwrap())
            .iter()
            .map(|o| o.1.as_str())
            .collect::<Vec<_>>(),
        ["aborted", "aborted"]
    );
}

#[test]
fn ac8_interrupt_while_waiting_denies_the_asks_by_the_kernel() {
    let (mut k, id) = asking();
    k.interrupt("alice", &id, None).unwrap();
    let r = rec(&mut k, &id);
    assert!(r.open.is_empty());
    assert!(r.deadlines.is_empty());
    assert!(matches!(
        r.turn,
        Turn::Idle {
            stop: Stop::Interrupted,
            ..
        }
    ));
    let asks = entries(&mut k, "alice", &id, "ask");
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[1]["body"]["decision"]["by"], "kernel");
    assert_eq!(asks[1]["body"]["decision"]["outcome"], "deny");
    assert_eq!(lp(&mut k, &id)["state"], "idle");
}

#[test]
fn ac8_cancel_ends_jobs_closes_the_handle_and_delete_does_too() {
    let (mut k, id) = running_session();
    k.host.calls.clear();
    k.cancel("alice", &id).unwrap();
    assert_eq!(slots(&mut k).job, 0);
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::ToolsClose { handle } if handle == "h1")));
    assert!(matches!(
        turn(&mut k, &id),
        Turn::Idle {
            stop: Stop::Interrupted,
            ..
        }
    ));
    let (mut k, id) = running_session();
    k.host.calls.clear();
    k.delete("alice", &id).unwrap();
    assert_eq!(slots(&mut k).job, 0);
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::ToolsClose { handle } if handle == "h1")));
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::CancelStream { .. })));
}

// ---- AC9, AC10

#[test]
fn ac9_calls_come_from_the_output_item_done_events_when_output_is_empty() {
    // dev 18.3: `ToolCalls(3)` is three `output_item.done` events, then `Completed`.
    let mut k = kernel_with(
        &profile(r#","allow":["tool"]"#),
        vec![tool("tool", true, true)],
    );
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    let s = stream_of(&mut k, &id);
    let events: Vec<Vec<u8>> = (1..=3)
        .map(|n| {
            json!({"type":"response.output_item.done","output_index":n-1,"item":{
                "type":"function_call","id":format!("fc_{n}"),"call_id":format!("call_{n}"),
                "name":"tool","arguments":"{}","status":"completed"}})
            .to_string()
            .into_bytes()
        })
        .collect();
    let mut items = vec![sse_head()];
    items.extend(events);
    items.push(
        json!({"type":"response.completed","response":{"id":"resp_1","object":"response",
            "status":"completed","usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3}}})
        .to_string()
        .into_bytes(),
    );
    feed(&mut k, &s, items);
    assert_eq!(calls_of(&k).len(), 3);
    for (_, s) in jobs(&mut k, &id) {
        finish(&mut k, &s, json!(1));
    }
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 2);
    let input = body_of(&reqs[1])["input"].clone();
    let types: Vec<&str> = input
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        [
            "message",
            "function_call",
            "function_call",
            "function_call",
            "function_call_output",
            "function_call_output",
            "function_call_output"
        ]
    );
}

#[test]
fn ac9_a_non_empty_output_wins_over_the_streamed_items() {
    let mut k = kernel();
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            item_done(&call_of("x", "notes_a", "{}")),
            completed(&[call_of("c1", "notes_a", "{}")]),
        ],
    );
    let ran = calls_of(&k);
    assert_eq!(ran.len(), 1);
    assert_eq!(ran[0].1, format!("{id}/c1"));
}

#[test]
fn ac10_a_job_item_that_leaves_ready_queues_the_request_and_the_call_s_end_sends_it() {
    let mut k = kernel();
    let id = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "notes_a", "{}")]);
    let j = jobs(&mut k, &id);
    let tail = slots(&mut k).tail;
    job_item(&mut k, &j[0].1, json!({"ok": 1}));
    // The request waited in the FIFO inside the items call and was sent at its end.
    let s = slots(&mut k);
    assert_eq!(s.tail, tail + 1);
    assert_eq!(s.head, s.tail);
    assert_eq!(starts(&k).len(), 2);
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
}

#[test]
fn ac10_a_decided_ask_that_leaves_ready_sends_the_request_in_its_own_call() {
    let mut k = kernel_with(&profile(""), vec![tool("notes_a", true, true)]);
    let id = k.start("alice", "k", "o", "{}", IN).unwrap().value;
    attached(&mut k, "alice", &id);
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), completed(&[call_of("c1", "notes_a", "{}")])],
    );
    let tail = slots(&mut k).tail;
    k.answer("alice", &id, "a1", "c1", r#"{"deny":true,"reason":"no"}"#)
        .unwrap();
    // `answer` is no items call: the request starts at once.
    assert_eq!(slots(&mut k).tail, tail);
    assert_eq!(starts(&k).len(), 2);
    assert_eq!(
        outputs(&starts(&k)[1]),
        [("c1".to_string(), r#"{"error":"denied: no"}"#.to_string())]
    );
}

// ---- review r1

/// The `agent-tools` calls in the host log: each is made under the origin of the call that runs.
fn tools_effects(k: &K) -> Vec<HostCall> {
    k.host
        .calls
        .iter()
        .filter(|c| {
            matches!(
                c,
                HostCall::ToolsOpen { .. }
                    | HostCall::ToolsDescribe { .. }
                    | HostCall::ToolsCall { .. }
            )
        })
        .cloned()
        .collect()
}

// ---- review r2

/// F2.1 (the deputy's ruling, 9.5 "After a restart"): an open job of an idempotent tool is
/// aborted after a restart, like any other; no toolset is opened and no job is sent again, and
/// the turn goes on with the next request.
#[test]
fn review_restart_aborts_an_idempotent_job_and_opens_and_calls_nothing() {
    let mut k = kernel();
    let id = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "notes_a", "{}")]);
    assert_eq!(calls_of(&k).len(), 1);
    k.host.calls.clear();
    k.host.generation = 2;
    k.read("alice", &id, 0, None, 10).unwrap();
    assert_eq!(tools_effects(&k), [], "no ToolsOpen and no ToolsCall");
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 1);
    assert_eq!(
        outputs(&reqs[0]),
        [("c1".to_string(), "aborted".to_string())]
    );
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
    assert_eq!(slots(&mut k).job, 0);
}

/// F2.2: the restart pass steps the sessions of every owner under the origin of the call that
/// armed it; it makes no `agent-tools` call for them. Open jobs (idempotent ones included) and
/// pending calls are aborted, and each turn goes on with its next request.
#[test]
fn review_the_restart_pass_makes_no_tools_effect_under_another_owner() {
    let mut k = kernel();
    let a = session_with_calls(&mut k, "alice", "k", &[call_of("c1", "notes_a", "{}")]);
    // Ten calls: eight open jobs, two pending behind the session's job slots.
    let calls: Vec<Value> = (1..=10)
        .map(|n| call_of(&format!("c{n}"), "notes_a", "{}"))
        .collect();
    let b = session_with_calls(&mut k, "bob", "k", &calls);
    assert_eq!(calls_of(&k).len(), 9);
    k.host.calls.clear();
    // A call of another owner sees the new boot and arms the pass.
    k.host.generation = 2;
    k.list("carol", None, 1).unwrap();
    let mut ended = false;
    for _ in 0..20 {
        let before = k.host.calls.len();
        k.timer("restart");
        if k.host.calls[before..].contains(&HostCall::TimerCancel {
            id: "restart".into(),
        }) {
            ended = true;
            break;
        }
    }
    assert!(ended, "the pass ends");
    assert_eq!(tools_effects(&k), [], "no agent-tools call in the pass");
    for (id, n) in [(&a, 1), (&b, 10)] {
        assert_eq!(rec(&mut k, id).boot, 2);
        assert_eq!(lp(&mut k, id)["state"], "requesting");
        assert_eq!(lp(&mut k, id)["jobs"].as_u64().unwrap_or(0), 0);
    }
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 2);
    let mut outs: Vec<usize> = reqs.iter().map(|r| outputs(r).len()).collect();
    outs.sort();
    assert_eq!(outs, [1, 10]);
    assert!(reqs
        .iter()
        .flat_map(outputs)
        .all(|(_, o)| o == "aborted"));
    assert_eq!(slots(&mut k).job, 0);
}
