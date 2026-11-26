//! S134-3: stage F profile rules (9.2, 11) and the admission check of 9.7 (steps 2 to 5).

use agent_kernel::delegation::{admit, live_cap};
use agent_kernel::{check, parse_profile, Backend, Profile, Refusal};
use serde_json::{json, Value};

fn fields(p: Value, o: Value, b: Backend) -> Vec<String> {
    check(&p.to_string(), &o.to_string(), b, "m", None)
        .into_iter()
        .map(|r| r.field)
        .collect()
}

fn merge(mut base: Value, extra: Value) -> Value {
    for (k, v) in extra.as_object().unwrap() {
        if v.is_null() {
            base.as_object_mut().unwrap().remove(k);
        } else {
            base[k] = v.clone();
        }
    }
    base
}

fn oa(extra: Value) -> Value {
    merge(json!({"backend": "openai", "model": "m"}), extra)
}

#[test]
fn ac1_claude_refuses_the_stage_f_fields() {
    for (k, v) in [
        ("delegates", json!(["x"])),
        ("subagent-limits", json!({})),
        ("max-tokens", json!(5)),
        ("max-tool-errors", json!(3)),
    ] {
        let rs = check(&json!({k: v}).to_string(), "{}", Backend::Claude, "m", None);
        assert_eq!(rs.len(), 1, "{k}: {rs:?}");
        assert_eq!(
            (rs[0].field.as_str(), rs[0].backend.as_str()),
            (k, "claude")
        );
    }
    assert_eq!(
        fields(json!({}), json!({"max-tokens": 5}), Backend::Claude),
        ["max-tokens"]
    );
}

#[test]
fn ac2_openai_subagent_limits_by_key() {
    let f = |v: Value| {
        fields(
            oa(json!({"subagent-limits": v})),
            json!({}),
            Backend::Openai,
        )
    };
    assert!(f(json!({"live": 2})).is_empty());
    assert!(f(json!({"live": 20, "depth": 1})).is_empty());
    assert_eq!(f(json!({"live": 0})), ["subagent-limits.live"]);
    assert_eq!(f(json!({"live": 21})), ["subagent-limits.live"]);
    assert_eq!(f(json!({"live": "2"})), ["subagent-limits.live"]);
    assert_eq!(f(json!({"depth": 2})), ["subagent-limits.depth"]);
    assert_eq!(f(json!({"depth": 0})), ["subagent-limits.depth"]);
    assert_eq!(f(json!({"wide": 1})), ["subagent-limits.wide"]);
    assert!(fields(oa(json!({"delegates": ["a"]})), json!({}), Backend::Openai).is_empty());
}

fn root(extra: Value) -> Profile {
    let p = merge(
        oa(json!({"delegates": ["kid"], "mode": "manual", "ask": true,
            "allow": ["Tool(a)", "Tool(x1)"], "deny": ["Tool(d)"], "toolsets": ["t1", "t2"],
            "max-turns": 10, "max-tool-errors": 5, "max-tokens": 1000, "ask-expiry": 60,
            "persist": true})),
        extra,
    );
    let (r, ps) = parse_profile(&p.to_string());
    assert!(ps.is_empty(), "{ps:?}");
    r
}

fn kid(extra: Value) -> String {
    merge(
        oa(json!({"mode": "manual", "ask": true, "allow": ["Tool(a)"],
            "deny": ["Tool(d)", "Tool(e)"], "toolsets": ["t1"],
            "max-turns": 10, "max-tool-errors": 5, "max-tokens": 1000, "ask-expiry": 60,
            "persist": true})),
        extra,
    )
    .to_string()
}

fn run(r: &Profile, child: String) -> Result<Profile, Vec<Refusal>> {
    admit(r, "m", false, 0, "kid", |_| Some(child))
}

fn fail(r: &Profile, child: String) -> Vec<String> {
    let rs = run(r, child).expect_err("must be refused");
    for x in &rs {
        assert_eq!((x.backend.as_str(), x.model.as_str()), ("openai", "m"));
    }
    rs.into_iter().map(|x| x.field).collect()
}

