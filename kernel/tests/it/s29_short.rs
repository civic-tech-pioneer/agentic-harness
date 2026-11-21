//! S29-1: past its quota, a decision entry is the short form `{id, kind, ask-i, decision}`,
//! built from the record alone (TECHNICAL 9.3 "Past", 9.4 "The short decision").

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out, Resolved};
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

fn ask_of(id: &str, kind: &str, pad: usize) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": kind, "toolCallId": id, "toolCall": {"toolCallId": id, "title": "Edit"},
                "input": {"pad": "x".repeat(pad)}, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    let mut all = Vec::new();
    let mut from = 0;
    loop {
        let page = k.read("alice", id, from, None, 256).unwrap();
        if page.entries.is_empty() {
            break;
        }
        from += page.entries.len() as u64;
        all.extend(
            page.entries
                .iter()
                .map(|e| serde_json::from_str::<Value>(e).unwrap()),
        );
    }
    all
}

/// The decision entries after the first `n` entries.
fn decisions_after(k: &mut K, id: &str, n: usize) -> Vec<Value> {
    entries(k, id)
        .split_off(n)
        .into_iter()
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .collect()
}

fn entry_bytes(k: &K) -> usize {
    k.state
        .entries()
        .filter(|(key, _)| key.starts_with("entries/"))
        .map(|(key, v)| key.len() + v.len())
        .sum()
}

fn usage(k: &mut K, key: &str) -> u64 {
    Usage::load(&mut k.state, key).bytes
}

fn past_owner(k: &mut K) {
    k.limits.owner_bytes = usage(k, "quota/alice");
}

fn past_account(k: &mut K) {
    k.limits.account_bytes = usage(k, "quota");
    k.limits.reserve_bytes = 1 << 30;
}

/// A waiting session: `a1` open, `a2` (a check) answered, so pending. Returns the id and the
/// entry count before the closing call.
fn waiting(k: &mut K) -> String {
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask_of("a1", "ask", 0)).unwrap();
    k.provider_item("t1", ask_of("a2", "ask", 0)).unwrap();
    k.answer("alice", &id, "ans1", "a2", r#"{"allow":true}"#)
        .unwrap();
    let r = rec(k, &id);
    assert_eq!((r.open.len(), r.pending_answers.len()), (1, 1));
    id
}

fn check_short(k: &mut K, id: &str, n: usize, reason: &str) {
    let r_open_i = |k: &mut K| {
        let es = entries(k, id);
        let i_of = |ask: &str| {
            es.iter()
                .position(|e| {
                    e["kind"] == "ask"
                        && e["body"]["id"] == ask
                        && e["body"]["toolCall"].is_object()
                })
                .unwrap() as u64
                + 1
        };
        (i_of("a1"), i_of("a2"))
    };
    let (i1, i2) = r_open_i(k);
    let ds = decisions_after(k, id, n);
    assert_eq!(ds.len(), 2);
    for d in &ds {
        let mut keys: Vec<&String> = d["body"].as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["ask-i", "decision", "id", "kind"]);
    }
    let by = |a: &str| ds.iter().find(|d| d["body"]["id"] == a).unwrap()["body"].clone();
    assert_eq!(by("a1")["ask-i"], i1);
    assert_eq!(by("a2")["ask-i"], i2);
    assert_eq!(
        by("a1")["decision"],
        json!({"outcome": "deny", "by": "kernel", "reason": reason})
    );
    assert_eq!(by("a2")["decision"]["outcome"], "allow");
}

fn ended_with(past: fn(&mut K)) {
    let mut k = kernel();
    let id = waiting(&mut k);
    let n = entries(&mut k, &id).len();
    past(&mut k);
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    check_short(&mut k, &id, n, "ended");
}

#[test]
fn ac1_ended_past_the_owner_quota_writes_short_decisions() {
    ended_with(past_owner);
}

#[test]
fn ac1_ended_past_the_account_quota_writes_short_decisions() {
    ended_with(past_account);
}

#[test]
fn ac1_the_pending_decision_keeps_its_input() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask_of("a1", "ask", 0)).unwrap();
    k.answer(
        "alice",
        &id,
        "ans1",
        "a1",
        r#"{"allow":true,"input":{"x":1}}"#,
    )
    .unwrap();
    let want = rec(&mut k, &id).pending_answers["a1"].decision();
    let n = entries(&mut k, &id).len();
    past_owner(&mut k);
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    let ds = decisions_after(&mut k, &id, n);
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0]["body"]["decision"], want);
    assert_eq!(ds[0]["body"]["decision"]["input"], json!({"x": 1}));
}

