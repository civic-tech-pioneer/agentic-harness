//! Asks as the kernel records them (TECHNICAL 9.3 pending answers and the cap of 64, 9.8 rule 4,
//! 16 case 73): the owner's answer is pending, the host's `ask-resolved` or a closing step writes
//! the one decision entry.

use agent_kernel::active::Slots;
use agent_kernel::adapters::claude::ClaudeAdapter;
use agent_kernel::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

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
        boot: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn started(k: &mut K) -> String {
    k.start("alice", "k", "p", "{}", IN).unwrap().value
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

/// One stream line through the adapter into the kernel.
fn feed(k: &mut K, a: &mut ClaudeAdapter, id: &str, line: Value) {
    for out in a.line(line.to_string().as_bytes(), 1000) {
        k.provider(id, ProviderEvent::Out(out)).unwrap();
    }
}

fn ask_line(kind: &str, id: &str) -> Value {
    json!({"type": kind, "id": id, "tool": "Edit", "input": {"file_path": "/w/x"},
        "tool_use_id": format!("t-{id}"), "expires-at": 9000})
}

fn resolved_line(id: &str, by: &str, outcome: Option<&str>) -> Value {
    let mut v = json!({"type": "ask-resolved", "id": id, "by": by});
    if let Some(o) = outcome {
        v["outcome"] = json!(o);
    }
    v
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

/// The decision entries of the whole transcript, by ask id: each id must have at most one.
fn decisions(k: &mut K, id: &str) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let mut p = k.read("alice", id, 0, None, 256).unwrap();
    let mut all: Vec<String> = Vec::new();
    loop {
        let n = p.entries.len();
        all.append(&mut p.entries);
        if n == 0 || p.last as usize <= all.len() {
            break;
        }
        p = k.read("alice", id, all.len() as u64, None, 256).unwrap();
    }
    for e in all {
        let e: Value = serde_json::from_str(&e).unwrap();
        if e["kind"] == "ask" && e["body"]["decision"].is_object() {
            let ask = e["body"]["id"].as_str().unwrap().to_string();
            let d = e["body"]["decision"].clone();
            assert!(
                out.insert(ask.clone(), d).is_none(),
                "two decisions for {ask}"
            );
        }
    }
    out
}

fn answers(k: &K) -> Vec<(String, Value)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Answer { ask, decision, .. }) => {
                Some((ask.clone(), decision.clone()))
            }
            _ => None,
        })
        .collect()
}

fn owner(k: &mut K, id: &str, key: &str, ask: &str, d: &str) {
    k.answer("alice", id, key, ask, d).unwrap();
}

#[test]
fn ac1_an_answer_is_pending_and_writes_no_entry() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("ask", "a1"));
    let asks = |es: &[Value]| es.iter().filter(|e| e["kind"] == "ask").count();
    let n = asks(&entries(&mut k, &id));
    owner(&mut k, &id, "k1", "a1", r#"{"deny":true,"reason":"no"}"#);
    // only the turn follows (`waiting` to `running`); no decision entry
    assert_eq!(asks(&entries(&mut k, &id)), n);
    let r = rec(&mut k, &id);
    assert!(r.open.is_empty());
    let p = &r.pending_answers["a1"];
    assert_eq!(
        (p.i, p.kind.as_str(), p.outcome.as_str()),
        (3, "ask", "deny")
    );
    assert_eq!((p.by.as_str(), p.reason.as_deref()), ("owner", Some("no")));
    let raw: Value =
        serde_json::from_slice(&k.state.get(&format!("session/{id}")).unwrap()).unwrap();
    assert_eq!(raw["pending-answers"]["a1"]["i"], 3);
    // a second answer is refused
    let KernelError::Refused(rs) = k
        .answer("alice", &id, "k2", "a1", r#"{"allow":true}"#)
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(rs[0].field, "ask");
}

#[test]
fn ac1_a_pending_answer_over_1_kib_or_the_64th_pending_refuses_and_writes_nothing() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("ask", "a1"));
    let before = k.state.get(&format!("session/{id}"));
    let reason = "x".repeat(1100);
    let d = json!({"deny": true, "reason": reason}).to_string();
    assert_eq!(
        k.answer("alice", &id, "k1", "a1", &d).unwrap_err(),
        KernelError::TooLarge(1024)
    );
    assert_eq!(k.state.get(&format!("session/{id}")), before);
    assert!(k.state.get("key/alice/k1").is_none());
    // 64 pending answers: the 65th ask is open, its answer is busy
    let mut r = rec(&mut k, &id);
    for n in 0..64 {
        r.pending_answers.insert(
            format!("p{n}"),
            record::PendingAnswer {
                i: 2,
                kind: "ask".into(),
                outcome: "allow".into(),
                by: "owner".into(),
                for_: None,
                reason: None,
            },
        );
    }
    r.open.clear();
    r.open.push(OpenAsk {
        id: "a1".into(),
        kind: "ask".into(),
        i: 2,
        expires_at: None,
    });
    r.save(&mut k.state).unwrap();
    let before = k.state.get(&format!("session/{id}"));
    let sent = answers(&k).len();
    let e = k
        .answer("alice", &id, "k2", "a1", r#"{"allow":true}"#)
        .unwrap_err();
    assert!(
        matches!(&e, KernelError::Busy(m) if m.contains("64")),
        "{e:?}"
    );
    assert_eq!(k.state.get(&format!("session/{id}")), before);
    assert_eq!(answers(&k).len(), sent);
}

