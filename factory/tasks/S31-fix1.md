Folder: kernel
Effort: medium
Depends on: S31-3
Slice: S31 — the broker: session allows and rule matching

## Goal

A session allow on `Bash` holds only the exact command run the way it was
allowed. Today `broker::reach` builds `Bash(<command>)` from `command` alone,
so an allow given for `{"command":"ls"}` also answers
`{"command":"ls","dangerouslyDisableSandbox":true}`: the same command outside
the shell grant's sandbox (adversary breach, red test
`adv_s31::adv_s31_bash_allow_covers_sandbox_escape`, commit f5ad66c).

Fix (foreman's decision, fail-closed): a `Bash` ask offers a reach only when
its input object has no key other than `command`, `description`, `timeout`
and `run_in_background`; any other key (`dangerouslyDisableSandbox` among
them, whatever its value) gives no reach, so such an ask always goes to the
owner (or is denied `no-owner`) and can never be answered from an allow.

Spec: TECHNICAL 9.6 "Session allows" and its "As built (S31)" bullet; 14
"A session allow can be taken back".

## Acceptance

AC1. The red test `adv_s31::adv_s31_bash_allow_covers_sandbox_escape`
passes; nothing else changes.
AC2. Unit (`tests/it/broker.rs`): `Bash` with
`{"command":"ls","dangerouslyDisableSandbox":true}`, with
`{"command":"ls","dangerouslyDisableSandbox":false}` and with
`{"command":"ls","x":1}` -> no reach; with `{"command":"ls",
"description":"d","timeout":5,"run_in_background":true}` ->
`Bash(ls)`.
AC3. HELD attacks stay held (the existing tests in `adv_s31.rs`,
`broker.rs`, `allows.rs` pass unchanged).
AC4. `cargo test`, clippy `-D warnings`, `cargo fmt --check`,
`factory/gate.sh` pass.

## Spec

`design/TECHNICAL.md` 9.6.

## Interface

None changed.

## Files

`kernel/src/broker.rs`, `kernel/tests/it/broker.rs`.

## Changed tests

none (searched: no test gives a Bash ask a key outside the four).

## Out of scope

Pinning `allowUnsandboxedCommands: false` in the host's settings (watane
host, a cross-project item for the PO); the 9.6 text (the foreman).
