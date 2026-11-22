//! The unchecked Claude Code provider (TECHNICAL 2.2, 10 "Unchecked"). Only the kernel may call it.
wit_bindgen::generate!({ path: "../wit", world: "claude-code", generate_all });

use exports::mods::claude_code::sessions::{Guest, Started};
use watane::host::claude_types::{Decision, SessionError, StartError};
use watane::host::streams::{self, StreamError};
use watane::host::{caller, claude_code, claude_session};

/// The identity of the kernel `mods:agent` while it is unpublished (PLAN R-B3); the published
/// `<publisher>/agent` replaces it before the provider is published.
const KERNEL: &str = "local/agent";

/// The only identity that passes: an exact byte comparison.
fn is_kernel(identity: &str) -> bool {
    identity.as_bytes() == KERNEL.as_bytes()
}

/// The host calls `start` makes, in the order it makes them.
trait Host {
    fn start(&mut self, launch: &str) -> Result<String, StartError>;
    fn say(&mut self, s: &str, text: &str) -> Result<(), SessionError>;
    fn hand_off(&mut self, s: &str) -> Result<(), StreamError>;
    fn cancel(&mut self, s: &str);
}

/// The text of an input, as the kernel reads it: the `text` of the ACP text blocks of a JSON
/// array joined with a blank line (none when there is none); any other input is its own text
/// (none when empty).
fn input_text(input: &str) -> Option<String> {
    let text = match serde_json::from_str::<serde_json::Value>(input) {
        Ok(serde_json::Value::Array(blocks)) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => input.to_string(),
    };
    (!text.is_empty()).then_some(text)
}

fn session_error_case(e: &SessionError) -> &'static str {
    match e {
        SessionError::NotYours => "not-yours",
        SessionError::UnknownStream => "unknown-stream",
        SessionError::SlashCommand => "slash-command",
        SessionError::UnknownAsk => "unknown-ask",
        SessionError::NotAllowedHere => "not-allowed-here",
        SessionError::NotOwner => "not-owner",
        SessionError::NotDirect => "not-direct",
    }
}

fn stream_error_case(e: &StreamError) -> &'static str {
    match e {
        StreamError::NotYours => "not-yours",
        StreamError::UnknownStream => "unknown-stream",
        StreamError::Full => "full",
        StreamError::TooLarge => "too-large",
    }
}

fn bad_launch(field: &str, reason: &str) -> StartError {
    StartError::BadLaunch(watane::host::claude_types::BadLaunch {
        field: field.to_string(),
        reason: reason.to_string(),
    })
}

/// `start` after the owner check (10 "Unchecked").
fn start_with<H: Host>(h: &mut H, launch: &str, input: &str) -> Result<Started, StartError> {
    let s = h.start(launch)?;
    let said = match input_text(input) {
        Some(text) => h.say(&s, &text),
        None => Ok(()),
    };
    if let Err(e) = said {
        h.cancel(&s);
        return Err(bad_launch("input", session_error_case(&e)));
    }
    if let Err(e) = h.hand_off(&s) {
        h.cancel(&s);
        return Err(bad_launch("stream", stream_error_case(&e)));
    }
    Ok(Started {
        stream: s,
        direct: true,
    })
}

struct Real;

impl Host for Real {
    fn start(&mut self, launch: &str) -> Result<String, StartError> {
        claude_code::start(launch)
    }
    fn say(&mut self, s: &str, text: &str) -> Result<(), SessionError> {
        claude_session::say(s, text)
    }
    fn hand_off(&mut self, s: &str) -> Result<(), StreamError> {
        streams::hand_off(s)
    }
    fn cancel(&mut self, s: &str) {
        streams::cancel(s)
    }
}

struct Mod;

impl Guest for Mod {
    fn start(launch: String, input: String) -> Result<Started, StartError> {
        if !is_kernel(&caller::identity()) {
            return Err(StartError::NotOwner);
        }
        start_with(&mut Real, &launch, &input)
    }

    fn say(_s: String, _content: String) -> Result<(), SessionError> {
        refuse(&caller::identity())
    }

    fn answer(_s: String, _ask: String, _d: Decision) -> Result<(), SessionError> {
        refuse(&caller::identity())
    }

    fn interrupt(_s: String) -> Result<(), SessionError> {
        refuse(&caller::identity())
    }

    fn compact(_s: String) -> Result<(), SessionError> {
        refuse(&caller::identity())
    }
}

/// The answer of every function but `start`: the kernel talks to a direct stream itself.
fn refuse(identity: &str) -> Result<(), SessionError> {
    if is_kernel(identity) {
        Err(SessionError::NotDirect)
    } else {
        Err(SessionError::NotOwner)
    }
}

