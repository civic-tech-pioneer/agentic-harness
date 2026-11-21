Folder: kernel
Effort: medium
Depends on: S20-1, S20-2, S20-3
Slice: S20 — asks as the kernel records and shows them

## Goal

The kernel itself, not only the adapter's memory, keeps one ask per id and
one card per call. An ask or check item whose `id` is open or pending in the
record, or whose `toolCallId` has an open or pending ask with another `tool`
or `input`, becomes an `unknown` entry and changes nothing, even when a fresh
adapter (a second stream, a reloaded instance) hands it in. The adversary's
three red tests pass. Two small fail-closed rules ride along: an `Edit` with
`replace_all` shows no diff, and a pending answer keeps the changed `input`
an `allow` carried so the decision entry shows what runs.

## Acceptance

AC1. The red tests in `kernel/tests/it/adv_s20.rs` pass, unchanged:
`adv_s20_reused_id_of_a_pending_ask_takes_a_second_answer`,
`adv_s20_reused_id_of_a_pending_ask_gets_two_decisions`,
`adv_s20_reused_id_of_an_open_ask_shows_another_call`.

AC2. In `Kernel::out_entry` (sessions.rs), before anything is appended for
an `ask` entry without `decision`: when its `id` is in `rec.open` or
`rec.pending_answers`, or its `toolCallId` equals that of an open or pending
ask whose `tool` (the `toolCall.title`) or `input` differs, append one
`unknown` entry with `raw` the ask body and change nothing else (no push of
an ask, no `open`, no turn change, no request). To compare without reading
every ask, `OpenAsk` and `PendingAnswer` gain `call: String` (serde
`toolCallId`, default empty), and the kernel reads at most the one matching
ask body (by its `i`) to compare `tool` and `input`. The same `tool` and
`input` on the same call (checked mode: a check, then the ask) stays an ask.
This check comes before the cap of 64.

AC3. An `Edit` whose `input.replace_all` is `true` gets no `content` (the
diff would understate what runs; the card shows `rawInput`). Test in
`adapters.rs`.

AC4. An owner's `answer` `{"allow":true,"input":{…}}` keeps `input` in the
pending answer (`PendingAnswer.input: Option<Value>`, serde `input`, skipped
when absent), and the decision entry written from that pending answer
carries `"input"` in `decision`. The 1 KiB cap of a pending answer does not
count `input` (the decision's input is bounded by the 256 KiB entry rule and
parts); say so in 9.3. Test in `asks.rs`.

AC5. Tests: one each for AC2's second-adapter cases not covered by AC1: a
fresh adapter's ask with a new id on the `toolCallId` of a pending ask with
another input (`unknown`, and a later `answer` to the new id refused
`{field: "ask"}` since it never opened); the same `tool` and `input` on that
call is an ask.

AC6. Spec, `design/TECHNICAL.md`: 9.8, the "are each an `unknown` entry"
list: the id and call rules hold against the session record (`open` and
`pending-answers`), not only a stream's memory; 9.3 record: `open` items
and `pending-answers` gain `toolCallId` (and pending `input`); 9.8 AC2
paragraph of S20-1 (kind and diff): `Edit` with `replace_all: true` has no
diff; 9.3 `interrupt` bullet: with a replacement on `unknown` the
replacement starts first (queued inputs wait), resuming as R5 does.

## Spec

`design/TECHNICAL.md` 9.3, 9.4, 9.8, 2.3 (`input` on allow), 14 (an ask
shows the call that will run).

## Interface

Uses S20-2's `PendingAnswer`, `OpenAsk`, `close_all`; no new public calls.

## Files

`kernel/src/sessions.rs`, `kernel/src/record.rs`,
`kernel/src/adapters/claude.rs` (AC3 only), `kernel/tests/it/asks.rs`,
`kernel/tests/it/adapters.rs`, `design/TECHNICAL.md` (AC6 lines).
Not `kernel/tests/it/adv_s20.rs`.

## Changed tests

None expected: `call` and `input` default when absent. Report any that change.

## Out of scope

Session-allow matching and its interaction with revoking (PO question);
the host's check of `input` on allow.
