Folder: console
Effort: medium
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The adversary breached "An ask shows the tool call that will run" through S9-fix6's capped scroll
box: the ask's Input (the only place that shows `replace_all`, a differing `file_path`, or a
command) sat below the box's fold with no cue that the box scrolls. After this task the Input is
the first thing in the box, and whenever the box holds more than it shows, a visible cue says so.

## Acceptance

AC1. `console/test/adv_S9_ask_scroll_hides_input.slow.mjs` (commit 97fe4c3) passes unchanged, and
     every other slow test (`npm run test:slow`) still passes, including
     `regress_S9_needs_you_focus_visible` and the validator's header-and-ask-bar check in
     `layout.slow.mjs`.
AC2. Inside `.wa-ask-scroll` the order is: Input block, then the diff(s) (each with its path line
     where S9-fix1 shows one). Not asking: unchanged (diff, no Input).
AC3. While the box's content overflows and is not scrolled to its end, a visible line at its
     bottom edge reads "More below ↓" (muted, on the panel background, part of the card, not
     aria-hidden; e.g. a sticky last child or an element outside the box shown by state), and it
     disappears once the reader reaches the end; it updates on scroll and on resize
     (ResizeObserver when present). jsdom test: with stubbed scrollHeight/clientHeight/scrollTop
     the cue shows and hides; no overflow, no cue.
AC4. `npm test`, typecheck pass; `ui/` rebuilt.

## Spec

TECHNICAL 14 (row "An ask shows the tool call that will run"), AGT-F-42, 18.6.

## Interface

none.

## Files

`console/kit/parts/rows.tsx`, `console/kit/styles/kit.css`, `console/kit/frame_S9.test.tsx`,
`console/kit/thread.test.tsx` (only an order assertion if one pins diff-before-input), `ui/`.

## Changed tests

Any test that pins the Input after the diff (S9-fix1's validator
`ac2_input_block_is_labelled_and_below_the_diff` in `frame_S9.test.tsx`): to Input first. The red
slow test must not be edited.

## Out of scope

Gating Allow on scrolling to the end; diff size limits.
