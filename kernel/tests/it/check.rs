use agent_kernel::{check, Backend};
use serde_json::json;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;

fn fields(p: &str, o: serde_json::Value, b: Backend) -> Vec<String> {
    check(p, &o.to_string(), b, "m", None)
        .into_iter()
        .map(|r| r.field)
        .collect()
}

#[test]
fn ac2_every_widening_field_is_refused_at_once() {
    let o = json!({"tools+": ["Edit"], "allow+": ["Read"], "workspaces+": ["x"],
        "subagents+": [], "mode": "dont-ask"});
    let mut f = fields(P, o, Backend::Claude);
    f.sort();
    assert_eq!(
        f,
        [
            "overlay.allow+",
            "overlay.mode",
            "overlay.subagents+",
            "overlay.tools+",
            "overlay.workspaces+"
        ]
    );
}

#[test]
fn ac3_a_full_overlay_compiles_clean_on_claude() {
    let o = json!({"system-prompt-append": "x", "skills+": [{"name": "s", "files": {"SKILL.md": "t"}}],
        "tools-": ["Read"], "deny+": ["Bash"], "max-turns": 5});
    assert!(fields(P, o, Backend::Claude).is_empty());
}

#[test]
fn ac5_compiled_profile_meets_grammar_and_table() {
    let rs = check(
        P,
        &json!({"deny+": ["Nope(x)"]}).to_string(),
        Backend::Claude,
        "m",
        None,
    );
    assert_eq!(
        rs.iter().map(|r| r.field.as_str()).collect::<Vec<_>>(),
        ["deny"]
    );
    let o = json!({"skills+": [{"name": "s", "files": {"SKILL.md": "t", "x.md": "u"}}]});
    assert_eq!(fields(r#"{}"#, o, Backend::Openai), ["skills[0].files"]);
}

#[test]
fn ac5_duplicates_are_reported_once() {
    // the bad rule is in the profile and survives into the compiled profile
    let rs = check(r#"{"deny":["Nope(x)"]}"#, "{}", Backend::Claude, "m", None);
    assert_eq!(rs.len(), 1);
}

#[test]
fn ac6_all_sources_together_with_backend_and_model() {
    let rs = check(
        r#"{"effort":"huge","max-budget-usd":1}"#,
        r#"{"mode":"plan"}"#,
        Backend::Openai,
        "gpt-5",
        None,
    );
    let mut f: Vec<&str> = rs.iter().map(|r| r.field.as_str()).collect();
    f.sort();
    assert_eq!(f, ["effort", "max-budget-usd", "overlay.mode"]);
    assert!(rs
        .iter()
        .all(|r| r.backend == "openai" && r.model == "gpt-5"));
}

#[test]
fn ac6_a_broken_profile_still_has_its_overlay_checked() {
    let rs = check("{", r#"{"tools+": []}"#, Backend::Claude, "m", None);
    let mut f: Vec<&str> = rs.iter().map(|r| r.field.as_str()).collect();
    f.sort();
    assert_eq!(f, ["overlay.tools+", "profile"]);
}

#[test]
fn s148_check_skill_description_length_before_escaping() {
    for backend in [Backend::Openai, Backend::Claude] {
        for (description, too_long) in [
            ("a".repeat(1000 * 1024), true),
            ("&".repeat(1000 * 1024), true),
            ("&".repeat(1024), false),
            (format!("{}\n b", "a".repeat(1022)), false),
            (format!("{}\n b", "a".repeat(1023)), true),
        ] {
            let profile = json!({"skills": [{"name":"one", "files": {
                "SKILL.md": format!("---\ndescription: {description}\n---\nbody"),
            }}]});
            let refusals = check(&profile.to_string(), "{}", backend, "m", None);
            if too_long {
                assert_eq!(refusals.len(), 1, "{refusals:?}");
                assert_eq!(refusals[0].field, "skills[0].files");
                assert!(refusals[0].reason.contains("description"));
                assert!(refusals[0].reason.contains("1024"));
            } else {
                assert!(refusals.is_empty(), "{refusals:?}");
            }
        }
    }
}
