Folder: console
Effort: medium
Depends on: S7-2
Slice: S7 — Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and presses Stop without lag (P14).

## Goal

Reducing one kernel message costs about the same at row 10 as at row 5 000, so opening a long
session gives no main-thread stall from the reducer. S7-2 measured about 0.35 ms per message at
5 000 rows in jsdom (1.9 s of CPU to reduce the whole history, a ~0.8 s stall when it arrives),
with `uniqueKeys` (rebuilds a Set of all keys per message), `find` (a linear scan of all rows per
tool call) and `reduce`'s full `rows.slice()` as the hot spots. What the reducer produces does not
change: every existing reducer and view test passes unchanged.

## Acceptance

AC1. Cost per message does not grow with the transcript: a node test (`// @vitest-environment
     node`) reduces `longStory(5000)`'s entries (S7-2's `console/fake/long.ts`) one message at a
     time and asserts the total is at most 400 ms and that the mean cost of the last 500 messages
     is at most 2 x the mean of the first 500 (prints both).
AC2. Same output: for `longStory(500)` and for the scripted story (`fake/story.ts`), the view after
     reducing every message equals, deep, the view the reducer at 5882c51 gives (keep a copy of
     the old reducer only inside the test file if you need it, or compare against a recorded
     JSON fixture taken before your change).
AC3. Allowed means: an index from call id and row key to row position kept in the view's internal
     `x` (rebuilt or patched when rows are spliced), keys checked against that index instead of a
     new Set per message, and the rows array copied only when it changes. The view stays an
     immutable value to React (a new `rows` array whenever a row changes; unchanged rows keep
     their identity).
AC4. The held cases pass with their assertions unchanged: `console/kit/adv_S9.test.tsx`,
     `adv_S9_diff.test.tsx`, `adv_S15.test.tsx`, `adv_S4.test.tsx`,
     `regress_S15_fix1_entry_call.test.ts`, `regress_S15_fix2_bar_resets.test.tsx`, `s7_4.test.tsx`,
     and the slow `test/adv_S9_ask_scroll_hides_input.slow.mjs`. `npm test`, `npm run
     typecheck`, `factory/gate.sh` pass, `ui/` rebuilt and committed.

## Spec

TECHNICAL 18.3 (the table; the ask card rules of 9.8 as the reducer keeps them); 17 (budgets);
`research_notes/performance-review-2026-11-20.md` P14. TECHNICAL 14 "An ask shows the tool call
that will run": the index must never point a card at another call's row.

## Interface

None new. `Internal` in `console/kit/model.ts` may gain index fields (internal only).

## Files

`console/kit/reduce.ts`, `console/kit/model.ts` (`Internal` only), new tests in `console/kit/`,
`ui/` (rebuilt).

## Changed tests

None: all reducer and view tests keep their assertions.

## Out of scope

The window and frame coalescing (S7-2, done); the kernel's governor pacing of `read` at open.
