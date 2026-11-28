//! Profiles: the launch fields of TECHNICAL 4.1 that are not chosen per session (9.2), their
//! parse and their grammar (4.1, 4.2). Hand-written grammars: no regex.

use crate::error::Problem;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Claude,
    Openai,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Claude => "claude",
            Backend::Openai => "openai",
        }
    }
}

impl FromStr for Backend {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "claude" => Ok(Backend::Claude),
            "openai" => Ok(Backend::Openai),
            other => Err(format!("backend must be claude or openai, not {other:?}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SystemPrompt {
    Replace(String),
    Append(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub name: String,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subagent {
    pub name: String,
    pub description: String,
    pub prompt: String,
    /// Always written, `[]` included: an absent list reads as "inherit every tool".
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PreToolCheck {
    pub timeout_s: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fallback_models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<SystemPrompt>,
    pub tools: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub toolsets: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub delegates: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_tool_check: Option<PreToolCheck>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subagents: Vec<Subagent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_depth: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_limits: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Skill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_errors: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_budget_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_expiry: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persist: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
}

pub(crate) const KIB: usize = 1024;
pub(crate) const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
pub(crate) const MODES: [&str; 4] = ["manual", "accept-edits", "plan", "dont-ask"];
/// The tool allowlist of the `claude-code-shell` grant (4.2).
pub(crate) const TOOLS: [&str; 12] = [
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "NotebookEdit",
    "Skill",
    "Task",
    "TodoWrite",
    "Bash",
    "WebFetch",
    "WebSearch",
];
const PER_SESSION: [&str; 4] = ["workspace", "extra-workspaces", "agent-handle", "session"];
/// First words that run, wrap or schedule another command (4.1); equal to the spec list,
/// which `tests/it/runners.rs` reads.
const RUNNERS: [&str; 98] = [
    ".",
    "ash",
    "at",
    "awk",
    "bash",
    "batch",
    "builtin",
    "bun",
    "bunx",
    "busybox",
    "byobu",
    "capsh",
    "chroot",
    "chrt",
    "command",
    "coproc",
    "corepack",
    "crontab",
    "csh",
    "dash",
    "deno",
    "doas",
    "dtach",
    "elixir",
    "enable",
    "env",
    "eval",
    "exec",
    "expect",
    "fakeroot",
    "fc",
    "fish",
    "flock",
    "gawk",
    "gdb",
    "ionice",
    "irb",
    "julia",
    "ksh",
    "lldb",
    "ltrace",
    "lua",
    "mawk",
    "mksh",
    "nawk",
    "nice",
    "node",
    "nohup",
    "npx",
    "nsenter",
    "numactl",
    "osascript",
    "parallel",
    "perl",
    "php",
    "pipx",
    "pkexec",
    "pnpx",
    "prlimit",
    "pwsh",
    "r",
    "rbash",
    "rscript",
    "ruby",
    "run0",
    "runuser",
    "screen",
    "script",
    "sem",
    "setpriv",
    "setsid",
    "sg",
    "sh",
    "source",
    "ssh",
    "stdbuf",
    "strace",
    "su",
    "sudo",
    "systemd-run",
    "taskset",
    "tclsh",
    "tcsh",
    "time",
    "timeout",
    "tmux",
    "toybox",
    "trap",
    "ts-node",
    "tsx",
    "unshare",
    "uvx",
    "valgrind",
    "watch",
    "xargs",
    "yarnpkg",
    "zellij",
    "zsh",
];
/// Runtime name prefixes that are refused with any version suffix (4.1).
const RUNNER_PREFIXES: [&str; 7] = ["python", "pypy", "node", "perl", "ruby", "php", "lua"];
#[doc(hidden)]
pub fn runner_words() -> &'static [&'static str] {
    &RUNNERS
}
#[doc(hidden)]
pub fn runner_prefixes() -> &'static [&'static str] {
    &RUNNER_PREFIXES
}
/// Package runners and the only second words an allow rule may pin for them: every other
/// subcommand, abbreviation or forwarder can reach an `exec` (4.1).
const PACKAGE_RUNNERS: [(&str, &[&str]); 5] = [
    (
        "npm",
        &[
            "install", "i", "ci", "test", "ls", "list", "audit", "outdated",
        ],
    ),
    (
        "pnpm",
        &["install", "add", "test", "list", "ls", "audit", "outdated"],
    ),
    (
        "yarn",
        &["install", "add", "test", "list", "audit", "outdated"],
    ),
    (
        "uv",
        &["pip", "sync", "lock", "add", "remove", "tree", "venv"],
    ),
    ("devbox", &["install", "add"]),
];

pub(crate) fn model_ok(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'[' | b']'))
}

fn slug_ok(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn p(field: impl Into<String>, reason: impl Into<String>) -> Problem {
    Problem::new(field, reason)
}

// ---- parse ----

pub(crate) fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

pub(crate) fn wrong(field: &str, want: &str, v: &Value) -> Problem {
    p(field, format!("must be {want}, not {}", kind(v)))
}

pub(crate) fn string(v: &Value, field: &str, out: &mut Vec<Problem>) -> Option<String> {
    match v.as_str() {
        Some(s) => Some(s.to_string()),
        None => {
            out.push(wrong(field, "a string", v));
            None
        }
    }
}

fn boolean(v: &Value, field: &str, out: &mut Vec<Problem>) -> Option<bool> {
    match v.as_bool() {
        Some(b) => Some(b),
        None => {
            out.push(wrong(field, "a boolean", v));
            None
        }
    }
}

pub(crate) fn uint(v: &Value, field: &str, out: &mut Vec<Problem>) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if v.is_number() {
        out.push(p(field, "must be a whole number, not negative"));
    } else {
        out.push(wrong(field, "a whole number", v));
    }
    None
}

pub(crate) fn narrow<T: TryFrom<u64>>(
    v: &Value,
    field: &str,
    range: &str,
    out: &mut Vec<Problem>,
) -> Option<T> {
    let n = uint(v, field, out)?;
    match T::try_from(n) {
        Ok(t) => Some(t),
        Err(_) => {
            out.push(p(field, format!("outside {range}")));
            None
        }
    }
}

pub(crate) fn list(v: &Value, field: &str, out: &mut Vec<Problem>) -> Vec<String> {
    let Some(items) = v.as_array() else {
        out.push(wrong(field, "an array of strings", v));
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|x| match x.as_str() {
            Some(s) => Some(s.to_string()),
            None => {
                out.push(wrong(field, "an array of strings", x));
                None
            }
        })
        .collect()
}

fn system_prompt(v: &Value, out: &mut Vec<Problem>) -> Option<SystemPrompt> {
    let f = "system-prompt";
    let Some(m) = v.as_object() else {
        out.push(wrong(f, "an object {replace | append: text}", v));
        return None;
    };
    for k in m
        .keys()
        .filter(|k| !["replace", "append"].contains(&k.as_str()))
    {
        out.push(p(format!("{f}.{k}"), "not a system-prompt field"));
    }
    let given: Vec<(&String, &Value)> = m
        .iter()
        .filter(|(k, _)| ["replace", "append"].contains(&k.as_str()))
        .collect();
    if given.len() != 1 {
        out.push(p(f, "must be exactly one of replace or append"));
        return None;
    }
    let (k, val) = given[0];
    let text = string(val, &format!("{f}.{k}"), out)?;
    Some(if k == "replace" {
        SystemPrompt::Replace(text)
    } else {
        SystemPrompt::Append(text)
    })
}

fn unknown_keys(m: &Map<String, Value>, known: &[&str], prefix: &str, out: &mut Vec<Problem>) {
    for k in m.keys().filter(|k| !known.contains(&k.as_str())) {
        out.push(p(format!("{prefix}{k}"), "not a subagent field"));
    }
}

fn subagents(v: &Value, out: &mut Vec<Problem>) -> Vec<Subagent> {
    let Some(items) = v.as_array() else {
        out.push(wrong("subagents", "an array", v));
        return Vec::new();
    };
    let mut res = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let pre = format!("subagents[{i}]");
        let Some(m) = item.as_object() else {
            out.push(wrong(&pre, "an object", item));
            continue;
        };
        unknown_keys(
            m,
            &["name", "description", "prompt", "tools", "model"],
            &format!("{pre}."),
            out,
        );
        let text = |key: &str, required: bool, out: &mut Vec<Problem>| match m.get(key) {
            Some(x) => string(x, &format!("{pre}.{key}"), out).unwrap_or_default(),
            None => {
                if required && key != "name" {
                    out.push(p(format!("{pre}.{key}"), "missing"));
                }
                String::new()
            }
        };
        let name = text("name", true, out);
        let description = text("description", true, out);
        let prompt = text("prompt", true, out);
        let tools = m
            .get("tools")
            .map(|x| list(x, &format!("{pre}.tools"), out))
            .unwrap_or_default();
        let model = m
            .get("model")
            .and_then(|x| string(x, &format!("{pre}.model"), out));
        res.push(Subagent {
            name,
            description,
            prompt,
            tools,
            model,
        });
    }
    res
}

