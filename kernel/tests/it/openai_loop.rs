//! S40-3: the OpenAI loop (TECHNICAL 9.5): requests through `https-stream`, the reply, errors,
//! pauses, slots, interrupt, restart.

use agent_kernel::active::Slots;
use agent_kernel::openai::answer::backoff_ms;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str =
    r#"{"backend":"openai","model":"gpt-5","system-prompt":{"replace":"You are Alice's helper."}}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const URL: &str = "https://api.openai.com/v1/responses";

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/o", P.as_bytes().to_vec()).unwrap();
    for who in ["alice", "bob"] {
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
        ..FakeHost::default()
    };
    Kernel::new(st, host)
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

/// The id of the request stream the loop of `id` waits on.
fn stream_of(k: &mut K, id: &str) -> String {
    lp(k, id)["stream"].as_str().unwrap().to_string()
}

fn head(status: u16, ct: &str) -> Vec<u8> {
    json!({"status": status, "headers": {"content-type": ct}})
        .to_string()
        .into_bytes()
}

fn sse_head() -> Vec<u8> {
    head(200, "text/event-stream")
}

fn delta(t: &str) -> Vec<u8> {
    json!({"type":"response.output_text.delta","item_id":"m","output_index":0,"content_index":0,"delta":t})
        .to_string()
        .into_bytes()
}

fn message(t: &str) -> Value {
    json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":t}]})
}

fn item_done(item: &Value) -> Vec<u8> {
    json!({"type":"response.output_item.done","output_index":0,"item":item})
        .to_string()
        .into_bytes()
}

fn completed(i: u64, o: u64, output: &[Value]) -> Vec<u8> {
    json!({"type":"response.completed","response":{"id":"r","usage":{"input_tokens":i,"output_tokens":o},"output":output}})
        .to_string()
        .into_bytes()
}

fn call(id: &str) -> Value {
    json!({"type":"function_call","id":"fc","call_id":id,"name":"tool","arguments":"{}"})
}

/// Delivers `items` numbered after the stream's last one.
fn feed(k: &mut K, stream: &str, items: Vec<Vec<u8>>) -> Called<SinkOutcome> {
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
    k.provider_lines(stream, batch).unwrap()
}

fn turn(k: &mut K, id: &str) -> Turn {
    rec(k, id).turn
}

fn idle(stop: Stop, tokens: Option<u64>) -> Turn {
    Turn::Idle {
        stop,
        seconds: Some(0),
        tools: None,
        tokens,
    }
}

fn started(k: &mut K, owner: &str, key: &str) -> String {
    k.start(owner, key, "o", "{}", IN).unwrap().value
}

/// A session whose first request is answered `text` and has ended.
fn answered(k: &mut K, owner: &str, key: &str, text: &str) -> String {
    let id = started(k, owner, key);
    let s = stream_of(k, &id);
    feed(
        k,
        &s,
        vec![
            sse_head(),
            item_done(&message(text)),
            completed(10, 5, &[message(text)]),
        ],
    );
    k.provider_ended_with(&s, None).unwrap();
    id
}

fn timer_after(k: &K, id: &str) -> Vec<u64> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::TimerAfter { id: i, delay_ms } if i == id => Some(*delay_ms),
            _ => None,
        })
        .collect()
}

#[test]
fn ac1_start_sends_one_request_and_no_provider() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let calls = starts(&k);
    assert_eq!(calls.len(), 1);
    let req = &calls[0];
    let keys: Vec<&str> = req
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        ["account", "body", "headers", "idle-s", "method", "url"]
    );
    assert_eq!(req["url"], URL);
    assert_eq!(req["method"], "POST");
    assert_eq!(
        req["headers"],
        json!({"content-type":"application/json","accept":"text/event-stream"})
    );
    assert_eq!(req["account"], "openai");
    assert_eq!(req["idle-s"], 90);
    let b = body_of(req);
    assert_eq!(b["model"], "gpt-5");
    assert_eq!(b["instructions"], "You are Alice's helper.");
    assert_eq!(
        b["input"],
        json!([{"type":"message","role":"user","content":[{"type":"input_text","text":"hi"}]}])
    );
    assert_eq!(b["store"], false);
    assert_eq!(b["stream"], true);
    assert_eq!(b["prompt_cache_key"], id.as_str());
    assert!(!k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::StartProvider { .. })));
    let stream = stream_of(&mut k, &id);
    let key: Value =
        serde_json::from_slice(&k.state.get(&format!("stream/{stream}")).unwrap()).unwrap();
    assert_eq!(key["kind"], "request");
    assert_eq!(key["session"], id.as_str());
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    assert_eq!(slots(&mut k).request, 1);
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
    assert_eq!(rec(&mut k, &id).streams, [stream]);
}

