//! S36 review round 1 (Codex): a session allow grants only the reach its ask offered (F1.1), and
//! its `allow` entry is pushed in transcript order (F1.2). TECHNICAL 9.3 `folder`, 9.4, 9.6, 14.

use agent_kernel::active::Slots;
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
    crate::fixture_access::put(&mut st, "access/alice", br#"["p"]"#.to_vec()).unwrap();
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

/// F1.1: the first `system/init` decides the folder, even when it names none: a later init's
/// `cwd` never gives an open ask a reach it did not have.
#[test]
fn regress_s36_first_init_without_cwd_is_authoritative() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    feed(&mut k, &t, &mut n, &[init(None), read("a1", "/w/a.txt")]);
    assert!(stored_ask(&mut k, &s, "a1").get("reach").is_none());
    feed(&mut k, &t, &mut n, &[init(Some("/w"))]);
    assert_eq!(rec(&mut k, &s).folder, None);
    assert!(k.answer("alice", &s, "k1", "a1", SESSION).is_err());
    assert!(rec(&mut k, &s).allows.is_empty());
}

/// F1.1: a session allow grants only the reach recorded on its ask; a scope the kernel would
/// compute now (the folder learned later) but the ask never offered is refused.
#[test]
fn regress_s36_session_allow_must_match_the_asks_reach() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    feed(&mut k, &t, &mut n, &[read("a1", "/w/a.txt")]);
    assert!(stored_ask(&mut k, &s, "a1").get("reach").is_none());
    // the folder arrives after the ask opened, by whatever route
    let mut r = rec(&mut k, &s);
    r.folder = Some("/w".into());
    r.save(&mut k.state).unwrap();
    assert!(k.answer("alice", &s, "k1", "a1", SESSION).is_err());
    assert!(rec(&mut k, &s).allows.is_empty());
    // a plain allow of the same ask still goes through
    k.answer("alice", &s, "k2", "a1", r#"{"allow":true}"#)
        .unwrap();
}

/// F1.2: the answer's pushes come in transcript order: the `allow` entry before the turn entry
/// the closed ask causes.
#[test]
fn regress_s36_session_allow_pushes_in_transcript_order() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    feed(
        &mut k,
        &t,
        &mut n,
        &[init(Some("/w")), read("a1", "/w/a.txt")],
    );
    let pushes = k.answer("alice", &s, "k1", "a1", SESSION).unwrap().pushes;
    let entries: Vec<(u64, String)> = pushes
        .iter()
        .filter(|p| p.value["type"] == "entry")
        .map(|p| {
            let e = &p.value["entry"];
            (
                e["i"].as_u64().unwrap(),
                e["kind"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert!(entries.iter().any(|(_, k)| k == "allow"), "{entries:?}");
    let is: Vec<u64> = entries.iter().map(|(i, _)| *i).collect();
    let mut sorted = is.clone();
    sorted.sort();
    assert_eq!(is, sorted, "{entries:?}");
}

/// F1.1: an init the kernel cannot read (a key twice) still uses up the first init.
#[test]
fn regress_s36_an_unread_init_uses_up_the_first_init() {
    let mut k = kernel();
    let (s, t) = started(&mut k);
    let mut n = 0;
    let dup = r#"{"type":"system","subtype":"init","cwd":"/x","cwd":"/x"}"#.to_string();
    feed(
        &mut k,
        &t,
        &mut n,
        &[dup, init(Some("/w")), read("a1", "/w/a.txt")],
    );
    assert_eq!(rec(&mut k, &s).folder, None);
    assert!(stored_ask(&mut k, &s, "a1").get("reach").is_none());
    let unknown = k
        .read("alice", &s, 0, None, 512)
        .unwrap()
        .entries
        .iter()
        .any(|e| e.contains("\"kind\":\"unknown\""));
    assert!(unknown);
}
