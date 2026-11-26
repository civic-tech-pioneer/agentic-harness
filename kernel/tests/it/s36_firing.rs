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
    let chunks = std::mem::take(&mut r.chunks);
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
    // five seconds before the idle deadline: the step's retry is armed for exactly that wait
    k.host.now = 1000 + TEN - 5000;
    k.host.calls.clear();
    k.state.reset_counts();
    k.timer(&format!("s/{id}"));
    // the one write the failed firing made: the pass's `slots` (boot 1 -> 2); nothing of the
    // step itself (its `turn` entry had no room) stays
    let c = k.state.counts();
    assert_eq!((c.puts, c.deletes), (1, 0));
    assert!(c.bytes_written > 0);
    assert_eq!(Slots::load(&mut k.state).boot, 2);
    let delays: Vec<u64> = k
        .host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::TimerAfter { id: i, delay_ms } if *i == format!("s/{id}") => Some(*delay_ms),
            _ => None,
        })
        .collect();
    assert_eq!(delays, vec![5000], "{:?}", k.host.calls);
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
    // the step's own writes were lost, so the record and accounts are charged exactly nothing
    assert_eq!(rec1, rec0);
    // once room is made, a later call of the owner does the step and is served (R6)
    let mut r = rec(&mut k, &id);
    r.chunks = chunks;
    r.save(&mut k.state).unwrap();
    let page = k
        .read("alice", &id, 0, None, 5)
        .expect("a later call succeeds");
    assert!(
        page.entries.iter().any(|e| e.contains("hi")),
        "{:?}",
        page.entries
    );
    assert_eq!(rec(&mut k, &id).boot, 2);
}

#[test]
fn ac2_a_due_idle_firing_at_the_ceiling_releases_the_stream_and_commits() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.provider(&id, idle()).unwrap();
    assert_eq!(Slots::load(&mut k.state).provider, 1);
    // the account sits exactly at its cap (`>=` is past, 9.3), the reserve gone
    k.limits.account_bytes = bytes(&mut k, "quota");
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
    // the fixture really is at the account's cap: the firing charged its own writes, so pin the cap to
    // exactly the bytes held now (equal is past, `>=`, 9.3). A rename to an empty title writes no more
    // bytes than it frees, so only the cap decision can refuse it (S131 r1 F1.6: a `say` is refused at
    // equality for want of room too, whatever the comparison); one byte under the cap it goes through.
    let held = bytes(&mut k, "quota");
    k.limits.account_bytes = held;
    let refused = k.rename("alice", &id, "");
    assert!(matches!(refused, Err(KernelError::Quota(_))), "{refused:?}");
    k.limits.account_bytes = held + 1;
    k.rename("alice", &id, "").unwrap();
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
