//! S36-1: a file tool's session allow takes its key from the host's resolved `path` under the
//! record's `folder` (TECHNICAL 4.5 step 4, 9.3, 9.4, 9.6, 9.8, 14).

use agent_kernel::active::Slots;
use agent_kernel::adapters::claude::{facts, ClaudeAdapter, Fact};
use agent_kernel::broker::{reach, scope};
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const SESSION: &str = r#"{"allow":true,"for":"session"}"#;
const UUID: &str = "5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11";

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

/// A session with its owner stream; the session id and its provider stream.
fn started(k: &mut K) -> (String, String) {
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    (id, t)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn lines(raw: &[String], from: u64) -> Vec<(u64, Vec<u8>)> {
    raw.iter()
        .enumerate()
        .map(|(i, l)| (from + i as u64, l.as_bytes().to_vec()))
        .collect()
}

fn init(cwd: &str) -> String {
    json!({"type":"system","subtype":"init","session_id":UUID,"cwd":cwd}).to_string()
}

fn ask_line(id: &str, tool: &str, input: Value, path: Option<&str>) -> String {
    let mut v = json!({"type":"ask","id":id,"tool":tool,"input":input,
        "tool_use_id":format!("t-{id}"),"expires-at":9000});
    if let Some(p) = path {
        v["path"] = json!(p);
    }
    v.to_string()
}

/// Feeds `raw` lines after the `n` already fed.
fn feed(k: &mut K, t: &str, n: &mut u64, raw: &[String]) {
    k.provider_lines(t, lines(raw, *n + 1)).unwrap();
    *n += raw.len() as u64;
}

fn stored_ask(k: &mut K, s: &str, id: &str) -> Value {
    k.read("alice", s, 0, None, 512)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .find(|e| e["kind"] == "ask" && e["body"]["id"] == id)
        .unwrap()["body"]
        .clone()
}

fn r(folder: Option<&str>, tool: &str, input: Value, path: Option<&str>) -> Option<String> {
    reach("alice", folder, "ask", tool, &input, path)
}

#[test]
fn ac1_the_ask_entry_keeps_the_hosts_path() {
    let mut a = ClaudeAdapter::new();
    let mut body = |l: String| {
        a.line(l.as_bytes(), 1000)[0]
            .entry
            .as_ref()
            .unwrap()
            .body
            .clone()
    };
    let input = json!({"file_path": "/w/a.txt"});
    for kind in ["ask", "check"] {
        let mut v: Value =
            serde_json::from_str(&ask_line(kind, "Read", input.clone(), Some("/w/a.txt"))).unwrap();
        v["type"] = json!(kind);
        let b = body(v.to_string());
        assert_eq!(b["kind"], kind);
        assert_eq!(b["path"], "/w/a.txt");
        assert_eq!(b["input"], input);
    }
    let b = body(ask_line("b", "Read", input.clone(), None));
    assert!(b.get("path").is_none());
    let mut v: Value = serde_json::from_str(&ask_line("c", "Read", input, None)).unwrap();
    v["path"] = json!(5);
    let out = &a.line(v.to_string().as_bytes(), 1000)[0];
    let b = &out.entry.as_ref().unwrap().body;
    assert_eq!(b["kind"], "ask");
    assert!(b.get("path").is_none());
}

#[test]
fn ac2_system_init_sets_the_folder_once() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    let before = k.read("alice", &s, 0, None, 512).unwrap().entries.len();
    feed(&mut k, &t, &mut n, &[init("/w")]);
    assert_eq!(rec(&mut k, &s).folder.as_deref(), Some("/w"));
    assert_eq!(rec(&mut k, &s).claude_uuid.as_deref(), Some(UUID));
    feed(&mut k, &t, &mut n, &[init("/x")]);
    assert_eq!(rec(&mut k, &s).folder.as_deref(), Some("/w"));
    assert_eq!(
        k.read("alice", &s, 0, None, 512).unwrap().entries.len(),
        before
    );
    // not a plain absolute folder: nothing
    for bad in ["w", "/w/", "/w(", "/w)", "/w\n", ""] {
        let mut k = kernel();
        let (s, t) = started(&mut k);
        feed(&mut k, &t, &mut 0, &[init(bad)]);
        assert_eq!(rec(&mut k, &s).folder, None, "{bad:?}");
    }
    assert_eq!(
        facts(init("/w").as_bytes()),
        [Fact::Session(UUID.into()), Fact::Folder(Some("/w".into()))]
    );
}

#[test]
fn ac3_the_key_comes_from_path_under_folder() {
    let w = Some("/w");
    let input = json!({"file_path": "rel/../../etc/x"});
    assert_eq!(
        r(w, "Read", input.clone(), Some("/w/a/b.txt")).as_deref(),
        Some("Read(/w/**)")
    );
    assert_eq!(r(w, "Read", input.clone(), Some("/etc/passwd")), None);
    assert_eq!(r(w, "Read", input, None), None);
    let inside = json!({"file_path": "/w/a.txt"});
    assert_eq!(r(w, "Read", inside.clone(), Some("/elsewhere/a.txt")), None);
    assert_eq!(r(w, "Read", inside.clone(), Some("/wx/a.txt")), None);
    assert_eq!(r(None, "Read", inside, Some("/w/a.txt")), None);
}

