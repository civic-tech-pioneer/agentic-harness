//! Adversary probes of S11 (9.4 bounds).

use agent_kernel::state::MemState;
use agent_kernel::transcript::{Meta, NewEntry, Transcript};
use serde_json::{json, Value};

fn msg(body: Value) -> NewEntry {
    NewEntry {
        kind: "message".into(),
        uuid: Some("u\n\"x".into()),
        body,
    }
}

#[test]
fn adv_s11_read_after_max_returns_nothing() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    for k in 0..3 {
        t.append(&msg(json!({"t": k})), 1).unwrap();
    }
    // No entry has i > u64::MAX: the page must be empty, and the call must not trap.
    let page = t.read(u64::MAX, None, 256);
    assert!(page.entries.is_empty(), "got {:?}", page.entries);
    let page = t.read(u64::MAX, Some(u64::MAX), 256);
    assert!(page.entries.is_empty(), "got {:?}", page.entries);
}
