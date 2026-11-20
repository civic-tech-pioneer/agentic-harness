Folder: console
Effort: medium
Depends on: none
Slice: S4 — Alice opens the console on a scripted fake kernel and sees a session as TECHNICAL 18 describes, and after a reload the same transcript with nothing doubled.

## Goal

The read side follows the same paused, bounded policy as the attach side: no answer of a kernel
makes the console read in a loop, and a stream that syncs and ends at once, over and over, backs
off and stops like any other failure.

## Acceptance

AC1. The red test of commit ebd77d7,
     `adv_S4_pages_that_add_nothing_under_a_huge_behind_read_in_a_loop_without_pause`, passes
     with its assertions unchanged; every other test keeps passing.
AC2. In `pageIn`, a page that adds no new index inside the range it asked for is a failed page
     and goes through `fail()` (the 1, 2, 4 s backoff and the stop after 5). One attach episode
     reads at most `ceil((through - lastIndex) / 256)` pages; past that, `fail()`.
AC3. Pages are reduced incrementally: each page is fed to the view once, not re-reduced with
     every earlier page (a test counts `reduce` calls, or the work, for 10 pages: linear).
AC4. The try count resets on `synced` only once the stream has stayed open 10 s after it, or
     delivered a live item after it. A stream that syncs and ends at once, every time, backs off
     1, 2, 4 s and stops after 5 tries with an `unknown` row (test on fake timers). The
     immediate reattach after a healthy synced stream stays.
AC5. `npm run build` rebuilt; the committed `ui/` matches it.

## Spec

TECHNICAL 9.4, 18.3; the S4-fix3 brief (this refines its AC2 reset rule). Do not edit `design/`,
`kernel/`, `factory/` or `AGENTS.md`.

## Interface

Unchanged.

## Files

`console/kit/kernel.ts`, `console/kit/*.test.ts`, `console/kit/adv_S4.test.tsx` (no assertion
changed), `ui/**` (built only).

## Changed tests

`console/kit/kernel.test.ts` tests that assert a reset right on `synced`
(`ac2_failed_attach_backs_off_and_a_sync_resets_the_count`,
`ac2_end_after_synced_reattaches_at_once_and_only_once_then_waits_and_resets`,
`ac2_streams_that_sync_and_end_at_once_reattach_at_once_only_once_in_a_row`): the reset now
needs 10 s open or a live item after `synced` (AC4); what they prove otherwise stays.

## Out of scope

The scan; index order checks; the real kernel adapter.
