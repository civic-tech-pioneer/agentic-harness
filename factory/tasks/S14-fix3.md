Folder: kernel
Effort: low
Depends on: S14-fix2
Slice: S14 — this fix closes the adversary's second-pass breach of "A session is private" (a partial fix of S14-fix2's).

## Goal

`Joiner::push` gives every joined whole that is a JSON object the `i` of the group's last part
(9.4: "the whole entry carries the index of its last part"): it inserts `i` when the whole has
none, and overwrites it when the whole has one. So no joined entry comes out without an index.

## Acceptance

AC1. The red test `adv_s14_part_group_whole_without_i_has_no_index` in `kernel/tests/it/adv_s14.rs`
     passes, unchanged.
AC2. `adv_s14_part_group_whole_takes_another_index` and the `ac5_part_*` tests pass unchanged.
AC3. A non-object whole, or a last part with no numeric `i`, yields what it yields today.

## Spec

`design/TECHNICAL.md` 9.4 (parts; "a corrupt line cannot take another entry's index").

## Interface

None.

## Files

`kernel/src/transcript.rs`, `kernel/tests/it/transcript.rs`.

## Changed tests

`ac10_joiner_drops_broken_groups` (`tests/it/transcript.rs`), its last line: the joined `{"a":1}`
from parts with `i` 1 and 2 now comes back as `{"a":1,"i":2}`.

## Out of scope

Everything else.
