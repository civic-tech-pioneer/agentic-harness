//! Adversary, S33: attacks on the reach of a session allow (TECHNICAL 9.6, 14).

use agent_kernel::broker::reach;
use serde_json::json;

/// A `Glob` whose `pattern` is absolute searches the pattern's own base folder, not `path`
/// (Claude Code's Glob: `getPath` returns the pattern's base dir when it is absolute, and the
/// search runs there). The kernel reads only `path`, so a `Glob(<ws>/**)` allow the owner gave
/// for a search in the workspace also answers a search of `/home` or `/etc`.
#[test]
fn adv_s33_glob_absolute_pattern_escapes_workspace() {
    let ws = Some("/w");
    let seen = reach(
        "alice",
        ws,
        "ask",
        "Glob",
        &json!({"path": "/w/src", "pattern": "*.rs"}),
        Some("/w/src"),
    );
    assert_eq!(seen.as_deref(), Some("Glob(/w/**)"));
    let wider = reach(
        "alice",
        ws,
        "ask",
        "Glob",
        &json!({"path": "/w/src", "pattern": "/home/*/.ssh/*"}),
        Some("/w/src"),
    );
    assert_ne!(
        wider, seen,
        "a Glob of /home/*/.ssh/* must not match the workspace allow"
    );
}
