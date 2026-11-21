//! The Claude adapter: stream-json lines and host items to updates and entries (TECHNICAL 9.8, 9.4, 4.5, 9.6).

use std::collections::HashMap;

use agent_client_protocol_schema::v1::{
    ContentBlock, ContentChunk, Diff, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus,
    SessionUpdate, ToolCall, ToolCallContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    ToolKind,
};
use claude_codes::io::ContentBlock as ClaudeBlock;
use claude_codes::io::ToolResultContent;
use claude_codes::ClaudeOutput;
use serde_json::{json, Map, Value};

use super::{cut_output, unknown, Entry, Kind, Out, Resolved};

/// Turns one Claude stream item into `Out`s. Its only state is in memory.
#[derive(Debug, Default)]
pub struct ClaudeAdapter {
    tools: HashMap<String, (String, Value)>,
    result_seen: bool,
    idle_seen: bool,
    /// Asks and checks the host has not resolved yet, by id, as their entry body.
    open: HashMap<String, Value>,
    /// Whether the stream is the host's own (`true`) or a checked provider's (10).
    direct: bool,
    /// Thinking of the step so far (9.4 `thought`).
    thinking: Thinking,
}

#[derive(Debug, Default)]
struct Thinking {
    /// Texts of finished `thinking` blocks.
    blocks: Vec<String>,
    /// Deltas since the last block (a block repeats what its deltas streamed).
    deltas: String,
    first_at: u64,
    last_at: u64,
    any: bool,
}

impl Thinking {
    fn touch(&mut self, at: u64) {
        if !self.any {
            self.first_at = at;
            self.any = true;
        }
        self.last_at = at;
    }

    fn take(&mut self) -> Option<Value> {
        let t = std::mem::take(self);
        if !t.any {
            return None;
        }
        let mut parts = t.blocks;
        if !t.deltas.is_empty() {
            parts.push(t.deltas);
        }
        let ms = t.last_at.saturating_sub(t.first_at);
        Some(json!({"text": parts.join("\n\n"), "seconds": (ms + 500) / 1000}))
    }
}

fn update(u: SessionUpdate) -> Option<Value> {
    serde_json::to_value(u).ok()
}

fn chunk(text: &str, thought: bool) -> Option<Value> {
    let c = ContentChunk::new(ContentBlock::from(text));
    update(if thought {
        SessionUpdate::AgentThoughtChunk(c)
    } else {
        SessionUpdate::AgentMessageChunk(c)
    })
}

fn only(update: Option<Value>) -> Out {
    Out {
        update,
        ..Out::default()
    }
}

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

impl ClaudeAdapter {
    /// A stream the host feeds directly.
    pub fn new() -> Self {
        ClaudeAdapter {
            direct: true,
            ..Self::default()
        }
    }

    /// The checked provider's stream (10): the host's `answered` may be the judge's.
    pub fn checked() -> Self {
        Self::default()
    }

    /// Adapt one line (a stdout line, or a host item as JSON) seen at `at` ms since the epoch.
    pub fn line(&mut self, line: &[u8], at: u64) -> Vec<Out> {
        let mut outs = self.item(line, at);
        for out in &mut outs {
            if let Some(Value::Object(u)) = out.update.as_mut() {
                let meta = u.entry("_meta").or_insert_with(|| json!({}));
                if let Some(m) = meta.as_object_mut() {
                    let w = m.entry("watane").or_insert_with(|| json!({}));
                    if let Some(w) = w.as_object_mut() {
                        w.insert("at".into(), json!(at));
                    }
                }
            }
        }
        outs
    }

    fn item(&mut self, line: &[u8], at: u64) -> Vec<Out> {
        let text = match std::str::from_utf8(line) {
            Ok(t) => t,
            Err(_) => return vec![unknown(Value::String(escape_invalid(line)))],
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return vec![];
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            return vec![unknown(Value::String(trimmed.to_string()))];
        };
        if has_duplicate_key(trimmed) {
            return vec![unknown(Value::String(trimmed.to_string()))];
        }
        let ty = str_of(&value, "type");
        match ty {
            Some("initialized") | Some("tool_progress") | Some("task_progress") => return vec![],
            Some("ask") => return self.ask(&value, "ask"),
            Some("check") => return self.ask(&value, "check"),
            Some("ask-resolved") => return self.resolved(&value),
            Some("wrapped") | Some("verdict") | Some("checking") => return vec![unknown(value)],
            Some("system")
                if matches!(
                    str_of(&value, "subtype"),
                    Some("hook_started" | "hook_response" | "tool_progress" | "task_progress")
                ) =>
            {
                return vec![]
            }
            _ => {}
        }
        match serde_json::from_value::<ClaudeOutput>(value.clone()) {
            Ok(output) => self.output(output, &value, at),
            Err(_) => vec![unknown(value)],
        }
    }

