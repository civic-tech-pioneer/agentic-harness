//! The refuse table: TECHNICAL 11, for a backend and a model; AGT-F-4.

use crate::error::{Problem, Refusal};
use crate::profile::{model_ok, Backend, Profile};
use serde::Deserialize;

/// The OpenAI models the account lists, each with its reasoning efforts.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Catalog {
    pub models: Vec<CatalogModel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogModel {
    pub slug: String,
    pub efforts: Vec<String>,
}

const PATH_TOOLS: [&str; 6] = ["Read", "Write", "Edit", "Glob", "Grep", "NotebookEdit"];

pub(crate) fn is_path_rule(rule: &str) -> bool {
    rule.split_once('(')
        .is_some_and(|(name, _)| PATH_TOOLS.contains(&name))
}

/// Applies the refuse table; every refusal is returned at once.
pub fn refuse(
    p: &Profile,
    backend: Backend,
    model: &str,
    catalog: Option<&Catalog>,
) -> Vec<Refusal> {
    let mut o: Vec<Problem> = Vec::new();
    let mut add = |f: &str, r: &str| o.push(Problem::new(f, r));
    let model_valid = model_ok(model);
    if !model_valid {
        add("model", "outside [A-Za-z0-9.\\-\\[\\]]{1,64}");
    }
    match backend {
        Backend::Claude => {
            if p.ask_expiry.is_some_and(|s| s != 600) {
                add(
                    "ask-expiry",
                    "Claude Code asks expire after 10 minutes (600 seconds) only",
                );
            }
            let shell = p
                .tools
                .iter()
                .any(|t| ["Bash", "WebFetch", "WebSearch"].contains(&t.as_str()));
            if shell && p.sandbox.is_none() {
                add(
                    "sandbox",
                    "tools with Bash, WebFetch or WebSearch need a sandbox (4.2)",
                );
            }
            if !p.delegates.is_empty() {
                add("delegates", "refused on claude: delegation is OpenAI only");
            }
            if p.subagent_limits.is_some() {
                add(
                    "subagent-limits",
                    "refused on claude: it limits OpenAI child sessions",
                );
            }
            if p.max_tool_errors.is_some() {
                add("max-tool-errors", "refused on claude");
            }
            if p.max_tokens.is_some() {
                add("max-tokens", "refused on claude");
            }
        }
        Backend::Openai => {
            if !p.fallback_models.is_empty() {
                add("fallback-models", "refused unless empty on openai");
            }
            if !p.tools.is_empty() {
                add("tools", "refused unless empty on openai");
            }
            if p.allow.iter().any(|r| is_path_rule(r)) {
                add("allow", "an argument rule is refused on openai");
            }
            if p.deny.iter().any(|r| is_path_rule(r)) {
                add("deny", "an argument rule is refused on openai");
            }
            if p.mode
                .as_deref()
                .is_some_and(|m| m != "manual" && m != "dont-ask")
            {
                add("mode", "openai takes manual or dont-ask only");
            }
            for (i, s) in p.subagents.iter().enumerate() {
                if !s.tools.is_empty() {
                    add(
                        &format!("subagents[{i}].tools"),
                        "a subagent with tools is refused on openai",
                    );
                }
            }
            for (i, s) in p.skills.iter().enumerate() {
                if s.files.keys().any(|k| k != "SKILL.md") {
                    add(
                        &format!("skills[{i}].files"),
                        "a skill with files besides SKILL.md is refused on openai",
                    );
                } else if !s.files.contains_key("SKILL.md") {
                    add(
                        &format!("skills[{i}].files"),
                        "an OpenAI skill needs SKILL.md",
                    );
                }
            }
            if p.sandbox.is_some() {
                add("sandbox", "refused unless absent on openai");
            }
            if p.max_budget_usd.is_some() {
                add("max-budget-usd", "refused on openai");
            }
            if let Some(v) = &p.subagent_limits {
                for (k, x) in v.as_object().into_iter().flatten() {
                    let f = format!("subagent-limits.{k}");
                    match k.as_str() {
                        "live" if !x.as_u64().is_some_and(|n| (1..=20).contains(&n)) => {
                            add(&f, "a whole number from 1 to 20");
                        }
                        "depth" if x.as_u64() != Some(1) => add(&f, "only 1"),
                        "live" | "depth" => {}
                        _ => add(&f, "not a subagent-limits key (live, depth)"),
                    }
                }
            }
            if let (Some(c), true) = (catalog, model_valid) {
                match c.models.iter().find(|m| m.slug == model) {
                    None => add("model", "not a slug the account lists in /v1/models"),
                    Some(m) => {
                        if p.effort.as_ref().is_some_and(|e| !m.efforts.contains(e)) {
                            add("effort", "the model does not list that effort level");
                        }
                    }
                }
            }
        }
    }
    o.iter().map(|x| x.refusal(backend, model)).collect()
}
