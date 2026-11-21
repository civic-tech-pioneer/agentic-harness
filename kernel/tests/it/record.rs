use agent_kernel::*;
use serde_json::json;

const LINES: [&str; 10] = [
    r#"{"state":"running","since":1763600000000,"activity":{"kind":"checking","on":"a1"}}"#,
    r#"{"state":"running","since":1763600000000,"activity":{"kind":"compacting"}}"#,
    r#"{"state":"running","since":1763600000000,"activity":{"kind":"consent","what":"x"}}"#,
    r#"{"state":"waiting","asks":["a1"],"expires-at":1763600060000}"#,
    r#"{"state":"paused","until":1763603600000}"#,
    r#"{"state":"idle","stop":{"kind":"completed"},"seconds":12,"tools":3,"tokens":5400}"#,
    r#"{"state":"idle","stop":{"kind":"stopped","limit":"quota"}}"#,
    r#"{"state":"idle","stop":{"kind":"failed","error":"boom"}}"#,
    r#"{"state":"unknown"}"#,
    r#"{"state":"resume-lost"}"#,
];

#[test]
fn ac1_turn_lines_round_trip_exactly() {
    for l in LINES {
        let t: Turn = serde_json::from_str(l).unwrap();
        assert_eq!(serde_json::to_string(&t).unwrap(), l);
        assert_eq!(
            t.to_json(),
            serde_json::from_str::<serde_json::Value>(l).unwrap()
        );
    }
}

#[test]
fn ac1_omitted_optionals_entry_body_and_active() {
    let w = Turn::Waiting {
        asks: vec![],
        expires_at: None,
    };
    assert_eq!(
        serde_json::to_string(&w).unwrap(),
        r#"{"state":"waiting","asks":[]}"#
    );
    assert_eq!(
        serde_json::to_string(&Turn::Paused { until: None }).unwrap(),
        r#"{"state":"paused"}"#
    );
    let i = Turn::Idle {
        stop: Stop::Interrupted,
        seconds: None,
        tools: None,
        tokens: None,
    };
    assert_eq!(
        serde_json::to_string(&i).unwrap(),
        r#"{"state":"idle","stop":{"kind":"interrupted"}}"#
    );
    let r: Turn = serde_json::from_str(LINES[0]).unwrap();
    assert_eq!(
        r.entry_body(),
        json!({"state":"running","since":1763600000000u64})
    );
    assert!(r.is_active());
    assert!(w.is_active());
    assert!(Turn::Paused { until: None }.is_active());
    assert!(!i.is_active());
    assert!(!Turn::Unknown.is_active());
    assert!(!Turn::ResumeLost.is_active());
}

fn rec() -> Record {
    Record {
        id: "s1".into(),
        ..Default::default()
    }
}

#[test]
fn ac2_record_round_trips_with_kebab_keys() {
    let mut st = MemState::new();
    let mut r = rec();
    r.claude_uuid = Some("u".into());
    r.active_page = Some(2);
    r.next_index = 5;
    r.counters.tool_errors_in_a_row = 3;
    r.open.push(OpenAsk {
        id: "a".into(),
        kind: "ask".into(),
        expires_at: None,
        call: String::new(),
        i: 4,
    });
    r.save(&mut st).unwrap();
    assert_eq!(Record::load(&mut st, "s1"), Some(r));
    let raw: serde_json::Value = serde_json::from_slice(&st.get("session/s1").unwrap()).unwrap();
    for k in ["claude-uuid", "active-page", "next-index"] {
        assert!(raw.get(k).is_some(), "{k}");
    }
    assert!(raw["counters"].get("tool-errors-in-a-row").is_some());
    assert_eq!(Record::load(&mut st, "nope"), None);
}

#[test]
fn ac2_over_512_kib_is_too_large_and_writes_nothing() {
    let mut st = MemState::new();
    let mut r = rec();
    r.title = "x".repeat(600_000);
    assert_eq!(r.save(&mut st), Err(KernelError::TooLarge(524_288)));
    assert_eq!(st.len(), 0);
    // Exactly 512 KiB is stored; one byte more is not.
    r.title = String::new();
    let base = serde_json::to_vec(&r).unwrap().len();
    r.title = "x".repeat(524_288 - base);
    assert!(r.save(&mut st).is_ok());
    r.title.push('x');
    assert_eq!(r.save(&mut st), Err(KernelError::TooLarge(524_288)));
}

#[test]
fn ac3_key_and_title_caps() {
    assert!(check_key(&"k".repeat(256)).is_ok());
    assert_eq!(check_key(&"k".repeat(257)), Err(KernelError::TooLarge(256)));
    assert!(check_title(&"k".repeat(256)).is_ok());
    assert_eq!(
        check_title(&"k".repeat(257)),
        Err(KernelError::TooLarge(256))
    );
}

fn allow(i: usize, reach: &str) -> agent_kernel::Allow {
    agent_kernel::Allow {
        id: i.to_string(),
        tool: String::new(),
        key: reach.to_string(),
    }
}

