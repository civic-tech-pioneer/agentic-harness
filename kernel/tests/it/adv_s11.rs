//! Adversary probes of S11 (9.4 bounds).

use agent_kernel::state::MemState;
use agent_kernel::transcript::{Meta, NewEntry, Replay, Transcript};
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
    let page = t.read(u64::MAX, None, 256).unwrap();
    assert!(page.entries.is_empty(), "got {:?}", page.entries);
    let page = t.read(u64::MAX, Some(u64::MAX), 256).unwrap();
    assert!(page.entries.is_empty(), "got {:?}", page.entries);
}

fn filled(n: u64, st: &mut MemState, meta: &mut Meta) {
    let mut t = Transcript::new(st, "s", meta);
    for k in 0..n {
        t.append(&msg(json!({"t": k})), 1).unwrap();
    }
}

#[test]
fn adv_s11_read_empty_ranges_return_nothing() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    filled(8, &mut st, &mut meta);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    for (a, b) in [
        (0, Some(0)),
        (0, Some(1)),
        (5, Some(3)),
        (u64::MAX, None),
        (u64::MAX, Some(u64::MAX)),
    ] {
        let p = t.read(a, b, 256).unwrap();
        assert!(p.entries.is_empty(), "{a} {b:?}: {:?}", p.entries);
        assert_eq!(p.last, 8);
    }
}

#[test]
fn adv_s11_replay_after_max_is_empty() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    filled(3, &mut st, &mut meta);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.replay(u64::MAX), Replay::Entries(vec![]));
}

#[test]
fn adv_s11_read_max_u32_is_held_to_256() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    filled(300, &mut st, &mut meta);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(0, None, u32::MAX).unwrap().entries.len(), 256);
}

#[test]
fn adv_s11_empty_transcript_extremes() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    for (a, b, m) in [
        (0, None, 256),
        (u64::MAX, None, 256),
        (u64::MAX, Some(u64::MAX), u32::MAX),
        (0, Some(0), 1),
        (5, Some(3), u32::MAX),
        (0, None, u32::MAX),
    ] {
        let p = t.read(a, b, m).unwrap();
        assert!(p.entries.is_empty());
        assert_eq!(p.last, 0);
    }
    assert_eq!(t.replay(u64::MAX), Replay::Entries(vec![]));
    assert_eq!(t.replay(0), Replay::Entries(vec![]));
}
