//! S134-2: every turn stops at its limits (TECHNICAL 9.7 "Limits" and "Usage", 9.5): the OpenAI
//! loop's step, tool-error and token caps with the one tool-free summary step, the Claude
//! `error_max_turns` stop, and the Claude context sample or clear.

use agent_kernel::active::Slots;
use agent_kernel::adapters::claude::ClaudeAdapter;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const MORE: &str = r#"[{"type":"text","text":"more"}]"#;

fn tool(name: &str) -> Tool {
    Tool {
        name: name.into(),
        description: format!("does {name}"),
        input_schema: r#"{"type":"object","properties":{}}"#.into(),
        parallel: true,
        idempotent: false,
    }
}

/// An OpenAI kernel whose profile `o` has `extra` fields (limits, deny...).
fn kernel(extra: &str) -> K {
    let profile = format!(
        r#"{{"backend":"openai","model":"gpt-5","system-prompt":{{"replace":"x"}},"toolsets":["notes"],"allow":["notes_a","notes_b"]{extra}}}"#
    );
    let mut st = MemState::new();
    st.put("profile/o", profile.into_bytes()).unwrap();
    st.put("access/alice", br#"["o"]"#.to_vec()).unwrap();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        generation: 1,
        now: 1_000_000,
        tools: vec![tool("notes_a"), tool("notes_b"), tool("notes_c")],
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn lp(k: &mut K, id: &str) -> Value {
    serde_json::from_slice(&k.state.get(&format!("loop/{id}")).unwrap()).unwrap()
}

fn stream_of(k: &mut K, id: &str) -> String {
    lp(k, id)["stream"].as_str().unwrap().to_string()
}

fn starts(k: &K) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::HttpsStart { request } => {
                let r: Value = serde_json::from_str(request).unwrap();
                Some(serde_json::from_str(r["body"].as_str().unwrap()).unwrap())
            }
            _ => None,
        })
        .collect()
}

fn ran(k: &K) -> Vec<String> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::ToolsCall { key, .. } => Some(key.rsplit('/').next().unwrap().to_string()),
            _ => None,
        })
        .collect()
}

fn head(status: u16, ct: &str) -> Vec<u8> {
    json!({"status": status, "headers": {"content-type": ct}})
        .to_string()
        .into_bytes()
}

fn sse_head() -> Vec<u8> {
    head(200, "text/event-stream")
}

fn call(id: &str, name: &str) -> Value {
    json!({"type":"function_call","id":"fc","call_id":id,"name":name,"arguments":"{}"})
}

fn text(t: &str) -> Value {
    json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":t}]})
}

/// A `response.completed` with `usage` as `(input, output)`, or none.
fn done(output: &[Value], usage: Option<(u64, u64)>) -> Vec<u8> {
    let mut r = json!({"id":"r","output":output});
    if let Some((i, o)) = usage {
        r["usage"] = json!({"input_tokens": i, "output_tokens": o});
    }
    json!({"type":"response.completed","response":r})
        .to_string()
        .into_bytes()
}

/// Answers the loop's current request with `output`, 10 + 5 tokens, and ends its stream.
fn reply(k: &mut K, id: &str, output: &[Value]) {
    reply_with(k, id, output, Some((10, 5)));
}

fn reply_with(k: &mut K, id: &str, output: &[Value], usage: Option<(u64, u64)>) {
    let s = stream_of(k, id);
    let last = |k: &mut K| {
        k.state
            .get(&format!("stream/{s}"))
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v["last"].as_u64())
            .unwrap_or(0)
    };
    let n = last(k);
    k.provider_lines(&s, vec![(n + 1, sse_head()), (n + 2, done(output, usage))])
        .unwrap();
    k.provider_ended_with(&s, None).unwrap();
}

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

fn job_end(k: &mut K, stream: &str, v: Value) {
    k.provider_lines(stream, vec![(1, v.to_string().into_bytes())])
        .unwrap();
    k.provider_ended_with(stream, None).unwrap();
}