    fn output(&mut self, output: ClaudeOutput, raw: &Value, at: u64) -> Vec<Out> {
        match output {
            ClaudeOutput::Assistant(a) => {
                let uuid = a.uuid;
                a.message
                    .content
                    .into_iter()
                    .filter_map(|b| self.block(b, uuid.as_deref(), at))
                    .collect()
            }
            ClaudeOutput::User(u) => u
                .message
                .content
                .into_iter()
                .filter_map(|b| match b {
                    ClaudeBlock::ToolResult(r) => Some(self.tool_result(r, u.uuid.as_deref())),
                    _ => None,
                })
                .collect(),
            ClaudeOutput::StreamEvent(s) => self.stream_event(&s.event, at).into_iter().collect(),
            ClaudeOutput::System(s) => self.system(&s.data, raw),
            ClaudeOutput::Result(_) => self.result(raw),
            ClaudeOutput::RateLimitEvent(_) => vec![],
            _ => vec![unknown(raw.clone())],
        }
    }

    fn block(&mut self, block: ClaudeBlock, uuid: Option<&str>, at: u64) -> Option<Out> {
        match block {
            ClaudeBlock::Text(t) if !t.text.is_empty() => {
                let mut body =
                    json!({"role": "agent", "content": [{"type": "text", "text": t.text}]});
                if let Some(thought) = self.thinking.take() {
                    body["thought"] = thought;
                }
                Some(Out {
                    update: chunk(&t.text, false),
                    entry: Some(Entry {
                        kind: Kind::Message,
                        uuid: uuid.map(str::to_string),
                        body,
                    }),
                    ..Out::default()
                })
            }
            ClaudeBlock::Text(_) => None,
            ClaudeBlock::Thinking(t) if !t.thinking.is_empty() => {
                // A block repeats what its deltas streamed: it adds no time and is not pushed again.
                let streamed = !self.thinking.deltas.is_empty();
                if !streamed {
                    self.thinking.touch(at);
                }
                self.thinking.deltas.clear();
                self.thinking.blocks.push(t.thinking.clone());
                (!streamed).then(|| only(chunk(&t.thinking, true)))
            }
            ClaudeBlock::Thinking(_) => None,
            ClaudeBlock::ToolUse(t) => {
                self.tools
                    .insert(t.id.clone(), (t.name.clone(), t.input.clone()));
                if t.name == "TodoWrite" {
                    return Some(only(update(SessionUpdate::Plan(todo_plan(&t.input)))));
                }
                let call = ToolCall::new(t.id, t.name)
                    .status(ToolCallStatus::Pending)
                    .raw_input(t.input);
                let mut upd = update(SessionUpdate::ToolCall(call));
                if let Some(Value::Object(o)) = upd.as_mut() {
                    // ACP leaves a default status out; 9.8 names `pending`.
                    o.insert("status".into(), json!("pending"));
                }
                Some(only(upd))
            }
            ClaudeBlock::Unknown(v) if str_of(&v, "type") == Some("redacted_thinking") => None,
            ClaudeBlock::Unknown(v) => Some(unknown(v)),
            other => Some(unknown(serde_json::to_value(other).unwrap_or(Value::Null))),
        }
    }

