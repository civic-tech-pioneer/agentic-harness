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
    for who in ["alice", "bob"] {
        crate::fixture_access::put(&mut st, &format!("access/{who}"), br#"["p"]"#.to_vec())
            .unwrap();
    }
    // The host is at boot 1 and the kernel has seen it: no restart pass starts (R1).
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

fn started(k: &mut K, owner: &str, key: &str) -> String {
    crate::fixture_access::put(
        &mut k.state,
        &format!("access/{owner}"),
        br#"["p"]"#.to_vec(),
    )
    .unwrap();
    let id = k.start(owner, key, "p", "{}", IN).unwrap().value;
    k.attach(owner, &id, 0).unwrap();
    id
}

/// The session's workspace is `/w`, so an `Edit` of `/w/a.rs` has a reach (9.6).
fn in_workspace(k: &mut K, id: &str) {
    let mut r = Record::load(&mut k.state, id).unwrap();
    r.folder = Some("/w".into());
    r.save(&mut k.state).unwrap();
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
                "input": {"file_path": "/w/a.rs"}, "path": "/w/a.rs", "expires-at": expires}),
        }),
        ..Out::default()
    })
}

fn resolved(id: &str, by: &str, outcome: Option<&str>) -> ProviderEvent {
    ProviderEvent::Out(Out {
        resolved: Some(agent_kernel::adapters::Resolved {
            id: id.into(),
            by: by.into(),
            outcome: outcome.map(str::to_string),
            direct: true,
            raw: json!({"type": "ask-resolved", "id": id, "by": by}),
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
    // S35 AC2: the provider's `start` carries the input, so no turn request goes out.
    assert!(requests(&k).is_empty());
    assert!(k.host.calls.iter().any(|c| matches!(
        c,
        HostCall::StartProvider { input, .. } if input == IN
    )));
    assert_eq!(
        c.pushes
            .iter()
            .map(|p| p.value.clone())
            .filter(|v| v["type"] != "entry")
            .collect::<Vec<_>>(),
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
        c.pushes.iter().map(|p| p.value.clone()).collect::<Vec<_>>(),
        [json!({"type": "queue", "inputs": [{"n": 1, "input": "one"}]})]
    );
    assert_eq!(k.say("alice", &id, "b", "two").unwrap().value, 2);
    // S35: the start sent no turn request, and queueing sends none.
    assert_eq!(requests(&k).len(), 0);
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
        c.pushes.iter().map(|p| p.value.clone()).collect::<Vec<_>>(),
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
        c.pushes
            .iter()
            .map(|p| p.value.clone())
            .filter(|v| v["type"] != "entry")
            .collect::<Vec<_>>(),
        [
            json!({"type": "turn", "state": {"state": "waiting", "asks": ["a1"], "expires-at": 9000}})
        ]
    );
    let c2 = k.provider(&id, ask("a2", "ask", 7000)).unwrap();
    assert_eq!(
        c2.pushes
            .iter()
            .find(|p| p.value["type"] == "turn")
            .unwrap()
            .value["state"],
        json!({"state": "waiting", "asks": ["a1", "a2"], "expires-at": 7000})
    );
    // a check opens but does not change the turn
    let c3 = k.provider(&id, ask("c1", "check", 1)).unwrap();
    // its `ask` entry goes out as an `entry` item and no turn item follows (S35-fix1)
    assert!(c3.pushes.iter().all(|p| p.value["type"] == "entry"));
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
            stream: "t1".into(),
            direct: true,
            input: "next".into(),
            resume: false
        }
    );
    // each `turn` entry goes out before its turn item (S35-fix1)
    assert_eq!(c.pushes.len(), 6);
    assert_eq!(c.pushes[0].value["type"], "entry");
    assert_eq!(c.pushes[1].value["state"]["state"], "idle");
    assert_eq!(c.pushes[2].value["entry"]["kind"], "message");
    assert_eq!(c.pushes[3].value["type"], "entry");
    assert_eq!(c.pushes[4].value["state"]["state"], "running");
    assert_eq!(c.pushes[5].value, json!({"type": "queue", "inputs": []}));
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
        c.pushes
            .iter()
            .map(|p| p.value.clone())
            .filter(|v| v["type"] != "entry")
            .collect::<Vec<_>>(),
        [
            json!({"type": "turn", "state": {"state": "idle", "stop": {"kind": "completed"}, "seconds": 0}})
        ]
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
        c.pushes[0].value["state"]["activity"],
        json!({"kind": "compacting"})
    );
    assert_eq!(k.state.counts().puts, 0);
}

