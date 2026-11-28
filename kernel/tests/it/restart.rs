//! Restart (TECHNICAL 9.3 R1-R6, 16 cases 68, 69, 71): detection, the pass, the step, `say` on
//! `unknown`, the lazy step. OpenAI's rows of the step table wait for stage D (case 70).

use agent_kernel::active::{self, PageEntry, Slots};
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::*;
use serde_json::{json, Value};

type K = Kernel<MemState, FakeHost>;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const IN: &str = r#"[{"type":"text","text":"hi"}]"#;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    for who in ["alice", "bob"] {
        crate::fixture_access::put(&mut st, &format!("access/{who}"), br#"["p"]"#.to_vec())
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
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn started(k: &mut K, owner: &str, key: &str) -> String {
    crate::fixture_access::put(
        &mut k.state,
        &format!("access/{owner}"),
        br#"["p"]"#.to_vec(),
    )
    .unwrap();
    let id = k.start(owner, key, "p", "{}", IN).unwrap().value;
    k.attach(owner, &id, 0).unwrap();
    id
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

/// The launch and the input of the one provider `start` since the calls were cleared.
fn start_of(k: &K) -> (Value, String) {
    let all: Vec<_> = k
        .host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::StartProvider { launch, input } => Some((
                serde_json::from_str::<Value>(launch).unwrap(),
                input.clone(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(all.len(), 1, "{all:?}");
    all.into_iter().next().unwrap()
}

fn ask(id: &str, expires: u64) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": "t", "toolCall": {"toolCallId": "t", "title": "TodoWrite"},
                "input": {"todos": []}, "expires-at": expires}),
        }),
        ..Out::default()
    })
}

