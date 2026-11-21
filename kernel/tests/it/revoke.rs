//! Revoking `access/` (TECHNICAL 9.7, 16 case 74): a call that would give a session more work reads
//! `access/<owner>` again; calls that only read or take power away still work.

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::*;
use serde_json::json;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
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
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn revoke(k: &mut K, value: &[u8]) {
    k.state.put("access/alice", value.to_vec()).unwrap();
}

/// A session with one queued input.
fn queued(k: &mut K) -> String {
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    k.say("alice", &id, "q", "later").unwrap();
    id
}

fn requests(k: &K) -> Vec<Request> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

fn ask_event(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": "t", "toolCall": {"toolCallId": "t", "title": "Edit"},
                "input": {}, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
}

/// The record, queue, transcript and active page: what a refused call must leave alone.
fn snap(k: &mut K, id: &str) -> Vec<Option<Vec<u8>>> {
    let r = rec(k, id);
    let mut keys = vec![
        Record::key_of(id),
        Queue::key_of(id),
        "slots".to_string(),
        "active/0".to_string(),
    ];
    keys.extend((0..r.chunks.len()).map(|n| chunk_key(id, n)));
    keys.into_iter().map(|x| k.state.get(&x)).collect()
}

fn is_profile(e: KernelError) -> bool {
    matches!(e, KernelError::Refused(r) if r.len() == 1 && r[0].field == "profile")
}

#[test]
fn case74_each_call_that_gives_more_work_is_refused_and_writes_nothing() {
    let mut k = kernel();
    let id = queued(&mut k);
    k.provider(&id, ask_event("a1")).unwrap();
    revoke(&mut k, br#"["other"]"#);
    let before = snap(&mut k, &id);
    let sent = requests(&k);
    assert!(is_profile(k.say("alice", &id, "k2", "x").unwrap_err()));
    assert!(is_profile(
        k.interrupt("alice", &id, Some("x")).unwrap_err()
    ));
    for d in [r#"{"allow":true}"#, r#"{"allow":true,"for":"session"}"#] {
        assert!(is_profile(
            k.answer("alice", &id, "k3", "a1", d).unwrap_err()
        ));
    }
    assert_eq!(snap(&mut k, &id), before);
    assert_eq!(requests(&k), sent);
    assert!(k.state.get("key/alice/k2").is_none() && k.state.get("key/alice/k3").is_none());
}

#[test]
fn case74_continue_on_an_unknown_session_is_refused() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.host.generation = 2;
    k.read("alice", &id, 0, None, 1).unwrap();
    assert_eq!(rec(&mut k, &id).turn, Turn::Unknown);
    revoke(&mut k, b"[]");
    let (before, sent) = (snap(&mut k, &id), requests(&k));
    assert!(is_profile(k.say("alice", &id, "k2", "go on").unwrap_err()));
    assert_eq!(snap(&mut k, &id), before);
    assert_eq!(requests(&k), sent);
}

#[test]
fn case74_a_turn_end_starts_no_queued_input_and_keeps_the_queue() {
    let mut k = kernel();
    let id = queued(&mut k);
    revoke(&mut k, br#"["other"]"#);
    let sent = requests(&k);
    let c = k
        .provider(
            &id,
            ProviderEvent::Out(Out {
                idle: true,
                ..Out::default()
            }),
        )
        .unwrap();
    assert!(c.pushes.iter().all(|p| p.value["type"] == "turn"));
    assert_eq!(rec(&mut k, &id).turn, Turn::default());
    assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
    assert_eq!(requests(&k), sent);
    assert!(rec(&mut k, &id).active_page.is_none());
    // The queue stays for `unqueue`.
    k.unqueue("alice", &id, 1).unwrap();
    assert!(k.queued("alice", &id).unwrap().is_empty());
}

#[test]
fn case74_a_profile_still_listed_starts_the_queued_input() {
    let mut k = kernel();
    let id = queued(&mut k);
    k.provider(&id, ProviderEvent::Turn(Turn::default()))
        .unwrap();
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
    assert!(k.queued("alice", &id).unwrap().is_empty());
}

#[test]
fn case74_an_unreadable_access_value_acts_as_an_empty_one() {
    for bad in [&b"not json"[..], br#"{"p":1}"#, br#"[1,2]"#, b""] {
        let mut k = kernel();
        let id = queued(&mut k);
        revoke(&mut k, bad);
        assert!(is_profile(k.say("alice", &id, "k2", "x").unwrap_err()));
        assert!(is_profile(
            k.start("alice", "k9", "p", "{}", IN).unwrap_err()
        ));
    }
    let mut k = kernel();
    let id = queued(&mut k);
    k.state.delete("access/alice");
    assert!(is_profile(k.say("alice", &id, "k2", "x").unwrap_err()));
}

#[test]
fn case74_the_calls_that_only_read_or_take_power_away_still_work() {
    let mut k = kernel();
    let id = queued(&mut k);
    k.provider(&id, ask_event("a1")).unwrap();
    k.provider(&id, ask_event("a2")).unwrap();
    revoke(&mut k, b"[]");
    k.read("alice", &id, 0, None, 10).unwrap();
    assert_eq!(k.list("alice", None, 10).unwrap().len(), 1);
    assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
    k.rename("alice", &id, "new").unwrap();
    k.answer("alice", &id, "k3", "a1", r#"{"deny":true}"#)
        .unwrap();
    k.interrupt("alice", &id, None).unwrap();
    assert_eq!(
        rec(&mut k, &id).turn,
        Turn::Idle {
            stop: Stop::Interrupted,
            seconds: None,
            tools: None,
            tokens: None
        }
    );
    k.unqueue("alice", &id, 1).unwrap();
    k.cancel("alice", &id).unwrap();
    k.delete("alice", &id).unwrap();
    assert!(Record::load(&mut k.state, &id).is_none());
}