pub(crate) fn skills(v: &Value, out: &mut Vec<Problem>) -> Vec<Skill> {
    let Some(items) = v.as_array() else {
        out.push(wrong("skills", "an array", v));
        return Vec::new();
    };
    let mut res = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let pre = format!("skills[{i}]");
        let Some(m) = item.as_object() else {
            out.push(wrong(&pre, "an object", item));
            continue;
        };
        for k in m
            .keys()
            .filter(|k| !["name", "files"].contains(&k.as_str()))
        {
            out.push(p(format!("{pre}.{k}"), "not a skill field"));
        }
        let name = m
            .get("name")
            .and_then(|x| string(x, &format!("{pre}.name"), out))
            .unwrap_or_default();
        let mut files = BTreeMap::new();
        match m.get("files") {
            None => {}
            Some(Value::Object(fm)) => {
                for (path, text) in fm {
                    if let Some(t) = string(text, &format!("{pre}.files.{path}"), out) {
                        files.insert(path.clone(), t);
                    }
                }
            }
            Some(x) => out.push(wrong(
                &format!("{pre}.files"),
                "an object of path to text",
                x,
            )),
        }
        res.push(Skill { name, files });
    }
    res
}

fn pre_tool_check(v: &Value, out: &mut Vec<Problem>) -> Option<PreToolCheck> {
    let Some(m) = v.as_object() else {
        out.push(wrong("pre-tool-check", "an object {timeout-s}", v));
        return None;
    };
    for k in m.keys().filter(|k| k.as_str() != "timeout-s") {
        out.push(p(
            format!("pre-tool-check.{k}"),
            "not a pre-tool-check field",
        ));
    }
    match m.get("timeout-s") {
        None => {
            out.push(p("pre-tool-check.timeout-s", "missing"));
            None
        }
        Some(x) => {
            uint(x, "pre-tool-check.timeout-s", out).map(|timeout_s| PreToolCheck { timeout_s })
        }
    }
}

