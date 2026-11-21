//! S25-2: owners held to 9.3 "Quota" (16 cases 53, 59): the accounts, the pages, the caps, a running turn that stops.

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

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

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn dump(k: &K) -> BTreeMap<String, Vec<u8>> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn usage(k: &mut K, key: &str) -> Usage {
    Usage::load(&mut k.state, key)
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
            body: json!({"id": id, "kind": "ask", "toolCallId": id, "toolCall": {"toolCallId": id, "title": "Edit"},
                "input": {}, "expires-at": 9000, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
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

fn resolved(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        resolved: Some(agent_kernel::adapters::Resolved {
            id: id.into(),
            by: "expired".into(),
            outcome: None,
            direct: true,
            raw: json!({"type": "ask-resolved", "id": id, "by": "expired"}),
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

fn entry_lines(k: &mut K, id: &str) -> Vec<Value> {
    let r = rec(k, id);
    let mut out = Vec::new();
    for n in 0..r.chunks.len() {
        let v = k.state.get(&format!("entries/{id}/{n}")).unwrap();
        for l in String::from_utf8(v).unwrap().lines() {
            out.push(serde_json::from_str(l).unwrap());
        }
    }
    out
}

/// Puts a session key the kernel does not write yet (`stream/`, `live/`) and charges it as a call would.
fn forge(k: &mut K, owner: &str, id: &str, key: &str, len: usize) {
    forge_value(k, owner, id, key, vec![b'x'; len]);
}

/// `forge` with the value given; the key must be new.
fn forge_value(k: &mut K, owner: &str, id: &str, key: &str, value: Vec<u8>) {
    let len = value.len();
    k.state.put(key, value).unwrap();
    let mut n = (key.len() + len) as i64;
    let mut r = rec(k, id);
    let was = k.state.get(&format!("session/{id}")).unwrap().len() as i64;
    r.bytes += n as u64;
    if let Some(s) = key.strip_prefix("stream/") {
        r.streams.push(s.to_string());
    }
    r.save(&mut k.state).unwrap();
    n += k.state.get(&format!("session/{id}")).unwrap().len() as i64 - was;
    for acct in [format!("quota/{owner}"), "quota".to_string()] {
        let mut u = usage(k, &acct);
        u.bytes = (u.bytes as i64 + n) as u64;
        u.save(&mut k.state, &acct).unwrap();
    }
}

/// AC2/AC3: every account equals the bytes the keys hold, by scope.
fn check(k: &K) {
    let all = dump(k);
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    // `stream/<id>` belongs to the session whose record lists the stream.
    let mut streams: BTreeMap<String, String> = BTreeMap::new();
    for (key, v) in &all {
        if let Some(id) = key.strip_prefix("session/") {
            let r: Record = serde_json::from_slice(v).unwrap();
            for st in &r.streams {
                streams.insert(st.clone(), id.to_string());
            }
            owners.insert(id.to_string(), r.owner);
        }
    }
    let mut bytes: BTreeMap<String, u64> = BTreeMap::new();
    let mut session_bytes: BTreeMap<String, u64> = BTreeMap::new();
    for (key, v) in &all {
        let n = (key.len() + v.len()) as u64;
        let (head, rest) = key.split_once('/').unwrap_or((key, ""));
        match head {
            "session" => *bytes.entry(owners[rest].clone()).or_default() += n,
            "key" | "owner" => {
                let seg = rest.split('/').next().unwrap();
                *bytes.entry(seg.to_string()).or_default() += n;
            }
            "queue" | "entries" | "wire" | "loop" | "stream" | "live" => {
                let id = rest.split('/').next().unwrap();
                let id = if head == "stream" {
                    &streams[id][..]
                } else {
                    id
                };
                *bytes.entry(owners[id].clone()).or_default() += n;
                *session_bytes.entry(id.to_string()).or_default() += n;
            }
            _ => {}
        }
    }
    for (id, owner) in &owners {
        let r: Record = serde_json::from_slice(&all[&format!("session/{id}")]).unwrap();
        assert_eq!(
            r.bytes,
            session_bytes.get(id).copied().unwrap_or(0),
            "record bytes of {id}"
        );
        let _ = owner;
    }
    let mut total = Usage::default();
    for owner in ["alice", "bob"] {
        let u: Usage = all
            .get(&format!("quota/{owner}"))
            .map(|v| serde_json::from_slice(v).unwrap())
            .unwrap_or_default();
        assert_eq!(
            u.bytes,
            bytes.get(owner).copied().unwrap_or(0),
            "bytes of {owner}"
        );
        let live = owners.values().filter(|o| *o == owner).count() as u64;
        assert_eq!(u.sessions, live, "sessions of {owner}");
        total.bytes += u.bytes;
        total.sessions += u.sessions;
    }
    let a: Usage = all
        .get("quota")
        .map(|v| serde_json::from_slice(v).unwrap())
        .unwrap_or_default();
    assert_eq!((a.bytes, a.sessions), (total.bytes, total.sessions));
}

#[test]
fn ac1_the_kernel_holds_limits_and_a_metered_state() {
    let mut k = kernel();
    assert_eq!(k.limits, Limits::default());
    k.state.reset_counts();
    let _ = Slots::load(&mut k.state);
    assert_eq!(k.state.counts().gets, 1);
}

#[test]
fn ac2_the_accounts_equal_the_bytes_of_the_keys() {
    let mut k = kernel();
    check(&k);
    let a1 = start(&mut k, "alice", "k1");
    check(&k);
    let b1 = start(&mut k, "bob", "k1");
    check(&k);
    k.say("alice", &a1, "s1", "queued one").unwrap();
    k.say("bob", &b1, "s1", "queued bob").unwrap();
    check(&k);
    k.provider(&a1, message("hello")).unwrap();
    k.provider(&a1, ask("a1")).unwrap();
    k.provider(&b1, ask("b1")).unwrap();
    check(&k);
    k.answer(
        "alice",
        &a1,
        "an1",
        "a1",
        r#"{"allow":true,"for":"session"}"#,
    )
    .unwrap();
    k.answer("bob", &b1, "an1", "b1", r#"{"deny":true,"reason":"no"}"#)
        .unwrap();
    check(&k);
    k.rename("alice", &a1, "a title").unwrap();
    k.unqueue("bob", &b1, 1).unwrap();
    check(&k);
    k.provider(&b1, idle()).unwrap();
    k.interrupt("alice", &a1, Some("replacement")).unwrap();
    check(&k);
    k.interrupt("bob", &b1, None).unwrap();
    k.cancel("bob", &b1).unwrap();
    check(&k);
    // a restart step (stale stream and live keys go)
    forge(&mut k, "alice", &a1, "stream/x1", 3);
    forge(&mut k, "alice", &a1, &format!("live/{a1}"), 4);
    check(&k);
    k.host.boot = 2;
    k.list("bob", None, 1).unwrap();
    k.timer("restart");
    k.timer("restart");
    check(&k);
    k.read("alice", &a1, 0, None, 10).unwrap();
    k.delete("alice", &a1).unwrap();
    check(&k);
    k.delete("bob", &b1).unwrap();
    check(&k);
}

#[test]
fn ac2_a_stale_session_read_steps_and_charges() {
    let mut k = kernel();
    let a1 = start(&mut k, "alice", "k1");
    k.provider(&a1, ask("a1")).unwrap();
    forge(&mut k, "alice", &a1, "stream/x1", 3);
    k.host.boot = 2;
    k.read("alice", &a1, 0, None, 10).unwrap();
    check(&k);
    assert_eq!(usage(&mut k, "quota").sessions, 1);
}

#[test]
fn ac2_a_call_that_changes_no_charged_byte_writes_neither_account() {
    let mut k = kernel();
    let a1 = start(&mut k, "alice", "k1");
    k.say("alice", &a1, "s1", "again").unwrap();
    let snap = dump(&k);
    k.state.reset_counts();
    k.start("alice", "k1", "p", "{}", IN).unwrap();
    k.say("alice", &a1, "s1", "again").unwrap();
    k.read("alice", &a1, 0, None, 10).unwrap();
    k.list("alice", None, 10).unwrap();
    k.queued("alice", &a1).unwrap();
    assert_eq!(k.state.counts().puts, 0);
    assert!(k.start("alice", "k2", "nope", "{}", IN).is_err());
    assert_eq!(dump(&k), snap);
}

#[test]
fn ac3_the_record_names_its_page_and_bytes() {
    let r = Record::default();
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["owner-page"], 0);
    assert_eq!(v["bytes"], 0);
    let r: Record = serde_json::from_str(r#"{"id":"x"}"#).unwrap();
    assert_eq!((r.owner_page, r.bytes), (0, 0));
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    assert!(rec(&mut k, &a).bytes > 0);
}

#[test]
fn ac4_delete_frees_without_reading_the_chunks_case_59() {
    let mut k = kernel();
    let ids: Vec<String> = (0..3)
        .map(|n| start(&mut k, "alice", &format!("k{n}")))
        .collect();
    for (n, id) in ids.iter().enumerate() {
        k.say("alice", id, &format!("s{n}"), "more").unwrap();
        k.provider(id, message("some reply")).unwrap();
    }
    let b = start(&mut k, "bob", "k1");
    check(&k);
    for id in &ids {
        k.state.reset_counts();
        k.delete("alice", id).unwrap();
        assert!(
            !k.state.get_keys().iter().any(|x| x.starts_with("entries/")),
            "{:?}",
            k.state.get_keys()
        );
        check(&k);
    }
    let kept: u64 = dump(&k)
        .iter()
        .filter(|(key, _)| key.starts_with("key/alice/"))
        .map(|(key, v)| (key.len() + v.len()) as u64)
        .sum();
    assert!(kept > 0);
    let u = usage(&mut k, "quota/alice");
    assert_eq!((u.bytes, u.sessions), (kept, 0));
    assert!(dump(&k)
        .keys()
        .all(|x| !x.starts_with("entries/s1") && !x.starts_with("queue/s1")));
    let _ = b;
    let acct = usage(&mut k, "quota");
    assert_eq!(acct.sessions, 1);
}

#[test]
fn ac5_an_owner_holds_at_most_64_sessions() {
    let mut k = kernel();
    let ids: Vec<String> = (0..64)
        .map(|n| start(&mut k, "alice", &format!("k{n}")))
        .collect();
    let snap = dump(&k);
    assert!(matches!(
        k.start("alice", "k64", "p", "{}", IN),
        Err(KernelError::Quota)
    ));
    assert_eq!(dump(&k), snap);
    start(&mut k, "bob", "k0");
    k.delete("alice", &ids[10]).unwrap();
    start(&mut k, "alice", "k64");
    check(&k);
}

#[test]
fn ac5_a_cap_past_256_opens_more_owner_pages() {
    let mut k = kernel();
    k.limits.owner_sessions = 300;
    let ids: Vec<String> = (0..300)
        .map(|n| start(&mut k, "alice", &format!("k{n}")))
        .collect();
    assert_eq!(rec(&mut k, &ids[255]).owner_page, 0);
    assert_eq!(rec(&mut k, &ids[256]).owner_page, 1);
    assert_eq!(usage(&mut k, "quota/alice").pages, vec![256, 1 + 43]);
    let page1: Vec<String> =
        serde_json::from_slice(&k.state.get("owner/alice/1").unwrap()).unwrap();
    assert_eq!(page1[0], ids[256]);
    // list pages through all 300 in start order
    let mut seen: Vec<String> = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let page = k.list("alice", after.as_deref(), 100).unwrap();
        if page.is_empty() {
            break;
        }
        after = Some(page.last().unwrap().id.clone());
        seen.extend(page.into_iter().map(|s| s.id));
    }
    assert_eq!(seen, ids);
    assert!(matches!(
        k.start("alice", "k300", "p", "{}", IN),
        Err(KernelError::Quota)
    ));
    // delete of a page-1 session reads page 1 and not page 0
    k.state.reset_counts();
    k.delete("alice", &ids[299]).unwrap();
    let keys = k.state.get_keys().to_vec();
    assert!(keys.contains(&"owner/alice/1".to_string()), "{keys:?}");
    assert!(!keys.contains(&"owner/alice/0".to_string()), "{keys:?}");
    for id in &ids[256..299] {
        k.delete("alice", id).unwrap();
    }
    assert!(k.state.get("owner/alice/1").is_none());
    assert_eq!(usage(&mut k, "quota/alice").pages, vec![256, 0]);
    check(&k);
    // the next start fills the first page with room
    let n = start(&mut k, "alice", "again");
    assert_eq!(rec(&mut k, &n).owner_page, 1);
    assert_eq!(usage(&mut k, "quota/alice").pages, vec![256, 1]);
    k.delete("alice", &ids[0]).unwrap();
    let m = start(&mut k, "alice", "again2");
    assert_eq!(rec(&mut k, &m).owner_page, 0);
    check(&k);
}

/// Alice with a waiting session (open ask `a1`, one queued input) and an idle one.
fn busy_alice(k: &mut K) -> (String, String) {
    let s1 = start(k, "alice", "k1");
    k.provider(&s1, ask("a1")).unwrap();
    k.say("alice", &s1, "q1", "queued input").unwrap();
    let s2 = start(k, "alice", "k2");
    k.provider(&s2, idle()).unwrap();
    (s1, s2)
}

#[test]
fn ac6_an_owner_past_its_bytes_is_refused_new_work() {
    let mut k = kernel();
    let (s1, s2) = busy_alice(&mut k);
    let b1 = start(&mut k, "bob", "k1");
    k.limits.owner_bytes = usage(&mut k, "quota/alice").bytes;
    let q = |k: &mut K, r: Result<Called<()>, KernelError>| {
        let snap = dump(k);
        assert!(matches!(r, Err(KernelError::Quota)), "{r:?}");
        assert_eq!(dump(k), snap);
    };
    let snap = dump(&k);
    let r = k.start("alice", "k3", "p", "{}", IN).map(|_| Called {
        value: (),
        pushes: vec![],
    });
    q(&mut k, r);
    let r = k.say("alice", &s2, "x", "hello").map(|_| Called {
        value: (),
        pushes: vec![],
    });
    q(&mut k, r);
    let r = k.say("alice", &s1, "y", "hello").map(|_| Called {
        value: (),
        pushes: vec![],
    });
    q(&mut k, r);
    let r = k.interrupt("alice", &s1, Some("again"));
    q(&mut k, r);
    let r = k.rename("alice", &s1, "t").map(|_| Called {
        value: (),
        pushes: vec![],
    });
    q(&mut k, r);
    let r = k.answer("alice", &s1, "z", "a1", r#"{"allow":true,"for":"session"}"#);
    q(&mut k, r);
    assert_eq!(dump(&k), snap);
    // what still works
    k.read("alice", &s1, 0, None, 10).unwrap();
    assert_eq!(k.list("alice", None, 10).unwrap().len(), 2);
    assert_eq!(k.queued("alice", &s1).unwrap().len(), 1);
    k.unqueue("alice", &s1, 1).unwrap();
    k.answer("alice", &s1, "z2", "a1", r#"{"allow":true}"#)
        .unwrap();
    k.interrupt("alice", &s1, None).unwrap();
    k.cancel("alice", &s2).unwrap();
    // another owner is not affected
    k.say("bob", &b1, "x", "hello").unwrap();
    start(&mut k, "bob", "k2");
    k.delete("alice", &s1).unwrap();
    k.delete("alice", &s2).unwrap();
    start(&mut k, "alice", "k3");
    check(&k);
}

#[test]
fn ac7_the_account_past_its_bytes_refuses_both_owners() {
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "bob", "k1");
    k.limits.account_bytes = usage(&mut k, "quota").bytes;
    assert!(usage(&mut k, "quota/alice").bytes < k.limits.owner_bytes);
    let snap = dump(&k);
    assert!(matches!(
        k.start("alice", "k2", "p", "{}", IN),
        Err(KernelError::Quota)
    ));
    assert!(matches!(
        k.start("bob", "k2", "p", "{}", IN),
        Err(KernelError::Quota)
    ));
    assert!(matches!(
        k.say("alice", &a, "x", "hi"),
        Err(KernelError::Quota)
    ));
    assert!(matches!(k.rename("bob", &b, "t"), Err(KernelError::Quota)));
    assert_eq!(dump(&k), snap);
    k.delete("bob", &b).unwrap();
    k.delete("alice", &a).unwrap();
    start(&mut k, "alice", "k2");
}

#[test]
fn ac8_a_running_turn_ends_stopped_quota_when_past() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.say("alice", &s1, "q1", "queued input").unwrap();
    k.limits.owner_bytes = usage(&mut k, "quota/alice").bytes;
    let before = entry_lines(&mut k, &s1).len();
    let n = requests(&k).len();
    let c = k.provider(&s1, message("reply")).unwrap();
    let lines = entry_lines(&mut k, &s1);
    assert_eq!(lines.len(), before + 1);
    let last = lines.last().unwrap();
    assert_eq!(last["kind"], "turn");
    assert_eq!(
        last["body"],
        json!({"state": "idle", "stop": {"kind": "stopped", "limit": "quota"}})
    );
    assert!(lines
        .iter()
        .all(|l| l["kind"] != "message" || l["body"]["role"] == "user"));
    assert_eq!(c.pushes.len(), 1);
    assert_eq!(c.pushes[0].value["state"]["state"], "idle");
    let reqs = requests(&k);
    assert_eq!(reqs.len(), n + 1);
    assert_eq!(
        reqs[n],
        Request::Interrupt {
            session: s1.clone()
        }
    );
    let r = rec(&mut k, &s1);
    assert!(r.active_page.is_none());
    assert_eq!(Slots::load(&mut k.state).active, [0; 4]);
    assert_eq!(k.queued("alice", &s1).unwrap().len(), 1);
    // later non-control items store nothing and send nothing
    let snap = dump(&k);
    let c = k.provider(&s1, message("more")).unwrap();
    assert!(c.pushes.is_empty());
    assert_eq!(requests(&k).len(), n + 1);
    assert_eq!(dump(&k), snap);
    check(&k);
}

#[test]
fn ac8_a_waiting_turn_stops_too_and_a_new_ask_is_denied() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.provider(&s1, ask("a1")).unwrap();
    k.limits.owner_bytes = usage(&mut k, "quota/alice").bytes;
    let n = requests(&k).len();
    let before = entry_lines(&mut k, &s1).len();
    k.provider(&s1, ask("a2")).unwrap();
    let lines = entry_lines(&mut k, &s1);
    assert_eq!(lines.len(), before + 1, "only the turn entry");
    assert_eq!(lines.last().unwrap()["kind"], "turn");
    let reqs = requests(&k);
    assert_eq!(
        reqs[n],
        Request::Interrupt {
            session: s1.clone()
        }
    );
    assert_eq!(
        reqs[n + 1],
        Request::Answer {
            session: s1.clone(),
            ask: "a2".into(),
            decision: json!({"deny": true, "reason": "quota"})
        }
    );
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Idle { .. }));
    check(&k);
}

#[test]
fn ac8_an_idle_session_gets_a_denied_ask_and_no_turn_change() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.provider(&s1, idle()).unwrap();
    k.limits.account_bytes = usage(&mut k, "quota").bytes;
    let snap = dump(&k);
    k.provider(&s1, ask("a9")).unwrap();
    assert_eq!(dump(&k), snap);
    assert_eq!(
        requests(&k).last().unwrap(),
        &Request::Answer {
            session: s1.clone(),
            ask: "a9".into(),
            decision: json!({"deny": true, "reason": "quota"})
        }
    );
}

#[test]
fn ac8_decisions_resolutions_and_turns_are_stored_while_past() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.provider(&s1, ask("a1")).unwrap();
    k.say("alice", &s1, "q1", "queued input").unwrap();
    k.limits.owner_bytes = usage(&mut k, "quota/alice").bytes;
    let n = requests(&k).len();
    // an ask decision entry (the entry repeated with a decision)
    let decided = ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": "a1", "kind": "ask", "toolCallId": "a1", "toolCall": {"toolCallId": "a1", "title": "Edit"},
                "input": {}, "decision": {"outcome": "deny", "by": "host"}}),
        }),
        ..Out::default()
    });
    k.provider(&s1, decided).unwrap();
    let lines = entry_lines(&mut k, &s1);
    assert!(lines
        .iter()
        .any(|l| l["kind"] == "ask" && l["body"]["decision"].is_object()));
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Running { .. }));
    // a new ask, not stored, stops the running turn
    k.provider(&s1, ask("a3")).unwrap();
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Idle { .. }));
    // a turn that ends starts no queued input
    k.provider(&s1, idle()).unwrap();
    assert_eq!(k.queued("alice", &s1).unwrap().len(), 1);
    assert!(!requests(&k)[n..]
        .iter()
        .any(|r| matches!(r, Request::Turn { .. })));
}

