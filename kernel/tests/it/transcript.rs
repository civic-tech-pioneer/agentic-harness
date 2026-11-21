//! The transcript over host state: TECHNICAL 9.1, 9.4, 16 (cases 34, 41, 49, 55).

use agent_kernel::state::{MemState, State, TooLarge, MAX_VALUE};
use agent_kernel::transcript::{chunk_key, Joiner, Meta, NewEntry, Replay, Transcript};
use agent_kernel::KernelError;
use serde_json::{json, Value};

fn entry(body: Value) -> NewEntry {
    NewEntry {
        kind: "message".into(),
        uuid: None,
        body,
    }
}

fn text_entry(n: usize) -> NewEntry {
    entry(json!({ "text": "x".repeat(n) }))
}

fn fill(st: &mut MemState, meta: &mut Meta, count: usize, size: usize) {
    let mut t = Transcript::new(st, "s", meta);
    for k in 0..count {
        t.append(&text_entry(size), 1000 + k as u64).unwrap();
    }
}

fn chunk_lines(st: &mut MemState, n: usize) -> Vec<String> {
    let v = st.get(&chunk_key("s", n)).expect("chunk");
    String::from_utf8(v)
        .unwrap()
        .split('\n')
        .map(String::from)
        .collect()
}

#[test]
fn ac1_state_limits_and_counts() {
    let mut st = MemState::new();
    assert_eq!(
        st.put("a", vec![0; MAX_VALUE + 1]),
        Err(TooLarge(4_194_304))
    );
    assert_eq!(st.len(), 0);
    assert!(st.put("a", vec![0; MAX_VALUE]).is_ok());
    assert_eq!(st.get("missing"), None);
    st.delete("missing");
    st.reset_counts();
    st.put("b", vec![1; 5]).unwrap();
    st.get("b");
    st.get("zz");
    st.delete("b");
    let c = st.counts();
    assert_eq!((c.gets, c.puts, c.deletes), (2, 1, 1));
    assert_eq!((c.bytes_read, c.bytes_written), (5, 5));
    assert_eq!(st.get_keys(), ["b", "zz"]);
}

#[test]
fn ac2_append_indexes_and_line_shape() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    for k in 1..=10u64 {
        let mut e = entry(json!({"n": k}));
        if k == 3 {
            e.uuid = Some("u-3".into());
        }
        assert_eq!(t.append(&e, 50 + k).unwrap(), k);
    }
    assert_eq!(meta.last, 10);
    let lines = chunk_lines(&mut st, 0);
    assert_eq!(lines.len(), 10);
    assert_eq!(
        lines[0],
        r#"{"v":1,"i":1,"kind":"message","at":51,"body":{"n":1}}"#
    );
    assert_eq!(
        lines[2],
        r#"{"v":1,"i":3,"kind":"message","at":53,"uuid":"u-3","body":{"n":3}}"#
    );
}

fn check_chunks(st: &mut MemState, meta: &Meta) {
    assert!(meta.chunks.windows(2).all(|w| w[0] < w[1]));
    let mut next = 1;
    for (n, first) in meta.chunks.iter().enumerate() {
        assert_eq!(*first, next);
        let lines = chunk_lines(st, n);
        let bytes = lines.join("\n").len();
        assert!(
            (lines.len() <= 256 && bytes <= 64 * 1024) || lines.len() == 1,
            "chunk {n}: {} entries, {bytes} bytes",
            lines.len()
        );
        next += lines.len() as u64;
    }
    assert_eq!(next, meta.last + 1);
}