#[test]
fn ac3_allow_caps() {
    let mut r = rec();
    for i in 0..64 {
        r.add_allow(allow(i, "x")).unwrap();
    }
    assert_eq!(
        r.add_allow(allow(99, "x")),
        Err(KernelError::TooLarge(65_536))
    );
    assert_eq!(r.allows.len(), 64);

    let mut r = rec();
    assert_eq!(
        r.add_allow(allow(1, &"x".repeat(1100))),
        Err(KernelError::TooLarge(65_536))
    );
    assert!(r.allows.is_empty());

    // 1 KiB each: the 64-allow count and the 64 KiB total meet at the 64th.
    let mut r = rec();
    for _ in 0..64 {
        r.add_allow(allow(0, &"x".repeat(1022 - 27))).unwrap();
    }
    assert_eq!(
        r.add_allow(allow(0, "y")),
        Err(KernelError::TooLarge(65_536))
    );
    assert_eq!(r.allows.len(), 64);
}

#[test]
fn ac3_streams_deadlines_chunks() {
    let mut r = rec();
    for i in 0..10 {
        r.add_stream(format!("s{i}")).unwrap();
    }
    match r.add_stream("x".into()) {
        Err(KernelError::Busy(m)) => assert!(m.contains("10")),
        e => panic!("{e:?}"),
    }
    for i in 0..64 {
        r.add_deadline(Deadline {
            what: "idle".into(),
            at: i,
        })
        .unwrap();
    }
    match r.add_deadline(Deadline {
        what: "idle".into(),
        at: 0,
    }) {
        Err(KernelError::Busy(m)) => assert!(m.contains("64")),
        e => panic!("{e:?}"),
    }
    r.chunks = vec![1; 16_383];
    assert!(!r.chunks_full());
    r.chunks.push(1);
    assert!(r.chunks_full());
}

#[test]
fn ac4_meta_round_trip() {
    let mut r = rec();
    assert_eq!(
        r.meta(),
        Meta {
            chunks: vec![],
            last: 0
        }
    );
    r.set_meta(&Meta {
        chunks: vec![1, 33],
        last: 40,
    });
    assert_eq!(r.next_index, 41);
    assert_eq!(
        r.meta(),
        Meta {
            chunks: vec![1, 33],
            last: 40
        }
    );
}

#[test]
fn ac5_input_cap() {
    assert!(check_input(&"x".repeat(262_144)).is_ok());
    assert_eq!(
        check_input(&"x".repeat(262_145)),
        Err(KernelError::TooLarge(262_144))
    );
}

#[test]
fn ac5_queue_numbers_caps_and_push_value() {
    let mut q = Queue::default();
    assert_eq!(q.push("a".into()), Ok(1));
    assert_eq!(q.push("b".into()), Ok(2));
    assert_eq!(q.remove(9), Err(KernelError::UnknownInput(9)));
    q.remove(2).unwrap();
    assert_eq!(q.push("c".into()), Ok(3));
    assert_eq!(
        q.push_value(),
        json!({"type":"queue","inputs":[{"n":1,"input":"a"},{"n":3,"input":"c"}]})
    );
    assert_eq!(
        q.take_oldest(),
        Some(Queued {
            n: 1,
            input: "a".into()
        })
    );
    assert_eq!(q.take_oldest().unwrap().n, 3);
    assert_eq!(q.take_oldest(), None);

    let mut q = Queue::default();
    for _ in 0..32 {
        q.push("x".into()).unwrap();
    }
    let before = q.clone();
    assert_eq!(q.push("x".into()), Err(KernelError::QueueFull));
    assert_eq!(q, before);

    let mut q = Queue::default();
    for _ in 0..8 {
        q.push("x".repeat(262_144)).unwrap();
    }
    let before = q.clone();
    assert_eq!(q.push("x".into()), Err(KernelError::QueueFull));
    assert_eq!(q, before);
}

#[test]
fn ac5_queue_load_save() {
    let mut st = MemState::new();
    assert!(Queue::load(&mut st, "s1").is_empty());
    let mut q = Queue::default();
    q.push("a".into()).unwrap();
    q.save(&mut st, "s1").unwrap();
    assert_eq!(st.len(), 1);
    let mut q2 = Queue::load(&mut st, "s1");
    assert_eq!(q2, q);
    q2.take_oldest();
    q2.save(&mut st, "s1").unwrap();
    assert_eq!(st.len(), 1, "an emptied queue keeps its next number");
}

#[test]
fn ac5_numbers_never_repeat_after_an_emptied_queue_is_saved() {
    let mut st = MemState::new();
    let mut q = Queue::load(&mut st, "s1");
    q.save(&mut st, "s1").unwrap();
    assert_eq!(st.len(), 0, "a never-used queue stays absent");
    assert_eq!(q.push("a".into()), Ok(1));
    q.remove(1).unwrap();
    q.save(&mut st, "s1").unwrap();
    let mut q = Queue::load(&mut st, "s1");
    assert!(q.is_empty());
    assert_eq!(q.remove(1), Err(KernelError::UnknownInput(1)));
    assert_eq!(q.push("b".into()), Ok(2));
}
