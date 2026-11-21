//! Adversary tests of S21 (sessions, queue, active index): each shows an attack that works.

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
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

/// 9.3 Sizes: the queue holds 32 inputs and 2 MiB, "so the `queue` update and `queued` carry at
/// most 2 MiB of inputs, under the 4 MiB item". An input of valid ACP JSON whose text is quotes
/// doubles when the queue stores it as a JSON string: eight inputs of 256 KiB (2 MiB, within
/// the cap) make a `queue/<id>` value and a queue push over 4 MiB. The host refuses the put, so a
/// client (or the queue it shares with an agent) gets `too-large(4194304)` where the queue still
/// had room, and the push item would not fit the 4 MiB item either.
#[test]
fn adv_s21_queue_of_escaped_inputs_passes_the_host_value() {
    let mut k = kernel();
    let id = k.start("alice", "k0", "p", "{}", IN).unwrap().value;
    let head = r#"[{"type":"text","text":""#;
    let tail = r#""}]"#;
    let quotes = (INPUT_BYTES - head.len() - tail.len()) / 2;
    let input = format!("{head}{}{tail}", "\\\"".repeat(quotes));
    assert!(input.len() <= INPUT_BYTES);
    assert!(serde_json::from_str::<serde_json::Value>(&input).is_ok());
    assert!(8 * input.len() <= QUEUE_BYTES);
    for n in 1..=8u64 {
        let r = k.say("alice", &id, &format!("q{n}"), &input);
        assert_eq!(
            r.map(|c| c.value),
            Ok(n),
            "input {n} of 8 (2 MiB in all) refused"
        );
    }
    assert_eq!(k.queued("alice", &id).unwrap().len(), 8);
}

/// 9.1 `access/<identity>`: "the profile names this client may use"; AGT-F-1: only the
/// participant says which client may use which profile. `start` reads `profile/<name>` for any
/// caller: a client the participant gave only `p` starts a session on `shell`, a profile made
/// for another client (say, one with file writes), so it runs what the participant never let it.
#[test]
fn adv_s21_start_uses_a_profile_the_client_has_no_access_to() {
    let mut k = kernel();
    let shell = r#"{"mode":"manual","tools":["Read","Edit","Write"],"max-turns":20}"#;
    k.state
        .put("profile/shell", shell.as_bytes().to_vec())
        .unwrap();
    k.state.put("access/bob", br#"["shell"]"#.to_vec()).unwrap();
    k.state.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
    let r = k.start("alice", "k", "shell", "{}", IN);
    assert!(
        matches!(&r, Err(KernelError::Refused(v)) if v.iter().any(|x| x.field == "profile")),
        "alice started a session on a profile only bob may use: {r:?}"
    );
    // and a client with no access entry at all
    let r = k.start("mallory", "k", "shell", "{}", IN);
    assert!(
        matches!(&r, Err(KernelError::Refused(_))),
        "a client with no access started a session: {r:?}"
    );
}
