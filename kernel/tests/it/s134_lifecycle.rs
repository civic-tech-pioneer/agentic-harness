//! S134-6: stopping a root stops its children, and a restart leaves no half delegation (TECHNICAL
//! 9.7 "The function and its lifetime", 9.3 R4 and R5, 9.5 interrupt): AC1 to AC6.

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

fn stop_of(k: &mut K, id: &str) -> Stop {
    match rec(k, id).turn {
        Turn::Idle { stop, .. } => stop,
        t => panic!("not idle: {t:?}"),
    }
}

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

fn calls_of(k: &mut K, root: &str) -> usize {
    lp(k, root)["calls"].as_array().map_or(0, Vec::len)
}

/// The outputs the root's next request carries after the owner says something to it.
fn root_outputs_after_say(k: &mut K, root: &str) -> Vec<(String, String)> {
    let n = bodies(k).len();
    k.say("alice", root, "after", MORE).unwrap();
    let b = bodies(k);
    assert_eq!(b.len(), n + 1, "the say sends one request");
    outputs(b.last().unwrap())
}

fn out(o: &[(String, String)], call: &str) -> String {
    o.iter().find(|(c, _)| c == call).unwrap().1.clone()
}

fn pause(k: &mut K, id: &str, secs: u64) {
    let s = stream_of(k, id);
    let n = last_seq(k, &s);
    let body = json!({"error":{"type":"usage_limit_reached","code":"usage_limit_reached",
        "message":"limit","plan_type":"plus","resets_in_seconds":secs}})
    .to_string()
    .into_bytes();
    let head = json!({"status": 429, "headers": {"content-type": "application/json"}})
        .to_string()
        .into_bytes();
    k.provider_lines(&s, vec![(n + 1, head), (n + 2, body)])
        .unwrap();
}

fn restart(k: &mut K) {
    k.host.generation = 2;
}

/// A bystander session and the keys a call read: none of them is the bystander's.
fn start_bystander(k: &mut K) -> String {
    k.start("alice", "by", "root", "{}", IN).unwrap().value
}

fn assert_bounded(k: &K, bystander: &str) {
    let keys: Vec<&String> = k
        .state
        .get_keys()
        .iter()
        .filter(|key| key.contains(bystander))
        .collect();
    assert!(keys.is_empty(), "a call read a stranger's keys: {keys:?}");
}

// ---- AC1

#[test]
fn ac1_interrupt_of_a_root_ends_both_children_interrupted_and_aborts_its_calls() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    let by = start_bystander(&mut k);
    k.host.calls.clear();
    k.state.reset_counts();
    k.interrupt("alice", &root, None).unwrap();
    assert_bounded(&k, &by);
    assert_eq!(stop_of(&mut k, &a), Stop::Interrupted);
    assert_eq!(stop_of(&mut k, &b), Stop::Interrupted);
    assert_eq!(stop_of(&mut k, &root), Stop::Interrupted);
    assert_eq!(calls_of(&mut k, &root), 0);
    assert!(children(&mut k, &root).is_empty());
    // The children's handles are kept: no toolset close was sent for them.
    assert!(!k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::Request { .. }) && format!("{c:?}").contains("Cancel")));
    // The root's calls got `aborted`, and the children's ends wrote no second output.
    let o = root_outputs_after_say(&mut k, &root);
    assert_eq!(o.len(), 2, "{o:?}");
    assert_eq!(out(&o, "c1"), "aborted");
    assert_eq!(out(&o, "c2"), "aborted");
    assert_eq!(kind_entries(&mut k, &root, "tool").len(), 2);
    // The children's records stay, and their association is cut.
    assert_eq!(rec(&mut k, &a).parent.as_deref(), Some(root.as_str()));
    assert_eq!(rec(&mut k, &a).task_call, None);
}

#[test]
fn ac1_the_roots_stop_push_comes_after_the_childrens_cleanup() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    k.host.calls.clear();
    let called = k.interrupt("alice", &root, None).unwrap();
    let turns: Vec<(String, bool)> = called
        .pushes
        .iter()
        .filter(|p| p.value["type"] == "turn")
        .map(|p| (p.session.clone(), true))
        .collect();
    let order: Vec<&str> = turns.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(order, [a.as_str(), b.as_str(), root.as_str()], "{order:?}");
}

// ---- AC2