fn entries(k: &mut K, owner: &str, id: &str) -> Vec<Value> {
    k.read(owner, id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn calls(k: &K, f: impl Fn(&HostCall) -> bool) -> usize {
    k.host.calls.iter().filter(|c| f(c)).count()
}

fn restart_timers(k: &K) -> usize {
    calls(
        k,
        |c| matches!(c, HostCall::TimerAfter { id, delay_ms: 0 } if id == "restart"),
    )
}

/// A running Claude session with two open asks, a stream, a live key and a queued input.
fn busy(k: &mut K) -> String {
    let id = started(k, "alice", "k");
    k.provider(&id, ask("a1", 9000)).unwrap();
    k.provider(&id, ask("a2", 7000)).unwrap();
    k.say("alice", &id, "q", "queued one").unwrap();
    let mut r = rec(k, &id);
    r.streams = vec!["x1".into()];
    r.deadlines.push(Deadline {
        what: "ask a1".into(),
        at: 9000,
    });
    r.deadlines.push(Deadline {
        what: "ask a2".into(),
        at: 7000,
    });
    r.save(&mut k.state).unwrap();
    k.state.put("stream/x1", b"{}".to_vec()).unwrap();
    k.state.put(&format!("live/{id}"), b"x1".to_vec()).unwrap();
    id
}

/// The host restarts; a call by an owner with no sessions starts the pass and steps nothing.
fn restart_host(k: &mut K, boot: u64) {
    k.host.generation = boot;
    k.list("nobody", None, 1).unwrap();
}

#[test]
fn ac1_every_call_reads_the_boot_and_a_change_starts_the_pass() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let mut sl = Slots::load(&mut k.state);
    sl.provider = 3;
    sl.request = 2;
    sl.job = 1;
    sl.timers = 5;
    sl.save(&mut k.state).unwrap();
    k.host.calls.clear();
    k.queued("alice", &id).unwrap();
    k.read("alice", &id, 0, None, 1).unwrap();
    k.rename("alice", &id, "t").unwrap();
    k.list("alice", None, 5).unwrap();
    k.unqueue("alice", &id, 9).unwrap_err();
    k.say("alice", &id, "k2", "x").unwrap();
    k.answer("alice", &id, "k3", "none", r#"{"deny":true}"#)
        .unwrap_err();
    k.timer("other");
    k.delete("alice", &id).unwrap();
    k.start("alice", "k4", "none", "{}", IN).unwrap_err();
    assert_eq!(calls(&k, |c| *c == HostCall::Generation), 10);
    assert_eq!(restart_timers(&k), 0);
    // the provider's item call reads neither
    let id = started(&mut k, "alice", "k5");
    k.host.calls.clear();
    k.provider(&id, ask("a1", 5)).unwrap();
    assert!(!k.host.calls.contains(&HostCall::Generation));
    // a change of boot
    k.host.generation = 2;
    k.host.calls.clear();
    k.state.reset_counts();
    k.list("alice", None, 5).unwrap();
    let s = Slots::load(&mut k.state);
    assert_eq!(
        (s.boot, s.provider, s.request, s.job, s.timers),
        (2, 0, 0, 0, 5)
    );
    assert_eq!(s.restart, Some(0));
    assert_eq!(
        k.host.calls,
        [
            HostCall::Generation,
            HostCall::TimerAfter {
                id: "restart".into(),
                delay_ms: 0
            }
        ]
    );
    // no session was stepped: the listed record was read, not written
    assert_eq!(rec(&mut k, &id).boot, 1);
    assert_eq!(k.state.counts().deletes, 0);
}

#[test]
fn ac2_the_pass_steps_one_session_per_firing_case_68() {
    let mut k = kernel();
    k.limits.provider = 4096;
    k.limits.owner_provider = 4096;
    k.limits.owner_sessions = 256;
    let mut ids = Vec::new();
    for o in 0..4 {
        for n in 0..256 {
            ids.push(started(&mut k, &format!("o{o}"), &format!("k{n}")));
        }
    }
    assert_eq!(Slots::load(&mut k.state).active, [256; 4]);
    let snap = ids
        .iter()
        .map(|i| k.state.get(&format!("session/{i}")))
        .collect::<Vec<_>>();
    k.host.generation = 2;
    // a boot change with no call changes nothing
    assert_eq!(
        ids.iter()
            .map(|i| k.state.get(&format!("session/{i}")))
            .collect::<Vec<_>>(),
        snap
    );
    restart_host(&mut k, 2);
    let armed = restart_timers(&k);
    let mut firings = 0;
    loop {
        k.state.reset_counts();
        let before = k.host.calls.len();
        let c = k.timer("restart");
        firings += 1;
        // the 4 pages at most, each read once by the search and once more by the step's removal
        let mut pages: Vec<&String> = k
            .state
            .get_keys()
            .iter()
            .filter(|x| x.starts_with("active/"))
            .collect();
        assert!(pages.len() <= 5, "{pages:?}");
        pages.sort();
        pages.dedup();
        assert!(pages.len() <= 4, "{pages:?}");
        assert!(k.state.counts().bytes_read < 96 * 1024 + 8 * 1024);
        let new = &k.host.calls[before..];
        if new.contains(&HostCall::TimerCancel {
            id: "restart".into(),
        }) {
            assert!(c.pushes.is_empty());
            break;
        }
        assert_eq!(
            c.pushes
                .iter()
                .filter(|p| p.value["type"] != "entry")
                .count(),
            1
        );
        assert!(new.contains(&HostCall::TimerAfter {
            id: "restart".into(),
            delay_ms: 0
        }));
        assert!(firings <= 1024);
    }
    assert_eq!(firings, 1025);
    assert_eq!(armed + 1024, restart_timers(&k));
    assert_eq!(Slots::load(&mut k.state).restart, None);
    assert_eq!(Slots::load(&mut k.state).active, [0; 4]);
    for i in &ids {
        assert_eq!(rec(&mut k, i).turn, Turn::Unknown);
    }
}

#[test]
fn ac2_a_second_boot_change_restarts_the_pass_from_page_zero() {
    let mut k = kernel();
    k.limits.provider = 4096;
    k.limits.owner_provider = 4096;
    k.limits.owner_sessions = 256;
    let ids: Vec<String> = (0..300)
        .map(|n| started(&mut k, &format!("o{}", n / 200), &format!("k{n}")))
        .collect();
    restart_host(&mut k, 2);
    for _ in 0..10 {
        k.timer("restart");
    }
    restart_host(&mut k, 3);
    assert_eq!(Slots::load(&mut k.state).restart, Some(0));
    // entries of boot 1 (never reached) and the ones stepped at boot 2 all go; nothing is stale
    let mut n = 0;
    while Slots::load(&mut k.state).restart.is_some() {
        k.timer("restart");
        n += 1;
        assert!(n <= 301);
    }
    assert_eq!(n, 291); // 290 stale + the last firing
                        // the 10 settled at boot 2 left the index and keep it; the other 290 are settled at boot 3
    let boots: Vec<u64> = ids.iter().map(|i| rec(&mut k, i).boot).collect();
    assert_eq!(boots.iter().filter(|b| **b == 2).count(), 10);
    assert_eq!(boots.iter().filter(|b| **b == 3).count(), 290);
    for i in &ids {
        assert_eq!(rec(&mut k, i).turn, Turn::Unknown);
    }
}

#[test]
fn ac3_the_step_settles_a_claude_turn_case_69() {
    let mut k = kernel();
    let id = busy(&mut k);
    // an open check is denied too (fail closed), in the same decision shape
    k.provider(
        &id,
        ProviderEvent::Out(Out {
            entry: Some(Entry {
                kind: Kind::Ask,
                uuid: None,
                body: json!({"id": "c1", "kind": "check", "toolCallId": "t2",
                    "toolCall": {"toolCallId": "t2", "title": "Bash"}, "input": {}}),
            }),
            ..Out::default()
        }),
    )
    .unwrap();
    restart_host(&mut k, 2);
    let c = k.timer("restart");
    assert_eq!(
        c.pushes
            .iter()
            .map(|p| p.value.clone())
            .filter(|v| v["type"] != "entry")
            .collect::<Vec<_>>(),
        [json!({"type": "turn", "state": {"state": "unknown"}})]
    );
    // a timer firing's entries go out live too (S35-fix1 AC3): each decision, then the turn entry
    let live: Vec<&Value> = c
        .pushes
        .iter()
        .filter(|p| p.value["type"] == "entry")
        .map(|p| &p.value["entry"]["kind"])
        .collect();
    assert_eq!(live, ["ask", "ask", "ask", "turn"], "{:?}", c.pushes);
    assert_eq!(c.pushes[0].session, id);
    let r = rec(&mut k, &id);
    assert_eq!((r.boot, r.turn.clone()), (2, Turn::Unknown));
    assert!(r.open.is_empty() && r.streams.is_empty() && r.deadlines.is_empty());
    assert_eq!(r.active_page, None);
    assert!(k.state.get("stream/x1").is_none());
    assert!(k.state.get(&format!("live/{id}")).is_none());
    assert_eq!(Slots::load(&mut k.state).active, [0; 4]);
    assert!(active::read_page(&mut k.state, 0).is_empty());
    assert_eq!(k.queued("alice", &id).unwrap().len(), 1);
    let es = entries(&mut k, "alice", &id);
    let denied: Vec<&Value> = es
        .iter()
        .filter(|e| e["kind"] == "ask" && e["body"]["decision"].is_object())
        .collect();
    assert_eq!(denied.len(), 3);
    for d in &denied {
        assert_eq!(
            d["body"]["decision"],
            json!({"outcome": "deny", "by": "kernel", "reason": "restart"})
        );
    }
    assert_eq!(denied[0]["body"]["reach"], "TodoWrite");
    assert_eq!(
        (
            denied[2]["body"]["id"].as_str(),
            denied[2]["body"]["kind"].as_str()
        ),
        (Some("c1"), Some("check"))
    );
    assert!(es.iter().all(|e| e["kind"] != "tool"));
    let last = es.last().unwrap();
    assert_eq!(
        (last["kind"].as_str(), &last["body"]),
        (Some("turn"), &json!({"state": "unknown"}))
    );
}

#[test]
fn ac3_paused_openai_and_settled_sessions_change_only_what_the_table_says() {
    let mut k = kernel();
    // paused: unchanged but for the boot
    let p = started(&mut k, "alice", "k1");
    k.provider(&p, ProviderEvent::Turn(Turn::Paused { until: Some(5000) }))
        .unwrap();
    // an OpenAI running session
    let o = started(&mut k, "alice", "k2");
    let mut r = rec(&mut k, &o);
    r.backend = "openai".into();
    r.streams = vec!["x2".into()];
    r.save(&mut k.state).unwrap();
    k.state.put("stream/x2", b"{}".to_vec()).unwrap();
    // an idle one with a stale stream
    let i = started(&mut k, "alice", "k3");
    k.provider(&i, ProviderEvent::Turn(Turn::default()))
        .unwrap();
    let mut r = rec(&mut k, &i);
    r.streams = vec!["x3".into()];
    r.save(&mut k.state).unwrap();
    k.state.put("stream/x3", b"{}".to_vec()).unwrap();
    let (turn_p, turn_o) = (rec(&mut k, &p).turn, rec(&mut k, &o).turn);
    restart_host(&mut k, 2);
    for _ in 0..3 {
        k.timer("restart");
    }
    k.read("alice", &i, 0, None, 1).unwrap();
    for (id, turn) in [(&p, turn_p), (&o, turn_o)] {
        let r = rec(&mut k, id);
        assert_eq!((r.boot, r.turn), (2, turn));
        assert!(r.active_page.is_some());
    }
    assert!(active::read_page(&mut k.state, 0)
        .iter()
        .all(|e: &PageEntry| e.boot == 2));
    assert!(k.state.get("stream/x2").is_none());
    let r = rec(&mut k, &i);
    assert_eq!((r.boot, r.turn, r.streams.len()), (2, idle0(), 0));
    assert!(k.state.get("stream/x3").is_none());
}

#[test]
fn ac4_a_step_writes_no_more_than_the_bound() {
    let mut k = kernel();
    let id = busy(&mut k);
    restart_host(&mut k, 2);
    k.state.reset_counts();
    k.read("alice", &id, 0, None, 1).unwrap();
    let c = k.state.counts();
    // record, slots, page, two decisions and the turn entry, and the two accounts
    assert!(c.puts <= 8, "{c:?}");
    assert_eq!(c.deletes, 2);
}

#[test]
fn ac5_say_on_unknown_resumes() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    let mut r = rec(&mut k, &id);
    r.claude_uuid = Some("11111111-2222-3333-4444-555555555555".into());
    r.save(&mut k.state).unwrap();
    restart_host(&mut k, 2);
    k.timer("restart");
    assert_eq!(rec(&mut k, &id).turn, Turn::Unknown);
    k.host.calls.clear();
    k.say("alice", &id, "s", "go on").unwrap();
    // S35: the new provider stream's `start` carries the input and the resume (4.1, R5).
    let (launch, input) = start_of(&k);
    assert_eq!(input, "go on");
    assert_eq!(
        launch["session"],
        json!({"resume": "11111111-2222-3333-4444-555555555555"})
    );
    assert!(!k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::Request(Request::Turn { .. }))));
    assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
    k.provider(&id, ProviderEvent::Turn(Turn::ResumeLost))
        .unwrap();
    assert_eq!(rec(&mut k, &id).turn, Turn::ResumeLost);
}

