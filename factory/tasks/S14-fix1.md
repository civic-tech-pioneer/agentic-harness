Folder: kernel
Effort: medium
Depends on: none
Slice: S14 — a kernel maker runs `cargo test` in `kernel/` and every item of S10's kernel list holds in code; this fix closes the adversary's breach of "An ask shows the tool call that will run".

## Goal

A host item id is a string (2.3 `answer(… ask: string …)`; the 9.4 ask body `"id":"<ask id>"`).
The Claude adapter takes an `ask`, `check` or `ask-resolved` item only when its `id` is a
non-empty JSON string; any other `id` (number, bool, object, array, empty string, absent, null)
makes the item `unknown` (raw = the item) and records nothing as open. So one host id can never be
open twice in two JSON forms (`"7"` and `7`, or `1` and `1.0`).

## Acceptance

AC1. The red tests `adv_s14_reused_id_as_number_opens_a_second_card` and
     `adv_s14_reused_id_in_another_number_form_is_an_ask` in `kernel/tests/it/adv_s14.rs` pass,
     unchanged.
AC2. An ask, a check and an `ask-resolved` each with `"id": 7`, and each with `"id": ""`, give one
     `unknown` entry (a fixture in `tests/fixtures/adapters/claude/strict.json` or `asks.json`).
AC3. Nothing else changes: every other existing test passes unchanged.

## Spec

`design/TECHNICAL.md` 2.3 (`answer`), 9.4 (ask body), 9.8, 4.5 step 4. S16 is writing the spec text
in parallel; build to this brief.

## Interface

None. `ClaudeAdapter`'s public API does not change.

## Files

`kernel/src/adapters/claude.rs`, `kernel/tests/fixtures/adapters/claude/*`, `kernel/tests/it/adapters.rs`.

## Changed tests

None (asks.json's ids are all strings, or null where it already expects `unknown`).

## Out of scope

The `by` values of `ask-resolved`, the shape of `input`, the range of `expires-at` (product
owner questions). The transcript (S14-fix2).
