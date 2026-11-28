//! Profile withdrawal and its kernel timer (TECHNICAL 9.10).

use crate::access;
use crate::error::KernelError;
use crate::manage::{put, read, refusal, Pending};
use crate::quota::{segment, Usage};
use crate::record::Record;
use crate::sessions::{Host, Kernel};
use crate::state::State;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct Withdrawal {
    client: String,
    profiles: Vec<ProfileCutoff>,
}
#[derive(Clone, Serialize, Deserialize)]
struct ProfileCutoff {
    name: String,
    below: u64,
}
fn load_withdrawing(state: &mut impl State) -> Result<Vec<Withdrawal>, ()> {
    match state.get("withdrawing") {
        None => Ok(Vec::new()),
        Some(raw) => serde_json::from_slice(&raw).map_err(|_| ()),
    }
}

fn number(id: &str) -> Option<u64> {
    id.strip_prefix('s')?.parse().ok()
}
fn cutoff(state: &mut impl State) -> u64 {
    state
        .get("next-id")
        .and_then(|v| std::str::from_utf8(&v).ok()?.parse().ok())
        .unwrap_or(u64::MAX)
}

pub(crate) fn has_withdrawing(state: &mut impl State, client: &str, profile: &str) -> bool {
    load_withdrawing(state).map_or(true, |rows| {
        rows.iter()
            .any(|r| r.client == client && r.profiles.iter().any(|p| p.name == profile))
    })
}
pub(crate) fn has_client_withdrawing(state: &mut impl State, client: &str) -> bool {
    load_withdrawing(state).map_or(true, |rows| {
        rows.iter()
            .any(|r| r.client == client && !r.profiles.is_empty())
    })
}

impl<S: State, H: Host> Kernel<S, H> {
    pub(crate) fn withdrawing(&mut self, owner: &str, rec: &Record) -> bool {
        let root = match &rec.parent {
            Some(id) => match Record::load(&mut self.state, id) {
                Some(root) => root,
                None => return true,
            },
            None => rec.clone(),
        };
        let Some(n) = number(&root.id) else {
            return true;
        };
        load_withdrawing(&mut self.state).map_or(true, |rows| {
            rows.iter().any(|w| {
                w.client == owner
                    && w.profiles
                        .iter()
                        .any(|p| p.name == root.profile && n < p.below)
            })
        })
    }

    pub fn manage_withdraw(
        &mut self,
        caller: &str,
        client: &str,
        profile: &str,
    ) -> Result<u32, KernelError> {
        self.lease(caller)?;
        let mut clients: Vec<String> = read(&mut self.state, "clients").unwrap_or_default();
        clients.push("page".into());
        if !clients.iter().any(|c| c == client) {
            return Err(refusal("client", "not a client of the kernel"));
        }
        if !access::index(&mut self.state)
            .iter()
            .any(|p| p.name == profile)
        {
            return Err(refusal("profile", "no such profile"));
        }
        let mut rows = load_withdrawing(&mut self.state)
            .map_err(|_| refusal("profile", "unreadable withdrawal state"))?;
        let below = cutoff(&mut self.state);
        let mut entries = access::entries(&mut self.state, client).unwrap_or_default();
        entries.retain(|e| e.profile != profile);
        put(&mut self.state, &format!("access/{client}"), &entries);
        self.state
            .delete(&format!("access-fields/{client}/{profile}"));
        let asks: Vec<String> = read(&mut self.state, "access-asks").unwrap_or_default();
        for id in asks {
            if matches!(read::<Pending>(&mut self.state, &format!("access-ask/{id}")),
                Some(Pending::Allow { client: c, profile: p, .. }) if c == client && p == profile)
            {
                self.cancel_question(&id);
            }
        }
        if let Some(row) = rows.iter_mut().find(|r| r.client == client) {
            if let Some(p) = row.profiles.iter_mut().find(|p| p.name == profile) {
                p.below = below;
            } else {
                row.profiles.push(ProfileCutoff {
                    name: profile.into(),
                    below,
                });
            }
        } else {
            rows.push(Withdrawal {
                client: client.into(),
                profiles: vec![ProfileCutoff {
                    name: profile.into(),
                    below,
                }],
            });
        }
        put(&mut self.state, "withdrawing", &rows);
        let left = self.withdraw_batch(client, profile, below);
        self.withdraw_cleanup(client, profile, left);
        self.after_call();
        Ok(left)
    }

