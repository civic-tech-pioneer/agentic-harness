//! Overlays: the per-session narrowing of a profile (TECHNICAL 9.2; AGT-F-3). An overlay can only
//! narrow: every key that would widen is a problem named `overlay.<key>`.

use crate::error::Problem;
use crate::profile::{
    kind, list, narrow, skills, string, uint, wrong, Profile, Skill, SystemPrompt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Overlay {
    #[serde(
        rename = "system-prompt-append",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_prompt_append: Option<String>,
    #[serde(rename = "skills+", skip_serializing_if = "Vec::is_empty")]
    pub skills_plus: Vec<Skill>,
    #[serde(rename = "tools-", skip_serializing_if = "Vec::is_empty")]
    pub tools_minus: Vec<String>,
    #[serde(rename = "deny+", skip_serializing_if = "Vec::is_empty")]
    pub deny_plus: Vec<String>,
    #[serde(rename = "max-turns", skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(rename = "max-tool-errors", skip_serializing_if = "Option::is_none")]
    pub max_tool_errors: Option<u32>,
    #[serde(rename = "max-tokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(rename = "max-budget-usd", skip_serializing_if = "Option::is_none")]
    pub max_budget_usd: Option<f64>,
    #[serde(rename = "subagent-depth", skip_serializing_if = "Option::is_none")]
    pub subagent_depth: Option<u8>,
}

fn widening(key: &str) -> &'static str {
    match key {
        "tools" | "tools+" => "adds tools",
        "allow" | "allow+" => "adds allow rules",
        "workspace" | "extra-workspaces" | "workspaces+" => "adds workspaces",
        "subagents" | "subagents+" => "adds subagent profiles",
        "mode" => "changes the permission mode",
        "system-prompt" => "replaces the instructions",
        _ => "not an overlay field",
    }
}

/// Parses an overlay document; every valid key is kept, every problem is returned.
pub fn parse_overlay(json: &str) -> (Overlay, Vec<Problem>) {
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return (
                Overlay::default(),
                vec![Problem::new("overlay", format!("not valid JSON: {e}"))],
            )
        }
    };
    let Value::Object(m) = &v else {
        let why = format!("not a JSON object: it is {}", kind(&v));
        return (Overlay::default(), vec![Problem::new("overlay", why)]);
    };
    let mut o = Overlay::default();
    let mut out = Vec::new();
    for (k, x) in m {
        let f = format!("overlay.{k}");
        let mut own = Vec::new();
        let e = &mut own;
        match k.as_str() {
            "system-prompt-append" => o.system_prompt_append = string(x, &f, e),
            "skills+" => o.skills_plus = skills(x, e),
            "tools-" => o.tools_minus = list(x, &f, e),
            "deny+" => o.deny_plus = list(x, &f, e),
            "max-turns" => o.max_turns = narrow(x, &f, "1..1000", e),
            "max-tool-errors" => o.max_tool_errors = narrow(x, &f, "1..4294967295", e),
            "max-tokens" => o.max_tokens = uint(x, &f, e),
            "max-budget-usd" => match x.as_f64() {
                Some(n) => o.max_budget_usd = Some(n),
                None => e.push(wrong(&f, "a number", x)),
            },
            "subagent-depth" => o.subagent_depth = narrow(x, &f, "0..3", e),
            other => e.push(Problem::new(f.clone(), widening(other))),
        }
        for mut pr in own {
            // `skills()` names its problems `skills[i]...`; they belong to the key `skills+`.
            if let Some(rest) = pr.field.strip_prefix("skills") {
                pr.field = format!("overlay.skills+{rest}");
            }
            out.push(pr);
        }
    }
    (o, out)
}

fn cap<T: PartialOrd + std::fmt::Display>(
    key: &str,
    want: Option<T>,
    have: Option<T>,
    slot: &mut Option<T>,
    out: &mut Vec<Problem>,
) {
    let Some(w) = want else { return };
    // `w <= h` and not `!(w > h)`: a NaN (only a typed overlay can hold one) is refused.
    match have {
        Some(h) if !matches!(w.partial_cmp(&h), Some(Ordering::Less | Ordering::Equal)) => out
            .push(Problem::new(
                format!("overlay.{key}"),
                format!("{w} is higher than the profile's {h}; an overlay can only narrow"),
            )),
        _ => *slot = Some(w),
    }
}

/// Compiles an overlay onto a profile; a limit above the profile's is a problem and not applied.
pub fn apply_overlay(p: &Profile, o: &Overlay) -> (Profile, Vec<Problem>) {
    let mut c = p.clone();
    let mut out = Vec::new();
    if let Some(t) = &o.system_prompt_append {
        let join = |s: &str| {
            if s.is_empty() {
                t.clone()
            } else {
                format!("{s}\n\n{t}")
            }
        };
        c.system_prompt = Some(match &p.system_prompt {
            Some(SystemPrompt::Replace(s)) => SystemPrompt::Replace(join(s)),
            Some(SystemPrompt::Append(s)) => SystemPrompt::Append(join(s)),
            None => SystemPrompt::Append(t.clone()),
        });
    }
    // An added skill may not take the name of another: it would replace that skill's files.
    for (i, s) in o.skills_plus.iter().enumerate() {
        if c.skills.iter().any(|k| k.name == s.name) {
            out.push(Problem::new(
                format!("overlay.skills+[{i}].name"),
                format!(
                    "{:?} is already a skill; an overlay can only add skills",
                    s.name
                ),
            ));
        } else {
            c.skills.push(s.clone());
        }
    }
    c.tools.retain(|t| !o.tools_minus.contains(t));
    for s in &mut c.subagents {
        s.tools.retain(|t| !o.tools_minus.contains(t));
    }
    c.deny.extend(o.deny_plus.iter().cloned());
    cap(
        "max-turns",
        o.max_turns,
        p.max_turns,
        &mut c.max_turns,
        &mut out,
    );
    cap(
        "max-tool-errors",
        o.max_tool_errors,
        p.max_tool_errors,
        &mut c.max_tool_errors,
        &mut out,
    );
    cap(
        "max-tokens",
        o.max_tokens,
        p.max_tokens,
        &mut c.max_tokens,
        &mut out,
    );
    cap(
        "max-budget-usd",
        o.max_budget_usd,
        p.max_budget_usd,
        &mut c.max_budget_usd,
        &mut out,
    );
    cap(
        "subagent-depth",
        o.subagent_depth,
        Some(p.subagent_depth.unwrap_or(1)),
        &mut c.subagent_depth,
        &mut out,
    );
    (c, out)
}