#[test]
fn ac8_ask_resolved_and_turn_events_are_stored_while_past() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.provider(&s1, ask("a1")).unwrap();
    k.say("alice", &s1, "q1", "queued input").unwrap();
    k.limits.owner_bytes = usage(&mut k, "quota/alice").bytes;
    let n = requests(&k).len();
    k.provider(&s1, resolved("a1")).unwrap();
    let lines = entry_lines(&mut k, &s1);
    assert!(lines
        .iter()
        .any(|l| l["kind"] == "ask" && l["body"]["decision"].is_object()));
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Running { .. }));
    k.provider(&s1, idle()).unwrap();
    assert!(matches!(
        rec(&mut k, &s1).turn,
        Turn::Idle {
            stop: Stop::Completed,
            ..
        }
    ));
    assert_eq!(k.queued("alice", &s1).unwrap().len(), 1);
    assert!(!requests(&k)[n..]
        .iter()
        .any(|r| matches!(r, Request::Turn { .. })));
    let s2 = start_other(&mut k);
    k.provider(
        &s2,
        ProviderEvent::Turn(Turn::Idle {
            stop: Stop::Interrupted,
            seconds: None,
            tools: None,
            tokens: None,
        }),
    )
    .unwrap();
    assert!(matches!(
        rec(&mut k, &s2).turn,
        Turn::Idle {
            stop: Stop::Interrupted,
            ..
        }
    ));
}

