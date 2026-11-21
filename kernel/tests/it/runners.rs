use crate::{fields, parse};
use agent_kernel::profile::{runner_prefixes, runner_words};
use std::collections::BTreeSet;

fn spec_41() -> String {
    let t = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../design/TECHNICAL.md"
    ))
    .unwrap();
    let i = t.find("#### 4.1").or_else(|| t.find("4.1 ")).unwrap_or(0);
    t[i..].to_string()
}

fn ticks(s: &str) -> BTreeSet<String> {
    s.split('`').skip(1).step_by(2).map(String::from).collect()
}

fn spec_words() -> BTreeSet<String> {
    let t = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../design/TECHNICAL.md"
    ))
    .unwrap();
    let m = "and none of these, because each runs, wraps or schedules";
    let i = t.find(m).expect("marker sentence");
    let rest = &t[i + m.len()..];
    let rest = &rest[rest.find("command:").unwrap() + 8..];
    let end = rest.find("`.\n").expect("end of sentence") + 1;
    ticks(&rest[..end])
}

fn diff(a: &BTreeSet<String>, b: &BTreeSet<String>) -> Vec<String> {
    a.difference(b).cloned().collect()
}

#[test]
fn ac1_runners_equal_spec_list() {
    let spec = spec_words();
    assert!(spec.len() >= 90, "parsed only {}", spec.len());
    let code: BTreeSet<String> = runner_words().iter().map(|s| s.to_string()).collect();
    assert_eq!(
        diff(&spec, &code),
        Vec::<String>::new(),
        "in spec, missing in code"
    );
    assert_eq!(
        diff(&code, &spec),
        Vec::<String>::new(),
        "in code, missing in spec"
    );
}

#[test]
fn ac2_prefixes_equal_spec_list() {
    let t = spec_41();
    let i = t
        .find("no name\n  starting")
        .or_else(|| t.find("no name starting"))
        .unwrap();
    let rest = &t[i..];
    let end = rest.find('(').unwrap();
    let spec = ticks(&rest[..end]);
    let code: BTreeSet<String> = runner_prefixes().iter().map(|s| s.to_string()).collect();
    assert_eq!(spec, code);
    assert_eq!(code.len(), 7);
}

fn refused(rule: &str) -> bool {
    let (_, ps) = parse(&format!(r#"{{"allow":["{rule}"]}}"#));
    fields(&ps).contains(&"allow")
}

#[test]
fn ac3_every_runner_and_prefix_refused() {
    for w in runner_words() {
        for r in [
            format!("Bash({w})"),
            format!("Bash({w} foo)"),
            format!("Bash({}:*)", w.to_uppercase()),
        ] {
            assert!(refused(&r), "{r}");
        }
    }
    for p in runner_prefixes() {
        for r in [
            format!("Bash({p})"),
            format!("Bash({p}3.12 x)"),
            format!("Bash({p}js)"),
        ] {
            assert!(refused(&r), "{r}");
        }
    }
}

#[test]
fn ac4_non_runners_allowed() {
    for r in [
        "git status",
        "ls",
        "cargo test",
        "make",
        "rg foo",
        "npm test",
        "yarn list",
    ] {
        assert!(!refused(&format!("Bash({r})")), "{r}");
    }
}

#[test]
fn ac5_yarn_ls_refused() {
    assert!(refused("Bash(yarn ls)"));
}

fn skill_problems(md: &str) -> Vec<agent_kernel::Problem> {
    let doc = serde_json::json!({"skills": [{"name": "a", "files": {"SKILL.md": md}}]});
    parse(&doc.to_string()).1
}

#[test]
fn ac6_continuation_line_with_colon_refused() {
    let ps = skill_problems("---\nname: a\ndescription: b\n  more: text\n---\n");
    assert!(
        ps.iter()
            .any(|p| p.reason.contains("continuation") && p.reason.contains("':'")),
        "{ps:?}"
    );
    assert!(skill_problems("---\nname: a\ndescription: b\n  more text\n---\n").is_empty());
}
