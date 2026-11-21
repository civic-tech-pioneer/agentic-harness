//! S26-4: attach and the owner stream (TECHNICAL 9.4 "A full owner stream", "The wire"; 3 "Deferred batch").

use agent_kernel::active::Slots;
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
        st.put(&format!("access/{who}"), br#"["p"]"#.to_vec())
            .unwrap();
    }
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        boot: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

/// A started session; its provider stream is `t1`.
fn started(k: &mut K) -> String {
    k.start("alice", "k1", "p", "{}", IN).unwrap().value
}

fn message(text: &str) -> Out {
    Out {
        entry: Some(Entry {
            kind: Kind::Message,
            uuid: None,
            body: json!({"role": "assistant", "content": [{"type": "text", "text": text}]}),
        }),
        ..Out::default()
    }
}

fn delta(text: &str) -> Out {
    Out {
        update: Some(
            json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}),
        ),
        ..Out::default()
    }
}

/// A message that finishes an entry and has its ACP update.
fn whole(text: &str) -> Out {
    Out {
        update: delta(text).update,
        ..message(text)
    }
}

fn ask(id: &str) -> Out {
    Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body: json!({"id": id, "kind": "ask", "toolCallId": id, "toolCall": {"toolCallId": id, "title": "Edit"},
                "input": {}, "expires-at": 9000, "reach": "Edit(/w/**)"}),
        }),
        ..Out::default()
    }
}

fn idle() -> ProviderEvent {
    ProviderEvent::Turn(Turn::Idle {
        stop: Stop::Completed,
        seconds: None,
        tools: None,
        tokens: None,
    })
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn live(k: &mut K, id: &str) -> Option<String> {
    k.state
        .get(&format!("live/{id}"))
        .map(|b| String::from_utf8(b).unwrap())
}

fn pushed(k: &K, stream: &str) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream: s, value } if s == stream => Some(value.clone()),
            _ => None,
        })
        .collect()
}

fn count_calls(k: &K, f: impl Fn(&HostCall) -> bool) -> usize {
    k.host.calls.iter().filter(|c| f(c)).count()
}

fn dump(k: &K) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn attach(k: &mut K, session: &str, after: u64) -> String {
    match k.attach("alice", session, after).unwrap().value {
        Attached::Live { stream } => stream,
        other => panic!("not live: {other:?}"),
    }
}

fn watane(v: &Value) -> &Value {
    &v["_meta"]["watane"]
}

#[test]
fn ac1_attach_replays_then_synced_turn_and_queue() {
    let mut k = kernel();
    let id = started(&mut k);
    k.say("alice", &id, "q1", "queued").unwrap();
    let last = rec(&mut k, &id).meta().last;
    let o = attach(&mut k, &id, 0);
    assert_eq!(o, "t2");
    let key: Value = serde_json::from_slice(&k.state.get("stream/t2").unwrap()).unwrap();
    assert_eq!(key, json!({"session": id, "kind": "owner"}));
    assert_eq!(live(&mut k, &id).as_deref(), Some("t2"));
    assert_eq!(rec(&mut k, &id).streams, ["t1", "t2"]);
    let sent = pushed(&k, "t2");
    let n = last as usize;
    assert_eq!(sent.len(), n + 3, "{sent:?}");
    for (j, v) in sent[..n].iter().enumerate() {
        assert_eq!(watane(v)["type"], "entry");
        assert_eq!(watane(v)["entry"]["i"], j as u64 + 1);
    }
    assert_eq!(
        watane(&sent[n]),
        &json!({"type": "synced", "through": last})
    );
    assert_eq!(watane(&sent[n + 1])["type"], "turn");
    assert_eq!(watane(&sent[n + 2])["type"], "queue");
    assert_eq!(watane(&sent[n + 2])["inputs"].as_array().unwrap().len(), 1);
    // after the last entry: nothing to replay, still synced
    let mut k2 = kernel();
    let id2 = started(&mut k2);
    let last2 = rec(&mut k2, &id2).meta().last;
    attach(&mut k2, &id2, last2);
    let sent = pushed(&k2, "t2");
    assert_eq!(sent.len(), 3);
    assert_eq!(
        watane(&sent[0]),
        &json!({"type": "synced", "through": last2})
    );
}

#[test]
fn ac1_owner_check_and_unknown_session() {
    let mut k = kernel();
    let id = started(&mut k);
    assert!(matches!(
        k.attach("bob", &id, 0),
        Err(KernelError::NotOwner)
    ));
    assert!(matches!(
        k.attach("alice", "nope", 0),
        Err(KernelError::UnknownSession(_))
    ));
    assert_eq!(
        count_calls(
            &k,
            |c| matches!(c, HostCall::Open { stream } if stream == "t2")
        ),
        0
    );
}

