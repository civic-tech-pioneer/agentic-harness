Folder: console
Effort: medium
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary BREACHED 1 and 2).

## Goal

"Answered, waiting" shows only for the ask this console answered in this session and that is still
open: one source of truth for it, cleared when that ask gets its decision and when the session
changes. A reopened ask id (a later stream reusing it) or another session's ask with the same id
shows its bar.

## Acceptance

AC1. `console/src/adv_S23_answered.test.tsx`
     `adv_S23_reopened_ask_id_shows_answered_waiting_for_an_ask_never_answered` passes, unchanged.
AC2. Same file, `adv_S23_session_switch_carries_answered_waiting_to_the_same_ask_id_of_another_session`
     passes, unchanged.
AC3. The card's line and the dock's count come from one state (e.g. App's answered set passed to
     the rows through `Actions` or a context; or the row's state, cleared as below, read by App),
     so they cannot disagree: an answered ask is removed from it when its decision entry arrives
     (the row's ask gets a `decision`, or the id leaves `openAsks`), on any refusal other than
     `{field: "ask"}`, and when `info.id` changes.
AC4. Held, unchanged: S23-1's tests (`src/answered_S23.test.tsx`, the ac6 hold tests, ac7/ac8,
     ac9, `test/adv_S23_press_in_ask_card_keeps_follow_off.slow.mjs`,
     `test/S23_pill_does_not_cover.slow.mjs`) and every held case of S23-1's AC11 (the S4, S9, S15
     and S7 adversaries' tests), with assertions unchanged.
AC5. Nothing else changes.

## Spec

TECHNICAL 18.3, 18.6 Asks, 9.4 / 9.8 rule 4, 14 "An ask shows the tool call that will run".
Brief S23-1 (AC1-AC5).

## Interface

None with siblings (S23-fix2 touches only highlighting).

## Files

`console/kit/parts/rows.tsx`, `console/kit/parts/askbar.tsx`, `console/kit/model.ts`,
`console/kit/thread.tsx` (only to pass the state down), `console/src/App.tsx`, tests in
`console/kit/`, `console/src/`. Do not build `ui/`.

## Changed tests

None: the red tests pass unchanged; S23-1's tests keep their assertions (a test that renders
`Transcript` with mock actions may need the new state passed in; say so).

## Out of scope

A stale row keeping an old ask when an id is reused on another call (adversary RISK b, a spec
question for the PO); highlighting.
