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
    let (_, ps) = parse(
        r#"{"bogus":1,"workspace":"w","extra-workspaces":[],"agent-handle":"h","session":{}}"#,
    );
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
        "model",
        "fallback-models",
        "effort",
        "mode",
        "system-prompt",
        "tools",
        "allow",
        "deny",
        "pre-tool-check.timeout-s",
        "subagents[0].permissionMode",
        "subagents[0].name",
        "subagents[0].tools",
        "subagent-depth",
        "skills[0].name",
        "skills[0].files",
        "max-turns",
        "max-budget-usd",
        "max-tool-errors",
        "max-tokens",
        "backend",
        "ask",
    ] {
        assert!(f.contains(&want), "missing {want} in {f:?}");
    }
    assert_eq!(f.iter().filter(|x| **x == "allow").count(), 4);
}

fn one(doc: Value) -> Vec<String> {
    parse(&doc.to_string())
        .1
        .into_iter()
        .map(|p| p.field)
        .collect()
}

#[test]
fn ac5_each_rule_alone() {
    assert_eq!(one(json!({"mode": "bypassPermissions"})), ["mode"]);
    assert_eq!(one(json!({"mode": "plan"})), Vec::<String>::new());
    assert_eq!(one(json!({"system-prompt": {}})), ["system-prompt"]);
    assert_eq!(
        one(json!({"system-prompt": {"append": "x".repeat(65537)}})),
        ["system-prompt"]
    );
    assert_eq!(
        one(json!({"system-prompt": {"append": "x".repeat(65536)}})),
        Vec::<String>::new()
    );
    assert_eq!(one(json!({"allow": ["Bash(npm exec foo)"]})), ["allow"]);
    assert_eq!(one(json!({"allow": ["Bash(uv run x)"]})), ["allow"]);
    assert_eq!(
        one(json!({"allow": ["Bash(/usr/bin/python3.12:*)"]})),
        ["allow"]
    );
    assert_eq!(
        one(json!({"allow": ["Bash(git status:*)", "Read(./src/**)", "mcp__watane__ls", "Edit"]})),
        Vec::<String>::new()
    );
    assert_eq!(one(json!({"deny": ["Bash(bash:*)"]})), Vec::<String>::new());
    for r in [
        "bash", "sh", "zsh", "fish", "dash", "env", "python", "python3", "node", "deno", "bun",
        "bunx", "npx", "uvx", "perl", "ruby", "php", "eval", "exec", "xargs", "sudo",
    ] {
        assert_eq!(
            one(json!({"allow": [format!("Bash({r} -c x)")]})),
            ["allow"],
            "{r}"
        );
    }
    for r in ["pnpm dlx x", "devbox run x"] {
        assert_eq!(
            one(json!({"allow": [format!("Bash({r})")]})),
            ["allow"],
            "{r}"
        );
    }
    assert_eq!(
        one(json!({"pre-tool-check": {"timeout-s": 121}})),
        ["pre-tool-check.timeout-s"]
    );
    assert_eq!(
        one(json!({"pre-tool-check": {"timeout-s": 5}})),
        Vec::<String>::new()
    );
    assert_eq!(one(json!({"subagent-depth": 3})), Vec::<String>::new());
    assert_eq!(one(json!({"subagent-depth": 1000})), ["subagent-depth"]);
    assert_eq!(one(json!({"max-turns": 1001})), ["max-turns"]);
    assert_eq!(one(json!({"max-turns": -3})), ["max-turns"]);
    assert_eq!(one(json!({"max-budget-usd": 0})), ["max-budget-usd"]);
    assert_eq!(
        one(json!({"json-schema": {"x": "y".repeat(70000)}})),
        ["json-schema"]
    );
    assert_eq!(
        one(json!({"skills": [{"name": "a", "files": {"x": "y".repeat(1024 * 1024)}}]})),
        ["skills"]
    );
    assert_eq!(
        one(json!({"subagents": [{"name": "a", "description": "d", "prompt": "p".repeat(65536)}]})),
        ["subagents"]
    );
    assert_eq!(one(json!({"tools": "Read"})), ["tools"]);
    assert_eq!(
        one(json!({"fallback-models": ["a", "b", "c", "d"]})),
        Vec::<String>::new()
    );
    assert_eq!(one(json!({"effort": "xhigh"})), Vec::<String>::new());
    assert_eq!(one(json!({"effort": "High"})), ["effort"]);
    for path in ["/etc/x", "a/../b", "..", "C:x", "\\x"] {
        assert_eq!(
            one(json!({"skills": [{"name": "a", "files": {path: "y"}}]})),
            ["skills[0].files"],
            "{path}"
        );
    }
    assert_eq!(
        one(json!({"skills": [{"name": "a-1", "files": {"a..b/c": "y"}}]})),
        Vec::<String>::new()
    );
    assert_eq!(
        one(json!({"subagents": [{"name": "a", "description": "d", "prompt": "p".repeat(65534)}]})),
        Vec::<String>::new()
    );
    assert_eq!(
        one(
            json!({"tools": ["Read"], "subagents": [{"name": "a", "description": "d", "prompt": "p", "tools": ["Write"]}]})
        ),
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
    assert_eq!(
        p.system_prompt,
        Some(SystemPrompt::Append("be brief".into()))
    );
    assert!(grammar_problems(&p).is_empty());
}

#[test]
fn ac5_wrong_types_keep_the_valid_fields() {
    let (p, ps) = parse_profile(r#"{"model": 3, "effort": "low", "tools": [1]}"#);
    assert_eq!(fields(&ps), ["model", "tools"]);
    assert_eq!(p.effort.as_deref(), Some("low"));
}

#[test]
fn ac5_rule_grammar_has_no_bypass() {
    for r in [
        // case, quotes, escapes, globs, expansions, assignments, redirections in the command name
        "Bash(Python3 -c x)",
        "Bash(BASH -c x)",
        "Bash(\"python3\" -c x)",
        "Bash('sh' -c x)",
        "Bash(\\python3 -c x)",
        "Bash(py\"\"thon3 -c x)",
        "Bash(FOO=1 python3:*)",
        "Bash(py*)",
        "Bash(b*:*)",
        "Bash(sh<x)",
        "Bash(bash${IFS}-c:*)",
        "Bash($(echo python3):*)",
        "Bash(`x`:*)",
        "Bash(*)",
        "Bash()",
        "Bash( :*)",
        "Bash(\tpython3 -c x)",
        // runners behind flags or aliases, and wrappers that run the next word
        "Bash(npm x foo)",
        "Bash(npm --yes exec foo)",
        "Bash(npm:*)",
        "Bash(npm e*)",
        "Bash(uv --quiet run x)",
        "Bash(uv tool run x)",
        "Bash(pnpm exec x)",
        "Bash(yarn dlx x)",
        "Bash(command python3:*)",
        "Bash(nohup python3:*)",
        "Bash(timeout 5 python3:*)",
        "Bash(time python3:*)",
        "Bash(ksh -c x)",
        "Bash(source x:*)",
        "Bash(. ./x:*)",
        // a second rule smuggled past the parentheses, control characters
        "Bash(git status:*\npython3 -c x)",
        "Read(x),Bash(python3:*)",
        "Read(x) Bash(python3:*)",
        "Read(\\(x),Bash(python3:*))",
        "Read(x)\u{0}",
        // names: case, lookalikes, odd MCP tools
        "bash(git status)",
        "Rеad",
        "mcp__watane__a.b",
        "mcp__watane__a b",
        "mcp__other__x",
        // AC3: shell syntax, non-ASCII, path executables
        "Bash(./scripts/fmt.sh)",
        "Bash(/usr/bin/git status)",
        "Bash(git status $HOME)",
        "Bash(git 'status')",
        "Bash(git \"status\")",
        "Bash(git sta\\tus)",
        "Bash(git status;ls)",
        "Bash(git status | cat)",
        "Bash(git status > x)",
        "Bash(git {a,b})",
        "Bash(git `x`)",
        "Bash(git status &)",
        "Bash(g\u{131}t status)",
    ] {
        assert_eq!(one(json!({"allow": [r]})), ["allow"], "{r:?}");
    }
    for r in [
        "Bash(git status:*)",
        "Bash(npm test:*)",
        "Bash(npm run build)",
        "Bash(uv pip install:*)",
        "Bash(cargo test)",
        "Bash(npm test)",
        "Bash(cargo build --release)",
        "mcp__watane__ls_x-1(a)",
    ] {
        assert_eq!(one(json!({"allow": [r]})), Vec::<String>::new(), "{r:?}");
    }
    assert_eq!(one(json!({"deny": ["Read(x),Bash(y)"]})), ["deny"]);
    for r in ["Bash(git a}b)", "Bash(git status < x)"] {
        assert_eq!(one(json!({"allow": [r]})), ["allow"], "{r:?}");
    }
    let ps = parse(&json!({"allow": ["Bash(git status < x)"]}).to_string()).1;
    assert!(ps[0].reason.contains("'<'"), "{}", ps[0].reason);
    let ps = parse(&json!({"allow": ["Bash(/usr/bin/git status)"]}).to_string()).1;
    assert!(ps[0].reason.contains("path"), "{}", ps[0].reason);
    assert_eq!(
        one(json!({"deny": ["Bash(git 'x' $y;z)", "Bash(/usr/bin/git)", "Bash"]})),
        Vec::<String>::new()
    );
    let ps = parse(&json!({"allow": ["Bash"]}).to_string()).1;
    assert_eq!(fields(&ps), ["allow"]);
    assert!(
        ps[0].reason.contains("allows any command"),
        "{}",
        ps[0].reason
    );
}

#[test]
fn ac3a_no_tools_serialize_as_an_empty_list() {
    let (p, ps) = parse_profile(
        &json!({"subagents": [{"name": "a", "description": "d", "prompt": "p"}]}).to_string(),
    );
    assert!(ps.is_empty(), "{ps:?}");
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["tools"], json!([]));
    assert_eq!(v["subagents"][0]["tools"], json!([]));
}

#[test]
fn ac3b_typed_structs_refuse_unknown_fields() {
    use agent_kernel::{PreToolCheck, Skill, Subagent};
    assert!(serde_json::from_str::<SystemPrompt>(r#"{"append":"x","extra":1}"#).is_err());
    assert!(serde_json::from_str::<PreToolCheck>(r#"{"timeout-s":5,"x":1}"#).is_err());
    assert!(serde_json::from_str::<Skill>(r#"{"name":"a","x":1}"#).is_err());
    assert!(serde_json::from_str::<Subagent>(
        r#"{"name":"a","description":"d","prompt":"p","x":1}"#
    )
    .is_err());
}

#[test]
fn ac3c_duplicate_names_name_the_later_one() {
    let sub = |n: &str| json!({"name": n, "description": "d", "prompt": "p"});
    let (_, ps) = parse_profile(&json!({"subagents": [sub("a"), sub("b"), sub("a")]}).to_string());
    assert_eq!(fields(&ps), ["subagents[2].name"]);
    let (_, ps) = parse_profile(
        &json!({"skills": [{"name": "s"}, {"name": "t"}, {"name": "s"}]}).to_string(),
    );
    assert_eq!(fields(&ps), ["skills[2].name"]);
}

fn skill_md(text: &str) -> Vec<agent_kernel::Problem> {
    parse_profile(&json!({"skills": [{"name": "s", "files": {"SKILL.md": text}}]}).to_string()).1
}

#[test]
fn ac3d_frontmatter_keeps_name_and_description_only() {
    assert!(skill_md("no frontmatter\nallowed-tools: Bash").is_empty());
    assert!(skill_md("---\nname: s\ndescription: d\n---\nbody").is_empty());
    assert!(skill_md("---\nname: s\ndescription: >\n  long\n  text\n---\nbody").is_empty());
    let ps = skill_md("---\nname: s\nhooks:\n  a: b\n---\nbody");
    assert_eq!(fields(&ps), ["skills[0].files"]);
    assert!(ps[0].reason.contains("hooks"), "{}", ps[0].reason);
    let ps = skill_md("---\n  name: s\n  allowed-tools: x\n---\n");
    assert_eq!(
        fields(&ps),
        ["skills[0].files"],
        "an indented mapping is still top level"
    );
    assert_eq!(
        fields(&skill_md(
            "---\n  name: s\n  description: >\n    long\n---\n"
        )),
        ["skills[0].files"],
        "an indented top-level block is refused"
    );
    assert!(skill_md("---\nname: s\ndescription: >-\n  folded\n  more text\n---\nb").is_empty());
    for bad in [
        "---\r\nname: s\rallowed-tools: x\n---\n",
        "\u{feff}---\nallowed-tools: x\n---\n",
        "\n---\nallowed-tools: x\n---\n",
        "---\nname: s\n\tallowed-tools: x\n---\n",
        "---\n{allowed-tools: x}\n---\n",
        "---\nname: a\nname: b\n---\n",
        "---\nname: s\u{e9}\n---\n",
    ] {
        assert_eq!(fields(&skill_md(bad)), ["skills[0].files"], "{bad:?}");
    }
    assert_eq!(
        fields(&skill_md("---\r\nallowed-tools: x\r\n---\r\n")),
        ["skills[0].files"]
    );
    let nested =
        json!({"skills": [{"name": "s", "files": {"x/Skill.md": "---\nhooks: x\n---\n"}}]});
    assert_eq!(
        fields(&parse_profile(&nested.to_string()).1),
        ["skills[0].files"],
        "a nested SKILL.md is checked too"
    );
}

#[test]
fn ac3e_skill_paths_are_plain_and_distinct() {
    for path in ["./a", "a/./b", "a//b", "a/", "é", "a b", "a:b", "a*"] {
        let (_, ps) =
            parse_profile(&json!({"skills": [{"name": "s", "files": {path: "y"}}]}).to_string());
        assert_eq!(fields(&ps), ["skills[0].files"], "{path}");
    }
    let (_, ps) = parse_profile(
        &json!({"skills": [{"name": "s", "files": {"a/B.md": "1", "a/b.md": "2", "c_d-1.x": "3"}}]})
            .to_string(),
    );
    assert_eq!(fields(&ps), ["skills[0].files"]);
}

#[test]
fn ac3f_more_runners_and_find_exec() {
    for r in [
        "Bash(gawk:*)",
        "Bash(mawk:*)",
        "Bash(nawk:*)",
        "Bash(busybox sh:*)",
        "Bash(toybox sh)",
        "Bash(setsid x)",
        "Bash(ionice x)",
        "Bash(chrt 1 x)",
        "Bash(taskset 1 x)",
        "Bash(unshare x)",
        "Bash(flock f x)",
        "Bash(watch x)",
        "Bash(find . -exec:*)",
        "Bash(find . -execdir sh)",
        "Bash(find . -ok x)",
        "Bash(find . -okdir x)",
        "Bash(find . '-exec' sh)",
        "Bash(find . -ex\\ec sh)",
        "Bash(find:*)",
        "Bash(find *)",
    ] {
        let (_, ps) = parse_profile(&json!({"allow": [r]}).to_string());
        assert_eq!(fields(&ps), ["allow"], "{r}");
    }
    let (_, ps) = parse_profile(&json!({"allow": ["Bash(find . -name x)"]}).to_string());
    assert!(ps.is_empty(), "{ps:?}");
}
