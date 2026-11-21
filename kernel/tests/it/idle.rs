//! S26-3: an idle Claude session gives its provider stream back after 10 minutes (TECHNICAL 9.3
//! "Deadlines and the session timer", 9.5 "Slots", case 52); a failed opening call leaks no stream.

use agent_kernel::active::Slots;
use agent_kernel::adapters::Out;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const NP: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20,"persist":false}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const TEN: u64 = 600_000;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    st.put("profile/np", NP.as_bytes().to_vec()).unwrap();
    for who in ["alice", "bob"] {
        st.put(&format!("access/{who}"), br#"["p","np"]"#.to_vec())
            .unwrap();
    }
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

fn start(k: &mut K, owner: &str, key: &str) -> String {
    k.start(owner, key, "p", "{}", IN).unwrap().value
}

fn done() -> ProviderEvent {
    ProviderEvent::Turn(Turn::Idle {
        stop: Stop::Completed,
        seconds: None,
        tools: None,
        tokens: None,
    })
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn slots(k: &mut K) -> Slots {
    Slots::load(&mut k.state)
}

fn timers(k: &mut K, owner: &str) -> u32 {
    Usage::load(&mut k.state, &Usage::owner_key(owner)).timers
}

fn idle_at(r: &Record) -> Vec<u64> {
    r.deadlines
        .iter()
        .filter(|d| d.what == "idle")
        .map(|d| d.at)
        .collect()
}

/// The host calls from `from` on that touch a timer or a stream.
fn host_calls(k: &K, from: usize) -> Vec<HostCall> {
    k.host.calls[from..]
        .iter()
        .filter(|c| !matches!(c, HostCall::Generation | HostCall::Request(_)))
        .cloned()
        .collect()
}

fn turn_requests(k: &K) -> Vec<bool> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Turn { resume, .. }) => Some(*resume),
            _ => None,
        })
        .collect()
}

/// AC8: the timer counts are those of the sessions that hold a deadline.
fn check(k: &mut K, owners: &[&str]) {
    let held: Vec<(String, String)> = k
        .state
        .entries()
        .filter(|(key, _)| key.starts_with("session/"))
        .map(|(_, v)| serde_json::from_slice::<Record>(v).unwrap())
        .filter(|r| !r.deadlines.is_empty())
        .map(|r| (r.id, r.owner))
        .collect();
    assert_eq!(slots(k).timers as usize, held.len(), "{held:?}");
    for o in owners {
        let mine = held.iter().filter(|(_, w)| w == o).count();
        assert_eq!(timers(k, o) as usize, mine, "{o}");
    }
}

#[test]
fn ac1_a_turn_that_ends_idle_sets_the_deadline_and_the_timer() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    let from = k.host.calls.len();
    k.provider(&id, done()).unwrap();
    let r = rec(&mut k, &id);
    assert_eq!(idle_at(&r), vec![1000 + TEN]);
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: TEN
        }]
    );
    assert_eq!(slots(&mut k).timers, 1);
    assert_eq!(timers(&mut k, "alice"), 1);
    check(&mut k, &["alice"]);
}

#[test]
fn ac1_out_idle_and_interrupt_and_a_quota_stop_set_it_too() {
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    k.provider(
        &a,
        ProviderEvent::Out(Out {
            idle: true,
            ..Out::default()
        }),
    )
    .unwrap();
    assert_eq!(idle_at(&rec(&mut k, &a)), vec![1000 + TEN]);
    let b = start(&mut k, "alice", "k2");
    k.host.now = 2000;
    k.interrupt("alice", &b, None).unwrap();
    assert_eq!(idle_at(&rec(&mut k, &b)), vec![2000 + TEN]);
    let c = start(&mut k, "alice", "k3");
    k.provider(
        &c,
        ProviderEvent::Turn(Turn::Idle {
            stop: Stop::Stopped {
                limit: "quota".into(),
            },
            seconds: None,
            tools: None,
            tokens: None,
        }),
    )
    .unwrap();
    assert_eq!(idle_at(&rec(&mut k, &c)).len(), 1);
    check(&mut k, &["alice"]);
}

#[test]
fn ac1_a_later_idle_replaces_the_earlier_one_and_costs_no_second_timer() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    k.host.now = 5000;
    k.say("alice", &id, "q", "again").unwrap();
    k.provider(&id, done()).unwrap();
    assert_eq!(idle_at(&rec(&mut k, &id)), vec![5000 + TEN]);
    assert_eq!(slots(&mut k).timers, 1);
    check(&mut k, &["alice"]);
}

