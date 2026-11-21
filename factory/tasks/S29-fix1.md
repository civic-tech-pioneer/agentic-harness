Folder: kernel
Effort: low
Depends on: none
Slice: S29 — ending a turn never fails at the ceiling

## Goal

Past quota, an ask entry from the adapter that already carries a `decision`
(no adapter makes one today, 9.8) is stored short with a decision the kernel
builds from a fixed shape, not the item's `decision` copied word for word.
Today (S29-1) the short entry copies the item's `decision`, so a 200 KB
`decision` on each of 3 open asks wrote 600 KB past the ceiling at a call
that cannot fail (adversary RISK on slice/S29, `out_entry`, the
`decided && past.short(..)` branch). After this task, every short decision
is bounded by what the record holds plus a fixed size, as 9.3 says.

## Acceptance

AC1. Past quota (cap at the account's current bytes, reserve 0, as
`s29_short.rs` does), an open ask gets a decided ask item whose `decision`
is `{"outcome":"deny","by":"host","reason":"<200 000 bytes>","extra":{...}}`:
the call returns `Ok`, the ask closes, and the stored short entry's
`decision` has only the keys `outcome`, `by`, and `reason` if present. Its
`reason` is at most 256 bytes (cut at a UTF-8 boundary), and the entry line
is under 1 KiB.

AC2. `outcome` is kept only when it is `allow`, `deny` or `none`, and `by`
only when it is one of the `by` values of 9.4 (`owner`, `session-allow`,
`checked`, `kernel`, `host`). Any other value, or a non-string, makes the
item an unreadable decision: it is not stored, and it ends an active turn
`stopped {limit: quota}` as an item not stored does (`stop_at_quota`).

AC3. Below quota nothing changes: the existing full-form path, and the tests
`quota.rs` `ac8_decisions_resolutions_and_turns_are_stored_while_past` and
`s29_short.rs` `ac7_...`, pass unchanged.

AC4. `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` and
`factory/gate.sh` pass.

## Spec

- 9.4 "The short decision" (S29): "`decision` is as above, a pending answer's
  `input` included. The kernel writes it from the record alone, without
  reading the ask entry. ... Past quota, an ask entry that already carries a
  decision (no adapter makes one today, 9.8) is stored short when its `id` is
  open or pending, and is not stored otherwise." Above: `"decision":{"outcome":
  "allow"|"deny"|"none","by":"<who>","for":"session"?,"reason":"<text>"?}`.
- 9.3 Quota *Past.* (the ceiling rule): "No call takes the account past 256
  MiB: one that would fails `quota` and, as every failed call, commits
  nothing (7). The exceptions cannot fail at the ceiling ... and a provider
  item while past ... what they write is bounded in bytes: at most 64
  decisions and one `turn` entry beyond the record and the keys they delete,
  and every decision written while past is short (9.4), so once past a call
  writes at most one full decision ... and the rest short (together within
  the record's 512 KiB plus 16 KiB, as they repeat only fields the record
  holds)."
- The foreman adds this sentence to 9.4 in the same slice (do not edit the
  spec yourself): "The decision of an adapter's decided entry stored short
  keeps only `outcome`, `by` and `reason` (cut at 256 bytes); any other
  value makes the entry unstored."

## Interface

None.

## Files

- `kernel/src/sessions.rs` (`out_entry` and a helper)
- `kernel/tests/it/s29_short.rs` (new tests)

## Changed tests

None: searched `kernel/tests/it/` for decided ask items past quota. The
existing ones (`quota.rs` ac8, `s29_short.rs` ac7) use small decisions with
`outcome`/`by` only.

## Out of scope

The shared reserve across owners, and a full decision's size at the crossing:
both are RISKs for the product owner.
