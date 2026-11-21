//! The host-independent mappings between the crate's types and the WIT's (AC4, AC5, AC6).

use crate::exports::mods::agent::sessions as wit;
use crate::watane::host::streams;
use agent_kernel::adapters;
use agent_kernel::{KernelError, Outcome, ProviderEvent, Refusal, SinkOutcome, StreamError};
use serde::Deserialize;
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

/// The JSON shape of a provider item's body (the providers' side, TECHNICAL 10): `{"out": ...}`
/// or `{"turn": ...}`. The crate's `ProviderEvent` does not derive `Deserialize`.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Wire {
    Out(Box<adapters::Out>),
    Turn(agent_kernel::record::Turn),
}

/// An item body as a `ProviderEvent`; a body that does not parse is an `unknown` entry
/// carrying the raw text as a JSON string (AC6).
pub fn provider_event(body: &[u8]) -> ProviderEvent {
    match serde_json::from_slice::<Wire>(body) {
        Ok(Wire::Out(o)) => ProviderEvent::Out(*o),
        Ok(Wire::Turn(t)) => ProviderEvent::Turn(t),
        Err(_) => ProviderEvent::Out(adapters::unknown(Value::String(
            String::from_utf8_lossy(body).into_owned(),
        ))),
    }
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
        assert!(matches!(
            sink_outcome(SinkOutcome::Defer),
            streams::SinkOutcome::Defer
        ));
    }

    #[test]
    fn ac6_unparsable_body_is_an_unknown_entry_with_raw_text() {
        match provider_event(b"not json") {
            ProviderEvent::Out(o) => {
                let e = o.entry.expect("entry");
                assert_eq!(e.body, serde_json::json!({"raw": "not json"}));
            }
            _ => panic!("out"),
        }
        assert!(matches!(
            provider_event(br#"{"turn":{"state":"paused"}}"#),
            ProviderEvent::Turn(_)
        ));
        assert!(matches!(
            provider_event(br#"{"out":{}}"#),
            ProviderEvent::Out(_)
        ));
    }
}
