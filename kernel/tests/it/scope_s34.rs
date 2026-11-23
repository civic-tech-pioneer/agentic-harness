//! S34-1: scope keys of session allows (TECHNICAL 9.6): `broker::scope`/`reach` per tool, and the
//! kernel matching a fresh key against the held `{id, tool, key}` pairs.

use agent_kernel::active::Slots;
use agent_kernel::adapters::claude::ClaudeAdapter;
use agent_kernel::broker::{reach, scope};
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

/// A session with no owner stream attached.
fn bare(k: &mut K) -> String {
    k.start("alice", "k", "p", "{}", IN).unwrap().value
}

fn started(k: &mut K) -> String {
    let id = bare(k);
    k.attach("alice", &id, 0).unwrap();
    id
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn feed(k: &mut K, a: &mut ClaudeAdapter, id: &str, line: Value) {
    for out in a.line(line.to_string().as_bytes(), 1000) {
        k.provider(id, ProviderEvent::Out(out)).unwrap();
    }
}

/// The path the host would resolve for an input: its own file or search path, as given (the
/// tests keep no symlinks; S36-1 tests the resolved path apart).
fn path_of(input: &Value) -> Option<&str> {
    ["file_path", "notebook_path", "path"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
}

fn line(kind: &str, id: &str, tool: &str, input: Value) -> Value {
    let mut v = json!({"type": kind, "id": id, "tool": tool, "input": input,
        "tool_use_id": format!("t-{id}"), "expires-at": 9000});
    if let Some(p) = path_of(&v["input"]) {
        v["path"] = json!(p);
    }
    v
}

fn bash(k: &mut K, a: &mut ClaudeAdapter, s: &str, id: &str, cmd: &str) {
    feed(k, a, s, line("ask", id, "Bash", json!({"command": cmd})));
}

fn resolved(id: &str, by: &str, outcome: Option<&str>) -> Value {
    let mut v = json!({"type": "ask-resolved", "id": id, "by": by});
    if let Some(o) = outcome {
        v["outcome"] = json!(o);
    }
    v
}

fn r(tool: &str, input: Value) -> Option<String> {
    reach("alice", Some("/w"), "ask", tool, &input, path_of(&input))
}

#[test]
fn ac1_bash_keys() {
    assert_eq!(
        r("Bash", json!({"command":"ls"})).as_deref(),
        Some("Bash(ls)")
    );
    assert_eq!(
        r("Bash", json!({"command":"ls","timeout":5})).as_deref(),
        Some("Bash(ls; timeout=5)")
    );
    assert_eq!(
        r(
            "Bash",
            json!({"run_in_background":true,"description":"d","timeout":5,"command":"ls"})
        )
        .as_deref(),
        Some("Bash(ls; timeout=5; run_in_background=true)")
    );
    assert_eq!(
        scope("Bash", None, &json!({"command":"ls"}), None).as_deref(),
        Some("ls")
    );
    for bad in [
        json!({"command":"ls","dangerouslyDisableSandbox":true}),
        json!({"command":"ls","x":1}),
        json!({"command":"ls","timeout":-1}),
        json!({"command":"ls","timeout":1.5}),
        json!({"command":"ls","timeout":"5"}),
        json!({"command":"a:b"}),
        json!({"command":"a;b"}),
        json!({"command":"a)b"}),
    ] {
        assert_eq!(r("Bash", bad.clone()), None, "{bad}");
    }
}

#[test]
fn ac2_file_tools_key_is_the_workspace() {
    let rows: [(&str, Value); 5] = [
        (
            "Edit",
            json!({"file_path":"/w/a/b.rs","old_string":"a","new_string":"b","replace_all":true}),
        ),
        ("Write", json!({"file_path":"/w/a/b.rs","content":"x"})),
        ("MultiEdit", json!({"file_path":"/w/a/b.rs","edits":[]})),
        (
            "NotebookEdit",
            json!({"notebook_path":"/w/a/b.rs","new_source":"x","cell_id":"1","cell_type":"code","edit_mode":"replace"}),
        ),
        (
            "Read",
            json!({"file_path":"/w/a/b.rs","offset":1,"limit":2,"pages":"1"}),
        ),
    ];
    for (tool, input) in rows {
        assert_eq!(
            r(tool, input.clone()),
            Some(format!("{tool}(/w/**)")),
            "{tool}"
        );
        let mut x = input.clone();
        x["_host"] = json!("x");
        assert_eq!(r(tool, x), None, "{tool} _host");
        let pk = if tool == "NotebookEdit" {
            "notebook_path"
        } else {
            "file_path"
        };
        for bad in ["/w/../etc", "/w/./a", "/w//a", "/etc/x", "/wx/a"] {
            let mut x = input.clone();
            x[pk] = json!(bad);
            assert_eq!(r(tool, x), None, "{tool} {bad}");
        }
        assert_eq!(
            reach("alice", None, "ask", tool, &input, path_of(&input)),
            None,
            "{tool} no ws"
        );
    }
}

#[test]
fn ac3_web_fetch_host() {
    let w = |u: &str| r("WebFetch", json!({"url": u, "prompt": "p"}));
    let dom = |h: &str| Some(format!("WebFetch(domain:{h})"));
    assert_eq!(w("https://Example.COM/a?b"), dom("example.com"));
    assert_eq!(w("http://example.com"), dom("example.com"));
    assert_eq!(w("https://example.com:443/"), dom("example.com"));
    assert_eq!(w("https://xn--bcher-kva.de"), dom("xn--bcher-kva.de"));
    assert_eq!(w("https://127.0.0.1/"), dom("127.0.0.1"));
    assert_eq!(w("https://evil.com#@example.com"), dom("evil.com"));
    assert_eq!(
        w("https://example.com.evil.com"),
        dom("example.com.evil.com")
    );
    let long = format!("https://example.com/{}", "a".repeat(2040));
    for bad in [
        "https://example.com:8443/",
        "https://u:p@example.com/",
        "https://u@example.com",
        "ftp://example.com",
        "file:///etc/passwd",
        "javascript:x",
        "example.com",
        "",
        "https://example.com/a b",
        "https://example.com/a\tb",
        "https://example.com/a\\b",
        "https://bücher.de",
        &long,
    ] {
        assert_eq!(w(bad), None, "{bad:?}");
    }
    // validator: the bounds and the rules a looser check would pass.
    let at = |n: usize| format!("https://example.com/{}", "a".repeat(n - 20));
    assert_eq!(w(&at(2048)), dom("example.com"));
    assert_eq!(w(&at(2049)), None);
    for bad in [
        "https://:p@example.com/",
        "https://example.com/\u{7f}",
        "https://a(b).com/",
        "https://a)b.com/",
    ] {
        assert_eq!(w(bad), None, "{bad:?}");
    }
    assert_eq!(
        r("WebFetch", json!({"url":"https://example.com","x":1})),
        None
    );
    assert_eq!(r("WebFetch", json!({"url": 5})), None);
    assert_eq!(r("WebFetch", json!({})), None);
}

#[test]
fn ac4_skill_name() {
    let s = |n: Value| r("Skill", json!({"skill": n}));
    assert_eq!(s(json!("pdf")).as_deref(), Some("Skill(pdf)"));
    assert_eq!(
        r("Skill", json!({"skill":"pdf","args":"x"})).as_deref(),
        Some("Skill(pdf)")
    );
    assert_eq!(
        s(json!("watane-run:pdf")).as_deref(),
        Some("Skill(watane-run:pdf)")
    );
    for bad in [
        "PDF",
        "a:b:c",
        ":a",
        "a:",
        "a b",
        "../x",
        &"a".repeat(65),
        "",
    ] {
        assert_eq!(s(json!(bad)), None, "{bad:?}");
    }
    let max = "a".repeat(64);
    assert_eq!(s(json!(max)), Some(format!("Skill({max})")));
    assert_eq!(
        s(json!(format!("{max}:{max}"))),
        Some(format!("Skill({max}:{max})"))
    );
    assert_eq!(s(json!(5)), None);
    assert_eq!(r("Skill", json!({"skill":"pdf","x":1})), None);
}

#[test]
fn ac5_todo_and_unknown_tools() {
    assert_eq!(
        r("TodoWrite", json!({"todos":[]})).as_deref(),
        Some("TodoWrite")
    );
    assert_eq!(r("TodoWrite", json!({"todos":[],"x":1})), None);
    for t in [
        "mcp__watane__send",
        "Task",
        "WebSearch",
        "AskUserQuestion",
        "ExitPlanMode",
        "Agent",
        "mcp__other__x",
    ] {
        assert_eq!(r(t, json!({})), None, "{t}");
        assert_eq!(
            r(t, json!({"todos":[],"skill":"pdf","command":"ls"})),
            None,
            "{t}"
        );
    }
    assert_eq!(
        reach(
            "alice",
            Some("/w"),
            "check",
            "TodoWrite",
            &json!({"todos":[]}),
            None
        ),
        None
    );
    assert_eq!(
        reach(
            "agent:x",
            Some("/w"),
            "ask",
            "TodoWrite",
            &json!({"todos":[]}),
            None
        ),
        None
    );
}

fn ask_in(k: &mut K, a: &mut ClaudeAdapter, s: &str, id: &str, tool: &str, input: Value) {
    feed(k, a, s, line("ask", id, tool, input));
}

fn by(k: &mut K, s: &str, id: &str) -> Option<String> {
    rec(k, s).pending_answers.get(id).map(|p| p.by.clone())
}

fn open(k: &mut K, s: &str, id: &str) -> bool {
    rec(k, s).open.iter().any(|o| o.id == id)
}

fn allow_entries(k: &mut K, s: &str) -> Vec<Value> {
    k.read("alice", s, 0, None, 512)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .filter(|e| e["kind"] == "allow")
        .map(|e| e["body"].clone())
        .collect()
}

#[test]
fn ac6_web_fetch_allow_holds_the_host_only() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    let f = |u: &str| json!({"url": u, "prompt": "p"});
    ask_in(
        &mut k,
        &mut a,
        &s,
        "w1",
        "WebFetch",
        f("https://example.com/a"),
    );
    k.answer("alice", &s, "k1", "w1", SESSION).unwrap();
    let held = rec(&mut k, &s);
    assert_eq!(
        held.allows,
        [Allow {
            id: held.allows[0].id.clone(),
            tool: "WebFetch".into(),
            key: "domain:example.com".into()
        }]
    );
    let id = held.allows[0].id.clone();
    assert_eq!(
        allow_entries(&mut k, &s),
        [
            json!({"id": id, "tool": "WebFetch", "key": "domain:example.com",
            "reach": "WebFetch(domain:example.com)", "given": true, "by": "owner"})
        ]
    );
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("w1", "answered", Some("allow")),
    );
    ask_in(
        &mut k,
        &mut a,
        &s,
        "w2",
        "WebFetch",
        f("https://example.com/other?q=1"),
    );
    assert_eq!(by(&mut k, &s, "w2").as_deref(), Some("session-allow"));
    ask_in(
        &mut k,
        &mut a,
        &s,
        "w3",
        "WebFetch",
        f("https://api.example.com/"),
    );
    ask_in(
        &mut k,
        &mut a,
        &s,
        "w4",
        "WebFetch",
        f("https://example.com.evil.com/"),
    );
    assert!(open(&mut k, &s, "w3") && open(&mut k, &s, "w4"));
    k.take_back("alice", &s, &id).unwrap();
    let es = allow_entries(&mut k, &s);
    assert_eq!(
        es[1],
        json!({"id": id, "tool": "WebFetch", "key": "domain:example.com",
            "reach": "WebFetch(domain:example.com)", "given": false, "by": "owner"})
    );
    ask_in(
        &mut k,
        &mut a,
        &s,
        "w5",
        "WebFetch",
        f("https://example.com/a"),
    );
    assert!(open(&mut k, &s, "w5"));
}