#[test]
fn ac5_active_index_follows_the_turn() {
    let mut k = kernel();
    // no owner stream attached: the counted writes are the start's own
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p"]"#.to_vec()).unwrap();
    k.state.reset_counts();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
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
    in_workspace(&mut k, &id);
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
            stream: "t1".into(),
            direct: true,
            ask: "a1".into(),
            decision: json!({"allow": true})
        }
    );
    let r = rec(&mut k, &id);
    let es = entries(&mut k, "alice", &id);
    let at = es.iter().find(|e| e["kind"] == "allow").unwrap();
    let i = at["i"].as_u64().unwrap().to_string();
    assert_eq!(
        at["body"],
        json!({"id": i, "tool": "Edit", "key": "/w/**", "reach": "Edit(/w/**)", "given": true, "by": "owner"})
    );
    assert_eq!(
        r.allows,
        [Allow {
            id: i,
            tool: "Edit".into(),
            key: "/w/**".into()
        }]
    );
    assert_eq!(
        r.open.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
        ["a2", "c1"]
    );
    assert_eq!(
        c.pushes
            .iter()
            .find(|p| p.value["type"] == "turn")
            .unwrap()
            .value["state"],
        json!({"state": "waiting", "asks": ["a2"], "expires-at": 6000})
    );
    // the answer writes no decision entry: it is pending until the host resolves the ask
    let es = entries(&mut k, "alice", &id);
    assert!(es.iter().all(|e| e["body"].get("decision").is_none()));
    let p = &rec(&mut k, &id).pending_answers["a1"];
    assert_eq!(
        (p.outcome.as_str(), p.by.as_str(), p.for_.as_deref()),
        ("allow", "owner", Some("session"))
    );
    k.provider(&id, resolved("a1", "answered", Some("allow")))
        .unwrap();
    let es = entries(&mut k, "alice", &id);
    let dec = es.iter().rev().find(|e| e["kind"] == "ask").unwrap();
    assert_eq!(dec["body"]["id"], "a1");
    assert_eq!(
        dec["body"]["decision"],
        json!({"outcome": "allow", "by": "owner", "for": "session"})
    );
    assert_eq!(dec["body"]["toolCall"]["title"], "Edit");
    assert!(rec(&mut k, &id).pending_answers.is_empty());
    // the last ask: back to running
    let c = k
        .answer("alice", &id, "k2", "a2", r#"{"deny":true,"reason":"no"}"#)
        .unwrap();
    assert_eq!(
        c.pushes
            .iter()
            .find(|p| p.value["type"] == "turn")
            .unwrap()
            .value["state"]["state"],
        "running"
    );
    assert_eq!(
        rec(&mut k, &id).turn,
        Turn::Running {
            since: 1000,
            activity: None
        }
    );
    k.provider(&id, resolved("a2", "answered", Some("deny")))
        .unwrap();
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
    in_workspace(&mut k, &id);
    k.provider(&id, ask("a1", "ask", 5000)).unwrap();
    let mut r = rec(&mut k, &id);
    for i in 0..64 {
        r.add_allow(Allow {
            id: i.to_string(),
            tool: "Edit".into(),
            key: format!("/w{i}/**"),
        })
        .unwrap();
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
    // owner "a" key "b/c" and owner "a/b" key "c" are separate keys (AC2 of S21-fix2)
    let x = started(&mut k, "a", "b/c");
    let y = started(&mut k, "a/b", "c");
    assert_ne!(x, y);
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
        c.pushes.iter().map(|p| p.value.clone()).collect::<Vec<_>>(),
        [json!({"type": "queue", "inputs": [{"n": 1, "input": "later"}]})]
    );
    assert_eq!(rec(&mut k, &id).turn, Turn::Paused { until: Some(9000) });
    assert_eq!(requests(&k).len(), n0 + 1);
}

