//! The Claude adapter: stream-json lines and host items to updates and entries (TECHNICAL 9.8, 9.4, 4.5, 9.6).

use std::collections::HashMap;

use agent_client_protocol_schema::v1::{
    ContentBlock, ContentChunk, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, SessionUpdate,
    ToolCall, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use claude_codes::io::ContentBlock as ClaudeBlock;
use claude_codes::io::ToolResultContent;
use claude_codes::ClaudeOutput;
use serde_json::{json, Map, Value};

use super::{cut_output, unknown, Entry, Kind, Out};

/// A Claude ask lives 10 minutes in the host (9.6).
const ASK_TTL_MS: u64 = 600_000;

/// Turns one Claude stream item into `Out`s. Its only state is in memory.
#[derive(Debug, Default)]
pub struct ClaudeAdapter {
    tools: HashMap<String, (String, Value)>,
    result_seen: bool,
    idle_seen: bool,
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Adapt one line (a stdout line, or a host item as JSON) seen at `at` ms since the epoch.
    pub fn line(&mut self, line: &[u8], at: u64) -> Vec<Out> {
        let text = String::from_utf8_lossy(line);
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return vec![];
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            return vec![unknown(Value::String(trimmed.to_string()))];
        };
        match str_of(&value, "type") {
            Some("initialized") => return vec![],
            Some("ask") => return self.ask(&value, "ask", at),
            Some("check") => return self.ask(&value, "check", at),
            Some("ask-resolved") => return self.resolved(&value),
            _ => {}
        }
        match serde_json::from_value::<ClaudeOutput>(value.clone()) {
            Ok(output) => self.output(output, &value),
            Err(_) => vec![unknown(value)],
        }
    }

    fn output(&mut self, output: ClaudeOutput, raw: &Value) -> Vec<Out> {
        match output {
            ClaudeOutput::Assistant(a) => {
                let uuid = a.uuid;
                a.message
                    .content
                    .into_iter()
                    .filter_map(|b| self.block(b, uuid.as_deref()))
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
            ClaudeOutput::StreamEvent(s) => stream_event(&s.event).into_iter().collect(),
            ClaudeOutput::System(s) => self.system(&s.data, raw),
            ClaudeOutput::Result(_) => self.result(raw),
            ClaudeOutput::RateLimitEvent(_) => vec![],
            _ => vec![unknown(raw.clone())],
        }
    }

    fn block(&mut self, block: ClaudeBlock, uuid: Option<&str>) -> Option<Out> {
        match block {
            ClaudeBlock::Text(t) if !t.text.is_empty() => Some(Out {
                update: chunk(&t.text, false),
                entry: Some(Entry {
                    kind: Kind::Message,
                    uuid: uuid.map(str::to_string),
                    body: json!({"role": "agent", "content": [{"type": "text", "text": t.text}]}),
                }),
                ..Out::default()
            }),
            ClaudeBlock::Text(_) => None,
            ClaudeBlock::Thinking(t) if !t.thinking.is_empty() => {
                Some(only(chunk(&t.thinking, true)))
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
        let content = vec![agent_client_protocol_schema::v1::ToolCallContent::from(text)];
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
        vec![Out {
            usage: raw.get("usage").cloned(),
            idle: self.take_idle(),
            ..Out::default()
        }]
    }

    fn ask(&mut self, v: &Value, kind: &str, at: u64) -> Vec<Out> {
        let (Some(id), Some(tool)) = (v.get("id").filter(|i| !i.is_null()), str_of(v, "tool"))
        else {
            return vec![unknown(v.clone())];
        };
        let input = v.get("input").cloned().unwrap_or(Value::Null);
        let call_id = v.get("tool_use_id").cloned().unwrap_or(Value::Null);
        let is_ask = kind == "ask";
        let options = if is_ask {
            json!([
                {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
                {"optionId": "allow-session", "name": "Allow for this session", "kind": "allow_always"},
                {"optionId": "deny", "name": "Deny", "kind": "reject_once"}
            ])
        } else {
            json!([
                {"optionId": "none", "name": "No objection", "kind": "allow_once"},
                {"optionId": "deny", "name": "Deny", "kind": "reject_once"}
            ])
        };
        let mut item = json!({
            "type": "ask", "id": id, "kind": kind,
            "toolCall": {"toolCallId": call_id, "title": tool, "rawInput": input},
            "options": options,
        });
        let mut body = json!({
            "id": id, "kind": kind, "tool-call-id": call_id, "tool": tool, "input": input,
        });
        if is_ask {
            item["expires-at"] = json!(at.saturating_add(ASK_TTL_MS));
            body["expires-at"] = json!(at.saturating_add(ASK_TTL_MS));
        }
        vec![Out {
            watane: Some(item),
            entry: Some(Entry {
                kind: Kind::Ask,
                uuid: None,
                body,
            }),
            ..Out::default()
        }]
    }

    fn resolved(&mut self, v: &Value) -> Vec<Out> {
        let (Some(id), Some(by)) = (v.get("id").filter(|i| !i.is_null()), v.get("by")) else {
            return vec![unknown(v.clone())];
        };
        vec![Out {
            watane: Some(json!({"type": "ask-resolved", "id": id, "by": by})),
            entry: Some(Entry {
                kind: Kind::Ask,
                uuid: None,
                body: json!({"id": id, "by": by}),
            }),
            ..Out::default()
        }]
    }
}

fn stream_event(event: &Value) -> Option<Out> {
    if str_of(event, "type") != Some("content_block_delta") {
        return None;
    }
    let delta = event.get("delta")?;
    match str_of(delta, "type")? {
        "text_delta" => Some(only(chunk(str_of(delta, "text")?, false))),
        "thinking_delta" => Some(only(chunk(str_of(delta, "thinking")?, true))),
        _ => None,
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