#[test]
fn ac5_with_inputs_queued_the_oldest_starts_and_the_say_joins_the_end() {
    let mut k = kernel();
    let id = busy(&mut k);
    restart_host(&mut k, 2);
    k.timer("restart");
    k.host.calls.clear();
    let n = k.say("alice", &id, "s", "later").unwrap().value;
    assert_eq!(n, 2);
    assert_eq!(start_of(&k).1, "queued one");
    let q = k.queued("alice", &id).unwrap();
    assert_eq!(q.len(), 1);
    assert_eq!((q[0].n, q[0].input.as_str()), (2, "later"));
}

#[test]
fn ac6_the_lazy_step_comes_first_in_read_and_answer_case_71() {
    let mut k = kernel();
    let id = busy(&mut k);
    restart_host(&mut k, 2);
    let es = entries(&mut k, "alice", &id);
    assert_eq!(
        es.iter()
            .filter(|e| e["body"]["decision"]["by"] == "kernel")
            .count(),
        2
    );
    assert_eq!(es.last().unwrap()["body"], json!({"state": "unknown"}));
    let KernelError::Refused(rs) = k
        .answer("alice", &id, "ak", "a1", r#"{"allow":true}"#)
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "ask");
    // a stepped session is not stepped again: no second denial or `unknown` entry
    assert_eq!(entries(&mut k, "alice", &id), es);
    // the pass finds the entry current
    k.host.calls.clear();
    let c = k.timer("restart");
    assert!(c.pushes.is_empty());
    assert!(k.host.calls.contains(&HostCall::TimerCancel {
        id: "restart".into()
    }));
}

