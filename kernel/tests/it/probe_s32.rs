//! S32-1: the stage A probe (TECHNICAL 9.9): numbered entries a timer writes and pushes live.

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::Value;

type K = Kernel<MemState, FakeHost>;

fn kernel() -> K {
    let mut st = MemState::new();
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

fn probe(k: &mut K, key: &str) -> String {
    k.probe_start("alice", key, 3, 1000).unwrap().value
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn dump(k: &K) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn fire(k: &mut K, id: &str) {
    k.host.now += 1000;
    k.timer(&format!("s/{id}"));
}

fn refused_field(e: KernelError) -> String {
    match e {
        KernelError::Refused(r) => {
            assert_eq!(r.len(), 1);
            r[0].field.clone()
        }
        other => panic!("{other:?}"),
    }
}

/// The `entry` items pushed on a stream: `(i, text)`.
fn shown(k: &K, stream: &str) -> Vec<(u64, String)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream: s, value } if s == stream => Some(value.clone()),
            _ => None,
        })
        .filter(|v| v["_meta"]["watane"]["type"] == "entry")
        .map(|v| {
            let e = &v["_meta"]["watane"]["entry"];
            assert_eq!(e["kind"], "message");
            assert_eq!(e["body"]["role"], "agent");
            (
                e["i"].as_u64().unwrap(),
                e["body"]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            )
        })
        .collect()
}

fn attach(k: &mut K, id: &str, after: u64) -> String {
    match k.attach("alice", id, after).unwrap().value {
        Attached::Live { stream } => stream,
        other => panic!("{other:?}"),
    }
}

fn items(n: std::ops::RangeInclusive<u64>) -> Vec<(u64, String)> {
    n.map(|i| (i, format!("Item {i} of 3"))).collect()
}

#[test]
fn ac8_a_probe_start_makes_the_session_and_arms_its_timer() {
    let mut k = kernel();
    let from = k.host.calls.len();
    let id = probe(&mut k, "k1");
    let r = rec(&mut k, &id);
    assert_eq!(r.backend, "probe");
    assert_eq!(r.title, "Numbered items");
    assert_eq!(r.profile, "");
    assert_eq!(r.turn, Turn::default());
    assert_eq!(
        r.deadlines,
        [Deadline {
            what: "probe".into(),
            at: 2000
        }]
    );
    assert!(r.streams.is_empty());
    assert!(k.host.calls[from..].contains(&HostCall::TimerAfter {
        id: format!("s/{id}"),
        delay_ms: 1000
    }));
    assert_eq!(Slots::load(&mut k.state).timers, 1);
    let list = k.list("alice", None, 10).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].backend, "probe");
}

#[test]
fn ac8_a_repeat_with_the_same_key_changes_nothing() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    let before = dump(&k);
    let calls = k.host.calls.len();
    let again = k.probe_start("alice", "k1", 3, 1000).unwrap();
    assert_eq!(again.value, id);
    assert!(
        k.host.calls[calls..]
            .iter()
            .all(|c| matches!(c, HostCall::Generation)),
        "no timer or stream"
    );
    assert_eq!(dump(&k), before);
}

#[test]
fn ac8_count_and_every_are_checked() {
    let mut k = kernel();
    for (c, e, field) in [
        (0, 1000, "count"),
        (1001, 1000, "count"),
        (3, 99, "every-ms"),
        (3, 60_001, "every-ms"),
    ] {
        let before = dump(&k);
        let err = k.probe_start("alice", "k", c, e).unwrap_err();
        assert_eq!(refused_field(err), field);
        assert_eq!(dump(&k), before);
    }
    for (c, e) in [(1, 100), (1000, 60_000)] {
        k.probe_start("alice", &format!("ok{c}"), c, e).unwrap();
    }
}

#[test]
fn ac8_a_probe_counts_against_the_owners_sessions() {
    let mut k = kernel();
    k.limits.owner_sessions = 1;
    probe(&mut k, "k1");
    assert!(matches!(
        k.probe_start("alice", "k2", 3, 1000),
        Err(KernelError::Quota(_))
    ));
    assert!(k.probe_start("bob", "k1", 3, 1000).is_ok());
}