#[test]
fn ac1_an_end_that_starts_the_next_queued_input_sets_no_deadline() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.say("alice", &id, "q", "next").unwrap();
    let from = k.host.calls.len();
    k.provider(&id, done()).unwrap();
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(host_calls(&k, from), vec![]);
    assert_eq!(slots(&mut k).timers, 0);
}

#[test]
fn ac2_a_say_removes_the_deadline_and_frees_the_timer() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    let from = k.host.calls.len();
    k.say("alice", &id, "q", "more").unwrap();
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerCancel {
            id: format!("s/{id}")
        }]
    );
    assert_eq!(slots(&mut k).timers, 0);
    assert_eq!(timers(&mut k, "alice"), 0);
}

#[test]
fn ac2_an_interrupt_replacement_removes_it_and_a_say_that_queues_keeps_it() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    k.interrupt("alice", &id, Some("again")).unwrap();
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(slots(&mut k).timers, 0);
    // a running turn: the say only queues, nothing about timers
    let from = k.host.calls.len();
    k.say("alice", &id, "q", "queued").unwrap();
    assert_eq!(host_calls(&k, from), vec![]);
    check(&mut k, &["alice"]);
}

#[test]
fn ac2_with_other_deadlines_left_the_timer_is_rearmed_for_the_earliest() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    let mut r = rec(&mut k, &id);
    r.deadlines.push(Deadline {
        what: "retry".into(),
        at: 1000 + TEN / 2,
    });
    r.save(&mut k.state).unwrap();
    let from = k.host.calls.len();
    k.say("alice", &id, "q", "more").unwrap();
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: TEN / 2
        }]
    );
    assert_eq!(slots(&mut k).timers, 1);
}

#[test]
fn ac3_ac4_the_timer_releases_the_stream_and_the_next_say_resumes() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    let t = rec(&mut k, &id).streams[0].clone();
    k.host.now = 1000 + TEN;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    let r = rec(&mut k, &id);
    assert!(r.deadlines.is_empty() && r.streams.is_empty());
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::CancelStream { stream: t.clone() }]
    );
    assert!(k.state.get(&format!("stream/{t}")).is_none());
    assert_eq!(slots(&mut k).provider, 0);
    assert_eq!(slots(&mut k).timers, 0);
    assert_eq!(timers(&mut k, "alice"), 0);
    let usage = Usage::load(&mut k.state, &Usage::owner_key("alice"));
    assert_eq!(usage.provider_at(1), 0);
    k.say("alice", &id, "q", "wake").unwrap();
    assert_eq!(
        k.host.calls[from..]
            .iter()
            .filter(|c| matches!(c, HostCall::Open { .. }))
            .count(),
        1
    );
    assert_eq!(turn_requests(&k).last(), Some(&true));
    check(&mut k, &["alice"]);
}

#[test]
fn ac4_persist_false_resumes_nothing() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "np", "{}", IN).unwrap().value;
    k.provider(&id, done()).unwrap();
    k.host.now = 1000 + TEN;
    k.timer(&format!("s/{id}"));
    k.say("alice", &id, "q", "wake").unwrap();
    assert_eq!(turn_requests(&k).last(), Some(&false));
}

#[test]
fn ac3_a_firing_for_a_deleted_session_or_before_the_deadline_changes_nothing() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    let before: Vec<_> = k
        .state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    k.host.now = 2000;
    k.timer(&format!("s/{id}"));
    let after: Vec<_> = k
        .state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    assert_eq!(before, after, "not due yet");
    k.delete("alice", &id).unwrap();
    let before: Vec<_> = k
        .state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    k.timer(&format!("s/{id}"));
    let after: Vec<_> = k
        .state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    assert_eq!(before, after, "deleted");
    k.timer("s/never-was");
}

#[test]
fn ac3_a_due_deadline_of_another_stage_stays_and_the_timer_follows_it() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    let mut r = rec(&mut k, &id);
    r.deadlines.push(Deadline {
        what: "retry".into(),
        at: 1000 + TEN + 5,
    });
    r.save(&mut k.state).unwrap();
    k.host.now = 1000 + TEN;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    let r = rec(&mut k, &id);
    assert_eq!(r.deadlines.len(), 1);
    assert_eq!(r.deadlines[0].what, "retry");
    assert!(r.streams.is_empty());
    assert_eq!(
        host_calls(&k, from)[1..],
        [HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: 5
        }]
    );
    assert_eq!(slots(&mut k).timers, 1);
}

