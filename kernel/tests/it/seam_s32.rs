//! S32-1: the kernel speaks host 19 (TECHNICAL 2.1 "The kernel's side of the seam", 7 timers, 9.1
//! `stream/<id>`, 9.4, case 7): fallible owner streams, provider streams only from the provider's
//! `start`, `push` failures, `close` outcomes, the 30-day timer limit, numbered batches.

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
    st.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
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

fn dump<H: Host>(k: &Kernel<MemState, H>) -> Vec<(String, Vec<u8>)> {
    k.state
        .entries()
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect()
}

fn rec<H: Host>(k: &mut Kernel<MemState, H>, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn msg(text: &str) -> ProviderEvent {
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Message,
            uuid: None,
            body: json!({"role": "agent", "content": [{"type": "text", "text": text}]}),
        }),
        update: Some(
            json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}),
        ),
        ..Out::default()
    })
}

fn texts<H: Host>(k: &mut Kernel<MemState, H>, id: &str) -> Vec<String> {
    let page = k.read("alice", id, 0, None, 256).unwrap();
    page.entries
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|e| e["kind"] == "message" && e["body"]["role"] == "agent")
        .map(|e| {
            e["body"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

fn attached<H: Host>(k: &mut Kernel<MemState, H>, id: &str, after: u64) -> String {
    match k.attach("alice", id, after).unwrap().value {
        Attached::Live { stream } => stream,
        other => panic!("not live: {other:?}"),
    }
}

/// A host whose `push` fails with a chosen error, to meet every `stream-error`.
#[derive(Default)]
struct Flaky {
    inner: FakeHost,
    push_error: Option<StreamError>,
}

impl Host for Flaky {
    fn generation(&mut self) -> u64 {
        self.inner.generation()
    }
    fn now(&mut self) -> u64 {
        self.inner.now()
    }
    fn new_id(&mut self) -> String {
        self.inner.new_id()
    }
    fn timer_after(&mut self, id: &str, delay_ms: u64) {
        self.inner.timer_after(id, delay_ms)
    }
    fn timer_cancel(&mut self, id: &str) {
        self.inner.timer_cancel(id)
    }
    fn request(&mut self, r: Request) {
        self.inner.request(r)
    }
    fn open_stream(&mut self) -> Result<String, StreamError> {
        self.inner.open_stream()
    }
    fn start_provider(&mut self, launch: &str, input: &str) -> Result<Started, String> {
        self.inner.start_provider(launch, input)
    }
    fn cancel_stream(&mut self, stream: &str) {
        self.inner.cancel_stream(stream)
    }
    fn push(&mut self, stream: &str, value: Value) -> Result<(), StreamError> {
        match self.push_error {
            Some(e) => Err(e),
            None => self.inner.push(stream, value),
        }
    }
    fn close_stream(&mut self, stream: &str, outcome: Outcome) {
        self.inner.close_stream(stream, outcome)
    }
    fn resume_stream(&mut self, stream: &str) {
        self.inner.resume_stream(stream)
    }
}

#[test]
fn ac1_stream_error_codes() {
    assert_eq!(StreamError::NotYours.code(), "not-yours");
    assert_eq!(StreamError::UnknownStream.code(), "unknown-stream");
    assert_eq!(StreamError::Full.code(), "full");
    assert_eq!(StreamError::TooLarge.code(), "too-large");
}

#[test]
fn ac2_an_owner_stream_that_cannot_open_changes_nothing() {
    for (err, code) in [
        (StreamError::NotYours, "not-yours"),
        (StreamError::Full, "full"),
    ] {
        let mut k = kernel();
        let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
        let before = dump(&k);
        let record = rec(&mut k, &id);
        k.host.open_error = Some(err);
        match k.attach("alice", &id, 0) {
            Err(KernelError::Backend(m)) => assert!(m.starts_with(code), "{m}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(dump(&k), before);
        assert!(k.state.get("live/s1").is_none());
        assert_eq!(rec(&mut k, &id).streams, record.streams);
    }
}

#[test]
fn ac3_no_provider_refuses_a_claude_start_and_writes_nothing() {
    let mut k = kernel();
    k.host.no_provider = true;
    let before = dump(&k);
    assert_eq!(
        k.start("alice", "k1", "p", "{}", IN).unwrap_err(),
        KernelError::Backend("no Claude Code provider is linked".into())
    );
    assert_eq!(dump(&k), before);
    assert!(k
        .host
        .calls
        .iter()
        .all(|c| !matches!(c, HostCall::StartProvider { .. })));
}

#[test]
fn ac3_a_provider_start_gives_the_provider_stream() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    // S35: the launch is the document of 4.1, not the profile as stored.
    let launch = k.host.calls.iter().find_map(|c| match c {
        HostCall::StartProvider { launch, input } => {
            assert_eq!(input, IN);
            Some(serde_json::from_str::<Value>(launch).unwrap())
        }
        _ => None,
    });
    assert_eq!(
        launch.unwrap(),
        json!({"version":1,"profile":"p","workspace":"scratch","mode":"manual","tools":["Read"],"max-turns":20})
    );
    assert_eq!(rec(&mut k, &id).streams, ["t1"]);
    assert!(k.state.get("stream/t1").is_some());
}

fn gone_scenario(err: Option<StreamError>) {
    let mut k = Kernel::new(
        {
            let mut st = MemState::new();
            st.put("profile/p", P.as_bytes().to_vec()).unwrap();
            st.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
            Slots {
                boot: 1,
                ..Slots::default()
            }
            .save(&mut st)
            .unwrap();
            st
        },
        Flaky {
            inner: FakeHost {
                generation: 1,
                now: 1000,
                ..FakeHost::default()
            },
            push_error: None,
        },
    );
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let last = rec(&mut k, &id).next_index - 1;
    let o = attached(&mut k, &id, last);
    k.host.push_error = err;
    k.provider(&id, msg("one")).unwrap();
    // the live stream is dropped, nothing is closed, the entry is kept
    assert!(k.state.get(&format!("stream/{o}")).is_none());
    assert!(k.state.get(&format!("live/{id}")).is_none());
    assert!(!rec(&mut k, &id).streams.contains(&o));
    assert!(k
        .host
        .inner
        .calls
        .iter()
        .all(|c| !matches!(c, HostCall::Close { .. })));
    assert_eq!(texts(&mut k, &id), ["one"]);
    // the next attach replays it once
    k.host.push_error = None;
    let o2 = attached(&mut k, &id, last);
    let replayed: Vec<Value> = k
        .host
        .inner
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Push { stream, value } if *stream == o2 => Some(value.clone()),
            _ => None,
        })
        .filter(|v| v["_meta"]["watane"]["type"] == "entry")
        .collect();
    assert_eq!(replayed.len(), 1);
}

#[test]
fn ac4_a_push_to_an_ended_owner_stream_drops_it_without_a_close() {
    for e in [
        StreamError::UnknownStream,
        StreamError::NotYours,
        StreamError::TooLarge,
    ] {
        gone_scenario(Some(e));
    }
}

#[test]
fn ac4_a_fake_gone_stream_is_dropped_in_an_item_call() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let o = attached(&mut k, &id, 0);
    k.host.gone.insert(o.clone());
    let c = k.provider_item("t1", msg("one")).unwrap();
    assert_eq!(c.value, SinkOutcome::Taken);
    assert!(k.state.get(&format!("stream/{o}")).is_none());
    assert!(k.state.get(&format!("live/{id}")).is_none());
    assert!(k
        .host
        .calls
        .iter()
        .all(|c| !matches!(c, HostCall::Close { .. })));
    assert_eq!(texts(&mut k, &id), ["one"]);
}

#[test]
fn ac5_close_passes_the_outcome() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    let a = attached(&mut k, &id, 0);
    let b = attached(&mut k, &id, 0);
    k.host.full.insert(b.clone());
    k.provider(&id, msg("x")).unwrap();
    let c = attached(&mut k, &id, 0);
    k.delete("alice", &id).unwrap();
    let closes: Vec<_> = k
        .host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Close { stream, outcome } => Some((stream.clone(), *outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        closes,
        [
            (a, Outcome::Replaced),
            (b, Outcome::Lagging),
            (c, Outcome::Closed)
        ]
    );
}

fn far(k: &mut K, id: &str, at: u64) {
    let mut r = rec(k, id);
    r.deadlines = vec![Deadline {
        what: "retry".into(),
        at,
    }];
    r.save(&mut k.state).unwrap();
}

fn delays(k: &K, from: usize) -> Vec<u64> {
    k.host.calls[from..]
        .iter()
        .filter_map(|c| match c {
            HostCall::TimerAfter { delay_ms, .. } => Some(*delay_ms),
            _ => None,
        })
        .collect()
}

#[test]
fn ac6_a_deadline_over_30_days_out_is_armed_at_30_days_and_hopped() {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.provider(&id, ProviderEvent::Turn(Turn::default()))
        .unwrap();
    let day = 24 * 60 * 60 * 1000;
    let at = 1000 + 70 * day;
    far(&mut k, &id, at);
    let timer = format!("s/{id}");
    // a firing that finds nothing due arms the rest, at most 30 days
    let from = k.host.calls.len();
    k.timer(&timer);
    assert_eq!(delays(&k, from), [MAX_TIMER_MS]);
    // the second hop
    k.host.now = 1000 + MAX_TIMER_MS;
    let from = k.host.calls.len();
    k.timer(&timer);
    assert_eq!(delays(&k, from), [MAX_TIMER_MS]);
    // the third finds 10 days left
    k.host.now = 1000 + 2 * MAX_TIMER_MS;
    let from = k.host.calls.len();
    k.timer(&timer);
    assert_eq!(delays(&k, from), [at - k.host.now]);
    // the last finds the deadline due: nothing more is armed
    k.host.now = at;
    let from = k.host.calls.len();
    k.timer(&timer);
    assert!(delays(&k, from).is_empty());
    for c in &k.host.calls {
        if let HostCall::TimerAfter { delay_ms, .. } = c {
            assert!(*delay_ms <= MAX_TIMER_MS);
        }
    }
}

fn batch_kernel() -> (K, String) {
    let mut k = kernel();
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    (k, id)
}

fn seq_of(k: &mut K, stream: &str) -> u64 {
    let b = k.state.get(&format!("stream/{stream}")).unwrap();
    serde_json::from_slice::<Value>(&b).unwrap()["last"]
        .as_u64()
        .unwrap_or(0)
}

#[test]
fn ac7_a_numbered_batch_takes_each_item_once() {
    let (mut k, id) = batch_kernel();
    let o = attached(&mut k, &id, 0);
    let items = |r: std::ops::RangeInclusive<u64>| -> Vec<(u64, ProviderEvent)> {
        r.map(|n| (n, msg(&format!("m{n}")))).collect()
    };
    let c = k.provider_batch("t1", items(1..=3)).unwrap();
    assert_eq!(c.value, SinkOutcome::Taken);
    assert_eq!(texts(&mut k, &id), ["m1", "m2", "m3"]);
    assert_eq!(seq_of(&mut k, "t1"), 3);
    // the same batch again changes nothing and pushes nothing
    let before = dump(&k);
    let calls = k.host.calls.len();
    let c = k.provider_batch("t1", items(1..=3)).unwrap();
    assert_eq!(c.value, SinkOutcome::Taken);
    assert!(c.pushes.is_empty());
    assert_eq!(dump(&k), before);
    assert_eq!(k.host.calls.len(), calls);
    // 2..5 takes only 4 and 5
    k.provider_batch("t1", items(2..=5)).unwrap();
    assert_eq!(texts(&mut k, &id), ["m1", "m2", "m3", "m4", "m5"]);
    assert_eq!(seq_of(&mut k, "t1"), 5);
    let pushes = k
        .host
        .calls
        .iter()
        .filter(|c| matches!(c, HostCall::Push { stream, .. } if *stream == o))
        .count();
    assert!(pushes >= 5);
}

#[test]
fn ac7_items_are_taken_in_seq_order() {
    let (mut k, id) = batch_kernel();
    let batch = vec![(3, msg("c")), (1, msg("a")), (2, msg("b"))];
    k.provider_batch("t1", batch).unwrap();
    assert_eq!(texts(&mut k, &id), ["a", "b", "c"]);
}

/// A state that logs each key written.
#[derive(Default)]
struct Spy {
    inner: MemState,
    puts: Vec<String>,
}

impl State for Spy {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.inner.get(key)
    }
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        self.puts.push(key.to_string());
        self.inner.put(key, value)
    }
    fn delete(&mut self, key: &str) {
        self.inner.delete(key)
    }
}

