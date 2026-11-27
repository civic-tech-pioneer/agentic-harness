//! The watane 19.3/19.7 consumer seam. Drive the real adapter with host stream items and outcomes.
use agent_kernel::{FakeHost, HostCall, Kernel, MemState, Record, SinkOutcome, State, Stop, Turn};
use serde_json::json;

const FOLDER: &str = "/home/alice/notes";
const WHAT: &str = "workspace:/home/alice/notes";

#[test]
fn ac1_ac2_ac3_workspace_stream_lifecycle() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let rec = Record::load(&mut k.state, &id).unwrap();
    let stream = rec.streams[0].clone();
    assert!(rec.folder.is_none());
    assert!(k.host.calls.iter().any(|c| matches!(c,
        HostCall::StartProvider { launch, .. }
        if serde_json::from_str::<serde_json::Value>(launch).unwrap()["workspace"] == FOLDER
    )));

    let pending = json!({"type": "consent-pending", "what": WHAT});
    let call = k
        .provider_lines(&stream, vec![(1, pending.to_string().into_bytes())])
        .unwrap();
    assert_eq!(call.value, SinkOutcome::Taken);
    assert!(matches!(
        Record::load(&mut k.state, &id).unwrap().turn,
        Turn::Running { activity: None, .. }
    ));
    k.attach("page", &id, 0).unwrap();
    assert!(k.host.calls.iter().any(|c| matches!(c,
        HostCall::Push { value, .. }
        if value["_meta"]["watane"]["state"]["activity"]["what"] == WHAT
    )));

    let initialized = json!({"type": "initialized", "workspace": FOLDER});
    let call = k
        .provider_lines(&stream, vec![(2, initialized.to_string().into_bytes())])
        .unwrap();
    assert!(call
        .pushes
        .iter()
        .any(|p| p.value["state"]["state"] == "running"
            && p.value["state"].get("activity").is_none()));
    assert_eq!(
        k.list("page", None, 5).unwrap()[0].workspace.as_deref(),
        Some(FOLDER)
    );

    let response = json!({
        "type": "assistant", "uuid": "answer-1", "session_id": "provider-1",
        "message": {"id": "message-1", "type": "message", "role": "assistant",
            "model": "fake", "content": [{"type": "text", "text": "I read the notes."}]}
    });
    k.provider_lines(&stream, vec![(3, response.to_string().into_bytes())])
        .unwrap();
    k.provider_ended_with(&stream, Some("gone".into())).unwrap();
    let rec = Record::load(&mut k.state, &id).unwrap();
    assert!(!rec.streams.contains(&stream));
    assert!(
        matches!(rec.turn, Turn::Idle { stop: Stop::Failed { cause: Some(cause), .. }, .. } if cause == "gone")
    );
    let entries = k.read("page", &id, 0, None, 20).unwrap().entries;
    assert!(entries.iter().any(|e| e.contains("hello")));
    assert!(entries.iter().any(|e| e.contains("I read the notes.")));
    let entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect();
    let said = |e: &&serde_json::Value| e.to_string().contains("I read the notes.");
    let agent: Vec<_> = entries.iter().filter(said).collect();
    assert_eq!(agent.len(), 1, "{entries:?}");
    assert_eq!(agent[0]["kind"], "message");
    assert_eq!(agent[0]["body"]["role"], "agent");
    assert_eq!(agent[0]["body"]["content"][0]["text"], "I read the notes.");
    assert!(
        entries.iter().all(|e| e["kind"] != "unknown"),
        "{entries:?}"
    );

    let next = k
        .start_in_folder("page", "next", "scratch", "{}", "again", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &next).unwrap().streams[0].clone();
    k.attach("page", &next, 0).unwrap();
    let call = k
        .provider_lines(&stream, vec![(1, pending.to_string().into_bytes())])
        .unwrap();
    assert!(call
        .pushes
        .iter()
        .any(|p| p.value["state"]["activity"]["what"] == WHAT));
    let refused = format!("consent-refused({WHAT})");
    k.provider_ended_with(&stream, Some(refused.clone()))
        .unwrap();
    let rec = Record::load(&mut k.state, &next).unwrap();
    assert_eq!(rec.streams, vec![rec.live.unwrap()]);
    assert!(rec.folder.is_none());
    assert!(
        matches!(rec.turn, Turn::Idle { stop: Stop::Failed { cause: Some(cause), .. }, .. } if cause == refused)
    );
}

