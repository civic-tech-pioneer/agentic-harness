//! S36-4: what the stage B run on watane's host found, each at the lowest level that shows it.

use agent_kernel::adapters::claude::{facts, Fact};
use serde_json::json;

/// A `system/init` whose `session_id` is not a uuid (dev's fake `claude` says `fake-session`) keeps
/// no Claude session: host 19.3 refuses a `resume` that is not a uuid (`bad-launch`), so Continue
/// after a restart would be refused for good; with none kept, the next launch starts a new one.
#[test]
fn s36_4_a_session_id_that_is_not_a_uuid_is_not_kept() {
    let line =
        |id: &str| json!({"type":"system","subtype":"init","session_id":id,"cwd":"/w"}).to_string();
    let kept = |id: &str| {
        facts(line(id).as_bytes())
            .into_iter()
            .any(|f| matches!(f, Fact::Session(_)))
    };
    assert!(!kept("fake-session"));
    assert!(!kept("5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b1"));
    assert!(!kept("5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11-0"));
    assert!(!kept("5b1f0a52x3c1e-4c53-9d0e-7a1d3f6c2b11"));
    assert!(kept("5b1f0a52-3c1e-4c53-9d0e-7a1d3f6c2b11"));
    assert!(kept("5B1F0A52-3C1E-4C53-9D0E-7A1D3F6C2B11"));
}

mod owed_idle {
    use agent_kernel::*;

    type K = Kernel<MemState, FakeHost>;
    const IDLE: &str = r#"{"type":"system","subtype":"session_state_changed","state":"idle"}"#;
    const RESULT: &str = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":1,"duration_api_ms":1,"num_turns":1,"session_id":"s","total_cost_usd":0,"usage":{"input_tokens":1,"output_tokens":1}}"#;

    fn started() -> (K, String, String) {
        let mut k: K = Kernel::new(MemState::new(), FakeHost::default());
        let id = k
            .start(
                "page",
                "k",
                "scratch",
                "{}",
                r#"[{"type":"text","text":"go"}]"#,
            )
            .unwrap()
            .value;
        k.attach("page", &id, 0).unwrap();
        let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
        (k, id, t)
    }

    fn turns(k: &mut K, id: &str) -> Vec<String> {
        k.read("page", id, 0, None, 256)
            .unwrap()
            .entries
            .iter()
            .map(|e| serde_json::from_str::<serde_json::Value>(e).unwrap())
            .filter(|e| e["kind"] == "turn")
            .map(|e| e["body"].to_string())
            .collect()
    }

    /// S36-4 (host run): Stop ends the turn `idle {interrupted}` at once; `claude`'s own `idle` for
    /// that turn, which comes later, ends nothing more (no second `turn` entry, no `completed`).
    #[test]
    fn s36_4_the_idle_of_an_interrupted_turn_ends_nothing_more() {
        let (mut k, id, t) = started();
        k.interrupt("page", &id, None).unwrap();
        let n = turns(&mut k, &id).len();
        k.provider_lines(&t, vec![(1, RESULT.into()), (2, IDLE.into())])
            .unwrap();
        let after = turns(&mut k, &id);
        assert_eq!(after.len(), n, "a turn entry after Stop: {after:?}");
        assert!(after.last().unwrap().contains("interrupted"));
        // the next turn still ends at its own idle
        k.say("page", &id, "k2", r#"[{"type":"text","text":"again"}]"#)
            .unwrap();
        k.provider_lines(&t, vec![(3, RESULT.into()), (4, IDLE.into())])
            .unwrap();
        let last = turns(&mut k, &id);
        assert!(last.last().unwrap().contains("completed"), "{last:?}");
    }

    /// Stop and send: the interrupted turn's late `idle` does not end the replacement's turn.
    #[test]
    fn s36_4_the_late_idle_does_not_end_the_replacement() {
        let (mut k, id, t) = started();
        k.interrupt("page", &id, Some(r#"[{"type":"text","text":"instead"}]"#))
            .unwrap();
        k.provider_lines(&t, vec![(1, RESULT.into()), (2, IDLE.into())])
            .unwrap();
        let rec = Record::load(&mut k.state, &id).unwrap();
        assert!(
            rec.turn.is_active(),
            "the replacement's turn ended: {:?}",
            rec.turn
        );
        k.provider_lines(&t, vec![(3, RESULT.into()), (4, IDLE.into())])
            .unwrap();
        let rec = Record::load(&mut k.state, &id).unwrap();
        assert!(!rec.turn.is_active());
    }
}
