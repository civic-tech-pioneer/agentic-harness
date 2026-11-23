//! S36-5 AC5: the state bytes per finished entry at stage B's load (TECHNICAL 17, row B): a Claude
//! session on the fake's load (10 text deltas, then one finished entry of 4 KiB), with transcripts of
//! 10 and 10 000 entries. Each entry's call reads and writes at most 2 MiB, within 10% between the two.
//! Slow (it streams 10 000 entries through the kernel): `cargo test -- --ignored s36_load`.

use agent_kernel::*;
use serde_json::json;

type K = Kernel<MemState, FakeHost>;

const IN: &str = r#"[{"type":"text","text":"load"}]"#;

fn delta(n: u64) -> Vec<u8> {
    json!({"type":"stream_event","session_id":"fake","uuid":"u","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":format!("delta {n} ")}}})
        .to_string()
        .into_bytes()
}

fn entry(n: u64) -> Vec<u8> {
    let mut text = format!("entry {n} ");
    text.extend(std::iter::repeat_n('x', 4096 - text.len()));
    json!({"type":"assistant","session_id":"fake","uuid":format!("a{n}"),"message":{"id":format!("m{n}"),"type":"message","role":"assistant","model":"fake","content":[{"type":"text","text":text}],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}}})
        .to_string()
        .into_bytes()
}

/// The bytes read and written per finished entry, after `n` entries on an attached session: the
/// mean over the next 30 entries (two chunks' worth, so where the chunk boundary falls does not
/// decide it), each after its 10 deltas (one call each), as the fake's load writes them.
fn cost_after(n: u64) -> u64 {
    let mut k: K = Kernel::new(MemState::new(), FakeHost::default());
    let id = k.start("page", "k", "scratch", "{}", IN).unwrap().value;
    k.attach("page", &id, 0).unwrap();
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let mut seq = 0;
    let mut feed = |k: &mut K, body: Vec<u8>| {
        seq += 1;
        k.provider_lines(&t, vec![(seq, body)]).unwrap();
    };
    for e in 1..=n {
        feed(&mut k, entry(e));
    }
    k.state.reset_counts();
    for e in n + 1..=n + 30 {
        for d in 0..10 {
            feed(&mut k, delta(d));
        }
        feed(&mut k, entry(e));
    }
    let c = k.state.counts();
    (c.bytes_read + c.bytes_written) / 30
}

#[test]
#[ignore = "slow: streams 10 000 entries through the kernel (S36-5, the stage B budget)"]
fn ac5_state_bytes_per_entry_at_10_and_10_000() {
    let small = cost_after(10);
    let large = cost_after(10_000);
    println!("state bytes per finished entry: {small} at 10 entries, {large} at 10 000");
    assert!(
        small <= 2 << 20 && large <= 2 << 20,
        "{small} and {large}: over 2 MiB"
    );
    let ratio = large as f64 / small as f64;
    assert!(
        (0.9..=1.1).contains(&ratio),
        "{large} / {small} = {ratio}: not within 10%"
    );
}

/// Case 56, P09 (found by the measure above): an items call of one text delta reads the stream's
/// key, its adapter state and `live/<session>`, writes only the stream's `last`, and reads no record
/// or account, whatever the transcript's length.
#[test]
fn case56_a_text_delta_reads_no_record() {
    let mut k: K = Kernel::new(MemState::new(), FakeHost::default());
    let id = k.start("page", "k", "scratch", "{}", IN).unwrap().value;
    k.attach("page", &id, 0).unwrap();
    let t = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    k.provider_lines(&t, vec![(1, entry(1))]).unwrap();
    k.state.reset_counts();
    k.provider_lines(&t, vec![(2, delta(0))]).unwrap();
    let keys = k.state.get_keys().to_vec();
    assert!(
        keys.iter().all(|key| key == &format!("stream/{t}")
            || key == &format!("adapter/{t}")
            || key == &format!("live/{id}")),
        "a delta read {keys:?}"
    );
    assert_eq!(
        k.state.counts().puts,
        1,
        "a delta wrote more than the stream's last"
    );
}
