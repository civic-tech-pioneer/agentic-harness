//! The OpenAI wire history (S40-1): repair, request body, body budget, active history, chunks.

use agent_kernel::openai::wire::*;
use agent_kernel::state::{MemState, State};
use serde_json::{json, Value};

const M: &str = "gpt-test";

fn wi(i: u64, step: u64, model: Option<&str>, item: Value) -> WireItem {
    WireItem {
        i,
        step,
        model: model.map(str::to_string),
        item,
    }
}

fn call(c: &str) -> Value {
    json!({"type":"function_call","call_id":c,"name":"t","arguments":"{}"})
}

fn out(c: &str, s: &str) -> Value {
    json!({"type":"function_call_output","call_id":c,"output":s})
}

fn reasoning(enc: Option<&str>) -> Value {
    let mut v = json!({"type":"reasoning","id":"rs_1","summary":[]});
    if let Some(e) = enc {
        v["encrypted_content"] = json!(e);
    }
    v
}

fn args<'a>(tools: &'a [Value]) -> BodyArgs<'a> {
    BodyArgs {
        session: "s1",
        model: M,
        instructions: "be brief",
        tools,
        reasoning: None,
        compact_threshold: 500_000,
    }
}

fn input_of(body: &str) -> Vec<Value> {
    let v: Value = serde_json::from_str(body).unwrap();
    v["input"].as_array().unwrap().clone()
}

fn st() -> MemState {
    MemState::new()
}

fn text(n: usize) -> String {
    "x".repeat(n)
}

#[test]
fn ac1_repair_aborts_orphan_call_and_drops_orphan_output() {
    let items = vec![
        wi(0, 0, None, user_message("hi")),
        wi(1, 1, Some(M), call("a")),
        wi(2, 1, Some(M), call("b")),
        wi(3, 1, Some(M), out("a", "ok")),
        wi(4, 1, None, out("zzz", "stray")),
    ];
    let r = repair(&items, M);
    assert_eq!(
        r,
        vec![
            user_message("hi"),
            call("a"),
            call("b"),
            out("b", "aborted"),
            out("a", "ok"),
        ]
    );
}

#[test]
fn ac1_repair_reasoning_needs_encrypted_content_and_same_model() {
    let items = vec![
        wi(0, 1, Some(M), reasoning(Some("enc"))),
        wi(1, 1, Some(M), reasoning(None)),
        wi(2, 1, Some("other"), reasoning(Some("enc2"))),
        wi(
            3,
            1,
            Some(M),
            json!({"type":"message","role":"assistant","content":[]}),
        ),
    ];
    let r = repair(&items, M);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0]["encrypted_content"], "enc");
    assert!(r[0].get("id").is_none());
}

#[test]
fn ac7_body_fields_in_order() {
    let tools = vec![json!({"type":"function","name":"t"})];
    let mut w = Wire::default();
    let reasoning = json!({"effort":"low"});
    let mut a = args(&tools);
    a.reasoning = Some(&reasoning);
    let body = w.body(&a, &[wi(0, 0, None, user_message("hi"))]).unwrap();
    let keys = [
        "\"model\"",
        "\"instructions\"",
        "\"input\"",
        "\"tools\"",
        "\"tool_choice\":\"auto\"",
        "\"parallel_tool_calls\":true",
        "\"reasoning\"",
        "\"include\":[\"reasoning.encrypted_content\"]",
        "\"store\":false",
        "\"stream\":true",
        "\"prompt_cache_key\":\"s1\"",
        "\"context_management\":[{\"type\":\"compaction\",\"compact_threshold\":200000}]",
    ];
    let mut at = 0;
    for k in keys {
        let p = body[at..]
            .find(k)
            .unwrap_or_else(|| panic!("{k} in order: {body}"));
        at += p + k.len();
    }
    // No tools, no reasoning: those fields are left out.
    let body = Wire::default()
        .body(&args(&[]), &[wi(0, 0, None, user_message("hi"))])
        .unwrap();
    for k in [
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "reasoning\":",
    ] {
        assert!(!body.contains(k), "{k}");
    }
    assert!(body.contains("\"compact_threshold\":200000"));
    let mut a = args(&[]);
    a.compact_threshold = 1000;
    let body = Wire::default().body(&a, &[]).unwrap();
    assert!(body.contains("\"compact_threshold\":1000"));
}