#[test]
fn ac1_more_than_128_entries_is_behind_and_writes_nothing() {
    let mut k = kernel();
    let id = started(&mut k);
    for n in 0..130 {
        k.provider_item("t1", ProviderEvent::Out(message(&format!("m{n}"))))
            .unwrap();
    }
    let last = rec(&mut k, &id).meta().last;
    let before = dump(&k);
    let calls = k.host.calls.len() + 1; // the boot check of the call
    k.state.reset_counts();
    let c = k.attach("alice", &id, 0).unwrap();
    assert_eq!(c.value, Attached::Behind { through: last });
    assert_eq!(k.state.counts().puts + k.state.counts().deletes, 0);
    assert_eq!(dump(&k), before);
    assert_eq!(k.host.calls.len(), calls, "opens and pushes nothing");
    // 128 after `after` still fits
    let o = attach(&mut k, &id, last - 128);
    assert_eq!(o, "t2");
}

#[test]
fn ac2_a_new_attach_replaces_the_old_stream_in_that_call() {
    let mut k = kernel();
    let id = started(&mut k);
    let p = attach(&mut k, &id, 0);
    let q = attach(&mut k, &id, 0);
    assert_ne!(p, q);
    assert!(k.host.calls.contains(&HostCall::Close {
        stream: p.clone(),
        outcome: "replaced".into()
    }));
    assert!(k.state.get(&format!("stream/{p}")).is_none());
    assert_eq!(live(&mut k, &id).as_deref(), Some(q.as_str()));
    assert_eq!(rec(&mut k, &id).streams, ["t1", q.as_str()]);
    // the kernel resumed the provider stream right after the close (3, "Forwarding")
    assert_eq!(
        resume_after_close(&k, &p, "replaced"),
        Some("t1".to_string())
    );
    // the old stream's `ended` changes nothing
    let before = dump(&k);
    k.stream_ended(&p);
    assert_eq!(dump(&k), before);
    assert_eq!(live(&mut k, &id).as_deref(), Some(q.as_str()));
    assert_eq!(count_calls(&k, |c| matches!(c, HostCall::Resume { .. })), 1);
}

/// The stream resumed by the host call that directly follows the `Close` of `stream`.
fn resume_after_close(k: &K, stream: &str, outcome: &str) -> Option<String> {
    let at = k.host.calls.iter().position(|c| {
        *c == HostCall::Close {
            stream: stream.into(),
            outcome: outcome.into(),
        }
    })?;
    match k.host.calls.get(at + 1) {
        Some(HostCall::Resume { stream }) => Some(stream.clone()),
        _ => None,
    }
}

#[test]
fn ac2_a_lagging_close_resumes_the_provider_stream_and_none_means_no_resume() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    k.host.full.insert(o.clone());
    k.say("alice", &id, "q2", "next").unwrap();
    assert_eq!(
        resume_after_close(&k, &o, "lagging"),
        Some("t1".to_string())
    );
    // no live provider stream: the close resumes nothing
    k.provider_item("t1", ProviderEvent::Ended).unwrap();
    k.host.full.clear();
    let o2 = attach(&mut k, &id, 0);
    k.host.full.insert(o2.clone());
    let resumes = count_calls(&k, |c| matches!(c, HostCall::Resume { .. }));
    k.say("alice", &id, "q3", "again").unwrap();
    assert!(k.host.calls.contains(&HostCall::Close {
        stream: o2,
        outcome: "lagging".into()
    }));
    assert_eq!(
        count_calls(&k, |c| matches!(c, HostCall::Resume { .. })),
        resumes
    );
}

