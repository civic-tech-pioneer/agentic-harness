//! S34-3: the kernel never asks a timer past 30 days (TECHNICAL 7 "Kernel timers", 9.3); a far
//! deadline is still met, by a firing that sets the timer again.

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const DAY: u64 = 86_400_000;
const MAX: u64 = 2_592_000_000;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    crate::fixture_access::put(&mut st, "access/alice", br#"["p"]"#.to_vec()).unwrap();
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

/// A session whose one `idle` deadline is `days` ahead, the timer set the way a turn end sets it.
fn far(k: &mut K, days: u64) -> String {
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider(
        &id,
        ProviderEvent::Turn(Turn::Idle {
            stop: Stop::Completed,
            seconds: None,
            tools: None,
            tokens: None,
        }),
    )
    .unwrap();
    // The idle deadline moved in the record; the test's first firing (an early one) arms it.
    let mut r = Record::load(&mut k.state, &id).unwrap();
    r.deadlines.clear();
    r.deadlines.push(Deadline {
        what: "idle".into(),
        at: k.host.now + days * DAY,
    });
    r.save(&mut k.state).unwrap();
    id
}

fn timer_calls(k: &K, from: usize) -> Vec<(String, u64)> {
    k.host.calls[from..]
        .iter()
        .filter_map(|c| match c {
            HostCall::TimerAfter { id, delay_ms } => Some((id.clone(), *delay_ms)),
            _ => None,
        })
        .collect()
}

fn snapshot(k: &K) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

/// The bytes of the keys a session's owner is charged for (`session` only for the record's own).
fn held(k: &K, session: bool) -> i64 {
    let kinds: &[&str] = if session {
        &["queue", "entries", "wire", "loop", "stream", "live"]
    } else {
        &[
            "session", "queue", "entries", "wire", "loop", "stream", "live", "key", "owner",
        ]
    };
    k.state
        .entries()
        .filter(|(key, _)| kinds.contains(&key.split('/').next().unwrap_or_default()))
        .map(|(key, v)| (key.len() + v.len()) as i64)
        .sum()
}

#[test]
fn ac1_ac2_a_deadline_40_days_ahead_arms_30_then_10_then_fires() {
    let mut k = kernel();
    let id = far(&mut k, 40);
    let t = format!("s/{id}");
    // The first firing (any early one) arms the timer for the earliest deadline, clamped.
    let from = k.host.calls.len();
    k.timer(&t);
    assert_eq!(timer_calls(&k, from), vec![(t.clone(), MAX)]);
    // Day 30: nothing handled, no state key written, the timer set for the 10 days left.
    k.host.now += MAX;
    let before = snapshot(&k);
    let from = k.host.calls.len();
    k.timer(&t);
    assert_eq!(timer_calls(&k, from), vec![(t.clone(), 10 * DAY)]);
    assert_eq!(before, snapshot(&k));
    // Day 40: handled as any due deadline.
    k.host.now += 10 * DAY;
    k.timer(&t);
    assert!(Record::load(&mut k.state, &id)
        .unwrap()
        .deadlines
        .is_empty());
    assert_eq!(Slots::load(&mut k.state).timers, 0);
}

#[test]
fn ac3_an_early_firing_rearms_and_changes_nothing_else() {
    let mut k = kernel();
    let id = far(&mut k, 1);
    let t = format!("s/{id}");
    k.host.now += 1000;
    let before = snapshot(&k);
    let from = k.host.calls.len();
    k.timer(&t);
    assert_eq!(timer_calls(&k, from), vec![(t, DAY - 1000)]);
    assert_eq!(before, snapshot(&k));
    // A session with no deadlines still asks nothing.
    let mut r = Record::load(&mut k.state, &id).unwrap();
    r.deadlines.clear();
    r.save(&mut k.state).unwrap();
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    assert_eq!(timer_calls(&k, from), vec![]);
}

#[test]
fn ac4_a_delay_under_30_days_is_asked_unchanged() {
    let mut k = kernel();
    let id = far(&mut k, 29);
    let t = format!("s/{id}");
    let from = k.host.calls.len();
    k.timer(&t);
    assert_eq!(timer_calls(&k, from), vec![(t, 29 * DAY)]);
}

#[test]
fn fix1_ac3_an_early_firing_keeps_the_restart_step_it_ran() {
    let mut k = kernel();
    let id = far(&mut k, 1);
    let t = format!("s/{id}");
    k.host.generation = 2;
    k.host.now += 1000;
    let mut slots = Slots::load(&mut k.state);
    slots.boot = 2;
    slots.save(&mut k.state).unwrap();
    // The stale keys of the older boot, planted as the restart tests do.
    let mut r = Record::load(&mut k.state, &id).unwrap();
    assert!(r.boot < 2);
    r.streams = vec!["x1".into()];
    r.save(&mut k.state).unwrap();
    let live = format!("live/{id}");
    k.state.put("stream/x1", b"{}".to_vec()).unwrap();
    k.state.put(&live, b"x1".to_vec()).unwrap();
    let (o0, a0, r0) = (
        Usage::load(&mut k.state, "quota/alice").bytes as i64,
        Usage::load(&mut k.state, "quota").bytes as i64,
        Record::load(&mut k.state, &id).unwrap().bytes as i64,
    );
    let (h0, s0) = (held(&k, false), held(&k, true));
    let from = k.host.calls.len();
    k.timer(&t);
    assert_eq!(timer_calls(&k, from), vec![(t.clone(), DAY - 1000)]);
    let r = Record::load(&mut k.state, &id).unwrap();
    assert_eq!(r.boot, 2);
    assert!(k.state.get(&live).is_none());
    assert!(k.state.get("stream/x1").is_none());
    // R4/S29: both accounts take what the step wrote, the record's `bytes` its session keys.
    let wrote = held(&k, false) - h0;
    assert!(wrote < 0, "the step deleted the stale keys");
    assert_eq!(
        Usage::load(&mut k.state, "quota/alice").bytes as i64 - o0,
        wrote
    );
    assert_eq!(Usage::load(&mut k.state, "quota").bytes as i64 - a0, wrote);
    assert_eq!(r.bytes as i64 - r0, held(&k, true) - s0);
    let before = snapshot(&k);
    let from = k.host.calls.len();
    k.timer(&t);
    assert_eq!(timer_calls(&k, from), vec![(t, DAY - 1000)]);
    assert_eq!(before, snapshot(&k));
}