#[test]
fn ac2_say_on_idle_or_paused_with_a_full_queue_starts_the_oldest() {
    for turn in [completed(), Turn::Paused { until: None }] {
        let mut k = kernel();
        let id = started(&mut k, "alice", "k");
        for i in 0..32 {
            k.say("alice", &id, &format!("c{i}"), &format!("in{i}"))
                .unwrap();
        }
        let mut r = rec(&mut k, &id);
        r.turn = turn;
        r.save(&mut k.state).unwrap();
        let n0 = requests(&k).len();
        let c = k.say("alice", &id, "new", "fresh").unwrap();
        let q = k.queued("alice", &id).unwrap();
        assert_eq!(q.len(), 32);
        assert_eq!(q[0].input, "in1");
        assert_eq!(q[31].input, "fresh");
        assert_eq!(c.value, q[31].n);
        assert_eq!(requests(&k).len(), n0 + 1);
    }
}

#[test]
fn ac3_oldest_out_and_new_in_still_over_the_bytes_is_queue_full_and_writes_nothing() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.say("alice", &id, "s", "x").unwrap();
    for i in 0..7 {
        k.say("alice", &id, &format!("b{i}"), &"x".repeat(262_144))
            .unwrap();
    }
    k.say("alice", &id, "t", &"x".repeat(262_143)).unwrap();
    let mut r = rec(&mut k, &id);
    r.turn = Turn::Paused { until: None };
    r.save(&mut k.state).unwrap();
    let n0 = requests(&k).len();
    k.state.reset_counts();
    assert_eq!(
        k.say("alice", &id, "over", &"x".repeat(262_144))
            .unwrap_err(),
        KernelError::QueueFull
    );
    assert_eq!(k.state.counts().puts + k.state.counts().deletes, 0);
    assert_eq!(requests(&k).len(), n0);
    assert_eq!(k.queued("alice", &id).unwrap().len(), 9);
    assert_eq!(rec(&mut k, &id).turn, Turn::Paused { until: None });
    // no key was kept: the same key, once there is room, does the call
    for n in 2..=9 {
        k.unqueue("alice", &id, n).unwrap();
    }
    k.say("alice", &id, "over", &"x".repeat(262_144)).unwrap();
    assert_eq!(requests(&k).len(), n0 + 1);
}

fn interrupted() -> Turn {
    Turn::Idle {
        stop: Stop::Interrupted,
        seconds: None,
        tools: None,
        tokens: None,
    }
}

fn page0(k: &mut K) -> Vec<String> {
    active::read_page(&mut k.state, 0)
        .into_iter()
        .map(|e| e.id)
        .collect()
}

#[test]
fn ac1_interrupt_without_a_replacement_ends_an_active_turn() {
    for t in [
        None,
        Some(Turn::Waiting {
            asks: vec!["a1".into()],
            expires_at: None,
        }),
        Some(Turn::Paused { until: Some(5000) }),
    ] {
        let mut k = kernel();
        let id = started(&mut k, "alice", "k");
        super::initialized(&mut k, "t1");
        k.provider(&id, ask("a1", "ask", 0)).unwrap();
        k.say("alice", &id, "q", "later").unwrap();
        if let Some(t) = t {
            let mut r = rec(&mut k, &id);
            r.turn = t;
            r.save(&mut k.state).unwrap();
        }
        let n = entries(&mut k, "alice", &id).len();
        let sent = requests(&k).len();
        let c = k.interrupt("alice", &id, None).unwrap();
        let r = rec(&mut k, &id);
        assert_eq!(r.turn, interrupted());
        assert_eq!(r.open.len(), 1, "open asks are left to the host");
        assert_eq!(r.active_page, None);
        assert!(page0(&mut k).is_empty());
        assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
        let es = entries(&mut k, "alice", &id);
        assert_eq!(es.len(), n + 1);
        assert_eq!(es[n]["kind"], "turn");
        assert_eq!(
            es[n]["body"],
            json!({"state": "idle", "stop": {"kind": "interrupted"}})
        );
        assert_eq!(c.pushes.len(), 2);
        assert_eq!(c.pushes[0].value["type"], "entry");
        assert_eq!(c.pushes[1].value["type"], "turn");
        assert_eq!(
            requests(&k)[sent..],
            [Request::Interrupt {
                session: id.clone(),
                stream: "t1".into(),
                direct: true
            }]
        );
    }
}

