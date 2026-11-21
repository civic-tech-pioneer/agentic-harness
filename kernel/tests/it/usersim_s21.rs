//! S21 user simulation: a kernel maker drives sessions through 9.3 on `MemState` with a fake boot.

use agent_kernel::active::{self, PageEntry, Slots};
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    st.put("access/ann", br#"["p"]"#.to_vec()).unwrap();
    let host = FakeHost {
        boot: 7,
        now: 5000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn ask(id: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": "t1", "toolCall": {"toolCallId": "t1", "title": "Edit"},
                "input": {}, "expires-at": 999_999, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    })
}

fn done() -> ProviderEvent {
    ProviderEvent::Turn(Turn::Idle {
        stop: Stop::Completed,
        seconds: None,
        tools: None,
        tokens: None,
    })
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("ann", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn page_writes(k: &K) -> usize {
    k.state.get_keys().len() // reads only; writes are counted by puts below
}

#[test]
fn usersim_s21_story() {
    let mut k = kernel();
    // A first call on a fresh state: boot 7 differs from slots' 0.
    let s1 = k.start("ann", "k1", "p", "{}", "hello").unwrap().value;
    let r = rec(&mut k, &s1);
    eprintln!("record after start: {}", serde_json::to_string(&r).unwrap());
    assert!(matches!(r.turn, Turn::Running { .. }));
    assert_eq!(r.active_page, Some(0));
    assert_eq!(active::read_page(&mut k.state, 0).len(), 1);

    // Outcome 1 caps.
    let long = "x".repeat(257);
    assert_eq!(
        k.start("ann", &long, "p", "{}", "hi").unwrap_err(),
        KernelError::TooLarge(256)
    );
    assert_eq!(
        k.rename("ann", &s1, &long).unwrap_err(),
        KernelError::TooLarge(256)
    );
    k.rename("ann", &s1, &"t".repeat(256)).unwrap();

    // Outcome 2: queue while running.
    let mut last = None;
    for i in 0..32 {
        let c = k.say("ann", &s1, &format!("q{i}"), "more").unwrap();
        last = Some(c);
    }
    let c = last.unwrap();
    eprintln!("say queued: {:?}", c);
    assert!(c.pushes.iter().any(|p| p.value["type"] == "queue"));
    assert_eq!(
        k.say("ann", &s1, "q-over", "more").unwrap_err(),
        KernelError::QueueFull
    );
    // Key not kept on failure: same key retries after unqueue.
    let q = k.queued("ann", &s1).unwrap();
    assert_eq!(q.len(), 32);
    let un = k.unqueue("ann", &s1, q[0].n).unwrap();
    assert_eq!(un.pushes[0].value["type"], "queue");
    assert_eq!(un.pushes[0].value["inputs"].as_array().unwrap().len(), 31);
    assert_eq!(
        k.unqueue("ann", &s1, q[0].n).unwrap_err(),
        KernelError::UnknownInput(q[0].n)
    );
    k.say("ann", &s1, "q-over", "more").unwrap();
    let big = "y".repeat(262_145);
    assert_eq!(
        k.say("ann", &s1, "big", &big).unwrap_err(),
        KernelError::TooLarge(262_144)
    );

    // Outcome 3: running -> waiting writes no page.
    k.state.reset_counts();
    let before = k.state.get("active/0");
    k.provider(&s1, ask("a1")).unwrap();
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Waiting { .. }));
    assert_eq!(k.state.get("active/0"), before);
    let _ = page_writes(&k);

    // Outcome 4: restart with a waiting Claude session, a paused one, an idle one.
    let s2 = k.start("ann", "k2", "p", "{}", "two").unwrap().value;
    k.provider(&s2, ProviderEvent::Turn(Turn::Paused { until: Some(77) }))
        .unwrap();
    let s3 = k.start("ann", "k3", "p", "{}", "three").unwrap().value;
    k.provider(&s3, done()).unwrap();
    assert_eq!(rec(&mut k, &s3).active_page, None);
    let listed: usize = (0..4)
        .map(|n| active::read_page(&mut k.state, n).len())
        .sum();
    assert_eq!(listed, 2);

    k.host.boot = 8;
    k.host.calls.clear();
    // list reports unknown for s1 without writing.
    k.state.reset_counts();
    let l = k.list("ann", None, 10).unwrap();
    eprintln!("list after boot change: {:?}", l);
    let s1info = l.iter().find(|i| i.id == s1).unwrap();
    assert_eq!(s1info.turn, r#"{"state":"unknown"}"#);
    let slots = Slots::load(&mut k.state);
    assert_eq!(slots.boot, 8);
    assert_eq!(slots.restart, Some(0));
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::TimerAfter { id, delay_ms: 0 } if id == "restart")));
    // Nothing stepped yet.
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Waiting { .. }));

    // Pass: at most A + 1 = 3 firings.
    let mut firings = 0;
    let mut pushes = vec![];
    loop {
        k.host.calls.clear();
        let c = k.timer("restart");
        firings += 1;
        pushes.extend(c.pushes);
        let rearmed = k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::TimerAfter { id, .. } if id == "restart"));
        if !rearmed || firings > 5 {
            break;
        }
    }
    eprintln!("firings {firings}, pushes {:?}", pushes);
    assert!(firings <= 3);
    let r1 = rec(&mut k, &s1);
    assert_eq!(r1.turn, Turn::Unknown);
    assert_eq!(r1.active_page, None);
    assert_eq!(r1.boot, 8);
    assert!(pushes.iter().any(|p| p.session == s1
        && p.value["type"] == "turn"
        && p.value["state"]["state"] == "unknown"));
    let es = entries(&mut k, &s1);
    eprintln!("s1 entries tail: {:?}", &es[es.len().saturating_sub(3)..]);
    assert!(es.iter().any(|e| e.to_string().contains("restart")));
    assert!(es
        .iter()
        .any(|e| e["kind"] == "turn" && e["body"]["state"] == "unknown"));
    assert_eq!(rec(&mut k, &s2).turn, Turn::Paused { until: Some(77) });
    assert_eq!(k.queued("ann", &s1).unwrap().len(), 32);

    // Answer to the denied ask refused {field: ask}.
    match k.answer("ann", &s1, "ans1", "a1", r#"{"allow":true}"#) {
        Err(KernelError::Refused(rs)) => assert_eq!(rs[0].field, "ask"),
        o => panic!("answer after restart: {o:?}"),
    }

    // R5: say on unknown resumes with the oldest queued input, the new one at the end.
    let first = k.queued("ann", &s1).unwrap()[0].clone();
    k.unqueue("ann", &s1, first.n).unwrap();
    k.host.calls.clear();
    k.say("ann", &s1, "cont", "Continue").unwrap();
    let reqs: Vec<_> = k
        .host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(r) => Some(r.clone()),
            _ => None,
        })
        .collect();
    eprintln!("requests after Continue: {:?}", reqs);
    match &reqs[..] {
        [Request::Turn { resume, input, .. }] => {
            assert!(*resume);
            assert_ne!(input, "Continue");
        }
        o => panic!("{o:?}"),
    }
    let q = k.queued("ann", &s1).unwrap();
    assert_eq!(q.last().unwrap().input, "Continue");
    assert!(matches!(rec(&mut k, &s1).turn, Turn::Running { .. }));
    assert_eq!(rec(&mut k, &s1).active_page, Some(0));

    // Outcome 5: a key replay returns the first result and writes nothing.
    k.state.reset_counts();
    let again = k.say("ann", &s1, "cont", "Continue").unwrap();
    eprintln!("replay: {:?}", again);
    assert_eq!(k.state.counts().puts, 0);
    assert_eq!(k.queued("ann", &s1).unwrap().len(), q.len());
}

