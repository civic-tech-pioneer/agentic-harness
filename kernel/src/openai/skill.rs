//! The OpenAI `skill` built-in (TECHNICAL 11). It reads only the compiled skill snapshot.

use crate::error::KernelError;
use crate::profile::Skill;
use crate::quota::Metered;
use crate::state::State;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) const SKILL: &str = "skill";

pub type Listing = Vec<(String, String)>;
pub(crate) type Revisions = BTreeMap<String, [u8; 32]>;

pub(crate) fn listing(skills: &[Skill]) -> Listing {
    skills
        .iter()
        .map(|skill| {
            let description = skill
                .files
                .get("SKILL.md")
                .map_or_else(String::new, |md| description(md));
            let row = format!(
                "<skill><name>{}</name><description>{}</description></skill>",
                escape(&skill.name),
                escape(&description),
            );
            (skill.name.clone(), row)
        })
        .collect()
}

pub(crate) fn definition(rows: &Listing) -> Value {
    let mut listing = String::from("Load the SKILL.md for one of the available skills.\n<skills>");
    for (_, row) in rows {
        listing.push('\n');
        listing.push_str(row);
    }
    listing.push_str("\n</skills>");
    json!({
        "type": "function",
        "name": SKILL,
        "description": listing,
        "parameters": {
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "additionalProperties": false
        },
        "strict": true,
    })
}

// The profile grammar already checks the frontmatter. Treat its values as plain text;
// an unterminated header supplies no description.
fn description(md: &str) -> String {
    let mut lines = md.lines();
    if lines.next() != Some("---") {
        return String::new();
    }
    let mut description = Vec::new();
    let mut in_description = false;
    for line in lines {
        if line == "---" {
            return description.join(" ");
        }
        if let Some(value) = line.strip_prefix("description:") {
            in_description = true;
            description.push(value.trim());
        } else if in_description && line.starts_with(' ') {
            description.push(line.trim());
        } else {
            in_description = false;
        }
    }
    String::new()
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn name(call: &Value) -> Result<String, &'static str> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        name: String,
    }
    call["arguments"]
        .as_str()
        .filter(|args| args.trim_start().starts_with('{'))
        .and_then(|args| serde_json::from_str::<Args>(args).ok())
        .map(|args| args.name)
        .ok_or("skill requires one string argument named name")
}

pub(crate) fn revision(skill: &Skill) -> Option<[u8; 32]> {
    skill
        .files
        .get("SKILL.md")
        .map(|md| Sha256::digest(md.as_bytes()).into())
}

fn index_key(id: &str) -> String {
    format!("skill-revisions/{id}")
}

fn file_key(id: &str, name: &str) -> String {
    format!("skills/{id}/{name}")
}

pub(crate) fn revisions(state: &mut impl State, id: &str) -> Revisions {
    // Each entry is the validated name, a NUL, then the fixed-width SHA-256 digest.
    fn decode(mut bytes: &[u8]) -> Option<Revisions> {
        let mut revisions = Revisions::new();
        while !bytes.is_empty() {
            let end = bytes.iter().position(|b| *b == 0)?;
            let name = std::str::from_utf8(&bytes[..end]).ok()?;
            let (digest, rest) = bytes[end + 1..].split_at_checked(32)?;
            revisions.insert(name.to_owned(), digest.try_into().ok()?);
            bytes = rest;
        }
        Some(revisions)
    }

    state
        .get(&index_key(id))
        .and_then(|bytes| decode(&bytes))
        .unwrap_or_default()
}

pub(crate) fn save(
    state: &mut impl State,
    id: &str,
    skills: Vec<Skill>,
) -> Result<(), KernelError> {
    if skills.is_empty() {
        return Ok(());
    }
    let mut index = Vec::new();
    let too_large = |err: crate::state::TooLarge| KernelError::TooLarge(err.0 as u64);
    for mut skill in skills {
        let md = skill
            .files
            .remove("SKILL.md")
            .expect("compiled OpenAI skill has SKILL.md");
        let digest = Sha256::digest(md.as_bytes());
        state
            .put(&file_key(id, &skill.name), md.into_bytes())
            .map_err(too_large)?;
        index.extend_from_slice(skill.name.as_bytes());
        index.push(0);
        index.extend_from_slice(&digest);
    }
    state.put(&index_key(id), index).map_err(too_large)
}

pub(crate) fn read(state: &mut impl State, id: &str, name: &str) -> Option<String> {
    String::from_utf8(state.get(&file_key(id, name))?).ok()
}

pub(crate) fn delete<S: State>(state: &mut Metered<S>, id: &str) {
    for name in revisions(state, id).keys() {
        state.delete_uncharged(&file_key(id, name));
    }
    state.delete_uncharged(&index_key(id));
}