#[test]
fn ac1_interrupt_on_an_inactive_session_changes_nothing() {
    for t in [completed(), Turn::Unknown, Turn::ResumeLost] {
        let mut k = kernel();
        let id = started(&mut k, "alice", "k");
        let mut r = rec(&mut k, &id);
        r.turn = t;
        r.save(&mut k.state).unwrap();
        let before = k.state.get(&Record::key_of(&id));
        let sent = requests(&k);
        let c = k.interrupt("alice", &id, None).unwrap();
        assert!(c.pushes.is_empty());
        assert_eq!(k.state.get(&Record::key_of(&id)), before);
        assert_eq!(requests(&k), sent);
    }
}

#[test]
fn ac1_interrupt_with_a_replacement_starts_it_and_the_session_stays_active() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    super::initialized(&mut k, "t1");
    k.say("alice", &id, "q", "later").unwrap();
    let sent = requests(&k).len();
    let c = k.interrupt("alice", &id, Some("instead")).unwrap();
    let r = rec(&mut k, &id);
    assert!(matches!(r.turn, Turn::Running { .. }));
    assert_eq!(r.active_page, Some(0));
    assert_eq!(page0(&mut k), std::slice::from_ref(&id));
    assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
    let kinds: Vec<(String, String)> = entries(&mut k, "alice", &id)
        .iter()
        .skip(2)
        .map(|e| (e["kind"].to_string(), e["body"]["state"].to_string()))
        .collect();
    assert_eq!(kinds.len(), 3);
    assert_eq!(kinds[0].1, "\"idle\"");
    assert_eq!(kinds[1].0, "\"message\"");
    assert_eq!(kinds[2].1, "\"running\"");
    // two turn entries and the two turn items (S35-fix1)
    assert_eq!(c.pushes.len(), 5);
    assert_eq!(
        requests(&k)[sent..],
        [
            Request::Interrupt {
                session: id.clone(),
                stream: "t1".into(),
                direct: true
            },
            Request::Turn {
                session: id.clone(),
                stream: "t1".into(),
                direct: true,
                input: "instead".into(),
                resume: false
            }
        ]
    );
    // On an idle session the replacement starts a turn and the session joins `active/`.
    k.provider(&id, ProviderEvent::Turn(completed())).unwrap();
    let mut r = rec(&mut k, &id);
    r.turn = completed();
    r.save(&mut k.state).unwrap();
    k.interrupt("alice", &id, Some("again")).unwrap();
    assert_eq!(rec(&mut k, &id).active_page, Some(0));
}

#[test]
fn ac1_interrupt_refusals_write_nothing() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let big = "x".repeat(262_145);
    let before = snapshot(&mut k, &[&Record::key_of(&id), "active/0", "slots"]);
    assert_eq!(
        k.interrupt("alice", &id, Some(&big)).unwrap_err(),
        KernelError::TooLarge(262_144)
    );
    assert_eq!(
        k.interrupt("bob", &id, None).unwrap_err(),
        KernelError::NotOwner
    );
    assert_eq!(
        k.interrupt("alice", "nope", None).unwrap_err(),
        KernelError::UnknownSession("nope".into())
    );
    assert_eq!(
        snapshot(&mut k, &[&Record::key_of(&id), "active/0", "slots"]),
        before
    );
}

