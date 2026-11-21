Folder: .
Effort: low
Depends on: S10-fix2
Slice: S10 — fix task for the user-sim's REJECTED (3) and its notes.

## Goal

A console maker reading 9.4, 18.3 and 18.5 finds every wire form the console cannot read named as
a console change, and no section that draws a button the kernel refuses. Documents only:
`design/TECHNICAL.md`. Quote each check.

## Acceptance

AC1. 9.4 "Code to change", console: accept a decision `outcome` `none` (a check the checked
     provider let through); `Decision.outcome` in `console/kit/model.ts` gains `none`.
     Check: `grep -n 'outcome. .none' design/TECHNICAL.md` or equivalent finds the line.
AC2. 9.4 "Code to change", console: read `part` entries and join parts 1..m (across pages and
     replay) into one entry before parsing, as 9.4 says; today `wire.ts` has no `part` case.
AC3. A check gets no answer buttons at all (the owner cannot answer it, 2.3): 18.3's `ask`
     (unanswered) row says "a `check` shows as a pending check with no answer buttons" (replace
     "has no Allow button"); 18.5 "Asks" says the bar is for an ask (`kind: "ask"`) only; "Needs
     you (n)" counts open asks only, not checks (as 9.3 `waiting`); 9.8's check sentence and 9.4's
     console line say "no answer buttons" (not "no Allow button"). Check:
     `grep -n 'no Allow button' design/TECHNICAL.md` finds nothing.
AC4. 17, stage B row: "new cases" adds 60, 61, 62, 63, 64 (Claude part), 65; stage E row (the
     checked provider) adds 64 (`verdict`/`checking` part) if case 64's text is split that way,
     else stage B takes it whole. Read the rows first.
AC5. 9.4: an `unreadable` entry's form is spelled: `{"v":<its v>,"i":<i>,"kind":"unreadable","at":<at>?,"raw":<the stored JSON>}`.
     The `queue` item says `input` is the input's JSON as a string (2.3 `queued-input`).
AC6. `factory/gate.sh` passes; 9.8's "From" cells unchanged.

## Spec

TECHNICAL 2.3, 9.3, 9.4, 9.8, 17, 18.3, 18.5; `console/kit/wire.ts`, `model.ts`.

## Interface

None.

## Files

`design/TECHNICAL.md` only.

## Changed tests

None.

## Out of scope

Any code.