/// A running session of bob, made before bob is past anything.
fn start_other(k: &mut K) -> String {
    start(k, "bob", "kx")
}

#[test]
fn ac9_a_call_cannot_take_the_account_past_the_reserve() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    let used = usage(&mut k, "quota").bytes;
    k.limits.account_bytes = used + 10;
    k.limits.reserve_bytes = 0;
    assert!(matches!(
        k.say("alice", &s1, "x", &"y".repeat(500)),
        Err(KernelError::Quota)
    ));
    k.limits.reserve_bytes = 1 << 20;
    k.say("alice", &s1, "x2", &"y".repeat(500)).unwrap();
}

#[test]
fn ac10_a_session_at_the_chunk_cap_ends_its_turn_case_53() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    let mut r = rec(&mut k, &s1);
    r.chunks = (1..=16_383).collect();
    r.next_index = 16_384;
    r.save(&mut k.state).unwrap();
    k.provider(&s1, message("reply")).unwrap();
    let r = rec(&mut k, &s1);
    assert_eq!(r.chunks.len(), 16_384);
    assert!(matches!(
        r.turn,
        Turn::Idle { stop: Stop::Stopped { ref limit }, .. } if limit == "quota"
    ));
    let line = k.state.get(&format!("entries/{s1}/16383")).unwrap();
    let v: Value = serde_json::from_slice(&line).unwrap();
    assert_eq!(v["kind"], "turn");
    assert!(requests(&k).contains(&Request::Interrupt {
        session: s1.clone()
    }));
    assert!(matches!(
        k.say("alice", &s1, "x", "more"),
        Err(KernelError::Quota)
    ));
}

