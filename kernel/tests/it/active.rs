//! Tests of the active index (TECHNICAL 9.1, 9.3 A2-A3, R2).

use agent_kernel::active::*;
use agent_kernel::{KernelError, MemState, State};

fn fresh() -> (MemState, Slots) {
    (MemState::new(), Slots::default())
}

#[test]
fn ac1_slots_load_save() {
    let mut st = MemState::new();
    let s = Slots::load(&mut st);
    assert_eq!(s, Slots::default());
    assert_eq!(s.restart, None);
    assert_eq!(s.active, [0; 4]);
    let s = Slots {
        boot: 7,
        restart: Some(2),
        active: [1, 2, 3, 4],
        ..Slots::default()
    };
    s.save(&mut st).unwrap();
    assert_eq!(Slots::load(&mut st), s);
    let v: serde_json::Value = serde_json::from_slice(&st.get("slots").unwrap()).unwrap();
    for k in [
        "provider", "request", "job", "timers", "boot", "active", "restart", "head", "tail",
    ] {
        assert!(v.get(k).is_some(), "{k}");
    }
}

#[test]
fn ac2_add_one_get_two_puts() {
    let (mut st, mut sl) = fresh();
    st.reset_counts();
    assert_eq!(add(&mut st, &mut sl, "a", 1).unwrap(), 0);
    let c = st.counts();
    assert_eq!((c.gets, c.puts), (1, 2));
    assert_eq!(st.get_keys(), ["active/0"]);
    assert_eq!(sl.active, [1, 0, 0, 0]);
    assert_eq!(Slots::load(&mut st).active, [1, 0, 0, 0]);
    assert_eq!(
        read_page(&mut st, 0),
        vec![PageEntry {
            id: "a".into(),
            boot: 1
        }]
    );
    let raw = String::from_utf8(st.get("active/0").unwrap()).unwrap();
    assert_eq!(raw, r#"[["a",1]]"#);
    sl.active[0] = 256;
    assert_eq!(add(&mut st, &mut sl, "b", 1).unwrap(), 1);
}

#[test]
fn ac3_full_is_busy() {
    let (mut st, mut sl) = fresh();
    sl.active = [256; 4];
    st.reset_counts();
    match add(&mut st, &mut sl, "x", 1) {
        Err(KernelError::Busy(m)) => assert!(m.contains("1024"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(st.counts().puts, 0);
}

#[test]
fn ac4_remove_and_reuse() {
    let (mut st, mut sl) = fresh();
    for i in 0..3 {
        add(&mut st, &mut sl, &format!("s{i}"), 1).unwrap();
    }
    st.reset_counts();
    remove(&mut st, &mut sl, "s1", 0).unwrap();
    let c = st.counts();
    assert_eq!((c.gets, c.puts), (1, 2));
    assert_eq!(sl.active[0], 2);
    let ids: Vec<_> = read_page(&mut st, 0).into_iter().map(|e| e.id).collect();
    assert_eq!(ids, ["s0", "s2"]);
    sl.active[0] = 255;
    sl.active[1] = 256;
    assert_eq!(add(&mut st, &mut sl, "n", 1).unwrap(), 0);
    assert_eq!(sl.active[0], 256);
}

#[test]
fn ac5_full_page_fits() {
    let (mut st, mut sl) = fresh();
    for i in 0..256 {
        let id = format!("{i:0>64}");
        add(&mut st, &mut sl, &id, u64::MAX).unwrap();
    }
    assert_eq!(sl.active[0], 256);
    assert!(st.get("active/0").unwrap().len() <= PAGE_BYTES_MAX);
}

#[test]
fn ac6_set_boot() {
    let (mut st, mut sl) = fresh();
    add(&mut st, &mut sl, "a", 1).unwrap();
    add(&mut st, &mut sl, "b", 1).unwrap();
    st.reset_counts();
    set_boot(&mut st, 0, "b", 9).unwrap();
    let c = st.counts();
    assert_eq!((c.gets, c.puts), (1, 1));
    let p = read_page(&mut st, 0);
    assert_eq!((p[0].boot, p[1].boot), (1, 9));
}

#[test]
fn ac7_next_stale_cursor() {
    let (mut st, mut sl) = fresh();
    sl.active = [256, 256, 0, 0];
    // build pages directly: page 0 all current, page 1 one stale
    sl.active = [0; 4];
    add(&mut st, &mut sl, "cur", 2).unwrap();
    sl.active[0] = 256;
    add(&mut st, &mut sl, "old", 1).unwrap();
    add(&mut st, &mut sl, "cur2", 2).unwrap();
    sl.boot = 2;
    sl.restart = Some(1);
    st.reset_counts();
    let (p, e) = next_stale(&mut st, &mut sl).unwrap();
    assert_eq!((p, e.id.as_str()), (1, "old"));
    assert_eq!(sl.restart, Some(1));
    assert_eq!(st.get_keys(), ["active/1"]);
    assert_eq!(st.counts().puts, 0);
    set_boot(&mut st, 1, "old", 2).unwrap();
    st.reset_counts();
    assert_eq!(next_stale(&mut st, &mut sl), None);
    assert_eq!(sl.restart, None);
    assert!(st.counts().gets <= 4);
    assert_eq!(st.counts().puts, 0);
}
