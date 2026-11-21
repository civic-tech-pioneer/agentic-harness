Folder: console
Effort: medium
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The adversary breached "An ask shows the tool call that will run" through S9-fix4's diff
normalisation: a change that only adds or removes a final newline shows as no change at all, yet
it can make a commented command run or stop a line from running. After this task a final-newline
change always shows as a change, and a shared last line without a final newline (the user-sim's
case) still shows as unchanged.

## Acceptance

AC1. `console/kit/adv_S9_diff.test.tsx` (commit f8ca275) passes unchanged, and
     `console/kit/regress_S9_diff_last_line.test.tsx` still passes unchanged.
AC2. Rule: append a final newline to both sides only when BOTH non-empty sides lack it (or both
     have it, which changes nothing). When exactly one side ends without a newline, diff the
     texts as they are, so the last line shows as removed and added, and add a visible muted line
     "No newline at end of file" under the side that lacks it (with that side's mark and class,
     and a screen-reader prefix like the others), as git does.
AC3. Old null or "" keeps showing every new line as added; new "" every old line as removed.
AC4. Nothing else changes; `npm test`, typecheck pass; `ui/` rebuilt.

## Spec

TECHNICAL 14 (row "An ask shows the tool call that will run"), 18.2 (diff view), 18.3.

## Interface

none.

## Files

`console/kit/parts/diff.tsx`, `console/kit/styles/kit.css`, `console/kit/frame_S9.test.tsx`, `ui/`.

## Changed tests

none; both red files must not be edited.

## Out of scope

Diff size limits; bidirectional control characters.
