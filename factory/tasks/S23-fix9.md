Folder: console
Effort: low
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (user-sim REJECTED outcomes 1 and 6: status subject, sidebar, Loading on open).

## Goal

After Alice answers the only open ask, the status line names no other call and the sidebar
leaves "Needs you" together with the dock. A session that opens says "Loading" from its first
frame, never "Idle".

## Acceptance

AC1. `console/kit/regress_S23_status_subject_after_answer.test.tsx` passes, unchanged: with
     `openAsks` empty, `askSubject` is empty (no `undefined === undefined` match); the word is
     "Waiting for you" alone.
AC2. `console/src/regress_S23_sidebar_after_answer.test.tsx` passes, unchanged: the sidebar groups
     the open session under "Needs you" exactly when `needs` is true; a `waiting` turn with
     `needs` false goes under "Running" (its glyph `g-run`). An idle or paused turn is as today.
     The hold under the pointer still holds (`src/sidebar_S23_3.test.tsx`).
AC3. `console/src/regress_S23_idle_on_open.test.tsx` passes, unchanged: until the current
     subscription's first view, App's view is `emptyView` with `loading: true` (Loading, shape
     `loading`), never "Idle"; the fix7 rules (`src/adv_S23_fix6_aba.test.tsx`,
     `src/ac2_subscription_token.test.tsx`, `src/ac2_strict_mode.test.tsx`) hold unchanged.
AC4. Held, unchanged: every S23 test, `src/shell.test.tsx` (incl.
     `ac5_open_row_follows_the_turn_in_the_app`), `src/App.test.tsx`, `kit/status*` tests, and the
     S4, S9, S15 and S7 adversaries' held cases.

## Spec

TECHNICAL 18.5, 18.6. Briefs S23-3, S23-fix7.

## Interface

None with S23-fix8.

## Files

`console/kit/status.tsx`, `console/src/Sidebar.tsx`, `console/src/App.tsx`, tests in `console/kit/`,
`console/src/`. Do not build `ui/`.

## Changed tests

None expected.

## Out of scope

The anchor loop and the answered line's look (S23-fix8).
