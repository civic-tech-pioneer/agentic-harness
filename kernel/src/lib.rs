//! The agent kernel's native part: errors (TECHNICAL 12), the profile check (4.1, 9.2, 11), the transcript (9.4).

pub mod access;
pub mod active;
pub mod adapters;
pub mod broker;
pub mod call;
pub mod check;
pub mod delegation;
pub mod error;
pub mod launch;
pub mod named;
pub mod openai;
pub mod overlay;
pub mod profile;
pub mod queue;
pub mod quota;
pub mod record;
pub mod refuse;
pub mod rules;
pub mod sessions;
pub mod state;
pub mod transcript;

pub use check::check;
pub use error::{KernelError, Problem, Refusal};
pub use named::{ProfileInfo, MAX_PROFILES};
pub use overlay::{apply_overlay, parse_overlay, Overlay};
pub use profile::{
    grammar_problems, parse_profile, Backend, PreToolCheck, Profile, Skill, Subagent, SystemPrompt,
};
pub use queue::{check_input, Queue, Queued, INPUT_BYTES, QUEUE_BYTES, QUEUE_INPUTS};
pub use quota::{
    scope, segment, Change, Limits, Metered, Scope, Usage, ACCOUNT_BYTES, OWNER_BYTES,
    OWNER_PROVIDER, OWNER_SESSIONS, PROVIDER_STREAMS, RESERVE_BYTES,
};
pub use record::{
    check_key, check_title, Activity, Allow, Counters, Deadline, OpenAsk, Record, Stop, Turn,
    ALLOWS, ALLOWS_BYTES, ALLOW_BYTES, DEADLINES, ID_BYTES, KEY_BYTES, RECORD_BYTES, STREAMS,
    TITLE_BYTES,
};
pub use refuse::{refuse, Catalog, CatalogModel};
pub use rules::{Rules, Verdict};
pub use sessions::{
    input_text, Attached, Called, FakeHost, Host, HostCall, Kernel, Outcome, ProviderEvent, Push,
    Request, SessionInfo, SinkOutcome, Started, StreamError, Tool, MAX_TIMER_MS,
};
pub use state::{Buffered, Counts, MemState, State, TooLarge, MAX_VALUE};
pub use transcript::{chunk_key, Joiner, Meta, NewEntry, Page, Replay, Transcript};
