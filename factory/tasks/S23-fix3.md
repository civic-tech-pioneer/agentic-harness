Folder: console
Effort: medium
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary pass 2, BREACHED 1).

## Goal

"Answered, waiting" is tied to the exact ask the participant answered, not to its id: an ask
reopened under the same id on another tool call (or with another input) shows its own bar, and
the stale row of the old call never says "Answered, waiting" for an ask that is no longer open.
A late refusal of an earlier answer cannot clear a newer answer to a replaced ask.

## Acceptance

AC1. `console/src/adv_S23_fix_answered.test.tsx`
     `adv_S23_fix_ask_reopened_under_its_id_on_another_call_shows_answered_without_a_bar` passes,
     unchanged.
AC2. The answered state records the ask that was answered (its id with its `toolCallId` and its
     ask object or its input), and an ask counts as answered only where the ask open for that id
     now (the reducer's open asks, not the first row holding the id) is that same ask; a card shows
     "Answered, waiting" only when its own row's ask is that same open ask. The dock, status
     subject and sidebar read the same filtered state as the card (one source, as S23-fix1).
AC3. A refusal (other than `{field: "ask"}`) of an answer removes that answer only if the recorded
     answer for the id is still the one the refusal belongs to; the row's refused line likewise
     belongs to the ask object it was set for. A unit test: answer a8, replace a8 under the same id,
     answer again, then refuse the first answer `busy`: the second card keeps "Answered, waiting".
AC4. Held, unchanged: `src/adv_S23_answered.test.tsx`, `src/validator_S23_fix1.test.tsx`,
     `src/answered_S23.test.tsx`, `kit/thread.test.tsx`, every held case of S23-1's AC11 (the S4,
     S9, S15 and S7 adversaries' tests), with assertions unchanged.
AC5. Nothing else changes.

## Spec

TECHNICAL 18.3, 18.6 Asks, 9.4 / 9.8 rule 4, 14 "An ask shows the tool call that will run".
Briefs S23-1, S23-fix1.

## Interface

None with siblings.

## Files

`console/src/App.tsx`, `console/kit/parts/rows.tsx`, `console/kit/model.ts`, `console/kit/reduce.ts`
(read only, or to expose the open asks' objects if needed without changing the view's other
fields), tests in `console/src/`, `console/kit/`. Do not build `ui/`.

## Changed tests

None.

## Out of scope

The reducer keeping a stale row's old ask when an id is reused on another call (first pass RISK b),
and a re-read rebuilding ask objects (pass 2 RISK b): spec questions for the PO.
