//! The ordinary sessions export runs under the identity supplied by Watane's Harness.
use serde_json::json;
use std::path::Path;
use watane_host::{Fakes, Harness, Outcome};

fn arming_requests(outcome: &Outcome) -> usize {
    outcome
        .requests
        .iter()
        .filter(|r| {
            r.function == "timers.after" && r.args == json!({"id":"page-origin","delay-ms":0})
        })
        .count()
}

fn main() {
    let folder = std::env::args_os().nth(1).expect("built probe mod");
    let harness = Harness::open(Path::new(&folder), Fakes::new()).unwrap();
    let call = |name| harness.call(name, json!({})).unwrap();

    call("timer-probe.seed-drain-owed");
    assert_eq!(call("timer-probe.drain-owed").value, json!(true));
    let client = harness
        .deliver("events.client-profiles", json!({}))
        .unwrap();
    assert_eq!(arming_requests(&client), 0);
    assert_eq!(call("timer-probe.drain-owed").value, json!(false));

    call("timer-probe.seed-drain-owed");
    let failure = harness
        .call(
            "sessions.rename",
            json!({"session":"missing","title":"unused"}),
        )
        .unwrap();
    assert!(failure.value.get("err").is_some());
    assert_eq!(arming_requests(&failure), 0);
    assert_eq!(call("timer-probe.drain-owed").value, json!(true));

    let read = |session| {
        harness
            .call(
                "sessions.read",
                json!({"session":session,"after":0,"before":null,"max":1}),
            )
            .unwrap()
    };
    assert!(read("missing").value.get("err").is_some());
    assert_eq!(call("timer-probe.drain-owed").value, json!(true));
    let success = read("read-probe");
    assert!(success.value.get("ok").is_some());
    assert_eq!(arming_requests(&success), 1);
    assert_eq!(call("timer-probe.drain-owed").value, json!(false));

    call("timer-probe.seed-drain-owed");
    // Profiles itself is read-only, so its drain and timer request come from run_client.
    assert_eq!(arming_requests(&call("sessions.profiles")), 1);
    assert_eq!(call("timer-probe.drain-owed").value, json!(false));
}
