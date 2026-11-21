//! S26-2: the kernel tracks its provider streams (TECHNICAL 9.1 `stream/<id>`, 9.4, 9.5 "Slots", R4).

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const NP: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20,"persist":false}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    st.put("profile/np", NP.as_bytes().to_vec()).unwrap();
    for who in ["alice", "bob", "carol"] {
        st.put(&format!("access/{who}"), br#"["p","np"]"#.to_vec())
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
        now: 1000,
        ..FakeHost::default()
    };
    let mut k = Kernel::new(st, host);
    k.limits.owner_sessions = 64;
    k
}

fn start(k: &mut K, owner: &str, key: &str) -> String {
    k.start(owner, key, "p", "{}", IN).unwrap().value
}

fn idle(stop: Stop) -> ProviderEvent {
    ProviderEvent::Turn(Turn::Idle {
        stop,
        seconds: None,
        tools: None,
        tokens: None,
    })
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

fn opens(k: &K) -> Vec<String> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Open { stream } => Some(stream.clone()),
            _ => None,
        })
        .collect()
}

fn last_resume(k: &K) -> bool {
    match requests(k).last() {
        Some(Request::Turn { resume, .. }) => *resume,
        other => panic!("no turn request: {other:?}"),
    }
}

fn dump(k: &K) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn usage(k: &mut K, owner: &str) -> Usage {
    Usage::load(&mut k.state, &Usage::owner_key(owner))
}

fn provider_keys(k: &K) -> Vec<String> {
    k.state
        .entries()
        .filter(|(key, v)| {
            key.starts_with("stream/")
                && serde_json::from_slice::<Value>(v).unwrap()["kind"] == "provider"
        })
        .map(|(key, _)| key.clone())
        .collect()
}

/// AC4: the kernel's count and each owner's count are those of the live provider keys of this boot.
fn check(k: &mut K, owners: &[&str]) {
    let live = provider_keys(k);
    let slots = Slots::load(&mut k.state);
    assert_eq!(slots.provider as usize, live.len(), "{live:?}");
    for o in owners {
        let mine = live
            .iter()
            .filter(|key| {
                let v: Value = serde_json::from_slice(&k.state.get(key).unwrap()).unwrap();
                let id = v["session"].as_str().unwrap().to_string();
                rec(k, &id).owner == *o
            })
            .count();
        let u = usage(k, o);
        assert_eq!(u.provider_at(slots.boot) as usize, mine, "{o}");
    }
}

fn busy_text(e: KernelError) -> String {
    match e {
        KernelError::Busy(t) => t,
        other => panic!("not busy: {other:?}"),
    }
}

#[test]
fn ac1_start_opens_a_provider_stream_and_records_it() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    assert_eq!(opens(&k), ["t1"]);
    let v: Value = serde_json::from_slice(&k.state.get("stream/t1").unwrap()).unwrap();
    assert_eq!(v, json!({"session": id, "kind": "provider"}));
    assert_eq!(rec(&mut k, &id).streams, ["t1"]);
    let slots = Slots::load(&mut k.state);
    assert_eq!(slots.provider, 1);
    let u = usage(&mut k, "alice");
    assert_eq!((u.provider, u.slots_boot), (1, slots.boot));
    assert_eq!(
        requests(&k),
        [Request::Turn {
            session: id.clone(),
            input: IN.into(),
            resume: false
        }]
    );
    // the stream key is charged to the owner and counts in the session's bytes (9.3 Bytes)
    let r = rec(&mut k, &id);
    let charged = "stream/t1".len() + k.state.get("stream/t1").unwrap().len();
    assert!(r.bytes >= charged as u64);
    // a turn on the live stream, and a queued input, open nothing
    k.provider(&id, idle(Stop::Completed)).unwrap();
    k.say("alice", &id, "k2", "again").unwrap();
    assert_eq!(opens(&k), ["t1"]);
    assert!(!last_resume(&k));
    k.say("alice", &id, "k3", "queued").unwrap();
    assert_eq!(opens(&k), ["t1"]);
    assert_eq!(Slots::load(&mut k.state).provider, 1);
    check(&mut k, &["alice"]);
}

