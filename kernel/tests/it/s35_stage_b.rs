//! S35-1: the kernel crate runs a real Claude session's turn (TECHNICAL 2.1 "The kernel's side of
//! the seam", 4.1, 4.6, 9.1, 9.3, 9.8 "Adapter state"): the launch document, raw provider lines
//! through the adapter with its state kept in `adapter/<stream>`, later turns, the cause of an
//! ended stream, the built-in `scratch` profile, `Buffered` state.

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// A state that logs every key it is asked to put, to prove how often a call writes a key.
#[derive(Default)]
struct Spy {
    inner: MemState,
    puts: Vec<String>,
}

impl State for Spy {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.inner.get(key)
    }
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        self.puts.push(key.to_string());
        self.inner.put(key, value)
    }
    fn delete(&mut self, key: &str) {
        self.inner.delete(key)
    }
}

type K = Kernel<Spy, FakeHost>;

const P: &str =
    r#"{"backend":"claude","model":"sonnet","tools":["Read"],"allow":["Read(**)"],"partial":true}"#;
const IN: &str = r#"[{"type":"text","text":"hello"}]"#;
const WS: &str = "scratch:0123456789abcdef0123456789abcdef";
const UUID: &str = "5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11";

fn kernel() -> K {
    let mut st = Spy::default();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
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

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn dump(k: &K) -> Vec<(String, Vec<u8>)> {
    k.state
        .inner
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn fake_lines() -> Vec<Vec<u8>> {
    include_str!("../fixtures/adapters/fake_default.jsonl")
        .lines()
        .map(|l| l.as_bytes().to_vec())
        .collect()
}

fn lines(raw: &[&str], from: u64) -> Vec<(u64, Vec<u8>)> {
    raw.iter()
        .enumerate()
        .map(|(i, l)| (from + i as u64, l.as_bytes().to_vec()))
        .collect()
}

fn launches(k: &K) -> Vec<(Value, String)> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::StartProvider { launch, input } => {
                Some((serde_json::from_str(launch).unwrap(), input.clone()))
            }
            _ => None,
        })
        .collect()
}

fn requests(k: &K) -> Vec<Request> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn agent_messages(k: &mut K, id: &str) -> Vec<Value> {
    entries(k, id)
        .into_iter()
        .filter(|e| e["kind"] == "message" && e["body"]["role"] == "agent")
        .collect()
}

/// Starts alice's session on `p` and attaches her owner stream; the stream names.
fn session(k: &mut K) -> (String, String, String) {
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let provider = rec(k, &id).streams[0].clone();
    let Attached::Live { stream: owner } = k.attach("alice", &id, 0).unwrap().value else {
        panic!("not live");
    };
    (id, provider, owner)
}

fn adapter_puts(k: &K) -> usize {
    k.state
        .puts
        .iter()
        .filter(|p| p.starts_with("adapter/"))
        .count()
}

// AC1

#[test]
fn ac1_the_launch_is_the_document_of_4_1() {
    let mut k = kernel();
    k.start("alice", "k1", "p", "{}", IN).unwrap();
    let (launch, input) = launches(&k).pop().unwrap();
    assert_eq!(input, IN);
    assert_eq!(
        launch,
        json!({"version":1,"profile":"p","workspace":"scratch","model":"sonnet","tools":["Read"],
               "allow":["Read(**)"],"partial":true})
    );
    // an overlay's narrowing shows
    let mut k = kernel();
    k.state
        .put(
            "profile/q",
            br#"{"model":"sonnet","tools":["Read"],"max-turns":20}"#.to_vec(),
        )
        .unwrap();
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p","q"]"#.to_vec()).unwrap();
    k.start(
        "alice",
        "k1",
        "q",
        r#"{"tools-":["Read"],"max-turns":3}"#,
        IN,
    )
    .unwrap();
    let (launch, _) = launches(&k).pop().unwrap();
    assert_eq!(launch["tools"], json!([]));
    assert_eq!(launch["max-turns"], 3);
}

