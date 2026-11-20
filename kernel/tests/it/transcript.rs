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
    let mut cost = Vec::new();
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
    let big = 3 * 1024 * 1024;
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
        let p = t.read(after, None, 256);
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
        let p = t.read(0, Some(before), 256);
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
    // max 0 is 1, over 256 is 256; empty transcript has last 0.
    assert_eq!(t.read(0, None, 0).entries.len(), 1);
    assert_eq!(t.read(0, None, 9999).entries.len(), 256);
    let mut st = MemState::new();
    let mut m = Meta::default();
    let p = Transcript::new(&mut st, "e", &mut m).read(0, None, 10);
    assert_eq!((p.entries.len(), p.last), (0, 0));
}

#[test]
fn ac6_page_byte_limit_still_progresses() {
    let (mut st, mut meta) = many(20, 100 * 1024);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(0, None, 256);
    assert_eq!(p.entries.len(), 20);
    let (mut st, mut meta) = many(12, 250 * 1024);
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(0, None, 256);
    assert_eq!(p.entries.len(), 8);
    let p = t.read(0, Some(13), 256);
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
        let p = t.read(after, None, 256);
        assert_eq!(idx(&p.entries[0]), 3, "after {after}");
        let p = t.read(after, Some(8), 256);
        assert_eq!(idx(&p.entries[0]), 3, "before read, after {after}");
    }
    // after on the last part, or before the group: unchanged.
    assert_eq!(idx(&t.read(5, None, 256).entries[0]), 6);
    assert_eq!(idx(&t.read(2, None, 256).entries[0]), 3);
    // A page may end inside a group.
    let p = t.read(0, None, 3);
    assert_eq!(idx(p.entries.last().unwrap()), 3);
    // replay moves too.
    let Replay::Entries(e) = t.replay(4) else {
        panic!()
    };
    assert_eq!(idx(&e[0]), 3);
}

#[test]
fn ac8_chunk_reads() {
    let (mut st, mut meta) = many(10_000, 100);
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    let p = t.read(9_990, None, 5);
    assert_eq!(p.entries.len(), 5);
    assert!(st.get_keys().len() <= 2, "{:?}", st.get_keys());
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(3_000, None, 256).entries.len(), 256);
    assert!(st.get_keys().len() <= 3, "{:?}", st.get_keys());
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    assert_eq!(t.read(0, Some(5_001), 10).entries.len(), 10);
    assert!(st.get_keys().len() <= 2, "{:?}", st.get_keys());
    let n = meta.chunks.len();
    let valid: Vec<String> = (0..n).map(|c| chunk_key("s", c)).collect();
    assert!(st.get_keys().iter().all(|k| valid.contains(k)));
    st.reset_counts();
    let mut t = Transcript::new(&mut st, "s", &mut meta);
    t.read(0, None, 256);
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
    for page in [t.read(0, None, 3), t.read(3, None, 256)] {
        for l in page.entries {
            out.extend(j.push(&l));
        }
    }
    let ids: Vec<u64> = out.iter().map(|v| v["i"].as_u64().unwrap()).collect();
    assert_eq!(ids, vec![1, 2, i, 6, 7]);
    assert_eq!(out[2]["body"]["text"], "z".repeat(700_000));
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
    assert_eq!(j.push(&part(2, 2, "1}")).unwrap(), json!({"a": 1}));
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
    let p = t.read(0, None, 10);
    assert_eq!(p.entries.len(), 5);
    for (k, l) in lines.iter().enumerate() {
        let v: Value = serde_json::from_str(&p.entries[k + 1]).unwrap();
        assert_eq!(v["kind"], "unreadable");
        assert_eq!(v["v"], 1);
        assert_eq!(v["i"], k as u64 + 2);
        assert_eq!(v["raw"], *l);
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
    assert_eq!(t.read(0, None, 10).entries.len(), 0);
    assert_eq!(t.read(0, Some(300), 10).entries.len(), 0);
    let _ = t.replay(290);
}

#[test]
fn ac13_behind_text() {
    let e = KernelError::Behind(310);
    assert_eq!(e.to_string(), "behind: 310 entries stored, page with read");
    assert_eq!(e.code(), "behind");
}
