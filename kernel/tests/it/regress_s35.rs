//! S35 user-sim (Alice on watane's host, slice/s99 9ba0c13 with the foreman's fake `claude`):
//! failures seen on the real programs, pinned at the kernel crate.

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::Value;

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"backend":"claude","model":"sonnet","tools":["Read"],"allow":["Read(**)"],"partial":true,"max-tokens":5}"#;
const IN: &str = r#"[{"type":"text","text":"hello"}]"#;
const WS: &str = "scratch:0123456789abcdef0123456789abcdef";
const UUID: &str = "5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11";

fn kernel() -> K {
    let mut st = MemState::default();
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

fn fake_lines() -> Vec<Vec<u8>> {
    include_str!("../fixtures/adapters/fake_default.jsonl")
        .lines()
        .map(|l| l.as_bytes().to_vec())
        .collect()
}

/// Alice's session after its first turn ended idle, her owner stream attached:
/// (session, provider stream, owner stream).
fn idle_session(k: &mut K) -> (String, String, String) {
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let Attached::Live { stream: o } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("not live");
    };
    let init = format!(r#"{{"type":"initialized","workspace":"{WS}"}}"#);
    let sys = format!(r#"{{"type":"system","subtype":"init","session_id":"{UUID}"}}"#);
    let mut batch = vec![(1, init.into_bytes()), (2, sys.into_bytes())];
    batch.extend((3..).zip(fake_lines().into_iter().skip(1)));
    k.provider_lines(&t, batch).unwrap();
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Idle { .. }
    ));
    (id, t, o)
}

/// The entry indexes the owner stream got: an `entry` item's `i`, or an update's `_meta.watane.i`.
fn live_indexes(k: &K, owner: &str, from: usize) -> Vec<u64> {
    k.host.calls[from..]
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream, value } if stream == owner => {
                let w = &value["_meta"]["watane"];
                if w["type"] == "entry" {
                    w["entry"]["i"].as_u64()
                } else {
                    w["i"].as_u64()
                }
            }
            _ => None,
        })
        .collect()
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

/// On the host, Alice's second message in a session ("turn two") never showed while the turn ran:
/// the owner stream got the `turn` entry (i 7) and the agent's reply, but nothing with i 6, her
/// own message; it appeared only after a reload. Every stored entry goes out live once with its
/// index (9.4, S35-fix1), and the user's message has no other way to the page.
#[test]
fn regress_s35_say_user_message_goes_out_live() {
    let mut k = kernel();
    let (id, _, o) = idle_session(&mut k);
    let from = k.host.calls.len();
    let n = k.say("alice", &id, "s2", "turn two").unwrap().value;
    let msg = entries(&mut k, &id)
        .into_iter()
        .find(|e| e["kind"] == "message" && e["body"]["content"][0]["text"] == "turn two")
        .expect("her message is stored");
    let i = msg["i"].as_u64().unwrap();
    let live = live_indexes(&k, &o, from);
    assert!(
        live.contains(&i),
        "say returned {n}; her message is entry {i}, the owner stream got only {live:?}"
    );
}

/// On the host, `say("/x")` on an idle Claude session returned ok, stored the message and a
/// `running` turn, and the session stayed Working for good: `claude-session.say` refuses
/// `slash-command` (4.5) and the component drops that error (2.1), so no turn end ever comes; a
/// later `say` only queued. Case 11 of 16: `say("/x")` is refused.
#[test]
fn regress_s35_say_slash_command_is_refused() {
    let mut k = kernel();
    let (id, _, _) = idle_session(&mut k);
    let before = entries(&mut k, &id).len();
    let r = k.say("alice", &id, "s2", "/x");
    assert!(
        r.is_err(),
        "say(\"/x\") was accepted: {:?}",
        r.map(|c| c.value)
    );
    assert_eq!(
        entries(&mut k, &id).len(),
        before,
        "a refused say stores nothing"
    );
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Idle { .. }
    ));
}
