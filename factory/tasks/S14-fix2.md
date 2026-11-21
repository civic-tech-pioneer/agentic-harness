Folder: kernel
Effort: low
Depends on: none
Slice: S14 — a kernel maker runs `cargo test` in `kernel/` and every item of S10's kernel list holds in code; this fix closes the adversary's breach of "A session is private".

## Goal

`Joiner::push` in `kernel/src/transcript.rs` gives the whole entry it joins from a part group the
`i` of the group's last part (9.4: "the whole entry carries the index of its last part"), never an
`i` the joined text claims. So a stored part group cannot make an entry take another entry's index.

## Acceptance

AC1. The red test `adv_s14_part_group_whole_takes_another_index` in `kernel/tests/it/adv_s14.rs`
     passes, unchanged.
AC2. A group whose joined text already carries the last part's `i` joins exactly as today (the
     existing `ac5_part_*` and joiner tests pass unchanged).
AC3. A joined whole that is not a JSON object, or a last part with no numeric `i`, yields what it
     yields today (nothing new is invented); nothing else changes.

## Spec

`design/TECHNICAL.md` 9.4 (parts: "the whole entry carries the index of its last part"; the
`unreadable` paragraph: "a corrupt line cannot take another entry's index").

## Interface

None. `Joiner::{new, push}` signatures do not change.

## Files

`kernel/src/transcript.rs`, `kernel/tests/it/transcript.rs`.

## Changed tests

None.

## Out of scope

The console's own joiner (a console slice). The adapter (S14-fix1).