#[test]
fn ac2_cancel_closes_asks_ends_the_turn_and_keeps_the_queue() {
    for t in [
        None,
        Some(completed()),
        Some(Turn::Unknown),
        Some(Turn::ResumeLost),
    ] {
        let mut k = kernel();
        let id = started(&mut k, "alice", "k");
        k.provider(&id, ask("a1", "ask", 0)).unwrap();
        k.provider(&id, ask("a2", "ask", 0)).unwrap();
        k.answer("alice", &id, "ka", "a2", r#"{"allow":true}"#)
            .unwrap();
        k.say("alice", &id, "q", "later").unwrap();
        let inactive = t.is_some();
        if let Some(t) = t {
            let mut r = rec(&mut k, &id);
            r.turn = t.clone();
            if inactive {
                r.active_page = None;
            }
            r.save(&mut k.state).unwrap();
        }
        let sent = requests(&k).len();
        k.cancel("alice", &id).unwrap();
        let r = rec(&mut k, &id);
        assert!(r.open.is_empty() && r.pending_answers.is_empty());
        let es = entries(&mut k, "alice", &id);
        let decided: Vec<&Value> = es
            .iter()
            .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
            .collect();
        assert_eq!(decided.len(), 2);
        assert_eq!(r.active_page, None);
        assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
        assert_eq!(
            requests(&k)[sent..],
            [Request::Cancel {
                session: id.clone(),
                stream: "t1".into()
            }]
        );
        if inactive {
            assert_ne!(r.turn, interrupted());
        } else {
            assert_eq!(r.turn, interrupted());
            assert!(page0(&mut k).is_empty());
        }
    }
}

#[test]
fn ac2_cancel_is_the_owners_and_not_refused_for_access() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    assert_eq!(k.cancel("bob", &id).unwrap_err(), KernelError::NotOwner);
    k.state.delete("access/alice");
    k.cancel("alice", &id).unwrap();
}

#[test]
fn ac4_an_openai_profile_without_a_model_is_refused() {
    let mut k = kernel();
    k.state
        .put("profile/o", br#"{"backend":"openai"}"#.to_vec())
        .unwrap();
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p","o"]"#.to_vec()).unwrap();
    let before = k.state.len();
    let KernelError::Refused(rs) = k.start("alice", "k", "o", "{}", IN).unwrap_err() else {
        panic!("not refused")
    };
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "model");
    assert_eq!(k.state.len(), before);
    // A Claude profile without `model` still starts.
    k.start("alice", "k2", "p", "{}", IN).unwrap();
}

#[test]
fn ac1_interrupt_with_a_replacement_on_an_inactive_session_only_starts_the_turn() {
    for (t, resume) in [
        (completed(), false),
        (Turn::Unknown, true),
        (Turn::ResumeLost, false),
    ] {
        let mut k = kernel();
        let id = started(&mut k, "alice", "k");
        k.provider(&id, idle()).unwrap();
        assert!(page0(&mut k).is_empty());
        let mut r = rec(&mut k, &id);
        r.turn = t;
        r.save(&mut k.state).unwrap();
        let sent = requests(&k).len();
        k.interrupt("alice", &id, Some("instead")).unwrap();
        // No turn to end: no `Request::Interrupt`; `unknown` resumes as R5 Continue does.
        assert_eq!(
            requests(&k)[sent..],
            [Request::Turn {
                session: id.clone(),
                stream: "t1".into(),
                direct: true,
                input: "instead".into(),
                resume
            }]
        );
        assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
        assert_eq!(page0(&mut k), std::slice::from_ref(&id));
    }
}

#[test]
fn ac1_ac2_interrupt_and_cancel_do_the_r6_step_first() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.host.generation = 2;
    let sent = requests(&k).len();
    // R6 makes the running turn `unknown`: nothing left to interrupt.
    k.interrupt("alice", &id, None).unwrap();
    assert_eq!(rec(&mut k, &id).turn, Turn::Unknown);
    assert_eq!(requests(&k).len(), sent);

    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.provider(&id, ask("a1", "ask", 0)).unwrap();
    k.host.generation = 2;
    let sent = requests(&k).len();
    k.cancel("alice", &id).unwrap();
    let r = rec(&mut k, &id);
    assert_eq!(r.turn, Turn::Unknown);
    let es = entries(&mut k, "alice", &id);
    let reasons: Vec<&Value> = es
        .iter()
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .map(|e| &e["body"]["decision"]["reason"])
        .collect();
    assert_eq!(reasons, [&json!("restart")]);
    // The restart step released the stream: the cancel names none.
    assert_eq!(
        requests(&k)[sent..],
        [Request::Cancel {
            session: id,
            stream: String::new()
        }]
    );
}