#[test]
fn ac3_chunk_rule() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    fill(&mut st, &mut meta, 300, 10);
    check_chunks(&mut st, &meta);
    assert_eq!(meta.chunks, vec![1, 257]);
    assert_eq!(chunk_lines(&mut st, 1).len(), 44);

    let mut st = MemState::new();
    let mut meta = Meta::default();
    fill(&mut st, &mut meta, 3, 10);
    Transcript::new(&mut st, "s", &mut meta)
        .append(&text_entry(100 * 1024), 1)
        .unwrap();
    Transcript::new(&mut st, "s", &mut meta)
        .append(&text_entry(10), 1)
        .unwrap();
    Transcript::new(&mut st, "s", &mut meta)
        .append(&text_entry(10), 1)
        .unwrap();
    assert_eq!(meta.chunks, vec![1, 4, 5]);
    check_chunks(&mut st, &meta);

    // A mixed sequence keeps the rule.
    let mut st = MemState::new();
    let mut meta = Meta::default();
    for k in 0..200usize {
        let size = [10, 5_000, 30_000, 70_000, 200][k % 5];
        Transcript::new(&mut st, "s", &mut meta)
            .append(&text_entry(size), 1)
            .unwrap();
    }
    check_chunks(&mut st, &meta);
}

#[test]
fn ac4_append_cost() {
    let (mut cost, mut avg) = (Vec::new(), Vec::new());
    for n in [10usize, 10_000] {
        let mut st = MemState::new();
        let mut meta = Meta::default();
        fill(&mut st, &mut meta, n, 4096);
        let before: Vec<Vec<u8>> = (0..meta.chunks.len() - 1)
            .map(|c| st.get(&chunk_key("s", c)).unwrap())
            .collect();
        st.reset_counts();
        Transcript::new(&mut st, "s", &mut meta)
            .append(&text_entry(4096), 1)
            .unwrap();
        let c = st.counts();
        assert_eq!(c.puts, 1);
        assert!(c.gets <= 1);
        assert!(c.bytes_written <= 64 * 1024 + 4300);
        cost.push((c.bytes_read + c.bytes_written) as f64);
        // Over a whole chunk's worth of appends (15 fit), the cost does not hang on the fill.
        st.reset_counts();
        fill(&mut st, &mut meta, 15, 4096);
        let c = st.counts();
        assert!(c.bytes_read <= 15 * 64 * 1024);
        avg.push((c.bytes_read + c.bytes_written) as f64);
        if n == 10_000 {
            fill(&mut st, &mut meta, 1000, 4096);
            for (c, v) in before.iter().enumerate() {
                assert_eq!(
                    &st.get(&chunk_key("s", c)).unwrap(),
                    v,
                    "chunk {c} rewritten"
                );
            }
        }
    }
    // 10 and 10 000 entries both leave 10 entries in the last chunk (15 fit per chunk).
    let ratio = cost[0] / cost[1];
    assert!((0.9..=1.1).contains(&ratio), "ratio {ratio}");
    let ratio = avg[0] / avg[1];
    assert!((0.9..=1.1).contains(&ratio), "average ratio {ratio}");
}

fn joined(lines: &[String]) -> String {
    let mut text = String::new();
    for (k, l) in lines.iter().enumerate() {
        let v: Value = serde_json::from_str(l).unwrap();
        assert!(l.len() <= 262_144);
        assert_eq!(v["kind"], "part");
        assert_eq!(v["body"]["n"], k as u64 + 1);
        assert_eq!(v["body"]["m"], lines.len() as u64);
        text.push_str(v["body"]["text"].as_str().unwrap());
    }
    text
}

#[test]
fn ac5_parts() {
    let big = 600 * 1024;
    let texts = [
        "a".repeat(big),
        "é😀".repeat(big / 6),
        "\"\n\u{1}\\\t".repeat(big / 5),
    ];
    for text in texts {
        let mut st = MemState::new();
        let mut meta = Meta::default();
        fill(&mut st, &mut meta, 2, 5);
        let e = NewEntry {
            kind: "message".into(),
            uuid: Some("u".into()),
            body: json!({ "text": text }),
        };
        let i = Transcript::new(&mut st, "s", &mut meta)
            .append(&e, 77)
            .unwrap();
        assert!(i > 3);
        assert_eq!(meta.last, i);
        let mut lines = Vec::new();
        let mut n = 0;
        while let Some(v) = st.get(&chunk_key("s", n)) {
            lines.extend(String::from_utf8(v).unwrap().split('\n').map(String::from));
            n += 1;
        }
        let parts = &lines[2..];
        assert_eq!(parts.len() as u64, i - 2);
        let whole = joined(parts);
        let expect = format!(
            r#"{{"v":1,"i":{i},"kind":"message","at":77,"uuid":"u","body":{}}}"#,
            serde_json::to_string(&e.body).unwrap()
        );
        assert_eq!(whole, expect);
        let v: Value = serde_json::from_str(parts.last().unwrap()).unwrap();
        assert_eq!(v["i"], i);
    }
}

