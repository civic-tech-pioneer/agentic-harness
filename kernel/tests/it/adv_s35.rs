//! Adversary, S35: a later launch (9.5) is built from the stored profile with no check (9.2, 11).

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::Value;

const P: &str =
    r#"{"backend":"claude","model":"sonnet","tools":["Read"],"deny":["Read(./secret/**)"]}"#;
const IN: &str = r#"[{"type":"text","text":"hello"}]"#;
const WS: &str = "scratch:0123456789abcdef0123456789abcdef";

fn kernel() -> Kernel<MemState, FakeHost> {
    let mut st = MemState::default();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    st.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    Kernel::new(
        st,
        FakeHost {
            generation: 1,
            now: 1000,
            ..FakeHost::default()
        },
    )
}

/// A session of `p` whose provider stream has ended: its next `say` launches again.
fn released(k: &mut Kernel<MemState, FakeHost>) -> String {
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let init = format!(r#"{{"type":"initialized","workspace":"{WS}"}}"#);
    let lines = include_str!("../fixtures/adapters/fake_default.jsonl")
        .lines()
        .skip(1)
        .map(|l| l.as_bytes().to_vec());
    let mut batch = vec![(1, init.into_bytes())];
    batch.extend((2..).zip(lines));
    k.provider_lines(&t, batch).unwrap();
    k.provider_ended(&t).unwrap();
    k.host.calls.clear();
    id
}

fn last_launch(k: &Kernel<MemState, FakeHost>) -> Option<Value> {
    k.host.calls.iter().rev().find_map(|c| match c {
        HostCall::StartProvider { launch, .. } => Some(serde_json::from_str(launch).unwrap()),
        _ => None,
    })
}

/// The participant's profile now says `deny` in a shape `check` refuses (a bare string). `start`
/// refuses it; the relaunch of a running session drops the deny and keeps the tools.
#[test]
fn adv_s35_relaunch_drops_a_deny_check_would_refuse() {
    let mut k = kernel();
    let id = released(&mut k);
    let edited =
        r#"{"backend":"claude","model":"sonnet","tools":["Read"],"deny":"Read(./secret/**)"}"#;
    k.state
        .put("profile/p", edited.as_bytes().to_vec())
        .unwrap();
    assert!(
        matches!(
            k.start("alice", "k2", "p", "{}", IN),
            Err(KernelError::Refused(_))
        ),
        "start refuses the edited profile"
    );
    k.host.calls.clear();
    let said = k.say("alice", &id, "s1", "again");
    if said.is_err() {
        return; // the relaunch refused too: held
    }
    let launch = last_launch(&k).expect("a relaunch");
    assert!(
        launch["deny"].is_array(),
        "relaunch ran tools {} without the participant's deny: {launch}",
        launch["tools"]
    );
}

/// The refuse table (4.2): Bash needs a sandbox. `start` refuses such a profile; the relaunch of
/// a running session launches Bash with no sandbox.
#[test]
fn adv_s35_relaunch_skips_the_refuse_table() {
    let mut k = kernel();
    let id = released(&mut k);
    let edited = r#"{"backend":"claude","model":"sonnet","tools":["Read","Bash"]}"#;
    k.state
        .put("profile/p", edited.as_bytes().to_vec())
        .unwrap();
    assert!(matches!(
        k.start("alice", "k2", "p", "{}", IN),
        Err(KernelError::Refused(_))
    ));
    k.host.calls.clear();
    if k.say("alice", &id, "s1", "again").is_err() {
        return;
    }
    let launch = last_launch(&k).expect("a relaunch");
    let bash = launch["tools"]
        .as_array()
        .is_some_and(|t| t.iter().any(|x| x == "Bash"));
    assert!(
        !bash || launch.get("sandbox").is_some(),
        "relaunch gave Bash with no sandbox: {launch}"
    );
}

fn dump(k: &Kernel<MemState, FakeHost>) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

#[test]
fn ac2_a_refused_relaunch_by_say_or_interrupt_changes_nothing_and_starts_nothing() {
    let mut k = kernel();
    let id = released(&mut k);
    let edited = r#"{"backend":"claude","model":"sonnet","tools":["Read","Bash"]}"#;
    k.state
        .put("profile/p", edited.as_bytes().to_vec())
        .unwrap();
    let want = match k.start("alice", "k2", "p", "{}", IN) {
        Err(KernelError::Refused(r)) => r,
        other => panic!("{other:?}"),
    };
    k.host.calls.clear();
    let before = dump(&k);
    match k.say("alice", &id, "s1", "again") {
        Err(KernelError::Refused(r)) => assert_eq!(r, want),
        other => panic!("{other:?}"),
    }
    match k.interrupt("alice", &id, Some(IN)) {
        Err(KernelError::Refused(r)) => assert_eq!(r, want),
        other => panic!("{other:?}"),
    }
    assert_eq!(dump(&k), before);
    assert!(last_launch(&k).is_none());
}

#[test]
fn ac4_a_stored_profile_scratch_needs_access_page() {
    let mut k = kernel();
    k.state
        .put("profile/scratch", br#"{"model":"haiku"}"#.to_vec())
        .unwrap();
    assert!(matches!(
        k.start("page", "k1", "scratch", "{}", IN),
        Err(KernelError::Refused(r)) if r[0].field == "profile"
    ));
    k.state
        .put("access/page", br#"["scratch"]"#.to_vec())
        .unwrap();
    k.start("page", "k2", "scratch", "{}", IN).unwrap();
}

/// The relaunch is a Claude stream: it is checked by Claude's refuse table even when the edited
/// profile now names another backend (whose table would let `ask-expiry` pass).
#[test]
fn validator_a_relaunch_is_checked_by_the_sessions_backend() {
    let mut k = kernel();
    let id = released(&mut k);
    let edited = r#"{"backend":"openai","model":"gpt-5","ask-expiry":30}"#;
    k.state
        .put("profile/p", edited.as_bytes().to_vec())
        .unwrap();
    match k.say("alice", &id, "s1", "again") {
        Err(KernelError::Refused(r)) => assert_eq!(r[0].field, "ask-expiry"),
        other => panic!("{other:?}"),
    }
    assert!(last_launch(&k).is_none());
}
