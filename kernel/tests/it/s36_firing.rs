//! S36-2: a session's timer firing never stops short (TECHNICAL 9.3 "Past", "Deadlines and the
//! session timer"): it always sets the timer again and charges what it wrote, past the reserve too.

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const TEN: u64 = 600_000;

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
        generation: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn idle() -> ProviderEvent {
    ProviderEvent::Turn(Turn::Idle {
        stop: Stop::Completed,
        seconds: None,
        tools: None,
        tokens: None,
    })
}

fn bytes(k: &mut K, key: &str) -> u64 {
    Usage::load(&mut k.state, key).bytes
}

#[test]
fn ac1_a_failed_restart_step_still_sets_the_timer_and_charges_what_it_wrote() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    // the step's `turn` entry has no room: its write fails
    let mut r = rec(&mut k, &id);
    r.chunks = (1..=agent_kernel::transcript::MAX_CHUNKS as u64).collect();
    r.deadlines.push(Deadline {
        what: "idle".into(),
        at: 1000 + TEN,
    });
    r.save(&mut k.state).unwrap();
    let (rec0, owner0, account0) = (
        rec(&mut k, &id).bytes,
        bytes(&mut k, &Usage::owner_key("alice")),
        bytes(&mut k, "quota"),
    );
    k.host.generation = 2;
    k.host.now = 1000 + TEN;
    k.host.calls.clear();
    k.timer(&format!("s/{id}"));
    assert!(
        k.host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::TimerAfter { id: i, .. } if *i == format!("s/{id}"))),
        "{:?}",
        k.host.calls
    );
    // the record is still of the older boot, so a later call does its step (R6)
    let r = rec(&mut k, &id);
    assert_eq!(r.boot, 1);
    assert_eq!(r.deadlines.len(), 1);
    // what the step wrote is in the record and both accounts, by the same amount
    let (rec1, owner1, account1) = (
        r.bytes,
        bytes(&mut k, &Usage::owner_key("alice")),
        bytes(&mut k, "quota"),
    );
    assert_eq!(owner1 - owner0, account1 - account0);
    assert_eq!(rec1 - rec0, owner1 - owner0);
}

#[test]
fn ac2_a_due_idle_firing_at_the_ceiling_releases_the_stream_and_commits() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.provider(&id, idle()).unwrap();
    assert_eq!(Slots::load(&mut k.state).provider, 1);
    k.limits.account_bytes = bytes(&mut k, "quota") + 1;
    k.limits.reserve_bytes = 0;
    k.host.now = 1000 + TEN;
    k.host.calls.clear();
    k.timer(&format!("s/{id}"));
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::CancelStream { .. })));
    assert_eq!(Slots::load(&mut k.state).provider, 0);
    let r = rec(&mut k, &id);
    assert!(r.streams.is_empty());
    assert!(r.deadlines.iter().all(|d| d.what != "idle"));
}

#[test]
fn ac3_a_firing_with_nothing_due_writes_nothing() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.provider(&id, idle()).unwrap();
    k.host.now = 2000;
    k.state.reset_counts();
    k.timer(&format!("s/{id}"));
    let c = k.state.counts();
    assert_eq!((c.puts, c.deletes), (0, 0));
}