#[test]
fn ac5_exact_limit_is_not_split() {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    let base = r#"{"v":1,"i":1,"kind":"message","at":1,"body":{"text":""}}"#.len();
    let e = text_entry(262_144 - base);
    Transcript::new(&mut st, "s", &mut meta)
        .append(&e, 1)
        .unwrap();
    assert_eq!(meta.last, 1);
    assert_eq!(chunk_lines(&mut st, 0)[0].len(), 262_144);
    Transcript::new(&mut st, "s", &mut meta)
        .append(&text_entry(262_144 - base + 1), 1)
        .unwrap();
    assert_eq!(meta.last, 3);
}

#[test]
fn ac5_part_count_crossing_a_digit_keeps_the_whole_index() {
    // At index 93, 8 parts would end at 100 (one digit more), which needs a 9th part:
    // the whole entry must carry the index of the part that really is last.
    let overhead = r#"{"v":1,"i":18446744073709551615,"kind":"part","at":1,"body":{"n":18446744073709551615,"m":18446744073709551615,"text":""}}"#.len();
    // Escaped length of the whole at i = 93 with an empty text: its 16 quotes double.
    let whole = r#"{"v":1,"i":93,"kind":"message","at":1,"body":{"text":""}}"#.len() + 16;
    let mut st = MemState::new();
    let mut meta = Meta::default();
    fill(&mut st, &mut meta, 92, 1);
    let i = Transcript::new(&mut st, "s", &mut meta)
        .append(&text_entry(8 * (262_144 - overhead) - whole), 1)
        .unwrap();
    assert_eq!(i, 101);
    let p = Transcript::new(&mut st, "s", &mut meta)
        .read(92, None, 256)
        .unwrap();
    let mut j = Joiner::new();
    let got: Vec<Value> = p.entries.iter().filter_map(|l| j.push(l)).collect();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["i"], i);
}

fn many(count: usize, size: usize) -> (MemState, Meta) {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    fill(&mut st, &mut meta, count, size);
    (st, meta)
}

fn idx(l: &str) -> u64 {
    serde_json::from_str::<Value>(l).unwrap()["i"]
        .as_u64()
        .unwrap()
}

#[test]
fn ac6_paging_both_ways() {
    let (mut st, mut meta) = many(10_000, 10);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let mut seen = Vec::new();
    let mut after = 0;
    loop {
        let p = t.read(after, None, 256).unwrap();
        assert_eq!(p.last, 10_000);
        if p.entries.is_empty() {
            break;
        }
        assert!(p.entries.len() <= 256);
        after = idx(p.entries.last().unwrap());
        seen.extend(p.entries.iter().map(|l| idx(l)));
    }
    assert_eq!(seen, (1..=10_000).collect::<Vec<_>>());
    let mut seen = Vec::new();
    let mut before = 10_001;
    loop {
        let p = t.read(0, Some(before), 256).unwrap();
        if p.entries.is_empty() {
            break;
        }
        before = idx(&p.entries[0]);
        let mut ids: Vec<u64> = p.entries.iter().map(|l| idx(l)).collect();
        assert!(ids.windows(2).all(|w| w[1] == w[0] + 1));
        ids.extend(seen);
        seen = ids;
    }
    assert_eq!(seen, (1..=10_000).collect::<Vec<_>>());
    // max 0 is refused and reads no chunk, over 256 is 256; empty transcript has last 0.
    drop(t);
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let Err(KernelError::Refused(r)) = t.read(0, None, 0) else {
        panic!("max 0 must be refused")
    };
    assert_eq!(r[0].field, "max");
    drop(t);
    assert_eq!(st.counts().gets, 0, "max 0 reads no chunk");
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(0, None, 1).unwrap().entries.len(), 1);
    assert_eq!(t.read(0, None, 9999).unwrap().entries.len(), 256);
    let mut st = MemState::new();
    let mut m = Meta::default();
    let p = Transcript::new(&mut st, "e", &mut m)
        .read(0, None, 10)
        .unwrap();
    assert_eq!((p.entries.len(), p.last), (0, 0));
}

