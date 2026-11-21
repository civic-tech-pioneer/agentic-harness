Folder: kernel
Effort: medium
Depends on: S26-fix1
Slice: S26 — the kernel tracks its provider streams

## Goal

The adversary's second red test passes: past quota, a turn gets at most one
`turn` entry whichever call changes it, not only provider items. Today
`work.past` is set only on the provider path, so an owner's `answer` (not
refused past quota without `for: session`) writes a `turn` entry for each
ask it closes. Every call that can change a turn without ending it must know
it is past.

## Acceptance

AC1. `adv_s26::adv_s26_past_quota_owner_answers_add_turn_entries`
(kernel/tests/it/adv_s26.rs, commit 37d8d39) passes, unchanged; so does
`adv_s26_past_quota_ask_resolutions_add_turn_entries`.

AC2. Every call that reads the owner's accounts and may reach `turn_to`
(`answer`, `interrupt`, `cancel`, `unqueue`, the restart step via R6 and the
pass, the session timer firing, `attach`'s R6 step, `stream_ended`) sets the
past flag from those accounts (9.3 "Past": read before the call writes), so
`turn_to` past quota writes no `turn` entry for a change that does not end
an active turn, the record and push following as in S26-fix1. Prefer one
place (e.g. the flag set where the accounts are read, or `turn_to` taking
it from the call's accounts) over a line per call, so a later caller (ask
expiry timers, stage D) cannot forget it. A change that ends an active turn
writes its one entry as before; the restart step's `unknown` entry is still
written (it ends a driven turn).

AC3. A test: past quota, an owner `answer` closes one of two open asks:
no `turn` entry is added, the record's turn is `waiting` with one ask and the
`turn` push matches it. Below quota the same call writes its entry as
before (existing tests).

AC4. Every other test passes; S26-fix1's tests and the adversary's HELD list
hold.

## Spec

- `design/TECHNICAL.md` 9.3 Quota ("Past", "Refused `quota` when past", "A
  provider item when past"); if the clause names only provider items, widen
  it in place to "any call".

## Interface

None changed.

## Files

- `kernel/src/sessions.rs`
- `kernel/tests/it/s26.rs` (the AC3 test)
- `design/TECHNICAL.md` 9.3 (the clause)

## Changed tests

None expected; list any you change.

## Out of scope

The transcript's last `turn` entry lagging the record past quota (accepted:
the record and the push are the truth; attach sends the record's turn). The
chunk-cap room used by ask closes (pre-existing; reported to the PO).
