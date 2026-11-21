Folder: kernel
Effort: medium
Depends on: S11-fix1
Slice: S11 — a kernel maker runs `cargo test` in `kernel/` and sees a session's transcript stored and read back as 9.4 and 9.1 say (this task: the adversary's breach, client-set indexes cannot overflow).

## Goal

`read` and `replay` take `after`, `before` and `max` from the client (2.3), so no value of them
can make the kernel panic (a trap in the WASM mod) or wrap around to another range. Today
`read(u64::MAX, …)` computes `after + 1` and overflows (`kernel/src/transcript.rs`, the
adversary's red test `adv_s11_read_after_max_returns_nothing` in `kernel/tests/it/adv_s11.rs`).

## Acceptance

AC1. The red test `adv_s11_read_after_max_returns_nothing` passes: `read(u64::MAX, None, 256)`
     and `read(u64::MAX, Some(u64::MAX), 256)` return an empty page with the right `last`.
AC2. The same class, one test each in `kernel/tests/it/adv_s11.rs`: `read(0, Some(0), …)`,
     `read(0, Some(1), …)`, `read(5, Some(3), …)` (before not above after + 1) return an empty
     page; `replay(u64::MAX)` returns `Entries(vec![])`; `read(after, None, u32::MAX)` is held to
     256; on an empty transcript every one of these returns an empty page with `last` 0. None
     panics in a debug build. No arithmetic on a client-set value in `transcript.rs` can
     overflow (checked, saturating, or guarded by an early return).
AC3. Nothing else changes: the other tests pass unchanged; `cargo test` and
     `cargo clippy --all-targets -- -D warnings` pass in `kernel/`.

## Spec

TECHNICAL 9.4 (`read`, replay), 2.3 (`read` and `attach` take `after`, `before`, `max` from the
client), 14 "A session is private" (a trap in the kernel takes down every owner's sessions).

## Interface

None changes.

## Files

`kernel/src/transcript.rs`, `kernel/tests/it/adv_s11.rs`.

## Changed tests

none (only new tests in `adv_s11.rs`; the red test is kept as is).

## Out of scope

The adversary's RISKs (`max` 0, an unreadable line's `i`, `append` validation, a partial
`append`): the product owner's, in S11's report. `design/`.