#[test]
fn ac11_the_bounds_hold() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    let sorted = |k: &K| {
        let mut v = k.state.get_keys().to_vec();
        v.sort();
        v
    };
    k.state.reset_counts();
    k.provider(&s1, message("hello")).unwrap();
    assert_eq!(
        sorted(&k),
        ["entries/s1/0", "quota", "quota/alice", "session/s1"]
    );
    assert_eq!(k.state.counts().puts, 4);
    k.state.reset_counts();
    k.say("alice", &s1, "k9", "queue me").unwrap();
    assert_eq!(
        sorted(&k),
        [
            "access/alice",
            "key/alice/k9",
            "queue/s1",
            "quota",
            "quota/alice",
            "session/s1",
            "slots"
        ]
    );
    assert_eq!(k.state.counts().puts, 5);
    k.provider(&s1, ask("a1")).unwrap();
    k.state.reset_counts();
    k.answer("alice", &s1, "k8", "a1", r#"{"allow":true}"#)
        .unwrap();
    let keys = sorted(&k);
    assert_eq!(keys.iter().filter(|x| *x == "quota").count(), 1);
    assert_eq!(keys.iter().filter(|x| *x == "quota/alice").count(), 1);
}

#[test]
fn validator_delete_frees_the_stream_and_live_keys() {
    let mut k = kernel();
    let a1 = start(&mut k, "alice", "k1");
    forge(&mut k, "alice", &a1, "stream/x1", 3);
    forge(&mut k, "alice", &a1, &format!("live/{a1}"), 4);
    check(&k);
    k.delete("alice", &a1).unwrap();
    assert!(dump(&k)
        .keys()
        .all(|x| !x.starts_with("stream/") && !x.starts_with("live/")));
    check(&k);
}

