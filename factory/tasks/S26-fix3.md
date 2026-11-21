Folder: kernel
Effort: medium
Depends on: S26-4
Slice: S26 — the kernel tracks its provider streams

## Goal

The user-sim's red tests pass: when the kernel itself closes a session's
owner stream (`lagging` outside an item call, `replaced` by a new `attach`),
it resumes the session's provider stream in that same call, so a provider
stream deferred on the old owner stream is never left frozen until the
host's 5-minute backstop kills the Claude session (TECHNICAL 3
"Forwarding": the kernel resumes every deferred upstream when the
downstream ends, any outcome; 9.4 "A full owner stream"; case 51).

## Acceptance

AC1. `regress_s26::regress_s26_lagging_close_resumes_the_deferred_provider`
and `regress_s26_replaced_close_resumes_the_deferred_provider`
(kernel/tests/it/regress_s26.rs, commit a541077) pass, unchanged.

AC2. Every place the kernel closes or drops a session's live owner stream
(`drop_live` for `lagging`, the `replaced` close in `attach`, and any other)
calls `Host::resume_stream(t)` for the session's live provider stream, if
any, in that call (after the close). Resume of a stream that is not deferred
does nothing (3), so no deferral state is kept. `delete` needs none (it
cancels the session). A test asserts the `Resume` follows the `Close` in the
host log for both cases, and that with no live provider stream nothing is
resumed.

AC3. The doc comment of `provider_item` says what the code does for an id
of an older boot (it reaches only via R1's guarantee that items come on
streams of this boot), not "changes nothing" unless the code makes it so.

AC4. TECHNICAL 9.4 "A full owner stream" (and the `attach` "replaced"
sentence if needed) says in one clause that the kernel resumes the
session's provider stream when it closes the owner stream itself.

AC5. The adversary's HELD attacks hold: attach owner check before any write;
kept-key retry under busy; busy before `open_stream` and writes; `guarded`
cancel; deferral rollback and single apply; `live` and `live/` agree on every
path; per-owner counts change only for the record's owner; ready/ended id
checks; idle deadline removal; past-quota item starts no turn; delete never
steps and always cancels; refusal texts; one turn entry per turn past quota
by any call.

## Spec

- `design/TECHNICAL.md` 3 ("Forwarding", "Resume"), 9.4 `attach`, "A full
  owner stream", 16 case 51.

## Interface

None changed.

## Files

- `kernel/src/sessions.rs`
- `kernel/tests/it/attach.rs` (the AC2 test)
- `design/TECHNICAL.md` 9.4

## Changed tests

Tests in `attach.rs` that assert the exact host log after a `lagging` or
`replaced` close (now with a `Resume`): list each one you change.

## Out of scope

A provider `ended` during an active turn (PO question); the Claude adapter's
handling of short hand-written lines.
