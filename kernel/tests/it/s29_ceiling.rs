//! S29-2: at and past the ceiling every call that ends a turn or closes a session's work
//! succeeds and frees what it holds (TECHNICAL 9.3 "Past", case 77).

use agent_kernel::active::{self, Slots};
use agent_kernel::adapters::{Entry, Kind, Out};
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
    st.put("access/bob", br#"["p"]"#.to_vec()).unwrap();
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

fn ask(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": id, "toolCall": {"toolCallId": id, "title": "Edit"},
                "input": {}, "reach": "Edit(/w/**)"}),
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

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn entries(k: &mut K, owner: &str, id: &str) -> Vec<Value> {
    k.read(owner, id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .collect()
}

fn count(k: &mut K, id: &str, kind: &str) -> usize {
    entries(k, "alice", id)
        .iter()
        .filter(|e| e["kind"] == kind)
        .count()
}

fn decisions(k: &mut K, id: &str) -> usize {
    entries(k, "alice", id)
        .iter()
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .count()
}

fn held(k: &K, session: bool) -> i64 {
    k.state
        .entries()
        .filter(|(key, _)| {
            let head = key.split('/').next().unwrap_or_default();
            let kinds: &[&str] = if session {
                &["queue", "entries", "wire", "loop", "stream", "live"]
            } else {
                &[
                    "session", "queue", "entries", "wire", "loop", "stream", "live", "key", "owner",
                ]
            };
            kinds.contains(&head)
        })
        .map(|(key, v)| (key.len() + v.len()) as i64)
        .sum()
}

fn ceiling(k: &mut K) {
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
}

fn stopped_quota() -> Turn {
    Turn::Idle {
        stop: Stop::Stopped {
            limit: "quota".into(),
        },
        seconds: None,
        tools: None,
        tokens: None,
    }
}

fn interrupted() -> Turn {
    Turn::Idle {
        stop: Stop::Interrupted,
        seconds: None,
        tools: None,
        tokens: None,
    }
}

struct Before(i64, i64, i64, i64, i64);

fn before(k: &mut K, owner: &str, id: &str) -> Before {
    Before(
        Usage::load(&mut k.state, &format!("quota/{owner}")).bytes as i64,
        Usage::load(&mut k.state, "quota").bytes as i64,
        rec(k, id).bytes as i64,
        held(k, false),
        held(k, true),
    )
}

/// The accounts and the record changed by exactly what the call wrote.
fn charged(k: &mut K, owner: &str, id: &str, b: &Before) {
    let wrote = held(k, false) - b.3;
    assert_eq!(
        Usage::load(&mut k.state, &format!("quota/{owner}")).bytes as i64 - b.0,
        wrote
    );
    assert_eq!(Usage::load(&mut k.state, "quota").bytes as i64 - b.1, wrote);
    assert_eq!(rec(k, id).bytes as i64 - b.2, held(k, true) - b.4);
}

fn requests(k: &K, pick: fn(&Request) -> bool) -> usize {
    k.host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::Request(r) if pick(r)))
        .count()
}

