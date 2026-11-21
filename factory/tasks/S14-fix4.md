Folder: kernel
Effort: low
Depends on: S14-fix3
Slice: S14 — this fix closes the user-sim's rejection of outcome 2 (a joined part group whose text is not an entry has no index).

## Goal

A part group joins into a whole that a reader gets exactly as `read` would give a stored line at
the last part's place: `Joiner::push`, when a group completes and the last part has a numeric `i`
(k), returns the result of the same rule `render` applies (the joined text, readable with `"i": k`,
or `{"v":…,"i":k,"kind":"unreadable","at"?…,"raw":"<the joined text>"}`), parsed as a `Value`.
So a joined whole that is not a JSON object, not JSON, has an unknown `kind` or a newer `v` is an
`unreadable` entry at index k, never an entry without an index. Reuse `render` (or one helper it
and the joiner share); do not write the rule twice.

## Acceptance

AC1. The red test `regress_s14_joined_non_object_whole_has_no_index` in
     `kernel/tests/it/regress_s14.rs` passes, unchanged.
AC2. The four `adv_s14_*` tests and the `ac5_part_*` tests pass unchanged; a normal 3-part entry
     joins byte-equal (as JSON) to the entry with the last part's `i`.
AC3. A joined text that is not JSON (e.g. parts `"{"` and `"x"`) gives an `unreadable` entry with
     `i` = the last part's and `raw` = the joined text (today: `None`; say in a test).
AC4. A last part with no numeric `i` yields what it yields today.

## Spec

`design/TECHNICAL.md` 9.4 (parts: "the whole entry carries the index of its last part"; the
`unreadable` paragraph).

## Interface

None. `Joiner::{new, push}` signatures do not change.

## Files

`kernel/src/transcript.rs`, `kernel/tests/it/transcript.rs`.

## Changed tests

- `ac10_joiner_drops_broken_groups`, last line: the toy whole `{"a":1}` (no `v`, no `kind`) from
  parts with `i` 1 and 2 is now `{"v":1,"i":2,"kind":"unreadable","raw":"{\"a\":1}"}`.
- `ac10_joiner_leaves_non_object_and_unindexed_wholes`: its first case (`[1,2]` from parts with `i`
  1, 2) is now `{"v":1,"i":2,"kind":"unreadable","raw":"[1,2]"}`; rename the test to fit. Its
  last-part-without-numeric-`i` cases stay as they are.

## Out of scope

Everything else.
