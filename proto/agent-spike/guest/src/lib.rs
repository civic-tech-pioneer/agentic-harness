//! Agent spike guest: the OpenAI tool loop and the Claude Code line mapper, as one
//! wasm32-wasip2 component. Assembled from wstd, sse-stream, claude-codes and the ACP schema.

use agent_client_protocol_schema::v1::{
    ContentChunk, SessionUpdate, ToolCall, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use wstd::http::{Body, Client, Request};

wit_bindgen::generate!({ path: "../wit", world: "agent" });

use spike::agent::{events, tools};

struct Component;
export!(Component);

fn emit(update: SessionUpdate) {
    events::emit(&serde_json::to_string(&update).unwrap_or_default());
}

fn text(t: impl Into<String>) -> SessionUpdate {
    SessionUpdate::AgentMessageChunk(ContentChunk::new(t.into().into()))
}

fn done(id: &str, output: Value, failed: bool) -> SessionUpdate {
    let status = if failed { ToolCallStatus::Failed } else { ToolCallStatus::Completed };
    SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        id.to_owned(),
        ToolCallUpdateFields::new().status(status).raw_output(output),
    ))
}

impl Guest for Component {
    fn openai(model: String, prompt: String) -> Result<String, String> {
        wstd::runtime::block_on(openai(model, prompt)).map_err(|e| e.to_string())
    }

    fn probe(url: String, headers: Vec<(String, String)>) -> Result<u16, String> {
        wstd::runtime::block_on(async move {
            let mut request = Request::get(url);
            for (k, v) in headers {
                request = request.header(k, v);
            }
            let response = Client::new().send(request.body(Body::empty())?).await?;
            Ok::<_, Error>(response.status().as_u16())
        })
        .map_err(|e| format!("{e:?}"))
    }

    fn claude_line(line: String) -> Result<Vec<String>, String> {
        use claude_codes::{ClaudeOutput, ContentBlock as C};
        let out: ClaudeOutput = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        let mut updates = Vec::new();
        match out {
            ClaudeOutput::Assistant(m) => {
                for block in m.message.content {
                    match block {
                        C::Text(t) => updates.push(text(t.text)),
                        C::ToolUse(t) => updates.push(SessionUpdate::ToolCall(
                            ToolCall::new(t.id, t.name.clone()).name(t.name).raw_input(t.input),
                        )),
                        _ => {}
                    }
                }
            }
            ClaudeOutput::User(m) => {
                for block in m.message.content {
                    if let C::ToolResult(r) = block {
                        let output = serde_json::to_value(&r.content).unwrap_or(Value::Null);
                        updates.push(done(&r.tool_use_id, output, r.is_error == Some(true)));
                    }
                }
            }
            // ponytail: system/result/control lines carry no session update; the host reads the result line itself.
            _ => {}
        }
        Ok(updates.iter().map(|u| serde_json::to_string(u).unwrap_or_default()).collect())
    }
}

/// The tools the model may call. A watane mod would derive these from the WIT of its providers.
fn tool_namespace() -> Value {
    json!({
        "type": "namespace",
        "name": "spike",
        "description": "Stand-ins for calls into other watane mods.",
        "tools": [
            {"type": "function", "name": "add", "description": "Add two integers.",
             "parameters": {"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
                            "required": ["a", "b"], "additionalProperties": false}},
            {"type": "function", "name": "now", "description": "The current time on the participant's machine.",
             "parameters": {"type": "object", "properties": {}, "additionalProperties": false}}
        ]
    })
}

type Error = Box<dyn std::error::Error>;

async fn openai(model: String, prompt: String) -> Result<String, Error> {
    let mut input = vec![json!({"role": "user", "content": prompt})];
    // ponytail: 8 model turns at most; a real mod takes the bound from its caller.
    for _ in 0..8 {
        let body = json!({
            "model": model,
            "instructions": "You are a test agent inside a sandbox. Use the tools when they help.",
            "input": input,
            "tools": [tool_namespace()],
            "include": ["reasoning.encrypted_content"],
            "store": false,
            "stream": true,
        });
        let request = Request::post("https://api.openai.com/v1/responses")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body)?))?;
        let response = Client::new().send(request).await?;
        let status = response.status();
        let mut body = response.into_body();
        if !status.is_success() {
            return Err(format!("{status}: {}", body.str_contents().await?).into());
        }
        // sse-stream wants a std error; wstd's body has anyhow.
        let body = http_body_util::BodyExt::map_err(body.into_boxed_body(), |e| {
            std::io::Error::other(e.to_string())
        });
        let mut events = sse_stream::SseStream::new(body);
        let mut output = None;
        while let Some(event) = events.next().await {
            let event = event?;
            let Some(data) = event.data else { continue };
            let v: Value = serde_json::from_str(&data)?;
            match v["type"].as_str().unwrap_or_default() {
                "response.output_text.delta" => emit(text(v["delta"].as_str().unwrap_or_default())),
                "response.completed" => output = Some(v["response"]["output"].clone()),
                "response.failed" | "response.incomplete" | "error" => {
                    return Err(format!("{data}").into());
                }
                _ => {}
            }
        }
        let output = output.ok_or("stream ended without response.completed")?;
        let items = output.as_array().cloned().unwrap_or_default();
        input.extend(items.iter().cloned());
        let calls: Vec<&Value> = items.iter().filter(|i| i["type"] == "function_call").collect();
        if calls.is_empty() {
            let answer = items
                .iter()
                .filter(|i| i["type"] == "message")
                .flat_map(|i| i["content"].as_array().cloned().unwrap_or_default())
                .filter_map(|c| c["text"].as_str().map(str::to_owned))
                .collect::<String>();
            return Ok(answer);
        }
        for call in calls {
            let id = call["call_id"].as_str().unwrap_or_default();
            let name = call["name"].as_str().unwrap_or_default();
            let args = call["arguments"].as_str().unwrap_or("{}");
            emit(SessionUpdate::ToolCall(
                ToolCall::new(id.to_owned(), name.to_owned())
                    .name(name.to_owned())
                    .raw_input(serde_json::from_str::<Value>(args).unwrap_or(Value::Null)),
            ));
            let result = tools::call(name, args);
            let out = result.clone().unwrap_or_else(|e| e);
            emit(done(id, Value::String(out.clone()), result.is_err()));
            input.push(json!({"type": "function_call_output", "call_id": id, "output": out}));
        }
    }
    Err("no answer after 8 model turns".into())
}
