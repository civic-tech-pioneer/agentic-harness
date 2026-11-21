Folder: console
Effort: medium
Depends on: none
Slice: S7 — Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and presses Stop without lag (P14).

## Goal

`npm test` stays within the suite-time budget. The user-sim measured it on an idle machine at
13.6–15.4 s on slice/S7 against 9.2–9.4 s on main 1b5d324 (A/B), with `src/long_S7.test.tsx`
taking 5.9–8.6 s of it (red test `console/test/regress_S7_npm_test_time.mjs`, commit 762c0f6,
budget 8.6 s). Foreman decision: the sketch requires the large-transcript fixture with its limits
in `npm test`, so the budget becomes 20% over main measured the same way (README "Time and
value"): 11.0 s; change `BUDGET_S` in the red test to 11.0 with a comment naming this decision,
and nothing else in it.

## Acceptance

AC1. `node test/regress_S7_npm_test_time.mjs` passes with `BUDGET_S = 11.0` on this machine when
     no other heavy process runs (check `uptime` before; quote it and three runs).
AC2. `src/long_S7.test.tsx` runs in at most 3 s inside the default run, keeping every limit of
     S7-2 AC7 (input 50 ms, Stop 100 ms, scroll 100 ms, stream 100 ms p95 over 20 samples; end shown
     within 2 s of open) on the 5 000-entry fixture with the stream at 50 deltas/s. Ways: one
     mount shared by the limit tests, the full-history page-in not awaited where a limit does not
     need it, fake timers for the governor's pacing where the limit is not about it, cheaper
     waits. No limit loosened, no sample count lowered, no assertion dropped.
AC3. The other new S7 test files stay cheap: `kit/s7_fix1_random.test.ts`, `kit/s7_fix1.test.ts`,
     `kit/window_S7.test.tsx`, `src/follow_S7_fix2.test.tsx`, `src/follow_S7_fix3.test.tsx`,
     `kit/code_S7_3.test.tsx`: any file over 1 s gets its cost cut without weakening (fewer random
     sequences only if the count stays ≥ 1 000; say what each costs before and after). A DOM-free
     file runs in the node environment.
AC4. Held cases with assertions unchanged (timing-only setup changes allowed and listed):
     `console/kit/adv_S9.test.tsx`, `adv_S9_diff.test.tsx`, `adv_S15.test.tsx`, `adv_S4.test.tsx`,
     `regress_S15_*`, `s7_4.test.tsx`, `follow_S7_fix2.test.tsx`, `follow_S7_fix3.test.tsx`.
     `npm test`, typecheck, `factory/gate.sh` pass.

## Spec

`.claude/factory/README.md` "Test pyramid", "Time and value"; S7-2 AC7 (the limits).

## Interface

None.

## Files

`console/src/long_S7.test.tsx`, the test files of AC3, `console/test/regress_S7_npm_test_time.mjs`
(`BUDGET_S` and its comment only), `console/vitest.config.ts`.

## Changed tests

`regress_S7_npm_test_time.mjs`: budget 8.6 -> 11.0 s (foreman decision above).

## Out of scope

Product code; the page-scroll fix (S7-fix4).
