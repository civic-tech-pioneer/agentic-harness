Folder: console
Effort: medium
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The user-sim rejected outcome 1 a second time: since S9-fix4, "Needs you" scrolls the ask card's
header into view, but when the card is taller than the transcript area (1280×800, 760×900) the
focused Allow once ends up hidden under the dock. After this task, after "Needs you", the card's
header AND its ask bar (with the focused Allow once) are both in view at 1440×900, 1280×800 and
760×900. Foreman decision (section 4, cheap to undo, inside `console/`): while a card is asking,
its body (diff and Input) has a capped height and scrolls on its own, so header and ask bar fit
together; if the area is still too short, the ask bar wins (focus must be visible), and the
status line already names the subject.

## Acceptance

AC1. `console/test/regress_S9_needs_you_focus_visible.slow.mjs` (commit 98c7ae5) passes unchanged
     at all its sizes.
AC2. While asking, the card body (diff + Input) has a max height (e.g. `max-height: 40vh`, tune it
     so AC1 holds and the 1440×900 page still shows the header and ask bar together), with
     `overflow: auto`; the scroll region is keyboard reachable (`tabindex="0"`) and named
     ("Changes and input"). Not asking: no cap.
AC3. Needs you: scroll the card to `start` when the whole card fits the area; otherwise scroll so
     the ask bar is fully in view (`block: "end"` on the ask bar or equivalent). Focus Allow once
     with `preventScroll: true`. Update the jsdom tests of S9-fix4 to these rules.
AC4. `npm test`, `npm run test:slow`, typecheck pass; `ui/` rebuilt.

## Spec

TECHNICAL 18.6 (Asks: Needs you; Accessibility), 18.4.

## Interface

none.

## Files

`console/src/App.tsx`, `console/src/App.test.tsx`, `console/kit/parts/rows.tsx`,
`console/kit/styles/kit.css`, `console/kit/frame_S9.test.tsx`, `ui/`.

## Changed tests

`src/App.test.tsx` S9-fix4 tests of the Needs-you scroll (`ac2_needs_you_scrolls_*`): to AC3.
The slow regress test must not be edited.

## Out of scope

The focus ring left on the pill (recorded for the PO).
