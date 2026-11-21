//! Adversary of slice S20 (TECHNICAL 14, "An ask shows the tool call that will run", "A repeated
//! call changes nothing"; 9.8 rule 4): attacks that the boundary did not stop.

use agent_kernel::active::Slots;
use agent_kernel::adapters::claude::ClaudeAdapter;
use agent_kernel::*;
use serde_json::{json, Value};

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
    let host = FakeHost {
        boot: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn feed(k: &mut K, a: &mut ClaudeAdapter, id: &str, line: Value) {
    for out in a.line(line.to_string().as_bytes(), 1000) {
        k.provider(id, ProviderEvent::Out(out)).unwrap();
    }
}

fn ask_line(id: &str, path: &str) -> Value {
    json!({"type": "ask", "id": id, "tool": "Edit",
        "input": {"file_path": path, "old_string": "a", "new_string": "b"},
        "tool_use_id": "toolu_1", "expires-at": 9000})
}

fn decisions_of(k: &mut K, id: &str, ask: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .filter(|e| {
            e["kind"] == "ask" && e["body"]["id"] == ask && e["body"]["decision"].is_object()
        })
        .map(|e| e["body"]["decision"].clone())
        .collect()
}

/// The adapter's `open` map lives in memory only; the record's pending answers survive. An ask
/// item that reuses the id of an ask with a pending answer (a second provider stream, or the same
/// stream after the kernel instance is reloaded) is opened again: the owner's second `answer`
/// passes and a second answer reaches the host for the one ask id.
#[test]
fn adv_s20_reused_id_of_a_pending_ask_takes_a_second_answer() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("a1", "/w/x"));
    k.answer("alice", &id, "k1", "a1", r#"{"deny":true}"#)
        .unwrap();
    // a fresh adapter (memory lost) sees the same ask id again, now for another file
    let mut fresh = ClaudeAdapter::new();
    feed(&mut k, &mut fresh, &id, ask_line("a1", "/w/other"));
    // 9.8 rule 4: while an ask has a pending answer, a second `answer` is refused `{field: "ask"}`
    let second = k.answer("alice", &id, "k2", "a1", r#"{"allow":true}"#);
    assert!(
        second.is_err(),
        "a second answer to ask a1 (pending deny) was taken: {:?}",
        Record::load(&mut k.state, &id).unwrap().pending_answers
    );
}

/// The same reuse ends with two decision entries for one ask id (9.8 rule 4: exactly one), and the
/// transcript shows a decision for a call other than the one the pending answer was given for.
#[test]
fn adv_s20_reused_id_of_a_pending_ask_gets_two_decisions() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("a1", "/w/x"));
    k.answer("alice", &id, "k1", "a1", r#"{"deny":true}"#)
        .unwrap();
    let mut fresh = ClaudeAdapter::new();
    feed(&mut k, &mut fresh, &id, ask_line("a1", "/w/other"));
    // the stream ends: the kernel's own closing step
    k.provider(&id, ProviderEvent::Ended).unwrap();
    let ds = decisions_of(&mut k, &id, "a1");
    assert_eq!(ds.len(), 1, "decisions for a1: {ds:?}");
}

/// An ask item that reuses the id of an ask still open in the record, from an adapter that does not
/// hold it (memory lost), is appended and pushed as a second card with another input; the record's
/// open ask still points at the first entry, so the owner's `allow` on what the newest card shows
/// (`/w/other`) lets the host run the first call (`/w/x`). 9.8: such an item is `unknown`, the open
/// one unchanged.
#[test]
fn adv_s20_reused_id_of_an_open_ask_shows_another_call() {
    let mut k = kernel();
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    let mut a = ClaudeAdapter::new();
    feed(&mut k, &mut a, &id, ask_line("a1", "/w/x"));
    let mut fresh = ClaudeAdapter::new();
    feed(&mut k, &mut fresh, &id, ask_line("a1", "/w/other"));
    let cards: Vec<Value> = k
        .read("alice", &id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str::<Value>(e).unwrap())
        .filter(|e| e["kind"] == "ask" && e["body"]["id"] == "a1")
        .map(|e| e["body"]["input"]["file_path"].clone())
        .collect();
    assert_eq!(cards, vec![json!("/w/x")], "ask entries for a1");
}
