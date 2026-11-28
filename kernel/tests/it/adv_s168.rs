use agent_kernel::active::Slots;
use agent_kernel::manage::OfferedProfile;
use agent_kernel::*;
use serde_json::{json, Value};

const IN: &str = r#"[{"type":"text","text":"hi"}]"#;
fn openai(x: &str) -> String {
    format!(r#"{{"backend":"openai","model":"gpt-5","system-prompt":{{"replace":"x"}}{x}}}"#)
}
fn last_ask(k: &Kernel<MemState, FakeHost>) -> String {
    k.host
        .calls
        .iter()
        .rev()
        .find_map(|c| match c {
            HostCall::AccessAsk { stream, .. } => Some(stream.clone()),
            _ => None,
        })
        .unwrap()
}
fn ok(k: &mut Kernel<MemState, FakeHost>) {
    let s = last_ask(k);
    k.manage_on_item(&s, &json!({"type":"granted"}));
    k.manage_on_ended(&s);
}
#[test]
fn adv_s168_stale_delegate_still_admitted_by_task() {
    let mut st = MemState::new();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let helper = openai("");
    let mut k = Kernel::new(
        st,
        FakeHost {
            generation: 1,
            now: 1_000_000,
            connection_id: Some("c1".into()),
            offered_profiles: vec![OfferedProfile {
                name: "helper".into(),
                description: "d".into(),
                profile: helper.clone(),
            }],
            ..FakeHost::default()
        },
    );
    k.profiles("local/c");
    k.manage_open("page").unwrap();
    let s = last_ask(&k);
    k.manage_on_item(&s, &json!({"type":"granted"}));
    k.manage_adopt(
        "page",
        "helper",
        &access::digest(helper.as_bytes()),
        "helper",
    )
    .unwrap();
    ok(&mut k);
    k.manage_put_profile(
        "page",
        "root",
        &openai(r#", "delegates":["helper"],"allow":["task"]"#),
    )
    .unwrap();
    ok(&mut k);
    k.manage_allow("page", "local/c", "root").unwrap();
    ok(&mut k);
    k.host.offered_profiles[0].profile = openai(r#", "max-turns":5"#);
    k.manage_offers("page", 0, 16).unwrap();
    assert!(access::index(&mut k.state)
        .iter()
        .any(|r| r.name == "helper" && r.stale));
    let id = k.start("local/c", "k", "root", "{}", IN).unwrap().value;
    let lp: Value = serde_json::from_slice(&k.state.get(&format!("loop/{id}")).unwrap()).unwrap();
    let stream = lp["stream"].as_str().unwrap().to_string();
    let head = json!({"status":200,"headers":{"content-type":"text/event-stream"}});
    let call = json!({"type":"function_call","id":"fc","call_id":"c1","name":"task","arguments": json!({"profile":"helper","input":"x"}).to_string()});
    let done = json!({"type":"response.completed","response":{"id":"r","output":[call],"usage":{"input_tokens":1,"output_tokens":1}}});
    k.provider_lines(
        &stream,
        vec![
            (1, head.to_string().into_bytes()),
            (2, done.to_string().into_bytes()),
        ],
    )
    .unwrap();
    assert!(
        Record::load(&mut k.state, &id).unwrap().children.is_empty(),
        "stale delegate admitted"
    );
}
