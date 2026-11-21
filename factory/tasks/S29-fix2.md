Folder: kernel
Effort: low
Depends on: none
Slice: S29 — ending a turn never fails at the ceiling

## Goal

A decided ask entry from the adapter that is stored closes its ask from
both `open` and `pending-answers`, so the same entry sent again finds
neither and is not stored. Today `out_entry` stores it (short past quota)
and calls `close_ask`, which removes only an `open` id. A pending answer
stays, so repeats of the item each store another short decision past the
ceiling, in calls that cannot fail (adversary breach, red test
`kernel/tests/it/adv_s29.rs` `adv_s29_pending_decided_ask_repeats_past_ceiling`,
commit 3dacf34).

## Acceptance

AC1. The red test `adv_s29_pending_decided_ask_repeats_past_ceiling` passes,
unchanged.

AC2. Below quota too: a decided ask entry for a pending id is stored once
(full form, as today), the pending answer leaves `pending-answers`, and a
second identical item stores no decision.

AC3. Every HELD attack still holds, and none of these is weakened: the
existing `s29_short.rs`, `s29_ceiling.rs`, `quota.rs` and `s28.rs` tests. In
particular, a repeated decided ask for an open id past quota writes nothing
after the first.

AC4. `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` and
`factory/gate.sh` pass.

## Spec

- 9.4 "The short decision" (S29), as the foreman wrote it in this slice:
  "Past quota, an ask entry that already carries a decision (no adapter
  makes one today, 9.8) is stored short when its `id` is open or pending,
  and is not stored otherwise. The decision of an adapter's decided entry
  stored short keeps only `outcome`, `by` and `reason` (cut at 256 bytes);
  any other value makes the entry unstored. A decided entry that is stored
  closes its ask: the id leaves `open` or `pending-answers` (as an
  `ask-resolved` does, 9.8 rule 4), so a repeat of it finds neither and is
  not stored."
- 9.3 Quota *Past.* (the ceiling rule): "No call takes the account past 256
  MiB: one that would fails `quota` and, as every failed call, commits
  nothing (7). The exceptions cannot fail at the ceiling ... and a provider
  item while past ... past the account no call starts new work, so what
  calls write after that is bounded by the asks and active turns open when
  it crossed."
- The model: `resolve` in `kernel/src/sessions.rs`, which removes a pending
  id with `rec.pending_answers.remove`.

## Interface

None.

## Files

- `kernel/src/sessions.rs` (`out_entry`)
- `kernel/tests/it/s29_short.rs` (AC2 test); `kernel/tests/it/adv_s29.rs`
  read-only

## Changed tests

None: searched `kernel/tests/it/` for decided ask items on a pending id;
none sends one.

## Out of scope

The escaped size of a 256-byte `reason` (up to about 1.5 KiB): bounded, and
a RISK for the product owner.
