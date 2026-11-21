//! S25-1: the quota pieces (TECHNICAL 9.3 Quota).

use agent_kernel::{scope, Change, Limits, MemState, Metered, Scope, State, Usage};

fn mem(pairs: &[(&str, usize)]) -> MemState {
    let mut m = MemState::new();
    for (k, n) in pairs {
        m.put(k, vec![b'x'; *n]).unwrap();
    }
    m
}

#[test]
fn ac1_limits_default() {
    let l = Limits::default();
    assert_eq!(l.owner_sessions, 64);
    assert_eq!(l.owner_bytes, 64 * 1_048_576);
    assert_eq!(l.account_bytes, 224 * 1_048_576);
    assert_eq!(l.reserve_bytes, 32 * 1_048_576);
}

#[test]
fn ac2_usage_roundtrip_and_json() {
    let mut st = MemState::new();
    assert_eq!(Usage::load(&mut st, "quota"), Usage::default());
    st.put("quota", b"garbage".to_vec()).unwrap();
    assert_eq!(Usage::load(&mut st, "quota"), Usage::default());
    let u = Usage {
        bytes: 5,
        sessions: 2,
        pages: vec![],
        ..Usage::default()
    };
    u.save(&mut st, "quota").unwrap();
    assert_eq!(st.get("quota").unwrap(), br#"{"bytes":5,"sessions":2}"#);
    assert_eq!(Usage::load(&mut st, "quota"), u);
    let p = Usage {
        bytes: 1,
        sessions: 0,
        pages: vec![1, 2],
        ..Usage::default()
    };
    p.save(&mut st, "quota/o").unwrap();
    assert_eq!(
        st.get("quota/o").unwrap(),
        br#"{"bytes":1,"sessions":0,"pages":[1,2]}"#
    );
    assert_eq!(Usage::load(&mut st, "quota/o"), p);
    assert_eq!(Usage::owner_key("a/b%c"), "quota/a%2Fb%25c");
    assert_eq!(Usage::ACCOUNT, "quota");
    let mut u = Usage {
        bytes: 3,
        ..Default::default()
    };
    u.add(4);
    assert_eq!(u.bytes, 7);
    u.add(-100);
    assert_eq!(u.bytes, 0);
}

#[test]
fn ac3_scopes() {
    assert_eq!(scope("session/s"), Scope::Record);
    for k in [
        "queue/s",
        "entries/s/0",
        "wire/s/1",
        "loop/s",
        "stream/s",
        "live/s",
    ] {
        assert_eq!(scope(k), Scope::Session, "{k}");
    }
    for k in ["key/o/k", "owner/o/1"] {
        assert_eq!(scope(k), Scope::Owner, "{k}");
    }
    for k in [
        "slots",
        "slots/wait/1",
        "active/1",
        "quota",
        "quota/o",
        "profile/p",
        "access/a",
        "adopted/x",
        "sessionx",
        "sessionx/1",
        "unknown/x",
        "",
    ] {
        assert_eq!(scope(k), Scope::Kernel, "{k}");
    }
}

#[test]
fn ac4_metering() {
    let mut m = Metered::new(mem(&[("queue/s", 10), ("key/o/k", 4), ("slots", 3)]));
    m.put("queue/s", vec![0; 99]).unwrap();
    m.put("new/x", vec![0; 1]).unwrap();
    assert_eq!(m.change(), Change::default(), "not open");
    m.open();
    m.put("queue/s", vec![0; 15]).unwrap();
    assert_eq!(
        m.change(),
        Change {
            owner: -84,
            session: -84
        }
    );
    m.put("key/o/k", vec![0; 10]).unwrap();
    assert_eq!(
        m.change(),
        Change {
            owner: -78,
            session: -84
        }
    );
    m.put("slots", vec![0; 500]).unwrap();
    m.put("entries/s/0", vec![0; 3]).unwrap();
    assert_eq!(
        m.change(),
        Change {
            owner: -78 + 14,
            session: -84 + 14
        }
    );
    m.delete("key/o/k");
    m.delete("key/o/k");
    m.delete("slots");
    assert_eq!(
        m.change(),
        Change {
            owner: -78 + 14 - 17,
            session: -84 + 14
        }
    );
    let c = m.close();
    assert_eq!(c.owner, -81);
    m.put("queue/s", vec![0; 1]).unwrap();
    assert_eq!(m.change(), c, "closed: no more charge");
    m.open();
    assert_eq!(m.change(), Change::default());
}

#[test]
fn ac5_few_reads() {
    let mut m = Metered::new(mem(&[("entries/s/0", 5)]));
    m.open();
    m.reset_counts();
    m.get("entries/s/0");
    m.put("entries/s/0", vec![0; 7]).unwrap();
    assert_eq!(m.counts().gets, 1);
    assert_eq!(m.change().owner, 2);

    m.reset_counts();
    m.put("queue/s", vec![0; 2]).unwrap();
    m.put("queue/s", vec![0; 4]).unwrap();
    assert_eq!(m.counts().gets, 1);

    // a refused put charges nothing
    let before = m.change();
    let big = vec![0u8; agent_kernel::MAX_VALUE + 1];
    assert!(m.put("queue/t", big).is_err());
    assert_eq!(m.change(), before);
}

#[test]
fn ac6_uncharged_and_adjusted() {
    let mut m = Metered::new(mem(&[("entries/s/0", 5)]));
    m.open();
    m.get("entries/s/0");
    m.reset_counts();
    m.delete_uncharged("entries/s/0");
    assert_eq!(m.counts().gets, 0);
    assert_eq!(m.change(), Change::default());
    assert!(m.entries().next().is_none());
    // the meter forgets the size it saw: a new put costs its whole name and value
    m.put("entries/s/0", vec![0; 2]).unwrap();
    assert_eq!(m.change().owner, 13);
    m.delete_uncharged("entries/s/0");
    m.adjust(Change {
        owner: -13,
        session: -13,
    });
    m.adjust(Change {
        owner: 5,
        session: 2,
    });
    m.adjust(Change {
        owner: -1,
        session: 0,
    });
    assert_eq!(
        m.change(),
        Change {
            owner: 4,
            session: 2
        }
    );
}

#[test]
fn ac7_inner_state() {
    let mut m = Metered::new(MemState::new());
    m.put("a", vec![1]).unwrap();
    m.get("a");
    assert_eq!(m.get_keys(), ["a".to_string()]);
    m.reset_counts();
    assert_eq!(m.counts().gets, 0);
    let inner = m.into_inner();
    assert_eq!(inner.len(), 1);
}

fn sum(m: &MemState, session_only: bool) -> i64 {
    m.entries()
        .filter(|(k, _)| match scope(k) {
            Scope::Kernel => false,
            Scope::Session => true,
            _ => !session_only,
        })
        .map(|(k, v)| (k.len() + v.len()) as i64)
        .sum()
}

#[test]
fn ac8_exact() {
    let mut m = Metered::new(mem(&[
        ("session/s", 30),
        ("queue/s", 8),
        ("entries/s/0", 20),
        ("wire/s/0", 9),
        ("loop/s", 4),
        ("stream/s", 2),
        ("live/s", 6),
        ("key/o/a", 7),
        ("owner/o/1", 3),
        ("slots", 5),
        ("quota", 9),
        ("profile/p", 11),
        ("active/1", 1),
    ]));
    let (b, bs) = (sum(&m, false), sum(&m, true));
    m.open();
    m.put("queue/s", vec![0; 40]).unwrap();
    m.put("queue/s", vec![0; 3]).unwrap();
    m.put("entries/s/1", vec![0; 12]).unwrap();
    m.put("session/s", vec![0; 90]).unwrap();
    m.put("key/o/b", vec![0; 1]).unwrap();
    m.put("key/o/a", vec![0; 70]).unwrap();
    m.put("owner/o/1", vec![0; 1]).unwrap();
    m.put("slots", vec![0; 50]).unwrap();
    m.delete("wire/s/0");
    m.delete("live/s");
    m.delete("key/o/a");
    m.delete("nothing/x");
    m.put("stream/s", vec![0; 20]).unwrap();
    m.delete("stream/s");
    m.put("wire/s/0", vec![0; 5]).unwrap();
    let c = m.close();
    let inner = m.into_inner();
    assert_eq!(c.owner, sum(&inner, false) - b);
    assert_eq!(c.session, sum(&inner, true) - bs);
}