#[test]
fn ac2_busy_at_eight_and_at_four_per_owner_waits_for_nothing() {
    let mut k = kernel();
    for n in 0..4 {
        start(&mut k, "alice", &format!("a{n}"));
        start(&mut k, "bob", &format!("b{n}"));
    }
    k.host.calls.clear();
    // the kernel holds 8, whoever asks
    let snap = dump(&k);
    let e = k.start("alice", "a4", "p", "{}", IN).unwrap_err();
    assert_eq!(busy_text(e), "8 provider streams");
    let e = k.start("carol", "c0", "p", "{}", IN).unwrap_err();
    assert_eq!(busy_text(e), "8 provider streams");
    assert_eq!(dump(&k), snap);
    assert!(requests(&k).is_empty() && opens(&k).is_empty());
    // free one of bob's: alice still holds 4
    let bob = rec(&mut k, "s2").id;
    k.delete("bob", &bob).unwrap();
    let snap = dump(&k);
    k.host.calls.clear();
    let e = k.start("alice", "a4", "p", "{}", IN).unwrap_err();
    assert_eq!(busy_text(e), "4 provider streams per owner");
    assert_eq!(dump(&k), snap);
    assert!(requests(&k).is_empty() && opens(&k).is_empty());
    // carol takes the free slot; a retry of the refused key succeeds once a slot is free
    start(&mut k, "carol", "c0");
    let a0 = "s1";
    k.provider(a0, idle(Stop::Completed)).unwrap();
    k.cancel("alice", a0).unwrap();
    start(&mut k, "carol", "c1");
    // the say would open a stream and the kernel holds 8
    let snap = dump(&k);
    k.host.calls.clear();
    let e = k.say("alice", a0, "say1", "more").unwrap_err();
    assert_eq!(busy_text(e), "8 provider streams");
    assert_eq!(dump(&k), snap);
    assert!(requests(&k).is_empty() && opens(&k).is_empty());
    // nothing was kept under the key: after a slot is free the same call succeeds
    let carol = rec(&mut k, "s9").id;
    k.delete("carol", &carol).unwrap();
    k.say("alice", a0, "say1", "more").unwrap();
    assert_eq!(opens(&k).len(), 1);
    check(&mut k, &["alice", "bob", "carol"]);
}

#[test]
fn ac2_a_say_that_queues_or_uses_a_live_stream_is_never_refused() {
    let mut k = kernel();
    k.limits.owner_provider = 8;
    let ids: Vec<String> = (0..8)
        .map(|n| start(&mut k, "alice", &format!("k{n}")))
        .collect();
    // running: queues
    k.say("alice", &ids[0], "q", "queued").unwrap();
    // idle with its stream live: takes the turn
    k.provider(&ids[1], idle(Stop::Completed)).unwrap();
    k.say("alice", &ids[1], "r", "go").unwrap();
    assert_eq!(opens(&k).len(), 8);
    // the owner's cap, named
    k.limits.provider = 100;
    k.limits.owner_provider = 4;
    let e = k.start("alice", "x", "p", "{}", IN).unwrap_err();
    assert_eq!(busy_text(e), "4 provider streams per owner");
    // a setting names its own value
    k.limits.owner_provider = 9;
    k.limits.provider = 8;
    let e = k.start("alice", "x", "p", "{}", IN).unwrap_err();
    assert_eq!(busy_text(e), "8 provider streams");
}

#[test]
fn ac3_items_go_by_stream_id() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.host.calls.clear();
    // an unknown id changes nothing
    k.state.reset_counts();
    let snap = dump(&k);
    let c = k.provider_item("nope", idle(Stop::Completed)).unwrap();
    assert!(c.pushes.is_empty());
    assert_eq!(dump(&k), snap);
    assert!(k.host.calls.is_empty());
    assert_eq!(k.state.get_keys(), ["stream/nope"]);
    assert_eq!(k.state.counts().puts, 0);
    // a known id acts on its session
    let c = k.provider_item("t1", idle(Stop::Completed)).unwrap();
    assert_eq!(c.pushes.len(), 1);
    assert_eq!(c.pushes[0].session, id);
    assert!(matches!(rec(&mut k, &id).turn, Turn::Idle { .. }));
}