#[test]
fn ac1_a_profile_name_outside_the_grammar_is_denied() {
    let mut k = kernel();
    k.state
        .put("profile/a b", br#"{"model":"sonnet"}"#.to_vec())
        .unwrap();
    k.state
        .put("profile/long", br#"{"model":"sonnet"}"#.to_vec())
        .unwrap();
    let long = "x".repeat(65);
    k.state
        .put(
            &format!("profile/{long}"),
            br#"{"model":"sonnet"}"#.to_vec(),
        )
        .unwrap();
    // Seed a raw pinned grant for the bad name: the fixture helper must not quietly omit it.
    let bad = br#"{"model":"sonnet"}"#;
    let digest = agent_kernel::access::digest(bad);
    k.state
        .put(
            "access/alice",
            serde_json::to_vec(&json!([{"profile":"a b","digest":digest,"rev":1}])).unwrap(),
        )
        .unwrap();
    k.state
        .put(
            "profiles",
            serde_json::to_vec(&json!([{"name":"a b","digest":digest,"rev":1,"bytes":bad.len()}]))
                .unwrap(),
        )
        .unwrap();
    assert!(matches!(
        k.start("alice", "bad", "a b", "{}", IN),
        Err(KernelError::Refused(ref v)) if v[0].field == "profile"
    ));
    k.state
        .put(
            "access/alice",
            serde_json::to_vec(&json!([{"profile":long,"digest":digest,"rev":1}])).unwrap(),
        )
        .unwrap();
    k.state
        .put(
            "profiles",
            serde_json::to_vec(&json!([{"name":long,"digest":digest,"rev":1,"bytes":bad.len()}]))
                .unwrap(),
        )
        .unwrap();
    let r = k.start("alice", "long-bad", &long, "{}", IN);
    assert!(matches!(r, Err(KernelError::Refused(ref v)) if v[0].field == "profile"));
    k.state.delete("profiles");
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["long"]"#.to_vec()).unwrap();
    let id = k.start("alice", "valid", "long", "{}", IN).unwrap().value;
    assert!(!id.is_empty());
}

// AC2

#[test]
fn ac2_start_sends_no_turn_request() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    assert!(requests(&k).is_empty());
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
    let es = entries(&mut k, &id);
    assert_eq!(es[0]["kind"], "message");
    assert_eq!(es[0]["body"]["role"], "user");
}

// AC3

/// The `by` of the one decision an `answered` host item gives an ask, on a stream the host
/// answered `direct`ly or not.
fn decided_by(not_direct: bool) -> (Value, String) {
    let mut k = kernel();
    k.host.not_direct = not_direct;
    let (id, t, _) = session(&mut k);
    let key: Value = serde_json::from_slice(&k.state.get(&format!("stream/{t}")).unwrap()).unwrap();
    let ask = r#"{"type":"ask","id":"a1","tool_use_id":"tu1","tool":"Read","input":{"file_path":"/x"},"expires-at":9999}"#;
    let resolved = r#"{"type":"ask-resolved","id":"a1","by":"answered","outcome":"deny"}"#;
    k.provider_lines(&t, lines(&[ask], 1)).unwrap();
    // a second call: the adapter remembers the open ask from the first
    k.provider_lines(&t, lines(&[resolved], 2)).unwrap();
    let decided: Vec<Value> = entries(&mut k, &id)
        .into_iter()
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .collect();
    assert_eq!(decided.len(), 1);
    (
        key,
        decided[0]["body"]["decision"]["by"]
            .as_str()
            .unwrap()
            .to_string(),
    )
}

#[test]
fn ac3_a_checked_stream_is_kept_and_read_by_the_checked_adapter() {
    let (key, by) = decided_by(false);
    assert!(key.get("checked").is_none(), "{key}");
    assert_eq!(by, "host");
    let (key, by) = decided_by(true);
    assert_eq!(key["checked"], true);
    assert_eq!(by, "checked");
}

#[test]
fn ac3_a_failed_provider_start_refuses_and_writes_nothing() {
    let mut k = kernel();
    k.host.no_provider = true;
    let before = dump(&k);
    assert_eq!(
        k.start("alice", "k1", "p", "{}", IN).unwrap_err(),
        KernelError::Backend("no Claude Code provider is linked".into())
    );
    assert_eq!(dump(&k), before);
}

// AC4

fn run_fake(batched: bool) -> (K, String, Vec<Value>, Vec<Value>) {
    let mut k = kernel();
    let (id, t, o) = session(&mut k);
    let raw = fake_lines();
    let from = k.host.calls.len();
    if batched {
        let batch = raw
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, l)| (i as u64 + 1, l))
            .collect();
        assert_eq!(
            k.provider_lines(&t, batch).unwrap().value,
            SinkOutcome::Taken
        );
    } else {
        for (i, l) in raw.into_iter().enumerate() {
            let r = k.provider_lines(&t, vec![(i as u64 + 1, l)]).unwrap();
            assert_eq!(r.value, SinkOutcome::Taken);
        }
    }
    let live: Vec<Value> = k.host.calls[from..]
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream, value } if *stream == o => Some(value.clone()),
            _ => None,
        })
        .collect();
    let read = entries(&mut k, &id);
    (k, id, live, read)
}

