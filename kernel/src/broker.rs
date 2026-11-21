//! The broker (TECHNICAL 9.6): what a session allow would add for an ask. Pure: the kernel sets it
//! when it stores a Claude ask, never the adapter or the provider.

use crate::profile::rule_problem;
use serde_json::Value;

/// The session allow `{"allow":true,"for":"session"}` would add for this ask, or none (9.6).
pub fn reach(
    owner: &str,
    workspace: Option<&str>,
    kind: &str,
    tool: &str,
    input: &Value,
) -> Option<String> {
    if kind != "ask" || owner.starts_with("agent:") {
        return None;
    }
    let path_key = match tool {
        "AskUserQuestion" | "ExitPlanMode" => return None,
        "Read" | "Write" | "Edit" => "file_path",
        "NotebookEdit" => "notebook_path",
        "Glob" | "Grep" => "path",
        "Bash" => return bash(input),
        _ => return named(tool),
    };
    let ws = workspace?;
    if !ws.starts_with('/')
        || ws.ends_with('/')
        || ws.contains(['(', ')'])
        || ws.chars().any(char::is_control)
    {
        return None;
    }
    if tool == "Glob" {
        let pat = input.get("pattern")?.as_str()?;
        if pat.starts_with(['/', '~']) || pat.contains('\\') || pat.split('/').any(|s| s == "..") {
            return None;
        }
    }
    let path = input.get(path_key)?.as_str()?;
    let rest = path.strip_prefix(ws)?.strip_prefix('/')?;
    if rest
        .split('/')
        .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return None;
    }
    Some(format!("{tool}({ws}/**)"))
}

/// `Bash(<command>)`: only that exact command run as allowed (no key beyond the four plain
/// ones, so no sandbox escape), and only when it passes the allow grammar (4.1); then
/// ` timeout=<n>` and ` run_in_background=<bool>` when the input has them (S33).
fn bash(input: &Value) -> Option<String> {
    let keys = ["command", "description", "timeout", "run_in_background"];
    if input
        .as_object()?
        .keys()
        .any(|k| !keys.contains(&k.as_str()))
    {
        return None;
    }
    let command = input.get("command")?.as_str()?;
    if command.contains(':') {
        return None;
    }
    let mut rule = format!("Bash({command})");
    if rule_problem(&rule, true).is_some() {
        return None;
    }
    if let Some(t) = input.get("timeout") {
        rule.push_str(&format!(" timeout={}", t.as_u64()?));
    }
    if let Some(b) = input.get("run_in_background") {
        rule.push_str(&format!(" run_in_background={}", b.as_bool()?));
    }
    Some(rule)
}

/// `TodoWrite` and `mcp__watane__<tool>`: their name, when it is a rule name. Every other tool
/// gets none (fail closed, 9.6).
fn named(tool: &str) -> Option<String> {
    let listed = tool == "TodoWrite"
        || tool
            .strip_prefix("mcp__watane__")
            .is_some_and(|t| !t.is_empty());
    if !listed || tool.contains(['(', ')']) {
        return None;
    }
    rule_problem(tool, true).is_none().then(|| tool.to_string())
}