#[test]
fn ac9_each_firing_stores_and_pushes_the_next_entry() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    let o = attach(&mut k, &id, 0);
    for _ in 0..3 {
        fire(&mut k, &id);
    }
    assert_eq!(shown(&k, &o), items(1..=3));
    // the third arms no further timer
    let armed = k
        .host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::TimerAfter { .. }))
        .count();
    assert_eq!(armed, 3, "the start and two firings");
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(Slots::load(&mut k.state).timers, 0);
    // a fourth firing does nothing
    let before = dump(&k);
    fire(&mut k, &id);
    assert_eq!(dump(&k), before);
}

#[test]
fn ac9_reattach_replays_exactly_the_missing_entries() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    for _ in 0..3 {
        fire(&mut k, &id);
    }
    let o = attach(&mut k, &id, 2);
    assert_eq!(shown(&k, &o), [(3, "Item 3 of 3".to_string())]);
    let o = attach(&mut k, &id, 0);
    assert_eq!(shown(&k, &o), items(1..=3));
    let page = k.read("alice", &id, 0, None, 10).unwrap();
    let got: Vec<Value> = page
        .entries
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(got.len(), 3);
    assert_eq!(got[2]["body"]["content"][0]["text"], "Item 3 of 3");
}

#[test]
fn ac9_a_full_owner_stream_is_dropped_lagging_and_loses_nothing() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    let o = attach(&mut k, &id, 0);
    fire(&mut k, &id);
    k.host.full.insert(o.clone());
    fire(&mut k, &id);
    assert!(k.host.calls.contains(&HostCall::Close {
        stream: o.clone(),
        outcome: Outcome::Lagging
    }));
    assert!(k.state.get(&format!("live/{id}")).is_none());
    k.host.full.clear();
    fire(&mut k, &id);
    assert_eq!(shown(&k, &o), items(1..=1));
    // the page had shown 1; it attaches after 1 and gets exactly 2 and 3
    let o2 = attach(&mut k, &id, 1);
    assert_eq!(shown(&k, &o2), items(2..=3));
}

#[test]
fn ac10_a_probe_takes_no_input() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    let no = KernelError::Backend("a probe session takes no input".into());
    assert_eq!(k.say("alice", &id, "s", "hi").unwrap_err(), no);
    assert_eq!(k.answer("alice", &id, "a", "x", "{}").unwrap_err(), no);
    assert_eq!(k.interrupt("alice", &id, None).unwrap_err(), no);
    assert_eq!(k.queued("alice", &id).unwrap_err(), no);
    assert_eq!(k.unqueue("alice", &id, 1).unwrap_err(), no);
    assert_eq!(k.take_back("alice", &id, "1").unwrap_err(), no);
    // another owner's call is refused as for any session
    assert_eq!(
        k.say("bob", &id, "s", "hi").unwrap_err(),
        KernelError::NotOwner
    );
    assert!(matches!(
        k.say("alice", "nope", "s", "hi"),
        Err(KernelError::UnknownSession(_))
    ));
}

#[test]
fn ac10_cancel_stops_the_entries() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    fire(&mut k, &id);
    k.cancel("alice", &id).unwrap();
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(Slots::load(&mut k.state).timers, 0);
    let before = dump(&k);
    fire(&mut k, &id);
    assert_eq!(dump(&k), before);
    assert_eq!(k.read("alice", &id, 0, None, 10).unwrap().entries.len(), 1);
}

#[test]
fn ac10_list_rename_delete_work() {
    let mut k = kernel();
    let id = probe(&mut k, "k1");
    k.rename("alice", &id, "Mine").unwrap();
    assert_eq!(k.list("alice", None, 10).unwrap()[0].title, "Mine");
    k.delete("alice", &id).unwrap();
    assert!(k.list("alice", None, 10).unwrap().is_empty());
    assert_eq!(Slots::load(&mut k.state).timers, 0);
    assert!(k.host.calls.contains(&HostCall::TimerCancel {
        id: format!("s/{id}")
    }));
}
