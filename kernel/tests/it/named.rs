//! S40-5: `check` and `profiles` by name (TECHNICAL 2.3, 9.1).
use agent_kernel::{FakeHost, Kernel, MemState, State};

fn kernel(entries: &[(&str, &str)]) -> Kernel<MemState, FakeHost> {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    for (key, v) in entries
        .iter()
        .filter(|(key, _)| !key.starts_with("access/"))
    {
        k.state.put(key, v.as_bytes().to_vec()).unwrap();
    }
    for (key, v) in entries.iter().filter(|(key, _)| key.starts_with("access/")) {
        crate::fixture_access::put(&mut k.state, key, v.as_bytes().to_vec()).unwrap();
    }
    k
}

fn fields(rs: &[agent_kernel::Refusal]) -> Vec<&str> {
    rs.iter().map(|r| r.field.as_str()).collect()
}

#[test]
fn ac3_a_name_the_caller_may_not_use_is_one_refusal_whether_or_not_it_exists() {
    let mut k = kernel(&[("access/alice", r#"["p"]"#), ("profile/other", "{}")]);
    for name in ["other", "ghost"] {
        let rs = k.check_named("alice", name, "{}");
        assert_eq!(fields(&rs), ["profile"]);
    }
    let a = k.check_named("alice", "other", "{}");
    let b = k.check_named("alice", "ghost", "{}");
    assert_eq!(a[0].reason, b[0].reason);
    assert_eq!(fields(&k.check_named("bob", "p", "{}")), ["profile"]);
}

#[test]
fn ac3_a_usable_name_with_no_profile_says_so_but_scratch_is_built_in() {
    let mut k = kernel(&[("access/alice", r#"["p"]"#)]);
    let rs = k.check_named("alice", "p", "{}");
    assert_eq!(
        (rs[0].field.as_str(), rs[0].reason.as_str()),
        ("profile", "changed since the participant allowed it")
    );
    assert_eq!(rs.len(), 1);
    assert!(k.check_named("page", "scratch", "{}").is_empty());
    let mut fresh = kernel(&[]);
    assert!(fresh.check_named("page", "scratch", "{}").is_empty());
}

#[test]
fn ac3_openai_profiles_are_checked_for_their_backend_and_model() {
    let mut k = kernel(&[
        ("access/alice", r#"["nomodel","tools","skills","ok"]"#),
        ("profile/nomodel", r#"{"backend":"openai"}"#),
        (
            "profile/tools",
            r#"{"backend":"openai","model":"gpt-x","tools":["Read"]}"#,
        ),
        (
            "profile/skills",
            r#"{"backend":"openai","model":"gpt-x","skills":[{"name":"s","files":{"SKILL.md":"x"}}]}"#,
        ),
        ("profile/ok", r#"{"backend":"openai","model":"gpt-x"}"#),
    ]);
    assert_eq!(fields(&k.check_named("alice", "nomodel", "{}")), ["model"]);
    let t = k.check_named("alice", "tools", "{}");
    assert_eq!(fields(&t), ["tools"]);
    assert_eq!(
        (t[0].backend.as_str(), t[0].model.as_str()),
        ("openai", "gpt-x")
    );
    assert!(k.check_named("alice", "skills", "{}").is_empty());
    assert!(k.check_named("alice", "ok", "{}").is_empty());
    // the overlay is checked too
    let o = k.check_named("alice", "ok", r#"{"tools+":["Edit"]}"#);
    assert!(fields(&o).contains(&"overlay.tools+"));
}

#[test]
fn ac4_profiles_lists_the_access_list_in_order_from_the_stored_profiles() {
    let mut k = kernel(&[
        ("access/alice", r#"["b","gone","a","c"]"#),
        ("profile/a", r#"{"backend":"openai","model":"gpt-x"}"#),
        ("profile/b", r#"{"model":"opus"}"#),
        ("profile/c", "{}"),
        ("profile/unlisted", "{}"),
    ]);
    let ps = k.profiles("alice");
    let got: Vec<_> = ps
        .iter()
        .map(|p| (p.name.as_str(), p.backend.as_str(), p.model.as_deref()))
        .collect();
    assert_eq!(
        got,
        [
            ("b", "claude", Some("opus")),
            ("a", "openai", Some("gpt-x")),
            ("c", "claude", None)
        ]
    );
    assert!(k.profiles("bob").is_empty());
}

#[test]
fn ac4_the_page_with_no_access_gets_scratch_only() {
    let mut k = kernel(&[]);
    let ps = k.profiles("page");
    assert_eq!(ps.len(), 1);
    assert_eq!(
        (ps[0].name.as_str(), ps[0].backend.as_str()),
        ("scratch", "claude")
    );
    assert!(ps[0].model.is_none());
    // a stored scratch with no access/page lists nothing; an unreadable access lists none
    let mut k = kernel(&[("profile/scratch", "{}")]);
    assert!(k.profiles("page").is_empty());
    let mut k = kernel(&[("access/page", "nonsense")]);
    assert!(k.profiles("page").is_empty());
}

#[test]
fn ac4_at_most_64_profiles_and_65_reads() {
    let mut entries: Vec<(String, String)> = Vec::new();
    let names: Vec<String> = (0..64).map(|i| format!("p{i}")).collect();
    entries.push((
        "access/alice".into(),
        serde_json::to_string(&names).unwrap(),
    ));
    for n in &names {
        entries.push((format!("profile/{n}"), "{}".into()));
    }
    let refs: Vec<(&str, &str)> = entries
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let mut k = kernel(&refs);
    k.state.reset_counts();
    let ps = k.profiles("alice");
    assert_eq!(ps.len(), 64);
    assert_eq!(ps[63].name, "p63");
    assert!(k.state.counts().gets <= 67, "{}", k.state.counts().gets);
}

/// S40-7 (stage D on watane's host): a path rule on an OpenAI profile was named twice, once by the
/// rule check and once by the refuse table; each field is named once.
#[test]
fn regress_s40_7_a_path_rule_on_openai_is_named_once() {
    let tidy =
        r#"{"backend":"openai","model":"gpt-5","tools":["Read","Edit"],"allow":["Read(/tmp/**)"]}"#;
    let mut k = kernel(&[("access/page", r#"["Tidy"]"#), ("profile/Tidy", tidy)]);
    let rs = k.check_named("page", "Tidy", "{}");
    assert_eq!(fields(&rs), ["tools", "allow"], "{rs:?}");
}

#[test]
fn regress_s165_openai_arg_rule_is_refused() {
    let mut k = kernel(&[
        ("access/page", r#"["p"]"#),
        (
            "profile/p",
            r#"{"backend":"openai","model":"gpt-5","allow":["skill"],"deny":["skill(secret)"]}"#,
        ),
    ]);
    assert_eq!(fields(&k.check_named("page", "p", "{}")), ["deny"]);
}

#[test]
fn adv_s165_openai_path_rule_on_tool_name_is_refused() {
    let mut k = kernel(&[
        ("access/page", r#"["p"]"#),
        (
            "profile/p",
            r#"{"backend":"openai","model":"gpt-5","deny":["notes_read(/secret/*)"]}"#,
        ),
    ]);
    assert_eq!(fields(&k.check_named("page", "p", "{}")), ["deny"]);
}

#[test]
fn adv_s165_openai_dotdot_rule_is_refused() {
    let mut k = kernel(&[
        ("access/page", r#"["p"]"#),
        (
            "profile/p",
            r#"{"backend":"openai","model":"gpt-5","allow":["notes_read(..)"]}"#,
        ),
    ]);
    assert_eq!(fields(&k.check_named("page", "p", "{}")), ["allow"]);
}