#[test]
fn ac6_page_byte_limit_still_progresses() {
    let (mut st, mut meta) = many(20, 100 * 1024);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(0, None, 256).unwrap();
    assert_eq!(p.entries.len(), 20);
    let (mut st, mut meta) = many(12, 250 * 1024);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(0, None, 256).unwrap();
    assert_eq!(p.entries.len(), 8);
    let p = t.read(0, Some(13), 256).unwrap();
    assert_eq!(idx(p.entries.last().unwrap()), 12);
    assert_eq!(p.entries.len(), 8);
}

fn parts_transcript() -> (MemState, Meta, u64) {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    fill(&mut st, &mut meta, 2, 5);
    let e = entry(json!({ "text": "z".repeat(700_000) }));
    let i = Transcript::new(&mut st, "s", &mut meta)
        .append(&e, 1)
        .unwrap();
    fill(&mut st, &mut meta, 2, 5);
    (st, meta, i)
}

#[test]
fn ac7_after_inside_group_moves_to_group_start() {
    let (mut st, mut meta, i) = parts_transcript();
    assert_eq!(i, 5); // parts 3, 4, 5
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    for after in [3, 4] {
        let p = t.read(after, None, 256).unwrap();
        assert_eq!(idx(&p.entries[0]), after + 1, "after {after}");
        let p = t.read(after, Some(8), 256).unwrap();
        assert_eq!(idx(&p.entries[0]), after + 1, "before read, after {after}");
    }
    // after on the last part, or before the group: unchanged.
    assert_eq!(idx(&t.read(5, None, 256).unwrap().entries[0]), 6);
    assert_eq!(idx(&t.read(2, None, 256).unwrap().entries[0]), 3);
    // A page may end inside a group.
    let p = t.read(0, None, 3).unwrap();
    assert_eq!(idx(p.entries.last().unwrap()), 3);
    // replay moves; read does not.
    let Replay::Entries(e) = t.replay(4) else {
        panic!()
    };
    assert_eq!(idx(&e[0]), 3);
}

