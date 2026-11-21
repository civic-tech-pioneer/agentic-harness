//! The broker (TECHNICAL 9.6): what a session allow would add for an ask. Pure: the kernel sets it
//! when it stores a Claude ask, never the adapter or the provider.

use crate::profile::rule_problem;
use serde_json::Value;

/// The session allow `{"allow":true,"for":"session"}` would add for this ask, rendered
/// `<Tool>(<key>)` (bare `<Tool>` for an empty key), or none (9.6).
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
    scope(tool, workspace, input).map(|key| render(tool, &key))
}

/// `<tool>(<key>)`, or the bare tool name for an empty key.
pub fn render(tool: &str, key: &str) -> String {
    if key.is_empty() {
        tool.to_string()
    } else {
        format!("{tool}({key})")
    }
}

/// The scope key of a call: the smallest part of its input that decides what it can reach (9.6).
/// A tool without a scope function, and an input its function does not know, get none.
pub fn scope(tool: &str, workspace: Option<&str>, input: &Value) -> Option<String> {
    let (known, path_key): (&[&str], &str) = match tool {
        "Bash" => return bash(input),
        "WebFetch" => return web_fetch(input),
        "Skill" => return skill(input),
        "TodoWrite" => return only_keys(input, &["todos"]).then(String::new),
        "Read" => (&["file_path", "offset", "limit", "pages"], "file_path"),
        "Write" => (&["file_path", "content"], "file_path"),
        "Edit" => (
            &["file_path", "old_string", "new_string", "replace_all"],
            "file_path",
        ),
        "MultiEdit" => (&["file_path", "edits"], "file_path"),
        "NotebookEdit" => (
            &[
                "notebook_path",
                "new_source",
                "cell_id",
                "cell_type",
                "edit_mode",
            ],
            "notebook_path",
        ),
        "Glob" => (&["pattern", "path"], "path"),
        "Grep" => (
            &[
                "pattern",
                "path",
                "glob",
                "type",
                "output_mode",
                "-A",
                "-B",
                "-C",
                "-n",
                "-i",
                "multiline",
                "head_limit",
                "offset",
            ],
            "path",
        ),
        _ => return None,
    };
    if !only_keys(input, known) {
        return None;
    }
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
    // A write there can make code run later (9.6, 4.3): it always asks.
    if matches!(tool, "Write" | "Edit" | "MultiEdit" | "NotebookEdit") {
        let last = rest.rsplit('/').next().unwrap_or("");
        let named = [
            "CLAUDE.md",
            "AGENTS.md",
            "build.rs",
            "package.json",
            "CLAUDE.local.md",
            "Makefile",
            "GNUmakefile",
        ]
        .iter()
        .any(|n| last.eq_ignore_ascii_case(n));
        // A file system that folds Unicode case or drops ignorable code points can open a
        // non-ASCII spelling as a named or dot file (9.6).
        let odd = rest.bytes().any(|b| !(0x20..=0x7e).contains(&b));
        if named || odd || rest.split('/').any(|s| s.starts_with('.')) {
            return None;
        }
    }
    Some(format!("{ws}/**"))
}

/// Whether the input is a JSON object whose every key is one of `known`.
fn only_keys(input: &Value, known: &[&str]) -> bool {
    input
        .as_object()
        .is_some_and(|o| o.keys().all(|k| known.contains(&k.as_str())))
}

/// `domain:<host>`: the URL's host after one WHATWG parse, for a plain http(s) URL only.
fn web_fetch(input: &Value) -> Option<String> {
    if !only_keys(input, &["url", "prompt"]) {
        return None;
    }
    let raw = input.get("url")?.as_str()?;
    if raw.is_empty()
        || raw.len() > 2048
        || !raw
            .bytes()
            .all(|b| (0x21..=0x7e).contains(&b) && b != b'\\')
    {
        return None;
    }
    let url = url::Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return None;
    }
    let host = url.host()?.to_string();
    if host.is_empty() || host.contains(['(', ')']) {
        return None;
    }
    Some(format!("domain:{host}"))
}

/// The skill's name: `[a-z0-9-]{1,64}`, or two of those joined by one `:`.
fn skill(input: &Value) -> Option<String> {
    if !only_keys(input, &["skill", "args"]) {
        return None;
    }
    let name = input.get("skill")?.as_str()?;
    let part = |p: &str| {
        (1..=64).contains(&p.len())
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    let ok = match name.split_once(':') {
        Some((a, b)) => part(a) && part(b),
        None => part(name),
    };
    ok.then(|| name.to_string())
}

/// The exact command (no key beyond the four plain ones, so no sandbox escape), and only when
/// `Bash(<command>)` passes the allow grammar (4.1); then `; timeout=<n>` and
/// `; run_in_background=<bool>` when the input has them (S33).
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
    if rule_problem(&format!("Bash({command})"), true).is_some() {
        return None;
    }
    let mut key = command.to_string();
    if let Some(t) = input.get("timeout") {
        key.push_str(&format!("; timeout={}", t.as_u64()?));
    }
    if let Some(b) = input.get("run_in_background") {
        key.push_str(&format!("; run_in_background={}", b.as_bool()?));
    }
    Some(key)
}
