// Compiled only in the disposable Harness component.
impl crate::exports::mods::agent::timer_probe::Guest for Export {
    fn session_firing_keeps_pause() -> bool {
        let mut state = WitState;
        let id = "timer-probe";
        let record = agent_kernel::Record {
            id: id.into(),
            owner: "local/probe-owner".into(),
            boot: host_info::generation(),
            backend: "openai".into(),
            turn: agent_kernel::Turn::Paused {
                until: Some(u64::MAX),
            },
            deadlines: vec![agent_kernel::Deadline {
                what: "resume".into(),
                at: u64::MAX,
            }],
            ..Default::default()
        };
        record.save(&mut state).expect("seed timer probe");
        <Export as timer_sink::Guest>::fired(format!("s/{id}"));
        agent_kernel::Record::load(&mut state, id)
            .is_some_and(|rec| matches!(rec.turn, agent_kernel::Turn::Paused { .. }))
    }
}
