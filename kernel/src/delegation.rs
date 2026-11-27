//! Admission of an OpenAI child (TECHNICAL 9.7 "Admission", steps 2 to 5; 14 "A child cannot
//! widen its owner's authority"). Pure: it compares as written and anything it cannot compare is
//! refused.

use crate::error::{Problem, Refusal};
use crate::profile::{grammar_problems, parse_profile, Backend, Profile, Skill};
use crate::refuse::refuse;

const DEFAULT_LIVE: u32 = 4;

/// The root's effective `subagent-limits.live` (9.2: 1..20, default 4). A value that is present
/// and not valid (a compiled root never has one) counts as 1, the narrowest cap.
pub fn live_cap(root: &Profile) -> u32 {
    match root.subagent_limits.as_ref().and_then(|v| v.get("live")) {
        None => DEFAULT_LIVE,
        Some(x) => x
            .as_u64()
            .filter(|n| (1..=20).contains(n))
            .map_or(1, |n| n as u32),
    }
}

/// An optional cap: absent on the root is unbounded; otherwise the child needs its own, no higher.
fn cap_ok<T: PartialOrd>(root: Option<T>, child: Option<T>) -> bool {
    match (root, child) {
        (None, _) => true,
        (Some(r), Some(c)) => c <= r,
        (Some(_), None) => false,
    }
}

/// Steps 2 to 5 of 9.7 Admission; see the task interface. Step 1 is the caller's.
pub fn admit(
    root: &Profile,
    root_model: &str,
    caller_is_child: bool,
    live: u32,
    name: &str,
    load: impl FnOnce(&str) -> Option<String>,
) -> Result<Profile, Vec<Refusal>> {
    admit_with_skills(
        root,
        root_model,
        caller_is_child,
        live,
        name,
        load,
        |skill| root.skills.contains(skill),
    )
}

pub(crate) fn admit_with_skills(
    root: &Profile,
    root_model: &str,
    caller_is_child: bool,
    live: u32,
    name: &str,
    load: impl FnOnce(&str) -> Option<String>,
    matches_skill: impl FnMut(&Skill) -> bool,
) -> Result<Profile, Vec<Refusal>> {
    let one = |field: &str, reason: &str| {
        Err(vec![
            Problem::new(field, reason).refusal(Backend::Openai, root_model)
        ])
    };
    if caller_is_child {
        return one(
            "subagent-limits.depth",
            "a child cannot delegate: depth is 1",
        );
    }
    if !root.delegates.iter().any(|d| d == name) {
        return one("profile", "not in the delegates of the calling profile");
    }
    // Step 3 ends with the load: a named but missing profile is refused `profile` before the
    // live cap of step 4 is looked at (9.7 Admission, S134 r1 F1.6).
    let Some(json) = load(name) else {
        return one("profile", "the delegate profile is missing or not adopted");
    };
    if live >= live_cap(root) {
        return one(
            "subagent-limits.live",
            "the live child limit of the calling profile is reached",
        );
    }

    let (c, mut problems) = parse_profile(&json);
    problems.extend(grammar_problems(&c));
    let mut bad: Vec<(&str, &str)> = Vec::new();
    if c.backend.as_deref() != Some("openai") {
        bad.push(("backend", "a child is an openai session"));
    }
    if c.model.as_deref() != Some(root_model) {
        bad.push(("model", "a child runs the model of the calling profile"));
    }
    if !c.toolsets.iter().all(|t| root.toolsets.contains(t)) {
        bad.push(("toolsets", "a toolset the calling profile lacks"));
    }
    if !c.skills.iter().all(matches_skill) {
        bad.push(("skills", "a skill or revision the calling profile lacks"));
    }
    if c.mode != root.mode {
        bad.push(("mode", "not the mode of the calling profile"));
    }
    if c.ask != root.ask {
        bad.push(("ask", "not the ask setting of the calling profile"));
    }
    if c.pre_tool_check != root.pre_tool_check {
        bad.push((
            "pre-tool-check",
            "not the pre-tool-check of the calling profile",
        ));
    }
    if !c.allow.iter().all(|r| root.allow.contains(r)) {
        bad.push(("allow", "an allow rule the calling profile lacks"));
    }
    if !root.deny.iter().all(|r| c.deny.contains(r)) {
        bad.push(("deny", "lacks a deny rule of the calling profile"));
    }
    if root.persist == Some(false) && c.persist != Some(false) {
        bad.push(("persist", "must be false where the calling profile's is"));
    }
    if !cap_ok(root.max_turns, c.max_turns) {
        bad.push(("max-turns", "absent or higher than the calling profile's"));
    }
    if !cap_ok(root.max_tool_errors, c.max_tool_errors) {
        bad.push((
            "max-tool-errors",
            "absent or higher than the calling profile's",
        ));
    }
    if !cap_ok(root.max_tokens, c.max_tokens) {
        bad.push(("max-tokens", "absent or higher than the calling profile's"));
    }
    // `ask-expiry` always has an effective value: absent is the 600-second default (9.6), so
    // an absent root value is not unbounded (S134 r1 F1.3).
    let expiry = |p: &Profile| Some(p.ask_expiry.unwrap_or(crate::openai::tools::ASK_EXPIRY_S));
    if !cap_ok(expiry(root), expiry(&c)) {
        bad.push(("ask-expiry", "absent or higher than the calling profile's"));
    }
    if !c.delegates.is_empty() {
        bad.push(("delegates", "a child names no delegates"));
    }
    if c.subagent_limits.is_some() {
        bad.push(("subagent-limits", "a child sets no delegation limits"));
    }
    problems.extend(bad.into_iter().map(|(f, r)| Problem::new(f, r)));

    let mut all: Vec<Refusal> = problems
        .iter()
        .map(|x| x.refusal(Backend::Openai, root_model))
        .collect();
    all.extend(refuse(&c, Backend::Openai, root_model, None));
    let mut seen: Vec<Refusal> = Vec::new();
    for r in all {
        if !seen
            .iter()
            .any(|s| s.field == r.field && s.reason == r.reason)
        {
            seen.push(r);
        }
    }
    if seen.is_empty() {
        Ok(c)
    } else {
        Err(seen)
    }
}