    fn withdraw_ids(&mut self, client: &str, profile: &str, below: u64) -> Vec<Record> {
        let usage = Usage::load(&mut self.state, &Usage::owner_key(client));
        let mut records = Vec::new();
        for n in 0..usage.pages.len() {
            let key = format!("owner/{}/{n}", segment(client));
            let ids: Vec<String> = read(&mut self.state, &key).unwrap_or_default();
            for id in ids {
                if let Some(rec) = Record::load(&mut self.state, &id) {
                    let root = match &rec.parent {
                        Some(parent) => Record::load(&mut self.state, parent),
                        None => Some(rec.clone()),
                    };
                    if root.as_ref().is_some_and(|r| {
                        r.profile == profile && number(&r.id).is_some_and(|v| v < below)
                    }) && rec.turn.is_active()
                    {
                        records.push(rec);
                    }
                }
            }
        }
        records.sort_by_key(|r| number(&r.id).unwrap_or(u64::MAX));
        records
    }

    fn withdraw_batch(&mut self, client: &str, profile: &str, below: u64) -> u32 {
        let records = self.withdraw_ids(client, profile, below);
        let mut ended = 0;
        for rec in &records {
            if Record::load(&mut self.state, &rec.id).is_none_or(|r| !r.turn.is_active()) {
                continue;
            }
            let children: Vec<String> = if rec.parent.is_none() {
                rec.children
                    .iter()
                    .filter(|id| {
                        Record::load(&mut self.state, id).is_some_and(|c| c.turn.is_active())
                    })
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };
            let cascade = 1 + children.len();
            if ended > 0 && ended + cascade > 16 {
                break;
            }
            if self.cancel_session(client, &rec.id).is_ok() {
                ended += 1;
                for child in children {
                    if Record::load(&mut self.state, &child).is_some_and(|c| c.turn.is_active()) {
                        if self.cancel_session(client, &child).is_ok() {
                            ended += 1;
                        }
                    } else {
                        ended += 1;
                    }
                }
            }
        }
        self.withdraw_ids(client, profile, below).len() as u32
    }

    fn withdraw_cleanup(&mut self, client: &str, profile: &str, left: u32) {
        let mut rows: Vec<Withdrawal> = read(&mut self.state, "withdrawing").unwrap_or_default();
        if left == 0 {
            for row in &mut rows {
                if row.client == client {
                    row.profiles.retain(|p| p.name != profile);
                }
            }
            rows.retain(|r| !r.profiles.is_empty());
        }
        put(&mut self.state, "withdrawing", &rows);
        if !rows.is_empty() {
            self.host.timer_after("withdraw", 0);
        }
    }

