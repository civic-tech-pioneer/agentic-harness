//! S127-3: a checked provider's `checking` and `verdict` items (TECHNICAL 9.3, 9.4, 9.8, 10).

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str =
    r#"{"backend":"claude","model":"sonnet","tools":["Read"],"allow":["Read(**)"],"partial":true}"#;
const IN: &str = r#"[{"type":"text","text":"hello"}]"#;
const ASK: &str = r#"{"type":"ask","id":"a1","tool_use_id":"tu1","tool":"Read","input":{"file_path":"/x"},"expires-at":9999}"#;

fn session(direct: bool) -> (K, String, String) {
    let mut st = MemState::default();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
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
        not_direct: !direct,
        ..FakeHost::default()
    };
    let mut k = Kernel::new(st, host);
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    k.attach("alice", &id, 0).unwrap();
    (k, id, t)
}

fn feed(k: &mut K, t: &str, item: &str) -> Called<SinkOutcome> {
    let last = k
        .state
        .get(&format!("stream/{t}"))
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["last"].as_u64())
        .unwrap_or(0);
    k.provider_lines(t, vec![(last + 1, item.as_bytes().to_vec())])
        .unwrap()
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn turn_states(c: &Called<SinkOutcome>) -> Vec<Value> {
    c.pushes
        .iter()
        .filter(|p| p.value["type"] == "turn")
        .map(|p| p.value["state"].clone())
        .collect()
}

#[test]
fn ac1_checking_is_the_live_activity_only() {
    let (mut k, id, t) = session(false);
    let before = entries(&mut k, &id).len();
    let mut rec0 = Record::load(&mut k.state, &id).unwrap();
    let c = feed(&mut k, &t, r#"{"type":"checking","on":"input/2"}"#);
    let s = turn_states(&c);
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!(s[0]["state"], "running");
    assert_eq!(s[0]["activity"], json!({"kind":"checking","on":"input"}));
    let c = feed(&mut k, &t, r#"{"type":"checking","on":"a1"}"#);
    assert_eq!(
        turn_states(&c)[0]["activity"],
        json!({"kind":"checking","on":"a1"})
    );
    assert_eq!(entries(&mut k, &id).len(), before);
    let mut rec1 = Record::load(&mut k.state, &id).unwrap();
    // the size accounting follows the stream cursor, not the item
    rec0.bytes = 0;
    rec1.bytes = 0;
    assert_eq!(rec1, rec0);
}

#[test]
fn ac1_checking_when_not_running_pushes_nothing() {
    let (mut k, _id, t) = session(false);
    feed(&mut k, &t, ASK);
    // the turn waits on the ask
    let c = feed(&mut k, &t, r#"{"type":"checking","on":"a1"}"#);
    assert!(turn_states(&c).is_empty());
}

#[test]
fn ac2_verdict_entry_with_the_call_of_an_open_ask() {
    let (mut k, id, t) = session(false);
    feed(&mut k, &t, ASK);
    feed(&mut k, &t, r#"{"type":"checking","on":"a1"}"#);
    let c = feed(
        &mut k,
        &t,
        r#"{"type":"verdict","on":"a1","outcome":"unsafe","reasons":["harm: 0.90"],"denials-in-a-row":2}"#,
    );
    let v: Vec<Value> = entries(&mut k, &id)
        .into_iter()
        .filter(|e| e["kind"] == "verdict")
        .collect();
    assert_eq!(v.len(), 1);
    assert_eq!(
        v[0]["body"],
        json!({"on":"a1","toolCallId":"tu1","outcome":"unsafe","reasons":["harm: 0.90"],"by":"checked","denials-in-a-row":2})
    );
    // an ask's verdict does not end the turn; a waiting turn gets no running push
    assert!(turn_states(&c).is_empty());
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Waiting { .. }
    ));
}

#[test]
fn ac2_verdict_on_input_and_breaker_have_no_call() {
    let (mut k, id, t) = session(false);
    feed(&mut k, &t, r#"{"type":"checking","on":"input/2"}"#);
    let c = feed(
        &mut k,
        &t,
        r#"{"type":"verdict","on":"input/2","outcome":"safe","reasons":[],"denials-in-a-row":0}"#,
    );
    let s = turn_states(&c);
    assert!(s
        .iter()
        .any(|x| x["state"] == "running" && x.get("activity").is_none()));
    feed(
        &mut k,
        &t,
        r#"{"type":"verdict","on":"breaker","outcome":"unsafe","reasons":["x"],"denials-in-a-row":3}"#,
    );
    let v: Vec<Value> = entries(&mut k, &id)
        .into_iter()
        .filter(|e| e["kind"] == "verdict")
        .collect();
    assert_eq!(v[0]["body"]["on"], "input");
    assert!(v[0]["body"].get("toolCallId").is_none());
    assert_eq!(v[1]["body"]["on"], "breaker");
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Running { .. }
    ));
}

#[test]
fn ac3_an_unsafe_or_unsure_input_verdict_fails_the_turn_judge() {
    for (on, outcome) in [("launch", "unsafe"), ("input/3", "unsure")] {
        let (mut k, id, t) = session(false);
        let item = json!({"type":"verdict","on":on,"outcome":outcome,"reasons":["r"]}).to_string();
        let c = feed(&mut k, &t, &item);
        let idle = Turn::Idle {
            stop: Stop::Failed {
                error: "judge".into(),
                cause: None,
            },
            seconds: None,
            tools: None,
            tokens: None,
        };
        let r = Record::load(&mut k.state, &id).unwrap();
        assert!(
            matches!(&r.turn, Turn::Idle { stop: Stop::Failed { error, .. }, .. } if error == "judge"),
            "{:?}",
            r.turn
        );
        let _ = idle;
        assert!(turn_states(&c).iter().any(|s| s["state"] == "idle"));
        k.provider_ended(&t).unwrap();
        let r2 = Record::load(&mut k.state, &id).unwrap();
        assert_eq!(r2.turn, r.turn);
    }
}

#[test]
fn ac3_a_safe_input_verdict_keeps_the_turn() {
    let (mut k, id, t) = session(false);
    feed(
        &mut k,
        &t,
        r#"{"type":"verdict","on":"launch","outcome":"safe","reasons":[]}"#,
    );
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Running { .. }
    ));
}

