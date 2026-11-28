//! Pinned access fixtures for integration tests (TECHNICAL 9.10).

use agent_kernel::access::{digest, AccessEntry, ProfileIndexEntry};
use agent_kernel::{State, TooLarge};

/// Seed a grant from the old fixture's list of names, pinning each stored profile.
/// Values that are not lists of names remain raw for denial tests.
pub fn put(state: &mut impl State, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
    let Ok(names) = serde_json::from_slice::<Vec<String>>(&value) else {
        return state.put(key, value);
    };
    let mut index: Vec<ProfileIndexEntry> = state
        .get("profiles")
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default();
    for name in &names {
        if let Some(bytes) = state.get(&format!("profile/{name}")) {
            let row = ProfileIndexEntry {
                name: name.clone(),
                digest: digest(&bytes),
                rev: 1,
                bytes: bytes.len() as u32,
                offer: None,
                stale: false,
                gone: false,
            };
            if let Some(old) = index.iter_mut().find(|r| r.name == *name) {
                *old = row;
            } else {
                index.push(row);
            }
        }
    }
    let entries: Vec<AccessEntry> = names
        .into_iter()
        .map(|name| AccessEntry {
            digest: index
                .iter()
                .find(|r| r.name == name)
                .map(|r| r.digest.clone())
                .unwrap_or_else(|| digest(&[])),
            profile: name,
            rev: 1,
        })
        .collect();
    state.put("profiles", serde_json::to_vec(&index).unwrap())?;
    state.put(key, serde_json::to_vec(&entries).unwrap())
}

#[test]
fn ac1_grant_fixture_pins_exact_profile_bytes_and_index() {
    let mut state = agent_kernel::MemState::new();
    state.put("profile/p", b"{ }".to_vec()).unwrap();
    put(&mut state, "access/alice", br#"["p"]"#.to_vec()).unwrap();
    let grant: Vec<AccessEntry> =
        serde_json::from_slice(&state.get("access/alice").unwrap()).unwrap();
    let rows: Vec<ProfileIndexEntry> =
        serde_json::from_slice(&state.get("profiles").unwrap()).unwrap();
    assert_eq!(grant[0].digest, digest(b"{ }"));
    assert_eq!(grant[0].rev, 1);
    assert_eq!(rows[0].name, "p");
    assert_eq!(rows[0].digest, grant[0].digest);
    assert_eq!(rows[0].bytes, 3);
}

#[test]
fn ac2_legacy_access_value_stays_unreadable() {
    let mut state = agent_kernel::MemState::new();
    state.put("access/alice", br#"["p"]"#.to_vec()).unwrap();
    assert!(agent_kernel::access::entries(&mut state, "alice").is_none());
}