#[test]
fn ac4_the_fake_default_turn_as_a_batch_and_as_single_lines() {
    let (mut a, ida, live_a, read_a) = run_fake(true);
    let (mut b, idb, live_b, read_b) = run_fake(false);
    for (k, id, live, read) in [(&mut a, ida, live_a, read_a), (&mut b, idb, live_b, read_b)] {
        // the delta live, then the message once, then the turn
        let kinds: Vec<&str> = live
            .iter()
            .map(|v| {
                v["sessionUpdate"]
                    .as_str()
                    .or_else(|| v["_meta"]["watane"]["type"].as_str())
                    .unwrap()
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "agent_message_chunk",
                "agent_message_chunk",
                "usage",
                "entry",
                "turn"
            ],
            "{live:?}"
        );
        assert_eq!(live[0]["content"]["text"], "This is the fake claude.");
        assert!(
            live[0]["_meta"]["watane"].get("i").is_none(),
            "the delta is not an entry"
        );
        let messages = agent_messages(k, &id);
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0]["body"]["content"][0]["text"],
            "This is the fake claude."
        );
        // the update that finishes the entry carries its index
        assert_eq!(live[1]["_meta"]["watane"]["i"], messages[0]["i"]);
        assert_eq!(
            live[4]["_meta"]["watane"]["state"],
            json!({"state":"idle","stop":{"kind":"completed"},"seconds":0})
        );
        let turns: Vec<&Value> = read.iter().filter(|e| e["kind"] == "turn").collect();
        assert_eq!(turns.last().unwrap()["body"]["state"], "idle");
        assert_eq!(rec(k, &id).claude_uuid.as_deref(), Some(UUID));
    }
    // one batch and one item per call read the same
    let strip = |v: &[Value]| -> Vec<Value> {
        v.iter()
            .map(|e| {
                let mut e = e.clone();
                e["at"] = json!(0);
                e
            })
            .collect()
    };
    assert_eq!(strip(&read_of(&mut a)), strip(&read_of(&mut b)));
}

fn read_of(k: &mut K) -> Vec<Value> {
    entries(k, "s1")
}

// AC5

#[test]
fn ac5_thinking_in_one_call_and_the_block_in_a_later_call_is_one_thought() {
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me think."}}}"#;
    let block = r#"{"type":"assistant","message":{"id":"m","type":"message","role":"assistant","model":"x","content":[{"type":"thinking","thinking":"Let me think.","signature":"s"}]},"session_id":"s","uuid":"a1"}"#;
    let text = r#"{"type":"assistant","message":{"id":"m","type":"message","role":"assistant","model":"x","content":[{"type":"text","text":"Done."}]},"session_id":"s","uuid":"a2"}"#;
    k.provider_lines(&t, lines(&[delta], 1)).unwrap();
    k.provider_lines(&t, lines(&[block], 2)).unwrap();
    k.provider_lines(&t, lines(&[text], 3)).unwrap();
    let m = agent_messages(&mut k, &id);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0]["body"]["thought"]["text"], "Let me think.");
}

#[test]
fn ac5_a_tool_use_in_one_call_and_its_result_in_a_later_call_is_one_tool_entry() {
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    let tu = r#"{"type":"assistant","message":{"id":"m","type":"message","role":"assistant","model":"x","content":[{"type":"tool_use","id":"tu1","name":"Read","input":{"file_path":"/a"}}]},"session_id":"s","uuid":"a1"}"#;
    let tr = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"tu1","type":"tool_result","content":"ok","is_error":false}]},"parent_tool_use_id":null,"session_id":"5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11","uuid":"e6000000-0000-4000-8000-000000000001"}"#;
    k.provider_lines(&t, lines(&[tu], 1)).unwrap();
    k.provider_lines(&t, lines(&[tr], 2)).unwrap();
    let tools: Vec<Value> = entries(&mut k, &id)
        .into_iter()
        .filter(|e| e["kind"] == "tool")
        .collect();
    assert_eq!(tools.len(), 1, "{:?}", entries(&mut k, &id));
    assert_eq!(tools[0]["body"]["title"], "Read");
}

