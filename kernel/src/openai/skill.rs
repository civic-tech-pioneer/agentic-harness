//! The OpenAI `skill` built-in (TECHNICAL 11). It reads only the compiled skill snapshot.

use crate::profile::Skill;
use serde::Deserialize;
use serde_json::{json, Value};

pub(crate) const SKILL: &str = "skill";

pub(crate) fn definition(skills: &[Skill]) -> Value {
    let mut listing = String::from("Load the SKILL.md for one of the available skills.\n<skills>");
    for skill in skills {
        let description = skill
            .files
            .get("SKILL.md")
            .map_or_else(String::new, |md| description(md));
        listing.push_str(&format!(
            "\n<skill><name>{}</name><description>{}</description></skill>",
            escape(&skill.name),
            escape(&description),
        ));
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

pub(crate) fn read<'a>(skills: &'a [Skill], call: &Value) -> Result<&'a str, &'static str> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        name: String,
    }
    let args: Args = call["arguments"]
        .as_str()
        .and_then(|args| serde_json::from_str(args).ok())
        .ok_or("skill requires one string argument named name")?;
    skills
        .iter()
        .find(|skill| skill.name == args.name)
        .and_then(|skill| skill.files.get("SKILL.md"))
        .map(String::as_str)
        .ok_or("the skill is not in the session's profile")
}
