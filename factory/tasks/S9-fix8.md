Folder: console
Effort: low
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

Outcome 7 of sketch S9 (added by the PO after hand-out, *guard*): the page test
`console/src/App.test.tsx` › `the page` › `ac1_app_shows_sidebar_header_rows_dock_on_the_played_story`
races the played story ("expected [] to include 'user'", PR #9). After this task it awaits the rows
before asserting, and no other test in `src/App.test.tsx` asserts rows of the played story at once.

## Acceptance

AC1. The test waits (`findBy*`/`waitFor`) until the rows it checks are in the DOM, then asserts
     exactly what it asserted before (no assertion dropped or weakened).
AC2. The other tests in `src/App.test.tsx` and `src/shell.test.tsx` that read rows of the played
     story right after `ready()` are checked the same way and made to wait where they race.
AC3. `npx vitest run src/App.test.tsx` passes 30 times in a row (a loop), and also with
     `--sequence.concurrent` off/on under load (e.g. while `npm run build` runs); say what you ran.
AC4. `npm test` passes.

## Spec

TECHNICAL 18.7.

## Interface

none.

## Files

`console/src/App.test.tsx`, `console/src/shell.test.tsx`.

## Changed tests

The named test (waits first, same assertions).

## Out of scope

Product code.
