//! The OpenAI rule matcher (TECHNICAL 9.6): deny, then ask, then allow; the strictest match wins.

use crate::error::Problem;
use std::collections::HashSet;

/// The decision for one tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone)]
struct Rule {
    name: String,
    /// `None` is a bare rule; `Some` is a glob over the call's argument.
    pat: Option<Vec<char>>,
}

/// Compiled allow, ask and deny rules.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    allow: Vec<Rule>,
    ask: Vec<Rule>,
    deny: Vec<Rule>,
    disabled: HashSet<String>,
}

enum Parsed {
    Rule(Rule),
    /// Does not parse, but names a tool: that tool is disabled.
    Disable(String),
    /// A path rule.
    Path,
    /// No readable name.
    Nameless,
}

fn valid_name(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn is_path(arg: &str) -> bool {
    arg.starts_with(['/', '~'])
        || arg.starts_with("./")
        || arg.starts_with("../")
        || arg == "."
        || arg == ".."
        || arg.contains("**")
}

fn parse(rule: &str) -> Parsed {
    let head = rule.split('(').next().unwrap_or("");
    if !valid_name(head) {
        return Parsed::Nameless;
    }
    let name = head.to_string();
    if rule.chars().any(char::is_control) {
        return Parsed::Disable(name);
    }
    if rule.len() == head.len() {
        return Parsed::Rule(Rule { name, pat: None });
    }
    let Some(arg) = rule[head.len() + 1..].strip_suffix(')') else {
        return Parsed::Disable(name);
    };
    if arg.is_empty() || arg.contains(['(', ')']) {
        return Parsed::Disable(name);
    }
    if is_path(arg) {
        return Parsed::Path;
    }
    Parsed::Rule(Rule {
        name,
        pat: Some(arg.chars().collect()),
    })
}

/// Whole-string glob match: `*` any run, `?` one character. Greedy with one backtrack point.
pub(crate) fn glob(pat: &[char], text: &[char]) -> bool {
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pat.len() && pat[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if p < pat.len() && (pat[p] == '?' || pat[p] == text[t]) {
            p += 1;
            t += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    pat[p..].iter().all(|&c| c == '*')
}

impl Rule {
    fn matches(&self, tool: &str, arg: Option<&[char]>) -> bool {
        if self.name != tool {
            return false;
        }
        match (&self.pat, arg) {
            (None, _) => true,
            (Some(p), Some(a)) => glob(p, a),
            (Some(_), None) => false,
        }
    }
}

impl Rules {
    /// Problems are path rules and rules with no readable name; the Rules are usable anyway.
    pub fn compile(allow: &[String], ask: &[String], deny: &[String]) -> (Rules, Vec<Problem>) {
        let mut rules = Rules::default();
        let mut problems = Vec::new();
        for (field, list) in [("allow", allow), ("ask", ask), ("deny", deny)] {
            for text in list {
                match parse(text) {
                    Parsed::Rule(r) => match field {
                        "allow" => rules.allow.push(r),
                        "ask" => rules.ask.push(r),
                        _ => rules.deny.push(r),
                    },
                    Parsed::Disable(name) => {
                        rules.disabled.insert(name);
                    }
                    Parsed::Path => problems.push(Problem::new(
                        field,
                        "a path rule is not supported for OpenAI sessions",
                    )),
                    Parsed::Nameless => problems.push(Problem::new(
                        field,
                        "a rule needs a tool name of 1 to 64 letters, digits, _ or -",
                    )),
                }
            }
        }
        (rules, problems)
    }

    pub fn decide(&self, tool: &str, arg: Option<&str>) -> Verdict {
        if self.disabled.contains(tool) {
            return Verdict::Deny;
        }
        let chars: Option<Vec<char>> = arg.map(|a| a.chars().collect());
        let hit = |rs: &[Rule]| rs.iter().any(|r| r.matches(tool, chars.as_deref()));
        if hit(&self.deny) {
            Verdict::Deny
        } else if hit(&self.ask) {
            Verdict::Ask
        } else if hit(&self.allow) {
            Verdict::Allow
        } else {
            Verdict::Ask
        }
    }
}
