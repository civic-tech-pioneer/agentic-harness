Folder: console
Effort: low
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary pass 5, BREACHED 1, and a RISK).

## Goal

An answer always goes to the session whose card was clicked. When App gets a new `info.id`, the
old session's view leaves the screen at once (the new session shows empty until its first view),
and an answer drawn on another session's view is never sent. Needs you finds the open card of an
id, never a decided card that shares it.

## Acceptance

AC1. `console/src/adv_S23_fix5_session_switch.test.tsx`
     `adv_S23_fix5_answer_on_old_session_card_goes_to_new_session` passes, unchanged.
AC2. The view App keeps is tagged with the session it belongs to; on a new `info.id` App shows
     `emptyView` until that session's first view, and `answerDrawn` sends only when the view it
     checks belongs to the current `info.id` (fail closed otherwise). A late view from the old
     session's `openSession` (after its `close`) is never shown under the new id.
AC3. Needs you (and the measure that hides it) look up the card of an id among open-ask cards only
     (`[data-ask="<id>"][data-asking]`), so a decided card that keeps the same `data-ask` is never
     the target. A unit test: an id decided on one call and reopened on another; Needs you puts
     focus on the open card's Allow once.
AC4. Held, unchanged: every S23 test in `console/src/` and `console/kit/` (answered, adv_S23_*,
     s23_fix4_*, s23_fix5_*, validator_S23_fix1), `src/shell.test.tsx`, `src/App.test.tsx`, and every
     held case of S23-1's AC11.

## Spec

TECHNICAL 18.1 (a client like any other), 18.3, 18.6 Asks, 14 "An ask shows the tool call that will
run". Briefs S23-1, S23-fix1, S23-fix4.

## Interface

None.

## Files

`console/src/App.tsx`, tests in `console/src/`. Do not build `ui/`.

## Changed tests

None expected.

## Out of scope

`answer` carrying the call id (kernel interface; pass 4 RISK); `settle` trusting the kernel's
`read` (pass 5 RISK); a reopened id keeping the old entry's output (pass 5 RISK, display only).
