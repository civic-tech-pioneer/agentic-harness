//! Integration tests of the kernel crate (TECHNICAL 16): one binary, one module per area.

mod errors;
mod grammar;
mod refuse;

use agent_kernel::{parse_profile, Problem, Profile};

pub fn parse(json: &str) -> (Profile, Vec<Problem>) {
    parse_profile(json)
}

pub fn fields(ps: &[Problem]) -> Vec<&str> {
    ps.iter().map(|p| p.field.as_str()).collect()
}