#[test]
fn ac1_a_host_that_refuses_the_request_refuses_the_start_and_writes_nothing() {
    let mut k = kernel();
    k.host.https_error = Some("destination".into());
    let before = k.state.len();
    let e = k.start("alice", "k", "o", "{}", IN).unwrap_err();
    assert_eq!(e, KernelError::Backend("destination".into()));
    assert_eq!(k.state.len(), before);
    assert!(starts(&k).is_empty());
}

#[test]
fn ac2_the_reply_streams_is_stored_and_ends_the_turn() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.attach("alice", &id, 0).unwrap();
    let s = stream_of(&mut k, &id);
    k.host.calls.clear();
    let reply = message("hello");
    let c = feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            delta("hel"),
            delta("lo"),
            item_done(&reply),
            completed(
                123,
                7,
                &[
                    json!({"type":"reasoning","id":"rs","encrypted_content":"e"}),
                    reply.clone(),
                ],
            ),
        ],
    );
    assert_eq!(c.value, SinkOutcome::Taken);
    // The deltas, the whole message and the usage reached the owner stream.
    let pushed: Vec<Value> = k
        .host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { value, .. } => Some(value.clone()),
            _ => None,
        })
        .collect();
    let chunks: Vec<&Value> = pushed
        .iter()
        .filter(|v| v["sessionUpdate"] == "agent_message_chunk")
        .collect();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0]["content"]["text"], "hel");
    assert_eq!(chunks[2]["content"]["text"], "hello");
    assert!(chunks[2]["_meta"]["watane"]["i"].is_u64());
    assert!(pushed.iter().any(|v| v["_meta"]["watane"]
        == json!({"type":"usage","context-used":123,"context-window":272000,"compact-at":200000})));
    assert_eq!(
        turn(&mut k, &id),
        idle(Stop::Completed, Some(130)),
        "tokens are the turn's input and output"
    );
    // The entry is stored, and the response's output is in the wire.
    let page = k.read("alice", &id, 0, None, 20).unwrap();
    assert!(page
        .entries
        .iter()
        .any(|l| l.contains(r#""kind":"message""#) && l.contains("hello")));
    let l = lp(&mut k, &id);
    assert_eq!(l["state"], "idle");
    assert_eq!(l["steps"], 1);
    assert_eq!(l["next"], 3);
    // The slot is freed at the stream's ended.
    assert_eq!(slots(&mut k).request, 1);
    k.provider_ended_with(&s, None).unwrap();
    assert_eq!(slots(&mut k).request, 0);
    assert!(k.state.get(&format!("stream/{s}")).is_none());
    assert!(rec(&mut k, &id).streams.iter().all(|x| x != &s));
}

#[test]
fn ac2b_consent_pending_shows_until_the_head_and_a_refusal_fails_the_turn() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.attach("alice", &id, 0).unwrap();
    let s = stream_of(&mut k, &id);
    k.host.calls.clear();
    let c = feed(
        &mut k,
        &s,
        vec![json!({"type":"consent-pending","what":"openai"})
            .to_string()
            .into_bytes()],
    );
    assert!(c
        .pushes
        .iter()
        .any(|p| p.value["state"]["activity"] == json!({"kind":"consent","what":"openai"})));
    // An attach during the wait shows it.
    let a = k.attach("alice", &id, 0).unwrap();
    let Attached::Live { .. } = a.value else {
        panic!("not live")
    };
    let shown = k.host.calls.iter().any(|c| match c {
        HostCall::Push { value, .. } => {
            value["_meta"]["watane"]["state"]["activity"]
                == json!({"kind":"consent","what":"openai"})
        }
        _ => false,
    });
    assert!(shown);
    let c = feed(&mut k, &s, vec![sse_head()]);
    assert!(c
        .pushes
        .iter()
        .any(|p| p.value["type"] == "turn" && p.value["state"].get("activity").is_none()));
    // A second request, refused by the participant.
    let id2 = started(&mut k, "bob", "k2");
    let s2 = stream_of(&mut k, &id2);
    feed(
        &mut k,
        &s2,
        vec![json!({"type":"consent-pending","what":"openai"})
            .to_string()
            .into_bytes()],
    );
    k.provider_ended_with(&s2, Some("consent-refused(openai)".into()))
        .unwrap();
    assert_eq!(
        turn(&mut k, &id2),
        idle(
            Stop::Failed {
                error: "consent-refused(openai)".into(),
                cause: None
            },
            None
        )
    );
}

#[test]
fn ac3_a_second_request_extends_the_first_and_keeps_its_instructions() {
    let mut k = kernel();
    let id = answered(&mut k, "alice", "k", "hello");
    k.say("alice", &id, "k2", r#"[{"type":"text","text":"more"}]"#)
        .unwrap();
    let calls = starts(&k);
    assert_eq!(calls.len(), 2);
    let (a, b) = (body_of(&calls[0]), body_of(&calls[1]));
    assert_eq!(a["instructions"], b["instructions"]);
    let (ia, ib) = (
        a["input"].as_array().unwrap(),
        b["input"].as_array().unwrap(),
    );
    assert_eq!(ia.len(), 1);
    assert_eq!(ib.len(), 3);
    assert_eq!(&ib[..1], &ia[..]);
    assert_eq!(ib[1]["role"], "assistant");
    assert_eq!(ib[2]["content"][0]["text"], "more");
    assert_eq!(lp(&mut k, &id)["instructions"], "You are Alice's helper.");
}

#[test]
fn ac3_an_overlay_appends_to_the_instructions_once() {
    let mut k = kernel();
    let id = k
        .start(
            "alice",
            "k",
            "o",
            r#"{"system-prompt-append":"Be brief."}"#,
            IN,
        )
        .unwrap()
        .value;
    assert_eq!(
        lp(&mut k, &id)["instructions"],
        "You are Alice's helper.\n\nBe brief."
    );
    assert_eq!(
        body_of(&starts(&k)[0])["instructions"],
        "You are Alice's helper.\n\nBe brief."
    );
}

#[test]
fn ac4_a_stream_that_closes_before_completing_fails_the_turn_and_stores_nothing() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), delta("par"), item_done(&message("partial"))],
    );
    k.provider_ended_with(&s, None).unwrap();
    assert_eq!(
        turn(&mut k, &id),
        idle(
            Stop::Failed {
                error: "the response ended before it completed".into(),
                cause: None
            },
            None
        )
    );
    let l = lp(&mut k, &id);
    assert_eq!(l["steps"], 0);
    assert_eq!(l["next"], 1, "only the input is in the wire");
    assert_eq!(slots(&mut k).request, 0);
}

