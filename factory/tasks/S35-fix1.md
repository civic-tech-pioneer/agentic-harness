Folder: kernel
Effort: medium
Depends on: none
Slice: S35 — stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude`

## Goal

On watane's real host (foreman's run, S99 9ba0c13 plus local patches), an items call stores
entries it never pushes live: the `unknown` entry of an unknown stream-json line (case 5) and the
`turn` entry that ends the turn. The live owner stream showed indexes 1, 2, 4 (a gap at 3) and no
5, while `read` and a reattach show 1 to 5. So the console's transcript differs between live and
after a reload ("An event this console does not know" appears only after the reload). After this
task every entry the kernel stores goes out live once, with its index (9.4).

## Acceptance

AC1 (red first). A test drives `start`, `attach`, and then `provider_lines` with the lines of
     `kernel/tests/fixtures/adapters/fake_default.jsonl` plus one unknown line
     `{"type":"made_up_by_case_5","x":1}` before the delta, as one batch and again one line per
     call. The set of `i` values on the live owner stream (from `_meta.watane.i` of an update, or
     the `entry` of an `entry` item) equals the set `read` returns, 1..=5, each once. It fails before
     the fix.
AC2. An entry with no ACP form (`unknown`, `turn`, `ask`, `allow`, `verdict`, `compaction`, `part`)
     goes out as an `entry` item; a `message` or `tool` as its finishing update (as today). The
     `turn` `_meta` item still goes out with each turn change (9.4), after the turn entry.
AC3. The same holds for entries stored outside an items call (`start`, `say`, `cancel`, `interrupt`,
     `provider_ended_with`, a timer firing) while a live owner stream exists: a test per call kind
     that stores an entry, or one table test.
AC4. Nothing else changes: `cargo test -p agent-kernel` (all), clippy, fmt; a deferred items call
     still pushes nothing.

## Spec

`design/TECHNICAL.md` 9.4 (the wire: entry items live and in replay; the finishing update), 9.3
(turn entries), 9.8.

## Interface

None changes (the wire of 9.4 as written).

## Files

kernel/src/**, kernel/tests/** . Not kernel/wasm, console/, design/.

## Changed tests

Tests that assert the exact live pushes of a call that stores a `turn` or `unknown` entry (search
kernel/tests for `pushes` and `HostCall::Push`): they now also see that entry's `entry` item.

## Out of scope

The console's reading of the extra live `turn` entries (the foreman checks it on the real host).
