//! S40-2: the reading of what the host's `https-stream` gives the OpenAI loop (TECHNICAL 9.5).

use agent_kernel::openai::answer::*;
use serde_json::json;

const NOW: u64 = 1_000_000;

#[test]
fn ac1_head_reads_status_and_event_stream() {
    let h = |s: &str| head(s.as_bytes());
    assert_eq!(
        h(r#"{"status":200,"headers":{"content-type":"text/event-stream"}}"#),
        Some(Head {
            status: 200,
            sse: true
        })
    );
    assert_eq!(
        h(r#"{"status":200,"headers":{"Content-Type":"Text/Event-Stream; charset=utf-8"}}"#),
        Some(Head {
            status: 200,
            sse: true
        })
    );
    assert_eq!(
        h(r#"{"status":429,"headers":{"content-type":"application/json"}}"#),
        Some(Head {
            status: 429,
            sse: false
        })
    );
    assert_eq!(h("not json"), None);
    assert_eq!(h("[1]"), None);
    assert_eq!(h(r#"{"headers":{}}"#), None);
    assert_eq!(h(r#"{"status":"200"}"#), None);
}

#[test]
fn ac2_sse_classifies_events() {
    let item = json!({"type":"function_call","call_id":"call_1"});
    let ev = json!({"type":"response.output_item.done","output_index":0,"item":item});
    assert_eq!(sse(ev.to_string().as_bytes(), NOW), Sse::Item(item));

    let done = br#"{"type":"response.completed","response":{"id":"resp_1","usage":{"input_tokens":5,"output_tokens":7,"total_tokens":12},"output":[{"type":"message"}]}}"#;
    assert_eq!(
        sse(done, NOW),
        Sse::Completed {
            usage: Usage {
                input_tokens: 5,
                output_tokens: 7
            },
            output: vec![json!({"type":"message"})]
        }
    );
    let bare = br#"{"type":"response.completed","response":{"id":"resp_1"}}"#;
    assert_eq!(
        sse(bare, NOW),
        Sse::Completed {
            usage: Usage::default(),
            output: vec![]
        }
    );

    let failed = br#"{"type":"response.failed","response":{"status":"failed","error":{"code":"server_error","message":"boom"}}}"#;
    assert_eq!(sse(failed, NOW), Sse::Failed(Failure::Fatal("boom".into())));
    let err = br#"{"type":"error","code":"x","message":"bad"}"#;
    assert_eq!(sse(err, NOW), Sse::Failed(Failure::Fatal("bad".into())));
    let err2 = br#"{"type":"error","error":{"message":"worse"}}"#;
    assert_eq!(sse(err2, NOW), Sse::Failed(Failure::Fatal("worse".into())));

    let delta = br#"{"type":"response.output_text.delta","delta":"a"}"#;
    assert!(matches!(sse(delta, NOW), Sse::Event(v) if v["delta"] == "a"));
    assert!(matches!(sse(b"[DONE]", NOW), Sse::Unreadable(_)));
    assert!(matches!(sse(b"[1]", NOW), Sse::Unreadable(_)));
    assert!(matches!(sse(&[0xff, 0xfe], NOW), Sse::Unreadable(_)));
}

#[test]
fn ac3_usage_limit_in_body_and_in_response_failed() {
    // dev TECHNICAL 18.3 `UsageLimit{resets_in_s: 60}`, as a literal.
    let body = br#"{"error":{"type":"usage_limit_reached","code":"usage_limit_reached","message":"The usage limit has been reached","plan_type":"plus","resets_in_seconds":60}}"#;
    assert_eq!(
        error_body(429, body, NOW),
        Failure::UsageLimit {
            until: Some(NOW + 60_000)
        }
    );
    // `Failed{code}` of the dev fake: no reset time.
    let failed = br#"{"type":"response.failed","response":{"id":"resp_1","object":"response","status":"failed","error":{"code":"usage_limit_reached","message":"usage_limit_reached"}}}"#;
    assert_eq!(
        sse(failed, NOW),
        Sse::Failed(Failure::UsageLimit { until: None })
    );
    // resets_at wins, in Unix seconds.
    let at = br#"{"error":{"code":"usage_limit_reached","resets_at":2000,"resets_in_seconds":5}}"#;
    assert_eq!(
        error_body(429, at, NOW),
        Failure::UsageLimit {
            until: Some(2_000_000)
        }
    );
    let none = br#"{"error":{"type":"usage_limit_reached"}}"#;
    assert_eq!(
        error_body(429, none, NOW),
        Failure::UsageLimit { until: None }
    );
    let in_failed = br#"{"type":"response.failed","response":{"error":{"code":"usage_limit_reached","resets_in_seconds":3}}}"#;
    assert_eq!(
        sse(in_failed, NOW),
        Sse::Failed(Failure::UsageLimit {
            until: Some(NOW + 3_000)
        })
    );
}

#[test]
fn ac4_retryable_and_fatal_statuses() {
    assert!(matches!(error_body(500, b"", NOW), Failure::Retryable(_)));
    assert!(matches!(
        error_body(503, b"<html>", NOW),
        Failure::Retryable(_)
    ));
    assert!(matches!(
        error_body(429, br#"{"error":{"type":"rate_limit"}}"#, NOW),
        Failure::Retryable(_)
    ));
    assert!(matches!(error_body(429, b"", NOW), Failure::Retryable(_)));
    assert_eq!(
        error_body(401, br#"{"error":{"message":"bad token"}}"#, NOW),
        Failure::Fatal("status 401: bad token".into())
    );
    assert_eq!(
        error_body(404, b"nope", NOW),
        Failure::Fatal("status 404".into())
    );
    assert!(matches!(error_body(400, b"{}", NOW), Failure::Fatal(m) if m.contains("400")));
    // `gone` is not retryable.
    assert!(matches!(
        ended(Some("gone"), false),
        Some(Failure::Fatal(_))
    ));
}

#[test]
fn ac5_ended_causes() {
    assert_eq!(ended(None, true), None);
    assert_eq!(
        ended(None, false),
        Some(Failure::Fatal(
            "the response ended before it completed".into()
        ))
    );
    assert_eq!(
        ended(Some("no-account"), false),
        Some(Failure::Fatal("not signed in to ChatGPT".into()))
    );
    assert_eq!(
        ended(Some("no-app"), false),
        Some(Failure::Fatal("no app to sign in to ChatGPT".into()))
    );
    for c in ["consent-refused(shell)", "gone", "cancelled"] {
        match ended(Some(c), false) {
            Some(Failure::Fatal(m)) => assert!(m.contains(c), "{m}"),
            o => panic!("{c}: {o:?}"),
        }
    }
    assert!(matches!(
        ended(Some("idle-timeout"), false),
        Some(Failure::Retryable(_))
    ));
    assert!(matches!(
        ended(Some("failed: reset"), true),
        Some(Failure::Retryable(_))
    ));
}

#[test]
fn ac6_backoff_doubles_and_runs_out() {
    let got: Vec<_> = (1..=6).map(backoff_ms).collect();
    assert_eq!(
        got,
        [1_000, 2_000, 4_000, 8_000, 16_000, 32_000]
            .map(Some)
            .to_vec()
    );
    assert_eq!(backoff_ms(7), None);
    assert_eq!(backoff_ms(u32::MAX), None);
    assert_eq!(MAX_ATTEMPTS, 6);
}

/// F1.7 (S40-2 AC1): a head with no `headers` object is not a head.
#[test]
fn r1_f1_7_a_head_needs_a_headers_object() {
    assert_eq!(head(br#"{"status":200}"#), None);
    assert_eq!(head(br#"{"status":200,"headers":[]}"#), None);
    assert_eq!(
        head(br#"{"status":200,"headers":"text/event-stream"}"#),
        None
    );
    assert_eq!(
        head(br#"{"status":200,"headers":{}}"#),
        Some(Head {
            status: 200,
            sse: false
        })
    );
}