#[test]
fn ac6_skill_allow_holds_the_name_only() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    ask_in(&mut k, &mut a, &s, "k1", "Skill", json!({"skill": "pdf"}));
    k.answer("alice", &s, "k1", "k1", SESSION).unwrap();
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("k1", "answered", Some("allow")),
    );
    ask_in(
        &mut k,
        &mut a,
        &s,
        "k2",
        "Skill",
        json!({"skill": "pdf", "args": "z"}),
    );
    assert_eq!(by(&mut k, &s, "k2").as_deref(), Some("session-allow"));
    ask_in(&mut k, &mut a, &s, "k3", "Skill", json!({"skill": "pdf2"}));
    assert!(open(&mut k, &s, "k3"));
}

#[test]
fn ac7_forged_reach_widens_nothing() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    let mut l = line(
        "ask",
        "f1",
        "WebFetch",
        json!({"url": "https://example.com/", "prompt": "p"}),
    );
    l["reach"] = json!("WebFetch(domain:example.com)");
    feed(&mut k, &mut a, &s, l);
    k.answer("alice", &s, "k1", "f1", SESSION).unwrap();
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("f1", "answered", Some("allow")),
    );
    let mut l = line(
        "ask",
        "f2",
        "WebFetch",
        json!({"url": "https://evil.com/", "prompt": "p"}),
    );
    l["reach"] = json!("WebFetch(domain:example.com)");
    feed(&mut k, &mut a, &s, l);
    assert!(open(&mut k, &s, "f2"));
    let es = k.read("alice", &s, 0, None, 512).unwrap().entries;
    let stored = es
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .find(|e| e["kind"] == "ask" && e["body"]["id"] == "f2")
        .unwrap();
    assert_eq!(stored["body"]["reach"], "WebFetch(domain:evil.com)");
}