#[test]
fn ac7_the_last_seq_is_one_write_per_batch_and_the_bytes_stay_exact() {
    let mut st = Spy::default();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    st.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
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
    let mut k = Kernel::new(st, host);
    let id = k.start("alice", "k1", "p", "{}", IN).unwrap().value;
    k.state.puts.clear();
    let items: Vec<_> = (1..=3).map(|n| (n, msg("x"))).collect();
    k.provider_batch("t1", items).unwrap();
    let n = k.state.puts.iter().filter(|x| *x == "stream/t1").count();
    assert_eq!(n, 1);
    // the record's bytes are those of its session keys
    let r = Record::load(&mut k.state, &id).unwrap();
    let mut sum = 0u64;
    for (key, v) in k.state.inner.entries() {
        if ["entries/", "queue/", "stream/", "live/"]
            .iter()
            .any(|p| key.starts_with(p))
        {
            sum += (key.len() + v.len()) as u64;
        }
    }
    assert_eq!(r.bytes, sum);
}

#[test]
fn ac7_a_batch_with_a_deferring_item_is_dropped_whole() {
    let (mut k, id) = batch_kernel();
    let o = attached(&mut k, &id, 0);
    let before = dump(&k);
    k.host.full.insert(o);
    let batch = vec![(1, msg("a")), (2, msg("b")), (3, msg("c"))];
    let c = k.provider_batch("t1", batch.clone()).unwrap();
    assert_eq!(c.value, SinkOutcome::Defer);
    assert_eq!(dump(&k), before);
    assert_eq!(seq_of(&mut k, "t1"), 0);
    // sent again once there is room, it is taken whole
    k.host.full.clear();
    assert_eq!(
        k.provider_batch("t1", batch).unwrap().value,
        SinkOutcome::Taken
    );
    assert_eq!(texts(&mut k, &id), ["a", "b", "c"]);
}

