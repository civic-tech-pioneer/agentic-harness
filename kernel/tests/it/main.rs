//! Integration tests of the kernel crate (TECHNICAL 16): one binary, one module per area.

mod active;
mod adapters;
mod adv_s11;
mod adv_s14;
mod adv_s20;
mod adv_s21;
mod adversary;
mod asks;
mod attach;
mod check;
mod errors;
mod example;
mod grammar;
mod idle;
mod meter;
mod overlay;
mod quota;
mod record;
mod refuse;
mod regress_s14;
mod regress_s25;
mod restart;
mod revoke;
mod runners;
mod s26;
mod sessions;
mod streams;
mod transcript;
mod usersim_s21;

use agent_kernel::{parse_profile, Problem, Profile};

pub fn parse(json: &str) -> (Profile, Vec<Problem>) {
    parse_profile(json)
}

pub fn fields(ps: &[Problem]) -> Vec<&str> {
    ps.iter().map(|p| p.field.as_str()).collect()
}
