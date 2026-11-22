Folder: kernel
Effort: medium
Depends on: none
Slice: S35 — stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude`

## Goal

The user-sim's two red tests (605b7a9, 35786a2, `kernel/tests/it/regress_s35.rs`) pass. A user
message stored by `say` (or a turn that starts from the queue) goes out live as its `entry` item,
so Alice sees her second message while its turn runs (9.4 as edited). An input that starts with `/`
after white space is refused `{field: "input", reason: "slash-command"}` at `start`, `say` and an
`interrupt` replacement, before any write (9.3 as edited), so no turn waits forever on a write the
host refused. And an idle `turn` entry that ends a running turn carries `seconds` (the whole
seconds since `since`, rounded), so the console stops showing "Completed in 0 s" for a 3 s turn.
The WASM wrapper's `check` no longer says "stage A".

## Acceptance

AC1. `regress_s35_say_user_message_goes_out_live` and `regress_s35_say_slash_command_is_refused` pass
     unchanged.
AC2. The live `i` set equals `read`'s over a session of two turns (start, the fake default turn,
     `say`, the fake default turn again), each index once; the second user message is an `entry` item
     before the second `turn` entry. A queued input that starts its turn when the first ends goes out
     the same way.
AC3. Slash rule: `"/x"`, `" /x"`, `"\u{feff}/x"`, `"\u{200b}/x"`, `"\t/x"` and the ACP array
     `[{"type":"text","text":" /x"}]` are refused at `start`, `say` and `interrupt(s, Some(..))`
     with exactly one refusal `{field: "input", reason: "slash-command"}`, the state byte-equal before
     and after. `"a /x"` and `"x/"` are taken. `[{"type":"image",...},{"type":"text","text":"/x"}]`
     is refused (its first text block starts with `/`, 4.5). White space: every char
     JavaScript's `trim` removes plus Unicode categories Z, Cc, Cf (host 19.3, step 5's table); test
     at least U+00A0, U+2028, U+FEFF, U+200B, U+0009.
AC4. An idle `turn` entry (and the `turn` item) that ends a `running` turn has `seconds` =
     round((now - since) / 1000); a 3 400 ms turn gives 3. Existing tests that compare an idle turn
     exactly get the field.
AC5. `kernel/wasm/src/component.rs` `check` answers a reason that does not name a stage ("check by
     profile name is not available yet"). `cargo test` (workspace), clippy, fmt, the gate pass.

## Spec

`design/TECHNICAL.md` 9.3 (the slash rule as edited; turn entries), 9.4 (as edited: a user message
live), 4.5 step 5 (the slash rule), 18.3.

## Interface

None changes (refusals and entries are 2.3's and 9.4's shapes).

## Files

kernel/src/**, kernel/tests/**, kernel/wasm/src/component.rs (the `check` text only).

## Changed tests

`s35_stage_b::ac3_entries_stored_outside_an_items_call_go_out_live`: its "the user message goes out
as no entry item" now expects the user message's `entry` item. Tests that compare an idle `turn`
exactly: `seconds` (search kernel/tests for `"idle"`).

## Out of scope

A request error the host returns for any other reason (S36: a dropped `claude-session` error ends
the turn). `compact` (no 2.3 call).