#[test]
fn ac2_ordinary_steps_prefix_byte_for_byte() {
    let mut w = Wire::default();
    let a = args(&[]);
    let mut items = vec![
        wi(0, 0, None, user_message("hi")),
        wi(
            1,
            1,
            Some(M),
            json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"yo"}]}),
        ),
        wi(2, 1, Some(M), reasoning(Some("e1"))),
        wi(3, 1, Some(M), call("a")),
        wi(4, 1, None, out("a", "ok")),
        wi(5, 1, None, user_message("again")),
    ];
    let b1 = w.body(&a, &items).unwrap();
    items.push(wi(
        6,
        2,
        Some(M),
        json!({"type":"message","role":"assistant","content":[]}),
    ));
    let b2 = w.body(&a, &items).unwrap();
    let i1 = serde_json::to_string(&input_of(&b1)).unwrap();
    let i2 = serde_json::to_string(&input_of(&b2)).unwrap();
    assert!(i2.starts_with(i1.trim_end_matches(']')));
    let ins = |b: &str| {
        let v: Value = serde_json::from_str(b).unwrap();
        v["instructions"].clone()
    };
    assert_eq!(ins(&b1), ins(&b2));
}

/// Steps 1..=n, each: reasoning (encrypted, `r` bytes), a call, a big output (`o` bytes).
fn history(steps: u64, r: usize, o: usize) -> Vec<WireItem> {
    let mut v = vec![wi(0, 0, None, user_message("go"))];
    let mut i = 1;
    for s in 1..=steps {
        let c = format!("c{s}");
        for item in [reasoning(Some(&text(r))), call(&c), out(&c, &text(o))] {
            let model = if is_output(&item) { None } else { Some(M) };
            v.push(wi(i, s, model, item));
            i += 1;
        }
    }
    v
}

fn is_output(v: &Value) -> bool {
    v["type"] == "function_call_output"
}

#[test]
fn ac3_trim_reasoning_then_outputs_and_keep_the_mark() {
    let a = args(&[]);
    let mut w = Wire::default();
    // 12 steps of 100 KiB reasoning: ~1.2 MiB; dropping reasoning before the mark is enough.
    let items = history(12, 100 * 1024, 100);
    let b = w.body(&a, &items).unwrap();
    let t = w.trim.clone().unwrap();
    assert_eq!(t.stage, TrimStage::Reasoning);
    // Mark: the first item of the last 4 steps (steps 9..=12 start at item 1 + 8*3).
    assert_eq!(t.mark, 25);
    assert!(b.len() <= BODY_TRIM_TO, "{}", b.len());
    assert!(b.len() > BODY_TRIM_TO - 120 * 1024);

    // Three small steps: the mark stays, each body extends the one before.
    let mut items = items;
    let mut prev = serde_json::to_string(&input_of(&b)).unwrap();
    for k in 0..3u64 {
        let s = 13 + k;
        let n = items.len() as u64;
        let c = format!("c{s}");
        items.push(wi(n, s, Some(M), call(&c)));
        items.push(wi(n + 1, s, None, out(&c, "ok")));
        let b = w.body(&a, &items).unwrap();
        assert_eq!(w.trim.as_ref().unwrap().mark, 25);
        let cur = serde_json::to_string(&input_of(&b)).unwrap();
        assert!(cur.starts_with(prev.trim_end_matches(']')), "step {s}");
        prev = cur;
    }
}

#[test]
fn ac3_trim_cuts_outputs_with_marker() {
    let a = args(&[]);
    let mut w = Wire::default();
    // 12 steps with 100 KiB outputs and no reasoning: outputs must be cut.
    let items = history(12, 1, 100 * 1024);
    let b = w.body(&a, &items).unwrap();
    let t = w.trim.clone().unwrap();
    assert_eq!(t.stage, TrimStage::Outputs);
    assert!(b.len() <= BODY_TRIM_TO);
    let inp = input_of(&b);
    let first_out = inp.iter().find(|v| is_output(v)).unwrap();
    let s = first_out["output"].as_str().unwrap();
    assert!(s.ends_with(&format!(" [cut by watane: {} bytes]", 100 * 1024)));
    assert_eq!(
        s.len(),
        8 * 1024 + format!(" [cut by watane: {} bytes]", 100 * 1024).len()
    );
    // The last 4 steps are whole.
    let last = inp.iter().rev().find(|v| is_output(v)).unwrap();
    assert_eq!(last["output"].as_str().unwrap().len(), 100 * 1024);
}

#[test]
fn ac3_mark_moves_only_when_a_body_passes_trim_at_again() {
    let a = args(&[]);
    let mut w = Wire::default();
    let mut items = history(12, 100 * 1024, 100);
    w.body(&a, &items).unwrap();
    assert_eq!(w.trim.as_ref().unwrap().mark, 25);
    // Big new steps push the body over 896 KiB again: the mark moves on.
    for s in 13..=16u64 {
        let n = items.len() as u64;
        items.push(wi(n, s, Some(M), reasoning(Some(&text(100 * 1024)))));
        items.push(wi(
            n + 1,
            s,
            Some(M),
            json!({"type":"message","role":"assistant","content":[]}),
        ));
    }
    let b = w.body(&a, &items).unwrap();
    assert!(w.trim.as_ref().unwrap().mark > 25);
    assert!(b.len() <= BODY_MAX);
}

