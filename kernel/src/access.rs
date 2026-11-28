//! Pinned profile versions (TECHNICAL 9.10). Unreadable state grants nothing.

use crate::state::State;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessEntry {
    pub profile: String,
    pub digest: String,
    pub rev: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferLink {
    pub name: String,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileIndexEntry {
    pub name: String,
    pub digest: String,
    pub rev: u32,
    pub bytes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer: Option<OfferLink>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gone: bool,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn entries(state: &mut impl State, owner: &str) -> Option<Vec<AccessEntry>> {
    let raw = state.get(&format!("access/{owner}"))?;
    let entries: Vec<AccessEntry> = serde_json::from_slice(&raw).ok()?;
    let mut names = HashSet::new();
    (entries.len() <= 64
        && entries.iter().all(|e| {
            !e.profile.is_empty()
                && e.profile.len() <= 64
                && e.profile
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                && valid_digest(&e.digest)
                && e.rev > 0
                && names.insert(&e.profile)
        }))
    .then_some(entries)
}

pub fn index(state: &mut impl State) -> Vec<ProfileIndexEntry> {
    state
        .get("profiles")
        .and_then(|raw| serde_json::from_slice::<Vec<ProfileIndexEntry>>(&raw).ok())
        .filter(|rows| rows.len() <= 256)
        .unwrap_or_default()
}

pub fn holds(entry: &AccessEntry, index: &[ProfileIndexEntry]) -> bool {
    index.iter().any(|row| {
        row.name == entry.profile
            && row.digest == entry.digest
            && row.rev == entry.rev
            && !row.stale
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MemState;
    use crate::{active::Slots, FakeHost, Kernel};

    #[test]
    fn ac1_legacy_and_malformed_entries_grant_nothing() {
        let mut state = MemState::new();
        for value in [
            br#"["p"]"#.as_slice(),
            br#"[{"profile":"p","digest":"ABC","rev":1}]"#,
        ] {
            state.put("access/alice", value.to_vec()).unwrap();
            assert!(entries(&mut state, "alice").is_none());
        }
    }

    #[test]
    fn ac1_entry_holds_only_at_the_allowed_version() {
        let digest = digest(b"profile");
        let entry = AccessEntry {
            profile: "p".into(),
            digest: digest.clone(),
            rev: 1,
        };
        let mut row = ProfileIndexEntry {
            name: "p".into(),
            digest,
            rev: 1,
            bytes: 7,
            offer: None,
            stale: false,
            gone: false,
        };
        assert!(holds(&entry, &[row.clone()]));
        row.rev = 2;
        assert!(!holds(&entry, &[row.clone()]));
        row.rev = 1;
        row.stale = true;
        assert!(!holds(&entry, &[row]));
    }

    #[test]
    fn ac1_kernel_refuses_legacy_access_before_reading_profile() {
        let mut state = MemState::new();
        state.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
        Slots {
            boot: 1,
            ..Slots::default()
        }
        .save(&mut state)
        .unwrap();
        let mut kernel = Kernel::new(
            state,
            FakeHost {
                generation: 1,
                ..FakeHost::default()
            },
        );
        assert_eq!(kernel.check_named("alice", "p", "{}")[0].field, "profile");
        assert!(kernel.profiles("alice").is_empty());
        assert_eq!(
            kernel
                .start("alice", "k", "p", "{}", "hi")
                .unwrap_err()
                .code(),
            "refused"
        );
        assert!(!kernel.state.get_keys().iter().any(|key| key == "profile/p"));
    }

    #[test]
    fn ac2_changed_revision_blocks_new_work_even_with_same_bytes() {
        let mut state = MemState::new();
        let profile = br#"{"backend":"claude"}"#;
        let d = digest(profile);
        state.put("profile/p", profile.to_vec()).unwrap();
        state
            .put(
                "profiles",
                serde_json::to_vec(&vec![ProfileIndexEntry {
                    name: "p".into(),
                    digest: d.clone(),
                    rev: 2,
                    bytes: profile.len() as u32,
                    offer: None,
                    stale: false,
                    gone: false,
                }])
                .unwrap(),
            )
            .unwrap();
        state
            .put(
                "access/alice",
                serde_json::to_vec(&vec![AccessEntry {
                    profile: "p".into(),
                    digest: d,
                    rev: 1,
                }])
                .unwrap(),
            )
            .unwrap();
        Slots {
            boot: 1,
            ..Slots::default()
        }
        .save(&mut state)
        .unwrap();
        let mut kernel = Kernel::new(
            state,
            FakeHost {
                generation: 1,
                ..FakeHost::default()
            },
        );
        assert!(kernel.profiles("alice").is_empty());
        assert_eq!(
            kernel.check_named("alice", "p", "{}")[0].reason,
            "changed since the participant allowed it"
        );
    }

    #[test]
    fn ac2_session_keeps_digest_and_rejects_later_version() {
        let mut state = MemState::new();
        let profile = br#"{"backend":"claude","ask":true}"#;
        let d = digest(profile);
        state.put("profile/p", profile.to_vec()).unwrap();
        let row = ProfileIndexEntry {
            name: "p".into(),
            digest: d.clone(),
            rev: 1,
            bytes: profile.len() as u32,
            offer: None,
            stale: false,
            gone: false,
        };
        state
            .put("profiles", serde_json::to_vec(&vec![row.clone()]).unwrap())
            .unwrap();
        state
            .put(
                "access/alice",
                serde_json::to_vec(&vec![AccessEntry {
                    profile: "p".into(),
                    digest: d.clone(),
                    rev: 1,
                }])
                .unwrap(),
            )
            .unwrap();
        Slots {
            boot: 1,
            ..Slots::default()
        }
        .save(&mut state)
        .unwrap();
        let mut kernel = Kernel::new(
            state,
            FakeHost {
                generation: 1,
                ..FakeHost::default()
            },
        );
        let id = kernel.start("alice", "k", "p", "{}", "hi").unwrap().value;
        let rec = crate::Record::load(&mut kernel.state, &id).unwrap();
        assert_eq!(rec.profile_digest.as_deref(), Some(d.as_str()));
        let mut newer = row;
        newer.rev = 2;
        kernel
            .state
            .put("profiles", serde_json::to_vec(&vec![newer]).unwrap())
            .unwrap();
        assert_eq!(
            kernel.say("alice", &id, "next", "hi").unwrap_err().code(),
            "refused"
        );
    }

    #[test]
    fn ac3_only_eligible_caller_registers() {
        let mut kernel = Kernel::new(MemState::new(), FakeHost::default());
        kernel.profiles("page");
        kernel.check_named("agent:tool", "p", "{}");
        kernel.profiles("local/factory");
        kernel.check_named("local/factory", "p", "{}");
        let clients: Vec<String> =
            serde_json::from_slice(&kernel.state.get("clients").unwrap()).unwrap();
        assert_eq!(clients, vec!["local/factory"]);
    }
}
