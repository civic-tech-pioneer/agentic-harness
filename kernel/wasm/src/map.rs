//! The host-independent mappings between the crate's types and the WIT's (AC4, AC5, AC6).

use crate::exports::mods::agent::sessions as wit;
use crate::watane::host::{agent_tools, claude_types, streams};
use agent_kernel::{KernelError, Outcome, Refusal, SinkOutcome, StreamError};
use serde_json::Value;

/// The refusal of the WIT, field for field.
pub fn refusal(r: Refusal) -> wit::Refusal {
    wit::Refusal {
        field: r.field,
        backend: r.backend,
        model: r.model,
        reason: r.reason,
    }
}

/// `KernelError` to the `error` variant of 2.3, one case to one.
pub fn error(e: KernelError) -> wit::Error {
    match e {
        KernelError::NotOwner => wit::Error::NotOwner,
        KernelError::UnknownSession(_) => wit::Error::UnknownSession,
        KernelError::Refused(list) => wit::Error::Refused(list.into_iter().map(refusal).collect()),
        KernelError::QueueFull => wit::Error::QueueFull,
        KernelError::Quota(_) => wit::Error::Quota,
        KernelError::Paused(n) => wit::Error::Paused(n),
        KernelError::Backend(m) => wit::Error::Backend(m),
        KernelError::UnknownInput(_) => wit::Error::UnknownInput,
        KernelError::UnknownAllow(_) => wit::Error::UnknownAllow,
        KernelError::Behind(n) => wit::Error::Behind(n),
        KernelError::TooLarge(n) => wit::Error::TooLarge(n),
        KernelError::Busy(_) => wit::Error::Busy,
    }
}

/// The host's `stream-error` to the crate's.
pub fn stream_error(e: streams::StreamError) -> StreamError {
    match e {
        streams::StreamError::NotYours => StreamError::NotYours,
        streams::StreamError::UnknownStream => StreamError::UnknownStream,
        streams::StreamError::Full => StreamError::Full,
        streams::StreamError::TooLarge => StreamError::TooLarge,
    }
}

/// The three outcomes a producer's `close` may pass (host 19.1).
pub fn outcome(o: Outcome) -> streams::Outcome {
    match o {
        Outcome::Closed => streams::Outcome::Closed,
        Outcome::Replaced => streams::Outcome::Replaced,
        Outcome::Lagging => streams::Outcome::Lagging,
    }
}

pub fn sink_outcome(o: SinkOutcome) -> streams::SinkOutcome {
    match o {
        SinkOutcome::Taken => streams::SinkOutcome::Taken,
        SinkOutcome::Defer => streams::SinkOutcome::Defer,
    }
}

/// A provider's `start-error` as the crate's `Err(cause)` (TECHNICAL 2.1): the case in kebab-case,
/// `bad-launch` with its field and reason.
pub fn start_error(e: claude_types::StartError) -> String {
    use claude_types::StartError as E;
    match e {
        E::BadLaunch(b) => format!("bad-launch: {}: {}", b.field, b.reason),
        E::Destination => "destination".into(),
        E::ForbiddenHeader(_) => "forbidden-header".into(),
        E::TooLarge => "too-large".into(),
        E::Full => "full".into(),
        E::NotOwner => "not-owner".into(),
        E::NotDirect => "not-direct".into(),
    }
}

/// An `agent-tools` error as the crate's `Err(cause)`: the case in kebab-case, with its text for
/// `consent-pending` and `consent-refused`.
pub fn tools_error(e: agent_tools::ToolsError) -> String {
    use agent_tools::ToolsError as E;
    match e {
        E::NotGranted => "not-granted".into(),
        E::UnknownHandle => "unknown-handle".into(),
        E::UnknownTool => "unknown-tool".into(),
        E::ConsentPending(what) => format!("consent-pending({what})"),
        E::ConsentRefused(what) => format!("consent-refused({what})"),
    }
}

