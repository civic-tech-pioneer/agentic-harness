//! Adversary, S32: attacks on the wrapper's stream routing and numbered batches (TECHNICAL 2.1, 9.4, 14).

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;

fn kernel() -> K {
    let mut st = MemState::new();
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

fn claude_kernel() -> K {
    let mut k = kernel();
    k.state
        .put(
            "profile/p",
            br#"{"mode":"manual","tools":["Read"],"max-turns":20}"#.to_vec(),
        )
        .unwrap();
    k.state.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
    k
}

/// `stream-sink.ended` returns nothing, so it cannot defer (host 19.1); the wrapper
/// (`kernel/wasm/src/component.rs`) maps it to `provider_item(s, Ended)` and drops the answer.
/// With the owner's live stream full (a page that stopped reading), `provider_item` answers
/// `Defer` and discards every write: the provider stream's end is lost for good. The session
/// stays `running` with nothing driving it, the dead stream keeps its `stream/` key and its
/// provider slot (kernel's and owner's), against 9.4 "Outside an item call it cannot defer:
/// it closes that stream `lagging`".
#[test]
fn adv_s32_ended_on_a_full_owner_stream_is_lost() {
    let mut k = claude_kernel();
    let id = k
        .start("alice", "k1", "p", "{}", r#"[{"type":"text","text":"hi"}]"#)
        .unwrap()
        .value;
    let o = match k.attach("alice", &id, 0).unwrap().value {
        Attached::Live { stream } => stream,
        other => panic!("{other:?}"),
    };
    let t = Record::load(&mut k.state, &id)
        .unwrap()
        .streams
        .into_iter()
        .find(|s| *s != o)
        .expect("the provider stream");
    k.host.full.insert(o.clone());
    let r = k.provider_item(&t, ProviderEvent::Ended).unwrap().value;
    let rec = Record::load(&mut k.state, &id).unwrap();
    assert!(
        r != SinkOutcome::Defer
            && k.state.get(&format!("stream/{t}")).is_none()
            && Slots::load(&mut k.state).provider == 0
            && !matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }),
        "ended answered {r:?}; the stream key stays: {}; provider slots {}; turn {:?}",
        k.state.get(&format!("stream/{t}")).is_some(),
        Slots::load(&mut k.state).provider,
        rec.turn
    );
}