fn ac2_check(max: u32) {
    let mut st = MemState::new();
    let mut meta = Meta::default();
    fill(&mut st, &mut meta, 3, 5);
    let big = entry(json!({ "text": "q".repeat(1024 * 1024) }));
    let want = serde_json::to_string(&big.body).unwrap();
    let bi = Transcript::new(&mut st, "s", &mut meta)
        .append(&big, 1)
        .unwrap();
    assert!(bi - 3 >= 4, "{} parts", bi - 3);
    fill(&mut st, &mut meta, 3, 5);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let last = t.read(0, None, 1).unwrap().last;
    // forwards
    let (mut after, mut seen, mut j, mut joined) = (0u64, Vec::new(), Joiner::new(), Vec::new());
    loop {
        let p = t.read(after, None, max).unwrap();
        let Some(l) = p.entries.last() else { break };
        assert!(idx(l) > after, "max {max}: stalled at after {after}");
        after = idx(l);
        for l in &p.entries {
            seen.push(idx(l));
            joined.extend(j.push(l));
        }
    }
    assert_eq!(seen, (1..=last).collect::<Vec<_>>(), "max {max}");
    let b: Vec<&Value> = joined.iter().filter(|v| v["i"] == bi).collect();
    assert_eq!(b.len(), 1);
    assert_eq!(serde_json::to_string(&b[0]["body"]).unwrap(), want);
    // backwards
    let (mut before, mut seen, mut lines) = (last + 1, Vec::new(), Vec::new());
    while before > 1 {
        let p = t.read(0, Some(before), max).unwrap();
        assert!(!p.entries.is_empty(), "max {max}");
        assert!(
            idx(&p.entries[0]) < before,
            "max {max}: stalled at before {before}"
        );
        before = idx(&p.entries[0]);
        for l in p.entries.iter().rev() {
            seen.push(idx(l));
            lines.push(l.clone());
        }
    }
    seen.reverse();
    lines.reverse();
    assert_eq!(seen, (1..=last).collect::<Vec<_>>(), "back max {max}");
    let mut j = Joiner::new();
    let out: Vec<Value> = lines.iter().filter_map(|l| j.push(l)).collect();
    let b: Vec<&Value> = out.iter().filter(|v| v["i"] == bi).collect();
    assert_eq!(b.len(), 1);
    assert_eq!(serde_json::to_string(&b[0]["body"]).unwrap(), want);
}

#[test]
fn ac2_paging_through_a_big_entry_progresses() {
    for max in [1, 3, 256] {
        ac2_check(max);
    }
}

#[test]
fn ac8_chunk_reads() {
    let (mut st, mut meta) = many(10_000, 100);
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(9_990, None, 5).unwrap();
    assert_eq!(p.entries.len(), 5);
    assert!(st.get_keys().len() <= 2, "{:?}", st.get_keys());
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(3_000, None, 256).unwrap().entries.len(), 256);
    assert!(st.get_keys().len() <= 3, "{:?}", st.get_keys());
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(0, Some(5_001), 10).unwrap().entries.len(), 10);
    assert!(st.get_keys().len() <= 2, "{:?}", st.get_keys());
    let n = meta.chunks.len();
    let valid: Vec<String> = (0..n).map(|c| chunk_key("s", c)).collect();
    assert!(st.get_keys().iter().all(|k| valid.contains(k)));
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    t.read(0, None, 256).unwrap();
    assert!(st.counts().bytes_read <= 2 * 1024 * 1024 + 320 * 1024);
}

#[test]
fn ac9_replay_decision() {
    let (mut st, mut meta) = many(128, 10);
    let Replay::Entries(e) = Transcript::new(&mut st, "s", &mut meta).replay(0) else {
        panic!()
    };
    assert_eq!(e.len(), 128);
    let (mut st, mut meta) = many(129, 10);
    st.reset_counts();
    assert_eq!(
        Transcript::new(&mut st, "s", &mut meta).replay(0),
        Replay::Behind { through: 129 }
    );
    assert_eq!(st.counts().gets, 0);
    let (mut st, mut meta) = many(10, 250 * 1024);
    assert_eq!(
        Transcript::new(&mut st, "s", &mut meta).replay(0),
        Replay::Behind { through: 10 }
    );
    let (mut st, mut meta) = many(5, 10);
    assert_eq!(
        Transcript::new(&mut st, "s", &mut meta).replay(5),
        Replay::Entries(vec![])
    );
}

#[test]
fn ac10_joiner() {
    let (mut st, mut meta, i) = parts_transcript();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let mut j = Joiner::new();
    let mut out = Vec::new();
    // Group split across pages read forwards.
    for page in [t.read(0, None, 3).unwrap(), t.read(3, None, 256).unwrap()] {
        for l in page.entries {
            out.extend(j.push(&l));
        }
    }
    let ids: Vec<u64> = out.iter().map(|v| v["i"].as_u64().unwrap()).collect();
    assert_eq!(ids, vec![1, 2, i, 6, 7]);
    let second = t.read(3, None, 256).unwrap();
    let n = serde_json::from_str::<Value>(&second.entries[0]).unwrap()["body"]["n"].as_u64();
    assert!(n > Some(1), "second page starts with part {n:?}");
    assert_eq!(out[2]["body"]["text"], "z".repeat(700_000));
    // S14-fix4 AC2: the 3-part whole is the stored entry, with the last part's i.
    let expect = json!({"v":1,"i":i,"kind":"message","at":1,"body":{"text":"z".repeat(700_000)}});
    assert_eq!(out[2], expect);
}

