//! The session record: TECHNICAL 9.3 (record, sizes, turn state), 9.1 (`session/<id>`).

use crate::error::KernelError;
use crate::state::State;
use crate::transcript::Meta;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ID_BYTES: usize = 64;
pub const KEY_BYTES: usize = 256;
pub const TITLE_BYTES: usize = 256;
pub const RECORD_BYTES: usize = 524_288;
pub const ALLOWS: usize = 64;
pub const ALLOWS_BYTES: usize = 65_536;
pub const ALLOW_BYTES: usize = 1024;
pub const STREAMS: usize = 10;
pub const DEADLINES: usize = 64;
const CHUNKS: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Activity {
    Checking { on: String },
    Compacting,
    Consent { what: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Stop {
    Completed,
    Interrupted,
    Stopped { limit: String },
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
pub enum Turn {
    Running {
        since: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        activity: Option<Activity>,
    },
    Waiting {
        asks: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expires_at: Option<u64>,
    },
    Paused {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until: Option<u64>,
    },
    Idle {
        stop: Stop,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seconds: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tools: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens: Option<u64>,
    },
    Unknown,
    ResumeLost,
}

impl Default for Turn {
    fn default() -> Self {
        Turn::Idle {
            stop: Stop::Completed,
            seconds: None,
            tools: None,
            tokens: None,
        }
    }
}

impl Turn {
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).expect("a turn serializes")
    }

    /// The body of a `turn` entry: the JSON without `activity`.
    pub fn entry_body(&self) -> Value {
        let mut v = self.to_json();
        if let Some(o) = v.as_object_mut() {
            o.remove("activity");
        }
        v
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Turn::Running { .. } | Turn::Waiting { .. } | Turn::Paused { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deadline {
    pub what: String,
    pub at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Counters {
    pub turns: u64,
    pub tool_errors_in_a_row: u64,
    pub tokens: u64,
}

/// An open ask or check, with the index of its ask entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAsk {
    pub id: String,
    /// "ask" or "check".
    pub kind: String,
    pub i: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Record {
    pub id: String,
    pub owner: String,
    pub key: String,
    pub profile: String,
    pub backend: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub turn: Turn,
    pub boot: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_page: Option<u8>,
    pub allows: Vec<Value>,
    pub counters: Counters,
    pub chunks: Vec<u64>,
    pub streams: Vec<String>,
    pub deadlines: Vec<Deadline>,
    pub open: Vec<OpenAsk>,
    pub next_index: u64,
    pub created: u64,
    pub updated: u64,
}

impl Default for Record {
    fn default() -> Self {
        Record {
            id: String::new(),
            owner: String::new(),
            key: String::new(),
            profile: String::new(),
            backend: String::new(),
            title: String::new(),
            handle: None,
            workspace: None,
            claude_uuid: None,
            parent: None,
            children: Vec::new(),
            turn: Turn::default(),
            boot: 0,
            active_page: None,
            allows: Vec::new(),
            counters: Counters::default(),
            chunks: Vec::new(),
            streams: Vec::new(),
            deadlines: Vec::new(),
            open: Vec::new(),
            next_index: 1,
            created: 0,
            updated: 0,
        }
    }
}

fn allow_len(v: &Value) -> usize {
    v.to_string().len()
}

impl Record {
    pub fn key_of(id: &str) -> String {
        format!("session/{id}")
    }

    pub fn load(state: &mut impl State, id: &str) -> Option<Record> {
        let bytes = state.get(&Self::key_of(id))?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Writes nothing when the record is over 512 KiB.
    pub fn save(&self, state: &mut impl State) -> Result<(), KernelError> {
        let bytes = serde_json::to_vec(self).expect("a record serializes");
        if bytes.len() > RECORD_BYTES {
            return Err(KernelError::TooLarge(RECORD_BYTES as u64));
        }
        state
            .put(&Self::key_of(&self.id), bytes)
            .map_err(|t| KernelError::TooLarge(t.0 as u64))
    }

    pub fn meta(&self) -> Meta {
        Meta {
            chunks: self.chunks.clone(),
            last: self.next_index.saturating_sub(1),
        }
    }

    pub fn set_meta(&mut self, m: &Meta) {
        self.chunks = m.chunks.clone();
        self.next_index = m.last + 1;
    }

    pub fn add_allow(&mut self, allow: Value) -> Result<(), KernelError> {
        let n = allow_len(&allow);
        let total: usize = self.allows.iter().map(allow_len).sum();
        if self.allows.len() >= ALLOWS || n > ALLOW_BYTES || total + n > ALLOWS_BYTES {
            return Err(KernelError::TooLarge(ALLOWS_BYTES as u64));
        }
        self.allows.push(allow);
        Ok(())
    }

    pub fn add_stream(&mut self, id: String) -> Result<(), KernelError> {
        if self.streams.len() >= STREAMS {
            return Err(KernelError::Busy(format!("{STREAMS} streams per session")));
        }
        self.streams.push(id);
        Ok(())
    }

    pub fn add_deadline(&mut self, d: Deadline) -> Result<(), KernelError> {
        if self.deadlines.len() >= DEADLINES {
            return Err(KernelError::Busy(format!(
                "{DEADLINES} deadlines per session"
            )));
        }
        self.deadlines.push(d);
        Ok(())
    }

    /// At 16 384 chunk starts the session is at its quota (9.3, case 53).
    pub fn chunks_full(&self) -> bool {
        self.chunks.len() >= CHUNKS
    }
}

pub fn check_key(key: &str) -> Result<(), KernelError> {
    if key.len() > KEY_BYTES {
        return Err(KernelError::TooLarge(KEY_BYTES as u64));
    }
    Ok(())
}

pub fn check_title(title: &str) -> Result<(), KernelError> {
    if title.len() > TITLE_BYTES {
        return Err(KernelError::TooLarge(TITLE_BYTES as u64));
    }
    Ok(())
}
