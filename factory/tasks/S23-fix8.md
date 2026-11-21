Folder: console
Effort: medium
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (user-sim REJECTED outcomes 1 and 4: answering blanks the page).

## Goal

Answering an ask, or Needs you to an ask at the end, never blanks the page: the transcript's
anchor hold never asks for a scroll position the content cannot reach and never re-renders in a
loop. The "Answered, waiting" line looks like the bar's row (same padding), and a refused answer
puts focus back on the card's Allow once.

## Acceptance

AC1. `console/test/regress_S23_answer_blanks_page.slow.mjs` passes, unchanged.
AC2. `kit/thread.tsx`: the anchor hold clamps the wanted `scrollTop` to `0 .. scrollHeight -
     clientHeight` and calls `tick()` only when the scroll position actually changed; no commit
     loop is possible when the content shrinks below the anchor. A jsdom test with stubbed
     `scrollHeight`/`clientHeight`/`scrollTop` (a clamping setter) shows a shrinking card ends in a
     bounded number of renders.
AC3. A slow Chromium script (may extend AC1's): Enter on Allow once, "Allow `<reach>` this session",
     Deny with a reason, a `busy` refusal, and Needs you to a second ask at the end of the
     transcript, at 1440 and 760 px: no page error, the root stays mounted.
AC4. The answered line (`.wa-answered`) has the same padding as the ask bar row, light and dark.
AC5. After a refusal other than `{field: "ask"}`, focus is on the card's Allow once (no scroll),
     not `document.body`.
AC6. Held, unchanged: the S4, S9, S15 and S7 adversaries' held cases (`kit/adv_S4.test.tsx`,
     `kit/adv_S9.test.tsx`, `kit/adv_S9_diff.test.tsx`, `kit/adv_S15.test.tsx`, `kit/regress_S15_*`,
     `src/follow_S7_fix2.test.tsx`, `src/follow_S7_fix3.test.tsx`, `test/adv_S9_*.slow.mjs`,
     `test/regress_S9_*.slow.mjs`, `test/adv_S7_*.slow.mjs`), every S23 test (`*S23*`, `*s23*`,
     `answered_*`, `validator_S23_*`, `ac2_*`), `test/long_session.slow.mjs`,
     `test/regress_S4_open_shows_end.slow.mjs`.

## Spec

TECHNICAL 18.6 Place and Long sessions (the reader's row held still), 18.4. Briefs S23-1, S7-2.

## Interface

None with S23-fix9 (it touches status, sidebar, App).

## Files

`console/kit/thread.tsx`, `console/kit/parts/askbar.tsx`, `console/kit/parts/rows.tsx`,
`console/kit/styles/kit.css`, tests in `console/kit/` and `console/test/`. Not `console/src/App.tsx`.
Build `ui/` locally for the slow tests, but do not commit it.

## Changed tests

None.

## Out of scope

The status subject, sidebar and Loading on open (S23-fix9).
