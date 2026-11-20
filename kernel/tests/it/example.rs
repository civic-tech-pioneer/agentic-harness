//! The `check` example's `run`, with a fake file reader (AC7).

#[allow(dead_code)]
#[path = "../../examples/check.rs"]
mod check_example;

use check_example::run;
use std::io;

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn files(path: &str) -> io::Result<String> {
    match path {
        "p.json" => Ok(r#"{"tools":["Read"],"max-budget-usd":2}"#.into()),
        "ok.json" => Ok("{}".into()),
        "o.json" => Ok(r#"{"mode":"plan"}"#.into()),
        "cat.json" => Ok(r#"{"models":[{"slug":"gpt-5","efforts":["low"]}]}"#.into()),
        "bad.json" => Ok("[".into()),
        _ => Err(io::Error::new(io::ErrorKind::NotFound, "no such file")),
    }
}

const NOTE: &str = "not checked against a list";

#[test]
fn ac7_prints_one_json_line_per_refusal_and_exits_1() {
    let o = run(&args(&["p.json", "o.json", "openai", "gpt-5"]), files);
    assert_eq!(o.code, 1);
    let lines: Vec<serde_json::Value> = o
        .stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3, "{}", o.stdout);
    assert!(o.stdout.lines().any(|l| l
        == r#"{"field":"overlay.mode","backend":"openai","model":"gpt-5","reason":"changes the permission mode"}"#));
    assert!(o.stderr.contains(NOTE));
}

#[test]
fn ac7_prints_nothing_and_exits_0_when_clean() {
    let o = run(&args(&["ok.json", "ok.json", "claude", "m"]), files);
    assert_eq!((o.code, o.stdout.as_str()), (0, ""));
    assert_eq!(o.stderr.matches('\n').count(), 1);
    assert!(o.stderr.contains(NOTE));
}

#[test]
fn ac7_a_catalog_file_is_used_and_silences_the_note() {
    let o = run(
        &args(&["ok.json", "ok.json", "openai", "gpt-9", "cat.json"]),
        files,
    );
    assert_eq!(o.code, 1);
    assert!(o.stdout.contains("/v1/models") && o.stderr.is_empty());
    let o = run(
        &args(&["ok.json", "ok.json", "openai", "gpt-5", "cat.json"]),
        files,
    );
    assert_eq!((o.code, o.stdout.as_str(), o.stderr.as_str()), (0, "", ""));
}

#[test]
fn ac7_bad_calls_are_usage_and_exit_2() {
    let bad: [&[&str]; 6] = [
        &["p.json", "o.json", "claude"],
        &["p.json", "o.json", "claude", "m", "cat.json", "x"],
        &["nope.json", "o.json", "claude", "m"],
        &["p.json", "o.json", "gemini", "m"],
        &["p.json", "o.json", "claude", "m", "nope.json"],
        &["p.json", "o.json", "claude", "m", "bad.json"],
    ];
    for a in bad {
        let o = run(&args(a), files);
        assert_eq!(o.code, 2, "{a:?}");
        assert!(o.stdout.is_empty() && o.stderr.contains("usage:"), "{a:?}");
    }
}
