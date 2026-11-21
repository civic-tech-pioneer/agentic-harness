Folder: console
Effort: low
Depends on: none
Slice: S7 — Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and presses Stop without lag (P14).

## Goal

The page itself never scrolls: only the transcript does. The user-sim found (red test
`console/test/regress_S7_page_never_scrolls.slow.mjs`, commit e604248) that the screen-reader-only
labels `.wa-sr` (`position: absolute`, `console/src/theme.css:67`; used by the diff's "Added:" /
"Removed:" labels, `console/kit/parts/diff.tsx:53`) have no positioned ancestor inside the
transcript's scroll area, so on a long session each makes the document taller (472 352 px for a
900 px window): Needs you scrolls the page 61 px and hides the header; PageDown on empty space
scrolls to a blank screen.

## Acceptance

AC1. The red test passes, its assertions unchanged.
AC2. `.wa-sr` is the standard visually-hidden pattern that cannot affect layout (clip, 1 px, no
     overflow) inside a positioned ancestor: `.wa-scroll` (and `.wa-row` if needed) gets
     `position: relative` or the label's own container does, whichever keeps every other slow
     layout check green. The document never grows past the viewport on `?long=5000` and on the
     story page (check both in the red test's way, by hand, and say the heights in your report).
AC3. Held cases pass with assertions unchanged: every `console/test/*.slow.mjs`
     (`npm run test:slow`: layout, regress_S4, regress_S9_needs_you_focus_visible,
     regress_S15_layout, adv_S9_ask_scroll_hides_input, long_session, adv_S7_needs_you_stream_scrolls_ask_away,
     adv_S7_arrow_key_resumes_follow_with_ask_focused), and `npm test`. `ui/` rebuilt.

## Spec

TECHNICAL 18.4 (layout), 18.6 (Place), 18.6 Accessibility (labels stay read out).

## Interface

None.

## Files

`console/src/theme.css`, `console/kit/styles/kit.css`, `ui/` (rebuilt).

## Changed tests

None.

## Out of scope

The npm test time (S7-fix5).