fn usage_limit_body(secs: u64) -> Vec<u8> {
    json!({"error":{"type":"usage_limit_reached","code":"usage_limit_reached","message":"limit","plan_type":"plus","resets_in_seconds":secs}})
        .to_string()
        .into_bytes()
}

fn turn_changes(c: &Called<SinkOutcome>) -> Vec<&Push> {
    c.pushes
        .iter()
        .filter(|p| p.value["type"] == "turn")
        .collect()
}

#[test]
fn ac5_a_usage_limit_pauses_until_the_reset_and_resumes_by_itself() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    let c = feed(
        &mut k,
        &s,
        vec![head(429, "application/json"), usage_limit_body(60)],
    );
    let until = k.host.now + 60_000;
    assert_eq!(turn_changes(&c).len(), 1);
    assert_eq!(turn(&mut k, &id), Turn::Paused { until: Some(until) });
    assert_eq!(
        rec(&mut k, &id).deadlines,
        [Deadline {
            what: "resume".into(),
            at: until
        }]
    );
    assert_eq!(timer_after(&k, &format!("s/{id}")), [60_000]);
    assert_eq!(slots(&mut k).request, 0, "the stream's slot is free");
    assert_eq!(slots(&mut k).timers, 1);
    // `say` on the timed pause queues.
    k.say("alice", &id, "q", r#"[{"type":"text","text":"later"}]"#)
        .unwrap();
    assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
    // The timer fires: the request goes again, the same body.
    k.host.now = until;
    k.timer(&format!("s/{id}"));
    let calls = starts(&k);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["body"], calls[1]["body"]);
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(slots(&mut k).timers, 0);
    let s2 = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s2,
        vec![
            sse_head(),
            item_done(&message("ok")),
            completed(4, 2, &[message("ok")]),
        ],
    );
    // The queued input starts the next turn when this one ends.
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    assert_eq!(starts(&k).len(), 3);
    assert!(k.queued("alice", &id).unwrap().is_empty());
}