#[test]
fn ac3_every_push_of_a_call_goes_to_the_live_stream_in_order() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    let before = pushed(&k, &o).len();
    let c = k.say("alice", &id, "q2", "more").unwrap();
    assert!(!c.pushes.is_empty());
    let sent = pushed(&k, &o);
    // the kernel's own items go under `_meta.watane` on the wire, as in the replay
    assert_eq!(
        sent[before..].to_vec(),
        c.pushes
            .iter()
            .map(|p| json!({"_meta": {"watane": p.value}}))
            .collect::<Vec<_>>()
    );
    assert!(sent[before..].iter().any(|v| watane(v)["type"] == "queue"));
    // a provider update: pushed with `at`; `i` when it finishes an entry
    k.host.now = 2000;
    let c = k
        .provider_item("t1", ProviderEvent::Out(whole("whole")))
        .unwrap();
    let last = rec(&mut k, &id).meta().last;
    let v = pushed(&k, &o).pop().unwrap();
    assert_eq!(v["sessionUpdate"], "agent_message_chunk");
    assert_eq!(watane(&v), &json!({"at": 2000, "i": last}));
    assert_eq!(c.pushes.last().unwrap().value, v);
    // a bare update has `at` and no `i`
    k.provider_item("t1", ProviderEvent::Out(delta("y")))
        .unwrap();
    let v = pushed(&k, &o).pop().unwrap();
    assert_eq!(watane(&v), &json!({"at": 2000}));
    // the timer's pushes too
    let sent = pushed(&k, &o).len();
    k.provider_item("t1", idle()).unwrap();
    let new = pushed(&k, &o)[sent..].to_vec();
    assert!(
        new.iter().any(|v| watane(v)["type"] == "turn"),
        "a turn item, under `_meta.watane`: {new:?}"
    );
}

#[test]
fn ac4_a_delta_reads_the_two_keys_and_writes_nothing() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    let before = dump(&k);
    k.state.reset_counts();
    let c = k
        .provider_item("t1", ProviderEvent::Out(delta("a")))
        .unwrap();
    assert_eq!(c.value, Item::Done);
    assert_eq!(k.state.get_keys(), ["stream/t1", &format!("live/{id}")]);
    let n = k.state.counts();
    assert_eq!((n.puts, n.deletes), (0, 0));
    assert_eq!(dump(&k), before);
    let v = pushed(&k, &o).pop().unwrap();
    assert_eq!(v["content"]["text"], "a");
    // no live stream: still the same reads, nothing pushed
    k.stream_ended(&o);
    let sent = k.host.calls.len();
    k.state.reset_counts();
    k.provider_item("t1", ProviderEvent::Out(delta("b")))
        .unwrap();
    assert_eq!(k.state.get_keys(), ["stream/t1", &format!("live/{id}")]);
    assert_eq!(
        k.host.calls.len(),
        sent,
        "nothing pushed to a closed stream"
    );
}

#[test]
fn ac5_full_in_an_item_call_defers_and_rolls_back_whole() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    k.provider_item(
        "t1",
        ProviderEvent::Turn(Turn::Running {
            since: 1,
            activity: None,
        }),
    )
    .unwrap();
    let before = dump(&k);
    let calls = k.host.calls.len();
    k.host.full.insert(o.clone());
    // an entry with a request-free write, then a turn that would set the idle timer
    for ev in [ProviderEvent::Out(whole("a")), idle()] {
        let c = k.provider_item("t1", ev).unwrap();
        assert_eq!(c.value, Item::Defer);
        assert!(c.pushes.is_empty());
        assert_eq!(dump(&k), before);
        assert_eq!(
            k.host.calls.len(),
            calls,
            "no request, no timer: {:?}",
            &k.host.calls[calls..]
        );
    }
    // a delta defers too
    assert_eq!(
        k.provider_item("t1", ProviderEvent::Out(delta("d")))
            .unwrap()
            .value,
        Item::Defer
    );
    // the same item again, after `ready`, is applied once
    k.host.full.clear();
    k.stream_ready(&o);
    let c = k
        .provider_item("t1", ProviderEvent::Out(whole("a")))
        .unwrap();
    assert_eq!(c.value, Item::Done);
    let texts = k
        .read("alice", &id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .filter(|l| l.contains("\"a\""))
        .count();
    assert_eq!(texts, 1);
    // an idle that applies sets the timer as before
    let c = k.provider_item("t1", idle()).unwrap();
    assert_eq!(c.value, Item::Done);
    assert!(count_calls(&k, |c| matches!(c, HostCall::TimerAfter { .. })) >= 1);
}

#[test]
fn ac5_a_deferred_ask_opens_nothing_and_asks_nothing_of_the_host() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    k.host.full.insert(o.clone());
    let calls = k.host.calls.len();
    let before = dump(&k);
    let c = k
        .provider_item("t1", ProviderEvent::Out(ask("a1")))
        .unwrap();
    assert_eq!(c.value, Item::Defer);
    assert_eq!(dump(&k), before);
    assert_eq!(k.host.calls.len(), calls);
    assert!(rec(&mut k, &id).open.is_empty());
    // delivered again after `ready`, the ask opens once
    k.host.full.clear();
    k.stream_ready(&o);
    k.provider_item("t1", ProviderEvent::Out(ask("a1")))
        .unwrap();
    assert_eq!(rec(&mut k, &id).open.len(), 1);
}

