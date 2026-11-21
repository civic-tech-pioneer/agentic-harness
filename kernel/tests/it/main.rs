//! Integration tests of the kernel crate (TECHNICAL 16): one binary, one module per area.

mod active;
mod adapters;
mod adv_s11;
mod adv_s14;
mod adversary;
mod check;
mod errors;
mod example;
mod grammar;
mod overlay;
mod record;
mod refuse;
mod regress_s14;
mod runners;
mod transcript;

use agent_kernel::{parse_profile, Problem, Profile};

pub fn parse(json: &str) -> (Profile, Vec<Problem>) {
    parse_profile(json)
}

pub fn fields(ps: &[Problem]) -> Vec<&str> {
    ps.iter().map(|p| p.field.as_str()).collect()
}
