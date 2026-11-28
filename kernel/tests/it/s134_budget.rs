//! S134-7: stage F's state I/O budget (TECHNICAL 17, row F "as D"): a root with two simultaneous
//! `task` children, each child one step with two tool jobs, at two history lengths, on the fake
//! host. The page-level budgets (item delay, Stop through the last child, writer busy share) need
//! `watane-dev` and are not measured here.

use agent_kernel::active::Slots;
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
const OWNERS: [&str; 2] = ["alice", "bob"];

fn tool(name: &str) -> Tool {
    Tool {
        name: name.into(),
        description: format!("does {name}"),
        input_schema: r#"{"type":"object","properties":{}}"#.into(),
        parallel: true,
        idempotent: true,
    }
}

fn openai(extra: &str) -> String {
    format!(r#"{{"backend":"openai","model":"gpt-5","system-prompt":{{"replace":"x"}}{extra}}}"#)
}

/// Two owners on the root profile `root` (the `notes` tools, which a child may not exceed, and `task`; two live children) whose children run the
/// profile `worker` with the `notes` tools.
fn kernel(with_skills: bool) -> K {
    let mut skills = vec![json!({
        "name": "notes",
        "files": {"SKILL.md": "---\ndescription: Summarize notes\n---\nGroup the notes by topic."},
    })];
    let child_skills = if with_skills {
        format!(r#","skills":{}"#, json!(skills))
    } else {
        String::new()
    };
    skills.push(json!({
        "name": "reference",
        "files": {"SKILL.md": format!("---\ndescription: Reference\n---\n{}", "a".repeat(1000 * 1024))},
    }));
    let root_skills = if with_skills {
        format!(r#","skills":{}"#, json!(skills))
    } else {
        String::new()
    };
    let allow_skill = if with_skills { r#","skill""# } else { "" };
    let mut st = MemState::new();
    st.put(
        "profile/root",
        openai(&format!(r#","toolsets":["notes"],"delegates":["worker"],"allow":["task","notes_*"{allow_skill}],"subagent-limits":{{"live":2}}{root_skills}"#))
            .into_bytes(),
    )
    .unwrap();
    st.put(
        "profile/worker",
        openai(&format!(
            r#","toolsets":["notes"],"allow":["notes_*"{allow_skill}]{child_skills}"#
        ))
        .into_bytes(),
    )
    .unwrap();
    for who in OWNERS {
        crate::fixture_access::put(&mut st, &format!("access/{who}"), br#"["root"]"#.to_vec())
            .unwrap();
    }
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        generation: 1,
        now: 1_000_000,
        tools: vec![tool("notes_a")],
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn lp(k: &mut K, id: &str) -> Value {
    serde_json::from_slice(&k.state.get(&format!("loop/{id}")).unwrap()).unwrap()
}

fn stream_of(k: &mut K, id: &str) -> String {
    lp(k, id)["stream"].as_str().unwrap().to_string()
}

fn sse_head() -> Vec<u8> {
    json!({"status": 200, "headers": {"content-type": "text/event-stream"}})
        .to_string()
        .into_bytes()
}

fn completed(output: &[Value]) -> Vec<u8> {
    json!({"type":"response.completed","response":{"id":"r","usage":{"input_tokens":10,"output_tokens":5},"output":output}})
        .to_string()
        .into_bytes()
}

fn call_of(id: &str, name: &str, args: &str) -> Value {
    json!({"type":"function_call","id":"fc","call_id":id,"name":name,"arguments":args})
}

fn last_seq(k: &mut K, stream: &str) -> u64 {
    k.state
        .get(&format!("stream/{stream}"))
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["last"].as_u64())
        .unwrap_or(0)
}

fn feed(k: &mut K, stream: &str, items: Vec<Vec<u8>>) {
    let last = last_seq(k, stream);
    let batch = items
        .into_iter()
        .enumerate()
        .map(|(n, b)| (last + 1 + n as u64, b))
        .collect();
    k.provider_lines(stream, batch).unwrap();
}

/// A response of `output` on the session's current request, and its end.
fn reply(k: &mut K, id: &str, output: &[Value]) {
    let s = stream_of(k, id);
    feed(k, &s, vec![sse_head(), completed(output)]);
    k.provider_ended_with(&s, None).unwrap();
}

fn jobs(k: &mut K, id: &str) -> Vec<String> {
    lp(k, id)["calls"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|c| c["st"] == "job")
        .map(|c| c["stream"].as_str().unwrap().to_string())
        .collect()
}

fn children(k: &mut K, root: &str) -> Vec<String> {
    Record::load(&mut k.state, root).unwrap().children
}

fn turn_stop(k: &mut K, id: &str) -> Stop {
    match Record::load(&mut k.state, id).unwrap().turn {
        Turn::Idle { stop, .. } => stop,
        t => panic!("not idle: {t:?}"),
    }
}

/// Bytes read and written since `since`.
fn spent(k: &K, since: Counts) -> u64 {
    let now = k.state.counts();
    (now.bytes_read - since.bytes_read) + (now.bytes_written - since.bytes_written)
}

/// One root step: the root answers with two `task` calls (compacting every tenth step, as D).
/// Returns the children and the bytes of the root's dispatch call.
fn root_step(k: &mut K, root: &str, n: u64) -> (Vec<String>, u64) {
    let mut output = vec![
        call_of(
            &format!("{n}-a"),
            "task",
            r#"{"profile":"worker","input":"A"}"#,
        ),
        call_of(
            &format!("{n}-b"),
            "task",
            r#"{"profile":"worker","input":"B"}"#,
        ),
    ];
    if n.is_multiple_of(10) {
        output.insert(
            0,
            json!({"type":"compaction","encrypted_content":"summary"}),
        );
    }
    let before = k.state.counts();
    reply(k, root, &output);
    let bytes = spent(k, before);
    let kids = children(k, root);
    assert_eq!(kids.len(), 2, "two simultaneous children at step {n}");
    (kids, bytes)
}

/// One child's step: two tool jobs, then the answer that resolves the root's call. Bytes of the
/// whole child step.
fn child_step(k: &mut K, child: &str, with_skills: bool) -> u64 {
    let before = k.state.counts();
    let mut calls = vec![
        call_of("j1", "notes_a", "{}"),
        call_of("j2", "notes_a", "{}"),
    ];
    if with_skills {
        calls.push(call_of("s1", "skill", r#"{"name":"notes"}"#));
    }
    reply(k, child, &calls);
    let running = jobs(k, child);
    assert_eq!(running.len(), 2, "two open tool jobs");
    for stream in running {
        let last = last_seq(k, &stream);
        k.provider_lines(
            &stream,
            vec![(
                last + 1,
                json!({"ok": "fixture result"}).to_string().into_bytes(),
            )],
        )
        .unwrap();
        k.provider_ended_with(&stream, None).unwrap();
    }
    reply(
        k,
        child,
        &[
            json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}]}),
        ],
    );
    spent(k, before)
}

/// Mean bytes per root step, per child step and per whole cycle over `SAMPLE` cycles of each owner
/// after `history` cycles of each, then each root stopped with two live children.
fn measure(history: u64, with_skills: bool) -> (u64, u64, u64, u64, u64) {
    // Long enough to span whole chunk cycles (64 KiB of wire or transcript), as D's 128 steps do:
    // the bytes read depend on where in its chunk the active history sits.
    const SAMPLE: u64 = 128;
    let mut k = kernel(with_skills);
    let roots: Vec<String> = OWNERS
        .iter()
        .map(|o| k.start(o, "load", "root", "{}", IN).unwrap().value)
        .collect();
    for (o, r) in OWNERS.iter().zip(&roots) {
        k.attach(o, r, 0).unwrap();
    }
    let (mut root_bytes, mut child_bytes, mut cycles, mut child_count) = (0, 0, 0, 0);
    // Each sampled step's bytes, for the p95 of 17 ("All p95"; S134 r1 F1.9).
    let (mut root_steps, mut child_steps): (Vec<u64>, Vec<u64>) = (Vec::new(), Vec::new());
    for n in 1..=history + SAMPLE {
        let sampled = n > history;
        if n == history + 1 {
            k.state.reset_counts();
        }
        for (o, root) in OWNERS.iter().zip(&roots) {
            let (kids, rb) = root_step(&mut k, root, n);
            let mut cb = 0;
            let mut each = Vec::new();
            for kid in &kids {
                let b = child_step(&mut k, kid, with_skills);
                each.push(b);
                cb += b;
            }
            if sampled {
                root_steps.push(rb);
                child_steps.extend(each);
                root_bytes += rb;
                child_bytes += cb;
                cycles += rb + cb;
                child_count += 2;
            }
            // The owner removes the finished children, so sessions do not pile up.
            for kid in kids {
                k.delete(o, &kid).unwrap();
            }
            k.host.calls.clear();
        }
    }
    // Stop of each root with live children.
    let mut stop_bytes = 0;
    for (o, root) in OWNERS.iter().zip(&roots) {
        let (kids, _) = root_step(&mut k, root, history + SAMPLE + 1);
        let before = k.state.counts();
        let calls_before = k.host.calls.len();
        k.interrupt(o, root, None).unwrap();
        stop_bytes += spent(&k, before);
        assert!(k.host.calls.len() >= calls_before, "counted calls");
        for id in kids.iter().chain([root]) {
            assert_eq!(turn_stop(&mut k, id), Stop::Interrupted, "{id} stopped");
        }
        assert!(children(&mut k, root).is_empty(), "no live child left");
    }
    println!("stage F, history {history}: stop of two roots with two live children each: {stop_bytes} bytes");
    let per_root = SAMPLE * OWNERS.len() as u64;
    (
        root_bytes / per_root,
        child_bytes / child_count,
        cycles / per_root,
        p95(root_steps),
        p95(child_steps),
    )
}

/// The 95th percentile (nearest rank) of a sample.
fn p95(mut v: Vec<u64>) -> u64 {
    v.sort_unstable();
    let rank = (v.len() * 95).div_ceil(100).max(1);
    v[rank - 1]
}

/// Stage F's state I/O budget (17): the bytes read and written per root step and per child step
/// stay within 4 MiB, and the 1,000-step figure within 10% of the 10-step one.
#[test]
#[ignore = "slow: feeds 1,000 root steps with two children of two tool jobs each, for two owners"]
fn s134_stage_f_state_bytes_per_step() {
    check_budget(false);
}

#[test]
#[ignore = "slow: measures skill output over 1,000 delegation cycles with two owners"]
fn s148_stage_f_skill_state_bytes_per_step() {
    check_budget(true);
}

fn check_budget(with_skills: bool) {
    let (early_root, early_child, early, early_root95, early_child95) = measure(10, with_skills);
    let (late_root, late_child, late, late_root95, late_child95) = measure(1_000, with_skills);
    println!(
        "stage F bytes p95 per root step at 10: {early_root95}, at 1000: {late_root95}; \
         per child step at 10: {early_child95}, at 1000: {late_child95}"
    );
    for v in [early_root95, early_child95, late_root95, late_child95] {
        assert!(v <= 4 << 20, "a step's p95 exceeds 4 MiB: {v}");
    }
    println!(
        "stage F bytes (read + written) per root step at 10: {early_root}, at 1000: {late_root}; \
         per child step at 10: {early_child}, at 1000: {late_child}; \
         per whole cycle (root and two children) at 10: {early}, at 1000: {late}"
    );
    for v in [early_root, early_child, early, late_root, late_child, late] {
        assert!(v <= 4 << 20, "a step exceeds 4 MiB: {v}");
    }
    // The history tolerance holds for the means and for the p95 series (S134 r2 F2.3).
    for (a, b, what) in [
        (early_root, late_root, "root step"),
        (early_child, late_child, "child step"),
        (early, late, "cycle"),
        (early_root95, late_root95, "root step p95"),
        (early_child95, late_child95, "child step p95"),
    ] {
        assert!(
            a.abs_diff(b) * 10 <= a,
            "state I/O per {what} changed from {a} to {b}"
        );
    }
}

#[test]
fn s148_delegate_skill_admission_state_bytes_at_limit() {
    let mut k = kernel(false);
    let md = "a".repeat((1 << 20) - "SKILL.md".len());
    let mut profile_bytes = 0;
    for profile in ["root", "worker"] {
        let key = format!("profile/{profile}");
        let mut doc: Value = serde_json::from_slice(&k.state.get(&key).unwrap()).unwrap();
        doc["skills"] = json!([{"name":"reference","files":{"SKILL.md":md}}]);
        let bytes = serde_json::to_vec(&doc).unwrap();
        profile_bytes = bytes.len() as u64;
        k.state.put(&key, bytes).unwrap();
    }
    for who in OWNERS {
        crate::fixture_access::put(
            &mut k.state,
            &format!("access/{who}"),
            br#"["root"]"#.to_vec(),
        )
        .unwrap();
    }
    let root = k.start("alice", "load", "root", "{}", IN).unwrap().value;
    let before = k.state.counts();
    reply(
        &mut k,
        &root,
        &[call_of(
            "one",
            "task",
            r#"{"profile":"worker","input":"A"}"#,
        )],
    );
    let after = k.state.counts();
    let read = after.bytes_read - before.bytes_read;
    let written = after.bytes_written - before.bytes_written;

    let kids = children(&mut k, &root);
    assert_eq!(kids.len(), 1);
    let child = &kids[0];
    assert_eq!(
        k.state.get(&format!("skills/{child}/reference")).unwrap(),
        md.as_bytes()
    );
    assert!(read >= profile_bytes);
    assert!(written >= md.len() as u64);
    println!("stage F one admitted child at the 1 MiB skill limit: {read} read + {written} written = {} bytes", read + written);
}
