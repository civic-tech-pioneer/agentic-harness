//! Adapters from a program's own stream to ACP v1 updates and transcript entries (TECHNICAL 9.8).
//!
//! Pure: no state I/O, no clock, no host. The caller owns the adapter and passes the time.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod claude;
pub mod responses;

/// The largest tool output kept whole, in bytes (9.4).
pub const OUTPUT_CUT: usize = 65_536;

fn is_false(b: &bool) -> bool {
    !*b
}

/// What one stream item turns into. Storage adds `v`, `i`, `at` to the entry (9.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Out {
    /// An ACP v1 `SessionUpdate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<Value>,
    /// Goes under `_meta.watane`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watane: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Entry>,
    /// For the counters (9.7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Value>,
    /// `idle {completed}`.
    #[serde(skip_serializing_if = "is_false")]
    pub idle: bool,
}

/// An entry of 9.4 without `v`, `i`, `at`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    pub body: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Message,
    Tool,
    Ask,
    Compaction,
    Unknown,
}

/// The `unknown` entry: body `{"raw": raw}`.
pub fn unknown(raw: Value) -> Out {
    Out {
        entry: Some(Entry {
            kind: Kind::Unknown,
            uuid: None,
            body: json!({ "raw": raw }),
        }),
        ..Out::default()
    }
}

/// Cut `text` at a UTF-8 boundary at or under 64 KiB; the original byte length when cut.
pub fn cut_output(text: &str) -> (String, Option<usize>) {
    if text.len() <= OUTPUT_CUT {
        return (text.to_string(), None);
    }
    let mut end = OUTPUT_CUT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), Some(text.len()))
}