#[test]
fn ac3_a_child_caller_gets_one_depth_refusal_and_no_load() {
    let r = root(json!({}));
    let rs = admit(&r, "m", true, 0, "kid", |_| panic!("loaded")).unwrap_err();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "subagent-limits.depth");
}

#[test]
fn ac4_membership_and_missing_profile() {
    let r = root(json!({}));
    let rs = admit(&r, "m", false, 0, "other", |_| panic!("loaded")).unwrap_err();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "profile");
    let rs = admit(&r, "m", false, 0, "kid", |_| None).unwrap_err();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "profile");
}

#[test]
fn ac5_live_cap() {
    assert_eq!(live_cap(&root(json!({}))), 4);
    assert_eq!(live_cap(&root(json!({"subagent-limits": {"live": 2}}))), 2);
    let r = root(json!({}));
    let rs = admit(&r, "m", false, 4, "kid", |_| Some(kid(json!({})))).unwrap_err();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "subagent-limits.live");
    assert!(admit(&r, "m", false, 3, "kid", |_| Some(kid(json!({})))).is_ok());
    let r2 = root(json!({"subagent-limits": {"live": 1}}));
    assert!(admit(&r2, "m", false, 1, "kid", |_| Some(kid(json!({})))).is_err());
}

/// S134 r1 F1.6: step 3 (membership and load) comes before the live cap of step 4, so a named
/// but missing delegate at the cap is refused `profile`.
#[test]
fn r1_f1_6_a_missing_delegate_at_the_cap_is_refused_on_profile() {
    let r = root(json!({}));
    let rs = admit(&r, "m", false, 4, "kid", |_| None).unwrap_err();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].field, "profile");
}

#[test]
fn ac6_each_row_gives_its_field_only() {
    let r = root(json!({}));
    assert!(fail(&r, kid(json!({"backend": "claude"}))).contains(&"backend".to_string()));
    assert_eq!(fail(&r, kid(json!({"model": "n"}))), ["model"]);
    assert_eq!(fail(&r, kid(json!({"model": null}))), ["model"]);
    assert_eq!(
        fail(&r, kid(json!({"toolsets": ["t1", "t3"]}))),
        ["toolsets"]
    );
    assert_eq!(fail(&r, kid(json!({"tools": ["Read"]}))), ["tools"]);
    assert_eq!(fail(&r, kid(json!({"workspace": "w"}))), ["workspace"]);
    assert_eq!(
        fail(&r, kid(json!({"extra-workspaces": ["w"]}))),
        ["extra-workspaces"]
    );
    assert_eq!(fail(&r, kid(json!({"mode": "dont-ask"}))), ["mode"]);
    assert_eq!(fail(&r, kid(json!({"ask": false}))), ["ask"]);
    assert_eq!(
        fail(&r, kid(json!({"pre-tool-check": {"timeout-s": 5}}))),
        ["pre-tool-check"]
    );
    assert_eq!(
        fail(&r, kid(json!({"allow": ["Tool(a)", "Tool(zz)"]}))),
        ["allow"]
    );
    assert_eq!(fail(&r, kid(json!({"allow": ["Tool(x*)"]}))), ["allow"]);
    assert_eq!(fail(&r, kid(json!({"deny": ["Tool(e)"]}))), ["deny"]);
    assert_eq!(fail(&r, kid(json!({"max-turns": 11}))), ["max-turns"]);
    assert_eq!(fail(&r, kid(json!({"max-turns": null}))), ["max-turns"]);
    assert_eq!(
        fail(&r, kid(json!({"max-tool-errors": 6}))),
        ["max-tool-errors"]
    );
    assert_eq!(
        fail(&r, kid(json!({"max-tool-errors": null}))),
        ["max-tool-errors"]
    );
    assert_eq!(fail(&r, kid(json!({"max-tokens": 1001}))), ["max-tokens"]);
    assert_eq!(fail(&r, kid(json!({"max-tokens": null}))), ["max-tokens"]);
    assert_eq!(fail(&r, kid(json!({"ask-expiry": 61}))), ["ask-expiry"]);
    assert_eq!(fail(&r, kid(json!({"ask-expiry": null}))), ["ask-expiry"]);
    assert_eq!(fail(&r, kid(json!({"delegates": ["z"]}))), ["delegates"]);
    assert_eq!(
        fail(&r, kid(json!({"subagent-limits": {"live": 1}}))),
        ["subagent-limits"]
    );
    let nop = root(json!({"persist": false}));
    assert_eq!(fail(&nop, kid(json!({"persist": true}))), ["persist"]);
    assert_eq!(fail(&nop, kid(json!({"persist": null}))), ["persist"]);
    assert!(run(&nop, kid(json!({"persist": false}))).is_ok());
    assert!(run(&r, kid(json!({"persist": false}))).is_ok());
}

