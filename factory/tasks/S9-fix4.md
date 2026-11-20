Folder: console
Effort: medium
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The user-sim rejected outcome 1: the ask diff shows an unchanged last line as removed and added
again when the old text has no final newline and the new text appends a line (the usual shape of
a Claude Edit). It also found that after "Needs you" the tall ask card's header ("Edit
src/sync.ts", "Waiting for you") is scrolled out of sight, so Alice sees the buttons but not what
asks (outcome 1: "an ask must read as one"). After this task the diff marks only real changes,
and Needs you brings the card's header into view while focus still lands on Allow once.

## Acceptance

AC1. `console/kit/regress_S9_diff_last_line.test.tsx` (commit 53af970) passes, unchanged. Compare
     lines, not text with or without a final newline (e.g. normalise both sides to end with one
     newline before `diffLines`, or diff arrays of lines); every existing diff test still passes.
AC2. Needs you: the first open ask's card (`[data-call="<id>"]`) is scrolled into view with its
     top at the top of the transcript area when it fits, else its top edge (`block: "start"`,
     or `"nearest"` when the card is shorter than the area), and Allow once gets focus with
     `preventScroll: true`, so the focus does not scroll the header away again. A view test
     asserts the scroll target is the card and the focus call. The open-on-load path (S4-fix6:
     the page opens at the end) keeps its test passing.
AC3. `npm test`, typecheck pass; `ui/` rebuilt; `npm run test:slow` passes (layout and the S4
     open-at-end probe; Chromium is preinstalled, `require('playwright')` works).

## Spec

TECHNICAL 18.3, 18.6 (Asks: Needs you), 18.2 (diff view). Do not edit `design/`.

## Interface

none.

## Files

`console/kit/parts/diff.tsx`, `console/src/App.tsx`, `console/src/App.test.tsx`,
`console/kit/frame_S9.test.tsx`, `ui/`.

## Changed tests

none expected; the regress test must not be edited.

## Out of scope

The sidebar hold at load, the ask bar's wrap at 760 px, Stop's place in the composer (recorded
for the PO).