/// A3 with 1 024 seeded entries: busy, its message names the cap, a leaving session frees room.
#[test]
fn usersim_s21_busy() {
    let mut k = kernel();
    let s = k.start("ann", "a", "p", "{}", "hi").unwrap().value;
    k.provider(&s, done()).unwrap();
    let mut slots = Slots::load(&mut k.state);
    for p in 0..4u8 {
        let entries: Vec<PageEntry> = (0..256)
            .map(|i| PageEntry {
                id: format!("z{p}-{i}"),
                boot: 7,
            })
            .collect();
        k.state
            .put(&active::page_key(p), serde_json::to_vec(&entries).unwrap())
            .unwrap();
        slots.active[p as usize] = 256;
    }
    slots.save(&mut k.state).unwrap();
    // A real one among them to leave later.
    k.state.reset_counts();
    let e = k.start("ann", "b", "p", "{}", "hi").unwrap_err();
    eprintln!("busy start: {e} / puts {}", k.state.counts().puts);
    assert!(matches!(&e, KernelError::Busy(m) if m.contains("1024") || m.contains("1 024")));
    assert_eq!(k.state.counts().puts, 0);
    let e = k.say("ann", &s, "c", "again").unwrap_err();
    assert!(matches!(e, KernelError::Busy(_)));
    // The same key works once room comes.
    let mut slots = Slots::load(&mut k.state);
    let mut p0 = active::read_page(&mut k.state, 0);
    p0.pop();
    k.state
        .put(&active::page_key(0), serde_json::to_vec(&p0).unwrap())
        .unwrap();
    slots.active[0] = 255;
    slots.save(&mut k.state).unwrap();
    k.say("ann", &s, "c", "again").unwrap();
    assert_eq!(rec(&mut k, &s).active_page, Some(0));
    k.delete("ann", &s).unwrap();
    assert_eq!(Slots::load(&mut k.state).active[0], 255);
    assert!(!active::read_page(&mut k.state, 0).iter().any(|e| e.id == s));
}

