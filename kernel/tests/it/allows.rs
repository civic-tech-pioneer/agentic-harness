//! S31-3: the kernel answers Claude asks itself (TECHNICAL 9.6): a held session allow, the "no
//! owner stream" deny, else the owner.

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

/// The path the host would resolve for an input: its own file or search path, as given (no
/// symlinks here; S36-1's tests feed the resolved path apart).
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

fn requests(k: &K) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Answer { ask, decision, .. }) => {
                Some(json!({"ask": ask, "decision": decision}))
            }
            _ => None,
        })
        .collect()
}

fn decisions(k: &mut K, id: &str) -> Vec<(String, Value)> {
    k.read("alice", id, 0, None, 512)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .map(|e| {
            (
                e["body"]["id"].as_str().unwrap().to_string(),
                e["body"]["decision"].clone(),
            )
        })
        .collect()
}

fn waiting(k: &mut K, id: &str) -> bool {
    matches!(rec(k, id).turn, Turn::Waiting { .. })
}

fn open_ids(k: &mut K, id: &str) -> Vec<String> {
    rec(k, id).open.iter().map(|o| o.id.clone()).collect()
}

fn resolved(id: &str, by: &str, outcome: Option<&str>) -> Value {
    let mut v = json!({"type": "ask-resolved", "id": id, "by": by});
    if let Some(o) = outcome {
        v["outcome"] = json!(o);
    }
    v
}

/// The id of the allow after `npm test` was allowed for the session by `a1`.
fn allowed(k: &mut K, a: &mut ClaudeAdapter, s: &str) -> String {
    bash(k, a, s, "a1", "npm test");
    k.answer("alice", s, "k1", "a1", SESSION).unwrap();
    rec(k, s).allows[0].id.clone()
}

#[test]
fn ac1_a_matching_ask_is_answered_allow_by_session_allow() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    allowed(&mut k, &mut a, &s);
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("a1", "answered", Some("allow")),
    );
    assert!(matches!(rec(&mut k, &s).turn, Turn::Running { .. }));
    let n = requests(&k).len();
    bash(&mut k, &mut a, &s, "a2", "npm test");
    let r = rec(&mut k, &s);
    assert_eq!(requests(&k).len(), n + 1);
    assert_eq!(
        requests(&k)[n],
        json!({"ask": "a2", "decision": {"allow": true}})
    );
    assert_eq!(r.pending_answers["a2"].by, "session-allow");
    assert_eq!(r.pending_answers["a2"].outcome, "allow");
    assert!(r.open.is_empty());
    assert!(matches!(r.turn, Turn::Running { .. }));
    let before = decisions(&mut k, &s).len();
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("a2", "answered", Some("allow")),
    );
    let ds = decisions(&mut k, &s);
    assert_eq!(ds.len(), before + 1);
    assert_eq!(
        ds.last().unwrap().1,
        json!({"outcome": "allow", "by": "session-allow"})
    );
    assert!(rec(&mut k, &s).pending_answers.is_empty());
    // a second answer to it is refused, and expiry gives the host's deny
    bash(&mut k, &mut a, &s, "a3", "npm test");
    assert!(k
        .answer("alice", &s, "k9", "a3", r#"{"deny":true}"#)
        .is_err());
    feed(&mut k, &mut a, &s, resolved("a3", "expired", None));
    assert_eq!(
        decisions(&mut k, &s).last().unwrap().1,
        json!({"outcome": "deny", "by": "host", "reason": "expired"})
    );
    for r in requests(&k) {
        let text = r.to_string();
        assert!(!text.contains("\"for\"") && !text.contains("updatedPermissions"));
    }
}

