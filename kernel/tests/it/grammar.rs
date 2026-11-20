use crate::{fields, parse};
use agent_kernel::{grammar_problems, parse_profile, refuse, Backend, SystemPrompt};
use serde_json::{json, Value};

#[test]
fn ac3_not_json_and_not_object_are_one_problem() {
    for doc in ["{nope", "[1]", "3", ""] {
        let (_, ps) = parse(doc);
        assert_eq!(ps.len(), 1, "{doc}");
        assert_eq!(ps[0].field, "profile");
        assert!(!ps[0].reason.is_empty());
    }
    assert!(parse("{nope").1[0].reason.contains("JSON"));
}

#[test]
fn ac4_unknown_and_per_session_keys() {
    let (_, ps) = parse(r#"{"bogus":1,"workspace":"w","extra-workspaces":[],"agent-handle":"h","session":{}}"#);
    assert_eq!(ps.len(), 5);
    let b = ps.iter().find(|p| p.field == "bogus").unwrap();
    assert_eq!(b.reason, "not a profile field");
    for k in ["workspace", "extra-workspaces", "agent-handle", "session"] {
        let p = ps.iter().find(|p| p.field == k).unwrap();
        assert_eq!(p.reason, "chosen per session, not in a profile (9.2)");
    }
}

fn bad_doc() -> Value {
    json!({
        "model": "bad model!",
        "fallback-models": ["a", "b", "c", "d", "e"],
        "effort": "huge",
        "mode": "auto",
        "system-prompt": {"replace": "x", "append": "y"},
        "tools": ["Read", "Nope"],
        "allow": ["Bash(python3 -c x)", "Read, Write", "-Read", "Bogus(x)"],
        "deny": ["mcp__watane__"],
        "pre-tool-check": {"timeout-s": 4},
        "subagents": [{"name": "Bad Name", "description": "d", "prompt": "p",
                       "tools": ["Grep"], "permissionMode": "bypass"}],
        "subagent-depth": 4,
        "skills": [{"name": "UP", "files": {"/abs": "x", "a/../b": "y"}}],
        "max-turns": 0,
        "max-budget-usd": -1,
        "max-tool-errors": 0,
        "max-tokens": 0,
        "backend": "gemini",
        "ask": "yes"
    })
}

#[test]
fn ac5_all_grammar_problems_reported_together() {
    let (_, ps) = parse(&bad_doc().to_string());
    let f = fields(&ps);
    for want in [
        "model", "fallback-models", "effort", "mode", "system-prompt", "tools", "allow", "deny",
        "pre-tool-check.timeout-s", "subagents[0].permissionMode", "subagents[0].name",
        "subagents[0].tools", "subagent-depth", "skills[0].name", "skills[0].files", "max-turns",
        "max-budget-usd", "max-tool-errors", "max-tokens", "backend", "ask",
    ] {
        assert!(f.contains(&want), "missing {want} in {f:?}");
    }
    assert_eq!(f.iter().filter(|x| **x == "allow").count(), 4);
}

fn one(doc: Value) -> Vec<String> {
    parse(&doc.to_string()).1.into_iter().map(|p| p.field).collect()
}

#[test]
fn ac5_each_rule_alone() {
    assert_eq!(one(json!({"mode": "bypassPermissions"})), ["mode"]);
    assert_eq!(one(json!({"mode": "plan"})), Vec::<String>::new());
    assert_eq!(one(json!({"system-prompt": {}})), ["system-prompt"]);
    assert_eq!(one(json!({"system-prompt": {"append": "x".repeat(65537)}})), ["system-prompt"]);
    assert_eq!(one(json!({"system-prompt": {"append": "x".repeat(65536)}})), Vec::<String>::new());
    assert_eq!(one(json!({"allow": ["Bash(npm exec foo)"]})), ["allow"]);
    assert_eq!(one(json!({"allow": ["Bash(uv run x)"]})), ["allow"]);
    assert_eq!(one(json!({"allow": ["Bash(/usr/bin/python3.12:*)"]})), ["allow"]);
    assert_eq!(one(json!({"allow": ["Bash(git status:*)", "Read(./src/**)", "mcp__watane__ls", "Edit"]})), Vec::<String>::new());
    assert_eq!(one(json!({"deny": ["Bash(bash:*)"]})), Vec::<String>::new());
    for r in ["bash", "sh", "zsh", "fish", "dash", "env", "python", "python3", "node", "deno", "bun",
        "bunx", "npx", "uvx", "perl", "ruby", "php", "eval", "exec", "xargs", "sudo"] {
        assert_eq!(one(json!({"allow": [format!("Bash({r} -c x)")]})), ["allow"], "{r}");
    }
    for r in ["pnpm dlx x", "devbox run x"] {
        assert_eq!(one(json!({"allow": [format!("Bash({r})")]})), ["allow"], "{r}");
    }
    assert_eq!(one(json!({"pre-tool-check": {"timeout-s": 121}})), ["pre-tool-check.timeout-s"]);
    assert_eq!(one(json!({"pre-tool-check": {"timeout-s": 5}})), Vec::<String>::new());
    assert_eq!(one(json!({"subagent-depth": 3})), Vec::<String>::new());
    assert_eq!(one(json!({"subagent-depth": 1000})), ["subagent-depth"]);
    assert_eq!(one(json!({"max-turns": 1001})), ["max-turns"]);
    assert_eq!(one(json!({"max-turns": -3})), ["max-turns"]);
    assert_eq!(one(json!({"max-budget-usd": 0})), ["max-budget-usd"]);
    assert_eq!(one(json!({"json-schema": {"x": "y".repeat(70000)}})), ["json-schema"]);
    assert_eq!(one(json!({"skills": [{"name": "a", "files": {"x": "y".repeat(1024 * 1024)}}]})), ["skills"]);
    assert_eq!(one(json!({"subagents": [{"name": "a", "description": "d", "prompt": "p".repeat(65536)}]})), ["subagents"]);
    assert_eq!(one(json!({"tools": "Read"})), ["tools"]);
    assert_eq!(
        one(json!({"tools": ["Read"], "subagents": [{"name": "a", "description": "d", "prompt": "p", "tools": ["Write"]}]})),
        ["subagents[0].tools"]
    );
}

#[test]
fn ac6_good_profile_round_trips_and_passes() {
    let doc = json!({
        "backend": "claude", "model": "claude-opus-5-5", "fallback-models": ["claude-sonnet-5-5"],
        "effort": "high", "system-prompt": {"append": "be brief"}, "tools": ["Read", "Edit"],
        "toolsets": ["t"], "delegates": ["d"], "allow": ["Read(./src/**)"], "deny": ["Bash(rm:*)"],
        "mode": "accept-edits", "ask": true, "pre-tool-check": {"timeout-s": 30},
        "subagents": [{"name": "rev", "description": "d", "prompt": "p", "tools": ["Read"], "model": "m"}],
        "subagent-depth": 2, "subagent-limits": {"n": 3},
        "skills": [{"name": "s", "files": {"SKILL.md": "hi"}}],
        "sandbox": {"enabled": true}, "json-schema": {"type": "object"},
        "max-turns": 10, "max-tool-errors": 3, "max-tokens": 1000, "max-budget-usd": 1.5,
        "ask-expiry": 600, "persist": false, "partial": true
    });
    let (p, ps) = parse(&doc.to_string());
    assert!(ps.is_empty(), "{ps:?}");
    assert!(refuse(&p, Backend::Claude, "claude-opus-5-5", None).is_empty());
    assert_eq!(serde_json::to_value(&p).unwrap(), doc);
    assert_eq!(p.system_prompt, Some(SystemPrompt::Append("be brief".into())));
    assert!(grammar_problems(&p).is_empty());
}

#[test]
fn ac5_wrong_types_keep_the_valid_fields() {
    let (p, ps) = parse_profile(r#"{"model": 3, "effort": "low", "tools": [1]}"#);
    assert_eq!(fields(&ps), ["model", "tools"]);
    assert_eq!(p.effort.as_deref(), Some("low"));
}