#[test]
fn ac1_host_rejects_a_forbidden_path_before_opening_a_session() {
    let cause = "bad-launch: workspace: protected folder";
    let host = FakeHost {
        start_error: Some(cause.into()),
        ..FakeHost::default()
    };
    let mut k = Kernel::new(MemState::default(), host);
    let error = k
        .start_in_folder(
            "page",
            "first",
            "scratch",
            "{}",
            "hello",
            "/home/alice/.ssh",
        )
        .unwrap_err();
    assert!(matches!(error, agent_kernel::KernelError::Backend(message) if message == cause));
    assert!(k.list("page", None, 5).unwrap().is_empty());
}

#[test]
fn ac2_wrapped_stdout_cannot_start_a_consent_wait() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let stdout = json!({"type": "consent-pending", "what": WHAT});
    let wrapped = json!({"type": "wrapped", "line": stdout.to_string()});
    k.provider_lines(&stream, vec![(1, wrapped.to_string().into_bytes())])
        .unwrap();
    k.attach("page", &id, 0).unwrap();
    assert!(!k.host.calls.iter().any(|c| matches!(c,
        HostCall::Push { value, .. }
        if value["_meta"]["watane"]["state"]["activity"]["kind"] == "consent"
    )));
}

#[test]
fn ac4_picked_folder_never_offers_recursive_write_keys() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    k.attach("page", &id, 0).unwrap();
    let path = format!("{FOLDER}/entry.txt");
    let lines = [
        json!({"type": "initialized", "workspace": FOLDER}),
        json!({"type": "system", "subtype": "init", "cwd": FOLDER}),
        json!({"type": "ask", "id": "write", "tool": "Write", "path": path,
            "input": {"file_path": path, "content": "notes"}, "tool_use_id": "tu1", "expires-at": 9000}),
    ];
    k.provider_lines(
        &stream,
        lines
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i as u64 + 1, v.to_string().into_bytes()))
            .collect(),
    )
    .unwrap();
    let page = k.read("page", &id, 0, None, 20).unwrap();
    let ask = page
        .entries
        .iter()
        .map(|e| serde_json::from_str::<serde_json::Value>(e).unwrap())
        .find(|e| e["kind"] == "ask")
        .unwrap();
    assert!(ask["body"].get("reach").is_none());
}

fn launch_workspaces(k: &Kernel<MemState, FakeHost>) -> Vec<serde_json::Value> {
    k.host
        .calls
        .iter()
        .filter_map(|c| match c {
            HostCall::StartProvider { launch, .. } => Some(
                serde_json::from_str::<serde_json::Value>(launch).unwrap()["workspace"].clone(),
            ),
            _ => None,
        })
        .collect()
}

#[test]
fn ac2_attach_after_initialized_shows_no_consent() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let pending = json!({"type": "consent-pending", "what": WHAT});
    let initialized = json!({"type": "initialized", "workspace": FOLDER});
    k.provider_lines(
        &stream,
        vec![
            (1, pending.to_string().into_bytes()),
            (2, initialized.to_string().into_bytes()),
        ],
    )
    .unwrap();
    k.host.calls.clear();
    k.attach("page", &id, 0).unwrap();
    assert!(!k.host.calls.iter().any(|c| matches!(c,
        HostCall::Push { value, .. }
        if value["_meta"]["watane"]["state"]["activity"]["kind"] == "consent")));
}

#[test]
fn ac3_a_say_after_refusal_asks_for_the_picked_folder_again() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    k.provider_ended_with(&stream, Some(format!("consent-refused({WHAT})")))
        .unwrap();
    k.host.calls.clear();
    k.say("page", &id, "s1", "again").unwrap();
    assert_eq!(launch_workspaces(&k), vec![json!(FOLDER)]);
}

