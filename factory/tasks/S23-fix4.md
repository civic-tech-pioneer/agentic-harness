Folder: console
Effort: medium
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary pass 3, BREACHED 1; pass 1 RISK b).

## Goal

An answer goes out only for the ask its card shows. When an open ask's id moves to another tool
call (a later ask entry with the same id and another `toolCallId`, no decision between), the old
call's card no longer shows that ask open (no bar, not `data-asking`); and the console never sends
an answer whose id's open ask is not the ask object the clicked card drew.

## Acceptance

AC1. `console/src/adv_S23_fix3_moved.test.tsx` `adv_S23_fix3_moved_ask_old_card_has_no_bar` passes,
     unchanged.
AC2. Reducer (`kit/reduce.ts`, the open branch of `case "ask"`): when the new open ask replaces an
     open ask of the same id on another call, the old call's row is synced as well, so it shows
     the next open ask or check on that call, else no open ask (its card keeps its call; it is
     not `data-asking` and has no bar). A kit-level test shows this on the reducer's view.
AC3. Fail closed at the send: the card passes the ask object it drew with its answer
     (`Actions.answer(id, decision, ask?)` or equivalent), and App sends `kernel.answer` only if
     `openAsk(view, id)` is that same object; else nothing is sent, nothing is recorded as
     answered, and the card shows its refused line (or simply its bar again). If `openAsk` is
     undefined, nothing is sent. A unit test drives App with a stale ask object and finds no
     `answer` call in the fake kernel's `calls`.
AC4. Held, unchanged: `src/adv_S23_fix_answered.test.tsx`, `src/answered_S23_fix3.test.tsx`,
     `src/adv_S23_answered.test.tsx`, `src/validator_S23_fix1.test.tsx`, `src/answered_S23.test.tsx`,
     `kit/thread.test.tsx`, the reducer's tests (`kit/reduce.test.ts`, `kit/s7_fix1*.test.ts`,
     `kit/regress_S15_fix1_entry_call.test.ts`), and every held case of S23-1's AC11 (the S4, S9,
     S15, S7 adversaries' tests), assertions unchanged.
AC5. Nothing else changes.

## Spec

TECHNICAL 18.3 (the ask's card is drawn from the ask entry only), 9.8 (a second ask on a call),
14 "An ask shows the tool call that will run". Briefs S23-1, S23-fix1, S23-fix3.

## Interface

None with siblings.

## Files

`console/kit/reduce.ts`, `console/kit/model.ts`, `console/kit/parts/askbar.tsx`,
`console/kit/parts/rows.tsx`, `console/src/App.tsx`, tests in `console/kit/`, `console/src/`.
Do not build `ui/`.

## Changed tests

None expected; a test harness that builds `Actions` may pass the ask object (no assertion changes).

## Out of scope

An identical re-delivery of an ask showing its bar again (pass 3 RISK: a spec question for the PO).
