//! The Responses adapter: streaming events and tool job outputs to updates and entries (TECHNICAL 9.8, 9.5, 9.4).
//!
//! Stateless: a job output comes with its `function_call` item.

use agent_client_protocol_schema::v1::{
    ContentBlock, ContentChunk, SessionUpdate, ToolCall, ToolCallContent, ToolCallStatus,
    ToolCallUpdate, ToolCallUpdateFields,
};
use serde_json::{json, Map, Value};

use super::{cut_output, unknown, Entry, Kind, Out};

/// Events the loop (9.5) reads or that carry nothing a client shows.
const NO_ROW: &[&str] = &[
    "response.created",
    "response.in_progress",
    "response.completed",
    "response.failed",
    "response.incomplete",
    "error",
    "response.output_item.added",
    "response.content_part.added",
    "response.content_part.done",
    "response.output_text.done",
    "response.reasoning_summary_part.added",
    "response.reasoning_summary_part.done",
    "response.reasoning_summary_text.done",
    "response.function_call_arguments.delta",
    "response.function_call_arguments.done",
];

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

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// Feed one parsed streaming event seen at `at`. Never fails: a stranger is an `unknown` entry.
pub fn event(event: &Value, at: u64) -> Vec<Out> {
    let _ = at;
    let Some(ty) = str_of(event, "type") else {
        return vec![unknown(event.clone())];
    };
    if NO_ROW.contains(&ty) {
        return vec![];
    }
    let known = match ty {
        "response.output_text.delta" => str_of(event, "delta").map(|d| text_delta(d, false)),
        "response.reasoning_summary_text.delta" => {
            str_of(event, "delta").map(|d| text_delta(d, true))
        }
        "response.output_item.done" => event.get("item").and_then(item_done),
        _ => None,
    };
    known.unwrap_or_else(|| vec![unknown(event.clone())])
}

fn text_delta(delta: &str, thought: bool) -> Vec<Out> {
    if delta.is_empty() {
        return vec![];
    }
    vec![Out {
        update: chunk(delta, thought),
        ..Out::default()
    }]
}

/// `None` when the item has the wrong shape for its type.
fn item_done(item: &Value) -> Option<Vec<Out>> {
    match str_of(item, "type")? {
        "message" => {
            if str_of(item, "role")? != "assistant" {
                return None;
            }
            let mut text = String::new();
            for part in item.get("content")?.as_array()? {
                match str_of(part, "type")? {
                    "output_text" => text.push_str(str_of(part, "text")?),
                    "refusal" => text.push_str(str_of(part, "refusal")?),
                    _ => return None,
                }
            }
            if text.is_empty() {
                return Some(vec![]);
            }
            Some(vec![Out {
                update: chunk(&text, false),
                entry: Some(Entry {
                    kind: Kind::Message,
                    uuid: None,
                    body: json!({"role": "agent", "content": [{"type": "text", "text": text}]}),
                }),
                ..Out::default()
            }])
        }
        "reasoning" => Some(vec![]),
        // A server compaction (9.5 "Compaction items"): the entry marks it.
        "compaction" => Some(vec![Out {
            entry: Some(Entry {
                kind: Kind::Compaction,
                uuid: None,
                body: json!({}),
            }),
            ..Out::default()
        }]),
        "function_call" => {
            let (id, name, input) = call_parts(item)?;
            let call = ToolCall::new(id.to_string(), name.to_string())
                .status(ToolCallStatus::Pending)
                .raw_input(input);
            let mut upd = update(SessionUpdate::ToolCall(call));
            if let Some(Value::Object(o)) = upd.as_mut() {
                // ACP leaves a default status out; 9.8 names `pending`.
                o.insert("status".into(), json!("pending"));
            }
            Some(vec![Out {
                update: upd,
                ..Out::default()
            }])
        }
        _ => None,
    }
}

/// `call_id`, `name` and the parsed `arguments` (the string when it is not JSON).
fn call_parts(item: &Value) -> Option<(&str, &str, Value)> {
    let args = str_of(item, "arguments")?;
    let input = serde_json::from_str(args).unwrap_or_else(|_| Value::String(args.to_string()));
    Some((str_of(item, "call_id")?, str_of(item, "name")?, input))
}

/// A tool job's `output` for the `function_call` item `call`, finished at `at`.
pub fn job_output(call: &Value, output: &str, ok: bool, at: u64) -> Vec<Out> {
    let _ = at;
    let Some((id, name, input)) = call_parts(call) else {
        return vec![unknown(json!({"call": call, "output": output, "ok": ok}))];
    };
    let (text, cut) = cut_output(output);
    let status = if ok {
        ToolCallStatus::Completed
    } else {
        ToolCallStatus::Failed
    };
    let content = vec![ToolCallContent::from(text)];
    let mut meta = None;
    let mut body = json!({
        "toolCallId": id,
        "title": name,
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
    let upd = ToolCallUpdate::new(id.to_string(), fields).meta(meta);
    vec![Out {
        update: update(SessionUpdate::ToolCallUpdate(upd)),
        entry: Some(Entry {
            kind: Kind::Tool,
            uuid: None,
            body,
        }),
        ..Out::default()
    }]
}