#[test]
fn ac2_a_different_ask_goes_to_the_owner() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    allowed(&mut k, &mut a, &s);
    for (i, cmd) in ["npm test --watch", "ls", "npm test "].iter().enumerate() {
        let id = format!("b{i}");
        bash(&mut k, &mut a, &s, &id, cmd);
        assert!(open_ids(&mut k, &s).contains(&id), "{cmd}");
        assert!(waiting(&mut k, &s));
        assert!(!rec(&mut k, &s).pending_answers.contains_key(&id));
    }
    // a file allow covers its tool only
    let mut k = kernel();
    let s = started(&mut k);
    let mut r = rec(&mut k, &s);
    r.folder = Some("/w".into());
    r.save(&mut k.state).unwrap();
    let mut a = ClaudeAdapter::new();
    feed(
        &mut k,
        &mut a,
        &s,
        line("ask", "e1", "Edit", json!({"file_path": "/w/x"})),
    );
    k.answer("alice", &s, "k1", "e1", SESSION).unwrap();
    let held = &rec(&mut k, &s).allows[0];
    assert_eq!((held.tool.as_str(), held.key.as_str()), ("Edit", "/w/**"));
    feed(
        &mut k,
        &mut a,
        &s,
        line("ask", "w1", "Write", json!({"file_path": "/w/y"})),
    );
    assert_eq!(open_ids(&mut k, &s), ["w1"]);
    assert!(waiting(&mut k, &s));
    // validator: the same key on another tool is a new pair, held beside the first.
    k.answer("alice", &s, "k2", "w1", SESSION).unwrap();
    let held: Vec<_> = rec(&mut k, &s)
        .allows
        .iter()
        .map(|a| (a.tool.clone(), a.key.clone()))
        .collect();
    assert_eq!(
        held,
        [("Edit", "/w/**"), ("Write", "/w/**")].map(|(t, k)| (t.to_string(), k.to_string()))
    );
}

#[test]
fn ac3_after_take_back_the_same_call_asks_again() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    let id = allowed(&mut k, &mut a, &s);
    k.take_back("alice", &s, &id).unwrap();
    let n = requests(&k).len();
    bash(&mut k, &mut a, &s, "a2", "npm test");
    assert_eq!(requests(&k).len(), n);
    assert_eq!(open_ids(&mut k, &s), ["a2"]);
    assert!(waiting(&mut k, &s));
}

#[test]
fn ac4_a_revoked_profile_does_not_answer_from_the_allow() {
    for bad in [&b"[]"[..], &b"{}"[..]] {
        let mut k = kernel();
        let s = started(&mut k);
        let mut a = ClaudeAdapter::new();
        allowed(&mut k, &mut a, &s);
        k.state.put("access/alice", bad.to_vec()).unwrap();
        bash(&mut k, &mut a, &s, "a2", "npm test");
        assert_eq!(
            open_ids(&mut k, &s),
            ["a1", "a2"].map(String::from)[1..],
            "{bad:?}"
        );
        assert!(waiting(&mut k, &s));
        assert!(!requests(&k).iter().any(|r| r["ask"] == "a2"));
        // restoring access answers the next matching ask from the allow again
        k.state.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
        bash(&mut k, &mut a, &s, "a3", "npm test");
        assert_eq!(rec(&mut k, &s).pending_answers["a3"].by, "session-allow");
        // without an owner stream a revoked allow is the no-owner deny
        k.state.put("access/alice", bad.to_vec()).unwrap();
        let mut rc = rec(&mut k, &s);
        rc.live = None;
        rc.save(&mut k.state).unwrap();
        bash(&mut k, &mut a, &s, "a4", "npm test");
        assert_eq!(
            rec(&mut k, &s).pending_answers["a4"].reason.as_deref(),
            Some("no-owner")
        );
    }
}

#[test]
fn ac5_no_owner_stream_is_a_no_owner_deny() {
    let mut k = kernel();
    let s = bare(&mut k);
    let mut a = ClaudeAdapter::new();
    bash(&mut k, &mut a, &s, "a1", "ls");
    let r = rec(&mut k, &s);
    let p = &r.pending_answers["a1"];
    assert_eq!((p.outcome.as_str(), p.by.as_str()), ("deny", "kernel"));
    assert_eq!(p.reason.as_deref(), Some("no-owner"));
    assert!(r.open.is_empty());
    assert!(!matches!(r.turn, Turn::Waiting { .. }));
    assert_eq!(
        requests(&k),
        [json!({"ask": "a1", "decision": {"deny": true, "reason": "no-owner"}})]
    );
    feed(&mut k, &mut a, &s, resolved("a1", "answered", Some("deny")));
    assert_eq!(
        decisions(&mut k, &s)[0].1,
        json!({"outcome": "deny", "by": "kernel", "reason": "no-owner"})
    );
    // after the stream's end clears `live`
    let mut k = kernel();
    let s = started(&mut k);
    let live = rec(&mut k, &s).live.unwrap();
    k.stream_ended(&live);
    assert!(rec(&mut k, &s).live.is_none());
    let mut a = ClaudeAdapter::new();
    bash(&mut k, &mut a, &s, "a1", "ls");
    assert_eq!(
        rec(&mut k, &s).pending_answers["a1"].reason.as_deref(),
        Some("no-owner")
    );
    assert!(!waiting(&mut k, &s));
}

