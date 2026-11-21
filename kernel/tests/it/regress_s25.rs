//! Acceptance regressions of S25 (9.3 Quota; FUNCTIONAL AGT-F-13: past its quota the kernel
//! refuses new sessions "and names the quota").

use agent_kernel::active::Slots;
use agent_kernel::*;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> Kernel<MemState, FakeHost> {
    let mut st = MemState::new();
    st.put(
        "profile/p",
        br#"{"mode":"manual","tools":["Read"],"max-turns":20}"#.to_vec(),
    )
    .unwrap();
    for who in ["alice", "bob"] {
        st.put(&format!("access/{who}"), br#"["p"]"#.to_vec())
            .unwrap();
    }
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    Kernel::new(
        st,
        FakeHost {
            boot: 1,
            now: 1000,
            ..FakeHost::default()
        },
    )
}

/// An owner with one session that is past its 64 MiB is told its "session quota is used up":
/// the text names the wrong cap (it has 1 of 64 sessions), so the mod cannot tell that only a
/// `delete` of bytes helps. The refusal should name the bytes cap.
#[test]
fn regress_s25_quota_message_names_the_bytes_cap() {
    let mut k = kernel();
    k.start("alice", "k1", "p", "{}", IN).unwrap();
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    let e = k.start("alice", "k2", "p", "{}", IN).unwrap_err();
    assert!(matches!(e, KernelError::Quota(_)));
    let text = e.to_string();
    assert!(
        !text.contains("session quota") && (text.contains("bytes") || text.contains("MiB")),
        "an owner past its bytes cap is told: {text}"
    );
}

/// Past the kernel's 224 MiB account, each owner under its own caps is told "the mod's session
/// quota is used up": neither its sessions nor its bytes are the cause.
#[test]
fn regress_s25_quota_message_names_the_account() {
    let mut k = kernel();
    k.start("alice", "k1", "p", "{}", IN).unwrap();
    k.start("bob", "k1", "p", "{}", IN).unwrap();
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    let e = k.start("bob", "k2", "p", "{}", IN).unwrap_err();
    assert!(matches!(e, KernelError::Quota(_)));
    let text = e.to_string();
    assert!(
        text == "quota: this mod's agent sessions are at their limit"
            && !["kernel", "account", "bytes", "MiB"]
                .iter()
                .any(|w| text.contains(w)),
        "an owner refused for the kernel's account is told: {text}"
    );
}
