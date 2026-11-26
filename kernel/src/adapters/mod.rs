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
    /// With `idle`: the turn's provider reported its own step limit, so the idle ends it
    /// `stopped {limit}` with this limit (9.7 "Limits").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    /// A `result`'s context meter push, `{"type":"usage",...}`: a sample, or the clear (9.7
    /// "Usage"). Live; the kernel also keeps the current sample for `attach`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter: Option<Value>,
    /// An `ask-resolved` the adapter checked; the kernel writes the decision (9.8 rule 4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<Resolved>,
    /// A checked provider's `checking` item of good shape: its `on`, as the provider wrote it
    /// (10 "Checking"). Live only; the kernel maps it to the turn's activity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checking: Option<String>,
}

/// A host `ask-resolved` item of good shape: what the adapter reports, the kernel decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolved {
    pub id: String,
    /// `answered`, `expired` or `cancelled`.
    pub by: String,
    /// Of `answered`: `allow`, `deny` or `none`.
    pub outcome: Option<String>,
    /// The stream is the host's own (`true`), or a checked provider's (`false`, 10).
    pub direct: bool,
    pub raw: Value,
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
    Verdict,
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
