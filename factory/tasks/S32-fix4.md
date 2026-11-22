Folder: console
Effort: medium
Depends on: none
Slice: S32 — stage A, end to end: Alice watches a real kernel stream in the console.

## Goal

When Alice sends a message and the kernel refuses it (on watane's real host a probe session answers
`{"err":{"backend":"a probe session takes no input"}}`), her text comes back into the composer and
the page says why in one line, instead of the text vanishing with an uncaught page error. The same
holds for every `say` error of 2.3 (`queue-full`, `quota`, `paused`, `busy`, ...).

## Acceptance

AC1. The red test `console/src/regress_S32_refused_say_loses_text.test.tsx` (commit ec0bc7a) passes,
     unchanged.
AC2. A refused `say` puts its text back into the composer, unless the participant typed since the
     send (then the composer keeps what was typed); the error's message shows in one line above
     the composer with `role="alert"` until the next edit or send.
AC3. No unhandled rejection from `say` (`console/src/App.tsx` `say: ... void kernel.say(...)`).
AC4. A `say` that succeeds clears the composer as today; every existing test passes, slow ones too
     (`npm run test:slow`), and `ui/` is rebuilt.

## Spec

TECHNICAL 18.6 "Send" (new, written by the foreman), 18.4 (look of the alert line: the existing
tokens), 2.3 (the errors).

## Interface

None with a sibling.

## Files

`console/src/` (App.tsx, Dock.tsx, a new test if needed), `console/kit/` if the alert needs a part,
`ui/` (the console build).

## Changed tests

Searched `console/src/*.test.tsx` and `console/test/`: tests that assert the composer clears on send
keep passing (success path). A test that asserts a refused send clears the composer: none found; if
the implementer finds one, it asserts the AC2 behaviour after.

## Out of scope

Disabling the composer on a probe session (felt-wrong list; stage B removes the probe). The
"opened elsewhere" text on `replaced` (S36 carry list).
