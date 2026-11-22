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

fn dump(k: &K) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn say_text(k: &mut K, id: &str, key: &str, input: &str) -> Result<(), KernelError> {
    k.say("alice", id, key, input).map(|_| ())
}

fn slash_refusal(r: Result<(), KernelError>) {
    match r {
        Err(KernelError::Refused(v)) => {
            assert_eq!(v.len(), 1);
            assert_eq!(
                (v[0].field.as_str(), v[0].reason.as_str()),
                ("input", "slash-command")
            );
        }
        other => panic!("not refused slash-command: {other:?}"),
    }
}

/// AC3: the slash rule at `start`, `say` and an `interrupt` replacement, white space as host 19.3.
#[test]
fn ac3_slash_rule_at_start_say_and_interrupt() {
    let refused_inputs = [
        "/x",
        " /x",
        "\u{feff}/x",
        "\u{200b}/x",
        "\t/x",
        "\u{a0}/x",
        "\u{2028}/x",
        r#"[{"type":"text","text":" /x"}]"#,
        r#"[{"type":"image","data":"AA==","mimeType":"image/png"},{"type":"text","text":"/x"}]"#,
    ];
    for (n, input) in refused_inputs.iter().enumerate() {
        let mut k = kernel();
        let before_start = dump(&k);
        slash_refusal(k.start("alice", "k9", "p", "{}", input).map(|_| ()));
        assert!(dump(&k) == before_start, "start wrote for {input:?}");
        let (id, _, _) = idle_session(&mut k);
        let before = dump(&k);
        slash_refusal(say_text(&mut k, &id, &format!("s{n}"), input));
        assert!(dump(&k) == before, "say wrote for {input:?}");
        slash_refusal(k.interrupt("alice", &id, Some(input)).map(|_| ()));
        assert!(dump(&k) == before, "interrupt wrote for {input:?}");
    }
    for input in ["a /x", "x/"] {
        let mut k = kernel();
        let (id, _, _) = idle_session(&mut k);
        say_text(&mut k, &id, "s1", input).unwrap();
        let mut k = kernel();
        k.start("alice", "k9", "p", "{}", input).unwrap();
    }
}

/// AC2: the live `i` set equals `read`'s over two turns, each once; a queued input goes out the same.
#[test]
fn ac2_live_indexes_equal_reads_over_two_turns() {
    let mut k = kernel();
    let from = k.host.calls.len();
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
    k.say("alice", &id, "s2", "turn two").unwrap();
    // a queued input: it joins while turn two runs and starts when that ends
    k.say("alice", &id, "s3", "turn three").unwrap();
    let batch: Vec<_> = (100..).zip(fake_lines()).collect();
    k.provider_lines(&t, batch).unwrap();
    let all = entries(&mut k, &id);
    let mut live = live_indexes(&k, &o, from);
    live.sort();
    let mut read: Vec<u64> = all.iter().map(|e| e["i"].as_u64().unwrap()).collect();
    read.sort();
    let kinds: Vec<&str> = all.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(live, read, "{kinds:?}");
    let second = all
        .iter()
        .position(|e| e["body"]["content"][0]["text"] == "turn two")
        .unwrap();
    assert_eq!(all[second + 1]["kind"], "turn");
}

/// AC4: an idle turn that ends a running one carries its whole seconds, rounded.
#[test]
fn ac4_idle_turn_entry_carries_seconds() {
    // (ms the turn ran, its whole seconds rounded): 3 400 ms gives 3, 3 500 ms gives 4
    for (ms, seconds) in [(3400, 3), (3500, 4)] {
        let mut k = kernel();
        let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
        let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
        k.host.now = 1000 + ms;
        let c = k.provider_item(&t, ProviderEvent::Ended).unwrap();
        let last = entries(&mut k, &id).pop().unwrap();
        assert_eq!(last["kind"], "turn");
        assert_eq!(last["body"]["seconds"], seconds, "entry after {ms} ms");
        // the `turn` item carries it too
        let item = c.pushes.iter().find(|p| p.value["type"] == "turn").unwrap();
        assert_eq!(
            item.value["state"]["seconds"], seconds,
            "item after {ms} ms"
        );
    }
}