#[test]
fn ac5_a_response_failed_with_a_usage_limit_pauses_with_no_time_and_say_resumes() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    let failed = json!({"type":"response.failed","response":{"error":{"code":"usage_limit_reached","message":"limit"}}});
    let c = feed(
        &mut k,
        &s,
        vec![sse_head(), failed.to_string().into_bytes()],
    );
    assert_eq!(turn_changes(&c).len(), 1);
    assert_eq!(turn(&mut k, &id), Turn::Paused { until: None });
    assert!(rec(&mut k, &id).deadlines.is_empty());
    k.say("alice", &id, "k2", r#"[{"type":"text","text":"again"}]"#)
        .unwrap();
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    assert_eq!(starts(&k).len(), 2);
}

#[test]
fn ac6_retries_back_off_then_pause_with_the_error_kept() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let timer = format!("s/{id}");
    for attempt in 1..=6u32 {
        let s = stream_of(&mut k, &id);
        let at = k.host.now;
        if attempt % 2 == 1 {
            feed(&mut k, &s, vec![head(500, "application/json")]);
            k.provider_ended_with(&s, None).unwrap();
        } else {
            k.provider_ended_with(&s, Some("idle-timeout".into()))
                .unwrap();
        }
        let ms = backoff_ms(attempt).unwrap();
        assert_eq!(lp(&mut k, &id)["state"], "retrying");
        assert_eq!(lp(&mut k, &id)["attempt"], attempt);
        assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
        assert_eq!(
            rec(&mut k, &id).deadlines,
            [Deadline {
                what: "retry".into(),
                at: at + ms
            }]
        );
        assert_eq!(*timer_after(&k, &timer).last().unwrap(), ms);
        k.host.now = at + ms;
        k.timer(&timer);
        assert_eq!(starts(&k).len() as u32, attempt + 1);
        assert_eq!(lp(&mut k, &id)["state"], "requesting");
    }
    let s = stream_of(&mut k, &id);
    feed(&mut k, &s, vec![head(503, "application/json")]);
    k.provider_ended_with(&s, None).unwrap();
    assert_eq!(turn(&mut k, &id), Turn::Paused { until: None });
    let l = lp(&mut k, &id);
    assert_eq!(l["state"], "paused");
    assert_eq!(l["last-error"], "status 503");
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(slots(&mut k).timers, 0);
    k.say("alice", &id, "k2", r#"[{"type":"text","text":"again"}]"#)
        .unwrap();
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
}

#[test]
fn ac6_a_400_ends_the_turn_with_the_error() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    let body = json!({"error":{"message":"bad model"}})
        .to_string()
        .into_bytes();
    feed(&mut k, &s, vec![head(400, "application/json"), body]);
    assert_eq!(
        turn(&mut k, &id),
        idle(
            Stop::Failed {
                error: "status 400: bad model".into(),
                cause: None
            },
            None
        )
    );
    assert_eq!(slots(&mut k).request, 0);
}

