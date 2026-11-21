Folder: console
Effort: medium
Depends on: none
Slice: S7 — Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and presses Stop without lag (P14).

## Goal

No scroll can resume following the end while focus is inside an open ask's card. The adversary's
rerun (red test `console/test/adv_S7_arrow_key_resumes_follow_with_ask_focused.slow.mjs`, commit
22065b8) showed that S7-fix2's pause ends on any reader scroll that lands within 40 px of the end
(ArrowDown, and by the same code PageDown, End, the wheel, a scrollbar drag) while Allow once
keeps focus; the stream then carries the card out of view again. Foreman decision (narrows 18.6
Place further): the follow is off whenever `document.activeElement` is inside a card with an open
ask (`[data-asking]`), whatever the scroll position; it can resume only after focus has left
such a card (then by a scroll within 40 px of the end, as today) or by the pill (whose click takes
focus).

## Acceptance

AC1. The red test `adv_S7_arrow_key_resumes_follow_with_ask_focused` passes, its assertions
     unchanged; `adv_S7_needs_you_stream_scrolls_ask_away` still passes.
AC2. One check, read at each decision to follow (each scroll event and each view or size change
     that would scroll to the end): "is focus inside an open ask's card", in one function used by
     both `kit/thread.tsx` and `src/App.tsx`. A jsdom test: with Allow once focused and the
     follow paused, a scroll event at the end (gap 0) and a later delta leave `scrollTop`
     unchanged; after focus moves to the composer, a scroll within 40 px of the end resumes it.
AC3. If the S7-fix2 "ignore a scroll event at the paused scrollTop" marker is no longer needed with
     this rule, remove it (keep the code to one rule); if it still is, say why in a comment.
AC4. The pill still works (S7-fix2 AC3 tests unchanged), and an ask decided while focus was in its
     card does not leave the follow stuck off: once focus is not in an open ask's card, the next
     scroll within 40 px of the end or the pill resumes it.
AC5. Held cases with assertions unchanged: `console/kit/adv_S9.test.tsx`, `adv_S9_diff.test.tsx`,
     `adv_S15.test.tsx`, `adv_S4.test.tsx`, `regress_S15_*`, `s7_4.test.tsx`, `window_S7.test.tsx`,
     `src/long_S7.test.tsx`, `src/follow_S7_fix2.test.tsx`, `code_S7_3.test.tsx`, and every slow
     script (`npm run test:slow`). `npm test`, typecheck, `factory/gate.sh` pass; `ui/` rebuilt.

## Spec

TECHNICAL 18.6 (Place, Asks), 14 "An ask shows the tool call that will run". The narrowing is the
foreman's decision; the spec text is for the PO.

## Interface

None between tasks.

## Files

`console/kit/thread.tsx`, `console/src/App.tsx`, `console/kit/index.ts`, new tests in
`console/kit/` and `console/src/`, `ui/` (rebuilt).

## Changed tests

`src/follow_S7_fix2.test.tsx` ac2 asserts "the follow resumes when the reader returns within 40 px
of the end" after focus leaves the card: unchanged (focus has left). None else.

## Out of scope

A second open ask while focus sits on the first (no fixture; a later slice).
