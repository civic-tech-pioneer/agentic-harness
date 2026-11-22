//! The launch document a Claude session's provider `start` gets (TECHNICAL 4.1, 4.6, 9.2): the
//! compiled profile less the kernel's own fields, with the kernel's `profile`, `workspace` and
//! `session`.

use crate::profile::Profile;
use serde_json::{json, Value};

/// The profile fields only the kernel reads (9.2): none goes to `claude`.
const KERNEL_ONLY: [&str; 7] = [
    "backend",
    "toolsets",
    "delegates",
    "max-tool-errors",
    "max-tokens",
    "ask-expiry",
    "subagent-limits",
];

/// A name host 19.3 takes as the launch's `profile`: `[A-Za-z0-9._-]{1,64}`.
fn is_launch_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// The launch of `compiled` (the profile with its overlay applied) for the session's profile
/// `name`, in `workspace` (4.6), resuming the Claude session `resume` when given. `base_tools`:
/// the profile named tools before the overlay, so a `tools: []` an overlay made is kept and an
/// absent one stays absent (4.1: both mean no built-in tools).
pub fn launch(
    name: &str,
    compiled: &Profile,
    base_tools: bool,
    workspace: &str,
    resume: Option<&str>,
) -> String {
    let Ok(Value::Object(mut doc)) = serde_json::to_value(compiled) else {
        unreachable!("a profile serializes to an object");
    };
    for k in KERNEL_ONLY {
        doc.remove(k);
    }
    if !base_tools && compiled.tools.is_empty() {
        doc.remove("tools");
    }
    doc.insert("version".into(), json!(1));
    if is_launch_name(name) {
        doc.insert("profile".into(), json!(name));
    }
    doc.insert("workspace".into(), json!(workspace));
    if let Some(uuid) = resume {
        doc.insert("session".into(), json!({ "resume": uuid }));
    }
    Value::Object(doc).to_string()
}
