Folder: console
Effort: low
Depends on: S15-fix1
Slice: S15 — Alice's console works against the kernel JSON of S10 unchanged.

## Goal

Adversary RISK (d) of S15's rerun, settled by the foreman as the narrower option: when an ask
card switches from a decided entry to another open entry on the same call, the ask bar starts
fresh. No deny form, typed reason or button focus carries over from the decided entry, so a press
meant for the first entry cannot answer the second.

## Acceptance

AC1. With asks a1 and a2 open on one call: open "Deny…" on a1's card and type a reason; a decision
     entry for a1 arrives; the card now shows a2 with no deny form open and no reason typed.
AC2. With focus on a1's Allow once, a decision for a1 arrives: Allow once of a2 does not have
     focus (focus is not on any answer button of the card).
AC3. Nothing else changes: every other test passes unchanged, including `adv_S9*.test.tsx`,
     `adv_S15.test.tsx`, `s15_1.test.tsx`, `regress_S15_fix1_entry_call.test.ts`, and
     `npm run test:slow`. `npm run typecheck` passes; `ui/` rebuilt and committed.
AC4. A test for AC1 and AC2 in `console/kit/adv_S15.test.tsx` style, new file
     `console/kit/regress_S15_fix2_bar_resets.test.tsx`.

## Spec

`design/TECHNICAL.md` 14 (row "An ask shows the tool call that will run"), 18.6 Asks.

## Interface

None. Likely `key={ask.id}` on `AskBar` in `console/kit/parts/rows.tsx`.

## Files

`console/kit/parts/rows.tsx`, `console/kit/parts/askbar.tsx`, the new test, `ui/` by `npm run build`.

## Changed tests

none (searched `console/kit/*.test.*`, `console/src/*.test.*`).

## Out of scope

A delay before buttons work after a switch.
