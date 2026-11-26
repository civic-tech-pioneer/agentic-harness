//! Slow S138 check. Only a real Watane scheduler and the kernel component exercise
//! the production timer import and the host's disabled-owner fallback together.

use redb::{Database, TableDefinition};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use watane_dev::scripted::{Reply, ScriptedResponses, SseEvent};
use watane_host::{Caller, Host, Limits, LocalName, Mode};

const FIXTURE_TOKEN: &str = "s138-scripted-token";

fn until<T>(what: &str, mut try_now: impl FnMut() -> Option<T>) -> T {
    let timeout = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(result) = try_now() {
            return result;
        }
        assert!(Instant::now() < timeout, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn seed_profile(home: &Path) {
    let db = until("the preview database to close", || {
        Database::open(home.join("state/state.redb")).ok()
    });
    let txn = db.begin_write().unwrap();
    {
        let id = blake3::hash(b"local/agent").to_hex()[..32].to_owned();
        let name = format!("s/{id}");
        let mut state = txn
            .open_table(TableDefinition::<&str, &[u8]>::new(&name))
            .unwrap();
        let profile: &[u8] = br#"{"backend":"openai","model":"gpt-5"}"#;
        let access: &[u8] = br#"["fixture"]"#;
        state.insert("profile/fixture", profile).unwrap();
        state.insert("access/local/echo", access).unwrap();
        let mut sizes = txn
            .open_table(TableDefinition::<&str, u64>::new("sizes"))
            .unwrap();
        sizes
            .insert(
                id.as_str(),
                ("profile/fixture".len() + profile.len() + "access/local/echo".len() + access.len())
                    as u64,
            )
            .unwrap();
    }
    txn.commit().unwrap();
}

fn scenario(agent: &Path, owner: &Path, disabled: bool) {
    let home = tempfile::tempdir().unwrap();
    let limits = Limits::default();
    let host = Host::open(home.path(), Mode::Preview, limits.clone()).unwrap();
    host.install_folders(&[agent.into(), owner.into()], 0)
        .unwrap();
    drop(host);
    seed_profile(home.path());
    let server = ScriptedResponses::start().unwrap();
    server.expect_token(FIXTURE_TOKEN);
    server.script(vec![
        Reply::UsageLimit { resets_in_s: 3 },
        Reply::Sse(vec![SseEvent::Completed {
            input_tokens: 1,
            output_tokens: 1,
        }]),
    ]);
    let host = Host::open(home.path(), Mode::Preview, limits).unwrap();
    let std::net::SocketAddr::V4(address) = server.responses_addr() else {
        panic!("the scripted server must use IPv4");
    };
    host.set_scripted_responses(address, &server.ca_pem(), FIXTURE_TOKEN.into(), None)
        .unwrap();

    let agent: LocalName = "agent".parse().unwrap();
    let owner: LocalName = "echo".parse().unwrap();
    let call = |function, args| {
        host.call(&Caller::Mod(owner.clone()), &agent, function, args)
            .unwrap()
    };
    let started = call(
        "sessions.start",
        json!({"key":"s138","profile":"fixture","overlay":"{}","input":"hello"}),
    );
    assert_eq!(started, json!({"ok":"s1"}));
    let turn = || {
        let list = call("sessions.list", json!({"after":null,"max":10}));
        serde_json::from_str::<Value>(list["ok"][0]["turn"].as_str().unwrap()).unwrap()
    };
    until("the usage-limit pause", || {
        (turn()["state"] == "paused").then_some(())
    });
    assert_eq!(server.requests().len(), 1);
    if disabled {
        host.disable(&owner).unwrap();
        // A call from a disabled owner cannot carry authority. Read the recorded result
        // only after the timer's deadline, then enable the owner for its next read.
        thread::sleep(Duration::from_secs(4));
        assert_eq!(server.requests().len(), 1);
        host.enable(&owner).unwrap();
        let failed = until("the disabled owner's failure", || {
            let t = turn();
            (t["state"] == "idle").then_some(t)
        });
        assert_eq!(failed["stop"]["kind"], "failed");
        assert_eq!(
            failed["stop"]["error"],
            "Timer did not carry the session owner's authority"
        );
        assert_eq!(server.requests().len(), 1);
    } else {
        until("the enabled owner's second model request", || {
            (server.requests().len() == 2).then_some(())
        });
        let completed = until("the resumed turn to complete", || {
            let t = turn();
            (t["state"] == "idle").then_some(t)
        });
        assert_eq!(completed["stop"]["kind"], "completed");
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let agent = args.next().expect("built agent mod directory");
    let owner = args.next().expect("Watane echo fixture directory");
    thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(move || {
            scenario(&agent, &owner, false);
            scenario(&agent, &owner, true);
        })
        .unwrap()
        .join()
        .unwrap();
    println!("S138 production timer origin: enabled and disabled owner passed");
}
