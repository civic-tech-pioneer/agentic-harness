Folder: console
Effort: low
Depends on: none
Slice: S32 — stage A, end to end (user-sim run 1: REJECTED 1).

## Goal

A session shows a turn footer ("Completed in n s · m tools") only at the end of a turn the
console saw running (TECHNICAL 18.3: "at a turn's end, a footer line"). Today `setTurn` in
`console/kit/reduce.ts` adds a footer whenever the first `turn` after an attach is idle (the
previous turn is undefined), so a probe session that never ran a turn shows "Completed in 0 s · 0
tools", at a different place after each reload.

## Acceptance

AC1. The red test `console/kit/regress_S32_probe_footer.test.ts`
     (`regress_S32_probe_session_shows_no_turn_footer_and_a_reload_draws_the_same_rows`) passes,
     unchanged.
AC2. A turn seen `running` (or `waiting`) and then `idle` in the same attach still gets its footer,
     as today (the existing footer tests pass unchanged).
AC3. Nothing else changes: `npm test` and `npm run test:slow` pass with every assertion
     unchanged. Held cases: S9 `kit/adv_S9*.test.tsx`, `kit/regress_S9_*`,
     `test/adv_S9_*.slow.mjs`, `test/regress_S9_*.slow.mjs`; S15 `kit/adv_S15.test.tsx`,
     `kit/regress_S15_*`; S7 `src/follow_S7_fix2.test.tsx`, `src/follow_S7_fix3.test.tsx`,
     `test/adv_S7_*.slow.mjs`; S23 `kit/adv_S23_*`, `src/adv_S23_*`,
     `kit/regress_S23_status_subject_after_answer.test.tsx`, `kit/s23_fix5_open_invariant.test.ts`,
     `test/adv_S23_*.slow.mjs`, `test/regress_S23_*.slow.mjs`; S30 `kit/short_S30_1.test.ts`,
     `src/short_S30_1.test.tsx`, `src/s30_2_status.test.tsx`, `kit/s30_2_status.test.tsx`,
     `kit/adv_S30_status_names_the_open_ask.test.tsx`; S32 `kit/watane.test.ts`,
     `src/served.test.tsx`, `test/served_S32.slow.mjs`, `test/served_S32_no_watane.slow.mjs`.
     If an existing test asserts a footer after a first idle `turn` with no running turn before it,
     stop and say so (it would be a changed test).

## Spec

TECHNICAL 18.3 (`turn` row), 18.5, 9.4 (attach: `synced`, then the turn).

## Interface

None.

## Files

`console/kit/reduce.ts`. Do not build or commit `ui/` (the foreman does).

## Changed tests

None (searched `console/` for "Completed in" and `footer`: the footer tests play a running turn
first).

## Out of scope

The status line's "Idle" while probe items arrive, a second tab's silent freeze, the reconnect
limit (18.2): for the product owner.
