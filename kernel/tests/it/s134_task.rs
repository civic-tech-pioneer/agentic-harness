//! S134-4: an OpenAI root delegates with `task` (TECHNICAL 9.7, 9.5, 9.4, 14): admission, the child
//! record and its first turn, the `child` entry, exactly one terminal output, and the stops.

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const MORE: &str = r#"[{"type":"text","text":"more"}]"#;

fn tool(name: &str, parallel: bool) -> Tool {
    Tool {
        name: name.into(),
        description: format!("does {name}"),
        input_schema: r#"{"type":"object","properties":{}}"#.into(),
        parallel,
        idempotent: false,
    }
}

fn openai(extra: &str) -> String {
    format!(r#"{{"backend":"openai","model":"gpt-5","system-prompt":{{"replace":"x"}}{extra}}}"#)
}

/// Alice's root profile `root` (delegates `foreman` and `validator`, `task` allowed) with `extra`
/// fields; the children are plain OpenAI profiles of the same model; bob has the plain `o`.
fn kernel(root_extra: &str) -> K {
    let mut st = MemState::new();
    let root = openai(&format!(
        r#","delegates":["foreman","validator"],"allow":["task"]{root_extra}"#
    ));
    st.put("profile/root", root.into_bytes()).unwrap();
    st.put("profile/foreman", openai("").into_bytes()).unwrap();
    st.put("profile/validator", openai("").into_bytes())
        .unwrap();
    st.put(
        "profile/o",
        openai(r#","toolsets":["notes"],"allow":["notes_*"]"#).into_bytes(),
    )
    .unwrap();
    st.put("access/alice", br#"["root"]"#.to_vec()).unwrap();
    st.put("access/bob", br#"["o"]"#.to_vec()).unwrap();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        generation: 1,
        now: 1_000_000,
        tools: vec![tool("notes_a", true), tool("notes_b", false)],
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn lp(k: &mut K, id: &str) -> Value {
    serde_json::from_slice(&k.state.get(&format!("loop/{id}")).unwrap()).unwrap()
}

fn stream_of(k: &mut K, id: &str) -> String {
    lp(k, id)["stream"].as_str().unwrap().to_string()
}

fn bodies(k: &K) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::HttpsStart { request } => {
                let r: Value = serde_json::from_str(request).unwrap();
                Some(serde_json::from_str(r["body"].as_str().unwrap()).unwrap())
            }
            _ => None,
        })
        .collect()
}

fn sse_head() -> Vec<u8> {
    json!({"status": 200, "headers": {"content-type": "text/event-stream"}})
        .to_string()
        .into_bytes()
}

fn done(output: &[Value], usage: Option<(u64, u64)>) -> Vec<u8> {
    let mut r = json!({"id":"r","output":output});
    if let Some((i, o)) = usage {
        r["usage"] = json!({"input_tokens": i, "output_tokens": o});
    }
    json!({"type":"response.completed","response":r})
        .to_string()
        .into_bytes()
}

fn text(t: &str) -> Value {
    json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":t}]})
}

fn task(id: &str, profile: &str, input: &str) -> Value {
    json!({"type":"function_call","id":"fc","call_id":id,"name":"task",
        "arguments": json!({"profile": profile, "input": input}).to_string()})
}

fn last_seq(k: &mut K, s: &str) -> u64 {
    k.state
        .get(&format!("stream/{s}"))
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["last"].as_u64())
        .unwrap_or(0)
}

/// Delivers the head and a completed response on the session's current request, without ending
/// the stream.
fn deliver(k: &mut K, id: &str, output: &[Value], usage: Option<(u64, u64)>) -> String {
    let s = stream_of(k, id);
    let n = last_seq(k, &s);
    k.provider_lines(&s, vec![(n + 1, sse_head()), (n + 2, done(output, usage))])
        .unwrap();
    s
}

fn reply(k: &mut K, id: &str, output: &[Value]) {
    let s = deliver(k, id, output, Some((10, 5)));
    k.provider_ended_with(&s, None).unwrap();
}

fn start_root(k: &mut K) -> String {
    k.start("alice", "k", "root", "{}", IN).unwrap().value
}

fn children(k: &mut K, root: &str) -> Vec<String> {
    rec(k, root).children
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn kind_entries(k: &mut K, id: &str, kind: &str) -> Vec<Value> {
    entries(k, id)
        .into_iter()
        .filter(|e| e["kind"] == kind)
        .collect()
}

/// `(call id, output)` of the function outputs a request carries.
fn outputs(req: &Value) -> Vec<(String, String)> {
    req["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["type"] == "function_call_output")
        .map(|i| {
            (
                i["call_id"].as_str().unwrap().to_string(),
                i["output"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn output_of(req: &Value, call: &str) -> Value {
    let o = outputs(req)
        .into_iter()
        .find(|(c, _)| c == call)
        .unwrap_or_else(|| panic!("no output for {call}: {:?}", outputs(req)))
        .1;
    serde_json::from_str(&o).unwrap_or(Value::String(o))
}

fn stop_of(k: &mut K, id: &str) -> Stop {
    match rec(k, id).turn {
        Turn::Idle { stop, .. } => stop,
        t => panic!("not idle: {t:?}"),
    }
}

/// A root that asks two children for work: the root's id and the two child ids.
fn root_with_two(k: &mut K) -> (String, String, String) {
    let root = start_root(k);
    reply(
        k,
        &root,
        &[
            task("c1", "foreman", "do A"),
            task("c2", "validator", "do B"),
        ],
    );
    let kids = children(k, &root);
    assert_eq!(kids.len(), 2, "{kids:?}");
    (root, kids[0].clone(), kids[1].clone())
}

fn pushed_entries(k: &K, stream: &str, kind: &str) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream: s, value } if s == stream => {
                let e = &value["_meta"]["watane"]["entry"];
                (e["kind"] == kind).then(|| e.clone())
            }
            _ => None,
        })
        .collect()
}

// ---- AC1: the request advertises `task`

#[test]
fn ac1_a_root_with_delegates_advertises_task_last() {
    let mut k = kernel("");
    let _ = start_root(&mut k);
    let b = &bodies(&k)[0];
    let tools = b["tools"].as_array().unwrap();
    let t = tools.last().unwrap();
    assert_eq!(t["name"], "task");
    assert_eq!(t["type"], "function");
    assert_eq!(t["parameters"]["type"], "object");
    assert_eq!(t["parameters"]["required"], json!(["profile", "input"]));
    let props = t["parameters"]["properties"].as_object().unwrap();
    assert_eq!(props.len(), 2);
    assert_eq!(props["profile"]["type"], "string");
    assert_eq!(props["input"]["type"], "string");
}

#[test]
fn ac1_the_toolsets_tools_come_before_task() {
    let mut k = kernel("");
    k.state
        .put(
            "profile/root",
            openai(r#","delegates":["foreman"],"allow":["task","notes_*"],"toolsets":["notes"]"#)
                .into_bytes(),
        )
        .unwrap();
    let _ = start_root(&mut k);
    let names: Vec<String> = bodies(&k)[0]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["notes_a", "notes_b", "task"]);
}

#[test]
fn ac1_no_delegates_a_child_and_a_summary_send_no_task() {
    // No delegates.
    let mut k = kernel("");
    k.state
        .put("access/alice", br#"["root","foreman"]"#.to_vec())
        .unwrap();
    let plain = k.start("alice", "p", "foreman", "{}", IN).unwrap().value;
    assert!(bodies(&k)[0].get("tools").is_none(), "{}", bodies(&k)[0]);
    reply(&mut k, &plain, &[text("x")]);
    // A child.
    let (_, kid, _) = root_with_two(&mut k);
    let reqs = bodies(&k);
    assert_eq!(reqs.len(), 4, "plain, root, two children");
    assert!(reqs[2].get("tools").is_none(), "{}", reqs[2]);
    let _ = kid;
    // A summary of a root.
    let mut k = kernel(r#","max-turns":1"#);
    let root = start_root(&mut k);
    assert!(bodies(&k)[0]["tools"].is_array());
    reply(&mut k, &root, &[task("c1", "foreman", "do")]);
    let reqs = bodies(&k);
    assert!(reqs
        .iter()
        .all(|r| r.get("tools").is_none() || r["input"].as_array().unwrap().len() == 1));
    assert!(
        children(&mut k, &root).is_empty(),
        "the capped step starts no child"
    );
    assert!(
        reqs.last().unwrap().get("tools").is_none(),
        "summary is tool-free"
    );
}

// ---- AC2: two parallel calls start two children

#[test]
fn ac2_two_calls_start_two_children_with_entries_and_open_calls() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    let AttachedLive(stream) = attach_live(&mut k, &root);
    reply(
        &mut k,
        &root,
        &[
            task("c1", "foreman", "do A"),
            task("c2", "validator", "do B"),
        ],
    );
    let kids = children(&mut k, &root);
    assert_eq!(kids.len(), 2);
    for (kid, profile, call) in [(&kids[0], "foreman", "c1"), (&kids[1], "validator", "c2")] {
        let r = rec(&mut k, kid);
        assert_eq!(r.owner, "alice");
        assert_eq!(r.parent.as_deref(), Some(root.as_str()));
        assert_eq!(r.backend, "openai");
        assert_eq!(r.profile, profile);
        assert!(r.workspace.is_none() && r.handle.is_none() && r.folder.is_none());
        assert!(r.requested_folder.is_none());
        assert_eq!(r.task_call.as_deref(), Some(call));
        assert!(matches!(r.turn, Turn::Running { .. }));
    }
    // The root waits in `tools` with both calls open as `child` states.
    let l = lp(&mut k, &root);
    assert_eq!(l["state"], "tools");
    let calls = l["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["st"], "child");
    assert_eq!(calls[0]["session"], kids[0].as_str());
    assert_eq!(calls[1]["session"], kids[1].as_str());
    // Two `child` entries, indexed, in the transcript and pushed live.
    let ce = kind_entries(&mut k, &root, "child");
    assert_eq!(ce.len(), 2);
    assert_eq!(
        ce[0]["body"],
        json!({"session": kids[0], "toolCallId": "c1"})
    );
    assert_eq!(
        ce[1]["body"],
        json!({"session": kids[1], "toolCallId": "c2"})
    );
    assert!(ce[0]["i"].as_u64().unwrap() < ce[1]["i"].as_u64().unwrap());
    let live = pushed_entries(&k, &stream, "child");
    assert_eq!(live.len(), 2);
    assert_eq!(live[0]["body"]["session"], kids[0].as_str());
    // Each child's first turn runs the call's input as its user message.
    let reqs = bodies(&k);
    assert_eq!(reqs.len(), 3);
    let first = |r: &Value| {
        r["input"][0]["content"][0]["text"]
            .as_str()
            .map(str::to_string)
    };
    assert_eq!(first(&reqs[1]).as_deref(), Some("do A"));
    assert_eq!(first(&reqs[2]).as_deref(), Some("do B"));
    assert_eq!(reqs[1]["prompt_cache_key"], kids[0].as_str());
    let msgs = kind_entries(&mut k, &kids[0], "message");
    assert_eq!(msgs[0]["body"]["role"], "user");
    assert_eq!(msgs[0]["body"]["content"][0]["text"], "do A");
    // The owner reads and attaches the child; another owner cannot.
    assert!(k.read("alice", &kids[0], 0, None, 10).is_ok());
    assert!(k.attach("alice", &kids[1], 0).is_ok());
    assert_eq!(
        k.read("bob", &kids[0], 0, None, 10).unwrap_err(),
        KernelError::NotOwner
    );
    // Replayed on attach.
    let AttachedLive(s2) = attach_live(&mut k, &root);
    assert_eq!(pushed_entries(&k, &s2, "child").len(), 2);
}

/// The owner stream of `attach`, as a tuple struct so the destructuring reads well.
struct AttachedLive(String);

fn attach_live(k: &mut K, id: &str) -> AttachedLive {
    match k.attach("alice", id, 0).unwrap().value {
        Attached::Live { stream } => AttachedLive(stream),
        a => panic!("not live: {a:?}"),
    }
}

// ---- AC3: the child's final text is the root's output

#[test]
fn ac3_a_completed_child_resolves_its_call_and_the_root_goes_on() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    reply(&mut k, &a, &[text("A done")]);
    // One call is still open: the root sends nothing yet.
    assert_eq!(bodies(&k).len(), 3);
    assert_eq!(lp(&mut k, &root)["calls"].as_array().unwrap().len(), 1);
    assert_eq!(rec(&mut k, &root).children, std::slice::from_ref(&b));
    assert_eq!(rec(&mut k, &a).task_call, None);
    assert_eq!(stop_of(&mut k, &a), Stop::Completed);
    reply(&mut k, &b, &[text("B done")]);
    let reqs = bodies(&k);
    assert_eq!(reqs.len(), 4, "the root's follow-up");
    assert_eq!(reqs[3]["prompt_cache_key"], root.as_str());
    assert_eq!(
        outputs(&reqs[3]),
        [
            (
                "c1".to_string(),
                json!({"ok":{"session": a, "message": "A done"}}).to_string()
            ),
            (
                "c2".to_string(),
                json!({"ok":{"session": b, "message": "B done"}}).to_string()
            ),
        ]
    );
    assert!(children(&mut k, &root).is_empty());
    reply(&mut k, &root, &[text("all done")]);
    assert_eq!(stop_of(&mut k, &root), Stop::Completed);
    // Shown as the tool entries.
    let tools = kind_entries(&mut k, &root, "tool");
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().all(|t| t["body"]["status"] == "completed"));
    assert_eq!(tools[0]["body"]["title"], "task");
}

#[test]
fn ac3_text_over_64_kib_is_cut_inside_the_message() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    reply(&mut k, &root, &[task("c1", "foreman", "go")]);
    let kid = children(&mut k, &root)[0].clone();
    let big = "ü\"x".repeat(40_000);
    reply(&mut k, &kid, &[text(&big)]);
    let out = outputs(bodies(&k).last().unwrap())[0].1.clone();
    assert!(out.len() <= 64 * 1024, "{}", out.len());
    let v: Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["ok"]["session"], kid.as_str());
    let m = v["ok"]["message"].as_str().unwrap();
    assert!(
        m.starts_with("ü\"xü\"x") && m.contains("[cut by watane"),
        "{}",
        &m[m.len() - 40..]
    );
}

// ---- AC4: how a child's first turn ends

fn one_child(root_extra: &str, child_extra: &str) -> (K, String, String) {
    let mut k = kernel(root_extra);
    k.state
        .put("profile/foreman", openai(child_extra).into_bytes())
        .unwrap();
    let root = start_root(&mut k);
    reply(&mut k, &root, &[task("c1", "foreman", "go")]);
    let kid = children(&mut k, &root)[0].clone();
    (k, root, kid)
}

fn root_output(k: &mut K) -> Value {
    let reqs = bodies(k);
    let last = reqs.last().unwrap();
    assert!(last["prompt_cache_key"].is_string());
    output_of(last, "c1")
}

#[test]
fn ac4_a_limit_stop_gives_the_limit_as_the_code() {
    let (mut k, root, kid) = one_child(r#","max-tokens":1000"#, r#","max-tokens":10"#);
    reply(&mut k, &kid, &[text("big")]);
    assert_eq!(
        root_output(&mut k),
        json!({"err":{"code":"max-tokens","session":kid}})
    );
    assert!(!rec(&mut k, &root).children.contains(&kid));
}

#[test]
fn ac4_a_failed_child_gives_backend() {
    let (mut k, _root, kid) = one_child("", "");
    let s = stream_of(&mut k, &kid);
    let n = last_seq(&mut k, &s);
    let head = json!({"status": 400, "headers": {"content-type": "application/json"}});
    k.provider_lines(&s, vec![(n + 1, head.to_string().into_bytes())])
        .unwrap();
    k.provider_ended_with(&s, None).unwrap();
    assert!(matches!(stop_of(&mut k, &kid), Stop::Failed { .. }));
    assert_eq!(
        root_output(&mut k),
        json!({"err":{"code":"backend","session":kid}})
    );
}

#[test]
fn ac4_an_empty_completion_gives_no_result() {
    let (mut k, _root, kid) = one_child("", "");
    reply(&mut k, &kid, &[]);
    assert_eq!(
        root_output(&mut k),
        json!({"err":{"code":"no-result","session":kid}})
    );
}

#[test]
fn ac4_the_owner_interrupting_the_first_turn_gives_interrupted() {
    let (mut k, root, kid) = one_child("", "");
    k.interrupt("alice", &kid, None).unwrap();
    assert_eq!(stop_of(&mut k, &kid), Stop::Interrupted);
    assert_eq!(
        root_output(&mut k),
        json!({"err":{"code":"interrupted","session":kid}})
    );
    assert!(children(&mut k, &root).is_empty());
}

#[test]
fn ac4_a_second_turn_of_the_child_writes_no_root_output() {
    let (mut k, root, kid) = one_child("", "");
    reply(&mut k, &kid, &[text("first")]);
    reply(&mut k, &root, &[task("c2", "foreman", "again")]);
    let n = bodies(&k).len();
    let kid2 = children(&mut k, &root)[0].clone();
    assert_ne!(kid, kid2);
    k.say("alice", &kid, "s1", MORE).unwrap();
    assert_eq!(bodies(&k).len(), n + 1, "the child's second turn runs");
    reply(&mut k, &kid, &[text("second")]);
    assert_eq!(stop_of(&mut k, &kid), Stop::Completed);
    // The second child's call is still open: the second turn of the first resolved nothing.
    assert_eq!(lp(&mut k, &root)["calls"].as_array().unwrap().len(), 1);
    assert_eq!(rec(&mut k, &kid2).task_call.as_deref(), Some("c2"));
    assert_eq!(bodies(&k).len(), n + 1);
}

#[test]
fn ac4_a_child_request_that_stops_busy_gives_busy() {
    let mut k = kernel(r#","toolsets":["notes"],"allow":["task","notes_*"]"#);
    k.state
        .put(
            "profile/foreman",
            openai(r#","toolsets":["notes"],"allow":["notes_*"]"#).into_bytes(),
        )
        .unwrap();
    k.state
        .put(
            "profile/root",
            openai(r#","toolsets":["notes"],"delegates":["foreman"],"allow":["task","notes_*"]"#)
                .into_bytes(),
        )
        .unwrap();
    k.state
        .put("access/alice", br#"["root","o"]"#.to_vec())
        .unwrap();
    k.limits.owner_request = 3;
    let root = start_root(&mut k);
    reply(&mut k, &root, &[task("c1", "foreman", "go")]);
    let kid = children(&mut k, &root)[0].clone();
    // The child calls a tool; its request stream is held while two other sessions fill alice's
    // request slots.
    let s = deliver(
        &mut k,
        &kid,
        &[
            json!({"type":"function_call","id":"f","call_id":"t1","name":"notes_a","arguments":"{}"}),
        ],
        Some((10, 5)),
    );
    k.start("alice", "o1", "o", "{}", IN).unwrap();
    k.provider_ended_with(&s, None).unwrap();
    k.start("alice", "o2", "o", "{}", IN).unwrap();
    k.start("alice", "o3", "o", "{}", IN).unwrap();
    // The child's job ends; its next request finds no slot: its turn stops busy.
    let job = lp(&mut k, &kid)["calls"][0]["stream"]
        .as_str()
        .unwrap()
        .to_string();
    k.provider_lines(&job, vec![(1, br#"{"ok":1}"#.to_vec())])
        .unwrap();
    k.provider_ended_with(&job, None).unwrap();
    assert!(matches!(stop_of(&mut k, &kid), Stop::Stopped { limit } if limit == "busy"));
    // The root's follow-up finds no slot either: its turn stops busy, the output is stored.
    assert!(matches!(stop_of(&mut k, &root), Stop::Stopped { limit } if limit == "busy"));
    let tools = kind_entries(&mut k, &root, "tool");
    let shown = tools[0]["body"]["content"][0]["content"]["text"]
        .as_str()
        .unwrap();
    let out: Value = serde_json::from_str(shown).unwrap();
    assert_eq!(out, json!({"err":{"code":"busy","session":kid}}));
}

// ---- AC5: refused dispatch

fn first_refusals(out: &Value) -> Vec<String> {
    assert_eq!(out["err"]["code"], "refused", "{out}");
    out["err"]["refusals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["field"].as_str().unwrap().to_string())
        .collect()
}

/// The root asks for `calls` in one response; returns the next request's body.
fn refused_for(k: &mut K, calls: &[Value]) -> (String, Value) {
    let root = start_root(k);
    reply(k, &root, calls);
    let b = bodies(k).last().unwrap().clone();
    (root, b)
}

#[test]
fn ac5_a_name_outside_delegates_and_a_missing_profile_are_refused_on_profile() {
    let mut k = kernel("");
    let ids = k.host.next_id;
    let (root, b) = refused_for(
        &mut k,
        &[task("c1", "scratch", "x"), task("c2", "ghost", "x")],
    );
    k.host.next_id = ids + 1;
    assert_eq!(first_refusals(&output_of(&b, "c1")), ["profile"]);
    assert_eq!(first_refusals(&output_of(&b, "c2")), ["profile"]);
    assert!(children(&mut k, &root).is_empty());
    assert!(kind_entries(&mut k, &root, "child").is_empty());
    // Only the root took an id.
    assert_eq!(k.list("alice", None, 10).unwrap().len(), 1);
    let mut k2 = kernel("");
    k2.state.delete("profile/validator");
    let (_, b) = refused_for(&mut k2, &[task("c1", "validator", "x")]);
    assert_eq!(first_refusals(&output_of(&b, "c1")), ["profile"]);
}

#[test]
fn ac5_a_third_call_at_the_live_cap_is_refused() {
    let mut k = kernel(r#","subagent-limits":{"live":2}"#);
    let (root, b) = {
        let root = start_root(&mut k);
        reply(
            &mut k,
            &root,
            &[
                task("c1", "foreman", "a"),
                task("c2", "foreman", "b"),
                task("c3", "foreman", "c"),
            ],
        );
        (root.clone(), bodies(&k).last().unwrap().clone())
    };
    assert_eq!(children(&mut k, &root).len(), 2);
    let _ = b;
    // The refusal is the third call's output, held until the others end.
    let kids = children(&mut k, &root);
    reply(&mut k, &kids[0], &[text("1")]);
    reply(&mut k, &kids[1], &[text("2")]);
    let last = bodies(&k).last().unwrap().clone();
    assert_eq!(
        first_refusals(&output_of(&last, "c3")),
        ["subagent-limits.live"]
    );
    assert_eq!(output_of(&last, "c1")["ok"]["message"], "1");
}

#[test]
fn ac5_a_widening_candidate_is_refused_on_its_field() {
    let mut k = kernel("");
    k.state
        .put(
            "profile/foreman",
            openai(r#","allow":["task","Bash"]"#).into_bytes(),
        )
        .unwrap();
    let (root, b) = refused_for(&mut k, &[task("c1", "foreman", "x")]);
    assert_eq!(first_refusals(&output_of(&b, "c1")), ["allow"]);
    assert!(children(&mut k, &root).is_empty());
}

#[test]
fn ac5_a_task_call_of_a_child_is_refused_on_depth() {
    let mut k = kernel("");
    k.state
        .put(
            "profile/foreman",
            openai(r#","allow":["task"]"#).into_bytes(),
        )
        .unwrap();
    let root = start_root(&mut k);
    reply(&mut k, &root, &[task("c1", "foreman", "go")]);
    let kid = children(&mut k, &root)[0].clone();
    let ids = k.host.next_id;
    reply(&mut k, &kid, &[task("g1", "validator", "deeper")]);
    assert_eq!(k.host.next_id, ids, "no id taken");
    let last = bodies(&k).last().unwrap().clone();
    assert_eq!(last["prompt_cache_key"], kid.as_str());
    assert_eq!(
        first_refusals(&output_of(&last, "g1")),
        ["subagent-limits.depth"]
    );
    assert!(children(&mut k, &kid).is_empty());
}

// ---- AC6: revoking

#[test]
fn ac6_a_task_after_revoking_the_roots_profile_is_refused_and_the_turn_goes_on() {
    let mut k = kernel("");
    let (root, a, _b) = root_with_two(&mut k);
    k.state.put("access/alice", b"[]".to_vec()).unwrap();
    // Reads still work.
    assert!(k.read("alice", &root, 0, None, 5).is_ok());
    assert!(k.read("alice", &a, 0, None, 5).is_ok());
    assert!(k.attach("alice", &a, 0).is_ok());
    assert_eq!(k.list("alice", None, 10).unwrap().len(), 3);
    // A child ends; `say` on it is refused on `profile`.
    reply(&mut k, &a, &[text("A")]);
    let err = k.say("alice", &a, "s", MORE).unwrap_err();
    assert!(
        matches!(&err, KernelError::Refused(r) if r[0].field == "profile"),
        "{err:?}"
    );
}

#[test]
fn ac6_the_task_call_after_revocation_gets_the_profile_refusal() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    k.state.put("access/alice", b"[]".to_vec()).unwrap();
    reply(&mut k, &root, &[task("c1", "foreman", "x")]);
    let b = bodies(&k).last().unwrap().clone();
    assert_eq!(first_refusals(&output_of(&b, "c1")), ["profile"]);
    assert!(children(&mut k, &root).is_empty());
    assert_eq!(lp(&mut k, &root)["state"], "requesting", "the turn goes on");
}

#[test]
fn ac6_a_child_whose_root_is_missing_is_refused_for_say() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    reply(&mut k, &root, &[task("c1", "foreman", "x")]);
    let kid = children(&mut k, &root)[0].clone();
    reply(&mut k, &kid, &[text("done")]);
    k.delete("alice", &root).unwrap();
    let err = k.say("alice", &kid, "s", MORE).unwrap_err();
    assert!(
        matches!(&err, KernelError::Refused(r) if r[0].field == "profile"),
        "{err:?}"
    );
    assert!(k.read("alice", &kid, 0, None, 5).is_ok());
}

#[test]
fn ac6_a_child_says_on_the_roots_access_not_its_own() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    reply(&mut k, &root, &[task("c1", "foreman", "x")]);
    let kid = children(&mut k, &root)[0].clone();
    reply(&mut k, &kid, &[text("done")]);
    // `foreman` is on no access list, yet the owner may continue the child.
    k.say("alice", &kid, "s", MORE).unwrap();
}

// ---- AC7: busy

#[test]
fn ac7_a_call_at_the_request_cap_gets_busy_and_no_child() {
    let mut k = kernel("");
    k.limits.owner_request = 2;
    let ids = k.host.next_id;
    let (root, b) = refused_for(
        &mut k,
        &[task("c1", "foreman", "a"), task("c2", "validator", "b")],
    );
    let _ = (b, ids);
    // The root holds one request until its stream ends: one child fits, the second is busy.
    assert_eq!(children(&mut k, &root).len(), 1);
    let kid = children(&mut k, &root)[0].clone();
    reply(&mut k, &kid, &[text("a")]);
    let last = bodies(&k).last().unwrap().clone();
    assert_eq!(output_of(&last, "c2"), json!({"err":{"code":"busy"}}));
    assert_eq!(output_of(&last, "c1")["ok"]["message"], "a");
}

#[test]
fn ac7_a_waiting_task_holds_no_slot_and_the_association_goes_once() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    let sl = Slots::load(&mut k.state);
    assert_eq!(sl.job, 0);
    assert_eq!(sl.request, 2, "the two children's requests only");
    reply(&mut k, &a, &[text("A")]);
    assert_eq!(rec(&mut k, &root).children, std::slice::from_ref(&b));
    // A second turn of the finished child resolves nothing more.
    k.say("alice", &a, "s", MORE).unwrap();
    reply(&mut k, &a, &[text("again")]);
    assert_eq!(rec(&mut k, &root).children, [b]);
    assert_eq!(lp(&mut k, &root)["calls"].as_array().unwrap().len(), 1);
}

#[test]
fn ac7_a_task_in_a_call_that_may_be_another_owners_is_busy() {
    let mut k = kernel(r#","toolsets":["notes"],"allow":["task","notes_*"]"#);
    k.limits.job = 1;
    // Bob holds the one job slot.
    let bob = k.start("bob", "b", "o", "{}", IN).unwrap().value;
    let call = |id: &str, name: &str| json!({"type":"function_call","id":"f","call_id":id,"name":name,"arguments":"{}"});
    reply(&mut k, &bob, &[call("b1", "notes_a")]);
    // Alice's serial step: a job that waits for a slot, then a `task`.
    let root = start_root(&mut k);
    reply(
        &mut k,
        &root,
        &[call("j1", "notes_b"), task("c1", "foreman", "x")],
    );
    assert!(lp(&mut k, &root)["calls"][0]["st"] == "pending");
    // Bob's job ends; the FIFO starts alice's job in bob's call, so its item is not alice's origin.
    let bj = lp(&mut k, &bob)["calls"][0]["stream"]
        .as_str()
        .unwrap()
        .to_string();
    k.provider_lines(&bj, vec![(1, br#"{"ok":1}"#.to_vec())])
        .unwrap();
    k.provider_ended_with(&bj, None).unwrap();
    let aj = lp(&mut k, &root)["calls"][0]["stream"]
        .as_str()
        .unwrap()
        .to_string();
    k.provider_lines(&aj, vec![(1, br#"{"ok":2}"#.to_vec())])
        .unwrap();
    k.provider_ended_with(&aj, None).unwrap();
    assert!(
        children(&mut k, &root).is_empty(),
        "no child in another origin's call"
    );
}

// ---- AC8: the broker's rules for `task`

#[test]
fn ac8_a_deny_rule_for_task_gives_the_profile_denial() {
    let mut k = kernel(r#","deny":["task"]"#);
    let (root, b) = refused_for(&mut k, &[task("c1", "foreman", "x")]);
    assert_eq!(
        output_of(&b, "c1"),
        json!({"error": "denied by the profile"})
    );
    assert!(children(&mut k, &root).is_empty());
}

#[test]
fn ac8_manual_mode_asks_and_the_child_starts_after_the_allow() {
    let mut k = kernel("");
    k.state
        .put(
            "profile/root",
            openai(r#","delegates":["foreman"],"mode":"manual""#).into_bytes(),
        )
        .unwrap();
    k.state
        .put(
            "profile/foreman",
            openai(r#","mode":"manual""#).into_bytes(),
        )
        .unwrap();
    let root = start_root(&mut k);
    let AttachedLive(_) = attach_live(&mut k, &root);
    reply(&mut k, &root, &[task("c1", "foreman", "x")]);
    assert!(children(&mut k, &root).is_empty());
    assert!(matches!(rec(&mut k, &root).turn, Turn::Waiting { .. }));
    assert_eq!(bodies(&k).len(), 1);
    k.answer("alice", &root, "a1", "c1", r#"{"allow":true}"#)
        .unwrap();
    assert_eq!(children(&mut k, &root).len(), 1);
    assert_eq!(bodies(&k).len(), 2, "the child's first request");
}

// ---- AC9: a toolset function named like a built-in

#[test]
fn ac9_a_toolset_function_named_task_fails_a_delegating_root_before_any_request() {
    let failed = |k: &mut K, id: &str| match rec(k, id).turn {
        Turn::Idle {
            stop: Stop::Failed { error, .. },
            ..
        } => assert!(
            error.contains("toolsets") && error.contains("task"),
            "{error}"
        ),
        t => panic!("{t:?}"),
    };
    // A root that names delegates advertises `task`: a toolset function of that name collides.
    let mut k = kernel(r#","toolsets":["notes"]"#);
    k.host.tools = vec![tool("notes_a", true), tool("task", true)];
    let id = start_root(&mut k);
    assert!(bodies(&k).is_empty(), "no request");
    failed(&mut k, &id);
    // S134 r1 F1.7: only active built-ins collide. `o` names no delegates, so `task` is free
    // there, and `skill` is free where the profile has no skills.
    for name in ["task", "skill"] {
        let mut k = kernel("");
        k.host.tools = vec![tool("notes_a", true), tool(name, true)];
        let id = k.start("bob", "b", "o", "{}", IN).unwrap().value;
        assert_eq!(bodies(&k).len(), 1, "{name}: the request is sent");
        assert!(rec(&mut k, &id).turn.is_active(), "{name}");
    }
    let mut k = kernel(r#","toolsets":["notes"]"#);
    k.host.tools = vec![tool("notes_a", true), tool("skill", true)];
    start_root(&mut k);
    assert_eq!(bodies(&k).len(), 1, "skill is no active built-in");
}

/// S134 r1 F1.7: a description kept from an earlier request is checked as a fresh one is.
#[test]
fn r1_f1_7_a_kept_description_naming_task_fails_the_next_request() {
    let mut k = kernel(r#","toolsets":["notes"]"#);
    let id = start_root(&mut k);
    reply(&mut k, &id, &[]);
    assert!(!rec(&mut k, &id).turn.is_active());
    let key = format!("loop/{id}");
    let mut v: Value = serde_json::from_slice(&k.state.get(&key).unwrap()).unwrap();
    v["tools"]["info"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "task", "parallel": true, "idempotent": false}));
    k.state.put(&key, serde_json::to_vec(&v).unwrap()).unwrap();
    let sent = bodies(&k).len();
    k.say("alice", &id, "s2", "again").unwrap();
    assert_eq!(bodies(&k).len(), sent, "no request");
    match rec(&mut k, &id).turn {
        Turn::Idle {
            stop: Stop::Failed { error, .. },
            ..
        } => assert!(error.contains("toolsets"), "{error}"),
        t => panic!("{t:?}"),
    }
}

// ---- AC10: the `child` entry

#[test]
fn ac10_the_child_entry_has_the_documented_shape_live_and_stored() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    let AttachedLive(stream) = attach_live(&mut k, &root);
    reply(&mut k, &root, &[task("c1", "foreman", "x")]);
    let kid = children(&mut k, &root)[0].clone();
    let live = pushed_entries(&k, &stream, "child");
    assert_eq!(live.len(), 1);
    assert_eq!(live[0]["kind"], "child");
    assert_eq!(live[0]["body"], json!({"session": kid, "toolCallId": "c1"}));
    assert!(live[0]["i"].as_u64().is_some());
    let stored = kind_entries(&mut k, &root, "child");
    assert_eq!(stored[0]["body"], live[0]["body"]);
    assert_eq!(stored[0]["i"], live[0]["i"]);
}

// ---- restart and the accounts

#[test]
fn a_child_of_an_older_boot_cannot_complete_a_pre_restart_call() {
    for step_root_first in [false, true] {
        let mut k = kernel("");
        let (root, a, _b) = root_with_two(&mut k);
        k.host.generation = 2;
        if step_root_first {
            // R6: reading the root runs its restart step: the open calls get `aborted`.
            let _ = k.read("alice", &root, 0, None, 5).unwrap();
            assert!(lp(&mut k, &root).get("calls").is_none());
        }
        // The owner continues the child; it completes its turn.
        k.say("alice", &a, "s", MORE).unwrap();
        reply(&mut k, &a, &[text("late")]);
        assert_eq!(stop_of(&mut k, &a), Stop::Completed);
        assert_eq!(
            rec(&mut k, &a).task_call,
            None,
            "the association is consumed"
        );
        let reqs = bodies(&k);
        assert!(
            reqs.iter()
                .all(|r| !outputs(r).iter().any(|(_, o)| o.contains("late"))),
            "no late output reached the root"
        );
        // The root has no output of the child's: the call was aborted or is still stale.
        let tools = kind_entries(&mut k, &root, "tool");
        assert!(tools.iter().all(|t| {
            let shown = t["body"]["content"][0]["content"]["text"]
                .as_str()
                .unwrap_or("");
            !shown.contains("late")
        }));
    }
}

#[test]
fn the_bytes_of_root_and_children_are_charged_exactly() {
    let used = |k: &mut K| Usage::load(&mut k.state, &Usage::owner_key("alice"));
    // What one plain session leaves behind once deleted (its kept start key, its page).
    let mut base = kernel("");
    let id = start_root(&mut base);
    reply(&mut base, &id, &[text("x")]);
    base.delete("alice", &id).unwrap();
    let left = used(&mut base).bytes;

    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    reply(&mut k, &a, &[text("A")]);
    reply(&mut k, &b, &[text("B")]);
    reply(&mut k, &root, &[text("done")]);
    assert!(used(&mut k).bytes > left);
    assert_eq!(used(&mut k).sessions, 3);
    for id in [&a, &b, &root] {
        k.delete("alice", id).unwrap();
    }
    let u = used(&mut k);
    assert_eq!(u.sessions, 0);
    assert_eq!(
        u.bytes, left,
        "every byte of the children was charged to their records"
    );
}

// ---- the root's pushes go on the root's own stream

#[test]
fn a_resolution_pushes_the_roots_output_on_the_roots_stream() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    let AttachedLive(rs) = attach_live(&mut k, &root);
    reply(&mut k, &root, &[task("c1", "foreman", "go")]);
    let kid = children(&mut k, &root)[0].clone();
    let AttachedLive(ks) = attach_live(&mut k, &kid);
    let from = k.host.calls.len();
    reply(&mut k, &kid, &[text("fin")]);
    let pushed = |k: &K, s: &str| -> Vec<Value> {
        k.host.calls[from..]
            .iter()
            .filter_map(|c| match c {
                HostCall::Push { stream, value } if stream == s => Some(value.clone()),
                _ => None,
            })
            .collect()
    };
    let on_root = pushed(&k, &rs);
    let on_kid = pushed(&k, &ks);
    let has_tool_update = |v: &[Value]| {
        v.iter()
            .any(|p| p["sessionUpdate"] == "tool_call_update" && p["toolCallId"] == "c1")
    };
    assert!(has_tool_update(&on_root), "{on_root:?}");
    assert!(
        !has_tool_update(&on_kid),
        "the child's stream gets no root update"
    );
    assert!(
        on_kid
            .iter()
            .any(|p| p["_meta"]["watane"]["type"] == "turn"),
        "{on_kid:?}"
    );
}

#[test]
fn a_full_root_stream_defers_the_whole_resolution() {
    let mut k = kernel("");
    let root = start_root(&mut k);
    let AttachedLive(rs) = attach_live(&mut k, &root);
    reply(&mut k, &root, &[task("c1", "foreman", "go")]);
    let kid = children(&mut k, &root)[0].clone();
    k.host.full.insert(rs.clone());
    let s = stream_of(&mut k, &kid);
    let n = last_seq(&mut k, &s);
    let out = k
        .provider_lines(
            &s,
            vec![
                (n + 1, sse_head()),
                (n + 2, done(&[text("x")], Some((1, 1)))),
            ],
        )
        .unwrap();
    assert_eq!(out.value, SinkOutcome::Defer);
    assert!(
        matches!(rec(&mut k, &kid).turn, Turn::Running { .. }),
        "nothing was kept"
    );
    assert_eq!(rec(&mut k, &kid).task_call.as_deref(), Some("c1"));
    k.host.full.remove(&rs);
    let out = k
        .provider_lines(
            &s,
            vec![
                (n + 1, sse_head()),
                (n + 2, done(&[text("x")], Some((1, 1)))),
            ],
        )
        .unwrap();
    assert_eq!(out.value, SinkOutcome::Taken);
    assert_eq!(rec(&mut k, &kid).task_call, None);
}

/// S134 r1 F1.1: a root that stops at a limit while a child is live stops that child too (the
/// interrupt cleanup of 9.5): the child's turn ends `interrupted`, it leaves the live map, and
/// its end writes no second output.
#[test]
fn r1_f1_1_a_root_stopped_at_max_tool_errors_interrupts_its_live_child() {
    let mut k = kernel("");
    k.state
        .put(
            "profile/root",
            openai(
                r#","toolsets":["notes"],"delegates":["foreman"],"allow":["task","notes_*"],"max-tool-errors":1"#,
            )
            .into_bytes(),
        )
        .unwrap();
    k.state
        .put(
            "profile/foreman",
            openai(r#","max-tool-errors":1"#).into_bytes(),
        )
        .unwrap();
    let root = start_root(&mut k);
    let job_call =
        json!({"type":"function_call","id":"f","call_id":"c2","name":"notes_a","arguments":"{}"});
    reply(&mut k, &root, &[task("c1", "foreman", "x"), job_call]);
    let kid = children(&mut k, &root)[0].clone();
    assert!(rec(&mut k, &kid).turn.is_active(), "the child is live");
    // S134 r2 F2.2: both sessions are attached; the child's Stop reaches its own stream.
    let AttachedLive(kid_stream) = attach_live(&mut k, &kid);
    let AttachedLive(root_stream) = attach_live(&mut k, &root);
    k.host.calls.clear();
    // The root's job fails while the child runs: the streak reaches the cap.
    let job = lp(&mut k, &root)["calls"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|c| c["stream"].as_str().map(str::to_string))
        .expect("the job runs");
    k.provider_lines(
        &job,
        vec![(1, br#"{"err":{"code":"e","message":"m"}}"#.to_vec())],
    )
    .unwrap();
    let r = rec(&mut k, &root);
    assert!(
        matches!(&r.turn, Turn::Idle { stop: Stop::Stopped { limit }, .. } if limit == "max-tool-errors"),
        "{:?}",
        r.turn
    );
    assert!(r.children.is_empty(), "the live map is empty");
    let c = rec(&mut k, &kid);
    assert!(
        matches!(
            c.turn,
            Turn::Idle {
                stop: Stop::Interrupted,
                ..
            }
        ),
        "{:?}",
        c.turn
    );
    assert!(c.task_call.is_none());
    let turn_at = |stream: &str, state: &str| {
        k.host.calls.iter().position(|h| {
            matches!(h, HostCall::Push { stream: s, value } if s == stream
                && value.to_string().contains("\"turn\"")
                && value.to_string().contains(state))
        })
    };
    let kid_stop = turn_at(&kid_stream, "interrupted").expect("the child's Stop is pushed live");
    let root_stop = turn_at(&root_stream, "max-tool-errors").expect("the root's Stop is pushed");
    assert!(kid_stop < root_stop, "the child's Stop comes first");
    // The root's call got the cleanup's `aborted`, once.
    let outs: Vec<Value> = entries(&mut k, &root)
        .into_iter()
        .filter(|e| e["kind"] == "tool" && e["body"]["toolCallId"] == "c1")
        .collect();
    assert_eq!(outs.len(), 1, "{outs:?}");
}

/// S134 r2 F2.1: in a session that names no delegates, a toolset function named `task` is an
/// ordinary described tool: its call runs as a job and the model gets the job's result, not a
/// delegation refusal.
#[test]
fn r2_f2_1_a_toolset_task_in_a_session_without_delegates_runs_as_its_tool() {
    let mut k = kernel("");
    k.host.tools = vec![tool("notes_a", true), tool("task", true)];
    k.state
        .put(
            "profile/o",
            openai(r#","toolsets":["notes"],"allow":["notes_*","task"]"#).into_bytes(),
        )
        .unwrap();
    let id = k.start("bob", "b", "o", "{}", IN).unwrap().value;
    let call = json!({"type":"function_call","id":"f","call_id":"t1","name":"task",
        "arguments": json!({"profile": "foreman", "input": "x"}).to_string()});
    let s = stream_of(&mut k, &id);
    let n = last_seq(&mut k, &s);
    k.provider_lines(
        &s,
        vec![(n + 1, sse_head()), (n + 2, done(&[call], Some((1, 1))))],
    )
    .unwrap();
    assert!(
        k.host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::ToolsCall { tool, .. } if tool == "task")),
        "the toolset's task runs as a job"
    );
    assert!(rec(&mut k, &id).children.is_empty(), "no child");
    let job = lp(&mut k, &id)["calls"][0]["stream"]
        .as_str()
        .expect("a job stream")
        .to_string();
    k.provider_lines(&job, vec![(1, br#"{"ok":"done by the toolset"}"#.to_vec())])
        .unwrap();
    k.provider_ended_with(&job, None).unwrap();
    let last = bodies(&k).pop().expect("the follow-up request");
    let out = last["input"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["type"] == "function_call_output" && i["call_id"] == "t1")
        .expect("the output")["output"]
        .clone();
    assert_eq!(out, json!("\"done by the toolset\""));
}

/// S134 r3 F3.1: a child's failed result that takes the root to `max-tool-errors` interrupts the
/// sibling, and the sibling's Stop reaches its own attached stream before the root's Stop.
#[test]
fn r3_f3_1_a_child_result_that_stops_the_root_pushes_the_siblings_stop_first() {
    let mut k = kernel(r#","max-tool-errors":1"#);
    for p in ["foreman", "validator"] {
        k.state
            .put(
                &format!("profile/{p}"),
                openai(r#","max-tool-errors":1"#).into_bytes(),
            )
            .unwrap();
    }
    let (root, a, b) = root_with_two(&mut k);
    let AttachedLive(b_stream) = attach_live(&mut k, &b);
    let AttachedLive(root_stream) = attach_live(&mut k, &root);
    k.host.calls.clear();
    // Child `a` fails: its `task` output is an `err`, the root's streak reaches the cap.
    let s = stream_of(&mut k, &a);
    let n = last_seq(&mut k, &s);
    let head = json!({"status": 400, "headers": {"content-type": "application/json"}});
    k.provider_lines(&s, vec![(n + 1, head.to_string().into_bytes())])
        .unwrap();
    k.provider_ended_with(&s, None).unwrap();
    assert!(
        matches!(stop_of(&mut k, &root), Stop::Stopped { limit } if limit == "max-tool-errors")
    );
    assert!(matches!(stop_of(&mut k, &b), Stop::Interrupted));
    let turn_at = |stream: &str, state: &str| {
        k.host.calls.iter().position(|h| {
            matches!(h, HostCall::Push { stream: s, value } if s == stream
                && value.to_string().contains("\"turn\"")
                && value.to_string().contains(state))
        })
    };
    let b_stop = turn_at(&b_stream, "interrupted").expect("the sibling's Stop is pushed live");
    let root_stop = turn_at(&root_stream, "max-tool-errors").expect("the root's Stop is pushed");
    assert!(b_stop < root_stop, "the sibling's Stop comes first");
}

fn skill_profile(name: &str, md: &str) -> Value {
    json!({"name": name, "files": {"SKILL.md": md}})
}

fn set_skills(k: &mut K, profile: &str, skills: &[Value]) {
    let key = format!("profile/{profile}");
    let mut p: Value = serde_json::from_slice(&k.state.get(&key).unwrap()).unwrap();
    p["skills"] = json!(skills);
    k.state.put(&key, serde_json::to_vec(&p).unwrap()).unwrap();
}

fn skill_call(id: &str, args: Value) -> Value {
    json!({"type":"function_call","id":id,"call_id":id,"name":"skill",
        "arguments": args.to_string()})
}

fn skill_definition(req: &Value) -> Option<&Value> {
    req["tools"]
        .as_array()?
        .iter()
        .find(|t| t["name"] == "skill")
}

#[test]
fn s148_ac1_lists_escaped_skills_and_returns_the_pinned_file_in_a_tool_row() {
    let md = "---\nname: one\ndescription: Read </description> <here> & there\n  next line\n---\nThe body.";
    let mut k = kernel(r#","allow":["task","skill"]"#);
    set_skills(
        &mut k,
        "root",
        &[
            skill_profile("one", md),
            skill_profile("two", "---\ndescription: Second\n---\nOther"),
        ],
    );
    let root = start_root(&mut k);
    let requests = bodies(&k);
    let def = skill_definition(&requests[0]).unwrap();
    assert_eq!(def["parameters"]["required"], json!(["name"]));
    let listing = def["description"].as_str().unwrap();
    for part in [
        "one",
        "two",
        "Second",
        "&lt;/description&gt; &lt;here&gt; &amp; there next line",
    ] {
        assert!(listing.contains(part), "{listing}");
    }
    assert!(!listing.contains("The body."));
    set_skills(&mut k, "root", &[skill_profile("one", "New revision.")]);
    reply(&mut k, &root, &[skill_call("s1", json!({"name":"one"}))]);
    assert_eq!(output_of(&bodies(&k)[1], "s1"), json!(md));
    let rows = kind_entries(&mut k, &root, "tool");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["body"]["toolCallId"], "s1");
    assert_eq!(rows[0]["body"]["status"], "completed");
    assert!(rows[0]["body"].to_string().contains("The body."));
    assert!(!k
        .host
        .calls
        .iter()
        .any(|call| matches!(call, HostCall::ToolsCall { .. })));
}

#[test]
fn s148_ac2_refuses_invalid_calls_and_extra_or_missing_files() {
    let mut k = kernel("");
    for files in [json!({"SKILL.md":"one","extra.md":"extra"}), json!({})] {
        set_skills(&mut k, "root", &[json!({"name":"one","files":files})]);
        let refusals = k.check_named("alice", "root", "{}");
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert_eq!(refusals[0].field, "skills[0].files");
        assert!(refusals[0].reason.contains("SKILL.md"));
        assert_eq!(
            k.start("alice", "k", "root", "{}", IN).unwrap_err(),
            KernelError::Refused(refusals)
        );
        assert!(bodies(&k).is_empty());
    }
    set_skills(&mut k, "root", &[skill_profile("one", "Body")]);
    let root = start_root(&mut k);
    k.attach("alice", &root, 0).unwrap();
    reply(
        &mut k,
        &root,
        &[
            skill_call("unknown", json!({"name":"two"})),
            skill_call("extra", json!({"name":"one","extra":true})),
            skill_call("array", json!(["one"])),
        ],
    );
    assert!(kind_entries(&mut k, &root, "ask").is_empty());
    let reqs = bodies(&k);
    let req = reqs
        .get(1)
        .expect("invalid calls close without a broker ask");
    assert!(output_of(req, "unknown").to_string().contains("not in"));
    assert!(output_of(req, "extra").to_string().contains("name"));
    assert!(output_of(req, "array").to_string().contains("name"));
}

#[test]
fn s148_ac3_children_receive_only_their_exact_subset() {
    let mut k = kernel(r#","allow":["task","skill"]"#);
    let one = skill_profile("one", "---\ndescription: First\n---\nBody one");
    let two = skill_profile("two", "---\ndescription: Second\n---\nBody two");
    set_skills(&mut k, "root", &[one.clone(), two]);
    let root = start_root(&mut k);
    for (i, skill) in [
        skill_profile("one", "Changed"),
        skill_profile("three", "New"),
    ]
    .into_iter()
    .enumerate()
    {
        set_skills(&mut k, "validator", &[skill]);
        let call = format!("bad{i}");
        reply(&mut k, &root, &[task(&call, "validator", "x")]);
        assert!(children(&mut k, &root).is_empty());
        assert_eq!(
            output_of(bodies(&k).last().unwrap(), &call)["err"]["refusals"][0]["field"],
            "skills"
        );
    }
    set_skills(&mut k, "foreman", &[one]);
    let key = "profile/foreman";
    let mut child: Value = serde_json::from_slice(&k.state.get(key).unwrap()).unwrap();
    child["allow"] = json!(["skill"]);
    k.state
        .put(key, serde_json::to_vec(&child).unwrap())
        .unwrap();
    reply(&mut k, &root, &[task("good", "foreman", "x")]);
    let kids = children(&mut k, &root);
    assert_eq!(kids.len(), 1);
    let reqs = bodies(&k);
    let listing = skill_definition(reqs.last().unwrap()).unwrap()["description"]
        .as_str()
        .unwrap();
    assert!(
        listing.contains("First") && !listing.contains("Second"),
        "{listing}"
    );
    reply(
        &mut k,
        &kids[0],
        &[skill_call("outside", json!({"name":"two"}))],
    );
    let reqs = bodies(&k);
    assert!(output_of(reqs.last().unwrap(), "outside")
        .to_string()
        .contains("not in"));
}

#[test]
fn s148_ac2_the_broker_can_deny_a_named_skill() {
    let mut k = kernel(r#","deny":["skill"],"allow":["task","skill"]"#);
    set_skills(&mut k, "root", &[skill_profile("one", "Body")]);
    let root = start_root(&mut k);
    reply(
        &mut k,
        &root,
        &[skill_call("denied", json!({"name":"one"}))],
    );
    assert_eq!(
        output_of(&bodies(&k)[1], "denied"),
        json!({"error":"denied by the profile"})
    );
}

#[test]
fn s148_ac4_skill_collides_with_fresh_and_cached_descriptions_only_when_active() {
    let mut k = kernel("");
    let _ = start_root(&mut k);
    assert!(skill_definition(&bodies(&k)[0]).is_none());

    for cached in [false, true] {
        let mut k = kernel(r#","toolsets":["notes"]"#);
        set_skills(&mut k, "root", &[skill_profile("one", "Body")]);
        if !cached {
            k.host.tools = vec![tool("skill", true)];
        }
        let id = start_root(&mut k);
        let sent = bodies(&k).len();
        if cached {
            assert_eq!(sent, 1);
            reply(&mut k, &id, &[]);
            let key = format!("loop/{id}");
            let mut stored = lp(&mut k, &id);
            stored["tools"]["info"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"skill"}));
            k.state
                .put(&key, serde_json::to_vec(&stored).unwrap())
                .unwrap();
            k.say("alice", &id, "s2", MORE).unwrap();
            assert_eq!(bodies(&k).len(), sent);
        } else {
            assert_eq!(sent, 0);
        }
        match stop_of(&mut k, &id) {
            Stop::Failed { error, .. } => assert!(
                error.contains("toolsets") && error.contains("skill"),
                "{error}"
            ),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn s148_ac5_claude_launch_keeps_all_plugin_files() {
    let mut k = kernel("");
    k.state
        .put(
            "profile/root",
            br#"{"skills":[{"name":"one","files":{"SKILL.md":"Body","extra.md":"Extra"}}]}"#
                .to_vec(),
        )
        .unwrap();
    start_root(&mut k);
    let launch = k
        .host
        .calls
        .iter()
        .find_map(|call| match call {
            HostCall::StartProvider { launch, .. } => Some(launch),
            _ => None,
        })
        .unwrap();
    let launch: Value = serde_json::from_str(launch).unwrap();
    assert_eq!(
        launch["skills"][0],
        json!({"name":"one","files":{"SKILL.md":"Body","extra.md":"Extra"}})
    );
}