#[test]
fn ac4_bad_shapes_are_unknown_entries() {
    let bad = [
        r#"{"type":"verdict","on":"","outcome":"safe","reasons":[]}"#,
        r#"{"type":"verdict","on":5,"outcome":"safe","reasons":[]}"#,
        r#"{"type":"verdict","on":"a1","outcome":"maybe","reasons":[]}"#,
        r#"{"type":"verdict","on":"a1","outcome":"safe","reasons":[1]}"#,
        r#"{"type":"verdict","on":"a1","outcome":"safe"}"#,
        r#"{"type":"checking","on":""}"#,
        r#"{"type":"checking"}"#,
    ];
    for b in bad {
        let (mut k, id, t) = session(false);
        let c = feed(&mut k, &t, b);
        assert!(turn_states(&c).is_empty(), "{b}");
        let last = entries(&mut k, &id).pop().unwrap();
        assert_eq!(last["kind"], "unknown", "{b}");
        assert_eq!(
            last["body"]["raw"],
            serde_json::from_str::<Value>(b).unwrap()
        );
    }
}

#[test]
fn ac4_on_a_direct_stream_both_stay_unknown() {
    for b in [
        r#"{"type":"verdict","on":"launch","outcome":"unsafe","reasons":[]}"#,
        r#"{"type":"checking","on":"launch"}"#,
    ] {
        let (mut k, id, t) = session(true);
        feed(&mut k, &t, b);
        assert_eq!(entries(&mut k, &id).pop().unwrap()["kind"], "unknown");
        assert!(matches!(
            Record::load(&mut k.state, &id).unwrap().turn,
            Turn::Running { .. }
        ));
    }
}

/// Review r1 F1.3: a denied later input with a follow-up already queued: the turn fails `judge`
/// and the kernel sends the queued input (the same queue advance as any other turn end, 9.3).
#[test]
fn f1_3_a_denied_input_starts_the_queued_follow_up() {
    let (mut k, id, t) = session(false);
    feed(
        &mut k,
        &t,
        r#"{"type":"verdict","on":"launch","outcome":"safe","reasons":[]}"#,
    );
    k.say("alice", &id, "k2", r#"[{"type":"text","text":"follow up"}]"#)
        .unwrap();
    let turns = |k: &K| {
        k.host
            .calls
            .iter()
            .filter(|c| {
                matches!(c, HostCall::Request(Request::Turn { input, .. }) if input.contains("follow up"))
            })
            .count()
    };
    assert_eq!(turns(&k), 0, "queued while the turn runs");
    feed(
        &mut k,
        &t,
        r#"{"type":"verdict","on":"launch","outcome":"unsafe","reasons":["r"]}"#,
    );
    assert_eq!(turns(&k), 1, "the queued input goes out when the denied turn ends");
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Running { .. }
    ));
}
