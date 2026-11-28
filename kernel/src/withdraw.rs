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

fn number(id: &str) -> Option<u64> {
    id.strip_prefix('s')?.parse().ok()
}
fn cutoff(state: &mut impl State) -> u64 {
    state
        .get("next-id")
        .and_then(|v| std::str::from_utf8(&v).ok()?.parse().ok())
        .unwrap_or(1)
}

pub(crate) fn has_withdrawing(state: &mut impl State, client: &str, profile: &str) -> bool {
    read::<Vec<Withdrawal>>(state, "withdrawing")
        .unwrap_or_default()
        .iter()
        .any(|r| r.client == client && r.profiles.iter().any(|p| p.name == profile))
}
pub(crate) fn has_client_withdrawing(state: &mut impl State, client: &str) -> bool {
    read::<Vec<Withdrawal>>(state, "withdrawing")
        .unwrap_or_default()
        .iter()
        .any(|r| r.client == client && !r.profiles.is_empty())
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
        read::<Vec<Withdrawal>>(&mut self.state, "withdrawing")
            .unwrap_or_default()
            .iter()
            .any(|w| {
                w.client == owner
                    && w.profiles
                        .iter()
                        .any(|p| p.name == root.profile && n < p.below)
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
        let mut rows: Vec<Withdrawal> = read(&mut self.state, "withdrawing").unwrap_or_default();
        let below = cutoff(&mut self.state);
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
            let cascade = if rec.parent.is_none() {
                1 + rec
                    .children
                    .iter()
                    .filter(|id| {
                        Record::load(&mut self.state, id).is_some_and(|c| c.turn.is_active())
                    })
                    .count()
            } else {
                1
            };
            if ended > 0 && ended + cascade > 16 {
                break;
            }
            if self.cancel(client, &rec.id).is_ok() {
                ended += cascade;
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
        let digest = "a".repeat(64);
        put(
            &mut k.state,
            "profiles",
            &vec![ProfileIndexEntry {
                name: "p".into(),
                digest: digest.clone(),
                rev: 1,
                bytes: 1,
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
        let rec = Record {
            id: format!("s{n}"),
            owner: "local/c".into(),
            profile: profile.into(),
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
        k.withdraw_firing();
        assert!(!has_withdrawing(&mut k.state, "local/c", "p"));
        assert!(!Record::load(&mut k.state, "s20").unwrap().turn.is_active());
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
}