#[test]
fn ac6_every_session_call_but_delete_steps_a_stale_record_first() {
    for call in ["say", "queued", "unqueue", "rename", "delete"] {
        let mut k = kernel();
        let id = busy(&mut k);
        restart_host(&mut k, 2);
        match call {
            "say" => {
                k.say("alice", &id, "s", "x").unwrap();
                assert!(matches!(rec(&mut k, &id).turn, Turn::Running { .. }));
            }
            "queued" => drop(k.queued("alice", &id).unwrap()),
            "unqueue" => {
                let c = k.unqueue("alice", &id, 1).unwrap();
                assert_eq!(
                    c.pushes
                        .iter()
                        .find(|p| p.value["type"] == "turn")
                        .unwrap()
                        .value,
                    json!({"type": "turn", "state": {"state": "unknown"}})
                );
            }
            "rename" => k.rename("alice", &id, "t").unwrap(),
            _ => {
                k.delete("alice", &id).unwrap();
                assert!(Record::load(&mut k.state, &id).is_none());
                assert!(k.state.get("stream/x1").is_none());
                assert_eq!(Slots::load(&mut k.state).active, [0; 4]);
                continue;
            }
        }
        let r = rec(&mut k, &id);
        assert_eq!(r.boot, 2, "{call}");
        assert!(r.open.is_empty(), "{call}");
        assert!(k.state.get("stream/x1").is_none(), "{call}");
    }
}

