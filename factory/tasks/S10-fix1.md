Folder: .
Effort: medium
Depends on: S10-1, S10-2
Slice: S10 — fix task for the adversary's BREACHED (B1–B4) and its cheap RISKs, at TECHNICAL 14 "An ask shows the tool call that will run".

## Goal

The adversary read slice S10's text as the vendor stdout and as a client, and found four ways the
new rules still let a forged or misleading ask through. Close each in `design/TECHNICAL.md`, as
fixed below. Each acceptance is a check that fails on today's text and passes after. Nothing
else changes. Documents only; no code. Write in the file's style; quote each check's output.

## Acceptance

AC1 (B1, a stdout line poses as a provider item). 4.5's wrap row names the list in full: a line
    whose top-level `type` is any watane item type that can share the stream: `ask`, `check`,
    `ask-resolved`, `initialized`, `wrapped`, `consent-pending`, and the checked provider's
    `verdict` and `checking` (10); "a slice that adds an item type to any of these streams adds it
    here". Section 10 (checked provider) adds: the kernel takes `verdict` and `checking` only from
    a checked provider's stream (`direct: false`); 9.8 adds: on a `direct: true` stream a
    `verdict` or `checking` item is an `unknown` entry. Check: `grep -n 'consent-pending' design/TECHNICAL.md`
    finds 4.5; `grep -n 'direct: true' design/TECHNICAL.md` finds 9.8.
AC2 (B2, host and kernel parse differently). 4.5: the host decides on the parsed value: a line that
    is not valid UTF-8, not exactly one JSON object, or that has a duplicate key at the top level
    is pushed as `wrapped` too (the duplicate-key RISK). 9.8: the kernel reads items strictly (no
    lossy UTF-8 decoding); an item it cannot parse is `unknown`, never an ask. Add both to 9.8's
    "Code to change (S12)" list as one line: "read lines strictly: no `from_utf8_lossy`; an
    unparsable line is `unknown`". Check: `grep -n 'from_utf8_lossy\|lossy' design/TECHNICAL.md`.
AC3 (B3, `none` from a client). 2.3's decision table loses the `{"none":true}` row: `answer`
    has four shapes; `{"none":true}` is refused `{field: "decision"}` (a check's `none` exists only
    in the checked provider's `claude-session.answer`, 2.2, 4.5). Case 60 reads: the four shapes
    are accepted, and each of `{}`, `{"none":true}`, `{"allow":true,"deny":true}`,
    `{"deny":true,"for":"session"}`, `{"allow":"yes"}` is refused `{field: "decision"}`; the
    owner's answer to a check is refused `{field: "ask"}`. Check: `grep -n '"none":true' design/TECHNICAL.md`
    shows no 2.3 table row, only the refusal and case 60.
AC4 (B4, which input the card shows). 18.3: an ask's card is drawn from the ask entry's `toolCall`
    and `input` only, never from an earlier `ToolCall` update or `tool` entry with the same
    `toolCallId`. 9.8 (under the ask row's paragraph): when an ask's input differs from the input
    of the pending `ToolCall` of the same id, the ask's is the one that runs and the one shown.
    Add a console line to 9.4's "Code to change" if `console/kit/reduce.ts` merges the ask's call
    into an earlier tool row by id (read it; name the change for "a console slice after S9").
AC5 (RISKs, narrowing only). 4.5: a `control_request` whose `tool_use_id` is missing, not a
    string, or empty is denied at once and pushes nothing (today: "without"). 9.8: an ask or check
    item whose `tool_use_id` is not a non-empty string, or whose `expires-at` is not a number, is
    `unknown`. 9.3 (turn JSON line for `waiting`): `asks` and `expires-at` count asks
    (`kind: "ask"`) only, not checks.
AC6. `factory/gate.sh` passes (the 9.8 "From" cells and 12 rows unchanged).

## Spec

TECHNICAL 2.2, 2.3, 4.5, 9.3, 9.4, 9.8, 10, 14, 16, 18.3; `kernel/src/adapters/claude.rs`
(`line`); `console/kit/reduce.ts` (how an ask's `call` patch is applied).

## Interface

None with a sibling. Item type list fixed in AC1.

## Files

`design/TECHNICAL.md` only.

## Changed tests

None (only `kernel/tests/it/adapters.rs` reads 9.8's "From" cells; they stay).

## Out of scope

Wrapping every `claude` line in an envelope (option 3 of S8's question, declined by the PO;
named again for the PO in the slice report). Any code.