#[test]
fn case73_a_direct_mode_owner_allow_then_answered_allow() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("ask", "a1"));
    owner(&mut k, &id, "k1", "a1", r#"{"allow":true}"#);
    feed(
        &mut k,
        &mut a,
        &id,
        resolved_line("a1", "answered", Some("allow")),
    );
    let d = decisions(&mut k, &id);
    assert_eq!(d["a1"], json!({"outcome": "allow", "by": "owner"}));
    assert!(rec(&mut k, &id).pending_answers.is_empty());
}

#[test]
fn case73_b_checked_mode_owner_allow_then_answered_deny() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::checked();
    feed(&mut k, &mut a, &id, ask_line("ask", "a1"));
    owner(&mut k, &id, "k1", "a1", r#"{"allow":true}"#);
    feed(
        &mut k,
        &mut a,
        &id,
        resolved_line("a1", "answered", Some("deny")),
    );
    let d = decisions(&mut k, &id);
    assert_eq!(d["a1"], json!({"outcome": "deny", "by": "checked"}));
    // a direct stream's differing answered is the host's
    let id2 = k.start("alice", "k2", "p", "{}", IN).unwrap().value;
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id2, ask_line("ask", "a1"));
    feed(
        &mut k,
        &mut a,
        &id2,
        resolved_line("a1", "answered", Some("deny")),
    );
    assert_eq!(
        decisions(&mut k, &id2)["a1"],
        json!({"outcome": "deny", "by": "host"})
    );
}

#[test]
fn case73_c_a_pending_answer_then_expired() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("ask", "a1"));
    owner(&mut k, &id, "k1", "a1", r#"{"allow":true}"#);
    feed(&mut k, &mut a, &id, resolved_line("a1", "expired", None));
    assert_eq!(
        decisions(&mut k, &id)["a1"],
        json!({"outcome": "deny", "by": "host", "reason": "expired"})
    );
    assert!(rec(&mut k, &id).pending_answers.is_empty());
}

#[test]
fn case73_d_expired_with_an_outcome_is_unknown_and_the_ask_stays() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("ask", "a1"));
    feed(
        &mut k,
        &mut a,
        &id,
        resolved_line("a1", "expired", Some("deny")),
    );
    assert!(decisions(&mut k, &id).is_empty());
    let es = entries(&mut k, &id);
    assert_eq!(es.last().unwrap()["kind"], "unknown");
    assert_eq!(rec(&mut k, &id).open.len(), 1);
    // an unknown or bad item leaves it open, so a later expired closes it
    feed(&mut k, &mut a, &id, resolved_line("a1", "user", None));
    feed(
        &mut k,
        &mut a,
        &id,
        resolved_line("a1", "answered", Some("none")),
    );
    feed(&mut k, &mut a, &id, resolved_line("a1", "answered", None));
    assert_eq!(rec(&mut k, &id).open.len(), 1);
    feed(&mut k, &mut a, &id, resolved_line("a1", "expired", None));
    assert_eq!(
        decisions(&mut k, &id)["a1"],
        json!({"outcome": "deny", "by": "host", "reason": "expired"})
    );
    assert!(rec(&mut k, &id).open.is_empty());
}

