Folder: kernel
Effort: medium
Depends on: none
Slice: S32 — stage A, end to end (adversary pass 1: BREACHED 1).

## Goal

The end of a provider stream is never lost. `stream-sink.ended` cannot defer (it returns
nothing), yet the wrapper maps it to `provider_item(s, ProviderEvent::Ended)`, which answers
`Defer` and drops every write when the session's live owner stream is full. After this task the
kernel, outside an item call, never defers an `ended`: when the owner stream is full it drops that
live stream `lagging` (TECHNICAL 9.4, "Outside an item call ... it cannot defer ... closes that
stream with outcome `lagging`") and applies the end in full (asks closed, provider slot and
`stream/` key released, turn settled), and the page reattaches after its last index.

## Acceptance

AC1. The red test `kernel/tests/it/adv_s32.rs::adv_s32_ended_on_a_full_owner_stream_is_lost`
     passes, unchanged.
AC2. The wrapper's `stream-sink.ended` (`kernel/wasm/src/component.rs`) calls a crate entry that
     cannot answer `Defer` (e.g. `Kernel::provider_ended(stream)`, returning `Called<()>`), so no
     `ended` result is ignored.
AC3. Nothing else changes: every other test passes with its assertions unchanged. Held cases of
     the S32 adversary (its HELD list, which the tests in `kernel/tests/it/probe_s32.rs`,
     `seam_s32.rs`, `adv_s32.rs` and the console's `kit/watane.test.ts` hold), and every earlier
     adversary's kernel tests (`kernel/tests/it/adv_*.rs`, `regress_*.rs`, `usersim_*.rs`).

## Spec

TECHNICAL 3 ("Forwarding"), 9.4 (outside an item call, `lagging`), 9.8 rule 4 (an ended provider
stream closes every open or pending ask), 2.1 ("The kernel's side of the seam").

## Interface

None outside the crate and its wrapper.

## Files

`kernel/src/sessions.rs`, `kernel/src/lib.rs`, `kernel/wasm/src/component.rs`, new tests under
`kernel/tests/it/`.

## Changed tests

None (searched `kernel/tests` for `ProviderEvent::Ended`: the existing tests call it with room on
the owner stream, which this task does not change).

## Out of scope

RISK 1 of the adversary (failed calls that wrote in WASM), RISK 2 (the provider wire): for the
product owner.
