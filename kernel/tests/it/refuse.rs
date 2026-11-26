use crate::parse;
use agent_kernel::{refuse, Backend, Catalog, CatalogModel};
use serde_json::json;

fn names(doc: serde_json::Value, b: Backend, model: &str, cat: Option<&Catalog>) -> Vec<String> {
    let (p, ps) = parse(&doc.to_string());
    assert!(ps.is_empty(), "{ps:?}");
    refuse(&p, b, model, cat)
        .into_iter()
        .map(|r| r.field)
        .collect()
}

#[test]
fn ac7_openai_refuses_every_field_at_once() {
    let doc = json!({
        "tools": ["Read", "Edit"], "sandbox": {"enabled": true}, "max-budget-usd": 2,
        "allow": ["Read(./src/**)"], "fallback-models": ["gpt-y"], "mode": "plan",
        "subagents": [{"name": "a", "description": "d", "prompt": "p", "tools": ["Read"]}],
        "skills": [{"name": "s", "files": {"SKILL.md": "x", "extra.txt": "y"}}]
    });
    let (p, ps) = parse(&doc.to_string());
    assert!(ps.is_empty());
    let rs = refuse(&p, Backend::Openai, "gpt-x", None);
    let mut f: Vec<&str> = rs.iter().map(|r| r.field.as_str()).collect();
    f.sort();
    let mut want = [
        "tools",
        "sandbox",
        "max-budget-usd",
        "allow",
        "fallback-models",
        "mode",
        "subagents[0].tools",
        "skills[0].files",
    ];
    want.sort();
    assert_eq!(f, want);
    assert!(rs
        .iter()
        .all(|r| r.backend == "openai" && r.model == "gpt-x" && !r.reason.is_empty()));
}

#[test]
fn ac7_openai_accepts_the_plain_profile() {
    let doc = json!({"allow": ["Bash(git status:*)"], "mode": "dont-ask",
        "ask-expiry": 30});
    assert!(names(doc, Backend::Openai, "gpt-x", None).is_empty());
}

#[test]
fn ac8_claude_ask_expiry_and_sandbox() {
    assert_eq!(
        names(json!({"ask-expiry": 60}), Backend::Claude, "m", None),
        ["ask-expiry"]
    );
    assert!(names(json!({"ask-expiry": 600}), Backend::Claude, "m", None).is_empty());
    for t in ["Bash", "WebFetch", "WebSearch"] {
        assert_eq!(
            names(json!({"tools": [t]}), Backend::Claude, "m", None),
            ["sandbox"]
        );
    }
    assert!(names(
        json!({"tools": ["Bash"], "sandbox": {"enabled": true}}),
        Backend::Claude,
        "m",
        None
    )
    .is_empty());
    assert!(names(json!({"tools": ["Read"]}), Backend::Claude, "m", None).is_empty());
}

#[test]
fn ac9_catalog_checks_model_and_effort() {
    let cat = Catalog {
        models: vec![CatalogModel {
            slug: "gpt-x".into(),
            efforts: vec!["low".into(), "high".into()],
        }],
    };
    let d = || json!({"effort": "max"});
    assert_eq!(names(d(), Backend::Openai, "gpt-q", Some(&cat)), ["model"]);
    assert_eq!(names(d(), Backend::Openai, "gpt-x", Some(&cat)), ["effort"]);
    assert!(names(
        json!({"effort": "low"}),
        Backend::Openai,
        "gpt-x",
        Some(&cat)
    )
    .is_empty());
    assert!(names(d(), Backend::Openai, "gpt-q", None).is_empty());
}

#[test]
fn ac9_model_argument_grammar_on_both_backends() {
    for b in [Backend::Claude, Backend::Openai] {
        assert_eq!(names(json!({}), b, "bad model", None), ["model"]);
        assert_eq!(names(json!({}), b, "", None), ["model"]);
        assert_eq!(names(json!({}), b, &"a".repeat(65), None), ["model"]);
        assert!(names(json!({}), b, "claude-opus-5-5[1m]", None).is_empty());
    }
    assert_eq!("openai".parse::<Backend>(), Ok(Backend::Openai));
    assert!("x".parse::<Backend>().is_err());
}

#[test]
fn s148_ac2_single_file_skills_are_accepted_by_both_backends() {
    let doc = json!({"skills": [{"name": "s", "files": {"SKILL.md": "x"}}]});
    assert!(names(doc.clone(), Backend::Openai, "gpt-x", None).is_empty());
    assert!(names(doc, Backend::Claude, "m", None).is_empty());
}