#[test]
fn ac1_cancel_at_the_ceiling_succeeds_and_frees() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask("a1")).unwrap();
    k.provider_item("t1", ask("a2")).unwrap();
    k.answer("alice", &id, "ans1", "a2", r#"{"allow":true}"#)
        .unwrap();
    let r = rec(&mut k, &id);
    assert!(matches!(r.turn, Turn::Waiting { .. }));
    assert_eq!((r.open.len(), r.pending_answers.len()), (1, 1));
    ceiling(&mut k);
    let b = before(&mut k, "alice", &id);
    let (d0, t0) = (decisions(&mut k, &id), count(&mut k, &id, "turn"));
    let (p0, tm0) = (
        Usage::load(&mut k.state, "quota/alice").provider,
        Usage::load(&mut k.state, "quota/alice").timers,
    );
    let (s_p0, s_t0) = {
        let s = Slots::load(&mut k.state);
        (s.provider, s.timers)
    };
    assert!(active::read_page(&mut k.state, 0)
        .iter()
        .any(|e| e.id == id));
    k.cancel("alice", &id).unwrap();
    assert!(!active::read_page(&mut k.state, 0)
        .iter()
        .any(|e| e.id == id));
    let r = rec(&mut k, &id);
    assert_eq!(r.turn, interrupted());
    assert!(r.open.is_empty() && r.pending_answers.is_empty());
    assert_eq!(decisions(&mut k, &id), d0 + 2);
    assert_eq!(count(&mut k, &id, "turn"), t0 + 1);
    assert!(k.state.get("stream/t1").is_none());
    assert!(k.state.get(&format!("live/{id}")).is_none());
    assert!(r.deadlines.is_empty() && r.active_page.is_none());
    let s = Slots::load(&mut k.state);
    // Only an idle turn holds a deadline (its timer goes in ac1_cancel_at_the_ceiling_frees_the_timer).
    assert_eq!((s.provider, s.timers), (s_p0 - 1, s_t0));
    let u = Usage::load(&mut k.state, "quota/alice");
    assert_eq!(u.provider, p0 - 1);
    assert_eq!(u.timers, tm0);
    assert_eq!(requests(&k, |r| matches!(r, Request::Cancel { .. })), 1);
    charged(&mut k, "alice", &id, &b);
}

#[test]
fn ac1_cancel_at_the_ceiling_frees_the_timer() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let done = ProviderEvent::Out(Out {
        idle: true,
        ..Out::default()
    });
    k.provider_item("t1", done).unwrap();
    assert_eq!(rec(&mut k, &id).deadlines.len(), 1);
    assert_eq!(Slots::load(&mut k.state).timers, 1);
    assert_eq!(Usage::load(&mut k.state, "quota/alice").timers, 1);
    ceiling(&mut k);
    let b = before(&mut k, "alice", &id);
    k.cancel("alice", &id).unwrap();
    assert!(rec(&mut k, &id).deadlines.is_empty());
    let s = Slots::load(&mut k.state);
    assert_eq!((s.provider, s.timers), (0, 0));
    assert_eq!(Usage::load(&mut k.state, "quota/alice").timers, 0);
    assert!(k.state.get("stream/t1").is_none());
    charged(&mut k, "alice", &id, &b);
}

#[test]
fn ac2_interrupt_without_replacement_at_the_ceiling() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    ceiling(&mut k);
    let b = before(&mut k, "alice", &id);
    let t0 = count(&mut k, &id, "turn");
    k.interrupt("alice", &id, None).unwrap();
    assert_eq!(rec(&mut k, &id).turn, interrupted());
    assert_eq!(count(&mut k, &id, "turn"), t0 + 1);
    assert_eq!(requests(&k, |r| matches!(r, Request::Interrupt { .. })), 1);
    charged(&mut k, "alice", &id, &b);
}

#[test]
fn ac2_interrupt_with_a_replacement_at_the_ceiling_is_refused() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    ceiling(&mut k);
    let (h0, a0) = (held(&k, false), Usage::load(&mut k.state, "quota").bytes);
    assert!(matches!(
        k.interrupt("alice", &id, Some("again")),
        Err(KernelError::Quota(_))
    ));
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
    assert_eq!(held(&k, false), h0);
    assert_eq!(Usage::load(&mut k.state, "quota").bytes, a0);
}

#[test]
fn ac3_a_message_at_the_ceiling_stops_the_turn() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    ceiling(&mut k);
    let b = before(&mut k, "alice", &id);
    let (m0, t0) = (count(&mut k, &id, "message"), count(&mut k, &id, "turn"));
    k.provider_item("t1", message("late")).unwrap();
    assert_eq!(rec(&mut k, &id).turn, stopped_quota());
    assert_eq!(count(&mut k, &id, "message"), m0);
    assert_eq!(count(&mut k, &id, "turn"), t0 + 1);
    assert_eq!(requests(&k, |r| matches!(r, Request::Interrupt { .. })), 1);
    charged(&mut k, "alice", &id, &b);
}