/// A turn that ends and starts the next queued input writes no page; enter/leave write one.
#[test]
fn usersim_s21_page_writes() {
    let mut k = kernel();
    let s = k.start("ann", "a", "p", "{}", "hi").unwrap().value;
    k.say("ann", &s, "b", "next").unwrap();
    let p0 = k.state.get("active/0");
    let sl = k.state.get("slots");
    k.provider(&s, done()).unwrap();
    assert!(matches!(rec(&mut k, &s).turn, Turn::Running { .. }));
    assert_eq!(k.state.get("active/0"), p0);
    assert_eq!(k.state.get("slots"), sl);
    k.provider(&s, done()).unwrap();
    assert_eq!(active::read_page(&mut k.state, 0).len(), 0);
    assert_eq!(rec(&mut k, &s).active_page, None);
}

/// R6: read, say, answer and delete on a stale record step it first; stale stream keys go.
#[test]
fn usersim_s21_lazy() {
    let mut k = kernel();
    let mut ids = vec![];
    for key in ["a", "b", "c", "d"] {
        let s = k.start("ann", key, "p", "{}", "hi").unwrap().value;
        k.provider(&s, ask(&format!("ask-{key}"))).unwrap();
        let mut r = rec(&mut k, &s);
        r.streams.push(format!("st-{key}"));
        r.save(&mut k.state).unwrap();
        k.state
            .put(
                &format!("stream/st-{key}"),
                format!(r#"{{"session":"{s}","kind":"provider"}}"#).into_bytes(),
            )
            .unwrap();
        k.state
            .put(&format!("live/{s}"), b"\"o1\"".to_vec())
            .unwrap();
        ids.push(s);
    }
    k.host.boot = 8;
    // read
    let es = entries(&mut k, &ids[0]);
    assert!(
        es.iter()
            .any(|e| e["kind"] == "turn" && e["body"]["state"] == "unknown"),
        "read did not step"
    );
    assert!(k.state.get("stream/st-a").is_none());
    assert!(k.state.get(&format!("live/{}", ids[0])).is_none());
    // answer
    match k.answer("ann", &ids[1], "x", "ask-b", r#"{"allow":true}"#) {
        Err(KernelError::Refused(rs)) => assert_eq!(rs[0].field, "ask"),
        o => panic!("{o:?}"),
    }
    assert_eq!(rec(&mut k, &ids[1]).turn, Turn::Unknown);
    // say
    k.host.calls.clear();
    k.say("ann", &ids[2], "y", "Continue").unwrap();
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::Request(Request::Turn { resume: true, .. }))));
    // resume refused
    k.provider(&ids[2], ProviderEvent::Turn(Turn::ResumeLost))
        .unwrap();
    assert_eq!(rec(&mut k, &ids[2]).turn, Turn::ResumeLost);
    assert_eq!(rec(&mut k, &ids[2]).active_page, None);
    // delete
    k.delete("ann", &ids[3]).unwrap();
    assert!(k.state.get("stream/st-d").is_none());
    assert!(k.state.get(&format!("live/{}", ids[3])).is_none());
    let listed: Vec<_> = (0..4)
        .flat_map(|n| active::read_page(&mut k.state, n))
        .collect();
    eprintln!("listed after lazy: {listed:?}");
    assert!(!listed.iter().any(|e| e.id == ids[3]));
    // pass: skips the ones already stepped
    let mut n = 0;
    loop {
        k.host.calls.clear();
        let c = k.timer("restart");
        n += 1;
        assert!(
            c.pushes.is_empty(),
            "pass stepped a settled session: {:?}",
            c.pushes
        );
        if !k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::TimerAfter { id, .. } if id == "restart"))
            || n > 10
        {
            break;
        }
    }
    eprintln!("firings for settled: {n}");
}