#[test]
fn ac10_joiner_drops_broken_groups() {
    let part = |n: u32, m: u32, text: &str| {
        format!(
            r#"{{"v":1,"i":{n},"kind":"part","at":1,"body":{{"n":{n},"m":{m},"text":"{text}"}}}}"#
        )
    };
    let mut j = Joiner::new();
    assert!(j.push(&part(1, 3, "{\\\"a")).is_none());
    assert!(j.push(&part(3, 3, "x")).is_none()); // skipped n=2
    assert!(j.push(&part(2, 3, "x")).is_none()); // group already dropped
    assert!(j.push(&part(1, 2, "{\\\"a\\\":")).is_none());
    let msg = r#"{"v":1,"i":9,"kind":"turn","at":1,"body":{}}"#;
    assert_eq!(j.push(msg).unwrap()["i"], 9); // non-part mid-group drops it
    assert!(j.push(&part(2, 2, "1}")).is_none());
    assert!(j.push("not json").is_none());
    assert!(j.push(&part(1, 2, "{\\\"a\\\":")).is_none());
    assert_eq!(
        j.push(&part(2, 2, "1}")).unwrap(),
        json!({"v":1,"i":2,"kind":"unreadable","raw":"{\"a\":1}"})
    );
}

#[test]
fn ac11_unreadable() {
    let (mut st, mut meta) = many(1, 5);
    let lines = [
        r#"{"v":1,"i":2,"kind":"mystery","at":1,"body":{}}"#,
        r#"{"v":2,"i":3,"kind":"message","at":1,"body":{}}"#,
        "plain text, not json",
        r#"{"v":1,"kind":"nope"}"#,
    ];
    let mut all = chunk_lines(&mut st, 0);
    all.extend(lines.iter().map(|s| s.to_string()));
    st.put(&chunk_key("s", 0), all.join("\n").into_bytes())
        .unwrap();
    meta.last = 5;
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(0, None, 10).unwrap();
    assert_eq!(p.entries.len(), 5);
    for (k, l) in lines.iter().enumerate() {
        let v: Value = serde_json::from_str(&p.entries[k + 1]).unwrap();
        assert_eq!(v["kind"], "unreadable");
        assert_eq!(v["v"], if k == 1 { 2 } else { 1 });
        assert_eq!(v["i"], k as u64 + 2);
        assert_eq!(v["raw"], *l);
        assert_eq!(v.get("at").is_some(), k < 2, "at only when a number");
    }
    let Replay::Entries(e) = t.replay(1) else {
        panic!()
    };
    assert_eq!(e, p.entries[1..].to_vec());
}

#[test]
fn ac12_limits() {
    let mut st = MemState::new();
    let mut meta = Meta {
        chunks: (0..16_384).map(|k| 1 + k as u64 * 256).collect(),
        last: 16_384 * 256,
    };
    let before = meta.clone();
    assert_eq!(
        Transcript::new(&mut st, "s", &mut meta).append(&text_entry(5), 1),
        Err(KernelError::Quota)
    );
    assert_eq!(meta, before);
    assert_eq!(st.len(), 0);
    // A missing chunk key reads as no entries; nothing panics.
    let mut meta = Meta {
        chunks: vec![1, 257],
        last: 300,
    };
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(0, None, 10).unwrap().entries.len(), 0);
    assert_eq!(t.read(0, Some(300), 10).unwrap().entries.len(), 0);
    let _ = t.replay(290);
}

