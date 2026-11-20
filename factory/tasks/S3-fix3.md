Folder: kernel
Effort: medium
Depends on: S3-fix2
Slice: S3 — a client mod's maker runs `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>` and sees every refusal at once, or none.

## Goal

The adversary's third pass (commit b23fc8e) hid a path executable behind a colon:
`Bash(x:/run)` passed because the kernel cut the first word at `:`, while Claude Code treats
`:` as part of the command unless the rule ends in `:*`. After this task, the kernel reads a
Bash allow rule the way Claude Code does, and a colon can appear only as that trailing `:*`.

## Acceptance

AC1. `cargo test --test it adv_s3` in `kernel/`: all nine tests pass, `adversary.rs` unchanged;
     every earlier test passes unchanged.
AC2. For an `allow` rule `Bash(<arg>)`: remove one trailing `:*` if present; then `<arg>` must
     contain no `:` (refused naming `allow`, "a colon is only allowed as the trailing :*"), and
     the first word is taken by splitting on whitespace only. `Bash(x:/run)`,
     `Bash(x:/run:*)`, `Bash(git:/tmp/x)`, `Bash(git:status)` are refused; `Bash(git status:*)`,
     `Bash(git:*)`, `Bash(npm test)` pass. Both `rule_problem` and `runner_problem` use this one
     first-word function.
AC3. A `~` in the first word is refused (tilde expansion yields a path). `Bash(ls ~)` is not
     changed by this criterion.
AC4. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean (format only
     your own files; if `cargo fmt` touches `adversary.rs`, commit that layout change in its own
     commit and say so).

## Spec

TECHNICAL 4.1 (allow grammar). Do not edit `design/` (slice S2 rewrites it now).

## Interface

Unchanged.

## Files

`kernel/src/profile.rs`, `kernel/tests/it/grammar.rs`. Not `kernel/tests/it/adversary.rs`.

## Changed tests

none (searched `kernel/tests/it/`: no accepted rule has a colon other than a trailing `:*`, or a
`~` in its first word; if you find one, it now asserts refusal; list it).

## Out of scope

The runner deny list's open end (`make`, `npm run`, `cargo run`, ...): noted for the product
owner.
