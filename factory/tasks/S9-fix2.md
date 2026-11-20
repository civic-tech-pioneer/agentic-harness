Folder: console
Effort: low
Depends on: none
Slice: S9 — Alice opens the console on the scripted session and it looks and reads like the canvas row "Revision 2", not only behaves like it.

## Goal

The adversary flagged a RISK: the shipped page's sidebar shows fake sessions that read as other
mods' sessions ("Mail · asks to send", "Desk · ChatGPT"), against TECHNICAL 18.1 (the console
lists only its own sessions; F-AGENT-6, N11). After this task `storySessions` holds only sessions
the page itself could own: places are workspaces (folder names), the subtitle is
`<place> · <provider or state>` with provider Claude Code or ChatGPT, and the frame's mix of
states stays (one Needs you, one Running, Recent with paused, unknown, idle).

## Acceptance

AC1. No `storySessions` row's place or meta names another mod or app (no "Mail", "Desk");
     places are folder names like the open session's ("notes", "site", "trip").
AC2. The mix of states is unchanged; `src/shell.test.tsx` `ac6_story_sessions_have_the_frames_mix`
     passes (adjust only its expected texts if it pins the old words).
AC3. `npm test`, typecheck pass; `ui/` rebuilt.

## Spec

TECHNICAL 18.1, 18.4. Do not edit `design/`.

## Interface

none.

## Files

`console/fake/page.ts`, `console/src/shell.test.tsx` (expected texts only), `ui/`.

## Changed tests

`src/shell.test.tsx` ac6 test, only if it pins the old texts.

## Out of scope

A real `list` (S10).