#[test]
fn ac7_interrupt_cancels_the_request_frees_its_slot_and_may_start_the_next() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    k.host.calls.clear();
    k.interrupt("alice", &id, None).unwrap();
    assert!(k
        .host
        .calls
        .contains(&HostCall::CancelStream { stream: s.clone() }));
    assert_eq!(slots(&mut k).request, 0);
    assert_eq!(turn(&mut k, &id), idle(Stop::Interrupted, None));
    assert_eq!(lp(&mut k, &id)["state"], "idle");
    assert!(k.state.get(&format!("stream/{s}")).is_none());
    // The stream's later ended changes nothing.
    k.provider_ended_with(&s, Some("cancelled".into())).unwrap();
    assert_eq!(turn(&mut k, &id), idle(Stop::Interrupted, None));
    // With a replacement.
    let id2 = started(&mut k, "alice", "k2");
    let s2 = stream_of(&mut k, &id2);
    let before = starts(&k).len();
    k.interrupt("alice", &id2, Some(r#"[{"type":"text","text":"instead"}]"#))
        .unwrap();
    assert_eq!(starts(&k).len(), before + 1);
    assert!(k
        .host
        .calls
        .contains(&HostCall::CancelStream { stream: s2.clone() }));
    assert_ne!(stream_of(&mut k, &id2), s2);
    assert!(matches!(turn(&mut k, &id2), Turn::Running { .. }));
    assert_eq!(slots(&mut k).request, 1);
}

#[test]
fn ac7_cancel_ends_the_stream_and_delete_removes_the_loop_and_the_wire() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    k.host.calls.clear();
    k.cancel("alice", &id).unwrap();
    assert!(k
        .host
        .calls
        .contains(&HostCall::CancelStream { stream: s.clone() }));
    assert_eq!(slots(&mut k).request, 0);
    assert_eq!(turn(&mut k, &id), idle(Stop::Interrupted, None));
    assert_eq!(lp(&mut k, &id)["state"], "idle");
    // A session with a long wire, a live request and a retry.
    let id2 = answered(&mut k, "alice", "k2", &"x".repeat(100_000));
    k.say("alice", &id2, "k3", r#"[{"type":"text","text":"more"}]"#)
        .unwrap();
    let s2 = stream_of(&mut k, &id2);
    assert!(k.state.get(&format!("wire/{id2}/0")).is_some());
    k.host.calls.clear();
    k.delete("alice", &id2).unwrap();
    assert!(k
        .host
        .calls
        .contains(&HostCall::CancelStream { stream: s2 }));
    assert!(k.state.get(&format!("loop/{id2}")).is_none());
    let left: Vec<String> = k
        .state
        .entries()
        .map(|(a, _)| a.clone())
        .filter(|a| a.contains(&format!("/{id2}")) || a.starts_with(&format!("session/{id2}")))
        .collect();
    assert_eq!(left, Vec::<String>::new());
    assert_eq!(slots(&mut k).request, 0);
}

#[test]
fn ac8_requests_wait_in_a_fifo_and_an_owner_at_its_cap_never_blocks_another() {
    let mut k = kernel();
    k.state.put("access/carol", br#"["o"]"#.to_vec()).unwrap();
    let mut a = Vec::new();
    let mut b = Vec::new();
    for n in 0..4 {
        a.push(started(&mut k, "alice", &format!("a{n}")));
        b.push(started(&mut k, "bob", &format!("b{n}")));
    }
    assert_eq!(slots(&mut k).request, 8);
    assert_eq!(starts(&k).len(), 8);
    // The ninth waits.
    let c = started(&mut k, "carol", "c0");
    assert_eq!(starts(&k).len(), 8);
    assert_eq!(lp(&mut k, &c)["state"], "ready");
    assert_eq!(lp(&mut k, &c)["wait"], 0);
    let w: Value = serde_json::from_slice(&k.state.get("slots/wait/0").unwrap()).unwrap();
    assert_eq!(w, json!({"session": c, "boot": 1}));
    assert!(matches!(turn(&mut k, &c), Turn::Running { .. }));
    // A tenth, of alice, who is at her cap, waits behind it.
    let a5 = started(&mut k, "alice", "a4");
    assert_eq!(lp(&mut k, &a5)["state"], "ready");
    // Bob's request ends: the oldest waiting one that may go is sent in that call.
    let sb = stream_of(&mut k, &b[0]);
    feed(
        &mut k,
        &sb,
        vec![sse_head(), completed(1, 1, &[message("x")])],
    );
    k.provider_ended_with(&sb, None).unwrap();
    assert_eq!(starts(&k).len(), 9);
    assert_eq!(lp(&mut k, &c)["state"], "requesting");
    assert_eq!(lp(&mut k, &a5)["state"], "ready", "alice is at her cap");
    assert_eq!(slots(&mut k).request, 8);
    // One of alice's ends: her waiting session goes.
    let sa = stream_of(&mut k, &a[0]);
    feed(
        &mut k,
        &sa,
        vec![sse_head(), completed(1, 1, &[message("x")])],
    );
    k.provider_ended_with(&sa, None).unwrap();
    assert_eq!(starts(&k).len(), 10);
    assert_eq!(lp(&mut k, &a5)["state"], "requesting");
    assert!(k.state.get("slots/wait/1").is_none());
    // A waiting session that is interrupted is skipped when reached.
    let d = started(&mut k, "carol", "c1");
    assert_eq!(lp(&mut k, &d)["state"], "ready");
    k.interrupt("carol", &d, None).unwrap();
    let sb1 = stream_of(&mut k, &b[1]);
    k.provider_ended_with(&sb1, Some("idle-timeout".into()))
        .unwrap();
    assert_eq!(starts(&k).len(), 10);
    assert_eq!(lp(&mut k, &d)["state"], "idle");
}

#[test]
fn ac9_a_restart_sends_the_loop_again_or_waits_and_keeps_retries_and_pauses() {
    let mut k = kernel();
    let req = started(&mut k, "alice", "k1");
    let retry = started(&mut k, "alice", "k2");
    let paused = started(&mut k, "alice", "k3");
    // `retry`: a 500; `paused`: a usage limit.
    let s = stream_of(&mut k, &retry);
    feed(&mut k, &s, vec![head(500, "application/json")]);
    k.provider_ended_with(&s, None).unwrap();
    let s = stream_of(&mut k, &paused);
    feed(
        &mut k,
        &s,
        vec![head(429, "application/json"), usage_limit_body(600)],
    );
    let (rec_retry, rec_paused) = (rec(&mut k, &retry), rec(&mut k, &paused));
    let old = stream_of(&mut k, &req);
    let before = starts(&k).len();
    k.host.generation = 2;
    k.list("nobody", None, 1).unwrap();
    for _ in 0..4 {
        k.timer("restart");
    }
    assert_eq!(
        starts(&k).len(),
        before + 1,
        "only the requesting one is sent"
    );
    let l = lp(&mut k, &req);
    assert_eq!(l["state"], "requesting");
    assert_ne!(l["stream"], old.as_str());
    assert!(matches!(turn(&mut k, &req), Turn::Running { .. }));
    assert_eq!(slots(&mut k).request, 1);
    assert_eq!(lp(&mut k, &retry)["state"], "retrying");
    assert_eq!(rec(&mut k, &retry).deadlines, rec_retry.deadlines);
    assert_eq!(turn(&mut k, &paused), rec_paused.turn);
    assert_eq!(rec(&mut k, &paused).deadlines, rec_paused.deadlines);
    for id in [&req, &retry, &paused] {
        assert_ne!(turn(&mut k, id), Turn::Unknown);
    }
    // The request of the old boot is gone with its key.
    assert!(k.state.get(&format!("stream/{old}")).is_none());
}

#[test]
fn ac9_a_restart_with_no_slot_free_waits_in_the_fifo() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k1");
    k.host.generation = 2;
    // Another owner's sessions take every slot of the new boot first.
    let mut other = Vec::new();
    for n in 0..4 {
        other.push(started(&mut k, "bob", &format!("b{n}")));
    }
    k.state.put("access/carol", br#"["o"]"#.to_vec()).unwrap();
    for n in 0..4 {
        other.push(started(&mut k, "carol", &format!("c{n}")));
    }
    assert_eq!(slots(&mut k).request, 8);
    let r = k.read("alice", &id, 0, None, 1);
    assert!(r.is_ok());
    assert_eq!(lp(&mut k, &id)["state"], "ready");
    assert!(lp(&mut k, &id)["wait"].is_u64());
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
}

#[test]
fn ac10_a_body_over_the_budget_fails_the_turn_and_the_next_say_restarts_from_a_summary() {
    let mut k = kernel();
    let big = "y".repeat(250_000);
    let id = started(&mut k, "alice", "k");
    for n in 0..5 {
        let s = stream_of(&mut k, &id);
        let m = message(&format!("{n}:{big}"));
        feed(
            &mut k,
            &s,
            vec![sse_head(), item_done(&m), completed(1, 1, &[m])],
        );
        k.provider_ended_with(&s, None).unwrap();
        if n < 4 {
            k.say(
                "alice",
                &id,
                &format!("s{n}"),
                r#"[{"type":"text","text":"more"}]"#,
            )
            .unwrap();
        }
    }
    let before = starts(&k).len();
    k.say("alice", &id, "s9", r#"[{"type":"text","text":"and more"}]"#)
        .unwrap();
    assert_eq!(starts(&k).len(), before, "nothing is sent");
    let Turn::Idle {
        stop: Stop::Failed { error, .. },
        ..
    } = turn(&mut k, &id)
    else {
        panic!("not failed")
    };
    assert_eq!(error, "too-large");
    assert_eq!(slots(&mut k).request, 0);
    // The next say restarts from the text of the last assistant messages, newest kept.
    k.say(
        "alice",
        &id,
        "s10",
        r#"[{"type":"text","text":"start again"}]"#,
    )
    .unwrap();
    let calls = starts(&k);
    assert_eq!(calls.len(), before + 1);
    let b = body_of(calls.last().unwrap());
    let input = b["input"].as_array().unwrap();
    assert_eq!(input.len(), 2);
    let summary = input[0]["content"][0]["text"].as_str().unwrap();
    assert_eq!(summary.len(), 32 * 1024);
    assert!(summary.ends_with('y'));
    assert_eq!(input[1]["content"][0]["text"], "start again");
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
    assert!(lp(&mut k, &id).get("too-large").is_none());
}

#[test]
fn ac11_the_calls_of_a_response_are_answered_aborted_and_the_next_request_goes() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    let out = [message("calling"), call("call_1"), call("call_2")];
    feed(
        &mut k,
        &s,
        vec![sse_head(), item_done(&out[1]), completed(9, 3, &out)],
    );
    // The next request is sent in the same call, with the outputs after the calls.
    let calls = starts(&k);
    assert_eq!(calls.len(), 2);
    let input = body_of(&calls[1])["input"].clone();
    let input = input.as_array().unwrap();
    let types: Vec<&str> = input.iter().map(|i| i["type"].as_str().unwrap()).collect();
    assert_eq!(
        types,
        [
            "message",
            "message",
            "function_call",
            "function_call",
            "function_call_output",
            "function_call_output"
        ]
    );
    assert_eq!(input[4]["call_id"], "call_1");
    assert_eq!(input[4]["output"], "aborted");
    assert_eq!(input[5]["call_id"], "call_2");
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    assert_ne!(stream_of(&mut k, &id), s);
    // The old stream's end only frees its slot.
    k.provider_ended_with(&s, None).unwrap();
    assert_eq!(slots(&mut k).request, 1);
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
}

#[test]
fn the_loop_state_round_trips_with_its_wire() {
    use agent_kernel::openai::run::{LoopState, Phase};
    let ls = LoopState {
        phase: Phase::Requesting {
            stream: "t1".into(),
            attempt: 2,
        },
        model: "gpt-5".into(),
        instructions: "x".into(),
        last_error: Some("e".into()),
        ..LoopState::default()
    };
    let bytes = serde_json::to_vec(&ls).unwrap();
    let back: LoopState = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, ls);
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["state"], "requesting");
    assert_eq!(v["attempt"], 2);
    assert_eq!(v["wire-chunks"], json!([]));
}

