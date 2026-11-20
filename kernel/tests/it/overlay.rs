use crate::fields;
use agent_kernel::{apply_overlay, parse_overlay, parse_profile, SystemPrompt};
use serde_json::json;

fn ov(doc: serde_json::Value) -> (agent_kernel::Overlay, Vec<agent_kernel::Problem>) {
    parse_overlay(&doc.to_string())
}

#[test]
fn ac1_accepts_exactly_the_narrowing_keys() {
    let (o, ps) = ov(
        json!({"system-prompt-append": "x", "skills+": [{"name": "s"}], "tools-": ["Read"],
        "deny+": ["Bash"], "max-turns": 3, "max-tool-errors": 2, "max-tokens": 9,
        "max-budget-usd": 1.5, "subagent-depth": 0}),
    );
    assert!(ps.is_empty(), "{ps:?}");
    assert_eq!(o.max_turns, Some(3));
    assert_eq!(o.skills_plus.len(), 1);
}

#[test]
fn ac1_widening_keys_have_their_reasons() {
    let keys = [
        ("tools", "adds tools"),
        ("tools+", "adds tools"),
        ("allow", "adds allow rules"),
        ("allow+", "adds allow rules"),
        ("workspace", "adds workspaces"),
        ("extra-workspaces", "adds workspaces"),
        ("workspaces+", "adds workspaces"),
        ("subagents", "adds subagent profiles"),
        ("subagents+", "adds subagent profiles"),
        ("mode", "changes the permission mode"),
        ("system-prompt", "replaces the instructions"),
        ("bogus", "not an overlay field"),
    ];
    for (k, why) in keys {
        let (_, ps) = ov(json!({ k: 1 }));
        assert_eq!(ps.len(), 1, "{k}");
        assert_eq!(ps[0].field, format!("overlay.{k}"));
        assert_eq!(ps[0].reason, why);
    }
}

#[test]
fn ac1_not_json_or_not_an_object() {
    for text in ["{", "[]", "3"] {
        let (_, ps) = parse_overlay(text);
        assert_eq!(fields(&ps), ["overlay"], "{text}");
    }
}

#[test]
fn ac1_wrong_types_are_named_under_the_key() {
    let (_, ps) = ov(json!({"tools-": "Read", "max-turns": "3", "skills+": [{"name": 1}]}));
    let mut f = fields(&ps);
    f.sort();
    assert_eq!(
        f,
        [
            "overlay.max-turns",
            "overlay.skills+[0].name",
            "overlay.tools-"
        ]
    );
}

fn base() -> agent_kernel::Profile {
    let (p, ps) = parse_profile(
        &json!({"tools": ["Read", "Edit"], "deny": ["Write"], "max-turns": 20,
            "subagents": [{"name": "r", "description": "d", "prompt": "p", "tools": ["Read", "Edit"]}]})
        .to_string(),
    );
    assert!(ps.is_empty(), "{ps:?}");
    p
}

#[test]
fn ac3_applies_every_narrowing() {
    let (o, ps) = ov(
        json!({"system-prompt-append": "more", "skills+": [{"name": "s"}],
        "tools-": ["Edit"], "deny+": ["Bash"], "max-turns": 5, "max-tool-errors": 3,
        "max-tokens": 100, "max-budget-usd": 2.0, "subagent-depth": 0}),
    );
    assert!(ps.is_empty());
    let (c, ps) = apply_overlay(&base(), &o);
    assert!(ps.is_empty(), "{ps:?}");
    assert_eq!(c.system_prompt, Some(SystemPrompt::Append("more".into())));
    assert_eq!(c.skills[0].name, "s");
    assert_eq!(c.tools, ["Read"]);
    assert_eq!(c.subagents[0].tools, ["Read"]);
    assert_eq!(c.deny, ["Write", "Bash"]);
    assert_eq!(
        (
            c.max_turns,
            c.max_tool_errors,
            c.max_tokens,
            c.max_budget_usd,
            c.subagent_depth
        ),
        (Some(5), Some(3), Some(100), Some(2.0), Some(0))
    );
}

#[test]
fn ac3_system_prompt_append_grows_or_keeps_replacing() {
    let o = ov(json!({"system-prompt-append": "B"})).0;
    let mut p = base();
    p.system_prompt = Some(SystemPrompt::Append("A".into()));
    assert_eq!(
        apply_overlay(&p, &o).0.system_prompt,
        Some(SystemPrompt::Append("A\n\nB".into()))
    );
    p.system_prompt = Some(SystemPrompt::Replace("A".into()));
    assert_eq!(
        apply_overlay(&p, &o).0.system_prompt,
        Some(SystemPrompt::Replace("A\n\nB".into()))
    );
}

#[test]
fn ac4_limits_may_only_narrow() {
    let p = base();
    let (c, ps) = apply_overlay(&p, &ov(json!({"max-turns": 21})).0);
    assert_eq!(fields(&ps), ["overlay.max-turns"]);
    assert!(ps[0].reason.contains("21") && ps[0].reason.contains("20"));
    assert_eq!(c.max_turns, Some(20));
    for ok in [20, 1] {
        assert!(apply_overlay(&p, &ov(json!({"max-turns": ok})).0)
            .1
            .is_empty());
    }
    // a limit the profile does not set narrows
    assert!(
        apply_overlay(&p, &ov(json!({"max-tokens": 5, "max-budget-usd": 9})).0)
            .1
            .is_empty()
    );
}

#[test]
fn ac4_subagent_depth_compares_to_default_one() {
    let p = base();
    assert!(apply_overlay(&p, &ov(json!({"subagent-depth": 1})).0)
        .1
        .is_empty());
    let ps = apply_overlay(&p, &ov(json!({"subagent-depth": 2})).0).1;
    assert_eq!(fields(&ps), ["overlay.subagent-depth"]);
}