#[test]
fn validator_a_failed_step_in_a_firing_keeps_the_accounts_exact() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.provider(&s1, ask("a1")).unwrap();
    // at the chunk cap, the last chunk with room for one entry: the step's decision is written,
    // its `turn` entry is refused `quota`
    let was = k.state.get(&format!("session/{s1}")).unwrap().len() as u64;
    let mut r = rec(&mut k, &s1);
    // chunk 0 keeps the ask; the others start past it
    let more = agent_kernel::transcript::MAX_CHUNKS as u64 - 1;
    r.chunks = std::iter::once(0).chain(1000..1000 + more).collect();
    r.save(&mut k.state).unwrap();
    let mut u = usage(&mut k, "quota/alice");
    let mut a = usage(&mut k, "quota");
    let grew = k.state.get(&format!("session/{s1}")).unwrap().len() as u64 - was;
    u.bytes += grew;
    a.bytes += grew;
    u.save(&mut k.state, "quota/alice").unwrap();
    a.save(&mut k.state, "quota").unwrap();
    let last = format!("entries/{s1}/{}", agent_kernel::transcript::MAX_CHUNKS - 1);
    let lines = vec!["{}"; agent_kernel::transcript::CHUNK_ENTRIES - 1].join("\n");
    forge_value(&mut k, "alice", &s1, &last, lines.clone().into_bytes());
    check(&k);
    k.host.boot = 2;
    k.list("alice", None, 1).unwrap();
    k.timer("restart");
    assert!(
        k.state.get(&last).unwrap().len() > lines.len(),
        "the decision was written"
    );
    assert_eq!(rec(&mut k, &s1).boot, 1, "the step failed");
    check(&k);
}

#[test]
fn validator_a_firing_past_the_reserve_still_charges_its_step() {
    let mut k = kernel();
    let s1 = start(&mut k, "alice", "k1");
    k.limits.account_bytes = usage(&mut k, "quota").bytes;
    k.limits.reserve_bytes = 0;
    k.host.boot = 2;
    k.list("alice", None, 1).unwrap();
    k.timer("restart");
    assert_eq!(rec(&mut k, &s1).turn, Turn::Unknown);
    check(&k);
}
