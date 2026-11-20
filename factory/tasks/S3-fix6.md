Folder: kernel
Effort: medium
Depends on: S3-fix5
Slice: S3 — a client mod's maker runs `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>` and sees every refusal at once, or none.

## Goal

The adversary's sixth pass (commit 1dc1f03): npm's built-in `env` script makes the allowed
`npm run` reach the refused `env` runner (`npm run env -- sh -c …`), and npm and uv options
after an allowed word name a program to run (`--script-shell=<file>`, `--python <file>`). After
this task, `run` is not an accepted second word for any package runner, and no word after a
package runner may be an option.

## Acceptance

AC1. `cargo test --test it adv_s3` in `kernel/`: all twelve tests pass, `adversary.rs`
     unchanged; every earlier test passes or changes only as `## Changed tests` says.
AC2. `run` is removed from the npm, pnpm and yarn rows of `PACKAGE_RUNNERS`.
AC3. In an allow `Bash` rule whose first word is a package runner (npm, pnpm, yarn, uv,
     devbox), any later word that starts with `-` is refused with the runner reason (this
     includes `--`). `Bash(npm test)`, `Bash(npm ci)`, `Bash(npm install x)`,
     `Bash(uv pip install x)` pass; `Bash(npm run build)`, `Bash(npm run env:*)`,
     `Bash(npm run:*)`, `Bash(npm install --script-shell=x)`, `Bash(npm ci -- x)`,
     `Bash(uv venv --python x)` are refused.
AC4. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean
     (a layout-only change of `adversary.rs` goes in its own commit; say so).

## Spec

TECHNICAL 4.1. Do not edit `design/`. A foreman decision for the product owner's record.

## Interface

Unchanged.

## Files

`kernel/src/profile.rs`, `kernel/tests/it/grammar.rs`. Not `kernel/tests/it/adversary.rs`.

## Changed tests

Tests that assert `npm run build` (S3-fix4's `ac3_runner_abbreviations…`, S3-fix5's
`ac2_package_runner_second_word_is_an_allow_list`), or `run` in the exact table
(`ac2_package_runner_table_is_exact`), now assert refusal for `run`; list each one.

## Out of scope

Installs running lifecycle scripts, Write editing `package.json`, and runners outside the list
(nodemon, poetry, pipx, nix, ...): the product owner's spec question (allow list, or the
sandbox as the only boundary).