#[test]
fn ac1_scratch_session_lists_no_workspace_and_bad_folders_are_refused() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start("page", "s", "scratch", "{}", "hello")
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let ws =
        json!({"type": "initialized", "workspace": "scratch:0123456789abcdef0123456789abcdef"});
    k.provider_lines(&stream, vec![(1, ws.to_string().into_bytes())])
        .unwrap();
    assert_eq!(k.list("page", None, 5).unwrap()[0].workspace, None);
    for bad in [
        "notes",
        "scratch",
        "scratch:0123456789abcdef0123456789abcdef",
    ] {
        k.host.calls.clear();
        let e = k
            .start_in_folder("page", bad, "scratch", "{}", "hello", bad)
            .unwrap_err();
        assert!(
            matches!(e, agent_kernel::KernelError::Refused(r) if r.len() == 1 && r[0].field == "folder"),
            "{bad}"
        );
        assert!(launch_workspaces(&k).is_empty());
    }

    k.state
        .put(
            "profile/o",
            br#"{"backend":"openai","model":"gpt-5"}"#.to_vec(),
        )
        .unwrap();
    k.state.put("access/page", br#"["o"]"#.to_vec()).unwrap();
    let e = k
        .start_in_folder("page", "openai", "o", "{}", "hello", FOLDER)
        .unwrap_err();
    assert!(
        matches!(e, agent_kernel::KernelError::Refused(r) if r.len() == 1 && r[0].field == "folder")
    );
    assert!(launch_workspaces(&k).is_empty());
}

#[test]
fn ac1_list_shows_the_picked_path_then_the_canonical_one() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let picked = "/home/alice/link-to-notes";
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", picked)
        .unwrap()
        .value;
    assert_eq!(
        k.list("page", None, 5).unwrap()[0].workspace.as_deref(),
        Some(picked)
    );
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let initialized = json!({"type": "initialized", "workspace": FOLDER});
    k.provider_lines(&stream, vec![(1, initialized.to_string().into_bytes())])
        .unwrap();
    assert_eq!(
        k.list("page", None, 5).unwrap()[0].workspace.as_deref(),
        Some(FOLDER)
    );
}

#[test]
fn picked_folder_rejects_control_characters_and_oversize_paths() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    for path in ["/notes\nnew", "/notes\0new"] {
        let err = k
            .start_in_folder("page", "first", "scratch", "{}", "hello", path)
            .unwrap_err();
        assert!(
            matches!(err, agent_kernel::KernelError::Refused(r) if r.len() == 1 && r[0].field == "folder")
        );
    }
    let path = format!("/{}", "a".repeat(4096));
    let err = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", &path)
        .unwrap_err();
    assert!(matches!(err, agent_kernel::KernelError::TooLarge(4096)));
    assert!(launch_workspaces(&k).is_empty());
    assert!(k
        .start_in_folder(
            "page",
            "fits",
            "scratch",
            "{}",
            "hello",
            &format!("/{}", "a".repeat(4095))
        )
        .is_ok());
}

#[test]
fn relaunched_stream_shows_a_new_consent_wait() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let initialized = json!({"type": "initialized", "workspace": FOLDER});
    k.provider_lines(&stream, vec![(1, initialized.to_string().into_bytes())])
        .unwrap();
    // The human withdraws the grant: host 19.7 ends the dependent stream with `gone`.
    k.provider_ended_with(&stream, Some("gone".into())).unwrap();
    k.say("page", &id, "k2", "again").unwrap();
    let rec = Record::load(&mut k.state, &id).unwrap();
    let next = rec
        .streams
        .iter()
        .find(|s| **s != stream)
        .expect("a new provider stream")
        .clone();
    k.attach("page", &id, 0).unwrap();
    let pending = json!({"type": "consent-pending", "what": WHAT});
    let call = k
        .provider_lines(&next, vec![(1, pending.to_string().into_bytes())])
        .unwrap();
    assert!(call
        .pushes
        .iter()
        .any(|p| p.value["state"]["activity"]["what"] == WHAT));
}

#[test]
fn interrupt_uses_the_current_launch_state() {
    for initialized in [false, true] {
        let mut k = Kernel::new(MemState::default(), FakeHost::default());
        let id = k
            .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
            .unwrap()
            .value;
        let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
        if initialized {
            let item = json!({"type": "initialized", "workspace": FOLDER});
            k.provider_lines(&stream, vec![(1, item.to_string().into_bytes())])
                .unwrap();
        }

        k.host.calls.clear();
        k.interrupt("page", &id, None).unwrap();
        let record = Record::load(&mut k.state, &id).unwrap();
        assert_eq!(record.streams.contains(&stream), initialized);
        assert!(matches!(
            record.turn,
            Turn::Idle {
                stop: Stop::Interrupted,
                ..
            }
        ));
        assert!(k.host.calls.iter().any(|call| match call {
            HostCall::Request(agent_kernel::Request::Interrupt { stream: sent, .. }) => {
                initialized && *sent == stream
            }
            HostCall::Request(agent_kernel::Request::Cancel { stream: sent, .. }) => {
                !initialized && *sent == stream
            }
            _ => false,
        }));
    }
}