#[test]
fn a_full_owner_stream_defers_a_batch_whole_and_it_is_taken_again() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let Attached::Live { stream: o } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("not live")
    };
    let s = stream_of(&mut k, &id);
    feed(&mut k, &s, vec![sse_head()]);
    k.host.full.insert(o.clone());
    let before = k.state.len();
    let r = k
        .provider_lines(
            &s,
            vec![
                (2, delta("a")),
                (3, item_done(&message("a"))),
                (4, completed(1, 1, &[message("a")])),
            ],
        )
        .unwrap();
    assert_eq!(r.value, SinkOutcome::Defer);
    assert_eq!(k.state.len(), before);
    assert_eq!(lp(&mut k, &id)["state"], "requesting");
    assert_eq!(starts(&k).len(), 1);
    k.host.full.clear();
    let r = k
        .provider_lines(
            &s,
            vec![
                (2, delta("a")),
                (3, item_done(&message("a"))),
                (4, completed(1, 1, &[message("a")])),
            ],
        )
        .unwrap();
    assert_eq!(r.value, SinkOutcome::Taken);
    assert_eq!(turn(&mut k, &id), idle(Stop::Completed, Some(2)));
}

#[test]
fn items_of_a_stream_that_is_no_longer_the_loops_are_ignored() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    k.interrupt("alice", &id, Some(r#"[{"type":"text","text":"instead"}]"#))
        .unwrap();
    // The cancelled stream's key is gone: its late items change nothing.
    let before = k.state.len();
    k.provider_lines(&s, vec![(1, sse_head()), (2, completed(1, 1, &[]))])
        .unwrap();
    assert_eq!(k.state.len(), before);
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
}

// ---- review r1

/// F1.4 (9.7 "Revoking", case 74): a `resume` deadline of a profile the owner may no longer use
/// leaves the turn `paused` with no time and sends nothing.
#[test]
fn r1_f1_4_a_timed_pause_whose_profile_was_revoked_stays_paused_with_no_time() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![head(429, "application/json"), usage_limit_body(60)],
    );
    let until = k.host.now + 60_000;
    assert_eq!(turn(&mut k, &id), Turn::Paused { until: Some(until) });
    k.state.put("access/alice", b"[]".to_vec()).unwrap();
    k.host.now = until;
    k.timer(&format!("s/{id}"));
    assert_eq!(turn(&mut k, &id), Turn::Paused { until: None });
    assert_eq!(starts(&k).len(), 1, "no request is sent");
    assert_eq!(lp(&mut k, &id)["state"], "paused");
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(slots(&mut k).timers, 0);
}

