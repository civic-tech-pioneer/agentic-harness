//! S28-1: a provider stream's `ended` on a `running` or `waiting` turn ends it
//! `idle {failed {provider-ended}}` (TECHNICAL 9.3, 9.8 rule 4, case 76).

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put(
        "profile/p",
        br#"{"mode":"manual","tools":["Read"],"max-turns":20}"#.to_vec(),
    )
    .unwrap();
    st.put(
        "profile/np",
        br#"{"mode":"manual","persist":false}"#.to_vec(),
    )
    .unwrap();
    st.put("access/alice", br#"["p","np"]"#.to_vec()).unwrap();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        boot: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn ask(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": id, "toolCall": {"toolCallId": id, "title": "Edit"},
                "input": {}, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn failed() -> Turn {
    Turn::Idle {
        stop: Stop::Failed {
            error: "provider-ended".into(),
        },
        seconds: None,
        tools: None,
        tokens: None,
    }
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .collect()
}

fn kinds(k: &mut K, id: &str) -> Vec<String> {
    entries(k, id)
        .iter()
        .map(|e| e["kind"].as_str().unwrap().to_string())
        .collect()
}

fn turns(k: &mut K, id: &str) -> usize {
    kinds(k, id).iter().filter(|x| *x == "turn").count()
}

fn turn_requests(k: &K) -> Vec<(String, bool)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Turn { input, resume, .. }) => {
                Some((input.clone(), *resume))
            }
            _ => None,
        })
        .collect()
}

fn opens(k: &K) -> usize {
    k.host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::Open { .. }))
        .count()
}

#[test]
fn ac1_ended_ends_the_turn() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    assert!(rec(&mut k, &id).active_page.is_some());
    let c = k.provider_item("t1", ProviderEvent::Ended).unwrap();
    let r = rec(&mut k, &id);
    assert_eq!(r.turn, failed());
    let last = entries(&mut k, &id).pop().unwrap();
    assert_eq!(last["kind"], "turn");
    assert_eq!(last["body"], failed().entry_body());
    assert!(c
        .pushes
        .iter()
        .any(|p| p.value["type"] == "turn" && p.value["state"] == failed().to_json()));
    assert!(r.active_page.is_none());
    assert!(r.deadlines.iter().all(|d| d.what != "idle"));
    assert!(r.streams.is_empty());
}

#[test]
fn ac2_asks_first_then_one_turn_entry() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask("a1")).unwrap();
    k.provider_item("t1", ask("a2")).unwrap();
    k.answer("alice", &id, "ans1", "a2", r#"{"allow":true}"#)
        .unwrap();
    assert!(matches!(rec(&mut k, &id).turn, Turn::Waiting { .. }));
    let n = entries(&mut k, &id).len();
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    let new: Vec<Value> = entries(&mut k, &id).split_off(n);
    let ks: Vec<&str> = new.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(ks, ["ask", "ask", "turn"]);
    assert_eq!(new[2]["body"], failed().entry_body());
    assert_eq!(rec(&mut k, &id).turn, failed());
}

#[test]
fn ac3_past_quota_one_turn_entry_and_its_push() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask("a1")).unwrap();
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    let n = turns(&mut k, &id);
    let c = k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert_eq!(rec(&mut k, &id).turn, failed());
    assert_eq!(turns(&mut k, &id), n + 1);
    assert!(c
        .pushes
        .iter()
        .any(|p| p.value["type"] == "turn" && p.value["state"] == failed().to_json()));
}

#[test]
fn ac4_the_queue_waits_for_a_person() {
    for (profile, resume) in [("p", true), ("np", false)] {
        let mut k = kernel();
        let id = k.start("alice", "k1", profile, "{}", IN).unwrap().value;
        k.say("alice", &id, "s1", "one").unwrap();
        k.host.calls.clear();
        k.provider_item("t1", ProviderEvent::Ended).unwrap();
        assert_eq!(rec(&mut k, &id).turn, failed());
        assert!(turn_requests(&k).is_empty());
        assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
        k.say("alice", &id, "s2", "two").unwrap();
        assert_eq!(turn_requests(&k), [("one".to_string(), resume)]);
        assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
        let q = k.queued("alice", &id).unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].input, "two");
        assert_eq!(opens(&k), 1);
    }
}

#[test]
fn ac5_the_next_say_starts_a_turn() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    k.host.calls.clear();
    k.say("alice", &id, "s1", "again").unwrap();
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
    assert_eq!(turn_requests(&k), [("again".to_string(), true)]);
    assert!(k.queued("alice", &id).unwrap().is_empty());
    let last_msg = entries(&mut k, &id)
        .into_iter()
        .rfind(|e| e["kind"] == "message")
        .unwrap();
    assert_eq!(last_msg["body"]["role"], "user");
}

#[test]
fn ac6_inactive_turns_are_unchanged() {
    let done = Turn::Idle {
        stop: Stop::Completed,
        seconds: None,
        tools: None,
        tokens: None,
    };
    for t in [done, Turn::Unknown, Turn::ResumeLost] {
        let mut k = kernel();
        let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
        k.provider(&id, ProviderEvent::Turn(t.clone())).unwrap();
        let n = turns(&mut k, &id);
        k.provider_item("t1", ProviderEvent::Ended).unwrap();
        let r = rec(&mut k, &id);
        assert_eq!(r.turn, t);
        assert_eq!(turns(&mut k, &id), n);
        assert!(r.streams.is_empty());
        assert!(k.state.get("stream/t1").is_none());
    }
}
