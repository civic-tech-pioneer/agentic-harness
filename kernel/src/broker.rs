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
/// ones, so no sandbox escape), and only when it passes the allow grammar (4.1).
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
    let rule = format!("Bash({command})");
    rule_problem(&rule, true).is_none().then_some(rule)
}

/// Any other tool: its name, when it is a rule name.
fn named(tool: &str) -> Option<String> {
    if tool.contains(['(', ')']) {
        return None;
    }
    rule_problem(tool, true).is_none().then(|| tool.to_string())
}
