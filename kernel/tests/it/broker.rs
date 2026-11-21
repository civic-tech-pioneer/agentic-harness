//! S31-2: session allows and their reach (TECHNICAL 9.3, 9.4, 9.6).

use agent_kernel::active::Slots;
use agent_kernel::adapters::{Entry, Kind, Out};
use agent_kernel::broker::reach;
use agent_kernel::*;
use serde_json::{json, Value};

fn r(ws: Option<&str>, tool: &str, input: Value) -> Option<String> {
    reach("alice", ws, "ask", tool, &input)
}

#[test]
fn ac1_reach_of_file_tools() {
    let ws = Some("/w");
    let edit = |p: Value| r(ws, "Edit", json!({"file_path": p}));
    assert_eq!(edit(json!("/w/a/b.rs")).as_deref(), Some("Edit(/w/**)"));
    for bad in ["/w2/x", "/w/../x", "/w/./x", "/w//x", "w/x", "/w/", "/w/a/"] {
        assert_eq!(edit(json!(bad)), None, "{bad}");
    }
    assert_eq!(edit(json!(5)), None);
    assert_eq!(r(None, "Edit", json!({"file_path": "/w/a"})), None);
    assert_eq!(
        r(ws, "NotebookEdit", json!({"notebook_path": "/w/n.ipynb"})).as_deref(),
        Some("NotebookEdit(/w/**)")
    );
    assert_eq!(
        r(ws, "Glob", json!({"path": "/w/src"})).as_deref(),
        Some("Glob(/w/**)")
    );
    assert_eq!(
        r(ws, "Grep", json!({"path": "/w/src"})).as_deref(),
        Some("Grep(/w/**)")
    );
    assert_eq!(r(ws, "Glob", json!({"pattern": "*"})), None);
    assert_eq!(r(ws, "Grep", json!({})), None);
    let p = json!({"file_path": "/w/a"});
    for bad_ws in ["w", "/w/", "/w(", "/w)", "/w\n"] {
        assert_eq!(r(Some(bad_ws), "Read", p.clone()), None, "{bad_ws}");
    }
}

#[test]
fn ac2_bash_reach_needs_only_the_plain_keys() {
    let bash = |i: Value| r(None, "Bash", i);
    for bad in [
        json!({"command":"ls","dangerouslyDisableSandbox":true}),
        json!({"command":"ls","dangerouslyDisableSandbox":false}),
        json!({"command":"ls","x":1}),
    ] {
        assert_eq!(bash(bad.clone()), None, "{bad}");
    }
    let ok = json!({"command":"ls","description":"d","timeout":5,"run_in_background":true});
    assert_eq!(bash(ok).as_deref(), Some("Bash(ls)"));
}

#[test]
fn ac1_reach_of_bash_and_other_tools() {
    let bash = |c: &str| r(None, "Bash", json!({"command": c}));
    assert_eq!(bash("npm test").as_deref(), Some("Bash(npm test)"));
    for bad in [
        "rm -rf /tmp/x; ls",
        "ls | wc",
        "python3 x.py",
        "npm run x",
        "curl a:b",
        "ls\nrm x",
        "",
        "ls *:*",
    ] {
        assert_eq!(bash(bad), None, "{bad:?}");
    }
    assert_eq!(r(None, "Bash", json!({"command": 5})), None);
    assert_eq!(r(None, "WebFetch", json!({})).as_deref(), Some("WebFetch"));
    assert_eq!(
        r(None, "mcp__watane__send", json!({})).as_deref(),
        Some("mcp__watane__send")
    );
    for none in ["AskUserQuestion", "ExitPlanMode", "Web Fetch", "Bash(ls)"] {
        assert_eq!(r(None, none, json!({})), None, "{none}");
    }
    assert_eq!(reach("alice", None, "check", "WebFetch", &json!({})), None);
    assert_eq!(reach("agent:x", None, "ask", "WebFetch", &json!({})), None);
}

type K = Kernel<MemState, FakeHost>;

fn kernel() -> K {
    let mut st = MemState::new();
    st.put(
        "profile/p",
        br#"{"mode":"manual","tools":["Read"],"max-turns":20}"#.to_vec(),
    )
    .unwrap();
    for who in ["alice", "bob"] {
        st.put(&format!("access/{who}"), br#"["p"]"#.to_vec())
            .unwrap();
    }
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let host = FakeHost {
        boot: 1,
        now: 1000,
        ..FakeHost::default()
    };
    Kernel::new(st, host)
}

fn started(k: &mut K) -> String {
    let id = k
        .start("alice", "k", "p", "{}", r#"[{"type":"text","text":"hi"}]"#)
        .unwrap()
        .value;
    k.attach("alice", &id, 0).unwrap();
    id
}

fn rec(k: &mut K, id: &str) -> Record {
    Record::load(&mut k.state, id).unwrap()
}

fn ask(id: &str, tool: &str, input: Value, reach: Option<&str>) -> ProviderEvent {
    let mut body = json!({"id": id, "kind": "ask", "toolCallId": id,
        "toolCall": {"toolCallId": id, "title": tool}, "input": input, "expires-at": 9000});
    if let Some(r) = reach {
        body["reach"] = json!(r);
    }
    ProviderEvent::Out(Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body,
        }),
        ..Out::default()
    })
}