#[test]
fn ac4_above_the_ceiling_persists() {
    for same_owner in [false, true] {
        let mut k = kernel();
        let a = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
        let other = if same_owner { "alice" } else { "bob" };
        let b = k.start(other, "k2", "p", "{}", IN).unwrap().value;
        let tb = rec(&mut k, &b).streams.first().cloned().unwrap();
        ceiling(&mut k);
        k.provider_item("t1", ProviderEvent::Ended).unwrap();
        assert!(Usage::load(&mut k.state, "quota").bytes > k.limits.account_bytes);
        k.host.calls.clear();
        k.provider_item(&tb, message("late")).unwrap();
        assert_eq!(rec(&mut k, &b).turn, stopped_quota());
        assert_eq!(requests(&k, |r| matches!(r, Request::Interrupt { .. })), 1);
        assert!(matches!(rec(&mut k, &a).turn, Turn::Idle { .. }));
    }
}

#[test]
fn ac5_an_ask_at_the_ceiling_is_denied_and_stops_the_turn() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    ceiling(&mut k);
    let b = before(&mut k, "alice", &id);
    let n = count(&mut k, &id, "ask");
    k.provider_item("t1", ask("a1")).unwrap();
    let r = rec(&mut k, &id);
    assert!(r.open.is_empty());
    assert_eq!(r.turn, stopped_quota());
    assert_eq!(count(&mut k, &id, "ask"), n);
    assert_eq!(
        requests(&k, |r| matches!(r, Request::Answer { decision, .. }
            if decision["reason"] == "quota")),
        1
    );
    assert_eq!(requests(&k, |r| matches!(r, Request::Interrupt { .. })), 1);
    charged(&mut k, "alice", &id, &b);
}

#[test]
fn ac6_new_work_is_still_refused_at_the_ceiling() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask("a1")).unwrap();
    ceiling(&mut k);
    let (h0, a0) = (held(&k, false), Usage::load(&mut k.state, "quota").bytes);
    assert!(matches!(
        k.say("alice", &id, "s1", "more"),
        Err(KernelError::Quota(_))
    ));
    assert!(matches!(
        k.start("alice", "k9", "p", "{}", IN),
        Err(KernelError::Quota(_))
    ));
    assert!(matches!(
        k.rename("alice", &id, "new"),
        Err(KernelError::Quota(_))
    ));
    assert!(matches!(
        k.answer(
            "alice",
            &id,
            "ans",
            "a1",
            r#"{"allow":true,"for":"session"}"#
        ),
        Err(KernelError::Quota(_))
    ));
    assert_eq!(held(&k, false), h0);
    assert_eq!(Usage::load(&mut k.state, "quota").bytes, a0);
}

/// 9.3 "Past": an interrupt without a replacement on a record of an older boot runs the restart
/// step (which ends the turn) and is charged for it, never refused.
#[test]
fn ac2_interrupt_without_replacement_after_a_restart_at_the_ceiling() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider_item("t1", ask("a1")).unwrap();
    k.host.boot = 2;
    ceiling(&mut k);
    let b = before(&mut k, "alice", &id);
    k.interrupt("alice", &id, None).unwrap();
    assert!(matches!(rec(&mut k, &id).turn, Turn::Unknown));
    charged(&mut k, "alice", &id, &b);
}

/// AC7: a provider item that starts below the ceiling keeps the ceiling check.
#[test]
fn ac7_an_item_below_the_ceiling_cannot_cross_it() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes + 10;
    k.limits.reserve_bytes = 0;
    let (h0, a0) = (held(&k, false), Usage::load(&mut k.state, "quota").bytes);
    assert!(matches!(
        k.provider_item("t1", message(&"y".repeat(500))),
        Err(KernelError::Quota(_))
    ));
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
    assert_eq!(held(&k, false), h0);
    assert_eq!(Usage::load(&mut k.state, "quota").bytes, a0);
}
