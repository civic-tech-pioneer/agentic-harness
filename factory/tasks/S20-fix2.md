Folder: kernel
Effort: low
Depends on: S20-fix1
Slice: S20 — asks as the kernel records and shows them

## Goal

A `MultiEdit` ask in which any edit has `replace_all: true` shows no diff, as an
`Edit` with `replace_all` already does: a one-place diff would understate what runs.
The spec names the true bound on a pending allow's `input`.

## Acceptance

AC1. `adv_s20::adv_s20_multiedit_replace_all_shows_a_diff` (commit 3393955) passes, unchanged.
AC2. A `MultiEdit` whose edits all have `replace_all` absent or `false` keeps its diffs
(a test in `kernel/tests/it/adapters.rs`).
AC3. `design/TECHNICAL.md`:
- 9.8, the kind and diff paragraph: a `MultiEdit` with any edit `replace_all: true` has
  no `content`, as `Edit`.
- 9.3: the sentence that bounds a pending answer's `input` by the 256 KiB entry rule says
  that the record's 512 KiB bounds it. A pending allow that would push the record past
  512 KiB is refused `too-large(524288)` and writes nothing.

## Spec

9.3 (`pending-answers`), 9.8 (kind and diff).

## Interface

None.

## Files

`kernel/src/adapters/claude.rs` (`diffs_of`), `kernel/tests/it/adapters.rs`, `design/TECHNICAL.md`.

## Changed tests

None.

## Out of scope

Everything else.
