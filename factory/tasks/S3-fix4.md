Folder: kernel
Effort: medium
Depends on: S3-fix3
Slice: S3 — a client mod's maker runs `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>` and sees every refusal at once, or none.

## Goal

The adversary's fourth pass (commit 67f5c30) showed that `Bash(npm exe:*)` passes, while npm
runs any unique abbreviation of a command (`npm exe -c '…'` is `npm exec`). After this task a
sub-runner's second word is refused when it could be an abbreviation of a refused subcommand,
and npm's other shell-running commands are refused too.

## Acceptance

AC1. `cargo test --test it adv_s3` in `kernel/`: all ten tests pass, `adversary.rs` unchanged;
     every earlier test passes unchanged.
AC2. For a package runner (`npm`, `pnpm`, `yarn`), a second word is refused when it is a
     prefix of any refused subcommand of that runner, at least two characters long, or
     equals an alias. npm refuses `exec`, `x`, `explore` and their prefixes (`ex`, `exe`,
     `exp`, `expl`, ...); `run`, `test`, `start` and the like stay accepted (they run the
     project's own scripts, the same class as `make`, the accepted open end). pnpm and yarn refuse `dlx`,
     `exec` and their prefixes (`dl`, `ex`, `exe`). `uv` refuses `run`, `tool` (and `ru`,
     `to`, `too`); `devbox` refuses `run` (and `ru`).
AC3. `Bash(npm test)`, `Bash(npm install)`, `Bash(npm ci)`, `Bash(npm run build)`,
     `Bash(uv pip install x)`, `Bash(pnpm install)` pass; `Bash(npm exe:*)`,
     `Bash(npm ex:*)`, `Bash(npm explore:*)`, `Bash(npm expl x)`, `Bash(pnpm dl x)`,
     `Bash(uv ru x)` are refused with the runner reason.
AC4. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean (a layout
     change of `adversary.rs` by `cargo fmt` goes in its own commit; say so).

## Spec

TECHNICAL 4.1 (allow grammar, the runner list's "..."). Do not edit `design/`.

## Interface

Unchanged.

## Files

`kernel/src/profile.rs`, `kernel/tests/it/grammar.rs`. Not `kernel/tests/it/adversary.rs`.

## Changed tests

none expected (searched `grammar.rs`: accepted npm rules are `npm test`; uv `uv pip install`).

## Out of scope

pnpm's fallback that runs any `node_modules/.bin/<name>`, and other members of the deny list's
open end: noted for the product owner (deny list vs allow list).
