Folder: kernel
Effort: medium
Depends on: none
Slice: S34 — adversary RISKs 1 and 4: no session allow for a write that can make code run later; an early timer firing keeps the restart step it ran

## Goal

Two fail-closed fixes from the adversary's pass on S34 (HELD, with RISKs):

1. A session allow on a file write never covers a path where a write can make
   code run later. `broker::scope` gives no key to a `Write`, `Edit`,
   `MultiEdit` or `NotebookEdit` input whose path, after the workspace, has a
   segment starting with `.` or whose last segment is one of the 4.3 names.
2. `session_firing` (kernel/src/sessions.rs, ~line 2788): when a firing finds
   no deadline due, it re-arms and returns early, without `finish`. If
   `settle` ran a restart step (a record of an older generation, R6), that
   step's record changes and metering are dropped. The early return must save
   what `settle` changed, through the same `finish` path as the main branch,
   and still re-arm.

**The rule (TECHNICAL 9.6 "As built (S34)", Files, written by the foreman for
this fix; quoted):**

> For `Write`, `Edit`, `MultiEdit` and `NotebookEdit` no segment after the
> workspace starts with `.` (`.git/config`, `.claude/`, `.mcp.json`,
> `.vscode/`, `.husky/`: a write there can make code run later) and the
> last segment is none of `CLAUDE.md`, `AGENTS.md`, `build.rs`,
> `package.json`, `Makefile` in any case (4.3); such a write always asks,
> on top of the host's protected paths (adversary RISK 1, S34).

`Read`, `Glob` and `Grep` are unchanged (a read does not plant code).

The human's rule (2026-11-21): "A tool without one, or an input that cannot
be scoped safely, gets no key and always asks."

**9.3 "Past" (quoted; fix 2 changes what a firing writes):** "The exceptions
cannot fail at the ceiling, since each ends a turn or closes a session's work
and frees what it holds: a firing of the restart pass, [...] As built (S29):
what they wrote, even a restart step that failed, is charged to both accounts
and the record's `bytes`". A session-timer firing that runs a restart step
must save and charge it as the main branch of `session_firing` does
(`finish`); a firing that changed nothing must still write no key (S34-3 AC3:
`timers_s34::ac3_an_early_firing_rearms_and_changes_nothing_else` stays green).

## Acceptance

AC1. Unit (`tests/it/scope_s34.rs`), workspace `/w`: `Write`/`Edit`/
`MultiEdit` on `/w/.git/config`, `/w/a/.claude/settings.json`, `/w/.mcp.json`,
`/w/.vscode/tasks.json`, `/w/.husky/pre-commit`, `/w/.env`, `/w/src/CLAUDE.md`,
`/w/claude.md`, `/w/AGENTS.md`, `/w/build.rs`, `/w/pkg/package.json`,
`/w/Makefile`, `/w/MAKEFILE` -> none; `NotebookEdit` on `/w/.ipynb_checkpoints/x.ipynb`
-> none; `Write` on `/w/src/main.rs` and `/w/docs/make.md` -> `Write(/w/**)`.
`Read`, `Glob`, `Grep` on `/w/.git/config` (Grep/Glob `path: "/w/.git"`) keep
`<Tool>(/w/**)`.
AC2. Through the kernel: after the owner allows `Edit` for the session on
`/w/src/a.rs`, an `Edit` ask on `/w/.git/config` opens for the owner (not
answered `session-allow`), and its stored ask has no `reach`.
AC3. Crate test: a session with an `idle` deadline ahead, whose record is of
an older generation (the host's generation raised, as the restart tests do),
gets a timer firing before the deadline: the restart step is saved (the
record's `boot` equals `slots.boot` after the call, the stale keys gone, the
accounts charged as R4/S29 say), the timer is re-armed for the deadline, and
a second firing before the deadline changes no key.
AC4. Every existing test passes unchanged, the S34 adversary's HELD list
included (`scope_s34.rs`, `allows.rs`, `adv_s31.rs`, `adv_s33.rs`,
`timers_s34.rs`, `idle.rs`). `cargo test`, `cargo clippy --all-targets -- -D
warnings`, `cargo fmt --check` in `kernel/`, and `factory/gate.sh` pass.

## Spec

`design/TECHNICAL.md` 9.6 "As built (S34)" (Files), 4.3, 9.3 "Restart" (R4,
R6), "Deadlines and the session timer", "Past".

## Interface

None.

## Files

`kernel/src/broker.rs`, `kernel/src/sessions.rs` (`session_firing` only),
`kernel/tests/it/scope_s34.rs`, `kernel/tests/it/timers_s34.rs` (or
`restart.rs`).

## Changed tests

None expected (searched `scope_s34.rs`, `broker.rs`, `allows.rs`,
`usersim_s33.rs`, `adv_s31.rs`, `adv_s33.rs` for a write on a dot path or a
4.3 name: none asserts a key for one). Name any you change.

## Out of scope

Read keys on dot paths; the host's own protected paths (watane).