/// S35-fix1 AC2, AC3: an ask entry, its decision when the host resolves it and a `say`'s `turn` entry go
/// out live as `entry` items with the index `read` gives them.
#[test]
fn s35_fix1_an_ask_its_decision_and_a_say_go_out_as_entry_items() {
    let live = |c: &Called<()>| -> Vec<(String, u64)> {
        c.pushes
            .iter()
            .filter(|p| p.value["type"] == "entry")
            .map(|p| {
                let e = &p.value["entry"];
                (
                    e["kind"].as_str().unwrap().to_string(),
                    e["i"].as_u64().unwrap(),
                )
            })
            .collect()
    };
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let c = k.provider(&id, ask("a1", "ask", 9000)).unwrap();
    let es = entries(&mut k, "alice", &id);
    let i = es.iter().find(|e| e["kind"] == "ask").unwrap()["i"]
        .as_u64()
        .unwrap();
    // the ask entry, then the `waiting` turn entry
    assert_eq!(live(&c), [("ask".into(), i), ("turn".into(), i + 1)]);
    let c = k.provider(&id, resolved("a1", "expired", None)).unwrap();
    let es = entries(&mut k, "alice", &id);
    let d = es
        .iter()
        .find(|e| e["kind"] == "ask" && e["i"] != i)
        .unwrap();
    assert_eq!(d["body"]["decision"]["by"], "host");
    let got = live(&c);
    assert_eq!(got[0], ("ask".into(), d["i"].as_u64().unwrap()), "{got:?}");
    assert!(got.iter().all(|(_, n)| *n > i + 1), "{got:?}");
    // an adapter's entry that carries the decision of an open ask goes out live as well
    k.provider(&id, ask("a2", "ask", 9000)).unwrap();
    let mut decided = ask("a2", "ask", 9000);
    if let ProviderEvent::Out(Out { entry: Some(e), .. }) = &mut decided {
        e.body["decision"] = json!({"outcome": "allow", "by": "host"});
    }
    let c = k.provider(&id, decided).unwrap();
    let es = entries(&mut k, "alice", &id);
    let d = es
        .iter()
        .rfind(|e| e["kind"] == "ask" && e["body"]["decision"]["outcome"] == "allow")
        .unwrap();
    assert_eq!(live(&c)[0], ("ask".into(), d["i"].as_u64().unwrap()));
    // an idle turn: the say's user message and its `running` turn entry both are
    k.provider(&id, idle()).unwrap();
    let c = k.say("alice", &id, "k2", "x").unwrap();
    let es = entries(&mut k, "alice", &id);
    let n = es.last().unwrap()["i"].as_u64().unwrap();
    assert_eq!(es.last().unwrap()["kind"], "turn");
    assert_eq!(es[es.len() - 2]["kind"], "message");
    let got: Vec<(String, u64)> = c
        .pushes
        .iter()
        .filter(|p| p.value["type"] == "entry")
        .map(|p| {
            (
                p.value["entry"]["kind"].as_str().unwrap().to_string(),
                p.value["entry"]["i"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(got, [("message".into(), n - 1), ("turn".into(), n)]);
}

/// S35-fix1 AC1, AC2 (9.4): an entry over 256 KiB goes out live as its `part` entries, each with the
/// index `read` gives it, as replay pushes it.
#[test]
fn s35_fix1_a_large_ask_goes_out_live_as_its_parts() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let n0 = rec(&mut k, &id).meta().last;
    let mut big = ask("a1", "ask", 9000);
    if let ProviderEvent::Out(Out { entry: Some(e), .. }) = &mut big {
        e.body["input"] = json!({"file_path": "/w/a.rs", "content": "x".repeat(300 * 1024)});
    }
    let c = k.provider(&id, big).unwrap();
    let stored: Vec<Value> = k
        .read("alice", &id, n0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect();
    let live: Vec<Value> = c
        .pushes
        .iter()
        .filter(|p| p.value["type"] == "entry")
        .map(|p| p.value["entry"].clone())
        .collect();
    assert_eq!(live[0]["kind"], "part");
    assert_eq!(live, stored);
}
