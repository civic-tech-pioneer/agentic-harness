use agent_kernel::{KernelError, Refusal};

#[test]
fn ac2_ten_codes_and_messages_name_the_cause() {
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
        (KernelError::Quota, "quota", "quota"),
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
    ];
    assert_eq!(all.len(), 10);
    for (e, code, data) in all {
        assert_eq!(e.code(), code);
        assert!(e.to_string().contains(data), "{e}");
        let _: &dyn std::error::Error = &e;
    }
}
