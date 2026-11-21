Folder: kernel
Effort: medium
Depends on: none
Slice: S28 — no session is left running with nothing driving it

## Goal

A provider stream's `ended` always takes effect, even when the kernel's
account is at its ceiling (account plus reserve, 256 MiB): the stream's key
and slots are freed, its asks are closed and an active Claude turn ends
`idle {failed {error: provider-ended}}`. Today (since S28-1) that `ended`
writes a `turn` entry, `finish` refuses it `quota`, and the whole item call
is dropped: the session stays `running` with nothing driving it and holds a
provider slot until a restart or `delete` (adversary breach, red test
`kernel/tests/it/adv_s28.rs` `adv_s28_ended_at_the_ceiling_keeps_the_dead_stream`,
commit 25045ed). The fix follows the one exception 9.3 already has, the
restart firing: the provider stream's `ended` cannot fail on quota; what it
writes is charged to both accounts and the record's `bytes`, past the
ceiling too. Its writes are bounded as R4's are (at most 64 decisions, 9.3
`open` row, and one `turn` entry). Also: `ended` changes only a `running` or
`waiting` turn, never `paused` (the 9.3 rule S28-1 wrote says "On any other
turn, `ended` changes no turn"; `Turn::is_active` includes `paused`).

## Acceptance

AC1. The red test `adv_s28_ended_at_the_ceiling_keeps_the_dead_stream`
passes, unchanged.

AC2. At the ceiling, a `waiting` session with one open ask and one pending
answer gets `ended`: the call returns `Ok`, the two decisions of case 73 (f)
and one `turn` entry `idle {failed {provider-ended}}` are written, the stream
key and slots are freed, and `quota/<owner>`, `quota` and the record's
`bytes` each grew by exactly what the call wrote (the sum of the changed key
sizes, as the restart-firing test of S25/S26 checks).

AC3. A `paused` session (set by hand, as `asks.rs`
`case73_g_the_restart_step_closes_every_ask_in_every_state` does) gets
`ended`: its turn stays `paused`, and no `turn` entry is written.

AC4. Below the ceiling nothing changes: every existing test passes
unchanged, the S28-1 tests in `s28.rs` included. A provider item other than
`ended` at the ceiling is still refused/dropped as before (the existing
quota tests show it).

AC5. A deferred `ended` (the live owner stream full) is still rolled back
whole and applied once when delivered again (the existing S26 deferral
tests and the adversary's HELD check pass).

AC6. The spec says so, in place in `design/TECHNICAL.md` 9.3 Quota "*Past.*":
the sentence "The one exception is a firing of the restart pass, which
cannot fail: what its step wrote, even a step that failed, is charged to
both accounts and the record's `bytes`." becomes "The exceptions are a
firing of the restart pass and a provider stream's `ended` (whose writes are
bounded as R4's: at most 64 decisions and one `turn` entry), which cannot
fail: what they wrote, even a restart step that failed, is charged to both
accounts and the record's `bytes`." `factory/gate.sh` passes.

## Spec

Quoted rules:
- 9.3 Quota, *Past.*: "No call takes the account past 256 MiB: one that
  would fails `quota` and, as every failed call, commits nothing (7). The
  one exception is a firing of the restart pass, which cannot fail: what its
  step wrote, even a step that failed, is charged to both accounts and the
  record's `bytes`."
- 9.3, the S28 bullet: "A provider stream's `ended` while the turn is
  `running` or `waiting` closes each open or pending ask (9.8 rule 4) and
  ends the turn `idle {failed {error: provider-ended}}` in the same call ...
  On any other turn, `ended` changes no turn."
- 9.8 rule 4: "The `ended` closes every open or pending ask or check of the
  session in the same call, so no Claude ask stays without a decision."
- The restart firing's code in `kernel/src/sessions.rs` (search "a firing
  cannot fail") is the model for charging past the ceiling.
- Where the spec is unclear, stop and say so in your last message.

## Interface

None with siblings. No public signature changes.

## Files

- `kernel/src/sessions.rs`
- `kernel/tests/it/s28.rs` (new tests); `kernel/tests/it/adv_s28.rs` read-only
- `design/TECHNICAL.md` section 9.3 only

## Changed tests

None: searched `kernel/tests/it/` for `reserve_bytes` and `Ended`; no test
asserts that `ended` fails at the ceiling or that it ends a `paused` turn.

## Out of scope

`cancel` at the ceiling (it fails `quota` there today, before S28 too: a
RISK for the product owner); `ended` on an older-boot record (R1 keeps it
from happening); the host's 5-minute backstop on a deferred `ended`.