/// F1.3 (9.5 Items): a delta of a request that completed is not pushed into the next turn.
#[test]
fn r1_f1_3_a_late_delta_of_a_completed_request_is_ignored() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.attach("alice", &id, 0).unwrap();
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            item_done(&message("one")),
            completed(1, 1, &[message("one")]),
        ],
    );
    k.say("alice", &id, "q", r#"[{"type":"text","text":"two"}]"#)
        .unwrap();
    let s2 = stream_of(&mut k, &id);
    assert_ne!(s, s2);
    k.host.calls.clear();
    let c = feed(&mut k, &s, vec![delta("late")]);
    assert!(c.pushes.is_empty(), "{:?}", c.pushes);
    assert!(!k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::Push { .. })));
}

/// F1.2 (S40-2 `error_body`): a non-2xx body the host splits is classified over its items
/// joined.
#[test]
fn r1_f1_2_a_split_usage_limit_body_pauses_until_the_reset() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    let body = usage_limit_body(60);
    let (a, b) = body.split_at(body.len() / 2);
    feed(
        &mut k,
        &s,
        vec![head(429, "application/json"), a.to_vec(), b.to_vec()],
    );
    k.provider_ended_with(&s, None).unwrap();
    let until = k.host.now + 60_000;
    assert_eq!(turn(&mut k, &id), Turn::Paused { until: Some(until) });
    assert_eq!(slots(&mut k).request, 0);
}

