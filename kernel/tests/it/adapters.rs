//! Tests of the adapters (TECHNICAL 9.8, 9.4, 16 case 5): golden fixtures, cut, unknown, purity.

use std::fs;
use std::path::{Path, PathBuf};

use agent_client_protocol_schema::v1::SessionUpdate;
use agent_kernel::adapters::{claude::ClaudeAdapter, cut_output, responses, unknown, Kind, Out};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture_dir(name: &str) -> PathBuf {
    root().join("tests/fixtures/adapters").join(name)
}

fn fixtures(name: &str) -> Vec<(String, Value)> {
    let mut files: Vec<PathBuf> = match fs::read_dir(fixture_dir(name)) {
        Ok(rd) => rd
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect(),
        Err(_) => vec![],
    };
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let v = serde_json::from_str(&fs::read_to_string(&p).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (p.file_name().unwrap().to_string_lossy().into_owned(), v)
        })
        .collect()
}

/// One input of a fixture of folder `dir`, run through that folder's adapter.
enum Adapter {
    Claude(ClaudeAdapter),
    Responses,
}

impl Adapter {
    fn new(dir: &str) -> Adapter {
        match dir {
            "claude" => Adapter::Claude(ClaudeAdapter::new()),
            "responses" => Adapter::Responses,
            other => panic!("no adapter for fixture folder {other}"),
        }
    }

    fn feed(&mut self, input: &Value) -> Vec<Out> {
        let at = input["at"].as_u64().expect("input.at");
        match self {
            Adapter::Claude(a) => match (input.get("line"), input.get("raw")) {
                (Some(line), _) => a.line(line.to_string().as_bytes(), at),
                (None, Some(raw)) => a.line(raw.as_str().unwrap().as_bytes(), at),
                _ => panic!("input has neither line nor raw"),
            },
            Adapter::Responses => match (input.get("event"), input.get("job")) {
                (Some(e), _) => responses::event(e, at),
                (None, Some(j)) => responses::job_output(
                    &j["call"],
                    j["output"].as_str().unwrap(),
                    j["ok"].as_bool().unwrap(),
                    at,
                ),
                _ => panic!("input has neither event nor job"),
            },
        }
    }
}

fn dirs() -> Vec<&'static str> {
    vec!["claude", "responses"]
}

#[test]
fn ac2_golden_fixtures_match() {
    let mut seen = 0;
    for dir in dirs() {
        for (name, fx) in fixtures(dir) {
            let inputs = fx["input"].as_array().unwrap();
            let expected = fx["expected"].as_array().unwrap();
            assert_eq!(
                inputs.len(),
                expected.len(),
                "{dir}/{name}: input and expected lengths"
            );
            let mut adapter = Adapter::new(dir);
            for (i, input) in inputs.iter().enumerate() {
                let got = serde_json::to_value(adapter.feed(input)).unwrap();
                assert_eq!(got, expected[i], "{dir}/{name}: input {i}");
                seen += 1;
            }
        }
    }
    assert!(seen > 0);
}

#[test]
fn ac2_every_row_of_9_8_has_a_fixture() {
    let spec = fs::read_to_string(root().join("../design/TECHNICAL.md")).unwrap();
    let start = spec.find("### 9.8 Adapters").expect("9.8");
    let rest = &spec[start..];
    let end = rest.find("Libraries:").expect("end of 9.8 table");
    let rows: Vec<String> = rest[..end]
        .lines()
        .filter(|l| l.starts_with('|'))
        .skip(2)
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .next()
                .unwrap()
                .trim()
                .to_string()
        })
        .collect();
    let have: Vec<String> = dirs()
        .into_iter()
        .flat_map(fixtures_of)
        .map(|fx| fx["row"].as_str().expect("row").to_string())
        .collect();
    assert_eq!(rows.len(), 12, "rows read: {rows:?}");
    for row in rows {
        assert!(have.contains(&row), "no fixture for the 9.8 row {row:?}");
    }
}

fn fixtures_of(dir: &str) -> Vec<Value> {
    fixtures(dir).into_iter().map(|(_, v)| v).collect()
}

#[test]
fn ac1_every_update_is_an_acp_session_update() {
    for dir in dirs() {
        for (name, fx) in fixtures(dir) {
            for (i, outs) in fx["expected"].as_array().unwrap().iter().enumerate() {
                for out in outs.as_array().unwrap() {
                    if let Some(u) = out.get("update") {
                        serde_json::from_value::<SessionUpdate>(u.clone())
                            .unwrap_or_else(|e| panic!("{dir}/{name} input {i}: {e}"));
                    }
                }
            }
        }
    }
}

fn line(a: &mut ClaudeAdapter, v: Value) -> Vec<Out> {
    a.line(v.to_string().as_bytes(), 1)
}