#[test]
fn ac5_an_agent_owner_with_a_stream_is_denied_too() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut r = rec(&mut k, &s);
    r.owner = "agent:x".into();
    r.save(&mut k.state).unwrap();
    let mut a = ClaudeAdapter::new();
    bash(&mut k, &mut a, &s, "a1", "ls");
    let r = rec(&mut k, &s);
    assert_eq!(r.pending_answers["a1"].reason.as_deref(), Some("no-owner"));
    assert!(r.open.is_empty() && !matches!(r.turn, Turn::Waiting { .. }));
}

#[test]
fn ac6_a_check_with_no_owner_stream_stays_open() {
    let mut k = kernel();
    let s = bare(&mut k);
    let mut a = ClaudeAdapter::checked();
    feed(
        &mut k,
        &mut a,
        &s,
        line("check", "c1", "Bash", json!({"command": "ls"})),
    );
    let r = rec(&mut k, &s);
    assert_eq!(r.open.len(), 1);
    assert_eq!(r.open[0].kind, "check");
    assert!(r.pending_answers.is_empty());
    assert!(requests(&k).is_empty());
}

#[test]
fn ac7_the_cap_counts_pending_answers() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    allowed(&mut k, &mut a, &s);
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("a1", "answered", Some("allow")),
    );
    for i in 0..63 {
        bash(&mut k, &mut a, &s, &format!("o{i}"), &format!("ls {i}"));
    }
    assert_eq!(rec(&mut k, &s).open.len(), 63);
    // with 63 held, a matching ask is answered from the allow: it is the 64th
    bash(&mut k, &mut a, &s, "m1", "npm test");
    let r = rec(&mut k, &s);
    assert_eq!((r.open.len(), r.pending_answers.len()), (63, 1));
    assert_eq!(r.pending_answers["m1"].by, "session-allow");
    bash(&mut k, &mut a, &s, "m2", "npm test");
    let r = rec(&mut k, &s);
    assert_eq!((r.open.len(), r.pending_answers.len()), (63, 1));
    assert_eq!(
        requests(&k).last().unwrap(),
        &json!({"ask": "m2", "decision": {"deny": true, "reason": "busy"}})
    );
}

#[test]
fn ac7_all_pending_and_no_open_is_busy_too() {
    // "Pending full but room to open" cannot happen: the cap counts open and pending together,
    // so at 64 pending answers nothing is left to open and the `busy` deny comes first.
    let mut k = kernel();
    let s = bare(&mut k);
    let mut a = ClaudeAdapter::new();
    for i in 0..64 {
        bash(&mut k, &mut a, &s, &format!("p{i}"), "ls");
    }
    let r = rec(&mut k, &s);
    assert_eq!((r.open.len(), r.pending_answers.len()), (0, 64));
    bash(&mut k, &mut a, &s, "x", "ls");
    assert_eq!(
        requests(&k).last().unwrap(),
        &json!({"ask": "x", "decision": {"deny": true, "reason": "busy"}})
    );
    assert_eq!(rec(&mut k, &s).pending_answers.len(), 64);
}

#[test]
fn ac8_past_quota_a_matching_ask_is_denied_quota() {
    let mut k = kernel();
    let s = started(&mut k);
    let mut a = ClaudeAdapter::new();
    allowed(&mut k, &mut a, &s);
    feed(
        &mut k,
        &mut a,
        &s,
        resolved("a1", "answered", Some("allow")),
    );
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    bash(&mut k, &mut a, &s, "a2", "npm test");
    assert_eq!(
        requests(&k).last().unwrap(),
        &json!({"ask": "a2", "decision": {"deny": true, "reason": "quota"}})
    );
    let r = rec(&mut k, &s);
    assert!(r.pending_answers.is_empty() && r.open.is_empty());
}