#[test]
fn ac13_behind_text() {
    let e = KernelError::Behind(310);
    assert_eq!(e.to_string(), "behind: 310 entries stored, page with read");
    assert_eq!(e.code(), "behind");
}

#[test]
fn ac1_unreadable_keeps_its_position_while_paging() {
    // Lines claiming a wrong `i` at positions 1, the middle and the last of a chunk.
    let (mut st, mut meta) = many(10, 5);
    let mut lines = chunk_lines(&mut st, 0);
    lines[0] = r#"{"v":1,"i":7,"kind":"mystery","at":4}"#.into();
    lines[4] = r#"{"v":3,"i":99999,"kind":"message","at":5,"body":{}}"#.into();
    lines[9] = r#"{"v":1,"i":7,"kind":"nope"}"#.into();
    st.put(&chunk_key("s", 0), lines.join("\n").into_bytes())
        .unwrap();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let want: Vec<u64> = (1..=10).collect();
    for max in [1, 3, 256] {
        let (mut after, mut seen) = (0, Vec::new());
        loop {
            let p = t.read(after, None, max).unwrap();
            let Some(l) = p.entries.last() else { break };
            after = idx(l);
            seen.extend(p.entries.iter().map(|l| idx(l)));
        }
        assert_eq!(seen, want, "max {max}");
        let (mut before, mut seen) = (11, Vec::new());
        while before > 1 {
            let p = t.read(0, Some(before), max).unwrap();
            before = idx(&p.entries[0]);
            seen.splice(0..0, p.entries.iter().map(|l| idx(l)));
        }
        assert_eq!(seen, want, "back max {max}");
    }
    let all = t.read(0, None, 256).unwrap().entries;
    let v: Value = serde_json::from_str(&all[4]).unwrap();
    assert_eq!((v["v"].clone(), v["i"].clone()), (json!(3), json!(5)));
    assert_eq!(v["at"], 5);
}

#[test]
fn ac2_append_takes_only_whole_entry_kinds() {
    let (mut st, mut meta) = many(2, 5);
    for k in [
        "message",
        "tool",
        "ask",
        "allow",
        "verdict",
        "turn",
        "compaction",
        "unknown",
    ] {
        let mut e = text_entry(3);
        e.kind = k.into();
        Transcript::new(&mut st, "s", &mut meta)
            .append(&e, 1)
            .unwrap();
    }
    let before = (meta.clone(), st.get(&chunk_key("s", 0)));
    let puts = st.counts().puts;
    for k in ["part", "unreadable", "", "Message", "bogus"] {
        let mut e = text_entry(3);
        e.kind = k.into();
        let Err(KernelError::Refused(r)) = Transcript::new(&mut st, "s", &mut meta).append(&e, 1)
        else {
            panic!("{k:?} must be refused")
        };
        assert_eq!((r[0].field.as_str(), r.len()), ("kind", 1));
        assert_eq!((meta.clone(), st.get(&chunk_key("s", 0))), before, "{k:?}");
        assert_eq!(st.counts().puts, puts, "{k:?} writes no state");
    }
}