#[test]
fn ac6_list_reports_unknown_for_a_stale_turn_and_writes_nothing() {
    let mut k = kernel();
    let id = busy(&mut k);
    let other = started(&mut k, "bob", "k");
    restart_host(&mut k, 2);
    k.state.reset_counts();
    let l = k.list("alice", None, 10).unwrap();
    assert_eq!(l[0].turn, r#"{"state":"unknown"}"#);
    let c = k.state.counts();
    assert_eq!((c.puts, c.deletes), (0, 0));
    // slots, `quota/<owner>`, the owner page and the one record it lists
    assert_eq!(c.gets, 4, "{:?}", k.state.get_keys());
    assert_eq!(rec(&mut k, &id).boot, 1);
    assert_eq!(rec(&mut k, &other).boot, 1);
}

#[test]
fn ac7_expires_at_is_the_earliest_open_ask_after_one_is_answered() {
    let mut k = kernel();
    let id = started(&mut k, "alice", "k");
    k.provider(&id, ask("a1", 9000)).unwrap();
    k.provider(&id, ask("a2", 7000)).unwrap();
    k.provider(&id, ask("a3", 8000)).unwrap();
    let raw: Value =
        serde_json::from_slice(&k.state.get(&format!("session/{id}")).unwrap()).unwrap();
    assert_eq!(raw["open"][1]["expires-at"], 7000);
    k.state.reset_counts();
    let c = k
        .answer("alice", &id, "k1", "a2", r#"{"allow":true}"#)
        .unwrap();
    assert_eq!(
        c.pushes
            .iter()
            .find(|p| p.value["type"] == "turn")
            .unwrap()
            .value["state"],
        json!({"state": "waiting", "asks": ["a1", "a3"], "expires-at": 8000})
    );
    let c = k
        .answer("alice", &id, "k2", "a3", r#"{"deny":true}"#)
        .unwrap();
    assert_eq!(
        c.pushes
            .iter()
            .find(|p| p.value["type"] == "turn")
            .unwrap()
            .value["state"],
        json!({"state": "waiting", "asks": ["a1"], "expires-at": 9000})
    );
    let none = OpenAsk {
        id: "z".into(),
        kind: "ask".into(),
        i: 1,
        expires_at: None,
        call: String::new(),
    };
    assert!(!serde_json::to_string(&none).unwrap().contains("expires-at"));
}

#[test]
fn ac2_a_failed_step_or_a_missing_record_does_not_stop_the_pass() {
    let mut k = kernel();
    let ids: Vec<String> = (0..4)
        .map(|n| started(&mut k, "alice", &format!("k{n}")))
        .collect();
    // ids[0]: deleted while still listed; ids[1]: unreadable; ids[2]: its step fails (no room
    // for the `turn` entry); ids[3]: an ordinary one after them
    k.state.delete(&format!("session/{}", ids[0]));
    k.state
        .put(&format!("session/{}", ids[1]), b"not json".to_vec())
        .unwrap();
    let mut r = rec(&mut k, &ids[2]);
    r.chunks = (1..=agent_kernel::transcript::MAX_CHUNKS as u64).collect();
    r.save(&mut k.state).unwrap();
    restart_host(&mut k, 2);
    let mut firings = 0;
    let mut pushes = 0;
    while Slots::load(&mut k.state).restart.is_some() {
        pushes += k
            .timer("restart")
            .pushes
            .iter()
            .filter(|p| p.value["type"] != "entry")
            .count();
        firings += 1;
        assert!(firings <= 5);
    }
    assert_eq!(firings, 5);
    assert_eq!(pushes, 1);
    assert!(k.host.calls.contains(&HostCall::TimerCancel {
        id: "restart".into()
    }));
    assert_eq!(rec(&mut k, &ids[3]).turn, Turn::Unknown);
    // the failed one stays listed, its entry current, its record stale for the lazy step (R6)
    let page = active::read_page(&mut k.state, 0);
    assert_eq!(
        page.iter()
            .map(|e| (e.id.as_str(), e.boot))
            .collect::<Vec<_>>(),
        [(ids[2].as_str(), 2)]
    );
    assert_eq!(Slots::load(&mut k.state).active, [1, 0, 0, 0]);
    let r = rec(&mut k, &ids[2]);
    assert!(r.boot == 1 && matches!(r.turn, Turn::Running { .. }));
}

fn idle0() -> Turn {
    Turn::Idle {
        stop: Stop::Completed,
        seconds: Some(0),
        tools: None,
        tokens: None,
    }
}
