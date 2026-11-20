//! `cargo run --example check -- <profile.json> <overlay.json> <backend> <model> [catalog.json]`:
//! prints one JSON line per refusal and exits 1; prints nothing and exits 0 when there is none;
//! a usage line on stderr and exit 2 for a bad call. Starts nothing, needs no host.

use agent_kernel::{check, Backend, Catalog};
use std::io;

const USAGE: &str =
    "usage: check <profile.json> <overlay.json> <claude|openai> <model> [catalog.json]";

pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

fn usage(why: &str) -> Outcome {
    Outcome {
        code: 2,
        stdout: String::new(),
        stderr: format!("{why}\n{USAGE}\n"),
    }
}

/// The whole example, with the file reader passed in so a test needs no files.
pub fn run(args: &[String], read: impl Fn(&str) -> io::Result<String>) -> Outcome {
    if args.len() != 4 && args.len() != 5 {
        return usage(&format!("expected 4 or 5 arguments, got {}", args.len()));
    }
    let load = |path: &str| read(path).map_err(|e| usage(&format!("cannot read {path}: {e}")));
    let (profile, overlay) = match (load(&args[0]), load(&args[1])) {
        (Ok(p), Ok(o)) => (p, o),
        (Err(u), _) | (_, Err(u)) => return u,
    };
    let backend: Backend = match args[2].parse() {
        Ok(b) => b,
        Err(e) => return usage(&e),
    };
    let catalog = match args.get(4) {
        None => None,
        Some(path) => match load(path).and_then(|t| {
            serde_json::from_str::<Catalog>(&t)
                .map_err(|e| usage(&format!("{path} is not a catalog: {e}")))
        }) {
            Ok(c) => Some(c),
            Err(u) => return u,
        },
    };
    let refusals = check(&profile, &overlay, backend, &args[3], catalog.as_ref());
    let stdout: String = refusals
        .iter()
        .map(|r| serde_json::to_string(r).expect("a refusal serializes") + "\n")
        .collect();
    let stderr = if catalog.is_none() {
        "note: model names and efforts were not checked against a list (no catalog file)\n"
            .to_string()
    } else {
        String::new()
    };
    Outcome {
        code: i32::from(!refusals.is_empty()),
        stdout,
        stderr,
    }
}

#[allow(dead_code)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let o = run(&args, |p| std::fs::read_to_string(p));
    print!("{}", o.stdout);
    eprint!("{}", o.stderr);
    std::process::exit(o.code);
}
