//! The active index `active/<n>` and `slots` over host state (TECHNICAL 9.1, 9.3 A2-A3, R2).

use crate::error::KernelError;
use crate::state::State;
use serde::{Deserialize, Serialize};

pub const ACTIVE_PAGES: u8 = 4;
pub const PAGE_SESSIONS: u32 = 256;
pub const ACTIVE_MAX: u32 = 1024;
pub const PAGE_BYTES_MAX: usize = 24 * 1024;

const SLOTS_KEY: &str = "slots";

/// The `slots` document: counters, the boot number, page counts, the restart cursor.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Slots {
    pub provider: u32,
    pub request: u32,
    pub job: u32,
    pub timers: u32,
    pub boot: u64,
    pub active: [u32; 4],
    pub restart: Option<u8>,
    pub head: u64,
    pub tail: u64,
}

impl Slots {
    pub fn load(state: &mut impl State) -> Slots {
        state
            .get(SLOTS_KEY)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, state: &mut impl State) -> Result<(), KernelError> {
        put(state, SLOTS_KEY, self)
    }
}

/// One session on a page; stored as an `[id, boot]` pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "(String, u64)", into = "(String, u64)")]
pub struct PageEntry {
    pub id: String,
    pub boot: u64,
}

impl From<(String, u64)> for PageEntry {
    fn from((id, boot): (String, u64)) -> Self {
        PageEntry { id, boot }
    }
}

impl From<PageEntry> for (String, u64) {
    fn from(e: PageEntry) -> Self {
        (e.id, e.boot)
    }
}

fn put<T: Serialize>(state: &mut impl State, key: &str, v: &T) -> Result<(), KernelError> {
    let bytes = serde_json::to_vec(v).map_err(|e| KernelError::Backend(e.to_string()))?;
    state
        .put(key, bytes)
        .map_err(|e| KernelError::TooLarge(e.0 as u64))
}

pub fn page_key(n: u8) -> String {
    format!("active/{n}")
}

pub fn read_page(state: &mut impl State, n: u8) -> Vec<PageEntry> {
    state
        .get(&page_key(n))
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// The first page with room, or `busy`: lets a caller refuse before its first write (A3).
pub fn check_room(slots: &Slots) -> Result<u8, KernelError> {
    slots
        .active
        .iter()
        .position(|&c| c < PAGE_SESSIONS)
        .map(|p| p as u8)
        .ok_or_else(|| {
            KernelError::Busy(format!("the active index is full ({ACTIVE_MAX} sessions)"))
        })
}

/// Adds `{id, boot}` to the first page with room; returns that page.
pub fn add(
    state: &mut impl State,
    slots: &mut Slots,
    id: &str,
    boot: u64,
) -> Result<u8, KernelError> {
    let page = check_room(slots)?;
    let mut entries = read_page(state, page);
    entries.push(PageEntry {
        id: id.to_string(),
        boot,
    });
    put(state, &page_key(page), &entries)?;
    slots.active[page as usize] += 1;
    slots.save(state)?;
    Ok(page)
}

/// Drops `id` from `page` only.
pub fn remove(
    state: &mut impl State,
    slots: &mut Slots,
    id: &str,
    page: u8,
) -> Result<(), KernelError> {
    let mut entries = read_page(state, page);
    let Some(i) = entries.iter().position(|e| e.id == id) else {
        return Ok(());
    };
    entries.remove(i);
    put(state, &page_key(page), &entries)?;
    let c = &mut slots.active[page as usize];
    *c = c.saturating_sub(1);
    slots.save(state)
}

pub fn set_boot(state: &mut impl State, page: u8, id: &str, boot: u64) -> Result<(), KernelError> {
    let mut entries = read_page(state, page);
    if let Some(e) = entries.iter_mut().find(|e| e.id == id) {
        e.boot = boot;
        put(state, &page_key(page), &entries)?;
    }
    Ok(())
}

/// The next entry of an older boot at or after the cursor; the caller saves `slots`.
pub fn next_stale(state: &mut impl State, slots: &mut Slots) -> Option<(u8, PageEntry)> {
    let from = slots.restart?;
    for page in from..ACTIVE_PAGES {
        if slots.active[page as usize] == 0 {
            continue;
        }
        if let Some(e) = read_page(state, page)
            .into_iter()
            .find(|e| e.boot < slots.boot)
        {
            slots.restart = Some(page);
            return Some((page, e));
        }
    }
    slots.restart = None;
    None
}
