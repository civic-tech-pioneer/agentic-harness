//! `check`: the profile, then the overlay, then the refuse table, all refusals at once
//! (TECHNICAL 9.2, 11). Starts nothing and reads nothing but its arguments.

use crate::error::Refusal;
use crate::overlay::{apply_overlay, parse_overlay};
use crate::profile::{grammar_problems, parse_profile, Backend};
use crate::refuse::{refuse, Catalog};

pub fn check(
    profile: &str,
    overlay: &str,
    backend: Backend,
    model: &str,
    catalog: Option<&Catalog>,
) -> Vec<Refusal> {
    let (p, mut problems) = parse_profile(profile);
    let (o, overlay_problems) = parse_overlay(overlay);
    problems.extend(overlay_problems);
    let (compiled, limit_problems) = apply_overlay(&p, &o);
    problems.extend(limit_problems);
    problems.extend(grammar_problems(&compiled));
    let mut all: Vec<Refusal> = problems.iter().map(|x| x.refusal(backend, model)).collect();
    all.extend(refuse(&compiled, backend, model, catalog));
    let mut seen: Vec<Refusal> = Vec::new();
    for r in all {
        if !seen
            .iter()
            .any(|s| s.field == r.field && s.reason == r.reason)
        {
            seen.push(r);
        }
    }
    seen
}