    fn tool_result(&mut self, r: claude_codes::io::ToolResultBlock, uuid: Option<&str>) -> Out {
        let text = match r.content {
            None => String::new(),
            Some(ToolResultContent::Text(s)) => s,
            Some(ToolResultContent::Structured(items)) => items
                .iter()
                .filter_map(|i| str_of(i, "text"))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        let (text, cut) = cut_output(&text);
        let status = if r.is_error == Some(true) {
            ToolCallStatus::Failed
        } else {
            ToolCallStatus::Completed
        };
        let content = vec![agent_client_protocol_schema::v1::ToolCallContent::from(
            text,
        )];
        let (title, input) = match self.tools.remove(&r.tool_use_id) {
            Some((n, i)) => (Value::String(n), i),
            None => (Value::Null, Value::Null),
        };
        let mut meta = None;
        let mut body = json!({
            "toolCallId": r.tool_use_id,
            "title": title,
            "rawInput": input,
            "status": status,
            "content": content,
        });
        if let Some(n) = cut {
            let mut m = Map::new();
            m.insert("watane".into(), json!({ "cut": n }));
            body["_meta"] = Value::Object(m.clone());
            meta = Some(m);
        }
        let fields = ToolCallUpdateFields::new().status(status).content(content);
        let upd = ToolCallUpdate::new(r.tool_use_id, fields).meta(meta);
        Out {
            update: update(SessionUpdate::ToolCallUpdate(upd)),
            entry: Some(Entry {
                kind: Kind::Tool,
                uuid: uuid.map(str::to_string),
                body,
            }),
            ..Out::default()
        }
    }

    fn system(&mut self, data: &Value, raw: &Value) -> Vec<Out> {
        match str_of(raw, "subtype") {
            Some("init") | Some("status") => vec![],
            Some("compact_boundary") => {
                let meta = data.get("compact_metadata").unwrap_or(&Value::Null);
                vec![Out {
                    entry: Some(Entry {
                        kind: Kind::Compaction,
                        uuid: None,
                        body: json!({
                            "trigger": meta.get("trigger").cloned().unwrap_or(Value::Null),
                            "pre-tokens": meta.get("pre_tokens").cloned().unwrap_or(Value::Null),
                        }),
                    }),
                    ..Out::default()
                }]
            }
            Some("session_state_changed") => {
                if str_of(data, "state") == Some("idle") {
                    self.idle_seen = true;
                    if self.take_idle() {
                        vec![Out {
                            idle: true,
                            ..Out::default()
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    self.result_seen = false;
                    self.idle_seen = false;
                    vec![]
                }
            }
            _ => vec![unknown(raw.clone())],
        }
    }

    /// True when both `result` and idle have been seen this turn; ends the turn.
    fn take_idle(&mut self) -> bool {
        let both = self.result_seen && self.idle_seen;
        if both {
            self.result_seen = false;
            self.idle_seen = false;
        }
        both
    }

    fn result(&mut self, raw: &Value) -> Vec<Out> {
        self.result_seen = true;
        self.thinking = Thinking::default();
        vec![Out {
            usage: raw.get("usage").cloned(),
            idle: self.take_idle(),
            ..Out::default()
        }]
    }

    fn ask(&mut self, v: &Value, kind: &str) -> Vec<Out> {
        let call_id = str_of(v, "tool_use_id").filter(|s| !s.is_empty());
        let id = v
            .get("id")
            .filter(|i| i.as_str().is_some_and(|s| !s.is_empty()));
        let tool = str_of(v, "tool");
        let expires = v.get("expires-at").filter(|e| e.as_u64().is_some());
        let (Some(call_id), Some(id), Some(tool), Some(expires)) = (call_id, id, tool, expires)
        else {
            return vec![unknown(v.clone())];
        };
        let Some(input) = v.get("input").filter(|i| i.is_object()).cloned() else {
            return vec![unknown(v.clone())];
        };
        if self.open.contains_key(&id.to_string()) {
            return vec![unknown(v.clone())];
        }
        let conflicts = self.open.values().any(|b| {
            str_of(b, "toolCallId") == Some(call_id)
                && (str_of(&b["toolCall"], "title") != Some(tool) || b["input"] != input)
        });
        if conflicts {
            return vec![unknown(v.clone())];
        }
        let mut tool_call = json!({
            "toolCallId": call_id, "title": tool, "kind": tool_kind(tool), "rawInput": input,
        });
        if let Some(diffs) = diffs_of(tool, &input) {
            tool_call["content"] = json!(diffs);
        }
        let body = json!({
            "id": id, "kind": kind, "toolCallId": call_id,
            "toolCall": tool_call,
            "input": input, "expires-at": expires,
        });
        self.open.insert(id.to_string(), body.clone());
        vec![ask_entry(body)]
    }

    fn resolved(&mut self, v: &Value) -> Vec<Out> {
        let id = v
            .get("id")
            .and_then(Value::as_str)
            .filter(|i| !i.is_empty());
        let (Some(id), Some(by)) = (id, str_of(v, "by")) else {
            return vec![unknown(v.clone())];
        };
        // `ask` keys `open` by the id's JSON text (quotes included).
        let key = Value::String(id.to_string()).to_string();
        let Some(body) = self.open.get(&key) else {
            return vec![unknown(v.clone())];
        };
        let is_check = str_of(body, "kind") == Some("check");
        let outcome = match by {
            "answered" => match str_of(v, "outcome") {
                Some("none") if !is_check => return vec![unknown(v.clone())],
                Some("allow") if is_check => return vec![unknown(v.clone())],
                Some(o @ ("allow" | "deny" | "none")) => Some(o.to_string()),
                _ => return vec![unknown(v.clone())],
            },
            "expired" | "cancelled" if v.get("outcome").is_none() => None,
            _ => return vec![unknown(v.clone())],
        };
        self.open.remove(&key);
        let id = id.to_string();
        vec![Out {
            resolved: Some(Resolved {
                id,
                by: by.to_string(),
                outcome,
                direct: self.direct,
                raw: v.clone(),
            }),
            ..Out::default()
        }]
    }

    fn stream_event(&mut self, event: &Value, at: u64) -> Option<Out> {
        if str_of(event, "type") != Some("content_block_delta") {
            return None;
        }
        let delta = event.get("delta")?;
        match str_of(delta, "type")? {
            "text_delta" => Some(only(chunk(str_of(delta, "text")?, false))),
            "thinking_delta" => {
                let t = str_of(delta, "thinking")?;
                self.thinking.touch(at);
                self.thinking.deltas.push_str(t);
                Some(only(chunk(t, true)))
            }
            _ => None,
        }
    }
}

/// The line with each byte that is not valid UTF-8 written as `\xNN` (no lossy decoding).
fn escape_invalid(line: &[u8]) -> String {
    let mut out = String::new();
    for chunk in line.utf8_chunks() {
        out.push_str(chunk.valid());
        for b in chunk.invalid() {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out.trim().to_string()
}

/// True when `text` is one JSON object with a key twice at its top level (4.5).
fn has_duplicate_key(text: &str) -> bool {
    use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = bool;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an object")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<bool, A::Error> {
            let mut seen = std::collections::HashSet::new();
            let mut dup = false;
            while let Some(k) = m.next_key::<String>()? {
                dup |= !seen.insert(k);
                m.next_value::<IgnoredAny>()?;
            }
            Ok(dup)
        }
    }
    serde_json::Deserializer::from_str(text)
        .deserialize_map(V)
        .unwrap_or(false)
}

/// The ACP `ToolKind` of a Claude tool name (9.8).
fn tool_kind(tool: &str) -> ToolKind {
    match tool {
        "Read" => ToolKind::Read,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => ToolKind::Edit,
        "Bash" => ToolKind::Execute,
        "Grep" | "Glob" => ToolKind::Search,
        "WebFetch" | "WebSearch" => ToolKind::Fetch,
        _ => ToolKind::Other,
    }
}

/// The ACP diffs an `Edit`, `Write` or `MultiEdit` input describes; `None` for any other tool or a bad input (9.8).
fn diffs_of(tool: &str, input: &Value) -> Option<Vec<ToolCallContent>> {
    let path = str_of(input, "file_path")?;
    let diff = |old: Option<&str>, new: &str| {
        let mut d = Diff::new(path, new);
        d.old_text = old.map(str::to_string);
        ToolCallContent::Diff(d)
    };
    match tool {
        "Edit" if input.get("replace_all") == Some(&Value::Bool(true)) => None,
        "Edit" => Some(vec![diff(
            Some(str_of(input, "old_string")?),
            str_of(input, "new_string")?,
        )]),
        "Write" => Some(vec![diff(None, str_of(input, "content")?)]),
        "MultiEdit" => input
            .get("edits")?
            .as_array()?
            .iter()
            .map(|e| {
                if e.get("replace_all") == Some(&Value::Bool(true)) {
                    return None;
                }
                Some(diff(
                    Some(str_of(e, "old_string")?),
                    str_of(e, "new_string")?,
                ))
            })
            .collect(),
        _ => None,
    }
}

fn ask_entry(body: Value) -> Out {
    Out {
        entry: Some(Entry {
            kind: Kind::Ask,
            uuid: None,
            body,
        }),
        ..Out::default()
    }
}

fn todo_plan(input: &Value) -> Plan {
    let entries = input
        .get("todos")
        .and_then(Value::as_array)
        .map(|todos| {
            todos
                .iter()
                .map(|t| {
                    let status = match str_of(t, "status") {
                        Some("in_progress") => PlanEntryStatus::InProgress,
                        Some("completed") => PlanEntryStatus::Completed,
                        _ => PlanEntryStatus::Pending,
                    };
                    PlanEntry::new(
                        str_of(t, "content").unwrap_or_default(),
                        PlanEntryPriority::Medium,
                        status,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    Plan::new(entries)
}
