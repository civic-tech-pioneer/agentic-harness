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