/// F1.2: a split body is classified once its items joined read as JSON (else at `ended`).
#[test]
fn r1_f1_2_a_split_error_body_is_classified_once_its_joined_items_read() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    let body = br#"{"error":{"message":"bad model name"}}"#;
    let (a, b) = body.split_at(10);
    feed(&mut k, &s, vec![head(400, "application/json"), a.to_vec()]);
    assert!(matches!(turn(&mut k, &id), Turn::Running { .. }));
    feed(&mut k, &s, vec![b.to_vec()]);
    let page = k.read("alice", &id, 0, None, 20).unwrap();
    let t = turn(&mut k, &id);
    assert!(
        matches!(&t, Turn::Idle { stop: Stop::Failed { error, .. }, .. } if error.contains("bad model name")),
        "{t:?} {:?}",
        page.entries
    );
    assert_eq!(slots(&mut k).request, 0);
}

fn kinds(k: &mut K, id: &str) -> Vec<String> {
    k.read("alice", id, 0, None, 50)
        .unwrap()
        .entries
        .iter()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

/// F1.6 (9.5 "Compaction items"): a compaction the server returns is marked by one
/// `compaction` entry, whether it streamed or came only in `response.completed`.
#[test]
fn r1_f1_6_a_server_compaction_writes_one_compaction_entry() {
    let comp = json!({"type":"compaction","id":"cmp","encrypted_content":"c"});
    // Streamed as an item, then in the output.
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![
            sse_head(),
            item_done(&comp),
            item_done(&message("ok")),
            completed(1, 1, &[comp.clone(), message("ok")]),
        ],
    );
    let ks = kinds(&mut k, &id);
    assert_eq!(
        ks.iter().filter(|k| *k == "compaction").count(),
        1,
        "{ks:?}"
    );
    assert!(!ks.iter().any(|k| k == "unknown"), "{ks:?}");
    // Only in the output of `response.completed`.
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let s = stream_of(&mut k, &id);
    feed(
        &mut k,
        &s,
        vec![sse_head(), completed(1, 1, &[comp.clone(), message("ok")])],
    );
    let ks = kinds(&mut k, &id);
    assert_eq!(
        ks.iter().filter(|k| *k == "compaction").count(),
        1,
        "{ks:?}"
    );
}