/// Parses a profile document. Every valid field is kept; every problem is returned (AC3-AC5).
pub fn parse_profile(json: &str) -> (Profile, Vec<Problem>) {
    let mut out = Vec::new();
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return (
                Profile::default(),
                vec![p("profile", format!("not valid JSON: {e}"))],
            )
        }
    };
    let Value::Object(m) = &v else {
        return (
            Profile::default(),
            vec![p(
                "profile",
                format!("not a JSON object: it is {}", kind(&v)),
            )],
        );
    };
    let mut pr = Profile::default();
    for (k, x) in m {
        let o = &mut out;
        match k.as_str() {
            "backend" => pr.backend = string(x, k, o),
            "model" => pr.model = string(x, k, o),
            "fallback-models" => pr.fallback_models = list(x, k, o),
            "effort" => pr.effort = string(x, k, o),
            "system-prompt" => pr.system_prompt = system_prompt(x, o),
            "tools" => pr.tools = list(x, k, o),
            "toolsets" => pr.toolsets = list(x, k, o),
            "delegates" => pr.delegates = list(x, k, o),
            "allow" => pr.allow = list(x, k, o),
            "deny" => pr.deny = list(x, k, o),
            "mode" => pr.mode = string(x, k, o),
            "ask" => pr.ask = boolean(x, k, o),
            "pre-tool-check" => pr.pre_tool_check = pre_tool_check(x, o),
            "subagents" => pr.subagents = subagents(x, o),
            "subagent-depth" => pr.subagent_depth = narrow(x, k, "0..3", o),
            "subagent-limits" if x.is_object() => pr.subagent_limits = Some(x.clone()),
            "subagent-limits" => o.push(wrong(k, "an object", x)),
            "skills" => pr.skills = skills(x, o),
            "sandbox" if x.is_object() => pr.sandbox = Some(x.clone()),
            "sandbox" => o.push(wrong(k, "an object", x)),
            "json-schema" => pr.json_schema = Some(x.clone()),
            "max-turns" => pr.max_turns = narrow(x, k, "1..1000", o),
            "max-tool-errors" => pr.max_tool_errors = narrow(x, k, "1..4294967295", o),
            "max-tokens" => pr.max_tokens = uint(x, k, o),
            "max-budget-usd" => match x.as_f64() {
                Some(f) => pr.max_budget_usd = Some(f),
                None => o.push(wrong(k, "a number", x)),
            },
            "ask-expiry" => pr.ask_expiry = uint(x, k, o),
            "persist" => pr.persist = boolean(x, k, o),
            "partial" => pr.partial = boolean(x, k, o),
            s if PER_SESSION.contains(&s) => {
                o.push(p(k, "chosen per session, not in a profile (9.2)"))
            }
            _ => o.push(p(k, "not a profile field")),
        }
    }
    out.extend(grammar_problems(&pr));
    (pr, out)
}