#[test]
fn ac2_cancel_of_a_root_cancels_each_child_and_keeps_their_records() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    let by = start_bystander(&mut k);
    k.host.calls.clear();
    k.state.reset_counts();
    k.cancel("alice", &root).unwrap();
    assert_bounded(&k, &by);
    let cancels = k
        .host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::Request { .. }) && format!("{c:?}").contains("Cancel"))
        .count();
    assert_eq!(cancels, 3, "one cancel each for the root and its children");
    for id in [&a, &b, &root] {
        assert_eq!(stop_of(&mut k, id), Stop::Interrupted);
    }
    let o = root_outputs_after_say(&mut k, &root);
    assert_eq!(
        (out(&o, "c1"), out(&o, "c2")),
        ("aborted".into(), "aborted".into())
    );
    // The owner can still read and delete the children.
    k.read("alice", &a, 0, None, 10).unwrap();
    k.delete("alice", &a).unwrap();
    k.read("alice", &b, 0, None, 10).unwrap();
}

#[test]
fn ac2_delete_of_a_root_cancels_each_child_and_keeps_their_records() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    let by = start_bystander(&mut k);
    k.host.calls.clear();
    k.state.reset_counts();
    k.delete("alice", &root).unwrap();
    assert_bounded(&k, &by);
    let cancels = k
        .host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::Request { .. }) && format!("{c:?}").contains("Cancel"))
        .count();
    assert_eq!(cancels, 3, "{:?}", k.host.calls);
    for id in [&a, &b] {
        assert_eq!(stop_of(&mut k, id), Stop::Interrupted);
        assert!(!rec(&mut k, id).turn.is_active());
        assert_eq!(rec(&mut k, id).task_call, None);
    }
    assert!(Record::load(&mut k.state, &root).is_none());
    k.read("alice", &a, 0, None, 10).unwrap();
    k.delete("alice", &a).unwrap();
    k.delete("alice", &b).unwrap();
}

// ---- AC3

#[test]
fn ac3_cancel_of_a_live_child_aborts_the_roots_call_once_and_the_root_goes_on() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    let by = start_bystander(&mut k);
    k.state.reset_counts();
    k.cancel("alice", &a).unwrap();
    assert_bounded(&k, &by);
    assert_eq!(calls_of(&mut k, &root), 1);
    assert_eq!(children(&mut k, &root), std::slice::from_ref(&b));
    let n = bodies(&k).len();
    // No request yet: the other call is still out. A second cancel resolves nothing.
    k.cancel("alice", &a).unwrap();
    assert_eq!(calls_of(&mut k, &root), 1);
    reply(&mut k, &b, &[text("B done")]);
    let bs = bodies(&k);
    assert_eq!(bs.len(), n + 1, "the root's one follow-up request");
    let o = outputs(bs.last().unwrap());
    assert_eq!(out(&o, "c1"), "aborted");
    assert!(out(&o, "c2").contains("B done"));
    assert_eq!(o.len(), 2);
}

#[test]
fn ac3_delete_of_a_live_child_aborts_the_roots_call_once() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    k.delete("alice", &a).unwrap();
    assert_eq!(calls_of(&mut k, &root), 1);
    assert_eq!(children(&mut k, &root), std::slice::from_ref(&b));
    reply(&mut k, &b, &[text("B done")]);
    let bs = bodies(&k);
    let o = outputs(bs.last().unwrap());
    assert_eq!(out(&o, "c1"), "aborted");
    assert!(out(&o, "c2").contains("B done"));
}

#[test]
fn ac3_interrupt_of_a_live_child_resolves_the_call_interrupted() {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    k.interrupt("alice", &a, None).unwrap();
    assert_eq!(stop_of(&mut k, &a), Stop::Interrupted);
    reply(&mut k, &b, &[text("B done")]);
    let o = outputs(bodies(&k).last().unwrap());
    assert_eq!(
        serde_json::from_str::<Value>(&out(&o, "c1")).unwrap(),
        json!({"err":{"code":"interrupted","session":a}})
    );
    assert_eq!(calls_of(&mut k, &root), 0);
}

// ---- AC4, AC5

/// Two children, the second one paused with a `resume` deadline.
fn restarted_with_paused(first_root: bool) -> (K, String, String, String) {
    let mut k = kernel("");
    let (root, a, b) = root_with_two(&mut k);
    pause(&mut k, &b, 600);
    assert!(matches!(
        rec(&mut k, &b).turn,
        Turn::Paused { until: Some(_) }
    ));
    assert_eq!(rec(&mut k, &b).deadlines.len(), 1);
    restart(&mut k);
    k.host.calls.clear();
    let ids = if first_root {
        [&root, &a, &b]
    } else {
        [&b, &a, &root]
    };
    for id in ids {
        k.read("alice", id, 0, None, 1).unwrap();
    }
    assert!(bodies(&k).is_empty(), "the restart sends no request");
    (k, root, a, b)
}

