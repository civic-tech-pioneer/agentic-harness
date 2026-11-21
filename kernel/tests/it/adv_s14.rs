//! Adversary probes of S14 (4.5, 9.8 ask guards; 9.4 the `i` of a place).

use agent_kernel::adapters::{claude::ClaudeAdapter, Kind, Out};
use agent_kernel::state::{MemState, State};
use agent_kernel::transcript::{chunk_key, Joiner, Meta, NewEntry, Transcript};
use serde_json::{json, Value};

fn kinds(outs: &[Out]) -> Vec<Kind> {
    outs.iter()
        .filter_map(|o| o.entry.as_ref().map(|e| e.kind))
        .collect()
}

/// A host id is a string (`answer`'s `ask: string`, the ask body's `"id":"<ask id>"`).
/// The same id written as a number opens a second card for the one host ask.
#[test]
fn adv_s14_reused_id_as_number_opens_a_second_card() {
    let mut a = ClaudeAdapter::new();
    let first = json!({"type":"ask","id":"7","tool_use_id":"t1","tool":"Bash",
        "input":{"command":"ls"},"expires-at":5});
    let second = json!({"type":"ask","id":7,"tool_use_id":"t2","tool":"Bash",
        "input":{"command":"rm -rf ~"},"expires-at":5});
    assert_eq!(
        kinds(&a.line(first.to_string().as_bytes(), 1)),
        vec![Kind::Ask]
    );
    let out = a.line(second.to_string().as_bytes(), 2);
    assert_eq!(
        kinds(&out),
        vec![Kind::Unknown],
        "id 7 while \"7\" is open must be unknown, got {:?}",
        out.iter()
            .map(|o| o.entry.as_ref().map(|e| e.body.clone()))
            .collect::<Vec<_>>()
    );
}

/// The same number written two ways (`1` and `1.0`) is the same id twice.
#[test]
fn adv_s14_number_ids_never_open_an_ask() {
    let mut a = ClaudeAdapter::new();
    let first = br#"{"type":"ask","id":1,"tool_use_id":"t1","tool":"Bash","input":{"command":"ls"},"expires-at":5}"#;
    let second = br#"{"type":"ask","id":1.0,"tool_use_id":"t2","tool":"Bash","input":{"command":"rm -rf ~"},"expires-at":5}"#;
    // A number id is refused, so no form of it can open twice.
    assert_eq!(kinds(&a.line(first, 1)), vec![Kind::Unknown]);
    assert_eq!(kinds(&a.line(second, 2)), vec![Kind::Unknown]);
}

fn msg(n: u64) -> NewEntry {
    NewEntry {
        kind: "message".into(),
        uuid: None,
        body: json!({ "n": n }),
    }
}

/// A stored part group whose joined text claims another entry's `i`: each part reads with
/// the `i` of its place, but the whole a reader joins takes index 1 (9.4: "the whole entry
/// carries the index of its last part"; "a corrupt line cannot take another entry's index").
#[test]
fn adv_s14_part_group_whole_takes_another_index() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    {
        let mut t = Transcript::new(&mut st, "s", &mut meta);
        for n in 0..5 {
            t.append(&msg(n), 1).unwrap();
        }
    }
    let raw = st.get(&chunk_key("s", 0)).unwrap();
    let mut lines: Vec<String> = String::from_utf8(raw)
        .unwrap()
        .split('\n')
        .map(String::from)
        .collect();
    // Places 4 and 5 become one group of two parts, both with the right `i`.
    let whole = r#"{"v":1,"i":1,"kind":"message","at":1,"body":{"forged":true}}"#;
    let (a, b) = whole.split_at(20);
    let part = |i: u64, n: u64, text: &str| {
        json!({"v":1,"i":i,"kind":"part","at":1,"body":{"n":n,"m":2,"text":text}}).to_string()
    };
    lines[3] = part(4, 1, a);
    lines[4] = part(5, 2, b);
    st.put(&chunk_key("s", 0), lines.join("\n").into_bytes())
        .unwrap();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let page = t.read(0, None, 256).unwrap();
    let mut j = Joiner::new();
    let got: Vec<Value> = page.entries.iter().filter_map(|l| j.push(l)).collect();
    let ids: Vec<u64> = got.iter().map(|v| v["i"].as_u64().unwrap()).collect();
    assert_eq!(
        ids,
        vec![1, 2, 3, 5],
        "the joined whole must carry the i of its last part"
    );
}

/// A stored part group whose joined text has no `i`: the whole a reader joins has no index at
/// all, where 9.4 gives it the index of its last part (S14-fix2 Goal), as `render` does for a
/// readable line without one.
#[test]
fn adv_s14_part_group_whole_without_i_has_no_index() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    {
        let mut t = Transcript::new(&mut st, "s", &mut meta);
        for n in 0..5 {
            t.append(&msg(n), 1).unwrap();
        }
    }
    let raw = st.get(&chunk_key("s", 0)).unwrap();
    let mut lines: Vec<String> = String::from_utf8(raw)
        .unwrap()
        .split('\n')
        .map(String::from)
        .collect();
    let whole = r#"{"v":1,"kind":"message","at":1,"body":{"forged":true}}"#;
    let (a, b) = whole.split_at(20);
    let part = |i: u64, n: u64, text: &str| {
        json!({"v":1,"i":i,"kind":"part","at":1,"body":{"n":n,"m":2,"text":text}}).to_string()
    };
    lines[3] = part(4, 1, a);
    lines[4] = part(5, 2, b);
    st.put(&chunk_key("s", 0), lines.join("\n").into_bytes())
        .unwrap();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let page = t.read(0, None, 256).unwrap();
    let mut j = Joiner::new();
    let got: Vec<Value> = page.entries.iter().filter_map(|l| j.push(l)).collect();
    let ids: Vec<Value> = got.iter().map(|v| v["i"].clone()).collect();
    assert_eq!(ids, vec![json!(1), json!(2), json!(3), json!(5)]);
}
