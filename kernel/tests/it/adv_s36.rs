//! S36 r1 adversary: the first `system/init` decides the folder, even one the kernel cannot
//! read (TECHNICAL 4.6 "Its folder, from system/init", 9.3 `folder`, 14 "A session allow covers
//! only what the participant saw").

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
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

fn started(k: &mut K) -> (String, String) {
    let id = k.start("alice", "k", "p", "{}", IN).unwrap().value;
    k.attach("alice", &id, 0).unwrap();
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    (id, t)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn feed(k: &mut K, t: &str, n: &mut u64, raw: &[String]) {
    let lines = raw
        .iter()
        .enumerate()
        .map(|(i, l)| (*n + 1 + i as u64, l.as_bytes().to_vec()))
        .collect();
    k.provider_lines(t, lines).unwrap();
    *n += raw.len() as u64;
}

fn init(cwd: Option<&str>) -> String {
    let mut v = json!({"type":"system","subtype":"init","session_id":UUID});
    if let Some(c) = cwd {
        v["cwd"] = json!(c);
    }
    v.to_string()
}

fn read(id: &str, path: &str) -> String {
    json!({"type":"ask","id":id,"tool":"Read","input":{"file_path":path},"path":path,
        "tool_use_id":format!("t-{id}"),"expires-at":9000})
    .to_string()
}

#[allow(dead_code)]
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


/// A `system/init` with a key twice whose `subtype` spells `init` with a JSON escape: every JSON
/// reader takes it as `init`, but `unread_init` looks for the literal bytes `"init"`, so it does
/// not use up the first init and a later init's `cwd` becomes the folder.
#[test]
fn adv_s36_escaped_unread_init_does_not_use_up_the_first_init() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    // `\u0069` is `i`: the subtype reads `init`, but the line never holds the bytes `"init"`.
    let esc = format!("{}u0069nit", '\\');
    let dup = format!(r#"{{"type":"system","subtype":"{esc}","cwd":"/x","cwd":"/x"}}"#);
    assert!(!dup.contains("\"init\""));
    feed(&mut k, &t, &mut n, &[dup, init(Some("/w"))]);
    assert!(rec(&mut k, &s).init_seen, "the first init was not used up");
    assert_eq!(rec(&mut k, &s).folder, None);
}

/// A `system/init` the kernel cannot read for another reason (a lone surrogate escape, which
/// `JSON.stringify` writes and serde_json refuses): it is no fact and no `unread_init`, so the
/// first init is not used up and a later init's `cwd` becomes the folder.
#[test]
fn adv_s36_unparsable_init_does_not_use_up_the_first_init() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    let bad = format!(
        r#"{{"type":"system","subtype":"init","session_id":"{UUID}","cwd":"/x","model":"\ud800"}}"#
    );
    feed(&mut k, &t, &mut n, &[bad, init(Some("/w"))]);
    assert!(rec(&mut k, &s).init_seen, "the first init was not used up");
    assert_eq!(rec(&mut k, &s).folder, None);
}
