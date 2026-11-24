//! What the host's `https-stream` gives the loop, classified (TECHNICAL 9.5 Errors; S40-2).
//!
//! Pure functions: the first item, each server-sent event, an error body and the way the stream
//! ended become what the loop does next. Times are milliseconds since the Unix epoch.

use serde_json::Value;

/// The first item of a stream: the response's status and whether its body is an event stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub status: u16,
    pub sse: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The plan's usage limit is reached; `until` is when it resets (ms), when known.
    UsageLimit {
        until: Option<u64>,
    },
    Retryable(String),
    Fatal(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sse {
    Event(Value),
    Item(Value),
    Completed { usage: Usage, output: Vec<Value> },
    Failed(Failure),
    Unreadable(String),
}

pub const MAX_ATTEMPTS: u32 = 6;

/// Reads `{"status":200,"headers":{...}}`; anything else is `None`.
pub fn head(item: &[u8]) -> Option<Head> {
    let v: Value = serde_json::from_slice(item).ok()?;
    let status = u16::try_from(v.get("status")?.as_u64()?).ok()?;
    let sse = v
        .get("headers")
        .and_then(Value::as_object)
        .map(|h| {
            h.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("content-type")
                    && v.as_str().is_some_and(|s| {
                        s.split(';')
                            .next()
                            .is_some_and(|m| m.trim().eq_ignore_ascii_case("text/event-stream"))
                    })
            })
        })
        .unwrap_or(false);
    Some(Head { status, sse })
}

/// Reads the data of one server-sent event.
pub fn sse(item: &[u8], now: u64) -> Sse {
    let v: Value = match serde_json::from_slice(item) {
        Ok(v @ Value::Object(_)) => v,
        _ => return Sse::Unreadable("the event is not a JSON object".into()),
    };
    match v.get("type").and_then(Value::as_str) {
        Some("response.output_item.done") => match v.get("item") {
            Some(i) => Sse::Item(i.clone()),
            None => Sse::Event(v),
        },
        Some("response.completed") => {
            let r = v.get("response");
            let n = |k: &str| {
                r.and_then(|r| r.get("usage"))
                    .and_then(|u| u.get(k))
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            };
            let usage = Usage {
                input_tokens: n("input_tokens"),
                output_tokens: n("output_tokens"),
            };
            let output = r
                .and_then(|r| r.get("output"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            Sse::Completed { usage, output }
        }
        Some("response.failed") => {
            let e = v.get("response").and_then(|r| r.get("error"));
            Sse::Failed(event_failure(e, "the response failed", now))
        }
        Some("error") => {
            // The error is in `error`, or the event itself carries `code` and `message`.
            let e = v.get("error").or(Some(&v));
            Sse::Failed(event_failure(e, "the stream reported an error", now))
        }
        _ => Sse::Event(v),
    }
}

fn event_failure(e: Option<&Value>, dflt: &str, now: u64) -> Failure {
    if let Some(e) = e {
        if is_usage_limit(e) {
            return Failure::UsageLimit {
                until: until(e, now),
            };
        }
    }
    let msg = e
        .and_then(|e| text(e, "message").or_else(|| text(e, "code")))
        .unwrap_or(dflt);
    Failure::Fatal(msg.to_string())
}

fn text<'a>(e: &'a Value, k: &str) -> Option<&'a str> {
    e.get(k).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn is_usage_limit(e: &Value) -> bool {
    ["type", "code"]
        .iter()
        .any(|k| e.get(k).and_then(Value::as_str) == Some("usage_limit_reached"))
}

/// When the limit resets, in ms: `resets_at` (Unix seconds), else `now` plus `resets_in_seconds`.
fn until(e: &Value, now: u64) -> Option<u64> {
    if let Some(at) = e.get("resets_at").and_then(Value::as_u64) {
        return Some(at.saturating_mul(1000));
    }
    let s = e.get("resets_in_seconds").and_then(Value::as_u64)?;
    Some(now.saturating_add(s.saturating_mul(1000)))
}

/// A response whose status is not 2xx: its whole body (the 64 KiB chunks joined).
pub fn error_body(status: u16, body: &[u8], now: u64) -> Failure {
    let v: Option<Value> = serde_json::from_slice(body).ok();
    let e = v.as_ref().and_then(|v| v.get("error"));
    if status == 429 {
        if let Some(e) = e.filter(|e| is_usage_limit(e)) {
            return Failure::UsageLimit {
                until: until(e, now),
            };
        }
        return Failure::Retryable(format!("status 429: too many requests{}", detail(e)));
    }
    if (500..600).contains(&status) {
        return Failure::Retryable(format!("status {status}{}", detail(e)));
    }
    Failure::Fatal(format!("status {status}{}", detail(e)))
}

fn detail(e: Option<&Value>) -> String {
    match e.and_then(|e| text(e, "message")) {
        Some(m) => format!(": {m}"),
        None => String::new(),
    }
}

/// How the stream ended (`cause` as `map::ended_cause` gives it; `None` is `closed`).
pub fn ended(cause: Option<&str>, completed: bool) -> Option<Failure> {
    match cause {
        None if completed => None,
        None => Some(Failure::Fatal(
            "the response ended before it completed".into(),
        )),
        Some("no-account") => Some(Failure::Fatal("not signed in to ChatGPT".into())),
        Some("no-app") => Some(Failure::Fatal("no app to sign in to ChatGPT".into())),
        Some(c) if c == "idle-timeout" || c.starts_with("failed:") => {
            Some(Failure::Retryable(c.to_string()))
        }
        Some(c) => Some(Failure::Fatal(format!("the stream ended: {c}"))),
    }
}

/// Wait before retry `attempt` (1-based): 1 s doubling, at most 60 s; `None` once retries ran out.
pub fn backoff_ms(attempt: u32) -> Option<u64> {
    if attempt == 0 || attempt > MAX_ATTEMPTS {
        return None;
    }
    Some((1_000u64 << (attempt - 1)).min(60_000))
}
