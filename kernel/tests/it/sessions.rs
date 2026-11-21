//! Tests of the session calls on `MemState` with a fake host (TECHNICAL 2.3, 9.3, 16 case 67).

use agent_kernel::active::{self, Slots};
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    let host = FakeHost {
        boot: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn started(k: &mut K, owner: &str, key: &str) -> String {
    k.start(owner, key, "p", "{}", IN).unwrap().value
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
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

fn entries(k: &mut K, owner: &str, id: &str) -> Vec<Value> {
    k.read(owner, id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn snapshot(k: &mut K, keys: &[&str]) -> Vec<Option<Vec<u8>>> {
    keys.iter().map(|x| k.state.get(x)).collect()
}

fn ask(id: &str, kind: &str, expires: u64) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": kind, "toolCallId": "t", "toolCall": {"toolCallId": "t", "title": "Edit"},
                "input": {}, "expires-at": expires, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
}

fn idle() -> ProviderEvent {
    ProviderEvent::Out(Out {
        idle: true,
        ..Out::default()
    })
}

fn completed() -> Turn {
    Turn::default()
}

#[test]
fn ac1_start_makes_the_record_and_everything_it_names() {
    let mut k = kernel();
    let c = k.start("alice", "k1", "p", "{}", IN).unwrap();
    assert_eq!(c.value, "s1");
    let r = rec(&mut k, "s1");
    assert_eq!(
        (r.owner.as_str(), r.profile.as_str(), r.backend.as_str()),
        ("alice", "p", "claude")
    );
    assert_eq!(
        r.turn,
        Turn::Running {
            since: 1000,
            activity: None
        }
    );
    assert_eq!(r.active_page, Some(0));
    let page: Vec<String> = serde_json::from_slice(&k.state.get("owner/alice/0").unwrap()).unwrap();
    assert_eq!(page, ["s1"]);
    let es = entries(&mut k, "alice", "s1");
    assert_eq!(es.len(), 2);
    assert_eq!(es[0]["kind"], "message");
    assert_eq!(es[0]["body"]["role"], "user");
    assert_eq!(
        es[0]["body"]["content"],
        serde_json::from_str::<Value>(IN).unwrap()
    );
    assert_eq!(es[1]["kind"], "turn");
    assert_eq!(es[1]["body"], json!({"state": "running", "since": 1000}));
    assert_eq!(
        requests(&k),
        [Request::Turn {
            session: "s1".into(),
            input: IN.into(),
            resume: false
        }]
    );
    assert_eq!(
        c.pushes,
        [json!({"type": "turn", "state": {"state": "running", "since": 1000}})]
    );
    assert_eq!(active::read_page(&mut k.state, 0).len(), 1);
    assert_eq!(Slots::load(&mut k.state).active, [1, 0, 0, 0]);
}

#[test]
fn ac1_absent_or_refused_profile_writes_nothing() {
    let mut k = kernel();
    k.state
        .put("profile/bad", br#"{"tools":["Nope("]}"#.to_vec())
        .unwrap();
    k.state.reset_counts();
    let e = k.start("alice", "k", "none", "{}", IN).unwrap_err();
    let KernelError::Refused(rs) = e else {
        panic!()
    };
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "profile");
    assert!(matches!(
        k.start("alice", "k", "p", r#"{"mode":"dont-ask"}"#, IN),
        Err(KernelError::Refused(_))
    ));
    assert!(matches!(
        k.start("alice", "k", "bad", "{}", IN),
        Err(KernelError::Refused(_))
    ));
    assert_eq!(k.state.counts().puts, 0);
    assert!(requests(&k).is_empty());
}

#[test]
fn ac2_same_key_replays_and_changes_nothing() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k1");
    let keys = [
        "slots",
        "session/s1",
        "owner/alice/0",
        "queue/s1",
        "active/0",
        "key/alice/k1",
        "entries/s1/0",
    ];
    let before = snapshot(&mut k, &keys);
    let n = requests(&k).len();
    let again = k.start("alice", "k1", "p", "{}", "other").unwrap();
    assert_eq!(again.value, id);
    assert!(again.pushes.is_empty());
    assert_eq!(snapshot(&mut k, &keys), before);
    assert_eq!(requests(&k).len(), n);
    assert_eq!(k.host.next_id, 1);
    // say and answer
    let a = k.say("alice", &id, "k2", "x").unwrap();
    let before = snapshot(
        &mut k,
        &keys
            .iter()
            .chain(&["key/alice/k2"])
            .copied()
            .collect::<Vec<_>>(),
    );
    let b = k.say("alice", &id, "k2", "y").unwrap();
    assert_eq!(a.value, b.value);
    assert_eq!(
        snapshot(
            &mut k,
            &keys
                .iter()
                .chain(&["key/alice/k2"])
                .copied()
                .collect::<Vec<_>>()
        ),
        before
    );
    k.provider(&id, ask("a1", "ask", 5000)).unwrap();
    k.answer("alice", &id, "k3", "a1", r#"{"allow":true}"#)
        .unwrap();
    let n = requests(&k).len();
    let before = snapshot(&mut k, &["session/s1", "entries/s1/0"]);
    k.answer("alice", &id, "k3", "a1", r#"{"allow":true}"#)
        .unwrap();
    assert_eq!(snapshot(&mut k, &["session/s1", "entries/s1/0"]), before);
    assert_eq!(requests(&k).len(), n);
}

#[test]
fn ac2_long_key_and_other_owner_and_failed_calls_keep_no_key() {
    let mut k = kernel();
    let long = "k".repeat(257);
    assert_eq!(
        k.start("alice", &long, "p", "{}", IN).unwrap_err(),
        KernelError::TooLarge(256)
    );
    assert_eq!(
        k.say("alice", "s1", &long, "x").unwrap_err(),
        KernelError::TooLarge(256)
    );
    assert_eq!(
        k.answer("alice", "s1", &long, "a", "{}").unwrap_err(),
        KernelError::TooLarge(256)
    );
    assert!(k.start("alice", &"k".repeat(256), "p", "{}", IN).is_ok());
    let a = started(&mut k, "alice", "same");
    let b = started(&mut k, "bob", "same");
    assert_ne!(a, b);
    // a failed call stores no key: too-large input, then the retry succeeds
    let big = "x".repeat(262_145);
    assert_eq!(
        k.say("alice", &a, "kk", &big).unwrap_err(),
        KernelError::TooLarge(262_144)
    );
    assert!(k.state.get("key/alice/kk").is_none());
    // queue full, then room again
    for i in 0..32 {
        k.say("alice", &a, &format!("q{i}"), "x").unwrap();
    }
    assert_eq!(
        k.say("alice", &a, "kk", "x").unwrap_err(),
        KernelError::QueueFull
    );
    assert!(k.state.get("key/alice/kk").is_none());
    k.unqueue("alice", &a, 1).unwrap();
    assert!(k.say("alice", &a, "kk", "x").is_ok());
    // a refused start stores no key
    assert!(k.start("alice", "pk", "none", "{}", IN).is_err());
    assert!(k.state.get("key/alice/pk").is_none());
}

#[test]
fn ac3_say_starts_on_idle_unknown_resume_lost_and_queues_while_active() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    // running: queues
    let c = k.say("alice", &id, "a", "one").unwrap();
    assert_eq!(c.value, 1);
    assert_eq!(
        c.pushes,
        [json!({"type": "queue", "inputs": [{"n": 1, "input": "one"}]})]
    );
    assert_eq!(k.say("alice", &id, "b", "two").unwrap().value, 2);
    assert_eq!(requests(&k).len(), 1);
    assert_eq!(
        k.queued("alice", &id).unwrap(),
        [
            Queued {
                n: 1,
                input: "one".into()
            },
            Queued {
                n: 2,
                input: "two".into()
            }
        ]
    );
    let c = k.unqueue("alice", &id, 1).unwrap();
    assert_eq!(
        c.pushes,
        [json!({"type": "queue", "inputs": [{"n": 2, "input": "two"}]})]
    );
    assert_eq!(
        k.unqueue("alice", &id, 1).unwrap_err(),
        KernelError::UnknownInput(1)
    );
    k.unqueue("alice", &id, 2).unwrap();
    for t in [completed(), Turn::Unknown, Turn::ResumeLost] {
        let mut r = rec(&mut k, &id);
        let was = r.turn.is_active();
        r.turn = t;
        if was {
            let mut sl = Slots::load(&mut k.state);
            active::remove(&mut k.state, &mut sl, &id, 0).unwrap();
            r.active_page = None;
        }
        r.save(&mut k.state).unwrap();
        let n0 = requests(&k).len();
        let c = k.say("alice", &id, &format!("s{}", n0), "go").unwrap();
        assert_eq!(requests(&k).len(), n0 + 1);
        assert_eq!(
            rec(&mut k, &id).turn,
            Turn::Running {
                since: 1000,
                activity: None
            }
        );
        assert_eq!(c.value, rec(&mut k, &id).next_index - 2);
        k.provider(&id, idle()).unwrap();
    }
}

#[test]
fn ac3_queue_limits() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let big = "x".repeat(262_144);
    assert_eq!(
        k.say("alice", &id, "k8", &"x".repeat(262_145)).unwrap_err(),
        KernelError::TooLarge(262_144)
    );
    assert_eq!(
        k.start("alice", "k9", "p", "{}", &"x".repeat(262_145))
            .unwrap_err(),
        KernelError::TooLarge(262_144)
    );
    for i in 0..8 {
        k.say("alice", &id, &format!("b{i}"), &big).unwrap();
    }
    assert_eq!(
        k.say("alice", &id, "over", "x").unwrap_err(),
        KernelError::QueueFull
    );
    for i in 1..=8 {
        k.unqueue("alice", &id, i).unwrap();
    }
    for i in 0..32 {
        k.say("alice", &id, &format!("c{i}"), "x").unwrap();
    }
    assert_eq!(
        k.say("alice", &id, "over", "x").unwrap_err(),
        KernelError::QueueFull
    );
}

#[test]
fn ac4_ask_waits_and_turn_end_starts_the_next_queued_input() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let c = k.provider(&id, ask("a1", "ask", 9000)).unwrap();
    assert_eq!(
        c.pushes,
        [
            json!({"type": "turn", "state": {"state": "waiting", "asks": ["a1"], "expires-at": 9000}})
        ]
    );
    let c2 = k.provider(&id, ask("a2", "ask", 7000)).unwrap();
    assert_eq!(
        c2.pushes[0]["state"],
        json!({"state": "waiting", "asks": ["a1", "a2"], "expires-at": 7000})
    );
    // a check opens but does not change the turn
    let c3 = k.provider(&id, ask("c1", "check", 1)).unwrap();
    assert!(c3.pushes.is_empty());
    let r = rec(&mut k, &id);
    assert_eq!(
        r.open
            .iter()
            .map(|o| (o.id.as_str(), o.kind.as_str()))
            .collect::<Vec<_>>(),
        [("a1", "ask"), ("a2", "ask"), ("c1", "check")]
    );
    let es = entries(&mut k, "alice", &id);
    assert_eq!(es[r.open[0].i as usize - 1]["body"]["id"], "a1");
    // the turn ends with a queued input
    k.say("alice", &id, "q", "next").unwrap();
    let n0 = requests(&k).len();
    let c = k.provider(&id, ProviderEvent::Turn(completed())).unwrap();
    assert_eq!(requests(&k).len(), n0 + 1);
    assert_eq!(
        requests(&k).last().unwrap(),
        &Request::Turn {
            session: id.clone(),
            input: "next".into(),
            resume: false
        }
    );
    assert_eq!(c.pushes.len(), 3);
    assert_eq!(c.pushes[0]["state"]["state"], "idle");
    assert_eq!(c.pushes[1]["state"]["state"], "running");
    assert_eq!(c.pushes[2], json!({"type": "queue", "inputs": []}));
    let es = entries(&mut k, "alice", &id);
    let kinds: Vec<_> = es
        .iter()
        .rev()
        .take(3)
        .map(|e| e["kind"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(kinds, ["turn", "message", "turn"]);
    assert_eq!(k.queued("alice", &id).unwrap(), []);
    // idle: true ends the turn
    let c = k.provider(&id, idle()).unwrap();
    assert_eq!(
        c.pushes,
        [json!({"type": "turn", "state": {"state": "idle", "stop": {"kind": "completed"}}})]
    );
    // activity is live only: pushed, never stored
    k.say("alice", &id, "again", "go").unwrap();
    let t = Turn::Running {
        since: 1000,
        activity: Some(Activity::Compacting),
    };
    k.state.reset_counts();
    let c = k.provider(&id, ProviderEvent::Turn(t)).unwrap();
    assert_eq!(
        c.pushes[0]["state"]["activity"],
        json!({"kind": "compacting"})
    );
    assert_eq!(k.state.counts().puts, 0);
}

#[test]
fn ac5_active_index_follows_the_turn() {
    let mut k = kernel();
    k.state.reset_counts();
    let id = started(&mut k, "alice", "k");
    let c = k.state.counts();
    assert_eq!(
        k.state.get_keys().iter().filter(|x| *x == "slots").count(),
        1
    );
    assert_eq!(
        k.state
            .get_keys()
            .iter()
            .filter(|x| x.starts_with("active/"))
            .count(),
        1
    );
    assert!(c.puts >= 2);
    // among active states: no page read or written
    k.state.reset_counts();
    k.provider(&id, ask("a1", "ask", 5)).unwrap();
    assert!(
        k.state
            .get_keys()
            .iter()
            .all(|x| !x.starts_with("active/") && x != "slots"),
        "{:?}",
        k.state.get_keys()
    );
    // out: one get and one put of slots and the page
    k.state.reset_counts();
    k.provider(&id, ProviderEvent::Turn(completed())).unwrap();
    let keys = k.state.get_keys().to_vec();
    assert_eq!(keys.iter().filter(|x| *x == "slots").count(), 1);
    assert_eq!(keys.iter().filter(|x| *x == "active/0").count(), 1);
    assert_eq!(rec(&mut k, &id).active_page, None);
    assert_eq!(Slots::load(&mut k.state).active, [0; 4]);
    assert!(active::read_page(&mut k.state, 0).is_empty());
    // a turn that ends and starts the next input writes no page
    k.say("alice", &id, "s", "go").unwrap();
    k.say("alice", &id, "q", "next").unwrap();
    k.state.reset_counts();
    k.provider(&id, idle()).unwrap();
    assert!(k
        .state
        .get_keys()
        .iter()
        .all(|x| !x.starts_with("active/") && x != "slots"));
    assert_eq!(Slots::load(&mut k.state).active, [1, 0, 0, 0]);
}

#[test]
fn ac5_full_index_is_busy_and_writes_nothing() {
    let mut k = kernel();
    let first = started(&mut k, "alice", "k");
    k.provider(&first, ProviderEvent::Turn(completed()))
        .unwrap();
    let mut sl = Slots::load(&mut k.state);
    for i in 0..1024 {
        active::add(&mut k.state, &mut sl, &format!("x{i}"), 0).unwrap();
    }
    let keys = [
        "slots",
        "session/s1",
        "owner/alice/0",
        "key/alice/k2",
        "entries/s1/0",
        "active/0",
        "active/3",
    ];
    let before = snapshot(&mut k, &keys);
    let n = requests(&k).len();
    let e = k.start("alice", "k2", "p", "{}", IN).unwrap_err();
    assert!(
        matches!(&e, KernelError::Busy(m) if m.contains("1024")),
        "{e:?}"
    );
    assert!(matches!(
        k.say("alice", &first, "k2", "x"),
        Err(KernelError::Busy(_))
    ));
    assert_eq!(snapshot(&mut k, &keys), before);
    assert_eq!(requests(&k).len(), n);
    // a leaving session frees room
    let mut sl = Slots::load(&mut k.state);
    active::remove(&mut k.state, &mut sl, "x0", 0).unwrap();
    assert!(k.say("alice", &first, "k2", "x").is_ok());
}

#[test]
fn ac6_answer_shapes_and_effects() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.provider(&id, ask("a1", "ask", 5000)).unwrap();
    k.provider(&id, ask("a2", "ask", 6000)).unwrap();
    k.provider(&id, ask("c1", "check", 6000)).unwrap();
    for bad in [
        r#"{"none":true}"#,
        r#"{"deny":true,"for":"session"}"#,
        r#"{"allow":"yes"}"#,
        "{}",
        r#"{"allow":true,"deny":true}"#,
        r#"{"allow":true,"reason":"x"}"#,
        r#"{"allow":true,"for":"all"}"#,
        r#"{"allow":true,"for":"session","input":{}}"#,
        "[]",
        "nope",
    ] {
        let KernelError::Refused(rs) = k.answer("alice", &id, "kb", "a1", bad).unwrap_err() else {
            panic!("{bad}")
        };
        assert_eq!((rs.len(), rs[0].field.as_str()), (1, "decision"), "{bad}");
    }
    for bad_ask in ["nope", "c1"] {
        let KernelError::Refused(rs) = k
            .answer("alice", &id, "kb", bad_ask, r#"{"allow":true}"#)
            .unwrap_err()
        else {
            panic!()
        };
        assert_eq!(rs[0].field, "ask");
    }
    assert!(k.state.get("key/alice/kb").is_none());
    let n = requests(&k).len();
    let c = k
        .answer(
            "alice",
            &id,
            "k1",
            "a1",
            r#"{"allow":true,"for":"session"}"#,
        )
        .unwrap();
    assert_eq!(requests(&k).len(), n + 1);
    assert_eq!(
        requests(&k).last().unwrap(),
        &Request::Answer {
            session: id.clone(),
            ask: "a1".into(),
            decision: json!({"allow": true, "for": "session"})
        }
    );
    let r = rec(&mut k, &id);
    assert_eq!(r.allows, [json!("Edit(/w/**)")]);
    assert_eq!(
        r.open.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
        ["a2", "c1"]
    );
    assert_eq!(
        c.pushes[0]["state"],
        json!({"state": "waiting", "asks": ["a2"], "expires-at": 5000})
    );
    let es = entries(&mut k, "alice", &id);
    let dec = es.iter().rev().find(|e| e["kind"] == "ask").unwrap();
    assert_eq!(dec["body"]["id"], "a1");
    assert_eq!(
        dec["body"]["decision"],
        json!({"outcome": "allow", "by": "owner", "for": "session"})
    );
    assert_eq!(dec["body"]["toolCall"]["title"], "Edit");
    // the last ask: back to running
    let c = k
        .answer("alice", &id, "k2", "a2", r#"{"deny":true,"reason":"no"}"#)
        .unwrap();
    assert_eq!(c.pushes[0]["state"]["state"], "running");
    assert_eq!(
        rec(&mut k, &id).turn,
        Turn::Running {
            since: 1000,
            activity: None
        }
    );
    let es = entries(&mut k, "alice", &id);
    let dec = es.iter().rev().find(|e| e["kind"] == "ask").unwrap();
    assert_eq!(
        dec["body"]["decision"],
        json!({"outcome": "deny", "by": "owner", "reason": "no"})
    );
    // an answer to a decided ask
    assert!(matches!(
        k.answer("alice", &id, "k3", "a2", r#"{"allow":true}"#),
        Err(KernelError::Refused(_))
    ));
}

#[test]
fn ac6_allows_over_the_limit_refuse_and_keep_the_ask_open() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.provider(&id, ask("a1", "ask", 5000)).unwrap();
    let mut r = rec(&mut k, &id);
    for i in 0..64 {
        r.add_allow(json!(format!("Edit(/w{i}/**)"))).unwrap();
    }
    r.save(&mut k.state).unwrap();
    let keys = ["session/s1", "entries/s1/0", "key/alice/k1"];
    let before = snapshot(&mut k, &keys);
    let n = requests(&k).len();
    assert_eq!(
        k.answer(
            "alice",
            &id,
            "k1",
            "a1",
            r#"{"allow":true,"for":"session"}"#
        )
        .unwrap_err(),
        KernelError::TooLarge(65_536)
    );
    assert_eq!(snapshot(&mut k, &keys), before);
    assert_eq!(requests(&k).len(), n);
    assert_eq!(rec(&mut k, &id).open.len(), 1);
    assert!(k
        .answer("alice", &id, "k1", "a1", r#"{"allow":true}"#)
        .is_ok());
}

#[test]
fn ac7_owner_checks_rename_read_delete_list() {
    let mut k = kernel();
    let a = started(&mut k, "alice", "k");
    let b = started(&mut k, "bob", "k");
    k.say("alice", &a, "q", "queued").unwrap();
    let id = a.as_str();
    assert_eq!(
        k.say("bob", id, "x", "y").unwrap_err(),
        KernelError::NotOwner
    );
    assert_eq!(
        k.answer("bob", id, "x", "a", "{}").unwrap_err(),
        KernelError::NotOwner
    );
    assert_eq!(k.queued("bob", id).unwrap_err(), KernelError::NotOwner);
    assert_eq!(k.unqueue("bob", id, 1).unwrap_err(), KernelError::NotOwner);
    assert_eq!(
        k.read("bob", id, 0, None, 5).unwrap_err(),
        KernelError::NotOwner
    );
    assert_eq!(k.rename("bob", id, "t").unwrap_err(), KernelError::NotOwner);
    assert_eq!(k.delete("bob", id).unwrap_err(), KernelError::NotOwner);
    assert_eq!(
        k.say("alice", "zz", "x", "y").unwrap_err(),
        KernelError::UnknownSession("zz".into())
    );
    assert_eq!(
        k.delete("alice", "zz").unwrap_err(),
        KernelError::UnknownSession("zz".into())
    );
    // rename
    assert_eq!(
        k.rename("alice", id, &"t".repeat(257)).unwrap_err(),
        KernelError::TooLarge(256)
    );
    k.rename("alice", id, "title").unwrap();
    // read is the transcript's read
    let p = k.read("alice", id, 0, None, 1).unwrap();
    assert_eq!(p.entries.len(), 1);
    assert_eq!(p.last, 2);
    assert!(k.read("alice", id, 0, None, 0).is_err());
    // list: the caller's only, writes nothing
    k.state.reset_counts();
    let l = k.list("alice", None, 10).unwrap();
    assert_eq!(k.state.counts().puts + k.state.counts().deletes, 0);
    assert_eq!(l.len(), 1);
    assert_eq!(
        (l[0].id.as_str(), l[0].title.as_str(), l[0].profile.as_str()),
        ("s1", "title", "p")
    );
    assert_eq!(l[0].backend, "claude");
    assert_eq!(
        serde_json::from_str::<Value>(&l[0].turn).unwrap()["state"],
        "running"
    );
    assert_eq!(k.list("bob", None, 10).unwrap()[0].id, b);
    let c = started(&mut k, "alice", "k3");
    assert_eq!(k.list("alice", Some(id), 10).unwrap()[0].id, c);
    assert_eq!(k.list("alice", None, 1).unwrap().len(), 1);
    // delete: everything but the keys
    k.delete("alice", id).unwrap();
    for key in ["session/s1", "entries/s1/0", "queue/s1"] {
        assert!(k.state.get(key).is_none(), "{key}");
    }
    assert!(k.state.get("key/alice/k").is_some());
    assert!(k.state.get("key/alice/q").is_some());
    assert_eq!(
        k.list("alice", None, 10)
            .unwrap()
            .iter()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>(),
        std::slice::from_ref(&c)
    );
    assert_eq!(
        active::read_page(&mut k.state, 0)
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["s2", "s3"]
    );
    assert_eq!(Slots::load(&mut k.state).active, [2, 0, 0, 0]);
    // the key still replays after the delete
    assert_eq!(k.start("alice", "k", "p", "{}", IN).unwrap().value, "s1");
}

#[test]
fn ac8_say_reads_no_more_on_a_long_transcript_than_on_a_fresh_one() {
    let gets = |long: bool| {
        let mut k = kernel();
        let id = started(&mut k, "alice", "k");
        k.provider(&id, ProviderEvent::Turn(completed())).unwrap();
        if long {
            for _ in 0..10_000 {
                let mut r = rec(&mut k, &id);
                let mut m = r.meta();
                let e = NewEntry {
                    kind: "message".into(),
                    uuid: None,
                    body: json!({"role": "agent", "content": []}),
                };
                Transcript::new(&mut k.state, &id, &mut m)
                    .append(&e, 1)
                    .unwrap();
                r.set_meta(&m);
                r.save(&mut k.state).unwrap();
            }
        }
        k.state.reset_counts();
        k.say("alice", &id, "k2", "x").unwrap();
        let keys = k.state.get_keys().to_vec();
        assert!(keys
            .iter()
            .all(|x| !x.starts_with("key/alice/") || x == "key/alice/k2"));
        let mut chunks: Vec<_> = keys.iter().filter(|x| x.starts_with("entries/")).collect();
        chunks.dedup();
        (k.state.counts().gets, chunks.len())
    };
    let (fresh, _) = gets(false);
    let (long, chunks_read) = gets(true);
    assert_eq!(chunks_read, 1, "only the last chunk is read");
    assert!(long <= fresh + 1, "{long} vs {fresh}");
}

#[test]
fn ac2_a_key_of_another_call_kind_or_owner_is_refused_and_kept() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.say("alice", &id, "s", "x").unwrap();
    k.provider(&id, ask("a1", "ask", 5000)).unwrap();
    k.answer("alice", &id, "a", "a1", r#"{"allow":true}"#)
        .unwrap();
    let keys = [
        "slots",
        "session/s1",
        "owner/alice/0",
        "queue/s1",
        "active/0",
        "key/alice/k",
        "key/alice/s",
        "key/alice/a",
        "entries/s1/0",
    ];
    let before = snapshot(&mut k, &keys);
    let n = requests(&k).len();
    let is_key = |e: KernelError| matches!(e, KernelError::Refused(r) if r.len() == 1 && r[0].field == "key");
    assert!(is_key(k.say("alice", &id, "k", "y").unwrap_err()));
    assert!(is_key(k.say("alice", &id, "a", "y").unwrap_err()));
    assert!(is_key(k.start("alice", "s", "p", "{}", IN).unwrap_err()));
    assert!(is_key(k.start("alice", "a", "p", "{}", IN).unwrap_err()));
    k.provider(&id, ask("a2", "ask", 5000)).unwrap();
    let before2 = snapshot(&mut k, &keys);
    assert!(is_key(
        k.answer("alice", &id, "k", "a2", r#"{"allow":true}"#)
            .unwrap_err()
    ));
    assert!(is_key(
        k.answer("alice", &id, "s", "a2", r#"{"allow":true}"#)
            .unwrap_err()
    ));
    assert_eq!(snapshot(&mut k, &keys), before2);
    assert_eq!(before[5..8], before2[5..8]);
    assert_eq!(requests(&k).len(), n);
    assert_eq!(k.host.next_id, 1);
    // the first results still replay
    assert_eq!(k.start("alice", "k", "p", "{}", IN).unwrap().value, id);
    assert_eq!(k.say("alice", &id, "s", "x").unwrap().value, 1);
    // owner "a" key "b/c" and owner "a/b" key "c" share one state key: no replay of another's
    let x = started(&mut k, "a", "b/c");
    let e = k.start("a/b", "c", "p", "{}", IN).unwrap_err();
    assert!(is_key(e));
    assert_eq!(k.start("a", "b/c", "p", "{}", IN).unwrap().value, x);
}

#[test]
fn ac3_say_resumes_a_pause_with_no_time_and_queues_on_a_timed_one() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let mut r = rec(&mut k, &id);
    r.turn = Turn::Paused { until: None };
    r.save(&mut k.state).unwrap();
    let listed = snapshot(&mut k, &["slots", "active/0"]);
    let n0 = requests(&k).len();
    k.host.now = 2000;
    k.state.reset_counts();
    let c = k.say("alice", &id, "s1", "go").unwrap();
    assert!(k.state.get_keys().iter().all(|x| !x.starts_with("active/")));
    assert_eq!(snapshot(&mut k, &["slots", "active/0"]), listed);
    assert_eq!(c.value, rec(&mut k, &id).next_index - 2);
    assert_eq!(
        rec(&mut k, &id).turn,
        Turn::Running {
            since: 2000,
            activity: None
        }
    );
    assert_eq!(requests(&k).len(), n0 + 1);
    assert!(k.queued("alice", &id).unwrap().is_empty());
    // paused until a time: the input waits in the queue
    let mut r = rec(&mut k, &id);
    r.turn = Turn::Paused { until: Some(9000) };
    r.save(&mut k.state).unwrap();
    let c = k.say("alice", &id, "s2", "later").unwrap();
    assert_eq!(c.value, 1);
    assert_eq!(
        c.pushes,
        [json!({"type": "queue", "inputs": [{"n": 1, "input": "later"}]})]
    );
    assert_eq!(rec(&mut k, &id).turn, Turn::Paused { until: Some(9000) });
    assert_eq!(requests(&k).len(), n0 + 1);
}
