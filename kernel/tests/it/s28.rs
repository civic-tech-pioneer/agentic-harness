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
    crate::fixture_access::put(&mut st, "access/alice", br#"["p","np"]"#.to_vec()).unwrap();
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

/// A turn that ended from `waiting` has no `since`, so no `seconds`.
fn failed_waiting() -> Turn {
    match failed() {
        Turn::Idle {
            stop,
            tools,
            tokens,
            ..
        } => Turn::Idle {
            stop,
            seconds: None,
            tools,
            tokens,
        },
        t => t,
    }
}

fn failed() -> Turn {
    Turn::Idle {
        stop: Stop::Failed {
            error: "provider-ended".into(),
            cause: None,
        },
        seconds: Some(0),
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

/// Each turn handed to the provider, with whether it resumes the Claude session: a new stream's
/// `start` carries the input and a launch that names `session {resume}` (4.1, 10).
fn turn_requests(k: &K) -> Vec<(String, bool)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Turn { input, resume, .. }) => {
                Some((input.clone(), *resume))
            }
            HostCall::StartProvider { launch, input } => {
                let v: Value = serde_json::from_str(launch).unwrap();
                Some((input.clone(), v["session"]["resume"].is_string()))
            }
            _ => None,
        })
        .collect()
}

/// Gives the session its Claude session id, as `system/init` does (9.8 "Adapter state").
fn init(k: &mut K, id: &str) {
    let t = rec(k, id).streams[0].clone();
    let line = br#"{"type":"system","subtype":"init","session_id":"c0ffee00-0000-4000-8000-000000000001"}"#;
    k.provider_lines(&t, vec![(1, line.to_vec())]).unwrap();
}

fn opens(k: &K) -> usize {
    k.host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::StartProvider { .. }))
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
    k.attach("alice", &id, 0).unwrap();
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
    assert_eq!(new[2]["body"], failed_waiting().entry_body());
    assert_eq!(rec(&mut k, &id).turn, failed_waiting());
}

#[test]
fn ac3_past_quota_one_turn_entry_and_its_push() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    k.provider_item("t1", ask("a1")).unwrap();
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    let n = turns(&mut k, &id);
    let c = k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert_eq!(rec(&mut k, &id).turn, failed_waiting());
    assert_eq!(turns(&mut k, &id), n + 1);
    assert!(c
        .pushes
        .iter()
        .any(|p| p.value["type"] == "turn" && p.value["state"] == failed_waiting().to_json()));
}

#[test]
fn ac4_the_queue_waits_for_a_person() {
    for (profile, resume) in [("p", true), ("np", false)] {
        let mut k = kernel();
        let id = k.start("alice", "k1", profile, "{}", IN).unwrap().value;
        init(&mut k, &id);
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
    init(&mut k, &id);
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
        seconds: Some(0),
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

fn message(text: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Message,
            uuid: None,
            body: json!({"role": "assistant", "content": [{"type": "text", "text": text}]}),
        }),
        ..Out::default()
    })
}

