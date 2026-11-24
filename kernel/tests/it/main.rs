//! Integration tests of the kernel crate (TECHNICAL 16): one binary, one module per area.

mod active;
mod adapters;
mod adv_s11;
mod adv_s14;
mod adv_s20;
mod adv_s21;
mod adv_s26;
mod adv_s28;
mod adv_s29;
mod adv_s31;
mod adv_s32;
mod adv_s33;
mod adv_s34;
mod adv_s35;
mod adv_s36;
mod adversary;
mod allows;
mod asks;
mod attach;
mod broker;
mod check;
mod errors;
mod example;
mod grammar;
mod idle;
mod meter;
mod openai_answer;
mod openai_loop;
mod openai_wire;
mod overlay;
mod quota;
mod record;
mod refuse;
mod regress_s14;
mod regress_s25;
mod regress_s26;
mod regress_s35;
mod restart;
mod review_s35_r1;
mod review_s36_r1;
mod revoke;
mod rules;
mod runners;
mod s26;
mod s28;
mod s29_ceiling;
mod s29_short;
mod s35_stage_b;
mod s36_firing;
mod s36_host;
mod s36_load;
mod s36_path;
mod s38;
mod scope_s34;
mod seam_s32;
mod sessions;
mod streams;
mod timers_s34;
mod transcript;
mod usersim_s21;
mod usersim_s33;

use agent_kernel::{parse_profile, Problem, Profile};

pub fn parse(json: &str) -> (Profile, Vec<Problem>) {
    parse_profile(json)
}

pub fn fields(ps: &[Problem]) -> Vec<&str> {
    ps.iter().map(|p| p.field.as_str()).collect()
}