fn bash(id: &str, cmd: &str) -> ProviderEvent {
    ask(id, "Bash", json!({"command": cmd}), None)
}

fn entries(k: &mut K, id: &str) -> Vec<Value> {
    k.read("alice", id, 0, None, 256)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn asked(k: &mut K, id: &str, ask_id: &str) -> Value {
    entries(k, id)
        .into_iter()
        .find(|e| e["kind"] == "ask" && e["body"]["id"] == ask_id)
        .unwrap()
}

fn answers(k: &K) -> Vec<Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::Request(Request::Answer { decision, .. }) => Some(decision.clone()),
            _ => None,
        })
        .collect()
}

const SESSION: &str = r#"{"allow":true,"for":"session"}"#;

#[test]
fn ac2_the_kernel_sets_the_reach_of_a_stored_ask() {
    let mut k = kernel();
    let id = started(&mut k);
    k.provider(&id, bash("a1", "npm test")).unwrap();
    k.provider(
        &id,
        ask("a2", "Bash", json!({"command": "ls | wc"}), Some("Bash")),
    )
    .unwrap();
    assert_eq!(asked(&mut k, &id, "a1")["body"]["reach"], "Bash(npm test)");
    assert!(asked(&mut k, &id, "a2")["body"].get("reach").is_none());
    // a reach the provider sends is replaced by the kernel's
    k.provider(
        &id,
        ask("a3", "Bash", json!({"command": "npm test"}), Some("Bash")),
    )
    .unwrap();
    assert_eq!(asked(&mut k, &id, "a3")["body"]["reach"], "Bash(npm test)");
}

fn allow_entries(k: &mut K, id: &str) -> Vec<Value> {
    entries(k, id)
        .into_iter()
        .filter(|e| e["kind"] == "allow")
        .collect()
}

#[test]
fn ac3_answer_for_session_adds_an_allow_and_sends_no_for() {
    let mut k = kernel();
    let id = started(&mut k);
    k.provider(&id, bash("a1", "npm test")).unwrap();
    k.provider(&id, bash("a2", "npm test")).unwrap();
    k.answer("alice", &id, "k1", "a1", SESSION).unwrap();
    let es = allow_entries(&mut k, &id);
    assert_eq!(es.len(), 1);
    let i = es[0]["i"].as_u64().unwrap().to_string();
    assert_eq!(
        es[0]["body"],
        json!({"id": i, "reach": "Bash(npm test)", "given": true, "by": "owner"})
    );
    assert_eq!(
        rec(&mut k, &id).allows,
        [Allow {
            id: i,
            reach: "Bash(npm test)".into()
        }]
    );
    assert_eq!(answers(&k), [json!({"allow": true})]);
    // the same reach again adds nothing and writes nothing
    k.answer("alice", &id, "k2", "a2", SESSION).unwrap();
    assert_eq!(allow_entries(&mut k, &id).len(), 1);
    assert_eq!(rec(&mut k, &id).allows.len(), 1);
    assert_eq!(
        answers(&k),
        [json!({"allow": true}), json!({"allow": true})]
    );
}

#[test]
fn ac4_take_back_removes_the_allow_and_writes_an_entry() {
    let mut k = kernel();
    let id = started(&mut k);
    k.provider(&id, bash("a1", "npm test")).unwrap();
    k.answer("alice", &id, "k1", "a1", SESSION).unwrap();
    let i = rec(&mut k, &id).allows[0].id.clone();
    let owner = match k.attach("alice", &id, 0).unwrap().value {
        Attached::Live { stream } => stream,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        k.take_back("bob", &id, &i).unwrap_err(),
        KernelError::NotOwner
    );
    assert!(matches!(
        k.take_back("alice", "zz", &i),
        Err(KernelError::UnknownSession(_))
    ));
    let n = k.host.calls.len();
    let c = k.take_back("alice", &id, &i).unwrap();
    assert!(rec(&mut k, &id).allows.is_empty());
    let es = allow_entries(&mut k, &id);
    assert_eq!(es.len(), 2);
    let body = json!({"id": i, "reach": "Bash(npm test)", "given": false, "by": "owner"});
    assert_eq!(es[1]["body"], body);
    assert_eq!(c.pushes[0].value["entry"]["body"], body);
    assert!(k.host.calls[n..].iter().any(|h| matches!(h,
        HostCall::Push { stream, value } if *stream == owner
            && value["_meta"]["watane"]["entry"]["body"] == body)));
    let before = entries(&mut k, &id).len();
    assert!(matches!(
        k.take_back("alice", &id, &i),
        Err(KernelError::UnknownAllow(_))
    ));
    assert_eq!(entries(&mut k, &id).len(), before);
}