fn ok(k: &mut K, id: &str, call_id: &str) {
    let j = jobs(k, id);
    let s = &j.iter().find(|(c, _)| c == call_id).unwrap().1;
    job_end(k, &s.clone(), json!({"ok": 1}));
}

fn err(k: &mut K, id: &str, call_id: &str) {
    let j = jobs(k, id);
    let s = &j.iter().find(|(c, _)| c == call_id).unwrap().1;
    job_end(
        k,
        &s.clone(),
        json!({"err": {"code": "boom", "message": "m"}}),
    );
}

fn start(k: &mut K) -> String {
    k.start("alice", "k", "o", "{}", IN).unwrap().value
}

fn stop_of(k: &mut K, id: &str) -> Stop {
    match rec(k, id).turn {
        Turn::Idle { stop, .. } => stop,
        t => panic!("not idle: {t:?}"),
    }
}

fn stopped(limit: &str) -> Stop {
    Stop::Stopped {
        limit: limit.into(),
    }
}

fn outputs(req: &Value) -> Vec<(String, String)> {
    req["input"]
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

// ---- AC1, AC2: max-turns and the summary step

#[test]
fn ac1_max_turns_aborts_the_calls_and_sends_one_tool_free_summary() {
    let mut k = kernel(r#","max-turns":2"#);
    let id = start(&mut k);
    reply(&mut k, &id, &[call("c1", "notes_a")]);
    ok(&mut k, &id, "c1");
    // Step 2 reaches the cap and calls a tool.
    assert_eq!(starts(&k).len(), 2);
    assert!(starts(&k)[1].get("tools").is_some());
    reply(&mut k, &id, &[call("c2", "notes_a")]);
    assert_eq!(ran(&k), ["c1"], "no job starts for the capped step");
    assert!(jobs(&mut k, &id).is_empty());
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 3, "one more request");
    assert!(reqs[2].get("tools").is_none(), "tool-free: {}", reqs[2]);
    assert!(outputs(&reqs[2]).contains(&("c2".into(), "aborted".into())));
    assert_eq!(lp(&mut k, &id)["summary"], true);
    assert!(rec(&mut k, &id).turn.is_active());
    // Its text completes the turn at the limit.
    reply(&mut k, &id, &[text("summary")]);
    assert_eq!(stop_of(&mut k, &id), stopped("max-turns"));
    assert_eq!(starts(&k).len(), 3);
    assert!(lp(&mut k, &id).get("summary").is_none());
}

#[test]
fn ac1_a_retry_of_the_summary_is_tool_free_again() {
    let mut k = kernel(r#","max-turns":1"#);
    let id = start(&mut k);
    reply(&mut k, &id, &[call("c1", "notes_a")]);
    assert_eq!(starts(&k).len(), 2);
    let s = stream_of(&mut k, &id);
    k.provider_lines(&s, vec![(1, head(503, "application/json"))])
        .unwrap();
    k.provider_ended_with(&s, None).unwrap();
    assert_eq!(lp(&mut k, &id)["state"], "retrying");
    k.host.now += 1_000;
    k.host.firing_origin = Some("alice".into());
    k.timer(&format!("s/{id}"));
    let reqs = starts(&k);
    assert_eq!(reqs.len(), 3);
    assert!(reqs[1].get("tools").is_none());
    assert!(reqs[2].get("tools").is_none(), "the retry is tool-free");
    assert_eq!(lp(&mut k, &id)["summary"], true);
}

#[test]
fn ac1_a_summary_that_calls_a_tool_runs_nothing() {
    let mut k = kernel(r#","max-turns":1"#);
    let id = start(&mut k);
    reply(&mut k, &id, &[call("c1", "notes_a")]);
    reply(&mut k, &id, &[call("c2", "notes_b")]);
    assert!(ran(&k).is_empty());
    assert_eq!(starts(&k).len(), 2, "no second summary");
    assert_eq!(stop_of(&mut k, &id), stopped("max-turns"));
}

#[test]
fn ac2_a_final_text_at_the_step_cap_completes_with_no_extra_request() {
    let mut k = kernel(r#","max-turns":1"#);
    let id = start(&mut k);
    reply(&mut k, &id, &[text("done")]);
    assert_eq!(stop_of(&mut k, &id), Stop::Completed);
    assert_eq!(starts(&k).len(), 1);
}

// ---- AC3: max-tool-errors

#[test]
fn ac3_two_failures_in_a_row_stop_and_abort_the_rest_without_counting() {
    let mut k = kernel(r#","max-tool-errors":2"#);
    let id = start(&mut k);
    reply(
        &mut k,
        &id,
        &[
            call("c1", "notes_a"),
            call("c2", "notes_a"),
            call("c3", "notes_b"),
        ],
    );
    err(&mut k, &id, "c1");
    assert_eq!(lp(&mut k, &id)["tool-errors-in-a-row"], 1);
    err(&mut k, &id, "c2");
    assert_eq!(stop_of(&mut k, &id), stopped("max-tool-errors"));
    assert_eq!(starts(&k).len(), 1, "no further request");
    assert!(lp(&mut k, &id).get("tool-errors-in-a-row").is_none());
    assert!(jobs(&mut k, &id).is_empty());
    // The open call got `aborted`: the next input's request carries it.
    k.say("alice", &id, "k2", MORE).unwrap();
    let outs = outputs(&starts(&k)[1]);
    assert!(outs.contains(&("c3".into(), "aborted".into())), "{outs:?}");
}

#[test]
fn ac3_a_profile_denial_counts_as_a_failure() {
    let mut k = kernel(r#","max-tool-errors":2,"deny":["notes_c"]"#);
    let id = start(&mut k);
    reply(&mut k, &id, &[call("c1", "notes_c"), call("c2", "notes_a")]);
    err(&mut k, &id, "c2");
    assert_eq!(stop_of(&mut k, &id), stopped("max-tool-errors"));
    assert_eq!(starts(&k).len(), 1);
}

#[test]
fn ac3_a_success_between_two_failures_resets_the_streak() {
    let mut k = kernel(r#","max-tool-errors":2"#);
    let id = start(&mut k);
    reply(&mut k, &id, &[call("c1", "notes_a")]);
    err(&mut k, &id, "c1");
    assert_eq!(lp(&mut k, &id)["tool-errors-in-a-row"], 1);
    reply(&mut k, &id, &[call("c2", "notes_a")]);
    ok(&mut k, &id, "c2");
    assert!(lp(&mut k, &id).get("tool-errors-in-a-row").is_none());
    reply(&mut k, &id, &[call("c3", "notes_a")]);
    err(&mut k, &id, "c3");
    assert!(rec(&mut k, &id).turn.is_active(), "the streak is 1 again");
    assert_eq!(starts(&k).len(), 4);
}

// ---- AC4, AC5: max-tokens

#[test]
fn ac4_a_response_over_the_token_cap_starts_no_job() {
    let mut k = kernel(r#","max-tokens":100"#);
    let id = start(&mut k);
    reply_with(&mut k, &id, &[call("c1", "notes_a")], Some((60, 50)));
    assert!(ran(&k).is_empty());
    assert_eq!(stop_of(&mut k, &id), stopped("max-tokens"));
    assert_eq!(starts(&k).len(), 1);
    k.say("alice", &id, "k2", MORE).unwrap();
    assert!(outputs(&starts(&k)[1]).contains(&("c1".into(), "aborted".into())));
}

#[test]
fn ac4_a_final_text_over_the_token_cap_stops_instead_of_completing() {
    let mut k = kernel(r#","max-tokens":100"#);
    let id = start(&mut k);
    reply_with(&mut k, &id, &[text("big")], Some((60, 50)));
    assert_eq!(stop_of(&mut k, &id), stopped("max-tokens"));
}

#[test]
fn ac4_the_token_cap_wins_over_the_step_cap() {
    let mut k = kernel(r#","max-tokens":100,"max-turns":1"#);
    let id = start(&mut k);
    reply_with(&mut k, &id, &[call("c1", "notes_a")], Some((60, 50)));
    assert_eq!(stop_of(&mut k, &id), stopped("max-tokens"));
    assert_eq!(starts(&k).len(), 1, "no summary step");
}

#[test]
fn ac4_tokens_add_up_over_the_steps_of_the_turn() {
    let mut k = kernel(r#","max-tokens":100"#);
    let id = start(&mut k);
    reply_with(&mut k, &id, &[call("c1", "notes_a")], Some((30, 20)));
    ok(&mut k, &id, "c1");
    assert_eq!(lp(&mut k, &id)["turn-tokens"], 50);
    reply_with(&mut k, &id, &[text("end")], Some((30, 30)));
    assert_eq!(stop_of(&mut k, &id), stopped("max-tokens"));
}

#[test]
fn ac5_a_token_cap_and_a_response_without_usage_fails_the_turn() {
    let mut k = kernel(r#","max-tokens":100"#);
    let id = start(&mut k);
    reply_with(&mut k, &id, &[call("c1", "notes_a")], None);
    assert!(ran(&k).is_empty());
    assert_eq!(
        stop_of(&mut k, &id),
        Stop::Failed {
            error: "usage-unavailable".into(),
            cause: None
        }
    );
}

#[test]
fn ac5_without_a_token_cap_a_response_without_usage_goes_on() {
    let mut k = kernel("");
    let id = start(&mut k);
    reply_with(&mut k, &id, &[call("c1", "notes_a")], None);
    assert_eq!(ran(&k), ["c1"]);
    assert!(rec(&mut k, &id).turn.is_active());
}

// ---- AC6: counters

#[test]
fn ac6_a_new_turn_gets_the_full_budget_again() {
    let mut k = kernel(r#","max-tokens":100"#);
    let id = start(&mut k);
    reply_with(&mut k, &id, &[text("big")], Some((60, 50)));
    assert_eq!(stop_of(&mut k, &id), stopped("max-tokens"));
    assert!(lp(&mut k, &id).get("turn-tokens").is_none());
    k.say("alice", &id, "k2", MORE).unwrap();
    reply_with(&mut k, &id, &[text("small")], Some((60, 30)));
    assert_eq!(stop_of(&mut k, &id), Stop::Completed);
}

#[test]
fn ac6_a_retry_and_an_ask_inside_the_turn_keep_the_counters() {
    // `notes_c` is not allowed: it asks.
    let mut k = kernel(r#","max-turns":9,"max-tool-errors":5"#);
    let id = start(&mut k);
    k.attach("alice", &id, 0).unwrap();
    reply(&mut k, &id, &[call("c1", "notes_a")]);
    err(&mut k, &id, "c1");
    let l = lp(&mut k, &id);
    assert_eq!(
        (l["turn-steps"].clone(), l["tool-errors-in-a-row"].clone()),
        (json!(1), json!(1))
    );
    // The second request fails once and is retried.
    let s = stream_of(&mut k, &id);
    k.provider_lines(&s, vec![(1, head(503, "application/json"))])
        .unwrap();
    k.provider_ended_with(&s, None).unwrap();
    assert_eq!(lp(&mut k, &id)["turn-steps"], 1);
    k.host.now += 1_000;
    k.host.firing_origin = Some("alice".into());
    k.timer(&format!("s/{id}"));
    // Then it asks.
    reply(&mut k, &id, &[call("c2", "notes_c")]);
    assert_eq!(lp(&mut k, &id)["state"], "waiting");
    let l = lp(&mut k, &id);
    assert_eq!(
        (l["turn-steps"].clone(), l["tool-errors-in-a-row"].clone()),
        (json!(2), json!(1))
    );
    k.answer("alice", &id, "a1", "c2", r#"{"allow":true}"#)
        .unwrap();
    let l = lp(&mut k, &id);
    assert_eq!(
        (l["turn-steps"].clone(), l["tool-errors-in-a-row"].clone()),
        (json!(2), json!(1))
    );
}

// ---- AC7, AC8: Claude

const CP: &str = r#"{"backend":"claude","model":"sonnet","tools":["Read"],"allow":["Read(**)"]}"#;
const IDLE: &str = r#"{"type":"system","subtype":"session_state_changed","state":"idle"}"#;

fn result(subtype: &str, extra: &str) -> String {
    format!(
        r#"{{"type":"result","subtype":"{subtype}","is_error":false,"duration_ms":1,"duration_api_ms":1,"num_turns":1,"session_id":"s","total_cost_usd":0,"usage":{{"input_tokens":1,"output_tokens":1}}{extra}}}"#
    )
}

const ONE_MODEL: &str = r#","model_usage_since_last_result":{"claude-sonnet":{"inputTokens":3,"outputTokens":52,"cacheReadInputTokens":14000,"cacheCreationInputTokens":120,"contextWindow":200000}}"#;

fn claude() -> (K, String, String, String) {
    let mut st = MemState::new();
    st.put("profile/p", CP.as_bytes().to_vec()).unwrap();
    st.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        generation: 1,
        now: 1000,
        ..FakeHost::default()
    };
    let mut k = Kernel::new(st, host);
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let t = rec(&mut k, &id).streams[0].clone();
    let Attached::Live { stream: o } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("not live");
    };
    (k, id, t, o)
}

fn pushes(k: &K, o: &str) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream, value } if stream == o => {
                Some(value["_meta"]["watane"].clone())
            }
            _ => None,
        })
        .filter(|w| w["type"] == "usage")
        .collect()
}

fn feed(k: &mut K, t: &str, from: u64, raw: &[&str]) {
    let items = raw
        .iter()
        .enumerate()
        .map(|(i, l)| (from + i as u64, l.as_bytes().to_vec()))
        .collect();
    k.provider_lines(t, items).unwrap();
}

#[test]
fn ac7_error_max_turns_then_idle_ends_the_turn_stopped_max_turns() {
    let (mut k, id, t, o) = claude();
    feed(&mut k, &t, 1, &[&result("error_max_turns", ""), IDLE]);
    assert_eq!(stop_of(&mut k, &id), stopped("max-turns"));
    let live: Vec<Value> = k
        .host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream, value } if *stream == o => {
                Some(value["_meta"]["watane"].clone())
            }
            _ => None,
        })
        .collect();
    assert!(live.iter().any(|w| w["type"] == "turn"
        && w["state"]["stop"] == json!({"kind":"stopped","limit":"max-turns"})));
    let page = k.read("alice", &id, 0, None, 256).unwrap();
    assert!(page
        .entries
        .iter()
        .any(|e| e.contains(r#""kind":"turn""#) && e.contains("max-turns")));
}

#[test]
fn ac7_a_success_result_still_completes() {
    let (mut k, id, t, _) = claude();
    feed(&mut k, &t, 1, &[&result("success", ""), IDLE]);
    assert_eq!(stop_of(&mut k, &id), Stop::Completed);
}

#[test]
fn ac7_the_late_result_of_an_interrupted_turn_does_not_end_the_replacement() {
    let (mut k, id, t, o) = claude();
    k.interrupt("alice", &id, Some(MORE)).unwrap();
    feed(
        &mut k,
        &t,
        1,
        &[&result("error_max_turns", ONE_MODEL), IDLE],
    );
    assert!(
        rec(&mut k, &id).turn.is_active(),
        "{:?}",
        rec(&mut k, &id).turn
    );
    assert!(pushes(&k, &o).is_empty(), "a late result gives no meter");
    feed(&mut k, &t, 3, &[&result("success", ""), IDLE]);
    assert_eq!(stop_of(&mut k, &id), Stop::Completed);
}

#[test]
fn ac8_one_step_of_one_model_pushes_the_context_sample() {
    let (mut k, id, t, o) = claude();
    feed(&mut k, &t, 1, &[&result("success", ONE_MODEL), IDLE]);
    assert_eq!(
        pushes(&k, &o),
        [json!({"type":"usage","context-used":14123,"context-window":200000})]
    );
    // A later attach replays the current sample after the queue item.
    let Attached::Live { stream: o2 } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("not live");
    };
    assert_eq!(
        pushes(&k, &o2),
        [json!({"type":"usage","context-used":14123,"context-window":200000})]
    );
}

#[test]
fn ac8_any_other_result_pushes_the_clear_and_attach_replays_nothing() {
    let clear = json!({"type":"usage","context-used":null,"context-window":null});
    let two = r#","model_usage_since_last_result":{"a":{"inputTokens":1,"contextWindow":10},"b":{"inputTokens":1,"contextWindow":10}}"#;
    let zero = r#","model_usage_since_last_result":{"a":{"inputTokens":1,"contextWindow":0}}"#;
    let multi = ONE_MODEL;
    for (case, line) in [
        ("none", result("success", "")),
        ("two models", result("success", two)),
        ("zero window", result("success", zero)),
        (
            "two steps",
            result("success", multi).replace(r#""num_turns":1"#, r#""num_turns":2"#),
        ),
    ] {
        let (mut k, id, t, o) = claude();
        feed(&mut k, &t, 1, &[&line, IDLE]);
        assert_eq!(pushes(&k, &o), vec![clear.clone()], "{case}");
        let Attached::Live { stream: o2 } = k.attach("alice", &id, 0).unwrap().value else {
            panic!("not live");
        };
        assert!(pushes(&k, &o2).is_empty(), "{case}");
    }
}

#[test]
fn ac8_a_later_turn_without_a_sample_clears_the_earlier_one() {
    let (mut k, id, t, o) = claude();
    feed(&mut k, &t, 1, &[&result("success", ONE_MODEL), IDLE]);
    k.say("alice", &id, "k2", MORE).unwrap();
    feed(&mut k, &t, 3, &[&result("success", ""), IDLE]);
    let p = pushes(&k, &o);
    assert_eq!(p.len(), 2);
    assert_eq!(p[1]["context-used"], Value::Null);
    let Attached::Live { stream: o2 } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("not live");
    };
    assert!(pushes(&k, &o2).is_empty());
}

#[test]
fn ac7_the_adapter_reports_the_limit_with_the_idle() {
    let mut a = ClaudeAdapter::new();
    let out = a.line(result("error_max_turns", "").as_bytes(), 1);
    assert!(!out[0].idle && out[0].limit.is_none());
    let out = a.line(IDLE.as_bytes(), 2);
    assert!(out[0].idle);
    assert_eq!(out[0].limit.as_deref(), Some("max-turns"));
    // The next turn starts clean.
    let out = a.line(result("success", "").as_bytes(), 3);
    assert!(out[0].limit.is_none());
}

/// S134 r1 F1.8: a loop saved by a build before stage F has no limits in its policy; when it is
/// next loaded the profile's caps are compiled in, so the turn in flight and later turns stop.
#[test]
fn r1_f1_8_a_loop_saved_before_stage_f_gets_the_profiles_limits() {
    let mut k = kernel(r#","max-tokens":100"#);
    let id = start(&mut k);
    // Rewrite the loop as an earlier build saved it: no caps, no marker.
    let key = format!("loop/{id}");
    let mut v: Value = serde_json::from_slice(&k.state.get(&key).unwrap()).unwrap();
    let o = v.as_object_mut().unwrap();
    o.remove("limits-compiled");
    let p = o["policy"].as_object_mut().expect("the policy");
    assert!(p.remove("max-tokens").is_some(), "the cap is in the policy");
    assert!(v.get("limits-compiled").is_none());
    k.state.put(&key, serde_json::to_vec(&v).unwrap()).unwrap();
    // The turn in flight stops at the cap.
    reply_with(&mut k, &id, &[text("big")], Some((60, 50)));
    assert_eq!(stop_of(&mut k, &id), stopped("max-tokens"));
    assert_eq!(lp(&mut k, &id)["limits-compiled"], true);
}
