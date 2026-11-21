Folder: .
Effort: medium
Depends on: S17-1
Slice: S17 — the kernel finds its running sessions after a restart (user-sim REJECTED run 1).

## Goal

`design/TECHNICAL.md`'s restart rules (9.3 R3, R4; cases 67–71) can be built without guessing: no
row for a state nothing produces, one outcome per OpenAI loop state, a true bound per step, and
the small gaps the user-sim named closed. Documents only, same style.

## Acceptance

AC1. R4: the "Claude, `paused`" row goes. One row covers `paused` of either backend: unchanged,
     its `resume` deadline and timer kept (7); it says only the OpenAI loop writes `paused` today
     (9.5; 9.8 gives nothing for a Claude rate limit). Case 69 loses its paused part; case 70
     (stage D) carries the paused check.
AC2. R4's OpenAI rows, one outcome per loop state (and 9.5 "After a restart" says the same):
     - `requesting`, or `ready` (waiting in the FIFO or not): goes to `ready`, takes a request
       slot or waits in the FIFO under this boot.
     - `retrying`: stays `retrying`; its `retry` deadline and timer are kept (7) and the request
       goes when it fires (9.5); nothing is sent at once, so no request is sent twice.
     - `tools`: stays `tools`; its open jobs of `idempotent` tools start again under their keys
       (taking job slots or waiting, as 9.5 "Slots"), the others get `aborted`; `pending` calls
       stay pending and start in waves as before.
     - `waiting`, `paused`: unchanged.
AC3. R3: a step reads and writes no more than an `interrupt` of that session or one step of 9.5
     (17's one-call bound), whichever applies, plus its page. Case 68 says the same.
AC4. 9.4's list of `kernel` decision reasons adds `restart` (R4).
AC5. R4's Claude row says what happens to a tool call that was streamed but has no `tool` entry:
     none is written (the call ran inside `claude`, whose process is gone; the kernel stores no
     open calls); the `unknown` turn entry after it ends the turn. And a sentence says AGT-F-27
     applies to the tool jobs of 6 (OpenAI loop, R4 `tools`), which are the only calls a mod marks
     safe to repeat.
AC6. One sentence in R4 (or after it) ties it to AGT-F-10: no stored turn is shown as `running`
     unless something drives it (a Claude turn becomes `unknown`; an OpenAI turn is `running`
     because its step starts it again; `list` reports the turn the step will give, R6).
AC7. Cases 67 and 68: the 1 024-session parts use state seeded into the crate test's host state
     (stage B's calls cannot list that many: provider slots, 9.5); say so.
AC8. 9.3 states the session id once: kernel-made, at most 64 bytes; the `active/<n>` row cites it.
     A3 says "at the default 64 sessions per owner".
AC9. R6 / case 71: `answer` to an ask the lazy step just denied returns what `answer` to any
     decided ask returns today (find it in 2.3/9.3/9.6 and cite it; if the spec says nothing,
     write `refused` with one refusal `{field: "ask"}`, as for a check, and note it).
AC10. `.claude/factory/check-docs.sh` exits 0, `factory/gate.sh` passes; only
      `design/TECHNICAL.md` changes.

## Spec

`design/TECHNICAL.md` 2.3, 6, 7, 9.3 (R3, R4, R6, A3), 9.4 (decision `by`), 9.5, 16 (67–71), 17.
`FUNCTIONAL.md` AGT-F-10, AGT-F-27.

## Interface

None beyond the rules above.

## Files

`design/TECHNICAL.md` only.

## Changed tests

none (as S17-1).

## Out of scope

FUNCTIONAL wording (the PO's), code.