#[test]
fn ac5_past_quota_take_back_still_works_and_answer_is_refused() {
    let mut k = kernel();
    let id = started(&mut k);
    k.provider(&id, bash("a1", "npm test")).unwrap();
    k.provider(&id, bash("a2", "npm run-all")).unwrap();
    k.provider(&id, bash("a3", "ls -la")).unwrap();
    k.answer("alice", &id, "k1", "a1", SESSION).unwrap();
    k.answer("alice", &id, "k3", "a3", SESSION).unwrap();
    let held: Vec<String> = rec(&mut k, &id)
        .allows
        .iter()
        .map(|a| a.id.clone())
        .collect();
    k.limits.owner_bytes = Usage::load(&mut k.state, "quota/alice").bytes;
    assert!(matches!(
        k.answer("alice", &id, "k2", "a2", SESSION),
        Err(KernelError::Quota(_))
    ));
    k.take_back("alice", &id, &held[0]).unwrap();
    assert_eq!(rec(&mut k, &id).allows.len(), 1);
    assert_eq!(allow_entries(&mut k, &id).len(), 3);
    // the account past its ceiling (no reserve left) does not refuse it either
    k.limits.account_bytes = Usage::load(&mut k.state, "quota").bytes;
    k.limits.reserve_bytes = 0;
    k.take_back("alice", &id, &held[1]).unwrap();
    assert!(rec(&mut k, &id).allows.is_empty());
    assert_eq!(allow_entries(&mut k, &id).len(), 4);
}

/// An `Edit` ask in a session whose workspace is `ws`, after `held` allows of `pad` bytes each.
fn edit_ask(ws: &str, held: usize, pad: usize) -> (K, String) {
    let mut k = kernel();
    let id = started(&mut k);
    let mut r = rec(&mut k, &id);
    for i in 0..held {
        let a = Allow {
            id: format!("{i:0>2}"),
            reach: String::new(),
        };
        let pad = pad - serde_json::to_string(&a).unwrap().len();
        r.add_allow(Allow {
            reach: "x".repeat(pad),
            ..a
        })
        .unwrap();
    }
    r.workspace = Some(ws.to_string());
    r.save(&mut k.state).unwrap();
    let input = json!({"file_path": format!("{ws}/a.rs")});
    k.provider(&id, ask("a1", "Edit", input, None)).unwrap();
    (k, id)
}

fn refused_whole(k: &mut K, id: &str) {
    let keys = ["session/s1", "entries/s1/0"];
    let snap: Vec<_> = keys.iter().map(|x| k.state.get(x)).collect();
    let sent = answers(k).len();
    assert_eq!(
        k.answer("alice", id, "k1", "a1", SESSION).unwrap_err(),
        KernelError::TooLarge(65_536)
    );
    let after: Vec<_> = keys.iter().map(|x| k.state.get(x)).collect();
    assert_eq!(after, snap);
    assert_eq!(answers(k).len(), sent);
    assert_eq!(rec(k, id).open.len(), 1);
    assert!(allow_entries(k, id).is_empty());
    assert!(k
        .answer("alice", id, "k2", "a1", r#"{"allow":true}"#)
        .is_ok());
}

#[test]
fn ac6_the_65th_allow_is_too_large() {
    let (mut k, id) = edit_ask("/w", 64, 40);
    refused_whole(&mut k, &id);
}

#[test]
fn ac6_an_allow_over_1_kib_is_too_large() {
    let (mut k, id) = edit_ask(&format!("/{}", "w".repeat(1100)), 0, 0);
    assert!(asked(&mut k, &id, "a1")["body"]["reach"].is_string());
    refused_whole(&mut k, &id);
}

#[test]
fn ac6_the_64th_allow_of_1_kib_fits_and_the_total_is_then_full() {
    // 63 allows of 1 KiB each hold 64 512 of the 65 536 bytes; the 64th of 1 KiB fills them
    let ws = format!("/{}", "w".repeat(1024 - 36 - 19));
    let (mut k, id) = edit_ask(&ws, 63, 1024);
    k.answer("alice", &id, "k1", "a1", SESSION).unwrap();
    let r = rec(&mut k, &id);
    assert_eq!(r.allows.len(), 64);
    let total: usize = r
        .allows
        .iter()
        .map(|a| serde_json::to_string(a).unwrap().len())
        .sum();
    assert!(total <= 65_536, "{total}");
}

#[test]
fn ac7_an_ask_with_no_reach_refuses_for_session() {
    let mut k = kernel();
    let id = started(&mut k);
    k.provider(&id, bash("a1", "ls | wc")).unwrap();
    let KernelError::Refused(rs) = k.answer("alice", &id, "k1", "a1", SESSION).unwrap_err() else {
        panic!()
    };
    assert_eq!(rs[0].field, "decision");
    assert!(answers(&k).is_empty());
}