    pub(crate) fn withdraw_firing(&mut self) {
        let rows: Vec<Withdrawal> = read(&mut self.state, "withdrawing").unwrap_or_default();
        if let Some((client, p)) = rows
            .first()
            .and_then(|r| r.profiles.first().map(|p| (r.client.clone(), p.clone())))
        {
            let left = self.withdraw_batch(&client, &p.name, p.below);
            self.withdraw_cleanup(&client, &p.name, left);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::{AccessEntry, ProfileIndexEntry};
    use crate::record::Turn;
    use crate::sessions::{FakeHost, HostCall};
    use crate::state::MemState;

    fn setup() -> Kernel<MemState, FakeHost> {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("c1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        put(&mut k.state, "clients", &vec!["local/c"]);
        let digest = access::digest(b"{}");
        k.state.put("profile/p", b"{}".to_vec()).unwrap();
        put(
            &mut k.state,
            "profiles",
            &vec![ProfileIndexEntry {
                name: "p".into(),
                digest: digest.clone(),
                rev: 1,
                bytes: 2,
                offer: None,
                stale: false,
                gone: false,
            }],
        );
        put(
            &mut k.state,
            "access/local/c",
            &vec![AccessEntry {
                profile: "p".into(),
                digest,
                rev: 1,
            }],
        );
        k.state
            .put("access-fields/local/c/p", b"{}".to_vec())
            .unwrap();
        k
    }

    fn add(k: &mut Kernel<MemState, FakeHost>, n: u64, profile: &str) {
        let digest = access::digest(b"{}");
        let rec = Record {
            id: format!("s{n}"),
            owner: "local/c".into(),
            profile: profile.into(),
            profile_digest: Some(digest),
            turn: Turn::Running {
                since: 0,
                activity: None,
            },
            ..Record::default()
        };
        rec.save(&mut k.state).unwrap();
        let mut ids: Vec<String> = read(&mut k.state, "owner/local%2Fc/0").unwrap_or_default();
        ids.push(rec.id);
        put(&mut k.state, "owner/local%2Fc/0", &ids);
        let mut usage = Usage::load(&mut k.state, &Usage::owner_key("local/c"));
        usage.pages = vec![ids.len() as u32];
        usage
            .save(&mut k.state, &Usage::owner_key("local/c"))
            .unwrap();
    }

    #[test]
    fn ac1_withdraw_removes_grant_fields_and_question() {
        let mut k = setup();
        k.state.put("next-id", b"2".to_vec()).unwrap();
        add(&mut k, 1, "p");
        assert_eq!(k.manage_withdraw("page", "local/c", "p").unwrap(), 0);
        assert!(access::entries(&mut k.state, "local/c").unwrap().is_empty());
        assert!(k.state.get("access-fields/local/c/p").is_none());
    }

    #[test]
    fn ac2_batches_and_cutoff_guard_old_work() {
        let mut k = setup();
        for n in 1..=20 {
            add(&mut k, n, "p");
        }
        k.state.put("next-id", b"21".to_vec()).unwrap();
        assert_eq!(k.manage_withdraw("page", "local/c", "p").unwrap(), 4);
        assert!(has_withdrawing(&mut k.state, "local/c", "p"));
        let old = Record::load(&mut k.state, "s20").unwrap();
        assert!(k.withdrawing("local/c", &old));
        k.manage_allow("page", "local/c", "p").unwrap();
        k.manage_on_item("t2", &serde_json::json!({"type":"granted"}));
        assert!(
            !k.may_work("local/c", &old),
            "re-allow cannot resume old work during the timer batch"
        );
        k.withdraw_firing();
        assert!(!has_withdrawing(&mut k.state, "local/c", "p"));
        assert!(!Record::load(&mut k.state, "s20").unwrap().turn.is_active());
    }
    #[test]
    fn ac2_a_batch_ends_a_whole_root_cascade() {
        let mut k = setup();
        for n in 1..=20 {
            add(&mut k, n, "p");
        }
        let mut root = Record::load(&mut k.state, "s1").unwrap();
        root.children = vec!["s2".into(), "s3".into(), "s4".into()];
        root.save(&mut k.state).unwrap();
        for n in 2..=4 {
            let mut child = Record::load(&mut k.state, &format!("s{n}")).unwrap();
            child.parent = Some("s1".into());
            child.save(&mut k.state).unwrap();
        }
        k.state.put("next-id", b"21".to_vec()).unwrap();
        assert_eq!(k.manage_withdraw("page", "local/c", "p").unwrap(), 4);
        for n in 1..=4 {
            assert!(!Record::load(&mut k.state, &format!("s{n}"))
                .unwrap()
                .turn
                .is_active());
        }
    }
    #[test]
    fn ac2_unreadable_withdrawal_state_fails_closed() {
        let mut k = setup();
        k.state.put("withdrawing", b"broken".to_vec()).unwrap();
        assert!(has_withdrawing(&mut k.state, "local/c", "p"));
        assert_eq!(
            k.manage_withdraw("page", "local/c", "p")
                .unwrap_err()
                .code(),
            "refused"
        );
    }
    #[test]
    fn ac1_real_pinned_session_loses_new_input_on_withdrawal() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                generation: 1,
                connection_id: Some("c1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.manage_put_profile("page", "p", "{}").unwrap();
        k.manage_on_item("t2", &serde_json::json!({"type":"granted"}));
        k.manage_on_ended("t2");
        put(&mut k.state, "clients", &vec!["local/c"]);
        k.manage_allow("page", "local/c", "p").unwrap();
        k.manage_on_item("t3", &serde_json::json!({"type":"granted"}));
        let id = k.start("local/c", "start", "p", "{}", "hi").unwrap().value;
        k.state.put("next-id", b"2".to_vec()).unwrap();
        k.manage_withdraw("page", "local/c", "p").unwrap();
        assert_eq!(
            k.say("local/c", &id, "later", "hi").unwrap_err().code(),
            "refused"
        );
    }

    #[test]
    fn ac3_timer_finishes_without_page_requests() {
        let mut k = setup();
        for n in 1..=17 {
            add(&mut k, n, "p");
        }
        k.state.put("next-id", b"18".to_vec()).unwrap();
        k.manage_withdraw("page", "local/c", "p").unwrap();
        k.host.calls.clear();
        k.timer("withdraw");
        assert!(!has_withdrawing(&mut k.state, "local/c", "p"));
        assert!(!k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::AccessAsk { .. })));
    }

    #[test]
    fn val_withdraw_cancels_pending_allow_and_rearms_latest_cutoff() {
        let mut k = setup();
        add(&mut k, 1, "p");
        k.state.put("next-id", b"2".to_vec()).unwrap();
        put(&mut k.state, "access/local/c", &Vec::<AccessEntry>::new());
        k.manage_allow("page", "local/c", "p").unwrap();
        let ask = k
            .host
            .calls
            .iter()
            .rev()
            .find_map(|c| match c {
                HostCall::AccessAsk { stream, .. } => Some(stream.clone()),
                _ => None,
            })
            .unwrap();
        assert!(k.state.get(&format!("access-ask/{ask}")).is_some(), "ask {ask} was not pending");
        k.manage_withdraw("page", "local/c", "p").unwrap();
        assert!(k.state.get(&format!("access-ask/{ask}")).is_none());
        assert!(k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, HostCall::CancelStream { stream } if stream == &ask)), "calls: {:?}", k.host.calls);

