use agent_kernel::{KernelError, Refusal};

#[test]
fn ac2_twelve_codes_and_messages_name_the_cause() {
    let r = Refusal {
        field: "effort".into(),
        backend: "openai".into(),
        model: "gpt-x".into(),
        reason: "nope".into(),
    };
    let all = [
        (KernelError::NotOwner, "not-owner", "own"),
        (
            KernelError::UnknownSession("s-42".into()),
            "unknown-session",
            "s-42",
        ),
        (KernelError::Refused(vec![r]), "refused", "effort"),
        (KernelError::QueueFull, "queue-full", "queue"),
        (KernelError::Quota("x".into()), "quota", "quota: x"),
        (KernelError::Paused(1789), "paused", "1789"),
        (
            KernelError::Backend("socket closed".into()),
            "backend",
            "socket closed",
        ),
        (KernelError::UnknownInput(77), "unknown-input", "77"),
        (
            KernelError::UnknownAllow("a-9".into()),
            "unknown-allow",
            "a-9",
        ),
        (KernelError::Behind(310), "behind", "310"),
        (KernelError::TooLarge(262144), "too-large", "262144"),
        (
            KernelError::Busy("1024 active sessions".into()),
            "busy",
            "1024 active",
        ),
    ];
    assert_eq!(all.len(), 12);
    for (e, code, data) in all {
        assert_eq!(e.code(), code);
        assert!(e.to_string().contains(data), "{e}");
        let _: &dyn std::error::Error = &e;
    }
}

#[test]
fn ac5_a_refusal_without_backend_and_model_prints_field_and_reason() {
    let bare = |b: &str, m: &str| Refusal {
        field: "profile".into(),
        backend: b.into(),
        model: m.into(),
        reason: "no such profile".into(),
    };
    let e = KernelError::Refused(vec![bare("", ""), bare("claude", "")]);
    assert_eq!(
        e.to_string(),
        "refused: 2 field(s) refused: profile: no such profile; profile (claude, ): no such profile"
    );
}
