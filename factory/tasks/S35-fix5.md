Folder: console
Effort: low
Depends on: none
Slice: S35 — stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude`

## Goal

Polish from the S35 user-sim (one attempt each): the session header shows a title (the kernel's
title, or, when it is empty, the first user message's text cut to 60 characters with "…", else
"New session"), and the sidebar names every session the same way, by that same title, with the
meta line `Scratch folder · Claude Code (unchecked)` for a `claude` session with profile `scratch`
(and `<backend> · <profile>` otherwise), for the current and the older ones alike.

## Acceptance

AC1. Served with two sessions, titles `""` (first user message "hi") and `""` (none read yet): the
     header's h1 of the open one reads "hi"; the sidebar rows read their titles by the same rule and
     the same meta form, whether or not the row is the current one (component test with a fake
     `Watane`, as `served.test.tsx` does).
AC2. A long first message (100 chars) gives a 60-char title plus "…".
AC3. `npm test`, `npm run typecheck`, `npm run test:slow`, `npm run build` (ui/) pass.

## Spec

`design/TECHNICAL.md` 18.3, 18.4 (header with title), frames `Main-2`.

## Interface

None.

## Files

console/src/**, console/kit/** if needed, ui/ (build only).

## Changed tests

Tests that assert the sidebar's meta text "claude · scratch" or an empty header title (search
console/src for `· scratch` and `Scratch folder`).

## Out of scope

Titles from the kernel (`rename`); the user-message look of Main-2 (a later console slice).
