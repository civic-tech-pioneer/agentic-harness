Folder: console
Effort: high
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The adversary breached "An ask shows the tool call that will run" (TECHNICAL 14; AGT-F-42 "its
full input"): since S9-1 an edit ask that carries a diff hides its input, keeps the streamed call
when the ask's own `toolCall` differs, and never shows a second diff's path. After this task the
ask card shows the frame's diff AND everything that will run. The decision this task takes at a
security boundary (foreman, recorded for the PO): the frame's "diff, not the raw input" yields to
14 — the card shows the diff first, then the ask's full input as a quiet key/value block
("Input"), always open; the diff and header come from the ask's own `toolCall`.

## Acceptance

AC1. The three red tests in `console/kit/adv_S9.test.tsx` (commit 3cf600b) pass, unchanged.
AC2. Every open ask's card shows its full `ask.input` (every key and value, as the pre-S9
     `InputList` did) below the diff, under a visible label "Input", styled quietly (muted label,
     mono values, the frame's code background); a call without a diff shows the same block.
AC3. When an `ask` entry arrives for a call already in a row, the row's call takes the ask's own
     `toolCall` (title, kind, locations, content) where the ask carries it, so the header path,
     the status-line subject and the diff are those of the call that will run (reducer,
     `console/kit/reduce.ts`). A reducer test covers it.
AC4. Each diff block shows its own path as visible text above it when the call has more than one
     diff, or when the diff's path differs from the header's path.
AC5. `kit/thread.test.tsx` `ac5_open_ask_shows_input_diff_and_bar_and_answers`: changed back to
     assert the input (`file_path`, `note`) IS shown, with the diff. `kit/frame_S9.test.tsx`
     `ac2_diff_ask_hides_input_and_command_ask_keeps_it` changes to: the diff shows AND the input
     shows. Nothing else changes; `npm test`, typecheck pass; `ui/` rebuilt.

## Spec

TECHNICAL 14 (row "An ask shows the tool call that will run"), 9.6, 9.8, 18.3, 18.6; FUNCTIONAL
AGT-F-42. Do not edit `design/`.

## Interface

none (internal to the kit).

## Files

`console/kit/parts/rows.tsx`, `console/kit/parts/diff.tsx`, `console/kit/reduce.ts`,
`console/kit/styles/kit.css`, `console/kit/reduce.test.ts`, `console/kit/thread.test.tsx`,
`console/kit/frame_S9.test.tsx`, `ui/`.

## Changed tests

The two named in AC5. `kit/adv_S9.test.tsx` must not be edited.

## Out of scope

A size limit on diffs (S7, performance); bidirectional control characters (pre-existing, later);
the sidebar data (S9-fix2).