#[test]
fn interrupt_with_replacement_relaunches_while_waiting_for_consent() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let original = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let pending = json!({"type": "consent-pending", "what": WHAT});
    k.provider_lines(&original, vec![(1, pending.to_string().into_bytes())])
        .unwrap();

    k.host.calls.clear();
    k.interrupt("page", &id, Some("instead")).unwrap();
    let rec = Record::load(&mut k.state, &id).unwrap();
    assert_eq!(rec.streams.len(), 1);
    assert_ne!(rec.streams[0], original);
    assert!(rec.turn.is_active());
    assert!(k.host.calls.iter().any(|call| matches!(call,
        HostCall::StartProvider { input, .. } if input == "instead"
    )));
    assert!(k.host.calls.iter().any(|call| matches!(call,
        HostCall::Request(agent_kernel::Request::Cancel { stream, .. }) if *stream == original
    )));
    assert!(!k.host.calls.iter().any(|call| matches!(
        call,
        HostCall::Request(
            agent_kernel::Request::Interrupt { .. } | agent_kernel::Request::Turn { .. }
        )
    )));
    assert_eq!(agent_kernel::active::Slots::load(&mut k.state).provider, 1);

    k.provider_ended(&original).unwrap();
    assert_eq!(Record::load(&mut k.state, &id).unwrap(), rec);
}

#[test]
fn ac1_native_initialization_advances_the_cursor_and_stop_interrupts() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
        .unwrap()
        .value;
    let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
    let key = format!("stream/{stream}");
    k.provider_item(
        &stream,
        agent_kernel::ProviderEvent::Consent(Some(WHAT.into())),
    )
    .unwrap();
    k.provider_item(
        &stream,
        agent_kernel::ProviderEvent::Workspace(FOLDER.into()),
    )
    .unwrap();

    let stored: serde_json::Value = serde_json::from_slice(&k.state.get(&key).unwrap()).unwrap();
    assert_eq!(stored["last"], 2);
    assert_eq!(stored["launching"], false);
    assert!(stored.get("consent").is_none());
    k.interrupt("page", &id, None).unwrap();
    assert!(Record::load(&mut k.state, &id)
        .unwrap()
        .streams
        .contains(&stream));
}

#[test]
fn ac2_stale_consent_does_not_cancel_an_initialized_or_legacy_stream() {
    for (marker, last) in [(None, 0), (None, 1), (Some(false), 0), (Some(false), 1)] {
        let mut k = Kernel::new(MemState::default(), FakeHost::default());
        let id = k
            .start_in_folder("page", "first", "scratch", "{}", "hello", FOLDER)
            .unwrap()
            .value;
        let stream = Record::load(&mut k.state, &id).unwrap().streams[0].clone();
        let mut stored = json!({"session": id, "kind": "provider", "last": last, "consent": WHAT});
        if let Some(marker) = marker {
            stored["launching"] = json!(marker);
        }
        k.state
            .put(
                &format!("stream/{stream}"),
                serde_json::to_vec(&stored).unwrap(),
            )
            .unwrap();
        k.host.calls.clear();

        k.interrupt("page", &id, None).unwrap();

        assert!(k.host.calls.iter().any(|call| matches!(call,
            HostCall::Request(agent_kernel::Request::Interrupt { stream: sent, .. }) if sent == &stream
        )));
        assert!(Record::load(&mut k.state, &id)
            .unwrap()
            .streams
            .contains(&stream));
    }
}

#[test]
fn ac3_native_read_leaves_owed_work_for_the_client_wrapper() {
    let mut k = Kernel::new(MemState::default(), FakeHost::default());
    let id = k
        .start("page", "first", "scratch", "{}", "hello")
        .unwrap()
        .value;
    let mut slots = agent_kernel::active::Slots::load(&mut k.state);
    slots.drain_owed = true;
    slots.save(&mut k.state).unwrap();

    assert!(k.read("outsider", &id, 0, None, 1).is_err());
    k.read("page", &id, 0, None, 1).unwrap();
    assert!(agent_kernel::active::Slots::load(&mut k.state).drain_owed);
    k.drain_owed();
    assert!(!agent_kernel::active::Slots::load(&mut k.state).drain_owed);
}
