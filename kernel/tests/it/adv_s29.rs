//! Adversary tests of S29 (9.3 "Past", 9.4 "The short decision").

use agent_kernel::active::Slots;
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

fn ask(id: &str, decision: Option<Value>) -> ProviderEvent {
    let mut body = json!({"id": id, "kind": "ask", "toolCallId": id,
        "toolCall": {"toolCallId": id, "title": "Edit"}, "input": {}});
    if let Some(d) = decision {
        body["decision"] = d;
    }
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body,
        }),
        ..Out::default()
    })
}

fn entry_bytes(k: &K) -> usize {
    k.state
        .entries()
        .filter(|(key, _)| key.starts_with("entries/"))
        .map(|(key, v)| key.len() + v.len())
        .sum()
}

/// Past the account's ceiling (reserve 0), an adapter repeats a decided ask entry for an ask
/// whose answer is pending. Each is stored short, and `close_ask` never drops the pending
/// answer, so every repeat matches again: the bytes written past the ceiling grow with the
/// count of items, not with the asks open when it crossed (9.3 "Past").
#[test]
fn adv_s29_pending_decided_ask_repeats_past_ceiling() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    k.provider_item("t1", ask("a2", None)).unwrap();
    k.answer("alice", &id, "ans1", "a2", r#"{"allow":true}"#)
        .unwrap();
    assert_eq!(
        Record::load(&mut k.state, &id)
            .unwrap()
            .pending_answers
            .len(),
        1
    );
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
    let before = entry_bytes(&k);
    // 256 control characters escape to 6 bytes each.
    let reason = "\u{1}".repeat(256);
    for _ in 0..200 {
        k.provider_item(
            "t1",
            ask(
                "a2",
                Some(json!({"outcome": "allow", "by": "host", "reason": reason})),
            ),
        )
        .unwrap();
    }
    let wrote = entry_bytes(&k) - before;
    let over = Usage::load(&mut k.state, "quota").bytes - k.limits.account_bytes;
    assert!(
        wrote < 4096,
        "past the ceiling 200 repeats wrote {wrote} entry bytes; account {over} over its ceiling"
    );
}
