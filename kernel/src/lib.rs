//! The agent kernel's native part: errors (TECHNICAL 12) and the profile check (4.1, 9.2, 11).

pub mod error;
pub mod profile;
pub mod refuse;

pub use error::{KernelError, Problem, Refusal};
pub use profile::{
    grammar_problems, parse_profile, Backend, PreToolCheck, Profile, Skill, Subagent, SystemPrompt,
};
pub use refuse::{refuse, Catalog, CatalogModel};
