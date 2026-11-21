//! S26-1: `delete` never steps and always cancels; one `turn` entry per turn past quota; the
//! account refusal names no other owner (TECHNICAL 9.3, 14).

use agent_kernel::active::{self, Slots};
use agent_kernel::adapters::{Entry, Kind, Out, Resolved};
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

/// The puts a `delete` of a waiting session writes, its record of an older boot or not.
fn delete_writes(stale: bool) -> u64 {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, ask("a1")).unwrap();
    if stale {
        k.host.boot = 2;
    }
    // another session's call takes the new boot first, so the delete's own writes compare
    start(&mut k, "bob", "k1");
    k.state.reset_counts();
    k.delete("alice", &id).unwrap();
    assert!(Record::load(&mut k.state, &id).is_none());
    assert_eq!(keys_of(&k, &id), Vec::<String>::new());
    k.state.counts().puts
}

#[test]
fn ac1_delete_writes_no_entry_decision_or_turn() {
    // no step: a stale session's delete writes what a current one's does (no decision, no turn)
    assert_eq!(delete_writes(true), delete_writes(false));
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
}

#[test]
fn ac3_past_quota_an_active_change_writes_nothing_but_activity_still_pushes() {
    let mut k = kernel();
    let s = start(&mut k, "alice", "k1");
    k.limits.owner_bytes = agent_kernel::Usage::load(&mut k.state, "quota/alice").bytes;
    let turn = Record::load(&mut k.state, &s).unwrap().turn;
    let n = turn_entries(&mut k, &s);
    // active to active (running to waiting): no entry, the turn stays
    let waiting = Turn::Waiting {
        asks: vec!["a1".into()],
        expires_at: None,
    };
    k.provider(&s, ProviderEvent::Turn(waiting)).unwrap();
    assert_eq!(turn_entries(&mut k, &s), n);
    assert_eq!(Record::load(&mut k.state, &s).unwrap().turn, turn);
    // an activity change alone is live only (9.3): still pushed past quota, nothing written
    let Turn::Running { since, .. } = turn else {
        panic!("a started session runs: {turn:?}")
    };
    let t = Turn::Running {
        since,
        activity: Some(Activity::Compacting),
    };
    let c = k.provider(&s, ProviderEvent::Turn(t)).unwrap();
    assert_eq!(
        c.pushes[0].value["state"]["activity"],
        json!({"kind": "compacting"})
    );
    assert_eq!(turn_entries(&mut k, &s), n);
}

fn resolved(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        resolved: Some(Resolved {
            id: id.into(),
            by: "answered".into(),
            outcome: Some("deny".into()),
            direct: true,
            raw: json!({"type": "ask-resolved", "id": id, "by": "answered"}),
        }),
        ..Out::default()
    })
}

/// The turn pushes of a call, with the record's turn after it.
fn turn_pushes(c: &Called<Item>) -> Vec<Value> {
    c.pushes
        .iter()
        .filter(|p| p.value["type"] == "turn")
        .map(|p| p.value["state"].clone())
        .collect()
}

/// S26-fix1 AC2, 9.3 Quota: past quota an ask that closes, and `close_all`, write no `turn`
/// entry, but the record's turn follows (the waiting asks shrink, then `running`) and is pushed.
#[test]
fn fix1_ac2_past_quota_closing_asks_move_the_turn_without_entries() {
    let mut k = kernel();
    let s = start(&mut k, "alice", "k1");
    for a in ["a1", "a2", "a3"] {
        k.provider_item("t1", ask(a)).unwrap();
    }
    k.limits.owner_bytes = agent_kernel::Usage::load(&mut k.state, "quota/alice").bytes;
    let n = turn_entries(&mut k, &s);
    let turn = |k: &mut K| Record::load(&mut k.state, &s).unwrap().turn;
    let c = k.provider_item("t1", resolved("a1")).unwrap();
    let Turn::Waiting { asks, .. } = turn(&mut k) else {
        panic!("still waiting: {:?}", turn(&mut k))
    };
    assert_eq!(asks, vec!["a2".to_string(), "a3".to_string()]);
    assert_eq!(turn_pushes(&c), vec![turn(&mut k).to_json()]);
    let c = k.provider_item("t1", resolved("a2")).unwrap();
    assert!(matches!(turn(&mut k), Turn::Waiting { ref asks, .. } if asks == &["a3"]));
    assert_eq!(turn_pushes(&c), vec![turn(&mut k).to_json()]);
    // `close_all` (the provider stream ends): the last ask goes, the turn runs
    let c = k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert!(matches!(turn(&mut k), Turn::Running { .. }));
    assert_eq!(turn_pushes(&c), vec![turn(&mut k).to_json()]);
    assert_eq!(turn_entries(&mut k, &s), n);
}