/// An `answer` decision (2.3's table) as the WIT `decision`: `allow` takes the changed `input`
/// as JSON text, `deny` its reason (empty when absent); `{"none":true}` is a check's `none`.
pub fn decision(d: &Value) -> claude_types::Decision {
    use claude_types::Decision as D;
    if d.get("allow") == Some(&Value::Bool(true)) {
        return D::Allow(d.get("input").map(Value::to_string));
    }
    if d.get("deny") == Some(&Value::Bool(true)) {
        let reason = d.get("reason").and_then(Value::as_str).unwrap_or("");
        return D::Deny(reason.to_string());
    }
    D::None
}

/// The `cause` a provider stream's outcome gives `provider_ended_with` (2.1): `None` for `closed`,
/// else the outcome's name.
pub fn ended_cause(o: &streams::Outcome) -> Option<String> {
    use streams::Outcome as O;
    Some(match o {
        O::Closed => return None,
        O::Cancelled => "cancelled".into(),
        O::Lagging => "lagging".into(),
        O::Replaced => "replaced".into(),
        O::ConsumerFailed => "consumer-failed".into(),
        O::IdleTimeout => "idle-timeout".into(),
        O::Gone => "gone".into(),
        O::Mismatch(f) => format!("mismatch({f})"),
        O::NoBinary => "no-binary".into(),
        O::Version(v) => format!("version {} < {}", v.found, v.need),
        O::ConsentRefused(x) => format!("consent-refused({x})"),
        O::NoApp => "no-app".into(),
        O::NoAccount => "no-account".into(),
        O::Failed(t) => format!("failed: {t}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac4_error_cases_map_one_to_one() {
        assert!(matches!(error(KernelError::NotOwner), wit::Error::NotOwner));
        assert!(matches!(
            error(KernelError::UnknownSession("s1".into())),
            wit::Error::UnknownSession
        ));
        assert!(matches!(
            error(KernelError::Behind(7)),
            wit::Error::Behind(7)
        ));
        assert!(matches!(
            error(KernelError::Paused(9)),
            wit::Error::Paused(9)
        ));
        assert!(matches!(
            error(KernelError::TooLarge(5)),
            wit::Error::TooLarge(5)
        ));
        assert!(matches!(
            error(KernelError::QueueFull),
            wit::Error::QueueFull
        ));
        assert!(matches!(
            error(KernelError::Quota("q".into())),
            wit::Error::Quota
        ));
        assert!(matches!(
            error(KernelError::Busy("b".into())),
            wit::Error::Busy
        ));
        assert!(matches!(
            error(KernelError::UnknownInput(1)),
            wit::Error::UnknownInput
        ));
        assert!(matches!(
            error(KernelError::UnknownAllow("a".into())),
            wit::Error::UnknownAllow
        ));
        match error(KernelError::Backend("x".into())) {
            wit::Error::Backend(m) => assert_eq!(m, "x"),
            _ => panic!("backend"),
        }
        let r = Refusal {
            field: "f".into(),
            backend: "claude".into(),
            model: "m".into(),
            reason: "r".into(),
        };
        match error(KernelError::Refused(vec![r])) {
            wit::Error::Refused(l) => {
                assert_eq!((l[0].field.as_str(), l[0].reason.as_str()), ("f", "r"))
            }
            _ => panic!("refused"),
        }
    }

    #[test]
    fn ac5_outcomes_and_stream_errors_map() {
        assert!(matches!(outcome(Outcome::Closed), streams::Outcome::Closed));
        assert!(matches!(
            outcome(Outcome::Replaced),
            streams::Outcome::Replaced
        ));
        assert!(matches!(
            outcome(Outcome::Lagging),
            streams::Outcome::Lagging
        ));
        assert_eq!(stream_error(streams::StreamError::Full), StreamError::Full);
        assert_eq!(
            stream_error(streams::StreamError::NotYours),
            StreamError::NotYours
        );
        assert_eq!(
            stream_error(streams::StreamError::UnknownStream),
            StreamError::UnknownStream
        );
        assert_eq!(
            stream_error(streams::StreamError::TooLarge),
            StreamError::TooLarge
        );
        assert!(matches!(
            sink_outcome(SinkOutcome::Defer),
            streams::SinkOutcome::Defer
        ));
        assert!(matches!(
            sink_outcome(SinkOutcome::Taken),
            streams::SinkOutcome::Taken
        ));
    }

    #[test]
    fn ac3_start_errors_map_to_their_case_name() {
        use claude_types::{BadLaunch, StartError as E};
        let bad = E::BadLaunch(BadLaunch {
            field: "model".into(),
            reason: "unknown".into(),
        });
        assert_eq!(start_error(bad), "bad-launch: model: unknown");
        assert_eq!(start_error(E::Destination), "destination");
        assert_eq!(
            start_error(E::ForbiddenHeader("x".into())),
            "forbidden-header"
        );
        assert_eq!(start_error(E::TooLarge), "too-large");
        assert_eq!(start_error(E::Full), "full");
        assert_eq!(start_error(E::NotOwner), "not-owner");
        assert_eq!(start_error(E::NotDirect), "not-direct");
    }

    #[test]
    fn ac1_tools_errors_map_to_their_case_name() {
        use agent_tools::ToolsError as E;
        assert_eq!(tools_error(E::NotGranted), "not-granted");
        assert_eq!(tools_error(E::UnknownHandle), "unknown-handle");
        assert_eq!(tools_error(E::UnknownTool), "unknown-tool");
        assert_eq!(
            tools_error(E::ConsentPending("workspace:/a".into())),
            "consent-pending(workspace:/a)"
        );
        assert_eq!(
            tools_error(E::ConsentRefused("shell".into())),
            "consent-refused(shell)"
        );
    }

    #[test]
    fn ac3_decisions_map_as_the_table_of_2_3() {
        use claude_types::Decision as D;
        assert!(matches!(
            decision(&serde_json::json!({"allow": true})),
            D::Allow(None)
        ));
        assert!(matches!(
            decision(&serde_json::json!({"allow": true, "for": "session"})),
            D::Allow(None)
        ));
        match decision(&serde_json::json!({"allow": true, "input": {"a": 1}})) {
            D::Allow(Some(i)) => assert_eq!(i, r#"{"a":1}"#),
            _ => panic!("allow with input"),
        }
        match decision(&serde_json::json!({"deny": true, "reason": "no"})) {
            D::Deny(r) => assert_eq!(r, "no"),
            _ => panic!("deny"),
        }
        match decision(&serde_json::json!({"deny": true})) {
            D::Deny(r) => assert_eq!(r, ""),
            _ => panic!("deny without reason"),
        }
        assert!(matches!(
            decision(&serde_json::json!({"none": true})),
            D::None
        ));
    }

    #[test]
    fn ac4_ended_cause_names_every_outcome_but_closed() {
        use streams::Outcome as O;
        let c = |o: O| ended_cause(&o);
        assert_eq!(c(O::Closed), None);
        assert_eq!(c(O::NoBinary).as_deref(), Some("no-binary"));
        assert_eq!(
            c(O::Mismatch("tools".into())).as_deref(),
            Some("mismatch(tools)")
        );
        let v = streams::VersionNeed {
            found: "2.1.200".into(),
            need: "2.1.295".into(),
        };
        assert_eq!(
            c(O::Version(v)).as_deref(),
            Some("version 2.1.200 < 2.1.295")
        );
        assert_eq!(c(O::Failed("boom".into())).as_deref(), Some("failed: boom"));
        assert_eq!(
            c(O::ConsentRefused("shell".into())).as_deref(),
            Some("consent-refused(shell)")
        );
        for (o, n) in [
            (O::Cancelled, "cancelled"),
            (O::Lagging, "lagging"),
            (O::Replaced, "replaced"),
            (O::ConsumerFailed, "consumer-failed"),
            (O::IdleTimeout, "idle-timeout"),
            (O::Gone, "gone"),
            (O::NoApp, "no-app"),
            (O::NoAccount, "no-account"),
        ] {
            assert_eq!(c(o).as_deref(), Some(n));
        }
    }
}