#[test]
fn ac5_at_the_kernel_cap_the_stream_is_released_at_once() {
    let mut k = kernel();
    k.limits.timers = 1;
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "bob", "k1");
    k.provider(&a, done()).unwrap();
    let t = rec(&mut k, &b).streams[0].clone();
    let from = k.host.calls.len();
    k.provider(&b, done()).unwrap();
    let r = rec(&mut k, &b);
    assert!(r.deadlines.is_empty() && r.streams.is_empty());
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::CancelStream { stream: t }]
    );
    assert_eq!(slots(&mut k).timers, 1);
    assert_eq!(slots(&mut k).provider, 1);
    check(&mut k, &["alice", "bob"]);
}

#[test]
fn ac5_at_the_owner_cap_too_but_a_session_with_a_timer_keeps_it() {
    let mut k = kernel();
    k.limits.owner_timers = 1;
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "alice", "k2");
    let c = start(&mut k, "bob", "k1");
    k.provider(&a, done()).unwrap();
    k.provider(&b, done()).unwrap();
    assert!(rec(&mut k, &b).deadlines.is_empty());
    assert!(rec(&mut k, &b).streams.is_empty());
    k.provider(&c, done()).unwrap();
    assert_eq!(rec(&mut k, &c).deadlines.len(), 1);
    // a session that already holds its timer replaces its deadline at the cap
    k.say("alice", &a, "q", "x").unwrap();
    k.provider(&a, done()).unwrap();
    assert_eq!(rec(&mut k, &a).deadlines.len(), 1);
    check(&mut k, &["alice", "bob"]);
}

#[test]
fn ac6_cancel_and_delete_remove_the_deadlines_and_the_timer() {
    let mut k = kernel();
    let a = start(&mut k, "alice", "k1");
    let b = start(&mut k, "alice", "k2");
    let c = start(&mut k, "alice", "k3");
    k.provider(&a, done()).unwrap();
    k.provider(&b, done()).unwrap();
    assert_eq!(slots(&mut k).timers, 2);
    let from = k.host.calls.len();
    k.cancel("alice", &a).unwrap();
    assert!(rec(&mut k, &a).deadlines.is_empty());
    assert!(host_calls(&k, from).contains(&HostCall::TimerCancel {
        id: format!("s/{a}")
    }));
    let from = k.host.calls.len();
    k.delete("alice", &b).unwrap();
    assert!(host_calls(&k, from).contains(&HostCall::TimerCancel {
        id: format!("s/{b}")
    }));
    assert_eq!(slots(&mut k).timers, 0);
    assert_eq!(timers(&mut k, "alice"), 0);
    // a record with no deadline drops no count
    let from = k.host.calls.len();
    k.delete("alice", &c).unwrap();
    assert!(!host_calls(&k, from)
        .iter()
        .any(|h| matches!(h, HostCall::TimerCancel { .. })));
    check(&mut k, &["alice"]);
}

#[test]
fn ac7_a_restart_keeps_the_counts_and_a_stale_idle_firing_releases_nothing() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    k.host.generation = 2;
    k.host.now = 1000 + TEN;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    assert!(host_calls(&k, from)
        .iter()
        .all(|h| !matches!(h, HostCall::CancelStream { .. })));
    let r = rec(&mut k, &id);
    assert!(r.deadlines.is_empty() && r.streams.is_empty());
    assert_eq!(slots(&mut k).timers, 0);
    assert_eq!(timers(&mut k, "alice"), 0);
    assert_eq!(slots(&mut k).provider, 0);
}

#[test]
fn ac7_a_restart_alone_leaves_the_timer_counts() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    k.host.generation = 2;
    k.say("bob-none", "x", "k", "y").unwrap_err();
    assert_eq!(slots(&mut k).timers, 1);
    assert_eq!(timers(&mut k, "alice"), 1);
}

#[test]
fn ac9_a_start_that_fails_past_the_reserve_cancels_its_stream() {
    let mut k = kernel();
    start(&mut k, "alice", "k1");
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes + 1;
    k.limits.reserve_bytes = 0;
    let from = k.host.calls.len();
    let e = k.start("bob", "k1", "p", "{}", IN).unwrap_err();
    assert!(matches!(e, KernelError::Quota(_)), "{e}");
    let calls = host_calls(&k, from);
    assert_eq!(
        calls,
        vec![
            HostCall::Open {
                stream: "t2".into()
            },
            HostCall::CancelStream {
                stream: "t2".into()
            }
        ]
    );
    assert!(k.host.calls[from..]
        .iter()
        .all(|c| !matches!(c, HostCall::Request(_))));
}