#[test]
fn case73_e_a_check_expired_then_an_answer_is_refused() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("check", "c1"));
    // `allow` on a check is unknown
    feed(
        &mut k,
        &mut a,
        &id,
        resolved_line("c1", "answered", Some("allow")),
    );
    assert!(decisions(&mut k, &id).is_empty());
    feed(&mut k, &mut a, &id, resolved_line("c1", "expired", None));
    assert_eq!(decisions(&mut k, &id).len(), 1);
    assert!(matches!(
        k.answer("alice", &id, "k1", "c1", r#"{"allow":true}"#),
        Err(KernelError::Refused(_))
    ));
}

/// An open ask a1 and a pending one a2, then `f` closes the session's asks.
fn two_asks(k: &mut K) -> String {
    let id = started(k);
    let mut a = ClaudeAdapter::new();
    feed(k, &mut a, &id, ask_line("ask", "a1"));
    feed(k, &mut a, &id, ask_line("ask", "a2"));
    owner(k, &id, "k1", "a2", r#"{"allow":true}"#);
    id
}

fn assert_two(k: &mut K, id: &str, reason: &str) {
    let d = decisions(k, id);
    assert_eq!(d.len(), 2);
    assert_eq!(
        d["a1"],
        json!({"outcome": "deny", "by": "kernel", "reason": reason})
    );
    assert_eq!(d["a2"], json!({"outcome": "allow", "by": "owner"}));
    let r = rec(k, id);
    assert!(r.open.is_empty() && r.pending_answers.is_empty());
}

#[test]
fn case73_f_the_streams_end_closes_every_ask() {
    let mut k = kernel();
    let id = two_asks(&mut k);
    k.provider(&id, ProviderEvent::Ended).unwrap();
    assert_two(&mut k, &id, "ended");
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
}

#[test]
fn case73_g_the_restart_step_closes_every_ask_in_every_state() {
    for state in ["running", "idle", "unknown", "paused"] {
        let mut k = kernel();
        let id = two_asks(&mut k);
        let turn = match state {
            "running" => None,
            "idle" => Some(Turn::default()),
            "unknown" => Some(Turn::Unknown),
            _ => Some(Turn::Paused { until: None }),
        };
        if let Some(t) = turn {
            let mut r = rec(&mut k, &id);
            r.turn = t;
            r.save(&mut k.state).unwrap();
        }
        k.host.boot = 2;
        k.list("nobody", None, 1).unwrap();
        k.read("alice", &id, 0, None, 1).unwrap();
        assert_two(&mut k, &id, "restart");
        let t = rec(&mut k, &id).turn;
        match state {
            "running" => assert_eq!(t, Turn::Unknown),
            "idle" => assert_eq!(t, Turn::default()),
            "unknown" => assert_eq!(t, Turn::Unknown),
            _ => assert_eq!(t, Turn::Paused { until: None }),
        }
    }
}

#[test]
fn case73_g_cancel_closes_the_same_two_asks_in_every_state() {
    for state in ["running", "idle", "unknown", "paused"] {
        let mut k = kernel();
        let id = two_asks(&mut k);
        let turn = match state {
            "running" => None,
            "idle" => Some(Turn::default()),
            "unknown" => Some(Turn::Unknown),
            _ => Some(Turn::Paused { until: None }),
        };
        if let Some(t) = turn {
            let mut r = rec(&mut k, &id);
            r.turn = t;
            r.save(&mut k.state).unwrap();
        }
        k.cancel("alice", &id).unwrap();
        assert_two(&mut k, &id, "cancel");
    }
}

#[test]
fn case73_g_the_restart_step_leaves_an_openai_sessions_asks() {
    let mut k = kernel();
    let id = two_asks(&mut k);
    let mut r = rec(&mut k, &id);
    r.backend = "openai".into();
    r.save(&mut k.state).unwrap();
    let turn = r.turn.clone();
    assert!(matches!(turn, Turn::Waiting { .. }));
    k.host.boot = 2;
    k.list("nobody", None, 1).unwrap();
    k.read("alice", &id, 0, None, 1).unwrap();
    assert!(decisions(&mut k, &id).is_empty());
    let r = rec(&mut k, &id);
    assert_eq!((r.boot, r.turn), (2, turn));
    assert_eq!((r.open.len(), r.pending_answers.len()), (1, 1));
}

#[test]
fn ac4_the_65th_ask_is_denied_busy_at_once_and_ended_writes_64_decisions() {
    let mut k = kernel();
    let id = started(&mut k);
    let mut a = ClaudeAdapter::new();
    for n in 0..64 {
        let kind = if n % 2 == 0 { "ask" } else { "check" };
        feed(&mut k, &mut a, &id, ask_line(kind, &format!("a{n}")));
    }
    assert_eq!(rec(&mut k, &id).open.len(), 64);
    let turn = rec(&mut k, &id).turn;
    feed(&mut k, &mut a, &id, ask_line("ask", "extra"));
    let r = rec(&mut k, &id);
    assert_eq!(r.open.len(), 64);
    assert_eq!(r.turn, turn);
    assert_eq!(
        decisions(&mut k, &id)["extra"],
        json!({"outcome": "deny", "by": "kernel", "reason": "busy"})
    );
    assert_eq!(
        answers(&k).last().unwrap(),
        &("extra".to_string(), json!({"deny": true, "reason": "busy"}))
    );
    // a later ask-resolved of it is unknown
    feed(&mut k, &mut a, &id, resolved_line("extra", "expired", None));
    assert_eq!(decisions(&mut k, &id).len(), 1);
    assert_eq!(entries(&mut k, &id).last().unwrap()["kind"], "unknown");
    k.provider(&id, ProviderEvent::Ended).unwrap();
    let d = decisions(&mut k, &id);
    assert_eq!(d.len(), 65);
    assert_eq!(d["a7"]["reason"], "ended");
    let r = rec(&mut k, &id);
    assert!(r.open.is_empty() && r.pending_answers.is_empty());
}

#[test]
fn ac2_the_adapter_reports_and_keeps_or_drops_the_open_id() {
    for (a, direct) in [
        (ClaudeAdapter::new(), true),
        (ClaudeAdapter::checked(), false),
    ] {
        let mut a = a;
        a.line(ask_line("ask", "a1").to_string().as_bytes(), 1);
        let one = |a: &mut ClaudeAdapter, v: Value| {
            let mut outs = a.line(v.to_string().as_bytes(), 1);
            assert_eq!(outs.len(), 1);
            outs.remove(0)
        };
        // bad shapes are unknown and keep the id
        for bad in [
            resolved_line("a1", "answered", Some("none")),
            resolved_line("a1", "answered", Some("maybe")),
            resolved_line("a1", "answered", None),
            resolved_line("a1", "cancelled", Some("deny")),
            resolved_line("nope", "expired", None),
            json!({"type": "ask-resolved", "id": "", "by": "expired"}),
        ] {
            let o = one(&mut a, bad);
            assert!(o.resolved.is_none() && o.entry.is_some());
        }
        let o = one(&mut a, resolved_line("a1", "answered", Some("allow")));
        let r = o.resolved.unwrap();
        assert!(o.entry.is_none());
        assert_eq!(
            (r.id.as_str(), r.by.as_str(), r.outcome.as_deref(), r.direct),
            ("a1", "answered", Some("allow"), direct)
        );
        // the id left the open map
        assert!(one(&mut a, resolved_line("a1", "expired", None))
            .resolved
            .is_none());
    }
}
