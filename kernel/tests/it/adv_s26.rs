//! Adversary tests of S26 (provider streams, owner streams, idle timer): each shows an attack that works.

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out, Resolved};
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

fn turn_entries(k: &mut K, id: &str) -> usize {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .filter(|e| serde_json::from_str::<Value>(e).unwrap()["kind"] == "turn")
        .count()
}

/// S26-1 AC3, TECHNICAL 9.3 Quota: past quota "a turn change is stored only when it ends an
/// active turn (so a turn gets at most one `turn` entry past quota)". A turn waiting on asks
/// opened before the owner went past gets a `turn` entry for each `ask-resolved` (the waiting
/// list shrinks, then `running`) through `close_ask`'s own `turn_to`, which skips the guard of
/// `change_turn`: up to 64 more entries a turn past quota.
#[test]
fn adv_s26_past_quota_ask_resolutions_add_turn_entries() {
    let mut k = kernel();
    let s = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &s, 0).unwrap();
    for n in 0..8 {
        k.provider_item("t1", ask(&format!("a{n}"))).unwrap();
    }
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    let at = turn_entries(&mut k, &s);
    for n in 0..8 {
        k.provider_item("t1", resolved(&format!("a{n}"))).unwrap();
    }
    k.provider_item(
        "t1",
        ProviderEvent::Turn(Turn::Idle {
            stop: Stop::Completed,
            seconds: None,
            tools: None,
            tokens: None,
        }),
    )
    .unwrap();
    let after = turn_entries(&mut k, &s);
    assert!(
        after - at <= 1,
        "past quota one turn got {} turn entries",
        after - at
    );
}

/// S26-fix1 AC2, TECHNICAL 9.3 Quota ("one `turn` entry per turn past quota"; `answer` without
/// `for: session` is not refused past quota): the fix put the guard on provider items only
/// (`work.past` is set in `provider_work`). An owner past its quota answers the asks of a turn
/// waiting on asks opened below it, one `answer` each: `answer`'s `close_ask` runs with
/// `Work::default()` (`past: false`), so each answer writes a `turn` entry (the waiting list
/// shrinks, then `running`): up to 64 more entries a turn past quota.
#[test]
fn adv_s26_past_quota_owner_answers_add_turn_entries() {
    let mut k = kernel();
    let s = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &s, 0).unwrap();
    for n in 0..8 {
        k.provider_item("t1", ask(&format!("a{n}"))).unwrap();
    }
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    let at = turn_entries(&mut k, &s);
    for n in 0..8 {
        k.answer(
            "alice",
            &s,
            &format!("ans{n}"),
            &format!("a{n}"),
            r#"{"deny":true}"#,
        )
        .unwrap();
    }
    k.provider_item(
        "t1",
        ProviderEvent::Turn(Turn::Idle {
            stop: Stop::Completed,
            seconds: None,
            tools: None,
            tokens: None,
        }),
    )
    .unwrap();
    let after = turn_entries(&mut k, &s);
    assert!(
        after - at <= 1,
        "past quota one turn got {} turn entries",
        after - at
    );
}
