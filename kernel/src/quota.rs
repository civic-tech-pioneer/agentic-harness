//! Quota pieces (TECHNICAL 9.3 "Quota", "As built (S25)"): the caps, the two accounts, and the metered state.

use crate::error::KernelError;
use crate::state::{State, TooLarge, MAX_VALUE};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub const OWNER_SESSIONS: u64 = 64;
pub const OWNER_BYTES: u64 = 64 << 20;
pub const ACCOUNT_BYTES: u64 = 224 << 20;
pub const RESERVE_BYTES: u64 = 32 << 20;
/// Provider streams the kernel holds, and one owner (9.5 "Slots").
pub const PROVIDER_STREAMS: u32 = 8;
pub const OWNER_PROVIDER: u32 = 4;
/// Model requests the kernel holds, and one owner (9.5 "Slots").
pub const REQUEST_STREAMS: u32 = 8;
pub const OWNER_REQUEST: u32 = 4;
/// Tool jobs the kernel holds, one owner, one session (9.5 "Slots").
pub const JOB_STREAMS: u32 = 16;
pub const OWNER_JOB: u32 = 8;
pub const SESSION_JOB: u32 = 8;
/// Session timers the kernel holds, and one owner (9.3 "Deadlines and the session timer").
pub const SESSION_TIMERS: u32 = 240;
pub const OWNER_TIMERS: u32 = 120;

