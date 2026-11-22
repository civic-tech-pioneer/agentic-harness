Folder: console
Effort: medium
Depends on: none
Slice: S32 — stage A, end to end: Alice watches a real kernel stream in the console.

## Goal

When Alice presses Deny… on an open ask, the reason form (the "Tell the agent why" field, Cancel,
Deny) opens where she can see it, as `design/frames/Main-2` shows it inside the ask card. Today, in
the scripted session (`npm run dev`), the form opens below the visible part of the thread: the
focused field sits under the dock and the "Needs you" pill, and she must scroll to see where she
types (`console/kit/parts/askbar.tsx` focuses the field with `preventScroll: true` and nothing
scrolls it into view). After this task, opening the form scrolls the transcript just enough that the
whole form is in view, clear of the dock and of any pill over the transcript; when it is already in
view nothing scrolls.

## Acceptance

AC1. A component test (new, e.g. `console/src/deny_S32_fix5.test.tsx` or in `console/kit/`) that
     fails on the commit before this task: an ask card whose bar sits at the end of a transcript
     viewport, with the form's opened position below the viewport's visible part or under the
     dock/pill; pressing Deny… -> the reason field has focus and the form's whole box (field and
     both buttons) lies inside the transcript's visible part, above the dock and the pill. Commit
     the test red first (its own commit), then the fix.
AC2. The form already fully in view: pressing Deny… scrolls nothing (scrollTop unchanged).
AC3. The scroll is the least that shows the form (no jump to the card's top when the form alone
     fits), and the transcript's follow rules of 18.6 "Place" hold: focus is in a card with an open
     ask, so the transcript does not then follow new output.
AC4. Every existing test passes, slow ones too (`npm run test:slow`), the gate passes, and `ui/`
     is rebuilt. A slow (Chromium) check that the form is in view after Deny… in the page is
     welcome if cheap, not required.

## Spec

TECHNICAL 18.6 "Asks" (the new sentence on opening Deny…, written by the foreman), 18.6 "Place"
(follow), 18.4 (layout: the dock and the pills over the transcript).

## Interface

None with a sibling.

## Files

`console/kit/parts/askbar.tsx`, `console/kit/thread.tsx` if the scroll belongs to the thread,
`console/src/App.tsx` / `console/src/Dock.tsx` only if the dock or pill height must be known, a new
test file, `ui/` (the console build).

## Changed tests

Searched `console/src/*.test.tsx`, `console/kit/*.test.tsx`, `console/test/*.slow.mjs` for Deny…:
`console/kit/frame_S9.test.tsx` (Deny… focus, Esc and Cancel return focus to Deny…),
`console/kit/thread.test.tsx` (deny with and without a reason),
`console/test/adv_S23_refused_deny_enter.slow.mjs`, `regress_S23_fix8_answer_paths.slow.mjs`: they
keep asserting what they assert today. `console/src/App.test.tsx:86` asserts a focus with
`preventScroll: true` for Needs you; it stays. None asserts that opening the form does not scroll.

## Out of scope

The Needs you pill's own placement and its hiding rule; the other felt-wrong items of the S32 report.
