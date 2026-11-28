//! S35 review round 1 (Codex): the wrapper's per-call rule (`call::run`, TECHNICAL 2.1 "A failed
//! call commits nothing") and `Buffered`'s order.

use agent_kernel::active::Slots;
use agent_kernel::call;
use agent_kernel::*;
use serde_json::Value;

const P: &str =
    r#"{"backend":"claude","model":"sonnet","tools":["Read"],"allow":["Read(**)"],"partial":true}"#;
const IN: &str = r#"[{"type":"text","text":"hello"}]"#;

fn seeded() -> MemState {
    let mut st = MemState::new();
    st.put("profile/p", P.as_bytes().to_vec()).unwrap();
    crate::fixture_access::put(&mut st, "access/alice", br#"["p"]"#.to_vec()).unwrap();
    st
}

fn host(generation: u64) -> FakeHost {
    FakeHost {
        generation,
        now: 1000,
        ..FakeHost::default()
    }
}

fn slots(st: &mut MemState) -> String {
    String::from_utf8(st.get("slots").expect("slots are stored")).unwrap()
}

fn timers(h: &FakeHost) -> Vec<String> {
    h.calls
        .iter()
        .filter_map(|c| match c {
            HostCall::TimerAfter { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// F1.1: on watane c2a7d4d a refused call on a new host generation armed `restart` before it was
/// refused; the buffer dropped the call's `slots`, the timer stayed, and its firing moved
/// `slots.boot` from 4 to 5 with no client call between. A refused call leaves persisted state
/// and pending timers as they were.
#[test]
fn f1_1_a_refused_call_on_a_new_generation_arms_no_timer_and_leaves_slots() {
    let (r, mut st, _) = call::run(seeded(), host(4), |k| {
        k.start("alice", "k1", "p", "{}", IN).map(|c| c.value)
    });
    r.unwrap();
    let before_slots = slots(&mut st);
    assert_eq!(
        serde_json::from_str::<Slots>(&before_slots).unwrap().boot,
        4
    );

    // The host opens again (generation 5); the first call is refused.
    let (r, mut st, h) = call::run(st, host(5), |k| {
        k.start("alice", "k2", "no-such-profile", "{}", IN)
            .map(|c| c.value)
    });
    assert!(r.is_err(), "{r:?}");
    let armed = timers(&h);
    assert_eq!(slots(&mut st), before_slots, "the refused call wrote slots");

    // Whatever it armed fires, as the host would, before any client call.
    for id in &armed {
        let (_, next, _) = call::run(st, host(5), |k| {
            k.timer(id);
            Ok(())
        });
        st = next;
    }
    let after_slots = slots(&mut st);
    assert_eq!(
        after_slots, before_slots,
        "timers armed by the refused call: {armed:?}"
    );
    assert!(armed.is_empty(), "the refused call armed {armed:?}");
}

/// A host whose provider refuses its first `start` with `full`, after the kernel took an id.
struct RefuseFirst {
    inner: FakeHost,
    refused: bool,
}

impl Host for RefuseFirst {
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
        if !self.refused {
            self.refused = true;
            return Err("full".into());
        }
        self.inner.start_provider(launch, input)
    }
    fn cancel_stream(&mut self, stream: &str) {
        self.inner.cancel_stream(stream)
    }
    fn push(&mut self, stream: &str, value: Value) -> Result<(), StreamError> {
        self.inner.push(stream, value)
    }
    fn close_stream(&mut self, stream: &str, outcome: Outcome) {
        self.inner.close_stream(stream, outcome)
    }
    fn resume_stream(&mut self, stream: &str) {
        self.inner.resume_stream(stream)
    }
}

/// F1.2: a `start` the provider refuses after the kernel took the id `s1` gives that id back: the
/// next `start` gets `s1` (with the buffer off it gets `s2`; checked by hand, S35 r1).
#[test]
fn f1_2_a_start_refused_after_its_id_was_taken_gives_the_id_back() {
    let h = RefuseFirst {
        inner: host(1),
        refused: false,
    };
    let (r, st, h) = call::run(seeded(), h, |k| {
        k.start("alice", "k1", "p", "{}", IN).map(|c| c.value)
    });
    assert!(
        matches!(&r, Err(KernelError::Backend(m)) if m == "full"),
        "{r:?}"
    );
    let (r, _, _) = call::run(st, h, |k| {
        k.start("alice", "k2", "p", "{}", IN).map(|c| c.value)
    });
    assert_eq!(r.unwrap(), "s1");
}

/// A state that records what it was asked to write, in order.
#[derive(Debug, Default)]
struct Trace(Vec<(&'static str, String)>);

impl State for Trace {
    fn get(&mut self, _key: &str) -> Option<Vec<u8>> {
        None
    }
    fn put(&mut self, key: &str, _value: Vec<u8>) -> Result<(), TooLarge> {
        self.0.push(("put", key.to_string()));
        Ok(())
    }
    fn delete(&mut self, key: &str) {
        self.0.push(("delete", key.to_string()));
    }
}

/// F1.3: S35-2 AC5, the buffered puts and deletes commit in call order (the review's red test).
#[test]
fn f1_3_buffered_commits_in_call_order() {
    let mut state = Buffered::new(Trace::default());
    state.put("z", vec![1]).unwrap();
    state.put("a", vec![2]).unwrap();
    state.delete("z");
    assert_eq!(
        state.commit().0,
        [
            ("put", "z".into()),
            ("put", "a".into()),
            ("delete", "z".into()),
        ]
    );
}