#[test]
fn ac5_adapter_state_is_written_once_per_call_and_goes_with_the_stream() {
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    let tu = r#"{"type":"assistant","message":{"id":"m","type":"message","role":"assistant","model":"x","content":[{"type":"tool_use","id":"tu1","name":"Read","input":{"file_path":"/a"}}]},"session_id":"s","uuid":"a1"}"#;
    let tu2 = tu.replace("tu1", "tu2");
    let tu3 = tu.replace("tu1", "tu3");
    k.state.puts.clear();
    k.provider_lines(&t, lines(&[tu, &tu2, &tu3], 1)).unwrap();
    assert_eq!(adapter_puts(&k), 1);
    assert!(k.state.get(&format!("adapter/{t}")).is_some());
    // charged to the session
    assert!(rec(&mut k, &id).bytes >= (format!("adapter/{t}").len()) as u64);
    // ended
    k.provider_ended(&t).unwrap();
    assert!(k.state.get(&format!("adapter/{t}")).is_none());

    // cancel
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    k.provider_lines(&t, lines(&[tu], 1)).unwrap();
    assert!(k.state.get(&format!("adapter/{t}")).is_some());
    k.cancel("alice", &id).unwrap();
    assert!(k.state.get(&format!("adapter/{t}")).is_none());

    // delete
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    k.provider_lines(&t, lines(&[tu], 1)).unwrap();
    k.delete("alice", &id).unwrap();
    assert!(k.state.get(&format!("adapter/{t}")).is_none());
    let left: Vec<String> = dump(&k)
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with("adapter/"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn ac5_a_deferred_call_leaves_the_state_unchanged() {
    let mut k = kernel();
    let (_, t, o) = session(&mut k);
    k.host.full = BTreeSet::from([o]);
    let before = dump(&k);
    let r = k
        .provider_lines(&t, (1..).zip(fake_lines()).collect())
        .unwrap();
    assert_eq!(r.value, SinkOutcome::Defer);
    assert_eq!(dump(&k), before);
}

// AC6

#[test]
fn ac6_initialized_sets_the_workspace_once() {
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    let init = format!(r#"{{"type":"initialized","workspace":"{WS}"}}"#);
    let before = entries(&mut k, &id).len();
    k.provider_lines(&t, lines(&[&init], 1)).unwrap();
    assert_eq!(rec(&mut k, &id).workspace.as_deref(), Some(WS));
    assert_eq!(entries(&mut k, &id).len(), before);
    // the same id again: nothing
    k.provider_lines(&t, lines(&[&init], 2)).unwrap();
    assert_eq!(entries(&mut k, &id).len(), before);
    // another id: an unknown entry, the workspace kept
    let other = r#"{"type":"initialized","workspace":"scratch:ffffffffffffffffffffffffffffffff"}"#;
    k.provider_lines(&t, lines(&[other], 3)).unwrap();
    assert_eq!(rec(&mut k, &id).workspace.as_deref(), Some(WS));
    let es = entries(&mut k, &id);
    assert_eq!(es.len(), before + 1);
    assert_eq!(es.last().unwrap()["kind"], "unknown");
}

#[test]
fn ac6_system_init_sets_the_claude_uuid_and_gives_no_entry() {
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    let before = entries(&mut k, &id).len();
    let init = format!(r#"{{"type":"system","subtype":"init","session_id":"{UUID}"}}"#);
    k.provider_lines(&t, lines(&[&init], 1)).unwrap();
    assert_eq!(rec(&mut k, &id).claude_uuid.as_deref(), Some(UUID));
    assert_eq!(entries(&mut k, &id).len(), before);
}

// AC7

#[test]
fn ac7_input_text_joins_the_text_blocks() {
    let blocks = r#"[{"type":"text","text":"a"},{"type":"image","data":"x","mimeType":"image/png"},{"type":"text","text":"b"}]"#;
    assert_eq!(input_text(blocks), "a\n\nb");
    assert_eq!(input_text("plain"), "plain");
    assert_eq!(input_text("[]"), "");
}

fn idle_session(k: &mut K) -> (String, String) {
    let (id, t, _) = session(k);
    let init = format!(r#"{{"type":"initialized","workspace":"{WS}"}}"#);
    let sys = format!(r#"{{"type":"system","subtype":"init","session_id":"{UUID}"}}"#);
    let mut batch = vec![(1, init.into_bytes()), (2, sys.into_bytes())];
    batch.extend((3..).zip(fake_lines().into_iter().skip(1)));
    k.provider_lines(&t, batch).unwrap();
    assert!(matches!(rec(k, &id).turn, Turn::Idle { .. }));
    (id, t)
}

#[test]
fn ac7_say_on_a_live_stream_sends_a_turn_request() {
    let mut k = kernel();
    let (id, t) = idle_session(&mut k);
    k.host.calls.clear();
    k.say("alice", &id, "s1", "again").unwrap();
    assert_eq!(
        requests(&k),
        [Request::Turn {
            session: id,
            stream: t,
            direct: true,
            input: "again".into(),
            resume: false
        }]
    );
    assert!(launches(&k).is_empty());
}

#[test]
fn ac7_say_with_no_live_stream_starts_one_in_the_same_workspace_and_resumes() {
    let mut k = kernel();
    let (id, t) = idle_session(&mut k);
    k.provider_ended(&t).unwrap();
    // the provider's `start` comes before the first write: refused, nothing changes
    k.host.no_provider = true;
    let before = dump(&k);
    assert!(matches!(
        k.say("alice", &id, "s1", "again"),
        Err(KernelError::Backend(_))
    ));
    assert_eq!(dump(&k), before);
    k.host.no_provider = false;
    k.host.calls.clear();
    k.say("alice", &id, "s1", "again").unwrap();
    let (launch, input) = launches(&k).pop().unwrap();
    assert_eq!(input, "again");
    assert_eq!(
        launch,
        json!({"version":1,"profile":"p","workspace":WS,"model":"sonnet","tools":["Read"],
               "allow":["Read(**)"],"partial":true,"session":{"resume":UUID}})
    );
    assert!(requests(&k).is_empty());
    assert_eq!(
        rec(&mut k, &id).streams.len(),
        2,
        "the new provider stream and the owner's"
    );
}

#[test]
fn ac7_a_later_launch_keeps_the_overlay_of_the_first() {
    let mut k = kernel();
    let overlay = r#"{"tools-":["Read"],"deny+":["Bash"],"max-turns":3}"#;
    let id = k.start("alice", "k1", "p", overlay, IN).unwrap().value;
    let t = rec(&mut k, &id).streams[0].clone();
    let (first, _) = launches(&k).pop().unwrap();
    assert_eq!(first["tools"], json!([]));
    let init = format!(r#"{{"type":"initialized","workspace":"{WS}"}}"#);
    let sys = format!(r#"{{"type":"system","subtype":"init","session_id":"{UUID}"}}"#);
    let mut batch = vec![(1, init.into_bytes()), (2, sys.into_bytes())];
    batch.extend((3..).zip(fake_lines().into_iter().skip(1)));
    k.provider_lines(&t, batch).unwrap();
    k.provider_ended(&t).unwrap();
    k.host.calls.clear();
    k.say("alice", &id, "s1", "again").unwrap();
    let (later, _) = launches(&k).pop().unwrap();
    let mut expected = first;
    expected["workspace"] = json!(WS);
    expected["session"] = json!({ "resume": UUID });
    assert_eq!(later, expected, "never wider than the first launch");
}

#[test]
fn ac7_a_profile_with_persist_false_resumes_nothing_and_no_workspace_means_scratch() {
    let mut k = kernel();
    k.state
        .put(
            "profile/np",
            br#"{"model":"sonnet","persist":false}"#.to_vec(),
        )
        .unwrap();
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p","np"]"#.to_vec()).unwrap();
    let id = k.start("alice", "k1", "np", "{}", IN).unwrap().value;
    let t = rec(&mut k, &id).streams[0].clone();
    // the Claude session id is kept, but never resumed; no `initialized` came: scratch
    let sys = format!(r#"{{"type":"system","subtype":"init","session_id":"{UUID}"}}"#);
    k.provider_lines(&t, lines(&[&sys], 1)).unwrap();
    k.provider_ended(&t).unwrap();
    k.host.calls.clear();
    k.say("alice", &id, "s1", "again").unwrap();
    let (launch, _) = launches(&k).pop().unwrap();
    assert_eq!(launch["workspace"], "scratch");
    assert!(launch.get("session").is_none());
}

#[test]
fn ac7_interrupt_answer_and_cancel_carry_the_stream() {
    for direct in [true, false] {
        let mut k = kernel();
        k.host.not_direct = !direct;
        let (id, t, _) = session(&mut k);
        super::initialized(&mut k, &t);
        k.host.calls.clear();
        let ask = r#"{"type":"ask","id":"a1","tool_use_id":"tu1","tool":"Read","input":{"file_path":"/x"},"expires-at":9999}"#;
        k.provider_lines(&t, lines(&[ask], 2)).unwrap();
        k.answer("alice", &id, "ka", "a1", r#"{"deny":true}"#)
            .unwrap();
        k.interrupt("alice", &id, None).unwrap();
        k.cancel("alice", &id).unwrap();
        let r = requests(&k);
        assert_eq!(
            r,
            [
                Request::Answer {
                    session: id.clone(),
                    stream: t.clone(),
                    direct,
                    ask: "a1".into(),
                    decision: json!({"deny": true})
                },
                Request::Interrupt {
                    session: id.clone(),
                    stream: t.clone(),
                    direct
                },
                Request::Cancel {
                    session: id.clone(),
                    stream: t.clone()
                }
            ]
        );
    }
}

// AC8

#[test]
fn ac8_an_ended_stream_with_a_cause_fails_the_turn_with_it() {
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    k.provider_ended_with(&t, Some("no-binary".into())).unwrap();
    assert_eq!(
        rec(&mut k, &id).turn,
        Turn::Idle {
            stop: Stop::Failed {
                error: "provider-ended".into(),
                cause: Some("no-binary".into())
            },
            seconds: Some(0),
            tools: None,
            tokens: None
        }
    );
    let es = entries(&mut k, &id);
    assert_eq!(
        es.last().unwrap()["body"],
        json!({"state":"idle","stop":{"kind":"failed","error":"provider-ended","cause":"no-binary"},"seconds":0})
    );
    // no cause: as before
    let mut k = kernel();
    let (id, t, _) = session(&mut k);
    k.provider_ended_with(&t, None).unwrap();
    assert_eq!(
        entries(&mut k, &id).last().unwrap()["body"],
        json!({"state":"idle","stop":{"kind":"failed","error":"provider-ended"},"seconds":0})
    );
}

// AC9

#[test]
fn ac9_the_page_gets_the_built_in_scratch_profile_on_first_run() {
    let mut k = kernel();
    k.state.delete("access/alice");
    k.start("page", "k1", "scratch", "{}", IN).unwrap();
    let (launch, _) = launches(&k).pop().unwrap();
    assert_eq!(
        launch,
        json!({"version":1,"profile":"scratch","workspace":"scratch","partial":true,"ask":true})
    );
    let refused = |r: Result<Called<String>, KernelError>| match r {
        Err(KernelError::Refused(rs)) => rs[0].field == "profile",
        _ => false,
    };
    assert!(refused(k.start("page", "k2", "other", "{}", IN)));
    assert!(refused(k.start("local/notes", "k3", "scratch", "{}", IN)));
    // an unreadable `access/page` allows none; a stored one without scratch no longer allows it
    k.state.put("access/page", b"x".to_vec()).unwrap();
    assert!(refused(k.start("page", "k4", "scratch", "{}", IN)));
    crate::fixture_access::put(&mut k.state, "access/page", br#"["p"]"#.to_vec()).unwrap();
    assert!(refused(k.start("page", "k5", "scratch", "{}", IN)));
    // a stored profile replaces the built-in
    k.state
        .put("profile/scratch", br#"{"model":"haiku"}"#.to_vec())
        .unwrap();
    crate::fixture_access::put(&mut k.state, "access/page", br#"["scratch"]"#.to_vec()).unwrap();
    k.start("page", "k6", "scratch", "{}", IN).unwrap();
    assert_eq!(launches(&k).pop().unwrap().0["model"], "haiku");
}

// AC10

#[test]
fn ac10_buffered_state_sees_its_writes_and_gives_them_up_on_discard() {
    let mut inner = MemState::new();
    inner.put("a", b"1".to_vec()).unwrap();
    inner.put("b", b"2".to_vec()).unwrap();
    let mut b = Buffered::new(inner);
    b.put("a", b"9".to_vec()).unwrap();
    b.put("c", b"3".to_vec()).unwrap();
    b.delete("b");
    assert_eq!(b.get("a"), Some(b"9".to_vec()));
    assert_eq!(b.get("b"), None);
    assert_eq!(b.get("c"), Some(b"3".to_vec()));
    assert!(b.put("big", vec![0; MAX_VALUE + 1]).is_err());
    let mut kept = b.discard();
    assert_eq!(kept.get("a"), Some(b"1".to_vec()));
    assert_eq!(kept.get("b"), Some(b"2".to_vec()));
    assert_eq!(kept.get("c"), None);

    let mut b = Buffered::new(kept);
    b.put("a", b"9".to_vec()).unwrap();
    b.delete("b");
    let mut done = b.commit();
    assert_eq!(done.get("a"), Some(b"9".to_vec()));
    assert_eq!(done.get("b"), None);
}

/// R1: a call that writes and then fails. A `start` that crosses the account's ceiling writes its
/// record, page and stream key before `finish` refuses `quota`: a state without the buffer keeps
/// them (the hole on watane's host, where only a trap rolls back), `Buffered::discard` drops them.
fn failing_start<S: State>(st: S) -> (S, KernelError) {
    let host = FakeHost {
        generation: 1,
        now: 1000,
        ..FakeHost::default()
    };
    let mut k = Kernel::new(st, host);
    k.state.put("profile/p", P.as_bytes().to_vec()).unwrap();
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p"]"#.to_vec()).unwrap();
    crate::fixture_access::put(&mut k.state, "access/bob", br#"["p"]"#.to_vec()).unwrap();
    k.start("alice", "k1", "p", "{}", IN).unwrap();
    let account = {
        let b = k.state.get("quota").unwrap();
        serde_json::from_slice::<Value>(&b).unwrap()["bytes"]
            .as_u64()
            .unwrap()
    };
    k.limits.account_bytes = account + 1;
    k.limits.reserve_bytes = 0;
    let e = k.start("bob", "k1", "p", "{}", IN).unwrap_err();
    (k.state.into_inner(), e)
}

#[test]
fn ac10_r1_a_call_that_fails_after_writing_leaves_the_buffered_state_unchanged() {
    // Without the buffer the failed call's writes stay.
    let (mut plain, e) = failing_start(MemState::new());
    assert!(matches!(e, KernelError::Quota(_)), "{e}");
    assert!(
        plain.get("session/s2").is_some(),
        "the failed start wrote its record"
    );

    // With it: the first start is committed, the failing one is discarded.
    let mut first = Buffered::new(MemState::new());
    first.put("seed", b"1".to_vec()).unwrap();
    let (st, e) = failing_start(first);
    assert!(matches!(e, KernelError::Quota(_)), "{e}");
    // Everything `failing_start` wrote, the first start's too, was buffered: the discard leaves
    // only what the state held before.
    let inner = st.discard();
    assert!(
        inner.is_empty(),
        "{:?}",
        inner.entries().map(|(k, _)| k).collect::<Vec<_>>()
    );
}

#[test]
fn ac10_r1_only_the_failed_call_is_discarded() {
    let host = || FakeHost {
        generation: 1,
        now: 1000,
        ..FakeHost::default()
    };
    let mut k = Kernel::new(Buffered::new(MemState::new()), host());
    k.state.put("profile/p", P.as_bytes().to_vec()).unwrap();
    crate::fixture_access::put(&mut k.state, "access/alice", br#"["p"]"#.to_vec()).unwrap();
    crate::fixture_access::put(&mut k.state, "access/bob", br#"["p"]"#.to_vec()).unwrap();
    k.start("alice", "k1", "p", "{}", IN).unwrap();
    // the wrapper commits a call that succeeded ...
    let mut committed = k.state.into_inner().commit();
    let before: Vec<(String, Vec<u8>)> = committed
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    let account = serde_json::from_slice::<Value>(&committed.get("quota").unwrap()).unwrap()
        ["bytes"]
        .as_u64()
        .unwrap();
    // ... and drops one that failed after writing
    let mut k = Kernel::new(Buffered::new(committed), host());
    k.host.next_id = 1;
    k.host.next_stream = 1;
    k.limits.account_bytes = account + 1;
    k.limits.reserve_bytes = 0;
    let e = k.start("bob", "k1", "p", "{}", IN).unwrap_err();
    assert!(matches!(e, KernelError::Quota(_)), "{e}");
    let after = k.state.into_inner().discard();
    let after: Vec<(String, Vec<u8>)> = after
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    assert_eq!(after, before);
}

// S35-fix1: every entry the kernel stores goes out live once, with its index (9.4)

const UNKNOWN: &str = r#"{"type":"made_up_by_case_5","x":1}"#;

fn indexes_on(k: &K, stream: &str) -> Vec<u64> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream: s, value } if s == stream => {
                let w = &value["_meta"]["watane"];
                if w["type"] == "entry" {
                    w["entry"]["i"].as_u64()
                } else {
                    w["i"].as_u64()
                }
            }
            _ => None,
        })
        .collect()
}

#[test]
fn ac1_every_stored_entry_goes_out_live_once_as_a_batch_and_per_line() {
    for batched in [true, false] {
        let mut k = kernel();
        let (id, t, o) = session(&mut k);
        let mut raw = fake_lines();
        raw.insert(raw.len().min(2), UNKNOWN.as_bytes().to_vec());
        let items: Vec<(u64, Vec<u8>)> = raw
            .into_iter()
            .enumerate()
            .map(|(i, l)| (i as u64 + 1, l))
            .collect();
        if batched {
            k.provider_lines(&t, items).unwrap();
        } else {
            for it in items {
                k.provider_lines(&t, vec![it]).unwrap();
            }
        }
        let read: Vec<u64> = entries(&mut k, &id)
            .iter()
            .map(|e| e["i"].as_u64().unwrap())
            .collect();
        assert_eq!(read, [1, 2, 3, 4, 5], "batched {batched}");
        let mut live = indexes_on(&k, &o);
        live.sort();
        assert_eq!(live, read, "batched {batched}: each once");
    }
}

#[test]
fn ac2_an_entry_without_an_acp_form_is_an_entry_item_and_the_turn_item_follows_it() {
    let mut k = kernel();
    let (_, t, o) = session(&mut k);
    let from = k.host.calls.len();
    let mut items: Vec<(u64, Vec<u8>)> = vec![(1, UNKNOWN.as_bytes().to_vec())];
    items.extend(
        fake_lines()
            .into_iter()
            .enumerate()
            .map(|(i, l)| (i as u64 + 2, l)),
    );
    k.provider_lines(&t, items).unwrap();
    let kinds: Vec<String> = k.host.calls[from..]
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream, value } if *stream == o => {
                let w = &value["_meta"]["watane"];
                Some(match value["sessionUpdate"].as_str() {
                    Some(u) => u.to_string(),
                    None if w["type"] == "entry" => format!("entry:{}", w["entry"]["kind"]),
                    None => w["type"].as_str().unwrap().to_string(),
                })
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "entry:\"unknown\"",
            "agent_message_chunk",
            "agent_message_chunk",
            "usage",
            "entry:\"turn\"",
            "turn"
        ]
    );
}

#[test]
fn ac3_entries_stored_outside_an_items_call_go_out_live() {
    let live_entries = |k: &K, o: &str, from: usize| -> Vec<String> {
        k.host.calls[from..]
            .iter()
            .filter_map(|c| match c {
                HostCall::Push { stream, value } if stream == o => {
                    let w = &value["_meta"]["watane"];
                    (w["type"] == "entry").then(|| w["entry"]["kind"].as_str().unwrap().to_string())
                }
                _ => None,
            })
            .collect()
    };
    // a cancel stores a turn (a say's user message as its entry item: sessions.rs,
    // `s35_fix1_an_ask_its_decision_and_a_say_go_out_as_entry_items`)
    let mut k = kernel();
    let (id, _, o) = session(&mut k);
    let from = k.host.calls.len();
    k.cancel("alice", &id).unwrap();
    assert_eq!(live_entries(&k, &o, from), ["turn"]);
    let last = rec(&mut k, &id).meta().last;
    assert_eq!(*indexes_on(&k, &o).iter().max().unwrap(), last);
    // interrupt
    let mut k = kernel();
    let (id, _, o) = session(&mut k);
    let from = k.host.calls.len();
    k.interrupt("alice", &id, None).unwrap();
    assert_eq!(live_entries(&k, &o, from), ["turn"]);
    // the provider's end
    let mut k = kernel();
    let (_, t, o) = session(&mut k);
    let from = k.host.calls.len();
    k.provider_ended_with(&t, Some("boom".into())).unwrap();
    assert_eq!(live_entries(&k, &o, from), ["turn"]);
}
