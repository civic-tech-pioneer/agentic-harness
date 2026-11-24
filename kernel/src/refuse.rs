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

fn is_path_rule(rule: &str) -> bool {
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
        }
        Backend::Openai => {
            if !p.fallback_models.is_empty() {
                add("fallback-models", "refused unless empty on openai");
            }
            if !p.tools.is_empty() {
                add("tools", "refused unless empty on openai");
            }
            if p.allow.iter().any(|r| is_path_rule(r)) {
                add("allow", "a path rule is refused on openai");
            }
            if p.deny.iter().any(|r| is_path_rule(r)) {
                add("deny", "a path rule is refused on openai");
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
            if !p.skills.is_empty() {
                add("skills", "skills wait for the skill tool on openai");
            }
            for (i, s) in p.skills.iter().enumerate() {
                if s.files.keys().any(|k| k != "SKILL.md") {
                    add(
                        &format!("skills[{i}].files"),
                        "a skill with files besides SKILL.md is refused on openai",
                    );
                }
            }
            if p.sandbox.is_some() {
                add("sandbox", "refused unless absent on openai");
            }
            if p.max_budget_usd.is_some() {
                add("max-budget-usd", "refused on openai");
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
