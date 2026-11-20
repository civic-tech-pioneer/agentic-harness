Folder: console
Effort: medium
Depends on: none
Slice: S4 — Alice opens the console on a scripted fake kernel and sees a session as TECHNICAL 18 describes, and after a reload the same transcript with nothing doubled.

## Goal

When Alice opens the session she sees its end at once, the open ask's bar in view (18.3: "the end
of the transcript shows at once"), with the dock compact enough to leave the transcript room;
and three small things the acceptance run found are put right.

## Acceptance

AC1. The red test of commit 12f226f, `console/test/regress_S4_open_shows_end.slow.mjs`, passes
     unchanged, and `npm run test:slow` runs it with the layout check.
AC2. On open, and on every replay that grows the transcript while the reader is within 40 px of
     the end, the transcript scrolls to its end (18.6 Place, the follow-at-end rule; the "New
     activity below" pill stays out of scope). A component test: after the story plays, the
     scroll container's `scrollTop` was set to its end (jsdom: assert the call or the value set).
AC3. The plan folds to its one-line label ("Plan · 1 of 3 done") while an ask waits or inputs
     are queued, unless the participant opened it (18.6 Plan); a test for each case.
AC4. Esc in the Deny reason field closes it and puts focus back on "Deny…"; Cancel does the same.
AC5. The fake's story sets the ask's `expiresAt` (and the waiting turn's) relative to the time
     the fake is made (now + 10 minutes), so the status line counts down; tests that pin the
     old constant follow.
AC6. `npm run build` rebuilt; the committed `ui/` matches it; `npm test` and `npm run test:slow`
     pass.

## Spec

TECHNICAL 18.3, 18.5, 18.6 (Place, Asks, Plan); design/FRONTEND-2.md findings 1, 8, 11. Do not
edit `design/`, `kernel/`, `factory/` or `AGENTS.md`.

## Interface

Unchanged.

## Files

`console/src/**`, `console/kit/**`, `console/fake/**`, `console/test/**`, `console/package.json`
(the `test:slow` script), `ui/**` (built only).

## Changed tests

Tests that assert the story's fixed `expiresAt` value or the time left in the status line
(`console/kit/thread.test.tsx`, `console/src/App.test.tsx`, `console/kit/reduce.test.ts`): they
take the time from the fake's clock; what they prove stays. The AC1 tests of App that expect the
plan's items listed in full while an ask waits: they expect the folded label.

## Out of scope

The "New activity below" pill; queue edit and remove; the fake answering asks; session list
grouping.
