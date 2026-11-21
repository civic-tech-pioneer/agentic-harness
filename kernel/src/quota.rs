//! Quota pieces (TECHNICAL 9.3 "Quota", "As built (S25)"): the caps, the two accounts, and the metered state.

use crate::error::KernelError;
use crate::state::{State, TooLarge};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const OWNER_SESSIONS: u64 = 64;
pub const OWNER_BYTES: u64 = 64 << 20;
pub const ACCOUNT_BYTES: u64 = 224 << 20;
pub const RESERVE_BYTES: u64 = 32 << 20;
/// Provider streams the kernel holds, and one owner (9.5 "Slots").
pub const PROVIDER_STREAMS: u32 = 8;
pub const OWNER_PROVIDER: u32 = 4;

/// The kernel settings for the caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub owner_sessions: u64,
    pub owner_bytes: u64,
    pub account_bytes: u64,
    pub reserve_bytes: u64,
    pub provider: u32,
    pub owner_provider: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            owner_sessions: OWNER_SESSIONS,
            owner_bytes: OWNER_BYTES,
            account_bytes: ACCOUNT_BYTES,
            reserve_bytes: RESERVE_BYTES,
            provider: PROVIDER_STREAMS,
            owner_provider: OWNER_PROVIDER,
        }
    }
}

/// An account as stored at `quota/<owner>` and `quota` (9.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub bytes: u64,
    pub sessions: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<u32>,
    /// The owner's provider streams, valid only while `slots_boot` is the boot of `slots` (9.1).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub provider: u32,
    #[serde(default, skip_serializing_if = "is_zero", rename = "slots-boot")]
    pub slots_boot: u64,
}

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl Usage {
    pub const ACCOUNT: &'static str = "quota";

    pub fn owner_key(owner: &str) -> String {
        format!("quota/{}", segment(owner))
    }

    /// An absent or unreadable value is the empty account.
    pub fn load(state: &mut impl State, key: &str) -> Usage {
        state
            .get(key)
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, state: &mut impl State, key: &str) -> Result<(), KernelError> {
        let v = serde_json::to_vec(self).expect("usage serializes");
        state
            .put(key, v)
            .map_err(|t| KernelError::TooLarge(t.0 as u64))
    }

    /// The owner's provider streams of this boot: a count tagged with an older boot reads zero.
    pub fn provider_at(&self, boot: u64) -> u32 {
        if self.slots_boot == boot {
            self.provider
        } else {
            0
        }
    }

    /// Takes one more provider stream, restarting the count when it is of an older boot.
    pub fn open_provider(&mut self, boot: u64) {
        self.provider = self.provider_at(boot) + 1;
        self.slots_boot = boot;
    }

    /// Frees one; an older boot's count is already zero.
    pub fn close_provider(&mut self, boot: u64) {
        self.provider = self.provider_at(boot).saturating_sub(1);
        self.slots_boot = boot;
    }

    /// Adds a signed delta to `bytes`, never below 0.
    pub fn add(&mut self, delta: i64) {
        self.bytes = if delta >= 0 {
            self.bytes.saturating_add(delta as u64)
        } else {
            self.bytes.saturating_sub(delta.unsigned_abs())
        };
    }
}

/// 9.1: `%` to `%25`, then `/` to `%2F`.
pub fn segment(owner: &str) -> String {
    owner.replace('%', "%25").replace('/', "%2F")
}

/// Whose bytes a key is (9.3 "Bytes").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Record,
    Session,
    Owner,
    Kernel,
}

pub fn scope(key: &str) -> Scope {
    let Some((head, _)) = key.split_once('/') else {
        return Scope::Kernel;
    };
    match head {
        "session" => Scope::Record,
        "queue" | "entries" | "wire" | "loop" | "stream" | "live" => Scope::Session,
        "key" | "owner" => Scope::Owner,
        _ => Scope::Kernel,
    }
}

/// The change in charged bytes: for the owner, and for the session keys alone.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub owner: i64,
    pub session: i64,
}

/// A state that, while open, adds up the change in bytes of the charged keys written or deleted.
pub struct Metered<S: State> {
    inner: S,
    open: bool,
    /// Value size of each key seen since `open` (`None`: absent).
    seen: HashMap<String, Option<usize>>,
    change: Change,
}

impl<S: State> Metered<S> {
    pub fn new(inner: S) -> Self {
        Metered {
            inner,
            open: false,
            seen: HashMap::new(),
            change: Change::default(),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }

    pub fn open(&mut self) {
        self.open = true;
        self.seen.clear();
        self.change = Change::default();
    }

    pub fn change(&self) -> Change {
        self.change
    }

    pub fn close(&mut self) -> Change {
        self.open = false;
        self.seen.clear();
        self.change
    }

    pub fn adjust(&mut self, c: Change) {
        self.change.owner += c.owner;
        self.change.session += c.session;
    }

    /// Deletes without a read and without a charge.
    pub fn delete_uncharged(&mut self, key: &str) {
        self.seen.remove(key);
        self.inner.delete(key);
    }

    fn old_size(&mut self, key: &str) -> Option<usize> {
        if let Some(s) = self.seen.get(key) {
            return *s;
        }
        let old = self.inner.get(key).map(|v| v.len());
        self.seen.insert(key.to_string(), old);
        old
    }

    fn charge(&mut self, key: &str, delta: i64) {
        self.change.owner += delta;
        if scope(key) == Scope::Session {
            self.change.session += delta;
        }
    }

    fn metered(&self, key: &str) -> bool {
        self.open && scope(key) != Scope::Kernel
    }
}

impl<S: State> State for Metered<S> {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        let v = self.inner.get(key);
        if self.metered(key) {
            self.seen
                .insert(key.to_string(), v.as_ref().map(|v| v.len()));
        }
        v
    }

    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        if !self.metered(key) {
            return self.inner.put(key, value);
        }
        let old = self.old_size(key);
        let new = value.len();
        self.inner.put(key, value)?;
        let cost = |n: Option<usize>| n.map_or(0, |n| (key.len() + n) as i64);
        self.charge(key, cost(Some(new)) - cost(old));
        self.seen.insert(key.to_string(), Some(new));
        Ok(())
    }

    fn delete(&mut self, key: &str) {
        if self.metered(key) {
            if let Some(old) = self.old_size(key) {
                self.charge(key, -((key.len() + old) as i64));
            }
            self.seen.insert(key.to_string(), None);
        }
        self.inner.delete(key);
    }
}

impl<S: State> std::ops::Deref for Metered<S> {
    type Target = S;
    fn deref(&self) -> &S {
        &self.inner
    }
}

impl<S: State> std::ops::DerefMut for Metered<S> {
    fn deref_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}
