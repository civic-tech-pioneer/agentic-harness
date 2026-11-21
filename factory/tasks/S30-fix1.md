Folder: console
Effort: medium
Depends on: none
Slice: S30 — the console draws a short decision (adversary pass 1: BREACHED 1).

## Goal

The status line's subject on a `waiting` turn names the call of the ask that is open now, the same
card Needs you goes to and the clock counts for. Today `askSubject` (`console/kit/status.tsx`)
takes the first row whose `ask.id` matches, decided or not, so after an ask id is reused on
another call the line names the old, decided call (TECHNICAL 14 "An ask shows the tool call that
will run"; 18.5 `waiting`).

## Acceptance

AC1. The red test `console/kit/adv_S30_status_names_the_open_ask.test.tsx`
     (`adv_S30_status_subject_names_the_open_asks_call_not_an_old_card_of_its_id`) passes,
     unchanged.
AC2. Nothing else changes: every other test passes with its assertions unchanged. Held cases of
     earlier adversaries: S9 `kit/adv_S9*.test.tsx`, `kit/regress_S9_*`,
     `test/adv_S9_*.slow.mjs`, `test/regress_S9_*.slow.mjs`; S15 `kit/adv_S15.test.tsx`,
     `kit/regress_S15_*`; S7 `src/follow_S7_fix2.test.tsx`, `src/follow_S7_fix3.test.tsx`,
     `test/adv_S7_*.slow.mjs`; S23 `kit/adv_S23_*`, `src/adv_S23_*`,
     `kit/regress_S23_status_subject_after_answer.test.tsx`, `kit/s23_fix5_open_invariant.test.ts`,
     `test/adv_S23_*.slow.mjs`, `test/regress_S23_*.slow.mjs`; S30 `kit/short_S30_1.test.ts`,
     `src/short_S30_1.test.tsx`, `src/s30_2_status.test.tsx`, `kit/s30_2_status.test.tsx`.
     Run `npm test` and `npm run test:slow`.
AC3. The subject and the clock come from the same ask object (`openAsk(view, id)`): name the row
     whose `ask` is that very object (or build the name from the open entry's own call).

## Spec

TECHNICAL 18.5 (`waiting`), 14 "An ask shows the tool call that will run", 9.8 (a closed id may
open again).

## Interface

None.

## Files

`console/kit/status.tsx`; `console/kit/reduce.ts` only to export a read helper. Do not build or
commit `ui/` (the foreman does; `test/build.test.ts`'s ui/ check is expected to fail).

## Changed tests

None (searched `console/` for `askSubject` and "Waiting for you ·").

## Out of scope

Earliest-expiry clock, an answered ask plus an open check, Needs you skipping a focused card
(adversary RISKs, for the PO).
