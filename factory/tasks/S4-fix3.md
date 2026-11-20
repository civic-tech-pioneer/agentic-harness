Folder: console
Effort: medium
Depends on: none
Slice: S4 — Alice opens the console on a scripted fake kernel and sees a session as TECHNICAL 18 describes, and after a reload the same transcript with nothing doubled.

## Goal

Every reattach of the console goes through one paused, bounded path, so no kernel answer or
stream end can make it attach in a busy loop, and an older connection can never overwrite the
view of a newer one.

## Acceptance

AC1. The red test of commit ceaa4a6, `adv_S4_stream_that_ends_at_once_reattaches_without_pause`,
     passes with its assertions unchanged; every other test keeps passing.
AC2. One reattach policy in `console/kit/kernel.ts` for every cause (a stream end other than
     `replaced`, a failed attach, a `behind` that cannot be paged, a failed page): the first
     reattach after a stream that delivered at least `synced` is immediate; any further one
     waits 1 s, then 2 s, 4 s, capped at 30 s; after 5 failed tries in a row the console stops,
     with an `unknown` row naming the cause. The count and the delay reset once an attach
     reaches `synced`. Tests on fake timers for: an end right after `{ok:true}`, an end after
     `synced` (reattaches at once, once), and the reset.
AC3. A connection that is not the current one (`gen` changed, or closed) changes neither the
     view nor the kernel: `pageIn` and every callback check it (a test interleaves a late
     `onEnd`, a pending timer and a page result of an old connection, and finds the view of the
     new one).
AC4. `npm run build` rebuilt; the committed `ui/` matches it.

## Spec

TECHNICAL 9.4 (attach, replaced), 18.3 (on every reconnect); S4-fix1 and S4-fix2 briefs (this
replaces fix2's AC3 counting). Do not edit `design/`, `kernel/`, `factory/` or `AGENTS.md`.

## Interface

Unchanged.

## Files

`console/kit/kernel.ts`, `console/kit/*.test.ts`, `console/kit/adv_S4.test.tsx` (no assertion
changed), `ui/**` (built only).

## Changed tests

`console/kit/kernel.test.ts` `ac3_stuck_behind_waits_1s_between_attaches_and_stops_after_3_with_an_unknown_row`
and `ac3_non_integer_behind_is_named_not_an_index_and_never_paged`: the timings and the stop
count follow AC2 (backoff, 5 tries); "never paged" and the named cause stay.

## Out of scope

The scan; index order checks.
