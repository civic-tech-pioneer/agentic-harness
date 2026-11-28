//! Adversary tests of S21 (sessions, queue, active index): each shows an attack that works.

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    for who in ["alice", "bob"] {
        crate::fixture_access::put(&mut st, &format!("access/{who}"), br#"["p"]"#.to_vec())
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
    let mut full = false;
    for n in 1..=8u64 {
        match k.say("alice", &id, &format!("q{n}"), &input) {
            Ok(_) => {}
            Err(KernelError::QueueFull) => full = true,
            Err(e) => panic!("input {n}: {e:?}, not Ok or QueueFull"),
        }
        let stored = k.state.get(&format!("queue/{id}")).map_or(0, |b| b.len());
        assert!(
            stored <= QUEUE_BYTES + 65_536,
            "stored queue {stored} bytes"
        );
        let pushed = serde_json::to_vec(&Queue::load(&mut k.state, &id).push_value())
            .unwrap()
            .len();
        assert!(pushed <= QUEUE_BYTES + 65_536, "queue push {pushed} bytes");
    }
    assert!(full, "escape-heavy inputs never reached QueueFull");
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
    crate::fixture_access::put(&mut k.state, "access/bob", br#"["shell"]"#.to_vec()).unwrap();
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p"]"#.to_vec()).unwrap();
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

/// S21-fix1 AC2: the cap counts stored bytes. Control characters escape 6x (`\u0001`).
#[test]
fn ac2_queue_cap_counts_stored_bytes() {
    let mut k = kernel();
    let id = k.start("alice", "k0", "p", "{}", IN).unwrap().value;
    // raw control characters: 256 KiB / 6 of them stored as 6 bytes each
    let raw = format!("[\"{}\"]", "\u{1}".repeat(INPUT_BYTES - 4));
    assert!(raw.len() <= INPUT_BYTES);
    let mut res = Vec::new();
    for n in 1..=3 {
        res.push(k.say("alice", &id, &format!("c{n}"), &raw));
    }
    assert!(
        res.iter().any(|r| matches!(r, Err(KernelError::QueueFull))),
        "{res:?}"
    );
    let stored = k.state.get(&format!("queue/{id}")).unwrap().len();
    assert!(stored <= QUEUE_BYTES + 65_536, "{stored}");
    // plain text still fits 8 inputs
    let mut k = kernel();
    let id = k.start("alice", "k0", "p", "{}", IN).unwrap().value;
    let plain = format!("[\"{}\"]", "a".repeat(INPUT_BYTES - 64 - 4));
    for n in 1..=8u64 {
        assert_eq!(
            k.say("alice", &id, &format!("p{n}"), &plain)
                .map(|c| c.value),
            Ok(n)
        );
    }
}

fn is_key(e: KernelError) -> bool {
    matches!(e, KernelError::Refused(r) if r.len() == 1 && r[0].field == "key")
}

/// AC1: an access value that is not a JSON array of strings allows none, and a refusal writes nothing.
#[test]
fn ac1_unreadable_access_allows_none_and_writes_nothing() {
    let mut k = kernel();
    k.state.reset_counts();
    for bad in [&b"not json"[..], br#"{"p":true}"#, br#"[1]"#, br#""p""#] {
        k.state.put("access/alice", bad.to_vec()).unwrap();
        let r = k.start("alice", "k", "p", "{}", IN);
        assert!(
            matches!(&r, Err(KernelError::Refused(v)) if v.len() == 1 && v[0].field == "profile"),
            "{r:?}"
        );
    }
    assert!(k.state.get("key/alice/k").is_none());
    assert!(k.state.get("owner/alice/0").is_none());
    assert_eq!(k.host.next_id, 0);
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["q"]"#.to_vec()).unwrap();
    k.state.reset_counts();
    for who in ["alice", "carol"] {
        let r = k.start(who, "k", "p", "{}", IN);
        assert!(
            matches!(&r, Err(KernelError::Refused(v)) if v.len() == 1 && v[0].field == "profile"),
            "{r:?}"
        );
    }
    assert_eq!(k.state.counts().puts, 0);
    assert_eq!(k.host.next_id, 0);
}

/// AC2: owner `page/x` key `k` and owner `page` key `x/k` are separate keys.
#[test]
fn ac2_owners_with_slashes_do_not_share_a_state_key() {
    let mut k = kernel();
    for who in ["page/x", "page", "100%"] {
        crate::fixture_access::put(&mut k.state, &format!("access/{who}"), br#"["p"]"#.to_vec())
            .unwrap();
    }
    let a = k.start("page/x", "k", "p", "{}", IN).unwrap().value;
    let b = k.start("page", "x/k", "p", "{}", IN).unwrap().value;
    assert_ne!(a, b);
    assert_eq!(k.start("page/x", "k", "p", "{}", IN).unwrap().value, a);
    assert_eq!(k.start("page", "x/k", "p", "{}", IN).unwrap().value, b);
    assert!(k.state.get("key/page%2Fx/k").is_some());
    let c = k.start("100%", "k", "p", "{}", IN).unwrap().value;
    assert!(k.state.get("key/100%25/k").is_some() && k.state.get("owner/100%25/0").is_some());
    assert_ne!(c, a);
}

/// AC3: a kept key is bound to its session; a replay naming another is refused, a foreign one is NotOwner.
#[test]
fn ac3_a_replay_names_the_session_it_acted_on() {
    let mut k = kernel();
    let s1 = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let s2 = k.start("alice", "k2", "p", "{}", IN).unwrap().value;
    let bobs = k.start("bob", "k3", "p", "{}", IN).unwrap().value;
    let n = k.say("alice", &s1, "s", IN).unwrap().value;
    assert_eq!(k.say("alice", &s1, "s", IN).unwrap().value, n);
    let before = k.state.get(&format!("session/{s2}"));
    let calls = k
        .host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::Request(_)))
        .count();
    assert!(is_key(k.say("alice", &s2, "s", IN).unwrap_err()));
    assert!(is_key(
        k.answer("alice", &s2, "s", "a", r#"{"allow":true}"#)
            .unwrap_err()
    ));
    assert_eq!(k.state.get(&format!("session/{s2}")), before);
    assert_eq!(
        k.host
            .calls
            .iter()
            .filter(|c| matches!(c, HostCall::Request(_)))
            .count(),
        calls
    );
    assert_eq!(
        k.say("bob", &s1, "s", IN).unwrap_err(),
        KernelError::NotOwner
    );
    assert_eq!(
        k.say("alice", &bobs, "s", IN).unwrap_err(),
        KernelError::NotOwner
    );
    // the caller's own deleted session still replays its kept result
    k.delete("alice", &s1).unwrap();
    assert_eq!(k.say("alice", &s1, "s", IN).unwrap().value, n);
}

/// AC4: a push from the restart timer names its session.
#[test]
fn ac4_a_push_names_its_session() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    let c = k.say("alice", &id, "s", IN).unwrap();
    assert!(c.pushes.iter().all(|p| p.session == id));
    assert!(!c.pushes.is_empty());
}