#[test]
fn ac2_cancel_past_writes_short_decisions() {
    let mut k = kernel();
    let id = waiting(&mut k);
    let n = entries(&mut k, &id).len();
    past_owner(&mut k);
    k.cancel("alice", &id).unwrap();
    check_short(&mut k, &id, n, "cancel");
}

#[test]
fn ac2_the_restart_step_past_writes_short_decisions() {
    let mut k = kernel();
    let id = waiting(&mut k);
    let n = entries(&mut k, &id).len();
    past_owner(&mut k);
    k.host.boot = 2;
    k.timer("restart");
    assert!(rec(&mut k, &id).open.is_empty());
    check_short(&mut k, &id, n, "restart");
}

fn resolved(id: &str, by: &str, outcome: Option<&str>) -> ProviderEvent {
    ProviderEvent::Out(Out {
        resolved: Some(Resolved {
            id: id.into(),
            by: by.into(),
            outcome: outcome.map(String::from),
            direct: true,
            raw: json!({"type": "ask-resolved", "id": id, "by": by}),
        }),
        ..Out::default()
    })
}

#[test]
fn ac3_ask_resolved_past_writes_a_short_decision() {
    let mut k = kernel();
    let id = waiting(&mut k);
    let n = entries(&mut k, &id).len();
    past_owner(&mut k);
    k.provider_item("t1", resolved("a1", "expired", None))
        .unwrap();
    k.provider_item("t1", resolved("a2", "answered", Some("allow")))
        .unwrap();
    let ds = decisions_after(&mut k, &id, n);
    assert_eq!(ds.len(), 2);
    for d in &ds {
        let mut keys: Vec<&String> = d["body"].as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["ask-i", "decision", "id", "kind"]);
    }
    assert_eq!(ds[0]["body"]["decision"]["reason"], "expired");
    assert_eq!(ds[1]["body"]["decision"]["outcome"], "allow");
}

#[test]
fn ac4_below_quota_a_decision_is_the_full_form() {
    let mut k = kernel();
    let id = waiting(&mut k);
    let n = entries(&mut k, &id).len();
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    let ds = decisions_after(&mut k, &id, n);
    assert_eq!(ds.len(), 2);
    assert!(ds.iter().all(|d| d["body"]["toolCall"].is_object()));
}

/// `n` open asks, each entry about `pad` bytes, in a running session.
fn many(k: &mut K, n: usize, pad: usize) -> String {
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    for j in 0..n {
        k.provider_item("t1", ask_of(&format!("a{j}"), "ask", pad))
            .unwrap();
    }
    id
}

fn is_short(d: &Value) -> bool {
    d["body"].as_object().unwrap().len() == 4
}

fn crossing(close: fn(&mut K, &str)) {
    let mut k = kernel();
    let id = many(&mut k, 4, 20 << 10);
    let n = entries(&mut k, &id).len();
    // just below the cap: the first full decision (over 20 KiB) takes the owner over it
    k.limits.owner_bytes = usage(&mut k, "quota/alice") + 1000;
    close(&mut k, &id);
    let ds = decisions_after(&mut k, &id, n);
    assert_eq!(ds.len(), 4);
    assert!(!is_short(&ds[0]), "the crossing decision is full");
    assert!(ds[1..].iter().all(is_short), "the rest are short");
}

#[test]
fn ac5_ended_crossing_the_cap_writes_the_rest_short() {
    crossing(|k, _| {
        k.provider_item("t1", ProviderEvent::Ended).unwrap();
    });
}

#[test]
fn ac5_cancel_crossing_the_cap_writes_the_rest_short() {
    crossing(|k, id| {
        k.cancel("alice", id).unwrap();
    });
}

#[test]
fn ac6_ended_past_with_64_large_asks_writes_under_a_mebibyte() {
    let mut k = kernel();
    let id = many(&mut k, 64, 110 << 10);
    assert_eq!(rec(&mut k, &id).open.len(), 64);
    past_owner(&mut k);
    let before = entry_bytes(&k);
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    assert!(rec(&mut k, &id).open.is_empty());
    let wrote = entry_bytes(&k).saturating_sub(before);
    assert!(wrote > 0 && wrote < 1 << 20, "wrote {wrote}");
}