fn messages_of(k: &mut K, owner: &str, id: &str) -> usize {
    k.read(owner, id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .filter(|e| serde_json::from_str::<Value>(e).unwrap()["kind"] == "message")
        .count()
}

/// S28-2 AC3 (passes before the change too: every call read the accounts first): the past flag
/// is each call's own. Alice at her cap drops a message; Bob's call right after stores his.
#[test]
fn ac3_the_past_flag_is_per_call() {
    let mut k = kernel();
    crate::fixture_access::put(&mut k.state, "access/bob", br#"["p"]"#.to_vec()).unwrap();
    let a = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &a, 0).unwrap();
    let b = k.start("bob", "k2", "p", "{}", IN).unwrap().value;
    for n in ["a1", "a2", "a3"] {
        k.provider(&a, ask(n)).unwrap();
    }
    let alice = Usage::load(&mut k.state, "quota/alice").bytes;
    assert!(Usage::load(&mut k.state, "quota/bob").bytes < alice);
    k.limits.owner_bytes = alice;
    let (na, nb) = (
        messages_of(&mut k, "alice", &a),
        messages_of(&mut k, "bob", &b),
    );
    k.provider(&a, message("dropped")).unwrap();
    k.provider(&b, message("kept")).unwrap();
    assert_eq!(messages_of(&mut k, "alice", &a), na, "alice's was stored");
    assert_eq!(messages_of(&mut k, "bob", &b), nb + 1, "bob's was dropped");
}

/// Every stored byte the accounts meter (the record, owner and per-session key kinds); with
/// `session` only the per-session kinds, which are the record's `bytes`.
fn held(k: &K, session: bool) -> i64 {
    k.state
        .entries()
        .filter(|(key, _)| {
            let head = key.split('/').next().unwrap_or_default();
            let kinds: &[&str] = if session {
                &["queue", "entries", "wire", "loop", "stream", "live"]
            } else {
                &[
                    "session", "queue", "entries", "wire", "loop", "stream", "live", "key", "owner",
                ]
            };
            kinds.contains(&head)
        })
        .map(|(key, v)| (key.len() + v.len()) as i64)
        .sum()
}

fn decisions(k: &mut K, id: &str) -> usize {
    entries(k, id)
        .iter()
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .count()
}

/// S28-fix1 AC2: at the ceiling the `ended` still takes effect and is charged, past it.
#[test]
fn ac2_ended_at_the_ceiling_closes_the_asks_and_charges_what_it_wrote() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    k.provider_item("t1", ask("a1")).unwrap();
    k.provider_item("t1", ask("a2")).unwrap();
    k.answer("alice", &id, "ans1", "a2", r#"{"allow":true}"#)
        .unwrap();
    let r = rec(&mut k, &id);
    assert!(matches!(r.turn, Turn::Waiting { .. }));
    assert_eq!((r.open.len(), r.pending_answers.len()), (1, 1));
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
    let (o0, a0, b0, h0, s0) = (
        Usage::load(&mut k.state, "quota/alice").bytes as i64,
        Usage::load(&mut k.state, "quota").bytes as i64,
        rec(&mut k, &id).bytes as i64,
        held(&k, false),
        held(&k, true),
    );
    let (d0, t0) = (decisions(&mut k, &id), turns(&mut k, &id));
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    let r = rec(&mut k, &id);
    assert_eq!(r.turn, failed_waiting());
    assert!(r.open.is_empty() && r.pending_answers.is_empty());
    assert_eq!(decisions(&mut k, &id), d0 + 2);
    assert_eq!(turns(&mut k, &id), t0 + 1);
    assert!(k.state.get("stream/t1").is_none());
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    let wrote = held(&k, false) - h0;
    assert!(wrote > 0);
    assert_eq!(
        Usage::load(&mut k.state, "quota/alice").bytes as i64 - o0,
        wrote
    );
    assert_eq!(Usage::load(&mut k.state, "quota").bytes as i64 - a0, wrote);
    assert_eq!(r.bytes as i64 - b0, held(&k, true) - s0);
}

/// S28-fix1 AC3: `ended` changes no `paused` turn.
#[test]
fn ac3_ended_leaves_a_paused_turn() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let mut r = rec(&mut k, &id);
    r.turn = Turn::Paused { until: None };
    r.save(&mut k.state).unwrap();
    let n = turns(&mut k, &id);
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert_eq!(rec(&mut k, &id).turn, Turn::Paused { until: None });
    assert_eq!(turns(&mut k, &id), n);
    assert!(k.state.get("stream/t1").is_none());
}

/// S29-2: at the ceiling an `ask` item succeeds, is not stored and is denied `quota`.
#[test]
fn ac4_an_ask_at_the_ceiling_is_denied_quota() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
    let (a0, h0) = (
        Usage::load(&mut k.state, "quota").bytes as i64,
        held(&k, false),
    );
    let n = kinds(&mut k, &id).iter().filter(|x| *x == "ask").count();
    k.provider_item("t1", ask("a1")).unwrap();
    assert!(rec(&mut k, &id).open.is_empty());
    assert_eq!(kinds(&mut k, &id).iter().filter(|x| *x == "ask").count(), n);
    assert!(k.host.calls.iter().any(|c| matches!(c,
        HostCall::Request(Request::Answer { decision, .. }) if decision["reason"] == "quota")));
    let wrote = held(&k, false) - h0;
    assert_eq!(Usage::load(&mut k.state, "quota").bytes as i64 - a0, wrote);
}