#[test]
fn ac6_unbounded_root_caps_pass_and_several_failures_come_together() {
    let r = root(
        json!({"max-turns": null, "max-tool-errors": null, "max-tokens": null,
        "ask-expiry": null}),
    );
    assert!(run(
        &r,
        kid(json!({"max-turns": 999, "max-tool-errors": 99, "max-tokens": 9, "ask-expiry": 7}))
    )
    .is_ok());
    let r = root(json!({}));
    let mut f = fail(
        &r,
        kid(json!({"model": "n", "mode": "dont-ask", "max-turns": 99, "deny": []})),
    );
    f.sort();
    assert_eq!(f, ["deny", "max-turns", "mode", "model"]);
}

#[test]
fn ac7_equal_and_narrower_are_admitted_and_returned() {
    let r = root(json!({}));
    let c = run(&r, kid(json!({}))).unwrap();
    assert_eq!(c.allow, ["Tool(a)"]);
    assert_eq!(c.max_tokens, Some(1000));
    let narrow = kid(
        json!({"allow": [], "toolsets": [], "max-turns": 1, "max-tool-errors": 1,
        "max-tokens": 1, "ask-expiry": 0, "deny": ["Tool(d)", "Tool(q)"]}),
    );
    assert!(run(&r, narrow).is_ok());
}

#[test]
fn ac8_the_roots_effective_profile_is_the_compiled_one() {
    use agent_kernel::{apply_overlay, parse_overlay};
    let r = root(json!({}));
    let (o, _) = parse_overlay(&json!({"max-turns": 3, "deny+": ["Tool(f)"]}).to_string());
    let (compiled, ps) = apply_overlay(&r, &o);
    assert!(ps.is_empty());
    assert_eq!(fail(&compiled, kid(json!({}))), ["deny", "max-turns"]);
    assert!(run(
        &compiled,
        kid(json!({"max-turns": 3, "deny": ["Tool(d)", "Tool(f)"]}))
    )
    .is_ok());
}

#[test]
fn a_candidate_that_does_not_parse_is_refused() {
    let r = root(json!({}));
    assert!(run(&r, "not json".into()).is_err());
    assert!(run(&r, "[]".into()).is_err());
}

/// S134 r1 F1.3: an absent `ask-expiry` is the 600-second default (9.6), not unbounded: a child
/// may not set a longer window under a root that sets none, and may set 600 or less.
#[test]
fn r1_f1_3_an_absent_root_ask_expiry_is_the_600_second_default() {
    let (r, ps) = parse_profile(
        &oa(
            json!({"delegates": ["kid"], "max-turns": 10, "max-tool-errors": 5,
            "max-tokens": 1000}),
        )
        .to_string(),
    );
    assert!(ps.is_empty(), "{ps:?}");
    let child = |v: Value| {
        oa(merge(
            json!({"max-turns": 10, "max-tool-errors": 5, "max-tokens": 1000}),
            v,
        ))
        .to_string()
    };
    let rs = admit(&r, "m", false, 0, "kid", |_| {
        Some(child(json!({"ask-expiry": 601})))
    })
    .unwrap_err();
    assert_eq!(
        rs.iter().map(|x| x.field.as_str()).collect::<Vec<_>>(),
        ["ask-expiry"]
    );
    assert!(admit(&r, "m", false, 0, "kid", |_| Some(child(
        json!({"ask-expiry": 600})
    )))
    .is_ok());
    assert!(admit(&r, "m", false, 0, "kid", |_| Some(child(json!({})))).is_ok());
}