// ---- grammar ----

/// A word the shell runs as it is: no quote, escape, expansion, glob, redirection or assignment.
/// Lowercased first: macOS file names ignore case.
fn plain(w: &str) -> bool {
    !w.is_empty()
        && w.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+/~-".contains(&b))
}

/// The part of an allow `Bash(<arg>)` rule that names the command: one trailing `:*` removed,
/// then split on whitespace only, as Claude Code reads it. Both checks below use it.
fn bash_words(arg: &str) -> impl Iterator<Item = &str> + Clone {
    arg.strip_suffix(":*").unwrap_or(arg).split_whitespace()
}

/// Why an allow `Bash(<arg>)` rule may run any code, if it may (4.1).
fn runner_problem(arg: &str) -> Option<&'static str> {
    let low = arg.to_lowercase();
    let mut words = bash_words(&low);
    let Some(first) = words.next() else {
        return Some("allows any command");
    };
    if first.contains('~') {
        return Some("starts with a word that the shell expands to a path");
    }
    if !plain(first) {
        return Some("does not start with a plain command name");
    }
    let base = first.rsplit('/').next().unwrap_or(first);
    if RUNNER_PREFIXES.iter().any(|x| base.starts_with(x)) || RUNNERS.contains(&base) {
        return Some("allows an interpreter or runner prefix");
    }
    if base == "find" {
        let rest: Vec<&str> = words.clone().collect();
        if low.contains('*') {
            return Some("allows any further arguments, such as -exec");
        }
        if rest.iter().any(|w| {
            let w = w.replace(['\'', '"', '\\'], "");
            ["-exec", "-execdir", "-ok", "-okdir"].contains(&w.as_str())
        }) {
            return Some("runs commands with -exec or -ok");
        }
    }
    if let Some((_, subs)) = PACKAGE_RUNNERS.iter().find(|(c, _)| *c == base) {
        // Options after the subcommand name a program to run (`--script-shell`, `--python`),
        // and a `*` lets the agent add them.
        if low.contains('*')
            || !words.next().is_some_and(|s| subs.contains(&s))
            || words.any(|w| w.starts_with('-'))
        {
            return Some("allows an interpreter or runner prefix");
        }
    }
    None
}

