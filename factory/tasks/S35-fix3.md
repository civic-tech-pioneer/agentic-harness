Folder: kernel
Effort: high
Depends on: none
Slice: S35 — stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude`

## Goal

The adversary's two red tests (5bdf57c, `kernel/tests/it/adv_s35.rs`) pass: a later launch (a
`say` or an `interrupt` with a replacement that opens a new provider stream) is compiled and
checked exactly as `start` is, so a profile edited since the session started can never make a
relaunch wider than what `check` and the refuse table allow. This decides a security boundary
(14, "A profile or overlay cannot widen what the participant set"): a relaunch with any refusal
is refused `refused(<refusals>)`, writes nothing and starts no stream (9.3 `overlay` row as edited).
Also: the first-run allowance of 9.1 covers only the built-in `scratch`; a stored `profile/scratch`
needs `access/page` to list it.

## Acceptance

AC1. `adv_s35_relaunch_drops_a_deny_check_would_refuse` and `adv_s35_relaunch_skips_the_refuse_table`
     pass unchanged.
AC2. A relaunch whose `check(profile, kept overlay)` has refusals: `say` returns
     `KernelError::Refused(<the same list check returns>)`, the state is byte-equal before and after
     (no entry, no queue change, no client key, no stream), and no `HostCall::StartProvider`. The
     same for `interrupt(session, Some(replacement))` when it needs a new stream.
AC3. A relaunch with no refusals is exactly as today (the S35-1 and validator tests still pass).
AC4. With no `access/page`: a stored `profile/scratch` is refused for `page` `{field: "profile"}`
     (as a name not listed); with no stored `profile/scratch` the built-in still starts (S35-1 AC9).
     With `access/page` `["scratch"]` a stored `profile/scratch` is used.
AC5. `cargo test -p agent-kernel` (all), clippy, fmt; the gate passes.

## Spec

`design/TECHNICAL.md` 9.1 (`access/`, as edited), 9.2 (compilation), 9.3 (the `overlay` row as
edited), 11, 14.

## Interface

None changes.

## Files

kernel/src/**, kernel/tests/** (not `adv_s35.rs`'s two tests, which must pass as written).

## Changed tests

The S35-1 test `ac9_the_page_gets_the_built_in_scratch_profile_on_first_run`: its "stored
`profile/scratch` is used instead of the built-in" case now needs `access/page` `["scratch"]`
(AC4).

## Out of scope

Adapter state past 4 MiB (an availability limit, S36); a turn whose input has no text (S36).