/// Case 68 with 1 024 listed sessions: A + 1 firings, bounded reads, a second boot restarts.
#[test]
fn usersim_s21_pass_bounds() {
    let mut k = kernel();
    let mut slots = Slots {
        boot: 7,
        provider: 3,
        timers: 5,
        ..Default::default()
    };
    for p in 0..4u8 {
        let mut page = vec![];
        for i in 0..256 {
            let id = format!("p{p}i{i:03}");
            let r = Record {
                id: id.clone(),
                owner: format!("o{}", (p as usize * 256 + i) / 64),
                profile: "p".into(),
                backend: "claude".into(),
                turn: Turn::Running {
                    since: 1,
                    activity: None,
                },
                boot: 7,
                active_page: Some(p),
                ..Record::default()
            };
            r.save(&mut k.state).unwrap();
            page.push(PageEntry { id, boot: 7 });
        }
        k.state
            .put(&active::page_key(p), serde_json::to_vec(&page).unwrap())
            .unwrap();
        slots.active[p as usize] = 256;
    }
    slots.save(&mut k.state).unwrap();
    k.host.boot = 9;
    // No call: nothing changes. First call: list for someone with nothing.
    k.list("nobody", None, 5).unwrap();
    let s = Slots::load(&mut k.state);
    assert_eq!(
        (s.provider, s.timers, s.restart, s.boot),
        (0, 5, Some(0), 9)
    );
    let mut firings = 0u32;
    let mut max_read = 0u64;
    loop {
        k.state.reset_counts();
        k.host.calls.clear();
        k.timer("restart");
        firings += 1;
        max_read = max_read.max(k.state.counts().bytes_read);
        if firings == 300 {
            // A second boot change mid-pass.
            k.host.boot = 10;
            k.list("nobody", None, 5).unwrap();
            assert_eq!(Slots::load(&mut k.state).restart, Some(0));
        }
        if !k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::TimerAfter { id, .. } if id == "restart"))
            || firings > 3000
        {
            break;
        }
    }
    eprintln!("firings {firings}, max bytes read per firing {max_read}");
    assert!(firings <= 1025 + 300, "firings {firings}");
    assert!(max_read <= 96 * 1024 + 512 * 1024);
    let listed: usize = (0..4)
        .map(|n| active::read_page(&mut k.state, n).len())
        .sum();
    assert_eq!(listed, 0);
    assert_eq!(
        Record::load(&mut k.state, "p3i255").unwrap().turn,
        Turn::Unknown
    );
}

/// `list` over stale records reports `unknown` and writes nothing (after R1's own write).
#[test]
fn usersim_s21_list_no_write() {
    let mut k = kernel();
    let s = k.start("ann", "a", "p", "{}", "hi").unwrap().value;
    k.host.boot = 8;
    k.list("ann", None, 5).unwrap();
    k.state.reset_counts();
    let l = k.list("ann", None, 5).unwrap();
    assert_eq!(k.state.counts().puts + k.state.counts().deletes, 0);
    assert_eq!(l[0].turn, r#"{"state":"unknown"}"#);
    assert!(matches!(rec(&mut k, &s).turn, Turn::Running { .. }));
}

/// Red: R5 `Continue` on an `unknown` session whose kept queue is full: the new input joins the
/// end and the oldest leaves to start the turn, so the queue never holds more than 32; the call
/// must not be refused `queue-full`.
#[test]
fn regress_s21_continue_full_queue() {
    let mut k = kernel();
    let s = k.start("ann", "a", "p", "{}", "hi").unwrap().value;
    for i in 0..32 {
        k.say("ann", &s, &format!("q{i}"), "more").unwrap();
    }
    k.host.boot = 8;
    let r = k.say("ann", &s, "cont", "Continue");
    assert!(r.is_ok(), "Continue after a restart with 32 queued: {r:?}");
    assert_eq!(k.queued("ann", &s).unwrap().len(), 32);
}
