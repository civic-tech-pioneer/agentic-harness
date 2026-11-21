//! S26-1: `delete` never steps and always cancels; one `turn` entry per turn past quota; the
//! account refusal names no other owner (TECHNICAL 9.3, 14).

use agent_kernel::active::{self, Slots};
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::transcript::MAX_CHUNKS;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    for who in ["alice", "bob"] {
        st.put(&format!("access/{who}"), br#"["p"]"#.to_vec())
            .unwrap();
    }
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

fn start(k: &mut K, owner: &str, key: &str) -> String {
    k.start(owner, key, "p", "{}", IN).unwrap().value
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

fn ask(id: &str) -> ProviderEvent {
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

fn idle(stop: Stop) -> Turn {
    Turn::Idle {
        stop,
        seconds: None,
        tools: None,
        tokens: None,
    }
}

fn turn_entries(k: &mut K, id: &str) -> usize {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .filter(|e| serde_json::from_str::<Value>(e).unwrap()["kind"] == "turn")
        .count()
}

fn keys_of(k: &K, id: &str) -> Vec<String> {
    k.state
        .entries()
        .map(|(a, _)| a.clone())
        .filter(|a| a.contains(&format!("/{id}")) && !a.starts_with("key/"))
        .collect()
}

/// A waiting session of an older boot, whose restart step would fail (no room for decisions).
fn stale_waiting(k: &mut K) -> String {
    let id = start(k, "alice", "k1");
    k.provider(&id, ask("a1")).unwrap();
    let mut r = Record::load(&mut k.state, &id).unwrap();
    r.chunks = (1..=MAX_CHUNKS as u64).collect();
    r.save(&mut k.state).unwrap();
    k.host.boot = 2;
    id
}

#[test]
fn ac1_delete_of_a_stale_session_runs_no_step_and_succeeds() {
    let mut k = kernel();
    let id = stale_waiting(&mut k);
    assert!(k.say("alice", &id, "q", "x").is_err(), "the step fails");
    k.delete("alice", &id).unwrap();
    assert!(Record::load(&mut k.state, &id).is_none());
    assert_eq!(keys_of(&k, &id), Vec::<String>::new());
    assert!(active::read_page(&mut k.state, 0)
        .iter()
        .all(|e| e.id != id));
    assert_eq!(Slots::load(&mut k.state).active[0], 0);
}

#[test]
fn ac1_delete_writes_no_entry_decision_or_turn() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, ask("a1")).unwrap();
    k.host.boot = 2;
    k.delete("alice", &id).unwrap();
    // no step: no entry, no decision, no turn (nothing of the session remains to hold one)
    assert!(Record::load(&mut k.state, &id).is_none());
    assert_eq!(keys_of(&k, &id), Vec::<String>::new());
}

#[test]
fn ac2_delete_sends_one_cancel_and_a_failed_delete_none() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, ask("a1")).unwrap();
    let n = requests(&k).len();
    assert_eq!(k.delete("bob", &id).unwrap_err(), KernelError::NotOwner);
    k.delete("alice", "zz").unwrap_err();
    assert_eq!(requests(&k).len(), n);
    k.delete("alice", &id).unwrap();
    assert_eq!(
        requests(&k)[n..],
        [Request::Cancel {
            session: id.clone()
        }]
    );
    // an idle session too
    let s2 = start(&mut k, "alice", "k2");
    k.provider(&s2, ProviderEvent::Turn(idle(Stop::Completed)))
        .unwrap();
    let n = requests(&k).len();
    k.delete("alice", &s2).unwrap();
    assert_eq!(requests(&k)[n..], [Request::Cancel { session: s2 }]);
}

#[test]
fn ac3_past_quota_a_turn_gets_one_turn_entry() {
    let mut k = kernel();
    let s = start(&mut k, "alice", "k1");
    k.limits.owner_bytes = agent_kernel::Usage::load(&mut k.state, "quota/alice").bytes;
    let out = ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Message,
            uuid: None,
            body: json!({"text": "x"}),
        }),
        ..Out::default()
    });
    let at_start = turn_entries(&mut k, &s);
    k.provider(&s, out).unwrap();
    k.provider(&s, ProviderEvent::Turn(idle(Stop::Completed)))
        .unwrap();
    // the stop of the active turn is the one entry; the inactive-to-inactive change adds none
    let after_first = turn_entries(&mut k, &s);
    assert_eq!(after_first, at_start + 1);
    let turn = Record::load(&mut k.state, &s).unwrap().turn;
    for i in 0..100 {
        let t = if i % 2 == 0 {
            Turn::Running {
                since: 5,
                activity: None,
            }
        } else {
            idle(Stop::Completed)
        };
        k.provider(&s, ProviderEvent::Turn(t)).unwrap();
    }
    assert_eq!(turn_entries(&mut k, &s), after_first);
    assert_eq!(Record::load(&mut k.state, &s).unwrap().turn, turn);
}

#[test]
fn ac4_the_account_refusal_names_no_other_owner() {
    let mut k = kernel();
    start(&mut k, "alice", "k1");
    start(&mut k, "bob", "k1");
    k.limits.account_bytes = agent_kernel::Usage::load(&mut k.state, "quota").bytes;
    let e = k.start("bob", "k2", "p", "{}", IN).unwrap_err();
    assert_eq!(
        e.to_string(),
        "quota: this mod's agent sessions are at their limit"
    );
    let _: Value = json!(null);
}