/// OpenAI matches tool names only. Refuse every argument rule before compilation.
pub(crate) fn openai_rule_problem(rule: &str) -> Option<String> {
    if rule.contains('(') {
        return Some(format!("rule {rule:?}: an argument rule is refused on openai"));
    }
    let (head, rest) = match rule.find('(') {
        Some(i) => rule.split_at(i),
        None => (rule, ""),
    };
    let plain: String = head
        .chars()
        .map(|c| if matches!(c, '*' | '?') { 'x' } else { c })
        .collect();
    let list = [format!("{plain}{rest}")];
    let (_, problems) = crate::rules::Rules::compile(&list, &[], &[]);
    problems
        .into_iter()
        .next()
        .map(|p| format!("rule {rule:?}: {}", p.reason))
}

pub(crate) fn rule_problem(rule: &str, is_allow: bool) -> Option<String> {
    if rule.chars().any(char::is_control) {
        return Some(format!("rule {rule:?} has a control character"));
    }
    // The parentheses are the rule's own, outermost and only: `Read(x),Bash(y)` is two rules.
    let (name, arg) = match rule.find('(') {
        None => (rule, None),
        Some(i) => match rule[i + 1..].strip_suffix(')') {
            Some(inner) if !inner.contains(['(', ')']) => (&rule[..i], Some(inner)),
            _ => {
                return Some(format!(
                    "rule {rule:?} must be Name or Name(arg), arg without parentheses"
                ))
            }
        },
    };
    if name.starts_with('-') || name.contains(',') || name.contains(' ') {
        return Some(format!(
            "rule {rule:?} has a comma, space or leading - outside the parentheses"
        ));
    }
    let mcp_ok = name.strip_prefix("mcp__watane__").is_some_and(|t| {
        !t.is_empty()
            && t.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    });
    if !TOOLS.contains(&name) && !mcp_ok {
        return Some(format!(
            "rule {rule:?} names no tool of the allowlist or mcp__watane__<tool>"
        ));
    }
    if is_allow && name == "Bash" {
        if arg.is_none() {
            return Some(format!("rule {rule:?} allows any command"));
        }
        if let Some(c) = arg.and_then(|a| {
            a.chars()
                .find(|c| !c.is_ascii() || "`${};|&<>'\"\\".contains(*c))
        }) {
            return Some(format!(
                "rule {rule:?} has the character {c:?}, which the shell reads as more than a word"
            ));
        }
        if arg.is_some_and(|a| a.strip_suffix(":*").unwrap_or(a).contains(':')) {
            return Some(format!(
                "rule {rule:?} in allow: a colon is only allowed as the trailing :*"
            ));
        }
        if arg.is_some_and(|a| bash_words(a).next().is_some_and(|w| w.contains('/'))) {
            return Some(format!(
                "rule {rule:?} starts with a path executable the agent may rewrite"
            ));
        }
        if let Some(why) = arg.and_then(runner_problem) {
            return Some(format!("rule {rule:?} {why}"));
        }
    }
    None
}

/// Plain ASCII `[A-Za-z0-9._-]` segments joined by single `/`, none `.` or `..`.
fn bad_path(path: &str) -> bool {
    path.split('/').any(|s| {
        s.is_empty()
            || s == "."
            || s == ".."
            || !s
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    })
}

