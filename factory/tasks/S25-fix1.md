Folder: kernel
Effort: low
Depends on: S25-2
Slice: S25 — owners are held to 9.3 "Quota"

## Goal

A `quota` refusal names the cap that caused it (FUNCTIONAL AGT-F-13, "name
the quota"; TECHNICAL 12, "Each error also carries a message that names the
cause"), as `busy` already does: the owner's session cap, the owner's bytes
cap, the kernel's account, the reserve ceiling, or the session's chunk cap.
The code stays `quota`; the WIT variant (2.3) is unchanged (no payload).

## Acceptance

AC1. The red tests of `kernel/tests/it/regress_s25.rs` (commit 69b6d9d)
pass. Their `matches!(e, KernelError::Quota)` may become
`KernelError::Quota(_)`; nothing else in them changes.
AC2. `KernelError::Quota(String)`: `code()` is still `"quota"`; Display is
`quota: <cause>`, the cause naming the cap with its number, e.g. "64
sessions per owner", "64 MiB per owner", "the kernel's 224 MiB account",
"the 256 MiB state", "16384 transcript chunks per session" (numbers from
`k.limits` / the constants, so a raised setting shows its value).
AC3. Every place that returns `quota` gives its own cause; one test per
cause asserts the text (extend `tests/it/quota.rs`).
AC4. Nothing else changes: every other test passes with only the pattern
`Quota` -> `Quota(_)` updated where it matches. The adversary's HELD
attacks (refusals before writes, replays write nothing, charges to the
right owner) stay held: no change to when a call is refused or what it
writes.

## Spec

`design/FUNCTIONAL.md` AGT-F-13; `design/TECHNICAL.md` 9.3 Quota, 12.

## Interface

`KernelError::Quota(String)`, like `KernelError::Busy(String)`.

## Files

`kernel/src/error.rs`, `kernel/src/sessions.rs`, `kernel/src/transcript.rs`,
`kernel/tests/it/*.rs` (pattern updates and the new tests only).

## Changed tests

Each `KernelError::Quota` pattern in `kernel/tests/it/` (errors.rs,
transcript.rs, quota.rs, regress_s25.rs, usersim/adv files if any) becomes
`Quota(_)`; `errors.rs` builds the variant with a message and still asserts
the code `quota`.

## Out of scope

Persisting `k.limits` (kernel settings are not stored yet).
