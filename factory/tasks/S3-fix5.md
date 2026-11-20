Folder: kernel
Effort: high
Depends on: S3-fix4
Slice: S3 — a client mod's maker runs `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>` and sees every refusal at once, or none.

## Goal

Five adversary passes each found another spelling that reaches a package runner's `exec`
(abbreviations, aliases, now the forwarders pnpm `recursive`/`m`/`multi` and yarn
`workspace`; commit 93f0c1b). A deny list over package-runner subcommands cannot hold. This
task decides, at the security boundary of the 4.1 allow grammar and for package runners only:
**the second word of an allow `Bash` rule for a package runner must be one of a small fixed
set; anything else is refused.** The general runner list (interpreters, wrappers) stays a deny
list; whether it becomes an allow list is the product owner's question.

## Acceptance

AC1. `cargo test --test it adv_s3` in `kernel/`: all eleven tests pass, `adversary.rs`
     unchanged; every earlier test passes, or changes only as `## Changed tests` says.
AC2. Package runners and their accepted second words (exact match after lowercasing; the
     rule must have a second word):
     - `npm`: `install`, `i`, `ci`, `test`, `run`, `ls`, `list`, `audit`, `outdated`
     - `pnpm`, `yarn`: `install`, `add`, `test`, `run`, `list`, `ls`, `audit`, `outdated`
     - `uv`: `pip`, `sync`, `lock`, `add`, `remove`, `tree`, `venv`
     - `devbox`: `install`, `add`
     Any other second word (or none, or `:*` right after the runner) is refused with the
     runner reason. `SUB_RUNNERS` and the prefix rule of S3-fix4 are replaced by this table.
AC3. `pnpx`, `yarnpkg`, `corepack` join the runner list (refused as first words).
AC4. Tests: `Bash(npm test)`, `Bash(npm install)`, `Bash(npm ci)`, `Bash(npm run build)`,
     `Bash(npm i x)`, `Bash(pnpm install)`, `Bash(yarn install)`, `Bash(uv pip install x)`,
     `Bash(uv sync)` pass. `Bash(pnpm m:*)`, `Bash(pnpm recursive exec:*)`,
     `Bash(yarn workspace p exec:*)`, `Bash(npm exe:*)`, `Bash(npm init)`, `Bash(npm:*)`,
     `Bash(npm)`, `Bash(pnpx x)`, `Bash(yarnpkg exec x)`, `Bash(corepack pnpm dlx x)`,
     `Bash(uv run x)`, `Bash(devbox run x)` are refused.
AC5. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean (a
     layout-only change of `adversary.rs` goes in its own commit; say so).

## Spec

TECHNICAL 4.1 (allow grammar). Do not edit `design/` (slice S2 rewrites it now); this is a
foreman decision for the product owner's record.

## Interface

Unchanged.

## Files

`kernel/src/profile.rs`, `kernel/tests/it/grammar.rs`. Not `kernel/tests/it/adversary.rs`.

## Changed tests

`grammar::ac3_runner_abbreviations_refused_and_project_scripts_pass` (S3-fix4) and the
runner lists of `ac5_*`: each rule they assert *refused* stays refused; a rule they assert
*accepted* that is outside AC2's table (e.g. `npm start`, `yarn build`) now asserts refusal;
list each one in your report.

## Out of scope

`npm run` / `pnpm run` running the project's own scripts, `make`, `cargo run`, `git -c`: the
accepted open end, noted for the product owner.