/// The kernel settings for the caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub owner_sessions: u64,
    pub owner_bytes: u64,
    pub account_bytes: u64,
    pub reserve_bytes: u64,
    pub provider: u32,
    pub owner_provider: u32,
    pub request: u32,
    pub owner_request: u32,
    pub job: u32,
    pub owner_job: u32,
    pub session_job: u32,
    pub timers: u32,
    pub owner_timers: u32,
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
            request: REQUEST_STREAMS,
            owner_request: OWNER_REQUEST,
            job: JOB_STREAMS,
            owner_job: OWNER_JOB,
            session_job: SESSION_JOB,
            timers: SESSION_TIMERS,
            owner_timers: OWNER_TIMERS,
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
    /// The owner's model requests, tagged with `slots_boot` as `provider` is.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub request: u32,
    /// The owner's tool jobs, tagged with `slots_boot` as `provider` is.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub job: u32,
    /// The owner's session timers; kept across a restart, so not tagged with a boot (9.3).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub timers: u32,
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

    /// The owner's model requests of this boot (an older boot's count reads zero).
    pub fn request_at(&self, boot: u64) -> u32 {
        if self.slots_boot == boot {
            self.request
        } else {
            0
        }
    }

    /// The owner's tool jobs of this boot (an older boot's count reads zero).
    pub fn job_at(&self, boot: u64) -> u32 {
        if self.slots_boot == boot {
            self.job
        } else {
            0
        }
    }

    /// Counts of an older boot are dropped when the tag moves to `boot`.
    fn retag(&mut self, boot: u64) {
        if self.slots_boot != boot {
            self.provider = 0;
            self.request = 0;
            self.job = 0;
            self.slots_boot = boot;
        }
    }

    /// Takes one more provider stream, restarting the count when it is of an older boot.
    pub fn open_provider(&mut self, boot: u64) {
        self.retag(boot);
        self.provider += 1;
    }

    /// Frees one; an older boot's count is already zero.
    pub fn close_provider(&mut self, boot: u64) {
        self.retag(boot);
        self.provider = self.provider.saturating_sub(1);
    }

    pub fn open_request(&mut self, boot: u64) {
        self.retag(boot);
        self.request += 1;
    }

    pub fn close_request(&mut self, boot: u64) {
        self.retag(boot);
        self.request = self.request.saturating_sub(1);
    }

    pub fn open_job(&mut self, boot: u64) {
        self.retag(boot);
        self.job += 1;
    }

    pub fn close_job(&mut self, boot: u64) {
        self.retag(boot);
        self.job = self.job.saturating_sub(1);
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

/// Whether the owner or the account was past its quota when a call read its accounts (9.3
/// Quota). The field is private to this module and only `Past::load` sets it, from the two
/// accounts it reads itself: no code can hold the flag without having read the accounts.
#[derive(Clone, Copy)]
pub(crate) struct Past {
    past: bool,
    /// The bytes the call may still write before the owner or the account reaches its cap (0 when
    /// past), as read with the accounts.
    room: u64,
}

impl Past {
    /// Reads the owner's account and the kernel's; past when the owner's bytes or the account's
    /// are at or over their caps.
    pub(crate) fn load(
        state: &mut impl State,
        owner_key: &str,
        limits: &Limits,
    ) -> (Past, Usage, Usage) {
        let owner = Usage::load(state, owner_key);
        let account = Usage::load(state, Usage::ACCOUNT);
        let past = owner.bytes >= limits.owner_bytes || account.bytes >= limits.account_bytes;
        let room = limits
            .owner_bytes
            .saturating_sub(owner.bytes)
            .min(limits.account_bytes.saturating_sub(account.bytes));
        (Past { past, room }, owner, account)
    }

    pub(crate) fn is_past(self) -> bool {
        self.past
    }

    /// Whether a decision written now is short (9.4 "The short decision"): past when the accounts
    /// were read, or the call's own writes so far (`written`, the open meter's change for the
    /// owner) have reached the room it had.
    pub(crate) fn short(self, written: i64) -> bool {
        self.past || written >= 0 && written as u64 >= self.room
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
        "queue" | "entries" | "wire" | "loop" | "skills" | "skill-revisions" | "stream"
        | "adapter" | "live" => Scope::Session,
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
    /// While held, the writes wait here (`None`: a delete) until `commit` or `discard` (3, "Deferred batch").
    held: Option<BTreeMap<String, Option<Vec<u8>>>>,
}

impl<S: State> Metered<S> {
    pub fn new(inner: S) -> Self {
        Metered {
            inner,
            open: false,
            seen: HashMap::new(),
            change: Change::default(),
            held: None,
        }
    }

    /// Holds every write that follows, so the call can still be dropped whole.
    pub fn hold(&mut self) {
        self.held = Some(BTreeMap::new());
    }

    /// Applies the held writes and goes on writing through.
    pub fn commit(&mut self) {
        for (k, v) in self.held.take().unwrap_or_default() {
            match v {
                Some(v) => {
                    let _ = self.inner.put(&k, v);
                }
                None => self.inner.delete(&k),
            }
        }
    }

    /// Drops the held writes: the state is as it was when `hold` ran.
    pub fn discard(&mut self) {
        self.held = None;
    }

    /// A read that sees the held writes; the inner state is read as ever, so the reads are counted.
    fn read(&mut self, key: &str) -> Option<Vec<u8>> {
        let v = self.inner.get(key);
        match self.held.as_ref().and_then(|h| h.get(key)) {
            Some(w) => w.clone(),
            None => v,
        }
    }

    fn write(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        match &mut self.held {
            Some(h) => {
                if value.len() > MAX_VALUE {
                    return Err(TooLarge(MAX_VALUE));
                }
                h.insert(key.to_string(), Some(value));
                Ok(())
            }
            None => self.inner.put(key, value),
        }
    }

    fn remove(&mut self, key: &str) {
        match &mut self.held {
            Some(h) => {
                h.insert(key.to_string(), None);
            }
            None => self.inner.delete(key),
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
        self.remove(key);
    }

    fn old_size(&mut self, key: &str) -> Option<usize> {
        if let Some(s) = self.seen.get(key) {
            return *s;
        }
        let old = self.read(key).map(|v| v.len());
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
        let v = self.read(key);
        if self.metered(key) {
            self.seen
                .insert(key.to_string(), v.as_ref().map(|v| v.len()));
        }
        v
    }

    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        if !self.metered(key) {
            return self.write(key, value);
        }
        let old = self.old_size(key);
        let new = value.len();
        self.write(key, value)?;
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
        self.remove(key);
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