#[test]
fn ac7_bash_allow_does_not_cover_background() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    bash(&mut k, &mut a, &s, "a1", "npm test");
    k.answer("alice", &s, "k1", "a1", SESSION).unwrap();
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("a1", "answered", Some("allow")),
    );
    ask_in(
        &mut k,
        &mut a,
        &s,
        "a2",
        "Bash",
        json!({"command":"npm test","run_in_background":true}),
    );
    assert!(open(&mut k, &s, "a2"));
}

#[test]
fn ac8_no_key_no_session_allow() {
    for tool in ["Task", "WebSearch", "mcp__watane__send"] {
        let mut k = kernel();
        let s = started(&mut k);
        let mut a = ClaudeAdapter::new();
        ask_in(&mut k, &mut a, &s, "q1", tool, json!({}));
        let e = k.answer("alice", &s, "k1", "q1", SESSION);
        assert!(
            matches!(&e, Err(KernelError::Refused(v)) if v[0].field == "decision"),
            "{tool}: {e:?}"
        );
        assert!(rec(&mut k, &s).allows.is_empty());
        k.answer("alice", &s, "k2", "q1", r#"{"allow":true}"#)
            .unwrap();
    }
}

#[test]
fn fix1_ac1_a_write_that_can_run_later_gets_no_key() {
    for p in [
        "/w/.git/config",
        "/w/a/.claude/settings.json",
        "/w/.mcp.json",
        "/w/.vscode/tasks.json",
        "/w/.husky/pre-commit",
        "/w/.env",
        "/w/src/CLAUDE.md",
        "/w/claude.md",
        "/w/AGENTS.md",
        "/w/build.rs",
        "/w/pkg/package.json",
        "/w/Makefile",
        "/w/MAKEFILE",
    ] {
        for t in ["Write", "Edit", "MultiEdit"] {
            assert_eq!(r(t, json!({"file_path": p})), None, "{t} {p}");
        }
    }
    assert_eq!(
        r(
            "NotebookEdit",
            json!({"notebook_path": "/w/.ipynb_checkpoints/x.ipynb"})
        ),
        None
    );
    for p in ["/w/src/main.rs", "/w/docs/make.md"] {
        assert_eq!(
            r("Write", json!({"file_path": p})).as_deref(),
            Some("Write(/w/**)")
        );
    }
    for (t, k, p) in [
        ("Read", "file_path", "/w/.git/config"),
        ("Glob", "path", "/w/.git"),
        ("Grep", "path", "/w/.git"),
    ] {
        let mut i = json!({"pattern": "x"});
        i[k] = json!(p);
        if t == "Read" {
            i = json!({"file_path": p});
        }
        assert_eq!(r(t, i), Some(format!("{t}(/w/**)")), "{t}");
    }
}

#[test]
fn fix1_ac2_a_session_allow_does_not_reach_a_dot_path() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut w = rec(&mut k, &s);
    w.folder = Some("/w".into());
    w.save(&mut k.state).unwrap();
    let mut a = ClaudeAdapter::new();
    ask_in(
        &mut k,
        &mut a,
        &s,
        "e1",
        "Edit",
        json!({"file_path": "/w/src/a.rs"}),
    );
    k.answer("alice", &s, "k1", "e1", SESSION).unwrap();
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("e1", "answered", Some("allow")),
    );
    ask_in(
        &mut k,
        &mut a,
        &s,
        "e2",
        "Edit",
        json!({"file_path": "/w/.git/config"}),
    );
    assert!(open(&mut k, &s, "e2"));
    assert_ne!(by(&mut k, &s, "e2").as_deref(), Some("session-allow"));
    // The stored ask entries: the first has the reach the allow took, the second none.
    let es = k.read("alice", &s, 0, None, 512).unwrap().entries;
    let stored = |id: &str| {
        es.iter()
            .map(|e| serde_json::from_str::<Value>(e).unwrap())
            .find(|e| e["kind"] == "ask" && e["body"]["id"] == id)
            .unwrap()
    };
    assert_eq!(stored("e1")["body"]["reach"], "Edit(/w/**)");
    assert!(stored("e2")["body"].get("reach").is_none());
}

#[test]
fn fix2_ac2_unicode_and_new_names_get_no_write_key() {
    for p in [
        "/w/CLAUDE.local.md",
        "/w/claude.LOCAL.md",
        "/w/GNUmakefile",
        "/w/src/caf\u{e9}.rs",
        "/w/\u{200C}.git/config",
    ] {
        assert_eq!(
            r("Write", json!({"file_path": p, "content": "x"})),
            None,
            "{p}"
        );
    }
    assert_eq!(
        r("Read", json!({"file_path": "/w/src/caf\u{e9}.rs"})).as_deref(),
        Some("Read(/w/**)")
    );
    assert_eq!(
        r(
            "Write",
            json!({"file_path": "/w/src/main.rs", "content": "x"})
        )
        .as_deref(),
        Some("Write(/w/**)")
    );
    // The ends of printable ASCII (a space, a tilde) stay writable.
    assert_eq!(
        r(
            "Write",
            json!({"file_path": "/w/my notes~.txt", "content": "x"})
        )
        .as_deref(),
        Some("Write(/w/**)")
    );
}
