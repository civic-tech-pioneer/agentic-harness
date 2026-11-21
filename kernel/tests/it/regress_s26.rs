//! User-sim S26: an owner stream the kernel closes itself (`lagging` outside an item call,
//! `replaced` by a second `attach`) while the session's provider stream is deferred on it must
//! resume that provider stream (TECHNICAL 3 "Forwarding": on any outcome the mod resumes every
//! deferred upstream; 9.4 "A full owner stream"; case 51: the deferred upstream stays alive).
//! Today the kernel deletes the owner stream's keys when it closes it, so the host's later
//! `ended` finds no key and nothing ever resumes the provider stream: it stays deferred until
//! the host's 5-minute backstop ends it `consumer-failed`.

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

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
    Kernel::new(
        st,
        FakeHost {
            generation: 1,
            now: 1000,
            ..FakeHost::default()
        },
    )
}

fn done() -> ProviderEvent {
    ProviderEvent::Turn(Turn::Idle {
        stop: Stop::Completed,
        seconds: None,
        tools: None,
        tokens: None,
    })
}

/// A session with a live owner stream on which its provider stream's item was deferred.
fn deferred() -> (K, String, String, String) {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let Attached::Live { stream: o } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("attach opened no stream")
    };
    k.host.full.insert(o.clone());
    assert_eq!(k.provider_item(&t, done()).unwrap().value, Item::Defer);
    (k, id, t, o)
}

fn resumed(k: &K, t: &str) -> bool {
    k.host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::Resume { stream } if stream == t))
}

#[test]
fn regress_s26_lagging_close_resumes_the_deferred_provider() {
    let (mut k, id, t, o) = deferred();
    // A `say` outside an item call finds the owner stream full: the kernel closes it `lagging`.
    k.say("alice", &id, "q1", IN).unwrap();
    assert!(k.host.calls.contains(&HostCall::Close {
        stream: o.clone(),
        outcome: "lagging".into()
    }));
    k.host.full.clear();
    // The host then reports that stream's end to its producer.
    k.stream_ended(&o);
    assert!(
        resumed(&k, &t),
        "provider stream {t} stays deferred after its owner stream {o} ended lagging"
    );
}

#[test]
fn regress_s26_replaced_close_resumes_the_deferred_provider() {
    let (mut k, id, t, o) = deferred();
    k.host.full.clear();
    // The page reattaches: the kernel closes the old owner stream `replaced`.
    k.attach("alice", &id, 0).unwrap();
    k.stream_ended(&o);
    assert!(
        resumed(&k, &t),
        "provider stream {t} stays deferred after its owner stream {o} was replaced"
    );
}
