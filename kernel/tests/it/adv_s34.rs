//! Adversary, S34 (re-run on S34-fix1): attacks on the write rule of a session allow (TECHNICAL
//! 9.6 "As built (S34)" Files, 14) and on the session timer's restart step (9.3 "Past").

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::broker::scope;
use agent_kernel::*;
use serde_json::json;

type K = Kernel<MemState, FakeHost>;

/// On macOS (APFS: names match without case by default and without normalization always),
/// `Ma\u{212A}efile` (KELVIN SIGN, canonically `K`) and `AGENT\u{17F}.md` (LONG S, case-folds to
/// `s`) open `Makefile` and `AGENTS.md`. `eq_ignore_ascii_case` misses both, so the write gets
/// the key `<ws>/**` and a session allow answers it without asking.
#[test]
fn adv_s34_unicode_spelling_of_a_named_file_gets_a_key() {
    let ws = Some("/w");
    for name in [
        "Ma\u{212A}efile",
        "AGENT\u{17F}.md",
        "package.j\u{17F}on",
        "build.r\u{17F}",
    ] {
        let path = format!("/w/{name}");
        for (tool, input) in [
            ("Write", json!({"file_path": path, "content": "x"})),
            (
                "Edit",
                json!({"file_path": path, "old_string": "a", "new_string": "b"}),
            ),
        ] {
            assert_eq!(
                scope(tool, ws, &input),
                None,
                "{tool} {name:?} writes the protected file on macOS and must ask"
            );
        }
    }
}

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

fn ask(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": id,
                "toolCall": {"toolCallId": id, "title": "Edit"},
                "input": {}, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
}

fn held(k: &K) -> i64 {
    let kinds = [
        "session", "queue", "entries", "wire", "loop", "stream", "live", "key", "owner",
    ];
    k.state
        .entries()
        .filter(|(key, _)| kinds.contains(&key.split('/').next().unwrap_or_default()))
        .map(|(key, v)| (key.len() + v.len()) as i64)
        .sum()
}

/// The fix charges a restart step only on an early firing. A firing with its `idle` deadline due
/// runs the same step and ends in `finish`, which fails `quota` at the ceiling after the writes are in the
/// state and before the accounts are saved: the step's entries stay, uncharged (9.3 "Past": a
/// firing cannot fail, and charges what it wrote).
#[test]
fn adv_s34_due_firing_after_restart_at_ceiling_is_not_charged() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    // Six open asks: the restart step writes a decision entry for each.
    for n in ["a1", "a2", "a3", "a4", "a5", "a6"] {
        k.provider_item("t1", ask(n)).unwrap();
    }
    let mut r = Record::load(&mut k.state, &id).unwrap();
    r.deadlines.push(Deadline {
        what: "idle".into(),
        at: k.host.now,
    });
    r.save(&mut k.state).unwrap();
    k.host.generation = 2;
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
    let (o0, a0, h0) = (
        Usage::load(&mut k.state, "quota/alice").bytes as i64,
        Usage::load(&mut k.state, "quota").bytes as i64,
        held(&k),
    );
    k.timer(&format!("s/{id}"));
    let r = Record::load(&mut k.state, &id).unwrap();
    assert_eq!(r.boot, 2, "the step ran");
    let wrote = held(&k) - h0;
    assert!(wrote > 0, "the step wrote past the ceiling: {wrote}");
    assert_eq!(
        Usage::load(&mut k.state, "quota/alice").bytes as i64 - o0,
        wrote,
        "the owner is charged what the firing wrote"
    );
    assert_eq!(
        Usage::load(&mut k.state, "quota").bytes as i64 - a0,
        wrote,
        "the account is charged what the firing wrote"
    );
}