#[test]
fn ac3_still_over_one_mib_is_body_too_large() {
    let a = args(&[]);
    let mut w = Wire::default();
    // Four steps of 300 KiB: the last 4 steps alone pass 1 MiB.
    let items = history(5, 1, 300 * 1024);
    match w.body(&a, &items) {
        Err(BodyTooLarge(n)) => assert!(n > BODY_MAX),
        Ok(b) => panic!("sent {}", b.len()),
    }
}

#[test]
fn ac4_compaction_moves_active_deletes_chunks_and_reads_few() {
    let mut s = st();
    let mut w = Wire::default();
    // Each item ~20 KiB: three per chunk.
    for k in 0..12u64 {
        w.append(
            &mut s,
            "x",
            k,
            Some(M),
            vec![json!({"type":"message","role":"user","content":text(20_000)})],
        )
        .unwrap();
    }
    assert!(w.wire_chunks.len() >= 4);
    let before = s.len();
    w.append(
        &mut s,
        "x",
        12,
        Some(M),
        vec![json!({"type":"compaction","encrypted_content":"c"})],
    )
    .unwrap();
    assert_eq!(w.active, 12);
    let live = w.wire_chunks.clone();
    assert!(live.len() < before);
    assert_eq!(s.len(), live.len(), "only live chunks stay: {:?}", live);
    assert!(live[0] <= 12);
    // A request reads only the chunk holding `active` (and later ones).
    s.reset_counts();
    let items = w.active_items(&mut s, "x");
    assert_eq!(items.first().unwrap().i, 12);
    assert!(s.get_keys().iter().all(|k| {
        let n: u64 = k.rsplit('/').next().unwrap().parse().unwrap();
        n >= *live.iter().rfind(|&&f| f <= 12).unwrap()
    }));
    let body = w.body(&args(&[]), &items).unwrap();
    assert_eq!(input_of(&body)[0]["type"], "compaction");
}

#[test]
fn ac5_restart_from_summary() {
    let mut s = st();
    let mut w = Wire::default();
    for k in 0..4u64 {
        w.append(&mut s, "x", k, Some(M), vec![user_message(&text(30_000))])
            .unwrap();
    }
    w.steps = 4;
    let a = args(&[]);
    let b0 = w.body(&a, &w.active_items(&mut s, "x")).unwrap();
    w.restart_from_summary(&mut s, "x", "the summary").unwrap();
    assert_eq!(w.active, 4);
    let items = w.active_items(&mut s, "x");
    assert_eq!(items.len(), 1);
    let b = w.body(&a, &items).unwrap();
    assert_eq!(input_of(&b), vec![user_message("the summary")]);
    let pre = |b: &str| b[..b.find("\"input\"").unwrap()].to_string();
    assert_eq!(pre(&b0), pre(&b));
    assert!(w.trim.is_none());
}

#[test]
fn ac6_chunks_hold_64k_or_one_big_item() {
    let mut s = st();
    let mut w = Wire::default();
    let mk = |n: usize| json!({"type":"message","role":"user","content":text(n)});
    w.append(
        &mut s,
        "x",
        0,
        None,
        vec![mk(30_000), mk(30_000), mk(30_000)],
    )
    .unwrap();
    assert_eq!(w.wire_chunks, vec![0, 2]);
    // An append rewrites at most the last chunk plus the new items.
    s.reset_counts();
    w.append(&mut s, "x", 1, None, vec![mk(10)]).unwrap();
    assert_eq!(s.counts().gets, 1);
    assert_eq!(s.counts().puts, 1);
    // A big item stands alone.
    w.append(&mut s, "x", 1, None, vec![mk(100_000), mk(10)])
        .unwrap();
    assert_eq!(w.wire_chunks, vec![0, 2, 4, 5]);
    let all = w.active_items(&mut s, "x");
    assert_eq!(
        all.iter().map(|x| x.i).collect::<Vec<_>>(),
        (0..6).collect::<Vec<_>>()
    );
    for &f in &w.wire_chunks {
        let raw = s.get(&format!("wire/x/{f}")).unwrap();
        let items: Vec<WireItem> = serde_json::from_slice(&raw).unwrap();
        let n: usize = items
            .iter()
            .map(|i| serde_json::to_string(i).unwrap().len())
            .sum();
        assert!(n <= 64 * 1024 || items.len() == 1);
    }
    w.delete_all(&mut s, "x");
    assert!(s.is_empty());
    assert_eq!(w, Wire::default());
}

#[test]
fn wire_state_serialises_kebab_case() {
    let w = Wire {
        wire_chunks: vec![0],
        trim: Some(Trim {
            mark: 1,
            through: 1,
            stage: TrimStage::Outputs,
        }),
        ..Wire::default()
    };
    let v = serde_json::to_value(&w).unwrap();
    assert!(v.get("wire-chunks").is_some());
    assert_eq!(v["trim"]["stage"], "outputs");
}
