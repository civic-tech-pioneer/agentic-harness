Folder: .
Effort: medium
Depends on: S10-fix1
Slice: S10 — fix task for the adversary's second pass: B4' (an update after the ask changes what the card shows), and its cheap RISKs.

## Goal

An open ask's card shows exactly the call that will run, whatever arrives after it, and the
rules S10 added each have a case in 16. Documents only: `design/TECHNICAL.md`. No code. Quote
each check's output.

## Acceptance

AC1 (B4'). 18.3's `ask` (unanswered) row: the card is drawn from the ask entry only, never from
    any `ToolCall`, `ToolCallUpdate` or `tool` entry of the same `toolCallId`, before or after the
    ask, while it is open; its argument line comes from the ask's `input` (not `toolCall.rawInput`
    or `title`); a decision line names the call by the ask's `toolCall.title` and shows its
    `input`'s argument. 9.8's paragraph says "any `ToolCall` of the same id, earlier or later".
    9.4's console "Code to change" line adds: while the ask is open, a later call patch of that
    id does not change the call the card shows. Check: `grep -n 'before or after the ask' design/TECHNICAL.md`.
AC2. 9.4 "Code to change", one more console line: open asks are keyed by the ask entry's `id`
    (2.3), not by `toolCallId`, so a check's decision never clears an ask bar of the same call.
AC3. 2.3 opens "`answer`'s `decision` has exactly one of `allow` or `deny`, its value `true`"
    (no `none` in that sentence), and the `claude-session.answer` cross-reference reads
    "(2.1, 4.5)" (check that `claude-session` is in 2.1 first; cite where it is).
AC4. 16: case 63 says "a `can_use_tool` whose `tool_use_id` is missing, empty or not a string";
    add case 64 (4.5, 9.8, 10): a stdout line that is not UTF-8, one with a duplicate top-level
    `type`, and a `{"type":"verdict",…}` line each reach the kernel as `wrapped` and become an
    `unknown` entry; a `verdict` or `checking` item on a `direct: true` stream is `unknown`; an
    ask item with a non-number `expires-at` is `unknown`, no ask. Add case 65 (18.3): with an ask
    open on call T for input A, a later `ToolCall` T with input B leaves the card showing A, and
    Allow answers the ask's `id`.
AC5. `factory/gate.sh` passes; 9.8's 12 "From" cells unchanged.

## Spec

TECHNICAL 2.1, 2.3, 9.4, 9.8, 16, 18.3; `console/kit/reduce.ts` (`upsertCall`, `removeAsk`).

## Interface

None.

## Files

`design/TECHNICAL.md` only.

## Changed tests

None.

## Out of scope

Any code; the envelope (option 3, declined by the PO).
