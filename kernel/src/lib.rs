//! The agent kernel's native part: errors (TECHNICAL 12) and the profile check (4.1, 9.2, 11).

pub mod adapters;
pub mod check;
pub mod error;
pub mod overlay;
pub mod profile;
pub mod refuse;

pub use check::check;
pub use error::{KernelError, Problem, Refusal};
pub use overlay::{apply_overlay, parse_overlay, Overlay};
pub use profile::{
    grammar_problems, parse_profile, Backend, PreToolCheck, Profile, Skill, Subagent, SystemPrompt,
};
pub use refuse::{refuse, Catalog, CatalogModel};