#[test]
fn ac4_counts_follow_opens_ends_deletes_and_cancels() {
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "alice", "k2");
    let c = start(&mut k, "bob", "k3");
    let d = start(&mut k, "bob", "k4");
    check(&mut k, &["alice", "bob"]);
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    check(&mut k, &["alice", "bob"]);
    k.cancel("alice", &b).unwrap();
    check(&mut k, &["alice", "bob"]);
    k.delete("bob", &c).unwrap();
    check(&mut k, &["alice", "bob"]);
    // a say reopens
    k.provider(&a, idle(Stop::Completed)).unwrap();
    k.say("alice", &a, "s1", "x").unwrap();
    assert_eq!(opens(&k).len(), 5);
    check(&mut k, &["alice", "bob"]);
    k.delete("bob", &d).unwrap();
    k.delete("alice", &a).unwrap();
    k.delete("alice", &b).unwrap();
    check(&mut k, &["alice", "bob"]);
    assert_eq!(Slots::load(&mut k.state).provider, 0);
}

#[test]
fn ac5_ended_frees_the_slot_and_keys() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, ProviderEvent::Out(adapters::Out::default()))
        .unwrap();
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert!(k.state.get("stream/t1").is_none());
    assert!(rec(&mut k, &id).streams.is_empty());
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    let u = usage(&mut k, "alice");
    assert_eq!(u.provider_at(1), 0);
    // its later items find no key
    k.state.reset_counts();
    k.host.calls.clear();
    let c = k.provider_item("t1", idle(Stop::Completed)).unwrap();
    assert!(c.pushes.is_empty());
    assert_eq!(k.state.counts().puts, 0);
    assert!(k.host.calls.is_empty());
}

#[test]
fn ac5_cancel_releases_at_once_and_delete_too() {
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "alice", "k2");
    k.cancel("alice", &a).unwrap();
    assert!(requests(&k).contains(&Request::Cancel { session: a.clone() }));
    assert!(k.state.get("stream/t1").is_none());
    assert!(rec(&mut k, &a).streams.is_empty());
    assert_eq!(Slots::load(&mut k.state).provider, 1);
    // the stream's later `ended` finds no key
    let snap = dump(&k);
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert_eq!(dump(&k), snap);
    k.delete("alice", &b).unwrap();
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    assert_eq!(usage(&mut k, "alice").provider_at(1), 0);
    check(&mut k, &["alice"]);
}

#[test]
fn ac6_a_say_after_ended_cancel_and_restart_resumes() {
    // after Ended
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, idle(Stop::Completed)).unwrap();
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    k.say("alice", &id, "s1", "x").unwrap();
    assert!(last_resume(&k));
    assert_eq!(opens(&k), ["t1", "t2"]);
    assert_eq!(rec(&mut k, &id).streams, ["t2"]);
    // a turn on the live stream does not resume
    k.provider(&id, idle(Stop::Completed)).unwrap();
    k.say("alice", &id, "s2", "y").unwrap();
    assert!(!last_resume(&k));
    assert_eq!(opens(&k).len(), 2);
    // after cancel
    k.cancel("alice", &id).unwrap();
    k.say("alice", &id, "s3", "z").unwrap();
    assert!(last_resume(&k));
    // after a restart: an idle session of an older boot
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, idle(Stop::Completed)).unwrap();
    k.host.generation = 2;
    k.say("alice", &id, "s1", "x").unwrap();
    assert!(last_resume(&k));
    let r = rec(&mut k, &id);
    assert_eq!(r.streams, ["t2"]);
    assert!(k.state.get("stream/t1").is_none());
    assert_eq!(Slots::load(&mut k.state).provider, 1);
    assert_eq!(usage(&mut k, "alice").provider_at(2), 1);
    check(&mut k, &["alice"]);
}

