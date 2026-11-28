use agent_kernel::active::Slots;
use agent_kernel::manage::OfferedProfile;
use agent_kernel::*;
use serde_json::json;

const P: &str = r#"{"mode":"manual","tools":["Read"],"max-turns":20}"#;
const P2: &str = r#"{"mode":"manual","tools":["Read","Grep"],"max-turns":20}"#;

fn grant(k: &mut Kernel<MemState, FakeHost>, stream: &str) {
    k.manage_on_item(stream, &json!({"type":"granted"}));
    k.manage_on_ended(stream);
}

#[test]
fn regress_s168_say_on_edited_profile_says_changed() {
    let mut st = MemState::new();
    Slots {
        boot: 1,
        ..Slots::default()
    }
    .save(&mut st)
    .unwrap();
    let mut k = Kernel::new(
        st,
        FakeHost {
            generation: 1,
            connection_id: Some("k1".into()),
            ..FakeHost::default()
        },
    );
    k.manage_open("page").unwrap();
    k.manage_on_item("t1", &json!({"type":"granted"}));
    k.manage_put_profile("page", "p", P).unwrap();
    grant(&mut k, "t2");
    k.profiles("local/c");
    k.manage_allow("page", "local/c", "p").unwrap();
    grant(&mut k, "t3");
    let id = k.start("local/c", "k", "p", "{}", "hi").unwrap().value;
    k.manage_put_profile("page", "p", P2).unwrap();
    grant(&mut k, "t5");
    let reason = match k.say("local/c", &id, "q", "more").unwrap_err() {
        KernelError::Refused(r) => r[0].reason.clone(),
        e => panic!("{e:?}"),
    };
    assert_eq!(reason, "changed since the participant allowed it");
}

#[test]
fn stale_profile_is_refused_its_offer_changed_until_readopted() {
    let mut st = MemState::new();
    Slots { boot: 1, ..Slots::default() }.save(&mut st).unwrap();
    let mut k = Kernel::new(st, FakeHost {
        generation: 1,
        connection_id: Some("k1".into()),
        offered_profiles: vec![OfferedProfile {
            name: "suggested".into(), description: "d".into(), profile: P.into(),
        }],
        ..FakeHost::default()
    });
    k.profiles("local/c");
    k.manage_open("page").unwrap();
    k.manage_on_item("t1", &json!({"type":"granted"}));
    k.manage_adopt("page", "suggested", &access::digest(P.as_bytes()), "mine").unwrap();
    grant(&mut k, "t2");
    k.manage_allow("page", "local/c", "mine").unwrap();
    grant(&mut k, "t3");
    k.host.offered_profiles[0].profile = P2.into();
    k.manage_offers("page", 0, 16).unwrap();
    assert_eq!(k.check_named("local/c", "mine", "{}")[0].reason, "its offer changed");
}
