Folder: kernel
Effort: medium
Depends on: S26-1, S26-2, S26-4
Slice: S26 — the kernel tracks its provider streams

## Goal

The adversary's red test passes: past quota, a turn gets at most one `turn`
entry even when its asks close one by one (9.3 Quota, "A provider item when
past"; S25 guard). And an item delivered on an owner stream's id is never
taken as a provider item (adversary RISK, closed here because it is one
check): it cannot free a provider slot it never took nor delete the owner
stream's keys.

## Acceptance

AC1. `adv_s26::adv_s26_past_quota_ask_resolutions_add_turn_entries`
(kernel/tests/it/adv_s26.rs) passes, unchanged.

AC2. Foreman decision: past quota, a turn change that does not end an active
turn writes no `turn` entry, whatever causes it (a provider `Turn`, an ask
that closes, `close_all`); the record's turn still follows (a `waiting` turn
whose asks close drops them from `asks` and becomes `running` in the record)
and the `turn` push still goes out, so clients and the active index stay
true. Only the entry is skipped. A change that ends an active turn writes
its one entry as before. Below quota nothing changes. TECHNICAL 9.3 "A
provider item when past" says this in one clause.

AC3. `provider_item(id, ..)` acts only when `stream/<id>` has `kind:
"provider"`; any other kind (an owner stream id) changes nothing, sends
nothing, returns `Ok` with `Item::Done` and no pushes, on both the full path
and the delta path. Test: after `attach` gives `o`, `provider_item(o,
Ended)` and `provider_item(o, <a delta>)` leave the state snapshot,
`slots.provider` and `live/<session>` unchanged.

AC4. Every other test still passes, and the adversary's HELD attacks hold
(attach owner check; kept-key retry under busy; busy before `open_stream`;
`guarded` cancels; deferral rolls back whole; `live` and `live/` agree;
per-owner counts change only for the record's owner; delete never steps).

## Spec

- `design/TECHNICAL.md` 9.3 Quota ("A provider item when past"), 9.4 "A
  stream finds its session without the record", 9.1 `stream/<id>`.

## Interface

None changed.

## Files

- `kernel/src/sessions.rs`
- `kernel/tests/it/attach.rs` or `streams.rs` (the AC3 test)
- `design/TECHNICAL.md` 9.3 (one clause)

## Changed tests

None expected; `s26.rs`'s AC3 tests must keep passing. List any you change.

## Out of scope

A provider `ended` while the turn is active (the turn stays `running`; spec
9.8 rule 4 is silent: a question for the product owner); `cancel` sending
`Request::Cancel` rather than `cancel_stream` (the host ends the stream).