#[test]
fn ac6_cut_output_at_a_utf8_boundary() {
    let (s, cut) = cut_output("short");
    assert_eq!((s.as_str(), cut), ("short", None));
    let text = format!("{}{}", "a".repeat(65_535), "é".repeat(10));
    let (s, cut) = cut_output(&text);
    assert_eq!(s.len(), 65_535);
    assert_eq!(cut, Some(text.len()));
    let exact = "b".repeat(65_536);
    assert_eq!(cut_output(&exact).1, None);
}

#[test]
fn ac6_long_tool_result_is_cut_and_marked() {
    let mut a = ClaudeAdapter::new();
    let big = "x".repeat(70_000);
    let outs = line(
        &mut a,
        json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_01Z","content":big}]},"session_id":"5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11","uuid":"u1"}),
    );
    let body = &outs[0].entry.as_ref().unwrap().body;
    assert_eq!(body["_meta"], json!({"watane": {"cut": 70_000}}));
    assert_eq!(
        body["content"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .len(),
        65_536
    );
}

#[test]
fn ac10_unknown_then_normal() {
    let mut a = ClaudeAdapter::new();
    let outs = a.line(b"\xff\xfe not json", 1);
    assert_eq!(outs.len(), 1);
    assert_eq!(outs[0].entry.as_ref().unwrap().kind, Kind::Unknown);
    assert_eq!(unknown(json!(1)).entry.unwrap().body, json!({"raw": 1}));
    assert!(a.line(b"  \n", 1).is_empty(), "a blank line is no item");
    let ok = line(
        &mut a,
        json!({"type":"assistant","message":{"id":"m","role":"assistant","model":"x","content":[{"type":"text","text":"hi"}]},"session_id":"s","uuid":"u"}),
    );
    assert_eq!(ok[0].entry.as_ref().unwrap().kind, Kind::Message);
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn random_value(r: &mut Rng) -> Value {
    match r.below(6) {
        0 => Value::Null,
        1 => json!(r.next() as i64),
        2 => json!("zz"),
        3 => json!([1, "a", null]),
        4 => json!({"k": [true]}),
        _ => json!(r.below(2) == 0),
    }
}

/// Replace one random node of `v` with a random value.
fn mutate(v: &mut Value, r: &mut Rng) {
    let children = match v {
        Value::Object(o) => o.len(),
        Value::Array(a) => a.len(),
        _ => 0,
    };
    if children == 0 || r.below(4) == 0 {
        *v = random_value(r);
        return;
    }
    let n = r.below(children);
    match v {
        Value::Object(o) => mutate(o.values_mut().nth(n).unwrap(), r),
        Value::Array(a) => mutate(&mut a[n], r),
        _ => {}
    }
}

#[test]
fn ac10_never_panics_on_random_input() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut a = ClaudeAdapter::new();
    for _ in 0..1000 {
        let n = r.below(80);
        let bytes: Vec<u8> = (0..n).map(|_| r.next() as u8).collect();
        let _: Vec<Out> = a.line(&bytes, 1);
    }
    let lines: Vec<Value> = fixtures("claude")
        .iter()
        .flat_map(|(_, fx)| fx["input"].as_array().unwrap().clone())
        .filter_map(|i| i.get("line").cloned())
        .collect();
    assert!(!lines.is_empty());
    for _ in 0..1000 {
        let mut v = lines[r.below(lines.len())].clone();
        mutate(&mut v, &mut r);
        let outs = a.line(v.to_string().as_bytes(), 1);
        assert!(outs.len() < 100);
    }
}

#[test]
fn ac12_adapters_are_pure() {
    let banned = [
        "std::fs",
        "std::time",
        "std::env",
        "std::net",
        "std::process",
        "static mut",
        "thread_local",
        "lazy_static",
        "OnceLock",
        "reqwest",
        "tokio",
    ];
    for entry in fs::read_dir(root().join("src/adapters")).unwrap() {
        let p = entry.unwrap().path();
        let src = fs::read_to_string(&p).unwrap();
        for b in banned {
            assert!(!src.contains(b), "{} uses {b}", p.display());
        }
    }
}

#[test]
fn ac5_responses_never_panics_on_random_input() {
    let mut r = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..1000 {
        let v = random_value(&mut r);
        let _: Vec<Out> = responses::event(&v, 1);
    }
    let events: Vec<Value> = fixtures("responses")
        .iter()
        .flat_map(|(_, fx)| fx["input"].as_array().unwrap().clone())
        .filter_map(|i| i.get("event").cloned())
        .collect();
    assert!(!events.is_empty());
    for _ in 0..1000 {
        let mut v = events[r.below(events.len())].clone();
        mutate(&mut v, &mut r);
        assert!(responses::event(&v, 1).len() < 100);
        let mut call = json!({"type":"function_call","call_id":"c","name":"n","arguments":"{}"});
        mutate(&mut call, &mut r);
        assert!(responses::job_output(&call, "o", r.below(2) == 0, 1).len() < 100);
    }
}

#[test]
fn ac6_s12_bad_utf8_line_is_unknown_with_escaped_bytes() {
    let mut a = ClaudeAdapter::new();
    let outs = a.line(b"{\"type\":\"initialized\"}\xff", 1);
    assert_eq!(outs.len(), 1);
    let e = outs[0].entry.as_ref().unwrap();
    assert_eq!(e.kind, Kind::Unknown);
    assert_eq!(e.body, json!({"raw": "{\"type\":\"initialized\"}\\xff"}));
}

#[test]
fn ac7_hook_and_progress_lines_give_nothing() {
    let mut a = ClaudeAdapter::new();
    for v in [
        json!({"type":"system","subtype":"hook_started","hook_id":"h"}),
        json!({"type":"system","subtype":"hook_response","hook_id":"h","outcome":"success"}),
        json!({"type":"system","subtype":"tool_progress"}),
        json!({"type":"system","subtype":"task_progress"}),
        json!({"type":"tool_progress","tool_use_id":"t","elapsed_time_seconds":2}),
        json!({"type":"task_progress","task_id":"t"}),
    ] {
        assert!(line(&mut a, v.clone()).is_empty(), "{v}");
    }
}

#[test]
fn ac8_at_merges_with_the_cut_marker() {
    let mut a = ClaudeAdapter::new();
    let big = "x".repeat(70_000);
    let outs = a.line(
        json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":big}]},"session_id":"5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11","uuid":"u1"})
            .to_string()
            .as_bytes(),
        42,
    );
    assert_eq!(
        outs[0].update.as_ref().unwrap()["_meta"],
        json!({"watane": {"cut": 70_000, "at": 42}})
    );
}

#[test]
fn ac9_thought_seconds_round_and_clear_at_result() {
    let mut a = ClaudeAdapter::new();
    let delta = |t: &str| json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":t}},"session_id":"s","uuid":"u"});
    let msg = |t: &str| json!({"type":"assistant","message":{"id":"m","role":"assistant","model":"x","content":[{"type":"text","text":t}]},"session_id":"s","uuid":"u"});
    a.line(delta("a").to_string().as_bytes(), 1000);
    a.line(delta("b").to_string().as_bytes(), 2500);
    let outs = a.line(msg("hi").to_string().as_bytes(), 3000);
    assert_eq!(
        outs[0].entry.as_ref().unwrap().body["thought"],
        json!({"text": "ab", "seconds": 2})
    );
    a.line(delta("c").to_string().as_bytes(), 5000);
    a.line(
        json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"duration_api_ms":1,"num_turns":1,"session_id":"s","total_cost_usd":0.0,"usage":{}})
            .to_string()
            .as_bytes(),
        6000,
    );
    let outs = a.line(msg("again").to_string().as_bytes(), 7000);
    assert!(outs[0]
        .entry
        .as_ref()
        .unwrap()
        .body
        .get("thought")
        .is_none());
}