#[test]
fn ac4_segment_rules_apply_to_path() {
    let w = Some("/w");
    let write = json!({"file_path": "x", "content": "c"});
    for bad in [
        "/w/.git/config",
        "/w/sub/Makefile",
        "/w/CLAUDE.md",
        "/w/caf\u{e9}.rs",
        "/w//a",
        "/w/a//b",
        "/w/",
        "/w/a/",
    ] {
        assert_eq!(r(w, "Write", write.clone(), Some(bad)), None, "{bad:?}");
    }
    assert_eq!(
        r(w, "Write", write.clone(), Some("/w/src/a.rs")).as_deref(),
        Some("Write(/w/**)")
    );
    assert_eq!(
        r(w, "Read", json!({"file_path": "x"}), Some("/w/.git/config")).as_deref(),
        Some("Read(/w/**)")
    );
    for tool in ["Glob", "Grep"] {
        assert_eq!(
            r(w, tool, json!({"pattern": "*"}), Some("/w")),
            Some(format!("{tool}(/w/**)")),
            "{tool}"
        );
    }
    for (tool, input) in [
        ("Read", json!({"file_path": "x"})),
        ("Edit", json!({"file_path": "x"})),
        ("Write", json!({"file_path": "x"})),
        ("NotebookEdit", json!({"notebook_path": "x"})),
    ] {
        assert_eq!(r(w, tool, input, Some("/w")), None, "{tool}");
    }
    // The Glob pattern rule and the known-keys rule stay.
    assert_eq!(r(w, "Glob", json!({"pattern": "/etc/*"}), Some("/w")), None);
    assert_eq!(r(w, "Glob", json!({}), Some("/w")), None);
    assert_eq!(
        r(w, "Grep", json!({"pattern": "x", "extra": 1}), Some("/w")),
        None
    );
    assert_eq!(
        scope(
            "Read",
            w,
            &json!({"file_path": "x", "_host": 1}),
            Some("/w/a")
        ),
        None
    );
    // Bash and the others ignore both.
    assert_eq!(
        scope("Bash", None, &json!({"command": "ls"}), Some("/etc")).as_deref(),
        Some("ls")
    );
}

#[test]
fn ac5_a_session_allow_end_to_end() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    let read = |id: &str, p: Option<&str>| {
        ask_line(id, "Read", json!({"file_path": "rel/../../etc/x"}), p)
    };
    feed(
        &mut k,
        &t,
        &mut n,
        &[init("/w"), read("a1", Some("/w/a/b.txt"))],
    );
    assert_eq!(stored_ask(&mut k, &s, "a1")["reach"], "Read(/w/**)");
    assert_eq!(stored_ask(&mut k, &s, "a1")["path"], "/w/a/b.txt");
    k.answer("alice", &s, "k1", "a1", SESSION).unwrap();
    let held = rec(&mut k, &s).allows;
    assert_eq!(
        (held[0].tool.as_str(), held[0].key.as_str()),
        ("Read", "/w/**")
    );
    let id = held[0].id.clone();
    let done = json!({"type":"ask-resolved","id":"a1","by":"answered","outcome":"allow"});
    feed(&mut k, &t, &mut n, &[done.to_string()]);
    let answers = |k: &K| {
        k.host
            .calls
            .iter()
            .filter(|c| matches!(c, HostCall::Request(Request::Answer { .. })))
            .count()
    };
    let sent = answers(&k);
    feed(&mut k, &t, &mut n, &[read("a2", Some("/w/c.txt"))]);
    assert_eq!(rec(&mut k, &s).pending_answers["a2"].by, "session-allow");
    assert_eq!(answers(&k), sent + 1);
    let last = k.host.calls.iter().rev().find_map(|c| match c {
        HostCall::Request(Request::Answer { ask, decision, .. }) => {
            Some((ask.clone(), decision.clone()))
        }
        _ => None,
    });
    let (ask, decision) = last.unwrap();
    assert_eq!(ask, "a2");
    assert_eq!(decision["allow"], true);
    feed(
        &mut k,
        &t,
        &mut n,
        &[
            json!({"type":"ask-resolved","id":"a2","by":"answered","outcome":"allow"}).to_string(),
            read("a3", Some("/etc/passwd")),
        ],
    );
    assert!(rec(&mut k, &s).open.iter().any(|o| o.id == "a3"));
    assert!(!rec(&mut k, &s).pending_answers.contains_key("a3"));
    k.take_back("alice", &s, &id).unwrap();
    feed(&mut k, &t, &mut n, &[read("a4", Some("/w/d.txt"))]);
    assert!(rec(&mut k, &s).open.iter().any(|o| o.id == "a4"));
    assert!(!rec(&mut k, &s).pending_answers.contains_key("a4"));
}