#[test]
fn ac7_past_a_decided_ask_entry_is_stored_short_when_open_and_not_otherwise() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask_of("a1", "ask", 0)).unwrap();
    let i1 = rec(&mut k, &id).open[0].i;
    past_owner(&mut k);
    let n = entries(&mut k, &id).len();
    let decided = |id: &str| {
        ProviderEvent::Out(Out {
            entry: Some(Entry {
                kind: Kind::Ask,
                uuid: None,
                body: json!({"id": id, "kind": "ask", "toolCallId": id,
                    "toolCall": {"toolCallId": id, "title": "Edit"}, "input": {},
                    "decision": {"outcome": "deny", "by": "host"}}),
            }),
            ..Out::default()
        })
    };
    let reqs = |k: &K| {
        k.host
            .calls
            .iter()
            .filter(|c| matches!(c, HostCall::Request(_)))
            .count()
    };
    let r0 = reqs(&k);
    k.provider_item("t1", decided("zz")).unwrap();
    // not stored; the active turn ends `stopped {limit: quota}` as for any item not stored, with
    // an interrupt and no deny (the entry carries its decision)
    let after = entries(&mut k, &id);
    assert!(after[n..].iter().all(|e| e["kind"] == "turn"));
    assert_eq!(after.len(), n + 1);
    assert_eq!(
        rec(&mut k, &id).turn,
        Turn::Idle {
            stop: Stop::Stopped {
                limit: "quota".into()
            },
            seconds: None,
            tools: None,
            tokens: None
        }
    );
    assert_eq!(reqs(&k), r0 + 1);
    assert!(matches!(
        k.host
            .calls
            .iter()
            .rev()
            .find(|c| matches!(c, HostCall::Request(_))),
        Some(HostCall::Request(Request::Interrupt { .. }))
    ));
    k.provider_item("t1", decided("a1")).unwrap();
    let ds = decisions_after(&mut k, &id, n);
    assert_eq!(ds.len(), 1);
    assert_eq!(
        ds[0]["body"],
        json!({"id": "a1", "kind": "ask", "ask-i": i1, "decision": {"outcome": "deny", "by": "host"}})
    );
    assert!(rec(&mut k, &id).open.is_empty());
}

fn decided_with(id: &str, decision: Value) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": id,
                "toolCall": {"toolCallId": id, "title": "Edit"}, "input": {},
                "decision": decision}),
        }),
        ..Out::default()
    })
}

#[test]
fn ac1_a_decided_ask_past_is_stored_with_a_bounded_decision() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask_of("a1", "ask", 0)).unwrap();
    past_owner(&mut k);
    let n = entries(&mut k, &id).len();
    let big = format!("{}é", "x".repeat(255));
    k.provider_item(
        "t1",
        decided_with(
            "a1",
            json!({"outcome": "deny", "by": "host", "reason": big + &"y".repeat(200_000),
                "extra": {"z": "w".repeat(1000)}}),
        ),
    )
    .unwrap();
    assert!(rec(&mut k, &id).open.is_empty());
    let ds = decisions_after(&mut k, &id, n);
    assert_eq!(ds.len(), 1);
    let d = ds[0]["body"]["decision"].as_object().unwrap();
    let mut keys: Vec<&String> = d.keys().collect();
    keys.sort();
    assert_eq!(keys, ["by", "outcome", "reason"]);
    assert_eq!(d["reason"].as_str().unwrap().len(), 255);
    assert!(ds[0].to_string().len() < 1024);
}

#[test]
fn ac2_an_unreadable_decision_is_not_stored_and_stops_the_turn() {
    for bad in [
        json!({"outcome": "maybe", "by": "host"}),
        json!({"outcome": "deny", "by": "nobody"}),
        json!({"outcome": 1, "by": "host"}),
        json!({"outcome": "deny", "by": ["host"]}),
        json!("deny"),
    ] {
        let mut k = kernel();
        let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
        k.provider_item("t1", ask_of("a1", "ask", 0)).unwrap();
        past_owner(&mut k);
        let n = entries(&mut k, &id).len();
        k.provider_item("t1", decided_with("a1", bad.clone()))
            .unwrap();
        assert!(decisions_after(&mut k, &id, n).is_empty(), "{bad}");
        assert!(matches!(
            rec(&mut k, &id).turn,
            Turn::Idle {
                stop: Stop::Stopped { .. },
                ..
            }
        ));
    }
}