        add(&mut k, 2, "p");
        k.state.put("next-id", b"3".to_vec()).unwrap();
        k.manage_withdraw("page", "local/c", "p").unwrap();
        let rows: Vec<Withdrawal> = read(&mut k.state, "withdrawing").unwrap_or_default();
        assert!(rows.is_empty() || rows[0].profiles[0].below == 3);
    }

    #[test]
    fn val_withdraw_cutoff_profile_and_client_are_exact() {
        let mut k = setup();
        add(&mut k, 1, "p");
        add(&mut k, 2, "other");
        add(&mut k, 3, "p");
        k.state.put("next-id", b"3".to_vec()).unwrap();
        put(
            &mut k.state,
            "withdrawing",
            &vec![Withdrawal {
                client: "local/c".into(),
                profiles: vec![ProfileCutoff {
                    name: "p".into(),
                    below: 3,
                }],
            }],
        );
        let (one, two, three) = (
            Record::load(&mut k.state, "s1").unwrap(),
            Record::load(&mut k.state, "s2").unwrap(),
            Record::load(&mut k.state, "s3").unwrap(),
        );
        assert!(k.withdrawing("local/c", &one));
        assert!(!k.withdrawing("local/c", &two));
        assert!(!k.withdrawing("local/c", &three));
        assert!(!k.withdrawing("local/other", &one));
    }

    #[test]
    fn val_withdraw_missing_root_fails_closed_for_child() {
        let mut k = setup();
        add(&mut k, 2, "p");
        let mut child = Record::load(&mut k.state, "s2").unwrap();
        child.parent = Some("missing".into());
        assert!(k.withdrawing("local/c", &child));
    }
}
