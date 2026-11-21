Folder: console
Effort: medium
Depends on: none
Slice: S7 — Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and presses Stop without lag (P14).

## Goal

An open ask stays in view while the agent keeps writing. The adversary showed (red test
`console/test/adv_S7_needs_you_stream_scrolls_ask_away.slow.mjs`, commit 3bdcbf5) that after
Needs you the card can sit within 40 px of the end, so the transcript keeps following the end and
the stream carries the card, then the focused Allow once, out of view: Enter then allows a call
the participant no longer sees. Foreman decision (18.6 Place, narrowed): the transcript does not
follow the end while focus is inside a card with an open ask (its ask bar, deny field, or the
card's scroll box), and Needs you ends the follow; new output then shows the "New activity below"
pill of 18.6 (missing from the code today), which returns to the end and resumes the follow.
Also: a code block never shows tokens of older code (adversary RISK on `kit/parts/code.tsx`).

## Acceptance

AC1. The red test `adv_S7_needs_you_stream_scrolls_ask_away` passes, its assertions unchanged.
AC2. Follow pauses: in a jsdom view test, with the reader at the end and the stream running, focus
     moved into an open ask's card (by Needs you, by Tab, or by a click on Deny…) stops the follow;
     further deltas do not change `scrollTop`. Focus leaving the card (the ask decided, or focus
     moved elsewhere) does not jump the view either; the follow resumes only when the reader
     returns within 40 px of the end (scroll) or uses the pill.
AC3. The pill: while the reader is not following and output is added below, a button "New
     activity below" shows above the dock, centered in the transcript column (frame look: a chip,
     32 px, 18.4); a click scrolls to the end and resumes the follow, and the pill goes. It is not
     shown while following. It is named and reachable by keyboard, and it never takes focus by
     itself (focus stays on Allow once).
AC4. Code block: when a block's `code` or `lang` changes, the tokens of the old code are dropped at
     once (plain text of the new code shows until its own highlight arrives); a block over 64 KiB
     after a change shows plain. A view test: a complete highlighted block whose code then grows
     past 64 KiB shows the whole new text, none of the old tokens.
AC5. Held cases pass with assertions unchanged: `console/kit/adv_S9.test.tsx`,
     `adv_S9_diff.test.tsx`, `adv_S15.test.tsx`, `adv_S4.test.tsx`, `regress_S15_*`,
     `s7_4.test.tsx`, `window_S7.test.tsx`, `src/long_S7.test.tsx`, and the slow
     `test/adv_S9_ask_scroll_hides_input.slow.mjs`, `test/regress_S9_needs_you_focus_visible.slow.mjs`,
     `test/regress_S4_open_shows_end.slow.mjs`, `test/long_session.slow.mjs`. `npm test`,
     `npm run test:slow`, `npm run typecheck`, `factory/gate.sh` pass; `ui/` rebuilt and committed.

## Spec

TECHNICAL 18.6 (Place, Asks), 18.4 (chip 32 px), 14 "An ask shows the tool call that will run".
The narrowing of 18.6 Place is the foreman's decision; the spec text is for the PO (this slice
touches only `console/` and `ui/`).

## Interface

None between tasks.

## Files

`console/src/App.tsx`, `console/kit/thread.tsx`, `console/src/Dock.tsx` (the pill's place, if it
sits in the dock), `console/kit/parts/code.tsx`, `console/kit/styles/kit.css`,
`console/src/theme.css`, new tests in `console/kit/` and `console/src/`, `ui/` (rebuilt).

## Changed tests

Searched for follow-the-end tests: `src/App.test.tsx` ("follow at end" tests) and
`kit/window_S7.test.tsx` `ac3_following_the_end...` keep their assertions (no focus in an ask card
there). None changes.

## Out of scope

Shiki ReDoS hardening (a RISK for the PO); any kernel change.