#[test]
fn ac6_full_outside_an_item_call_closes_the_stream_lagging() {
    let mut k = kernel();
    let id = started(&mut k);
    k.provider_item("t1", idle()).unwrap();
    let o = attach(&mut k, &id, 0);
    k.host.full.insert(o.clone());
    let n = rec(&mut k, &id).meta().last;
    let c = k.say("alice", &id, "q2", "next").unwrap();
    // the call committed as it would
    assert!(c.value > n);
    assert!(rec(&mut k, &id).turn.is_active());
    assert!(k.host.calls.contains(&HostCall::Close {
        stream: o.clone(),
        outcome: "lagging".into()
    }));
    assert!(k.state.get(&format!("stream/{o}")).is_none());
    assert!(live(&mut k, &id).is_none());
    assert_eq!(rec(&mut k, &id).streams, ["t1"]);
    // the requests still went out
    assert!(k
        .host
        .calls
        .iter()
        .any(|c| matches!(c, HostCall::Request(Request::Turn { .. }))));
    // later pushes go nowhere; bytes and accounts agree after the close
    let sent = k.host.calls.len();
    k.provider_item("t1", ProviderEvent::Out(delta("z")))
        .unwrap();
    k.interrupt("alice", &id, None).unwrap();
    assert!(!k.host.calls[sent..]
        .iter()
        .any(|c| matches!(c, HostCall::Push { .. } | HostCall::Close { .. })));
    k.host.full.clear();
    let o2 = attach(&mut k, &id, 0);
    assert_ne!(o2, o);
    let r = rec(&mut k, &id);
    let owner = Usage::load(&mut k.state, &Usage::owner_key("alice"));
    assert_eq!(owner.bytes, Usage::load(&mut k.state, "quota").bytes);
    assert!(r.bytes > 0);
}

#[test]
fn ac6_each_other_call_closes_a_full_stream_too() {
    for which in 0..4 {
        let mut k = kernel();
        let id = started(&mut k);
        k.say("alice", &id, "q", "queued").unwrap();
        let o = attach(&mut k, &id, 0);
        k.host.full.insert(o.clone());
        match which {
            0 => drop(k.interrupt("alice", &id, None).unwrap()),
            1 => drop(k.unqueue("alice", &id, 1).unwrap()),
            2 => drop(k.cancel("alice", &id).unwrap()),
            _ => drop(k.rename("alice", &id, "t")),
        }
        let closed = k.host.calls.contains(&HostCall::Close {
            stream: o.clone(),
            outcome: "lagging".into(),
        });
        // rename pushes nothing, so it has no full stream to find
        assert_eq!(closed, which != 3, "call {which}");
        assert_eq!(live(&mut k, &id).is_none(), which != 3);
    }
}

#[test]
fn ac7_ready_resumes_the_provider_stream_of_an_owner_stream() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    let before = dump(&k);
    k.stream_ready(&o);
    assert_eq!(
        k.host.calls.last(),
        Some(&HostCall::Resume {
            stream: "t1".into()
        })
    );
    assert_eq!(dump(&k), before);
    // unknown ids and a provider stream change nothing
    let n = k.host.calls.len();
    k.stream_ready("zz");
    k.stream_ready("t1");
    assert_eq!(k.host.calls.len(), n);
}

#[test]
fn ac8_the_owner_streams_end_resumes_the_provider_and_stores_without_pushing() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    k.stream_ended(&o);
    assert_eq!(
        k.host.calls.last(),
        Some(&HostCall::Resume {
            stream: "t1".into()
        })
    );
    assert!(k.state.get(&format!("stream/{o}")).is_none());
    assert!(live(&mut k, &id).is_none());
    assert_eq!(rec(&mut k, &id).streams, ["t1"]);
    let sent = pushed(&k, &o).len();
    k.provider_item("t1", ProviderEvent::Out(whole("kept")))
        .unwrap();
    assert_eq!(pushed(&k, &o).len(), sent);
    assert!(k
        .read("alice", &id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .any(|l| l.contains("kept")));
    // a second `ended` finds no key
    let n = k.host.calls.len();
    k.stream_ended(&o);
    assert_eq!(k.host.calls.len(), n);
    let owner = Usage::load(&mut k.state, &Usage::owner_key("alice"));
    assert_eq!(owner.bytes, Usage::load(&mut k.state, "quota").bytes);
}