#[test]
fn ac1_readable_line_takes_the_i_of_its_place() {
    let (mut st, mut meta) = many(10, 5);
    let mut lines = chunk_lines(&mut st, 0);
    // Spaced, so a re-serialization of a line whose i is right would show.
    lines[3] = r#"{"v":1, "i":4, "kind":"message", "at":3, "body":{}}"#.into();
    let good = lines[3].clone();
    lines[7] = r#"{"v":1,"i":3,"kind":"part","at":8,"body":{"n":1,"m":2,"text":"w"}}"#.into();
    lines[0] = r#"{"v":1,"i":7,"kind":"message","at":4,"uuid":"u","body":{"a":[1,2]}}"#.into();
    lines[4] = r#"{"v":1,"i":99999,"kind":"message","at":5,"body":{}}"#.into();
    lines[5] = r#"{"v":1,"i":"x","kind":"tool","at":6,"body":{}}"#.into();
    lines[9] = r#"{"v":1,"kind":"part","at":7,"body":{"n":1,"m":2,"text":"t"}}"#.into();
    st.put(&chunk_key("s", 0), lines.join("\n").into_bytes())
        .unwrap();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    for max in [1, 3, 256] {
        let (mut after, mut seen) = (0, Vec::new());
        loop {
            let p = t.read(after, None, max).unwrap();
            let Some(l) = p.entries.last() else { break };
            after = idx(l);
            seen.extend(p.entries.iter().map(|l| idx(l)));
        }
        assert_eq!(seen, (1..=10).collect::<Vec<u64>>(), "max {max}");
        let (mut before, mut seen) = (11, Vec::new());
        while before > 1 {
            let p = t.read(0, Some(before), max).unwrap();
            before = idx(&p.entries[0]);
            seen.splice(0..0, p.entries.iter().map(|l| idx(l)));
        }
        assert_eq!(seen, (1..=10).collect::<Vec<u64>>(), "back max {max}");
    }
    let all = t.read(0, None, 256).unwrap().entries;
    let v: Value = serde_json::from_str(&all[0]).unwrap();
    let want = json!({"v":1,"i":1,"kind":"message","at":4,"uuid":"u","body":{"a":[1,2]}});
    assert_eq!(v, want);
    assert_eq!(idx(&all[4]), 5);
    assert_eq!(idx(&all[5]), 6);
    assert_eq!(idx(&all[7]), 8);
    assert_eq!(idx(&all[9]), 10);
    let p: Value = serde_json::from_str(&all[9]).unwrap();
    assert_eq!(p["body"]["text"], "t");
    assert_eq!(all[3], good, "a correct i is given byte for byte");
    let Replay::Entries(r) = t.replay(4) else {
        panic!("replay")
    };
    assert_eq!(
        r.iter().map(|l| idx(l)).collect::<Vec<_>>(),
        (5..=10).collect::<Vec<u64>>()
    );
}

#[test]
fn ac10_joiner_reads_non_object_wholes_as_unreadable_and_leaves_unindexed() {
    let part = |i: &str, n: u32, text: &str| {
        format!(r#"{{"v":1,{i}"kind":"part","at":1,"body":{{"n":{n},"m":2,"text":"{text}"}}}}"#)
    };
    let mut j = Joiner::new();
    // A whole that is not an object is an unreadable entry at the last part's index.
    assert!(j.push(&part(r#""i":1,"#, 1, "[1,")).is_none());
    assert_eq!(
        j.push(&part(r#""i":2,"#, 2, "2]")).unwrap(),
        json!({"v":1,"i":2,"kind":"unreadable","raw":"[1,2]"})
    );
    // A last part with no numeric i leaves the whole's own fields alone.
    assert!(j.push(&part(r#""i":3,"#, 1, "{\\\"i\\\":")).is_none());
    assert_eq!(
        j.push(&part(r#""i":"x","#, 2, "7}")).unwrap(),
        json!({"i": 7})
    );
    assert!(j.push(&part(r#""i":5,"#, 1, "{\\\"a\\\":")).is_none());
    assert_eq!(j.push(&part("", 2, "1}")).unwrap(), json!({"a": 1}));
}

#[test]
fn ac10_joiner_non_json_whole_is_unreadable() {
    let part = |i: u32, n: u32, text: &str| {
        format!(
            r#"{{"v":1,"i":{i},"kind":"part","at":1,"body":{{"n":{n},"m":2,"text":"{text}"}}}}"#
        )
    };
    let mut j = Joiner::new();
    assert!(j.push(&part(4, 1, "{")).is_none());
    assert_eq!(
        j.push(&part(5, 2, "x")).unwrap(),
        json!({"v":1,"i":5,"kind":"unreadable","raw":"{x"})
    );
}
