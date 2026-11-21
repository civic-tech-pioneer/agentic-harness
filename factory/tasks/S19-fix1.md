Folder: kernel
Effort: low
Depends on: S19-2
Slice: S19 — this fix closes the adversary's RISK: a joined whole of kind `part` still comes out readable when the last part's `i` is not a whole number.

## Goal

`Joiner::push` never yields an entry of kind `part`. When a group completes and its last part
has no whole-number `i` (missing, `5.0`, `-1`, a string), and the joined text parses to an object
whose `kind` is `part`, `push` returns `None` (the group is dropped, as a broken group is), since
an `unreadable` entry needs an index. Foreman's decision: drop, not an index-less `unreadable`.
A joined whole of any other kind on that path reads as today; the whole-number-`i` path is as
S19-2 left it.

## Acceptance

AC1. Parts `i` 4 then `i` 5.0 (`m` 2) joining to `{"v":1,"i":5,"kind":"part","at":1,"body":{"n":1,"m":1,"text":"x"}}` -> `None`.
AC2. The same with the last part's `i` missing, or `"5"`: `None`.
AC3. A joined `message` whole whose last part has no `i` yields what it yields today.
AC4. Every other test passes unchanged, the S19 tests too; `factory/gate.sh` passes.

## Spec

PO decision, `factory/PLAN.md` Deputy decisions 2026-11-21 "S14's open points" ("a joined whole
of kind `part` is `unreadable`"); `design/TECHNICAL.md` 9.4. No `design/` edit.

## Interface

None.

## Files

`kernel/src/transcript.rs`, `kernel/tests/it/transcript.rs`.

## Changed tests

- `ac4_joiner_part_whole_without_last_i_is_as_before` (S19-2): today it asserts the `part` whole
  `{"v":1,"kind":"part","at":1}` comes back; after, `push` returns `None`. Rename to fit.

## Out of scope

Everything else.