/// The provider is the consumer of the stream `claude-code.start` opens only until `hand-off`,
/// in the same call, so the host delivers nothing to it.
impl exports::watane::host::stream_sink::Guest for Mod {
    fn items(_s: String, _items: Vec<streams::Item>) -> streams::SinkOutcome {
        streams::SinkOutcome::Taken
    }
    fn ended(_s: String, _outcome: streams::Outcome) {}
}

export!(Mod);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac2_only_the_kernel_identity_passes() {
        assert!(is_kernel("local/agent"));
        for other in [
            "local/agents",
            "local/agent ",
            "Local/agent",
            "x/agent",
            "local/notes",
            "page",
            "agent:local/agent",
            "host",
            "",
        ] {
            assert!(!is_kernel(other), "{other:?}");
        }
    }

    #[derive(Default)]
    struct Rec {
        calls: Vec<String>,
        start: Option<StartError>,
        say: Option<SessionError>,
        hand_off: Option<StreamError>,
    }

    impl Host for Rec {
        fn start(&mut self, launch: &str) -> Result<String, StartError> {
            self.calls.push(format!("start {launch}"));
            match self.start.take() {
                Some(e) => Err(e),
                None => Ok("S".into()),
            }
        }
        fn say(&mut self, s: &str, text: &str) -> Result<(), SessionError> {
            self.calls.push(format!("say {s} {text}"));
            self.say.take().map_or(Ok(()), Err)
        }
        fn hand_off(&mut self, s: &str) -> Result<(), StreamError> {
            self.calls.push(format!("hand-off {s}"));
            self.hand_off.take().map_or(Ok(()), Err)
        }
        fn cancel(&mut self, s: &str) {
            self.calls.push(format!("cancel {s}"));
        }
    }

    const INPUT: &str =
        r#"[{"type":"text","text":"a"},{"type":"image","data":"x"},{"type":"text","text":"b"}]"#;

    #[test]
    fn ac3_start_say_hand_off_in_order() {
        let mut h = Rec::default();
        let r = start_with(&mut h, " {\"x\":1} ", INPUT).unwrap();
        assert_eq!((r.stream.as_str(), r.direct), ("S", true));
        assert_eq!(h.calls, ["start  {\"x\":1} ", "say S a\n\nb", "hand-off S"]);
    }

    #[test]
    fn ac3_no_text_means_no_say() {
        for input in ["", "[]", r#"[{"type":"image"}]"#, r#"[{"type":"text"}]"#] {
            let mut h = Rec::default();
            start_with(&mut h, "L", input).unwrap();
            assert_eq!(h.calls, ["start L", "hand-off S"], "{input:?}");
        }
    }

    #[test]
    fn ac1_plain_input_is_its_own_text() {
        assert_eq!(input_text("hi").as_deref(), Some("hi"));
        assert_eq!(input_text("{}").as_deref(), Some("{}"));
        assert_eq!(input_text("").as_deref(), None);
        let mut h = Rec::default();
        start_with(&mut h, "L", "hi").unwrap();
        assert_eq!(h.calls, ["start L", "say S hi", "hand-off S"]);
    }

    #[test]
    fn ac3_start_error_passes_through() {
        let mut h = Rec {
            start: Some(StartError::Full),
            ..Rec::default()
        };
        assert!(matches!(
            start_with(&mut h, "L", INPUT),
            Err(StartError::Full)
        ));
        assert_eq!(h.calls, ["start L"]);
    }

    #[test]
    fn ac3_say_error_cancels_and_is_bad_launch_input() {
        let mut h = Rec {
            say: Some(SessionError::SlashCommand),
            ..Rec::default()
        };
        match start_with(&mut h, "L", INPUT) {
            Err(StartError::BadLaunch(b)) => {
                assert_eq!(
                    (b.field.as_str(), b.reason.as_str()),
                    ("input", "slash-command")
                )
            }
            _ => panic!("expected bad-launch"),
        }
        assert_eq!(h.calls, ["start L", "say S a\n\nb", "cancel S"]);
    }

    #[test]
    fn ac3_hand_off_error_cancels_and_is_bad_launch_stream() {
        let mut h = Rec {
            hand_off: Some(StreamError::NotYours),
            ..Rec::default()
        };
        match start_with(&mut h, "L", "[]") {
            Err(StartError::BadLaunch(b)) => {
                assert_eq!(
                    (b.field.as_str(), b.reason.as_str()),
                    ("stream", "not-yours")
                )
            }
            _ => panic!("expected bad-launch"),
        }
        assert_eq!(h.calls, ["start L", "hand-off S", "cancel S"]);
    }

    #[test]
    fn ac4_other_functions_not_direct_for_kernel_not_owner_for_others() {
        assert!(matches!(
            refuse("local/agent"),
            Err(SessionError::NotDirect)
        ));
        for other in ["local/agents", "page", "host", ""] {
            assert!(
                matches!(refuse(other), Err(SessionError::NotOwner)),
                "{other:?}"
            );
        }
    }
}
