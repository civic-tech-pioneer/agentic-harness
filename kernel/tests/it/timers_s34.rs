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
    let mut r = Record::load(&mut k.state, &id).unwrap();
    r.deadlines.clear();
    r.save(&mut k.state).unwrap();
    // A call that sets the deadline: put it in the record, then let a firing-free path arm it.
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