fn check_restarted(k: &mut K, root: &str, a: &str, b: &str) {
    assert_eq!(rec(k, root).turn, Turn::Unknown);
    assert_eq!(rec(k, a).turn, Turn::Unknown);
    assert_eq!(rec(k, b).turn, Turn::Unknown);
    let rb = rec(k, b);
    assert!(rb.deadlines.is_empty(), "the resume deadline is dropped");
    assert_eq!(lp(k, b)["state"], json!("idle"));
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::TimerCancel { id } if *id == format!("s/{b}"))));
    // The links stay.
    assert_eq!(rec(k, a).parent.as_deref(), Some(root));
    assert_eq!(rec(k, b).parent.as_deref(), Some(root));
    // R4 clears the live child map (9.3); the `child` entries and `parent` links keep them.
    assert!(rec(k, root).children.is_empty());
    assert_eq!(kind_entries(k, root, "child").len(), 2);
    assert_eq!(calls_of(k, root), 0);
}

#[test]
fn ac4_a_restart_with_a_paused_child_leaves_no_half_delegation_in_either_order() {
    for first_root in [true, false] {
        let (mut k, root, a, b) = restarted_with_paused(first_root);
        check_restarted(&mut k, &root, &a, &b);
    }
}

#[test]
fn ac4_the_roots_calls_get_aborted_and_nothing_reaches_the_root_later() {
    let (mut k, root, a, b) = restarted_with_paused(true);
    // The owner says to a child: R5 does not rejoin the old call.
    let n = bodies(&k).len();
    k.say("alice", &a, "s1", MORE).unwrap();
    assert_eq!(bodies(&k).len(), n + 1);
    reply(&mut k, &a, &[text("late A")]);
    k.say("alice", &b, "s2", MORE).unwrap();
    reply(&mut k, &b, &[text("late B")]);
    assert_eq!(calls_of(&mut k, &root), 0);
    assert_eq!(rec(&mut k, &root).turn, Turn::Unknown);
    // Then the root: its request carries the aborted outputs and no child result.
    let o = root_outputs_after_say(&mut k, &root);
    assert_eq!(
        (out(&o, "c1"), out(&o, "c2")),
        ("aborted".into(), "aborted".into())
    );
    assert_eq!(o.len(), 2);
}

#[test]
fn ac5_a_late_end_of_an_older_boot_child_writes_nothing_to_the_root() {
    let mut k = kernel("");
    let (root, a, _b) = root_with_two(&mut k);
    let stream = stream_of(&mut k, &a);
    restart(&mut k);
    k.read("alice", &root, 0, None, 1).unwrap();
    let before_entries = entries(&mut k, &root).len();
    let n = bodies(&k).len();
    // A late item of the old stream: the response completes after the restart.
    let n0 = last_seq(&mut k, &stream);
    let _ = k.provider_lines(
        &stream,
        vec![
            (n0 + 1, sse_head()),
            (n0 + 2, done(&[text("late")], Some((1, 1)))),
        ],
    );
    let _ = k.provider_ended_with(&stream, None);
    assert_eq!(entries(&mut k, &root).len(), before_entries);
    assert_eq!(bodies(&k).len(), n);
    assert_eq!(rec(&mut k, &root).turn, Turn::Unknown);
    assert_eq!(calls_of(&mut k, &root), 0);
}

/// S134 r1 F1.2: the `resume` deadline of a paused child checks the root's access (9.7
/// "Revoking"), as `say` does: a child authorized only through its root resumes, and once the
/// root's profile is withdrawn the child stays paused with no time, even when the child's own
/// profile is granted separately.
#[test]
fn r1_f1_2_a_paused_childs_resume_checks_the_roots_access() {
    let mut k = kernel("");
    let (_root, a, b) = root_with_two(&mut k);
    pause(&mut k, &a, 60);
    k.host.now += 61_000;
    k.timer(&format!("s/{a}"));
    assert!(
        matches!(rec(&mut k, &a).turn, Turn::Running { .. }),
        "resumed through the root's access: {:?}",
        rec(&mut k, &a).turn
    );
    pause(&mut k, &b, 60);
    k.state
        .put("access/alice", br#"["foreman","validator"]"#.to_vec())
        .unwrap();
    k.host.now += 61_000;
    k.timer(&format!("s/{b}"));
    assert_eq!(rec(&mut k, &b).turn, Turn::Paused { until: None });
}
