//! User simulation, S33: a kernel maker drives session allows through the public API.

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

fn started(k: &mut K, ws: Option<&str>) -> String {
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    if let Some(w) = ws {
        let mut r = rec(k, &id);
        r.folder = Some(w.into());
        r.save(&mut k.state).unwrap();
    }
    id
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn ask(k: &mut K, a: &mut ClaudeAdapter, s: &str, id: &str, tool: &str, input: Value) {
    let mut l = json!({"type": "ask", "id": id, "tool": tool, "input": input,
        "tool_use_id": format!("t-{id}"), "expires-at": 9000});
    // The path the host would resolve: the input's own, as given (no symlinks here).
    let p = ["file_path", "notebook_path", "path"]
        .iter()
        .find_map(|k| l["input"].get(*k).and_then(Value::as_str))
        .map(str::to_string);
    if let Some(p) = p {
        l["path"] = json!(p);
    }
    for out in a.line(l.to_string().as_bytes(), 1000) {
        k.provider(s, ProviderEvent::Out(out)).unwrap();
    }
}

fn resolve(k: &mut K, a: &mut ClaudeAdapter, s: &str, id: &str) {
    let l = json!({"type": "ask-resolved", "id": id, "by": "answered", "outcome": "allow"});
    for out in a.line(l.to_string().as_bytes(), 1000) {
        k.provider(s, ProviderEvent::Out(out)).unwrap();
    }
}

fn ask_entry(k: &mut K, s: &str, id: &str) -> Value {
    k.read("alice", s, 0, None, 512)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .find(|e| e["kind"] == "ask" && e["body"]["id"] == id && !e["body"]["decision"].is_object())
        .unwrap_or_else(|| panic!("no ask entry {id}"))
}

fn by(k: &mut K, s: &str, id: &str) -> Option<String> {
    rec(k, s).pending_answers.get(id).map(|p| p.by.clone())
}

fn is_open(k: &mut K, s: &str, id: &str) -> bool {
    rec(k, s).open.iter().any(|o| o.id == id)
}

#[test]
fn usersim_s33_no_session_allow_for_wide_tools() {
    for (tool, input) in [
        (
            "Task",
            json!({"description": "x", "prompt": "do", "subagent_type": "general-purpose"}),
        ),
        ("mcp__watane__notes_read", json!({"id": "n1"})),
        ("WebSearch", json!({"query": "rust"})),
        ("SomeUnlistedTool", json!({"x": 1})),
    ] {
        let mut k = kernel();
        let s = started(&mut k, Some("/w"));
        let mut a = ClaudeAdapter::new();
        ask(&mut k, &mut a, &s, "q1", tool, input.clone());
        let e = ask_entry(&mut k, &s, "q1");
        assert!(
            e["body"].get("reach").is_none(),
            "{tool}: reach stored: {e}"
        );
        let r = k.answer("alice", &s, "k1", "q1", SESSION);
        assert!(r.is_err(), "{tool}: session answer accepted");
        assert!(rec(&mut k, &s).allows.is_empty(), "{tool}");
        assert!(
            is_open(&mut k, &s, "q1"),
            "{tool}: ask closed by a refused answer"
        );
    }
}

#[test]
fn usersim_s33_bare_name_allow_kept() {
    let (tool, input) = ("TodoWrite", json!({"todos": []}));
    {
        let mut k = kernel();
        let s = started(&mut k, Some("/w"));
        let mut a = ClaudeAdapter::new();
        ask(&mut k, &mut a, &s, "q1", tool, input.clone());
        let e = ask_entry(&mut k, &s, "q1");
        assert_eq!(e["body"]["reach"], json!(tool), "{tool}: {e}");
        k.answer("alice", &s, "k1", "q1", SESSION).unwrap();
        assert_eq!(rec(&mut k, &s).allows[0].tool, tool);
        resolve(&mut k, &mut a, &s, "q1");
        ask(&mut k, &mut a, &s, "q2", tool, input);
        assert_eq!(
            by(&mut k, &s, "q2").as_deref(),
            Some("session-allow"),
            "{tool}"
        );
    }
}

fn bash_allowed(first: Value) -> (K, ClaudeAdapter, String) {
    let mut k = kernel();
    let s = started(&mut k, Some("/w"));
    let mut a = ClaudeAdapter::new();
    ask(&mut k, &mut a, &s, "a1", "Bash", first);
    k.answer("alice", &s, "k1", "a1", SESSION).unwrap();
    resolve(&mut k, &mut a, &s, "a1");
    (k, a, s)
}

#[test]
fn usersim_s33_bash_pins_background_and_timeout() {
    let cases: [(Value, Vec<Value>, Vec<Value>); 3] = [
        (
            json!({"command": "npm test", "description": "run tests"}),
            vec![
                json!({"command": "npm test", "run_in_background": true}),
                json!({"command": "npm test", "timeout": 600000}),
                json!({"command": "npm test", "run_in_background": false}),
            ],
            vec![
                json!({"command": "npm test"}),
                json!({"command": "npm test", "description": "other words"}),
            ],
        ),
        (
            json!({"command": "npm test", "timeout": 1000}),
            vec![
                json!({"command": "npm test", "timeout": 2000}),
                json!({"command": "npm test"}),
                json!({"command": "npm test", "timeout": 1000, "run_in_background": true}),
            ],
            vec![json!({"command": "npm test", "timeout": 1000, "description": "d"})],
        ),
        (
            json!({"command": "npm test", "run_in_background": true}),
            vec![
                json!({"command": "npm test", "run_in_background": false}),
                json!({"command": "npm test"}),
            ],
            vec![json!({"command": "npm test", "run_in_background": true, "description": "z"})],
        ),
    ];
    for (first, other, same) in cases {
        for o in other {
            let (mut k, mut a, s) = bash_allowed(first.clone());
            ask(&mut k, &mut a, &s, "b", "Bash", o.clone());
            assert!(
                is_open(&mut k, &s, "b"),
                "after {first}, {o} not opened for owner"
            );
            assert_eq!(by(&mut k, &s, "b"), None, "after {first}, {o}");
        }
        for o in same {
            let (mut k, mut a, s) = bash_allowed(first.clone());
            ask(&mut k, &mut a, &s, "c", "Bash", o.clone());
            assert_eq!(
                by(&mut k, &s, "c").as_deref(),
                Some("session-allow"),
                "after {first}, {o}"
            );
        }
    }
}

#[test]
fn usersim_s33_glob_absolute_pattern_asks_owner() {
    let mut k = kernel();
    let s = started(&mut k, Some("/w"));
    let mut a = ClaudeAdapter::new();
    ask(
        &mut k,
        &mut a,
        &s,
        "g1",
        "Glob",
        json!({"path": "/w/src", "pattern": "*.rs"}),
    );
    k.answer("alice", &s, "k1", "g1", SESSION).unwrap();
    assert_eq!(rec(&mut k, &s).allows[0].key, "/w/**");
    resolve(&mut k, &mut a, &s, "g1");
    ask(
        &mut k,
        &mut a,
        &s,
        "g2",
        "Glob",
        json!({"path": "/w/src", "pattern": "**/*.toml"}),
    );
    assert_eq!(by(&mut k, &s, "g2").as_deref(), Some("session-allow"));
    resolve(&mut k, &mut a, &s, "g2");
    for (i, p) in ["/home/*/.ssh/*", "~/.ssh/*", "../../etc/*", "/w/**"]
        .iter()
        .enumerate()
    {
        let id = format!("h{i}");
        ask(
            &mut k,
            &mut a,
            &s,
            &id,
            "Glob",
            json!({"path": "/w/src", "pattern": p}),
        );
        assert_eq!(by(&mut k, &s, &id), None, "pattern {p} answered by allow");
        assert!(is_open(&mut k, &s, &id), "{p}");
    }
}