#[test]
fn ac9_after_a_restart_the_old_keys_go_and_attach_works() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    k.host.boot = 2;
    k.timer("restart");
    k.timer("restart");
    assert!(live(&mut k, &id).is_none());
    assert!(k.state.get(&format!("stream/{o}")).is_none());
    let sent = k.host.calls.len();
    k.say("alice", &id, "q", "again").unwrap();
    assert!(!k.host.calls[sent..]
        .iter()
        .any(|c| matches!(c, HostCall::Push { stream, .. } if *stream == o)));
    let o2 = attach(&mut k, &id, 0);
    assert_ne!(o2, o);
    assert_eq!(live(&mut k, &id).as_deref(), Some(o2.as_str()));
}

#[test]
fn ac10_delete_closes_the_live_owner_stream() {
    let mut k = kernel();
    let id = started(&mut k);
    let o = attach(&mut k, &id, 0);
    k.delete("alice", &id).unwrap();
    assert!(k.host.calls.contains(&HostCall::Close {
        stream: o.clone(),
        outcome: "closed".into()
    }));
    assert!(k.state.get(&format!("stream/{o}")).is_none());
    assert!(live(&mut k, &id).is_none());
    let owner = Usage::load(&mut k.state, &Usage::owner_key("alice"));
    assert_eq!(owner.bytes, Usage::load(&mut k.state, "quota").bytes);
}

#[test]
fn held_writes_are_seen_by_reads_and_dropped_or_applied_whole() {
    let mut m = Metered::new(MemState::new());
    m.put("a", b"1".to_vec()).unwrap();
    m.hold();
    m.put("a", b"22".to_vec()).unwrap();
    m.put("b", b"3".to_vec()).unwrap();
    m.delete("a");
    assert_eq!(m.get("a"), None);
    assert_eq!(m.get("b"), Some(b"3".to_vec()));
    m.discard();
    assert_eq!(m.get("a"), Some(b"1".to_vec()));
    assert_eq!(m.get("b"), None);
    m.hold();
    m.put("b", b"3".to_vec()).unwrap();
    m.delete("a");
    m.commit();
    assert_eq!((m.get("a"), m.get("b")), (None, Some(b"3".to_vec())));
    m.hold();
    assert!(m.put("c", vec![0; MAX_VALUE + 1]).is_err());
    m.discard();
}

#[test]
fn ac1_attach_charges_both_keys_and_writes_past_the_quota() {
    let mut k = kernel();
    let id = started(&mut k);
    let owner0 = Usage::load(&mut k.state, &Usage::owner_key("alice")).bytes;
    let rec0 = rec(&mut k, &id).bytes;
    k.limits.owner_bytes = owner0;
    let o = attach(&mut k, &id, 0);
    let cost = (format!("stream/{o}").len()
        + k.state.get(&format!("stream/{o}")).unwrap().len()
        + format!("live/{id}").len()
        + o.len()) as u64;
    let r = rec(&mut k, &id);
    let owner = Usage::load(&mut k.state, &Usage::owner_key("alice")).bytes;
    assert!(owner - owner0 >= cost, "{owner} - {owner0} < {cost}");
    assert!(r.bytes - rec0 >= cost);
    assert_eq!(owner, Usage::load(&mut k.state, "quota").bytes);
}

/// S26-fix1 AC3, 9.1 `stream/<id>`: an item on an owner stream's id is no provider item.
#[test]
fn ac3_owner_stream_id_is_not_a_provider_item() {
    let mut k = kernel();
    let s = started(&mut k);
    let o = attach(&mut k, &s, 0);
    let slots = Slots::load(&mut k.state).provider;
    let owner = Usage::load(&mut k.state, &Usage::owner_key("alice")).provider_at(1);
    assert_eq!((slots, owner), (1, 1));
    let before = dump(&k);
    let calls = k.host.calls.len();
    let a = k.provider_item(&o, ProviderEvent::Ended).unwrap();
    assert!(matches!(a.value, Item::Done) && a.pushes.is_empty());
    let b = k.provider_item(&o, ProviderEvent::Out(delta("x"))).unwrap();
    assert!(matches!(b.value, Item::Done) && b.pushes.is_empty());
    assert_eq!(dump(&k), before);
    assert_eq!(Slots::load(&mut k.state).provider, slots);
    let after = Usage::load(&mut k.state, &Usage::owner_key("alice")).provider_at(1);
    assert_eq!(after, owner);
    // nothing sent: no push on the owner stream, no request, no stream closed
    assert_eq!(k.host.calls.len(), calls);
}
