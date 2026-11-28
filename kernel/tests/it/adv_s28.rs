//! S28 adversary: red tests for attacks that worked.

use agent_kernel::active::Slots;
use agent_kernel::*;

type K = Kernel<MemState, FakeHost>;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put(
        "profile/p",
        br#"{"mode":"manual","tools":["Read"],"max-turns":20}"#.to_vec(),
    )
    .unwrap();
    crate::fixture_access::put(&mut st, "access/alice", br#"["p"]"#.to_vec()).unwrap();
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

/// With the kernel's account at its 256 MiB ceiling (224 MiB + reserve), a provider stream's
/// `ended` on a `running` turn now has to write the `turn` entry `failed {provider-ended}`, so
/// the item call fails `quota` and is dropped whole: the session stays `running` with nothing
/// driving it, and the dead stream keeps its `stream/` key and its provider slot (kernel's and
/// owner's) until a restart or `delete`. Before S28 the same `ended` freed key and slot.
#[test]
fn adv_s28_ended_at_the_ceiling_keeps_the_dead_stream() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    // The account exactly at its ceiling: any call that adds a byte fails `quota`.
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
    let r = k.provider_item("t1", ProviderEvent::Ended);
    let rec = Record::load(&mut k.state, &id).unwrap();
    assert!(
        k.state.get("stream/t1").is_none() && Slots::load(&mut k.state).provider == 0,
        "the ended stream keeps its key and slot ({:?})",
        r.map(|_| ()).map_err(|e| e.to_string())
    );
    assert!(
        !matches!(rec.turn, Turn::Running { .. } | Turn::Waiting { .. }),
        "the turn stays {:?} with no provider stream",
        rec.turn
    );
}
