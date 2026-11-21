use agent_kernel::{Rules, Verdict::*};
use std::time::{Duration, Instant};

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn rules(a: &[&str], k: &[&str], d: &[&str]) -> Rules {
    let (r, p) = Rules::compile(&s(a), &s(k), &s(d));
    assert!(p.is_empty(), "{p:?}");
    r
}

#[test]
fn ac1_bare_allow_and_unnamed_asks() {
    let r = rules(&["read"], &[], &[]);
    assert_eq!(r.decide("read", None), Allow);
    assert_eq!(r.decide("read", Some("x")), Allow);
    assert_eq!(r.decide("write", None), Ask);
}

#[test]
fn ac2_strictest_wins() {
    let r = rules(&["shell"], &["shell(git push*)"], &["shell(rm *)"]);
    assert_eq!(r.decide("shell", Some("git status")), Allow);
    assert_eq!(r.decide("shell", Some("git push origin")), Ask);
    assert_eq!(r.decide("shell", Some("rm -rf x")), Deny);
    let r = rules(
        &["shell", "shell(rm *)"],
        &["shell(git push*)"],
        &["shell(rm *)"],
    );
    assert_eq!(r.decide("shell", Some("rm -rf x")), Deny);
}

#[test]
fn ac3_globs() {
    let r = rules(&["fetch(https://example.com/*)"], &[], &[]);
    assert_eq!(r.decide("fetch", Some("https://example.com/a/b")), Allow);
    assert_eq!(r.decide("fetch", Some("https://example.org/")), Ask);
    let r = rules(&["get(v?)"], &[], &[]);
    assert_eq!(r.decide("get", Some("v1")), Allow);
    assert_eq!(r.decide("get", Some("v12")), Ask);
    assert_eq!(r.decide("get", Some("v")), Ask);
    assert_eq!(r.decide("get", Some("vé")), Allow);
    let r = rules(&["f(ab)"], &[], &[]);
    assert_eq!(r.decide("f", Some("abc")), Ask);
    assert_eq!(r.decide("f", Some("ab")), Allow);
    let r = rules(&["f(*)"], &[], &[]);
    assert_eq!(r.decide("f", None), Ask);
    assert_eq!(r.decide("f", Some("")), Allow);
}

#[test]
fn ac4_unparsed_rule_disables_its_tool() {
    for bad in [
        "shell(",
        "shell(a(b))",
        "shell()",
        "shell(a)x",
        "shell(a\nb)",
    ] {
        for list in 0..3 {
            let mut l = [vec![], vec![], vec![]];
            l[list] = vec![bad];
            let (r, p) = Rules::compile(&s(&l[0]), &s(&l[1]), &s(&l[2]));
            assert!(p.is_empty(), "{bad}: {p:?}");
            assert_eq!(r.decide("shell", Some("a")), Deny, "{bad}");
            assert_eq!(r.decide("shell", None), Deny, "{bad}");
            assert_eq!(r.decide("other", None), Ask, "{bad}");
        }
        let (r, _) = Rules::compile(&s(&["shell", bad, "other"]), &[], &[]);
        assert_eq!(r.decide("shell", Some("a")), Deny);
        assert_eq!(r.decide("other", None), Allow);
    }
}

#[test]
fn ac5_path_and_nameless_rules_are_problems() {
    for (list, bad) in [
        ("allow", "read(/etc/*)"),
        ("ask", "read(~/x)"),
        ("deny", "read(./x)"),
        ("allow", "read(../x)"),
        ("ask", "read(.)"),
        ("deny", "read(src/**)"),
    ] {
        let l = s(&[bad]);
        let e: Vec<String> = vec![];
        let (a, k, d) = match list {
            "allow" => (&l, &e, &e),
            "ask" => (&e, &l, &e),
            _ => (&e, &e, &l),
        };
        let (_, p) = Rules::compile(a, k, d);
        assert_eq!(p.len(), 1, "{bad}");
        assert_eq!(p[0].field, list);
        assert!(p[0].reason.contains("path rule"), "{}", p[0].reason);
    }
    let long = "a".repeat(65);
    for bad in ["(x)", "a b", "", long.as_str()] {
        let (_, p) = Rules::compile(&s(&[bad]), &[], &[]);
        assert_eq!(p.len(), 1, "{bad:?}");
        assert_eq!(p[0].field, "allow");
    }
}

#[test]
fn ac6_names_are_case_sensitive() {
    let r = rules(&["Read"], &[], &[]);
    assert_eq!(r.decide("Read", None), Allow);
    assert_eq!(r.decide("read", None), Ask);
}

#[test]
fn ac7_pathological_pattern_is_fast() {
    let r = rules(&["f(*a*a*a*a*a*a*a*a*b)"], &[], &[]);
    let arg = "a".repeat(20_000);
    let t = Instant::now();
    assert_eq!(r.decide("f", Some(&arg)), Ask);
    assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
}

#[test]
fn names_take_underscore_dash_and_64_chars_and_dotdot_is_a_path() {
    let n64 = "a".repeat(64);
    let r = rules(&["web_fetch", "x-y", n64.as_str()], &[], &[]);
    assert_eq!(r.decide("web_fetch", None), Allow);
    assert_eq!(r.decide("x-y", None), Allow);
    assert_eq!(r.decide(&n64, None), Allow);
    let (_, p) = Rules::compile(&s(&["read(..)"]), &[], &[]);
    assert_eq!(p.len(), 1);
    assert!(p[0].reason.contains("path rule"), "{}", p[0].reason);
}

#[test]
fn glob_star_in_the_middle_backtracks() {
    let r = rules(&["f(a*c)", "g(*b?)"], &[], &[]);
    assert_eq!(r.decide("f", Some("abcbc")), Allow);
    assert_eq!(r.decide("f", Some("abcd")), Ask);
    assert_eq!(r.decide("g", Some("abxbby")), Allow);
    assert_eq!(r.decide("g", Some("abxy")), Ask);
}
