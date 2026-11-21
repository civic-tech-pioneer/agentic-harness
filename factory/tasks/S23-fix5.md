Folder: console
Effort: medium
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary pass 4, BREACHED 1).

## Goal

Close the class, not the case: after every message the reducer takes, each tool row shows an open
ask or check only if it is exactly the open entry (`x.open`) for that id on that row's own call;
else the row shows the next open ask or check on its call, or none. A decided card keeps its own
call, never another call's. Passes 1-4 each found one more stream that broke this when an ask id
moves between calls; this task makes it hold for every stream.

## Acceptance

AC1. `console/kit/adv_S23_fix4_decided.test.ts` `adv_S23_fix4_decision_on_old_call_leaves_no_card_asking`
     passes, unchanged.
AC2. The decision branch of `case "ask"` syncs every row that held the removed open entry (not
     only the decision's `toolCallId`), and when the decision names a call other than the one the
     ask was last open on, the decision's card keeps the decision's own call (`ownCall` does not
     take the moved entry's call for another call id).
AC3. Invariant, randomized: a kit test (seeded, about 1 000 sequences, under 1 s) over streams of
     asks and checks with ids from a small shared pool (moves, decisions naming either call,
     reopen, tool updates) asserts after every step: (a) each tool row's undecided `ask` is the
     same object as an `x.open` entry with that id and `toolCallId === row.call.id`; (b) every
     `x.open` ask of kind `ask` has exactly one row showing it, or its call shows another open
     ask first (9.8: first open ask shows); (c) `openAsks` equals the ids of `x.open` asks of kind
     `ask`; (d) a row whose `ask` has a decision shows its own call id. It fails on the code before
     this task (show it).
AC4. Held, unchanged: `src/adv_S23_fix3_moved.test.tsx`, `kit/s23_fix4_moved_reducer.test.ts`,
     `src/s23_fix4_stale_answer.test.tsx`, `src/adv_S23_fix_answered.test.tsx`,
     `src/answered_S23_fix3.test.tsx`, `src/adv_S23_answered.test.tsx`,
     `src/validator_S23_fix1.test.tsx`, `src/answered_S23.test.tsx`, `kit/thread.test.tsx`, the
     reducer's tests (`kit/reduce.test.ts`, `kit/s7_fix1*.test.ts`,
     `kit/regress_S15_fix1_entry_call.test.ts`, `kit/adv_S15.test.tsx`), and every held case of
     S23-1's AC11, assertions unchanged.
AC5. `npm test` time: the new random test under 1 s.

## Spec

TECHNICAL 18.3 (the ask's card from the ask entry only; a decided ask's card keeps the ask's call),
9.8 (first open ask shows), 14 "An ask shows the tool call that will run". Briefs S23-fix3, S23-fix4.

## Interface

None.

## Files

`console/kit/reduce.ts`, tests in `console/kit/`. Do not build `ui/`.

## Changed tests

None expected.

## Out of scope

`answer` carrying the call id (a kernel interface change, pass 4 RISK); an ask on a call that
already shows another open ask getting its own card (pass 4 RISK, pre-existing 9.8 rule).