#[test]
fn ac9_thinking_blocks_join_with_a_blank_line_and_replace_their_deltas() {
    let mut a = ClaudeAdapter::new();
    let delta = |t: &str| json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":t}},"session_id":"s","uuid":"u"});
    let think = |t: &str| json!({"type":"assistant","message":{"id":"m","role":"assistant","model":"x","content":[{"type":"thinking","thinking":t,"signature":"g"}]},"session_id":"s","uuid":"u"});
    let msg = json!({"type":"assistant","message":{"id":"m","role":"assistant","model":"x","content":[{"type":"text","text":"ok"}]},"session_id":"s","uuid":"u"});
    for (v, at) in [
        (delta("A"), 1000),
        (think("A"), 1100),
        (think("B"), 2000),
        (delta("C"), 4000),
    ] {
        a.line(v.to_string().as_bytes(), at);
    }
    let outs = a.line(msg.to_string().as_bytes(), 5000);
    assert_eq!(
        outs[0].entry.as_ref().unwrap().body["thought"],
        json!({"text": "A\n\nB\n\nC", "seconds": 3})
    );
}

#[test]
fn ac1_invalid_bytes_are_escaped_and_nothing_is_parsed() {
    let mut a = ClaudeAdapter::new();
    let raw = |outs: Vec<Out>| {
        assert_eq!(outs.len(), 1);
        let e = outs[0].entry.as_ref().unwrap();
        assert_eq!(e.kind, Kind::Unknown);
        e.body.clone()
    };
    assert_eq!(raw(a.line(b"\xff\xfe", 1)), json!({"raw": "\\xff\\xfe"}));
    let mut ask =
        br#"{"type":"ask","id":"a","tool":"Bash","input":{},"tool_use_id":"t","expires-at":5}"#
            .to_vec();
    ask.push(0xff);
    assert!(raw(a.line(&ask, 1))["raw"]
        .as_str()
        .unwrap()
        .ends_with("\\xff"));
    let src = fs::read_dir(root().join("src/adapters")).unwrap();
    for f in src {
        let text = fs::read_to_string(f.unwrap().path()).unwrap();
        assert!(!text.contains(concat!("from_utf8", "_lossy")));
    }
}
