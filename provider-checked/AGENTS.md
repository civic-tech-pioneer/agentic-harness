# This mod: Claude Code, checked

This is the checked Claude Code provider of the agentic harness (`mods:claude-code@0.1.0`): only the
kernel (`local/agent` while the kernel is unpublished, `KERNEL` in `logic/src/lib.rs`) may call its
`sessions`. It starts `claude` with `pre-tool-check`, forwards every item to its own stream, and
judges every input, `check` and `ask` with TypeSafe System One before it reaches `claude` or is
answered. A safe call goes on; anything else denies. It lives in `provider-checked/` of the
agentic-harness repository (R-B1) and is specified in `design/TECHNICAL.md` section 10 "Checked".
It imports nothing from `kernel/` or `console/`, and no file in this folder refers to a file outside
it. Native tests: `cargo test` in `logic/`, against a fake host (no network, no host).

# This folder

| Path | What it holds |
|---|---|
| mod.toml | name, summary, `[http] hosts` (the judge's host) |
| wit/world.wit | contract: `sessions`, `settings`, the host's sinks |
| wit/deps/watane-host/host.wit | the host's WIT, a copy of `provider/`'s; do not edit |
| logic/Cargo.toml | allowed dependencies (serde_json, wit-bindgen) |
| logic/src/lib.rs | the logic behind a `Host` trait; the state layout is described at its top |
| logic/src/judge.rs | the judge request, the reading of its answer, the verdict mapping |
| logic/src/tests.rs | whole scenarios against a fake host; the judge is played as raw bodies |
| ui/ | the settings page: thresholds and the judge's origin |
| tests/not_owner.json | contract test: every `sessions` function refuses a caller that is not the kernel |
| AGENTS.md, CLAUDE.md | Keep these guides |

The judge's origin is `JUDGE_ORIGIN` at build time (default `https://api.typesafe.ai`); the kernel's
identity is `AGENT_KERNEL_IDENTITY` at build time (default `local/agent`). The judge's key is the
host's account `judge`, entered in the app; this mod holds no key.

# Work

Allowed crates: serde_json 1 and wit-bindgen 0.63; no build script, procedural macro, `.cargo/` or
`rust-toolchain`. Keep `[workspace]` empty and `[lib] crate-type = ["cdylib"]`. Each call gets a fresh
instance: everything persistent goes through the state interface. `ui/` uses script files only.
For the commands of `watane-dev` (doctor, test, serve), the guide of `provider/AGENTS.md` applies.

# Tests

| What | Command |
|---|---|
| Logic and judge, native | `cargo test` in `logic/` |
| Format and lints | `cargo fmt --check`, `cargo clippy -- -D warnings` in `logic/` |
| Component builds | `cargo build --release --target wasm32-wasip2` in `logic/` |
| Contract | `watane-dev test --json` here |