#[test]
fn ac9_a_say_that_fails_after_opening_cancels_its_stream() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    k.host.now = 1000 + TEN;
    k.timer(&format!("s/{id}"));
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes + 1;
    k.limits.reserve_bytes = 0;
    let from = k.host.calls.len();
    let e = k.say("alice", &id, "q", &"x".repeat(2000)).unwrap_err();
    assert!(matches!(e, KernelError::Quota(_)), "{e}");
    assert_eq!(
        host_calls(&k, from),
        vec![
            HostCall::Open {
                stream: "t2".into()
            },
            HostCall::CancelStream {
                stream: "t2".into()
            }
        ]
    );
}

// Validator (S26-3 round 1): the timer follows only the deadlines this build handles.

fn with_retry(k: &mut K, id: &str, at: u64) {
    let mut r = rec(k, id);
    r.deadlines.push(Deadline {
        what: "retry".into(),
        at,
    });
    r.save(&mut k.state).unwrap();
}

#[test]
fn ac3_a_firing_before_the_deadline_only_sets_the_timer_again() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    k.host.now = 2000;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: TEN - 1000
        }]
    );
}

#[test]
fn ac3_a_due_deadline_of_a_later_stage_stays_counted_and_arms_no_timer() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    with_retry(&mut k, &id, 1000 + TEN);
    k.host.now = 1000 + TEN;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    let r = rec(&mut k, &id);
    assert_eq!(r.deadlines.len(), 1);
    assert_eq!(r.deadlines[0].what, "retry");
    assert!(r.streams.is_empty());
    // the stream goes; no timer for the retry that waits for its stage
    assert!(host_calls(&k, from)
        .iter()
        .all(|h| matches!(h, HostCall::CancelStream { .. })));
    assert_eq!(slots(&mut k).timers, 1);
    check(&mut k, &["alice"]);
    // a second firing: nothing to do, nothing re-armed
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    assert_eq!(host_calls(&k, from), vec![]);
    check(&mut k, &["alice"]);
    // with one still to come, a firing arms the timer for it alone
    with_retry(&mut k, &id, 1000 + TEN + 70);
    k.timer(&format!("s/{id}"));
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: 70
        }]
    );
    check(&mut k, &["alice"]);
}

#[test]
fn ac2_the_timer_is_rearmed_for_the_earliest_of_several_left() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    with_retry(&mut k, &id, 1000 + 90);
    with_retry(&mut k, &id, 1000 + 50);
    let from = k.host.calls.len();
    k.say("alice", &id, "q", "more").unwrap();
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: 50
        }]
    );
    check(&mut k, &["alice"]);
}

#[test]
fn ac5_a_session_that_holds_its_timer_sets_idle_at_the_cap() {
    let mut k = kernel();
    k.limits.owner_timers = 1;
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    with_retry(&mut k, &id, 1000 + 2 * TEN);
    k.say("alice", &id, "q", "more").unwrap();
    assert_eq!(timers(&mut k, "alice"), 1);
    k.provider(&id, done()).unwrap();
    let r = rec(&mut k, &id);
    assert_eq!(idle_at(&r), vec![1000 + TEN]);
    assert_eq!(r.streams.len(), 1);
    check(&mut k, &["alice"]);
}

#[test]
fn ac2_a_turn_the_provider_starts_removes_the_deadline() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    let from = k.host.calls.len();
    k.provider(
        &id,
        ProviderEvent::Turn(Turn::Running {
            since: 1000,
            activity: None,
        }),
    )
    .unwrap();
    assert!(rec(&mut k, &id).deadlines.is_empty());
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerCancel {
            id: format!("s/{id}")
        }]
    );
    check(&mut k, &["alice"]);
}

#[test]
fn ac3_a_due_deadline_of_a_later_stage_releases_nothing() {
    let mut k = kernel();
    let id = start(&mut k, "alice", "k1");
    k.provider(&id, done()).unwrap();
    with_retry(&mut k, &id, 2000);
    k.host.now = 2000;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    let r = rec(&mut k, &id);
    assert_eq!(r.streams.len(), 1);
    assert_eq!(r.deadlines.len(), 2);
    assert_eq!(
        host_calls(&k, from),
        vec![HostCall::TimerAfter {
            id: format!("s/{id}"),
            delay_ms: TEN - 1000
        }]
    );
    check(&mut k, &["alice"]);
}