/// Why the frontmatter of a `SKILL.md` is refused, if it has one that is not plainly
/// `name:` and `description:` (4.1). Anything that might read as frontmatter must be strict.
fn frontmatter_problem(text: &str) -> Option<String> {
    let rest = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if !rest.starts_with("---") {
        return None;
    }
    let Some(body) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return Some("frontmatter must start with --- and a newline at byte 0".into());
    };
    const MAX_DESCRIPTION: usize = 1024;
    let mut seen: Vec<&str> = Vec::new();
    let mut description_len = 0;
    for raw in body.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line == "---" {
            return None;
        }
        if let Some(c) = line.chars().find(|c| !(' '..='~').contains(c)) {
            return Some(format!(
                "frontmatter has the character {c:?}: only printable ASCII is allowed"
            ));
        }
        if line.starts_with(' ') && !seen.is_empty() {
            if line.contains(':') {
                return Some(format!("frontmatter continuation line {line:?} has a ':'"));
            }
            if seen.last() == Some(&"description") {
                description_len += 1 + line.trim().len();
            }
        } else {
            let key = ["name", "description"]
                .into_iter()
                .find(|k| line.strip_prefix(k).is_some_and(|r| r.starts_with(':')));
            match key {
                Some(k) if !seen.contains(&k) => {
                    seen.push(k);
                    if let Some(value) = line.strip_prefix("description:") {
                        description_len = value.trim().len();
                    }
                }
                Some(k) => return Some(format!("frontmatter key {k:?} appears twice")),
                None => {
                    return Some(format!(
                        "frontmatter line {line:?}: only name and description are allowed"
                    ))
                }
            }
        }
        // The grammar allows only printable ASCII, so bytes count characters here.
        if description_len > MAX_DESCRIPTION {
            return Some(format!(
                "frontmatter description exceeds {MAX_DESCRIPTION} characters"
            ));
        }
    }
    None
}

