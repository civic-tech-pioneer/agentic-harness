Folder: kernel
Effort: medium
Depends on: none
Slice: S34 — adversary breaches of S34-fix1: a Unicode spelling of a named file, and a due firing's restart step left uncharged at the ceiling

## Goal

The two red tests in `kernel/tests/it/adv_s34.rs` (commit 73369ed) pass,
and nothing else changes:

1. `adv_s34_unicode_spelling_of_a_named_file_gets_a_key`: `broker::scope`
   gives no key to `Write`, `Edit`, `MultiEdit` or `NotebookEdit` when any
   segment of the path after the workspace holds a byte outside printable
   ASCII (0x20..0x7e), and the last-segment list gains `CLAUDE.local.md` and
   `GNUmakefile`.
2. `adv_s34_due_firing_after_restart_at_ceiling_is_not_charged`:
   `session_firing`'s due path (it ends in `self.finish(...)`, ~line 2815)
   charges a restart step it ran as the early path does: when `settle` ran a
   step (the record's `boot` changed), it ends with `finish_as(.., true)`.

**The rule (TECHNICAL 9.6 "As built (S34)", Files, as edited by the foreman
for fix2; quoted):**

> For `Write`, `Edit`, `MultiEdit` and `NotebookEdit` no segment after the
> workspace starts with `.` (`.git/config`, `.claude/`, `.mcp.json`,
> `.vscode/`, `.husky/`: a write there can make code run later) or holds
> a byte outside printable ASCII (a file system that folds Unicode case
> or drops ignorable code points opens `Ma\u{212A}efile` as `Makefile`
> or `\u{200C}.git` as `.git`; adversary, S34-fix2), and the last
> segment is none of `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`,
> `build.rs`, `package.json`, `Makefile`, `GNUmakefile` in any ASCII
> case (4.3); such a write always asks [...].

**9.3 "Past" (quoted; fix 2 changes what a firing writes):** "As built
(S29): what they wrote, even a restart step that failed, is charged to both
accounts and the record's `bytes`" — the restart step is R4, run by R6 for
any call that reads an older record, and the restart pass's firing "cannot
fail at the ceiling". A session-timer firing that ran the step must charge it
the same way, whether or not a deadline was due.

## Acceptance

AC1. Both red tests in `adv_s34.rs` pass, unchanged.
AC2. Unit (`scope_s34.rs`): `Write` on `/w/CLAUDE.local.md`,
`/w/claude.LOCAL.md`, `/w/GNUmakefile`, `/w/src/caf\u{e9}.rs`,
`/w/\u{200C}.git/config` -> none; `Read` on `/w/src/caf\u{e9}.rs` keeps
`Read(/w/**)` (reads unchanged); `Write` on `/w/src/main.rs` keeps
`Write(/w/**)`.
AC3. A due firing on a record of the current generation (no step) at the
ceiling behaves as before (existing tests in `idle.rs`, `s29_ceiling.rs`,
`timers_s34.rs` pass unchanged).
AC4. The S34 HELD list stays held: every test in `scope_s34.rs`, `allows.rs`,
`adv_s31.rs`, `adv_s33.rs`, `timers_s34.rs`, `s29_ceiling.rs` passes
unchanged. `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` in `kernel/`, and `factory/gate.sh` pass.

## Spec

`design/TECHNICAL.md` 9.6 "As built (S34)" (Files), 9.3 "Restart" (R4, R6),
"Past".

## Interface

None.

## Files

`kernel/src/broker.rs`, `kernel/src/sessions.rs` (`session_firing` only),
`kernel/tests/it/scope_s34.rs`.

## Changed tests

None: the red tests pass as written; no existing test asserts a key for a
non-ASCII write path or the two new names (searched `scope_s34.rs`,
`broker.rs`, `allows.rs`, `usersim_s33.rs`).

## Out of scope

Other build files (`Cargo.toml`, `conftest.py`, ...): named in 9.6 as not
complete, a PO decision. A failed restart step inside `session_firing`
(older than S34; for the report).