#[test]
fn ac7_a_batch_on_an_unknown_stream_changes_nothing() {
    let (mut k, _) = batch_kernel();
    let before = dump(&k);
    let c = k.provider_batch("nope", vec![(1, msg("a"))]).unwrap();
    assert_eq!(c.value, SinkOutcome::Taken);
    assert_eq!(dump(&k), before);
}

#[test]
fn validator_a_firing_that_arms_a_far_deadline_hops_then_arms_the_rest() {
    // a firing whose next deadline is 50 days out arms 30 days; that firing finds nothing due
    // and arms the 20 days left (7: a nothing-due firing re-arms the rest)
    let mut k = kernel();
    let id = k.start("alice", "p1", "p", "{}", IN).unwrap().value;
    let day = 24 * 60 * 60 * 1000;
    let at = 2000 + 50 * day;
    let mut r = rec(&mut k, &id);
    r.deadlines.push(Deadline {
        what: "retry".into(),
        at,
    });
    r.save(&mut k.state).unwrap();
    k.host.now = 2000;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    assert_eq!(delays(&k, from), [MAX_TIMER_MS]);
    k.host.now = 2000 + MAX_TIMER_MS;
    let from = k.host.calls.len();
    k.timer(&format!("s/{id}"));
    assert_eq!(delays(&k, from), [at - k.host.now]);
}

#[test]
fn validator_a_seq_twice_in_one_batch_is_taken_once() {
    let (mut k, id) = batch_kernel();
    let batch = vec![(1, msg("a")), (1, msg("a")), (2, msg("b"))];
    k.provider_batch("t1", batch).unwrap();
    assert_eq!(texts(&mut k, &id), ["a", "b"]);
    assert_eq!(seq_of(&mut k, "t1"), 2);
}
