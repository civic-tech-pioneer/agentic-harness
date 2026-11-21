//! Adversary, S31: attacks on session allows (TECHNICAL 9.6, 14).

use agent_kernel::active::Slots;
use agent_kernel::adapters::claude::ClaudeAdapter;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const SESSION: &str = r#"{"allow":true,"for":"session"}"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put(
        "profile/p",
        br#"{"mode":"manual","tools":["Read"],"max-turns":20}"#.to_vec(),
    )
    .unwrap();
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

fn started(k: &mut K, who: &str) -> String {
    let id = k.start(who, "k", "p", "{}", IN).unwrap().value;
    k.attach(who, &id, 0).unwrap();
    id
}

fn feed(k: &mut K, a: &mut ClaudeAdapter, id: &str, line: Value) {
    for out in a.line(line.to_string().as_bytes(), 1000) {
        k.provider(id, ProviderEvent::Out(out)).unwrap();
    }
}

fn ask(k: &mut K, a: &mut ClaudeAdapter, s: &str, id: &str, kind: &str, tool: &str, input: Value) {
    let line = json!({"type": kind, "id": id, "tool": tool, "input": input,
        "tool_use_id": format!("t-{id}"), "expires-at": 9000});
    feed(k, a, s, line);
}

fn answers(k: &K) -> Vec<(String, Value)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Answer { ask, decision, .. }) => {
                Some((ask.clone(), decision.clone()))
            }
            _ => None,
        })
        .collect()
}

fn kernel_allowed(k: &K, id: &str) -> bool {
    answers(k)
        .iter()
        .any(|(a, d)| a == id && d.get("allow") == Some(&json!(true)))
}

/// The owner allows `ls` (sandboxed) for the session; then the agent asks to run the same command
/// with `dangerouslyDisableSandbox: true`. The owner never saw or allowed an unsandboxed run, yet
/// the kernel answers it `allow` from the session allow: the allow widens from "this command in the
/// sandbox" to "this command outside the sandbox" (the reach looks at `command` only).
#[test]
fn adv_s31_bash_allow_covers_sandbox_escape() {
    let mut k = kernel();
    let s = started(&mut k, "alice");
    let mut a = ClaudeAdapter::new();
    ask(
        &mut k,
        &mut a,
        &s,
        "a1",
        "ask",
        "Bash",
        json!({"command": "ls"}),
    );
    k.answer("alice", &s, "k1", "a1", SESSION).unwrap();
    feed(
        &mut k,
        &mut a,
        &s,
        json!({"type":"ask-resolved","id":"a1","by":"answered","outcome":"allow"}),
    );
    ask(
        &mut k,
        &mut a,
        &s,
        "a2",
        "ask",
        "Bash",
        json!({"command": "ls", "dangerouslyDisableSandbox": true}),
    );
    assert!(
        !kernel_allowed(&k, "a2"),
        "an unsandboxed run was allowed from a sandboxed session allow"
    );
}