#[test]
fn ac6_persist_false_and_resume_lost_start_fresh_and_unknown_resumes() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "np", "{}", IN).unwrap().value;
    k.provider(&id, idle(Stop::Completed)).unwrap();
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    k.say("alice", &id, "s1", "x").unwrap();
    assert!(!last_resume(&k));
    assert_eq!(opens(&k).len(), 2);
    // resume-lost
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, ProviderEvent::Turn(Turn::ResumeLost))
        .unwrap();
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    k.say("alice", &id, "s1", "x").unwrap();
    assert!(!last_resume(&k));
    // unknown (R5)
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.host.generation = 2;
    k.say("alice", &id, "s1", "x").unwrap();
    assert!(last_resume(&k));
    // unknown with `persist: false`: nothing to resume, a fresh session
    let mut k = kernel();
    let id = k.start("alice", "k1", "np", "{}", IN).unwrap().value;
    k.host.generation = 2;
    k.say("alice", &id, "s1", "x").unwrap();
    assert_eq!(rec(&mut k, &id).streams, ["t2"]);
    assert!(!last_resume(&k));
}

#[test]
fn ac6_an_interrupt_replacement_without_a_stream_opens_one_and_resumes() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.cancel("alice", &id).unwrap();
    k.interrupt("alice", &id, Some("again")).unwrap();
    assert!(last_resume(&k));
    assert_eq!(rec(&mut k, &id).streams, ["t2"]);
    check(&mut k, &["alice"]);
    // on a live stream: no open, no resume
    k.interrupt("alice", &id, Some("again")).unwrap();
    assert!(!last_resume(&k));
    assert_eq!(opens(&k).len(), 2);
}

#[test]
fn ac7_a_restart_decrements_no_count() {
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "alice", "k2");
    k.host.generation = 2;
    k.list("alice", None, 10).unwrap();
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    // the owner's count is of the older boot, so it reads zero, and is stored as it was
    let u = usage(&mut k, "alice");
    assert_eq!((u.provider, u.slots_boot), (2, 1));
    assert_eq!(u.provider_at(2), 0);
    k.timer("restart");
    k.timer("restart");
    assert!(k.state.get("stream/t1").is_none());
    assert!(k.state.get("stream/t2").is_none());
    assert!(rec(&mut k, &a).streams.is_empty());
    let u = usage(&mut k, "alice");
    assert_eq!((u.provider, u.slots_boot), (2, 1));
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    // a stale record's delete frees nothing either
    k.delete("alice", &b).unwrap();
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    assert_eq!(usage(&mut k, "alice").provider_at(2), 0);
}

#[test]
fn ac8_bounds() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    // an item that changes nothing about streams: one key more than before (the stream key)
    k.state.reset_counts();
    k.provider_item("t1", ProviderEvent::Out(adapters::Out::default()))
        .unwrap();
    assert_eq!(
        k.state.get_keys(),
        [
            "stream/t1",
            &format!("session/{id}"),
            "quota/alice",
            "quota"
        ]
    );
    assert_eq!(k.state.counts().puts, 0);
}

/// A state that logs each key written.
#[derive(Default)]
struct Spy {
    inner: MemState,
    puts: Vec<String>,
}

impl State for Spy {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.inner.get(key)
    }
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        self.puts.push(key.to_string());
        self.inner.put(key, value)
    }
    fn delete(&mut self, key: &str) {
        self.inner.delete(key)
    }
}

#[test]
fn ac8_start_and_an_opening_say_write_slots_and_the_owner_account_once() {
    let mut st = Spy::default();
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
    let mut k = Kernel::new(st, host);
    let once = |k: &Kernel<Spy, FakeHost>| {
        for key in ["slots", "quota/alice", "stream/t1"] {
            let n = k.state.puts.iter().filter(|x| *x == key).count();
            assert!(n <= 1, "{key} written {n} times");
        }
        assert_eq!(k.state.puts.iter().filter(|x| *x == "slots").count(), 1);
        assert_eq!(
            k.state.puts.iter().filter(|x| *x == "quota/alice").count(),
            1
        );
    };
    k.state.puts.clear();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    once(&k);
    k.provider(&id, idle(Stop::Completed)).unwrap();
    k.cancel("alice", &id).unwrap();
    k.state.puts.clear();
    k.say("alice", &id, "s1", "x").unwrap();
    assert_eq!(k.host.next_stream, 2);
    for key in ["slots", "quota/alice"] {
        assert_eq!(
            k.state.puts.iter().filter(|x| *x == key).count(),
            1,
            "{key}"
        );
    }
}