/// The grammar rules of 4.1, 4.2 and 9.2 on a typed profile.
pub fn grammar_problems(pr: &Profile) -> Vec<Problem> {
    let mut o = Vec::new();
    if let Some(b) = &pr.backend {
        if b.parse::<Backend>().is_err() {
            o.push(p("backend", format!("{b:?} is not claude or openai")));
        }
    }
    if let Some(m) = &pr.model {
        if !model_ok(m) {
            o.push(p(
                "model",
                format!("{m:?} is outside [A-Za-z0-9.\\-\\[\\]]{{1,64}}"),
            ));
        }
    }
    for m in pr.fallback_models.iter().filter(|m| !model_ok(m)) {
        o.push(p(
            "fallback-models",
            format!("{m:?} is outside [A-Za-z0-9.\\-\\[\\]]{{1,64}}"),
        ));
    }
    if pr.fallback_models.len() > 4 {
        o.push(p(
            "fallback-models",
            format!("{} entries, at most 4", pr.fallback_models.len()),
        ));
    }
    if let Some(e) = &pr.effort {
        if !EFFORTS.contains(&e.as_str()) {
            o.push(p(
                "effort",
                format!("{e:?} is not one of low, medium, high, xhigh, max"),
            ));
        }
    }
    if let Some(m) = &pr.mode {
        if !MODES.contains(&m.as_str()) {
            o.push(p(
                "mode",
                format!("{m:?} is not one of manual, accept-edits, plan, dont-ask"),
            ));
        }
    }
    if let Some(SystemPrompt::Replace(t) | SystemPrompt::Append(t)) = &pr.system_prompt {
        if t.len() > 64 * KIB {
            o.push(p(
                "system-prompt",
                format!("{} bytes, at most 64 KiB", t.len()),
            ));
        }
    }
    for t in pr.tools.iter().filter(|t| !TOOLS.contains(&t.as_str())) {
        o.push(p("tools", format!("{t:?} is not a tool of the allowlist")));
    }
    let openai = pr.backend.as_deref() == Some("openai");
    for (field, rules, is_allow) in [("allow", &pr.allow, true), ("deny", &pr.deny, false)] {
        for r in rules {
            let problem = if openai {
                openai_rule_problem(r)
            } else {
                rule_problem(r, is_allow)
            };
            if let Some(reason) = problem {
                o.push(p(field, reason));
            }
        }
    }
    if let Some(c) = &pr.pre_tool_check {
        if !(5..=120).contains(&c.timeout_s) {
            o.push(p(
                "pre-tool-check.timeout-s",
                format!("{} is outside 5..120", c.timeout_s),
            ));
        }
    }
    let mut sub_bytes = 0;
    for (i, s) in pr.subagents.iter().enumerate() {
        let pre = format!("subagents[{i}]");
        if pr.subagents[..i].iter().any(|k| k.name == s.name) {
            o.push(p(
                format!("{pre}.name"),
                format!("{:?} is already a subagent's name", s.name),
            ));
        }
        sub_bytes += s.name.len() + s.description.len() + s.prompt.len();
        sub_bytes += s.tools.iter().map(String::len).sum::<usize>();
        sub_bytes += s.model.as_ref().map_or(0, String::len);
        if !slug_ok(&s.name) {
            o.push(p(
                format!("{pre}.name"),
                format!("{:?} is outside [a-z0-9-]{{1,64}}", s.name),
            ));
        }
        for t in &s.tools {
            if !TOOLS.contains(&t.as_str()) {
                o.push(p(
                    format!("{pre}.tools"),
                    format!("{t:?} is not a tool of the allowlist"),
                ));
            } else if !pr.tools.contains(t) {
                o.push(p(
                    format!("{pre}.tools"),
                    format!("{t:?} is not in the profile's tools"),
                ));
            }
        }
        if let Some(m) = s.model.as_deref().filter(|m| !model_ok(m)) {
            o.push(p(
                format!("{pre}.model"),
                format!("{m:?} is outside [A-Za-z0-9.\\-\\[\\]]{{1,64}}"),
            ));
        }
    }
    if sub_bytes > 64 * KIB {
        o.push(p(
            "subagents",
            format!("{sub_bytes} bytes in all, at most 64 KiB"),
        ));
    }
    if let Some(d) = pr.subagent_depth {
        if d > 3 {
            o.push(p("subagent-depth", format!("{d} is outside 0..3")));
        }
    }
    let mut skill_bytes = 0;
    for (i, s) in pr.skills.iter().enumerate() {
        let pre = format!("skills[{i}]");
        if pr.skills[..i].iter().any(|k| k.name == s.name) {
            o.push(p(
                format!("{pre}.name"),
                format!("{:?} is already a skill's name", s.name),
            ));
        }
        if !slug_ok(&s.name) {
            o.push(p(
                format!("{pre}.name"),
                format!("{:?} is outside [a-z0-9-]{{1,64}}", s.name),
            ));
        }
        for (path, text) in &s.files {
            skill_bytes += path.len() + text.len();
            if bad_path(path) {
                o.push(p(
                    format!("{pre}.files"),
                    format!("path {path:?} must be plain [A-Za-z0-9._-] segments joined by /, without . or .. segments"),
                ));
            }
            let low = path.to_ascii_lowercase();
            if s.files
                .keys()
                .any(|k| k != path && k.to_ascii_lowercase() == low && k < path)
            {
                o.push(p(
                    format!("{pre}.files"),
                    format!("path {path:?} equals another path when lowercased"),
                ));
            }
            if low.rsplit('/').next() == Some("skill.md") {
                if let Some(why) = frontmatter_problem(text) {
                    o.push(p(format!("{pre}.files"), format!("{path} {why}")));
                }
            }
        }
    }
    if skill_bytes > 1024 * KIB {
        o.push(p(
            "skills",
            format!("{skill_bytes} bytes in all, at most 1 MiB"),
        ));
    }
    if let Some(j) = &pr.json_schema {
        let n = j.to_string().len();
        if n > 64 * KIB {
            o.push(p("json-schema", format!("{n} bytes, at most 64 KiB")));
        }
    }
    if let Some(n) = pr.max_turns.filter(|n| !(1..=1000).contains(n)) {
        o.push(p("max-turns", format!("{n} is outside 1..1000")));
    }
    if let Some(n) = pr.max_budget_usd.filter(|n| n.is_nan() || *n <= 0.0) {
        o.push(p("max-budget-usd", format!("{n} is not positive")));
    }
    if pr.max_tool_errors == Some(0) {
        o.push(p("max-tool-errors", "0 is not positive"));
    }
    if pr.max_tokens == Some(0) {
        o.push(p("max-tokens", "0 is not positive"));
    }
    o
}
