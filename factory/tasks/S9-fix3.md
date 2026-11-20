Folder: console
Effort: medium
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The adversary's rerun breached "An ask shows the tool call that will run" twice more: the ask's
own toolCall is merged over the streamed call (fields it omits keep streamed values), and while
an ask is open the stream can still put `rawOutput` (and `rawInput`) into the card. After this
task an open ask's card shows only what the ask carries: its toolCall replaces the streamed call
whole, and while the ask is open the stream changes nothing the card shows but the status.

## Acceptance

AC1. The two red tests in `console/kit/adv_S9.test.tsx` (commit bdff1df) pass, unchanged, and the
     other three still pass.
AC2. When an ask carries a `toolCall`, the row's call becomes exactly that call: a field the
     ask's toolCall omits is empty (`content: []`, `locations: []`, no `rawInput`, no
     `rawOutput`; `title` falls back to the kind word, never the streamed title). Reducer test.
AC3. While the ask is open, a stream update may change only `status`; `rawOutput`, `rawInput`,
     `title`, `kind`, `locations`, `content` stay those of the ask. The card renders no program
     output while asking. Reducer and view tests.
AC4. Nothing else changes; `npm test`, typecheck pass; `ui/` rebuilt.

## Spec

TECHNICAL 14 (row "An ask shows the tool call that will run"), 9.6, 9.8, 18.3; AGT-F-42. Do not
edit `design/`.

## Interface

none.

## Files

`console/kit/reduce.ts`, `console/kit/parts/rows.tsx`, `console/kit/reduce.test.ts`,
`console/kit/frame_S9.test.tsx`, `ui/`.

## Changed tests

none expected; `kit/adv_S9.test.tsx` must not be edited.

## Out of scope

Flagging a mismatch between the ask's toolCall and its input (a PO question, in the report);
updates after the decision; diff size limits.
