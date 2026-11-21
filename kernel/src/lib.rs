//! The agent kernel's native part: errors (TECHNICAL 12), the profile check (4.1, 9.2, 11), the transcript (9.4).

pub mod active;
pub mod adapters;
pub mod check;
pub mod error;
pub mod overlay;
pub mod profile;
pub mod queue;
pub mod record;
pub mod refuse;
pub mod sessions;
pub mod state;
pub mod transcript;

pub use check::check;
pub use error::{KernelError, Problem, Refusal};
pub use overlay::{apply_overlay, parse_overlay, Overlay};
pub use profile::{
    grammar_problems, parse_profile, Backend, PreToolCheck, Profile, Skill, Subagent, SystemPrompt,
};
pub use queue::{check_input, Queue, Queued, INPUT_BYTES, QUEUE_BYTES, QUEUE_INPUTS};
pub use record::{
    check_key, check_title, Activity, Counters, Deadline, OpenAsk, Record, Stop, Turn, ALLOWS,
    ALLOWS_BYTES, ALLOW_BYTES, DEADLINES, ID_BYTES, KEY_BYTES, RECORD_BYTES, STREAMS, TITLE_BYTES,
};
pub use refuse::{refuse, Catalog, CatalogModel};
pub use sessions::{
    Called, FakeHost, Host, HostCall, Kernel, ProviderEvent, Push, Request, SessionInfo,
};
pub use state::{Counts, MemState, State, TooLarge, MAX_VALUE};
pub use transcript::{chunk_key, Joiner, Meta, NewEntry, Page, Replay, Transcript};
