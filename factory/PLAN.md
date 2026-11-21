# Plan

Kept by the product owner (`/po`). The factory's process: `.claude/factory/README.md`.

## Order of work

The vision (`design/VISION.md`) has no numbered steps; the order is `design/TECHNICAL.md`
section 17. Since S2 the three design files are the contract; N5 stays open until stage F.

| Stage | Builds | Where | State |
|---|---|---|---|
| 0 | Design slice: drafts become the contract (S2); open probes of proposal section 18 (S1) | this repo | sketched |
| A | H1 streams, H5 durable requests, H7 timers; V8 prototype | watane host | cross-project, waits (below) |
| B | H2 + unchecked provider + kernel Claude sessions; dev's fake `claude` | watane host, dev; kernel; provider | cross-project, waits |
| C | H6 consents, app words, `claude` binary setting, plan line | watane app, pkg | cross-project, waits |
| D | H3 accounts, OpenAI loop (kernel 9.5), scripted Responses server | watane host, dev; kernel | cross-project, waits |
| E | Checked provider (needs a judge account, V7) | provider repo; host if N10 = host | waits on V7 |
| F | Delegation and counters (kernel 9.7), factory profiles | kernel | needs N5 |

**Cross-project dependency.** Stages A to D put code in watane (`../../watane`: host, dev, pkg,
app), which runs its own older factory and is not on this generic one. Each such stage becomes
one watane slice (provider, first) plus one slice here or in a provider repo (consumer). The
interface between them is `design/TECHNICAL.md` section 2.1 (`watane:host@0.1.0` additions) and
the watane contract rows of section 15. None of them is this product owner's to hand out until
watane is on the generic factory, or the human hands stage A to watane's own product owner. Stage
A's backend is pure watane; its console part (TECHNICAL 17) is this repository's.

## In flight

### S32: stage A, end to end: Alice watches a real kernel stream in the console (delivered 2026-11-21 10:47 UTC, PR #41 held: merges after watane S96, once the story passes on watane's own host; routine trig_01QrwDoJCFh8Q6DGBbTcF7qA)

Story: Alice runs `watane-dev serve` with the agent kernel installed, opens its console page, and
sees numbered items of a real kernel stream arrive in the page (18.4 layout and theme), live and
after a reload, with nothing doubled. One story in two repositories, landed together: watane's
half is S96 (host H1 streams, H5 durable requests, H7 timers, page connections; pkg stage 12), this
repository's half is this slice. Interface, named the same in both sketches: the
`watane:host@0.1.0` streams and timers of TECHNICAL 2.1 as watane S95 wrote them in host 19
(stream-sink `taken | defer`, `streams.resume`, `stream-producer.ended`, page connections).
Outcomes:
1. The kernel crate gains its first WIT world and a WASM build (`wit/world.wit`, `mod.toml`), so
   `check-mod.sh` checks it; the kernel opens a stream to its page and pushes items.
2. The console attaches to the real stream in `watane-dev serve` (the scripted fake stays for unit
   tests only); a reload re-attaches after its last index.
3. Cases 7, 20, 21 (kernel part), 36 and 51 (kernel part) pass, run against watane's host.
4. Folded in from S30 (stopped, branch `slice/S30` kept for reuse): the short decision form (9.4),
   the status line after an answer, the `long_S7` timing limits moved out of the default gate.
Every spec line this needs is written in this slice, not in a later one.
Touches: `kernel/`, `wit/`, `mod.toml`, `console/`, `ui/`, `design/TECHNICAL.md`. Protects: *core*.
Starts when watane S96 starts (it fakes nothing it can call for real once S96 merges; until then
it builds against S96's branch or the host 19 text).

Delivered (PR #41, not armed): the seam audit found 10 mismatches between the kernel's fake host
and host 19 (batches with `seq`, `SinkOutcome {Taken, Defer}`, the `outcome` variant, `bad-delay`,
`ended` through `provider_ended`, no host clock/id/generation yet, prebuilt `logic.wasm`, the world
staged in `kernel/wasm/wit/` until a watane-dev with host 19 ships), all fixed; a probe session
(`probe.start`, 9.9) is stage A's item source. User-sim ACCEPTED on a wasmtime stand-in host
(live, reload, lagging, replaced, cases 20 and 21); adversary BREACHED 1 (a lost provider `ended`
held a slot), fixed, HELD. Not yet run on watane's host: cases 7, 20, 21, 36, 51. Next: when S96
merges, a continuation routine rebases S32, runs the story on watane's real host, and the PO
compares the console and arms the merge.

## Sketched

### Stage B carry list (folded into stage B's end-to-end slices, never a slice of its own)

- S34 RISK 1: write keys for build and config files beyond the list (`Cargo.toml`, `conftest.py`,
  `pyproject.toml`, `justfile`, `Dockerfile`, ...): decide with the first slice that sets a
  `workspace` (rec: no write key outside a named source tree; else a longer list).
- S34 RISK 3: `session_firing` returns before re-arming and charging when a restart step fails, and
  a due firing with no step fails `quota` at the ceiling (`restart_firing` handles both).
- S26 accepted limits and S28's: verify with the real host's rollback (7) once wired.
- V27 (Waiting): the allowlist rule, before the shell grant (stage C).
- S32 R1 (first, before any provider item reaches the kernel): in WASM a kernel error returns
  normally and the host commits writes made before it, while the crate assumes a failed call
  commits nothing: buffer writes in the crate until success (rec), or trap on error.
- S32 R2: provider item bodies go through the adapters (9.8), not straight into `Out`.
- S32 functional: an early wall-clock firing can stop a probe for good (the probe goes in stage B).
- S32 user-sim "felt wrong": the first tab freezes with no word when a second takes over; the status
  line reads Idle while probe items arrive; `probe.start` with two fields out of range names one.


## Done

- **S2** the agent layer's contract (PR #3, merged 2026-11-20 22:02 UTC, ~20 min in flight). A
  foreman now reads VISION, FUNCTIONAL and TECHNICAL as contracts: N1-N4, N6-N11 and R1 are
  rules; N5 open until stage F; TECHNICAL 14 names files per boundary; 15 is a watane checklist;
  17 names the repository per stage; AGENTS.md exists. Adversary BREACHED 2 (any mod could drive
  the checked provider; boundary files missing), fixed, HELD. User-sim REJECTED twice on a gap
  older than the slice (how a launch gets a scratch workspace id); named in TECHNICAL 4.6 as a
  precondition of stage B's first slice, accepted by the PO. Implementer runs 1/1/2/1. No suite.

- **S5** the performance review's must-dos in the contract (PR #5, merged 2026-11-20 22:37 UTC,
  ~1.5 h). A foreman of stage A, B or D now finds a bound for every size and wait: 256 KiB entries
  and inputs (larger entries as lossless `part`s), `queue/<id>`, a 512 KiB session record,
  `too-large(u64)`; `stream-sink.items` returns `taken | defer` with a lag clock; slots 8/8/16 with
  `busy`; a trim mark; `read` pages backwards (`before`); budgets per stage in 17; cases 34-53.
  TECHNICAL 10: the checked provider judges only after `initialized`, which waits on the workspace
  consent. User-sim ACCEPTED on its 4th run; adversary BREACHED 3, fixed, HELD twice. Implementer
  runs 1/1/1. No suite (gate: links, under 1 s).
- **S3** the kernel's core (PR #6, merged 2026-11-20 22:49 UTC, ~1 h). A client mod's maker runs
  `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile> <overlay> <backend>
  <model>` and sees every refusal at once. Crate `agent-kernel` (serde only): errors (12),
  profiles and grammar (4.1, 4.2, 9.2), narrowing overlays, the refuse table (11). `factory/gate.sh`
  is the gate. User-sim ACCEPTED; adversary BREACHED 6 (Bash allow rules, skill frontmatter), all
  fixed with tests in `kernel/tests/it/adversary.rs`, HELD on pass 7. Implementer runs: 3 tasks,
  6 fix tasks. Suite: 57 tests, CI gate job 17 s.

- **S4** the console on a scripted session (PR #8, merged 2026-11-20 23:26 UTC, ~1.7 h). Alice
  runs `npm run dev` in `console/` and sees a session on a scripted fake kernel: her message,
  folded thinking, "Explored · n reads", an edit card with its diff, an open ask with its bar,
  "Needs you (1)", the status line counting down, the plan, the queue; a reload doubles nothing.
  assistant-ui's External-store runtime over the console's own reducer (`console/kit/`).
  User-sim ACCEPTED on run 2; adversary HELD after 5 fix rounds (one governor of 8 burst, 4/s
  over attach and read). Merged before the PO's side-by-side check: the PO's comparison with
  `Main-2` found the gaps that became S9. Implementer runs 2/1/1 + fixes 1/1/1/1/2/1. Suite:
  `npm test` 167 tests 6.2 s, gate 24 s, `npm run test:slow` 7.5 s.

- **S8** the kernel's adapters (PR #10, opened and armed by the PO, merged 2026-11-20 23:42 UTC;
  19 min in flight). `cargo test` in `kernel/` turns each row of 9.8 into its ACP v1 update and entry from
  golden fixtures; unknown lines become `unknown`, never an error. Real libraries: claude-codes
  =2.1.330, agent-client-protocol-schema =1.20.0. Adversary HELD with a RISK on 4.5 (PO decision
  below); user-sim ACCEPTED. Implementer runs 1/1. Suite: 66 tests 0.23 s; gate 13 s.

- **S6** the review's should-dos and S3's spec debts (PR #11, merged 2026-11-20 23:45 UTC, 21 min in flight). A foreman of
  stage B or D finds P07-P12 as rules with cases 54-59: active history from the newest compaction
  (P07), 64 KiB transcript chunks (P08), `stream/<id>` and `live/<session>` keys (P09), one timer
  per session, 240 kernel-wide and 120 per owner (P10), host byte budgets 512/128/16 MiB (P11),
  kept keys charged to their owner in a 224 MiB account (P12); real kernel files in 1 and 14;
  4.1 names every refused first word and runtime prefix; `ask-expiry` in seconds. User-sim
  ACCEPTED on run 2; adversary BREACHED 2 (the refused-word list), HELD on pass 3. Implementer
  runs 1/1/1. Gate 14 s, no tests changed. Code to change, for the next kernel slice: drop
  `yarn ls`, add the missing runner names and prefixes, the frontmatter `:` rule.

- **S11** the transcript store (PR #12, opened and armed by the PO after its BLOCKED, merged 2026-11-21 00:11 UTC; ~45 min).
  A kernel maker sees transcripts of 10 and 10 000 entries stored in 64 KiB chunks over an
  in-memory host-7 state with counted gets; entries over 256 KiB as UTF-8-safe `part`s joined byte
  for byte; `read` both ways under 256 entries and 2 MiB; attach's replay decision (128 / 2 MiB,
  else `behind`); `unreadable` keeps its `i`; `Bash(yarn ls)` refused. User-sim ACCEPTED (run 1);
  adversary BREACHED 1 (`read(u64::MAX)` overflow), fixed, HELD.

- **S10** the console's wire and S8's 4.5 rules in the contract (PR #13, armed by the PO, merged 2026-11-21 00:14 UTC; 24 min).
  A stage B kernel maker builds every stream message and entry body the console reads as JSON:
  `answer` takes four decision shapes (others refused), the turn JSON per state, 9.4 "The wire";
  4.5: a `control_request` without a string `tool_use_id` is denied, and a stdout line typed like
  any watane item (or not one UTF-8 JSON object) goes out as `{"type":"wrapped"}`; 9.8 "Give
  nothing"; cases 60-65; AGENTS.md tests made true. User-sim ACCEPTED on run 2; adversary BREACHED
  4 then 1, fixed, HELD on pass 3. Implementer runs 1 each of 5 tasks. Gate 20 s.

- **S13** S11's spec debts (PR #14, merged 2026-11-21 00:25 UTC, 5 min in flight). 9.4: only `attach` moves `after` to a
  group's first part, `read` takes it as given (case 34); 2.3: `max` 1 to 256, 0 refused, over 256
  read as 256; 7: "A call commits whole"; 9.4: `unreadable` keeps the `i` of its place, and "Who
  makes entries". User-sim ACCEPTED (run 1); no adversary (no code, 14 unchanged). Gate 16 s.

- **S12** the kernel catches up with the spec (PR #15, merged 2026-11-21 00:36 UTC, 19 min). `RUNNERS` equals 4.1's 98 words
  (a test reads 4.1) plus runtime prefixes; frontmatter `:` rule; `unreadable` at its own place;
  `append` takes only whole-entry kinds; `read` refuses `max` 0 with field `max`; the Claude
  adapter builds the 9.4 ask from the item (`expires-at` from the item, no `options`), writes
  `ask-resolved` expiry as a decision entry, `thought` on a message, `_meta.watane.at` on every
  update. User-sim ACCEPTED (run 1); adversary HELD (pass 1) on profile, transcript and ask
  boundaries, with RISKs folded into S14 and S16. Implementer runs 1/1/1. Kernel 102 tests; gate
  26 s.

- **S9** the console matches the frames (PR #16, compared and armed by the PO, merged 2026-11-21 00:45 UTC; ~1.2 h). Alice's
  console now reads like canvas Revision 2: the edit ask as one card (pencil, path, "Waiting for
  you", Input, a tinted folded diff, "Allow this edit?" with Deny… / Allow edits in `<reach>` this
  session / Allow once), the status line with subject and "denied in m:ss if unanswered", the
  Needs you pill, the frame's composer with Stop and the model line, header chips with Context %,
  the grouped "Agents" sidebar, chevrons on folds; the console flake fixed (`ready()` waits for
  every row kind, 20/20). PO check at 1440 and 760 px, light and dark: matches; the ask's full
  Input above the diff departs from the frame by TECHNICAL 14 (the section holds over a frame).
  User-sim ACCEPTED on run 3; adversary BREACHED 4 (the ask card could hide what runs), fixed,
  HELD. Implementer runs 1/1/2 + 8 fixes of 1. Suite: `npm test` 230 tests 7.8 s (+26%),
  `test:slow` 23 s, gate 45 s.

- **S16** S12's spec debts (PR #17, merged 2026-11-21 00:47 UTC, ~7 min). The done "Code to change" lists are gone; what S14
  still owes is named once, in 9.4 "Code to change (S14)", with case 66; 4.5: a host ask or check
  id is unique on its stream, never reused; 9.8: an item without `input` or with an open id is
  `unknown`; 9.4: `raw` as a JSON string, every line takes the `i` of its place, a repeated
  thinking block counts once. User-sim ACCEPTED (run 1); no adversary (no code, 14 unchanged).
  Gate 15 s.

- **S14** the kernel finishes the wire (PR #18, merged 2026-11-21 01:03 UTC, 22 min). Strict line reading (non-UTF-8 is
  `unknown`, bytes as `\xNN`); a duplicate top-level key is `unknown`; every line `read` or replay
  gives takes the `i` of its place, joined groups too; an ask or check without `input`, with a
  reused open id, or with a non-string id is `unknown`; a repeated thinking block is not pushed
  again. User-sim ACCEPTED on run 2; adversary BREACHED 3 then 1, fixed, HELD on passes 3 and 4
  (ask and privacy boundaries). Implementer runs 1/1/2/1/1/1. Kernel 111 tests; gate 18 s.

- **S17** the restart pass in the contract (PR #20, merged 2026-11-21 01:08 UTC, 15 min). A stage B kernel foreman finds:
  a host boot number `caller.boot()` (2.1, a watane need in 15); `active/<n>` (4 pages, at most
  1 024 sessions, else `busy`) of running, waiting and paused sessions; a lazy restart pass of at
  most `A + 1` calls, one session each, no key listing; one outcome per state (Claude running or
  waiting: asks denied `restart`, turn `unknown`; paused unchanged; OpenAI re-queued or kept);
  `say` on `unknown` resumes; cases 67-71. User-sim ACCEPTED on run 2; no adversary (no code, 14
  unchanged). Gate 17 s.

- **S19** the kernel fails closed on S14's open points (PR #21, merged 2026-11-21 01:21 UTC, 12 min). An `ask-resolved` with a
  `by` other than `expired` or `cancelled` is `unknown` and the ask stays open; an ask or check
  whose `input` is not an object or whose `expires-at` is not a u64 integer is `unknown`; a joined
  whole of kind `part` is `unreadable` (dropped when its last part has no whole `i`). User-sim
  ACCEPTED (run 1); adversary HELD twice (one RISK fixed in S19-fix1). Implementer runs 1/1/1.
  Kernel 116 tests in 1.63 s.

- **S18** spec text for S14 and S19 (PR #22, merged 2026-11-21 01:30 UTC, 16 min). 4.5: `ask-resolved` `by` is `answered`
  (with `outcome`), `expired` or `cancelled`; the host serializes every item itself. 9.8: strict
  item rules 1-6. 14 "Not protected": a first-key JSON reader, a nested duplicate key in `input`.
  AGT-F-10: "never show a turn as running when nothing drives it". Case 72. User-sim ACCEPTED
  (run 1); adversary BREACHED 2 (`answered` closing a provider-denied ask with no decision;
  `allow` on a check), fixed, HELD on pass 3. Gate 18 s.

- **S15** the console reads the wire S10 wrote (PR #23, compared and armed by the PO, merged 2026-11-21 01:35 UTC; 44 min).
  Alice's console reads S10's kernel JSON unchanged: `expires-at`, `answer` by the ask's `id`,
  `part` groups joined across pages, replay and live, checks with no buttons, the ask card from
  the ask entry only (each entry keeps its own call); S9's "felt wrong" list fixed. PO check at
  1440 and 760 px: look unchanged from S9, Allow once at the end of the bar. User-sim ACCEPTED
  (run 1); adversary BREACHED 1 (two entries on one call id), fixed, HELD over 4 000 random
  sequences; S9's held cases kept. Implementer runs 1 each of 5. Suite: `npm test` 280 tests
  10.0 s (+19%), `test:slow` 30 s, gate 21 s.

- **S22** one decision per ask in the record (PR #24, merged 2026-11-21 01:51 UTC, 14 min). A Claude ask's decision is written
  once, when it closes: the kernel's own answer is a pending answer until `answered`, and an
  outcome that differs (an unsafe verdict after an owner's allow) gives one `deny, by: checked`;
  the host closes a check with `expired` too (watane, 15); `ended` closes every open or pending
  ask; `pending-answers` capped at 64; a forging checked provider named in 14 (record only);
  case 73. User-sim ACCEPTED (run 1); adversary HELD twice (RISKs 1-5 fixed in S22-fix1).

- **S21** sessions, turns, the active index and the restart pass (PR #25, merged 2026-11-21 02:12 UTC, 46 min). A kernel maker
  drives sessions through 9.3 on `MemState` with a `FakeHost` (`caller.boot()`, timers, provider
  requests as recorded calls): start, say, queue, unqueue, answer, rename, delete, list; the
  active index; the restart pass, each call bounded; cases 67, 68, 69 (Claude), 71. `start`
  checks `access/<identity>`, failing closed. User-sim ACCEPTED on run 2 (Continue with a full
  queue); adversary BREACHED 2 (queue bytes counted before escaping; `start` ignored `access/`),
  fixed, HELD twice. Implementer runs 1 each of 7. Kernel 179 tests in 3.18 s; gate 22 s.

- **S24** S21's spec debts (PR #26, armed by the PO, merged 2026-11-21 02:29 UTC; 12 min). 9.3 gains `open`; 9.1 the formats of
  `active/`, `queue/`, `access/`, `key/` and `owner/`; `access/` is reread beyond `start` (say,
  interrupt, R5, answer allow, a turn's end), failing closed, with an unlisted name refused the
  same whether or not the profile exists; an OpenAI profile needs `model`; a child session needs
  no `access/` entry; revoking access does not stop a running turn (14); case 74. User-sim
  ACCEPTED (run 1); adversary HELD twice, 7 RISKs settled in the text.

- **S20** asks as the kernel records and shows them (PR #27, merged 2026-11-21 03:12 UTC, 37 min). One decision entry per
  Claude ask, written at close; pending answers in the record; `answered`, `expired`, `cancelled`
  decided against the record; `ended`, `cancel` and R4 close every open or pending ask, at most 64
  per session (a 65th denied `busy`); ask cards carry an ACP `kind` and, for edits, a diff from the
  input (none with `replace_all`); a second ask on a `toolCallId` with another tool or input is
  `unknown`; `interrupt` and `cancel`; `access/` reread by every call that gives a session more
  work. User-sim ACCEPTED (run 1); adversary BREACHED 3 then 1 (a reused id from a second stream
  or a reload; `MultiEdit` with `replace_all`), fixed, HELD. Implementer runs 2/1/1/1/1. Kernel 227
  tests in 3.9 s; gate 18 s.

- **S25** quotas (PR #28, merged 2026-11-21 04:04 UTC, ~48 min). Owners are held to 9.3 "Quota": 64 sessions and 64 MiB per
  owner (settable), the 224 MiB account and 32 MiB reserve, more owner pages past 256, `quota`
  refusals naming their cap, a running turn ends `stopped {limit: quota}`, kept keys stay charged;
  a key costs its name plus its value, metered per call; past quota a new ask is denied and the
  turn interrupted. User-sim ACCEPTED on run 2 (refusal text named the wrong cap); adversary HELD
  twice. Implementer runs 1/1/1. Kernel 262 tests in 3.67 s; gate 21 s.

- **S7** the console stays fast on a long session (PR #29, compared and armed by the PO, merged 2026-11-21 04:41 UTC; ~3 h).
  Alice opens a running 5 000-entry session and scrolls, types and stops without lag: about 35
  rows mounted, one render per frame, keystroke p50 ~30 ms, Stop 27 ms, code highlighted (Shiki,
  lazy chunks) once complete and visible; the reducer's cost per message is flat; `test:slow`
  builds once; the transcript does not follow the end while focus is in an open ask's card, with a
  "New activity below" pill. PO check: look unchanged from S15 at 1440 px; the long session in dark
  renders right. User-sim ACCEPTED on run 2 (page scrolled; `npm test` over budget); adversary
  BREACHED 2 (the stream carried a focused ask card out of view), fixed, HELD; earlier held cases
  kept. Implementer runs 1 each, fix5 2. Suite: long fixture in `npm test`; budget 14.5 s.

- **S27** the console's rules after S7 in the contract (PR #30, merged 2026-11-21 04:53 UTC, ~8 min). 18.2: the reducer's view
  is what the transcript draws, the runtime gets the newest 100 rows; 18.6 Place: no follow while
  focus is in an open ask's card, Needs you ends it, the pill resumes it, a pointer press on the
  scrollbar or the card's text does not count as leaving (S23 builds it); 18.6 "Long sessions":
  viewport ± 800 px, at most 100 rows plus focused and open-ask rows, one render per frame, code
  highlighted once complete and visible with a per-block budget (S23 sets it). User-sim ACCEPTED
  (run 1); no adversary (no code, 14 unchanged).

- **S26** the kernel tracks its provider streams (PR #31, merged 2026-11-21 05:30 UTC, 1.3 h). A kernel maker drives a Claude
  session through a fake provider stream: `stream/<id>` and the record's `streams`, 8 provider
  slots (4 per owner, `busy`, case 52), the 10-minute idle release through `s/<id>`, resume with
  `resume: true` after a release, `ended`, `cancel` or restart (not for `persist: false`),
  `attach` with `live/<session>`, deferral rolled back whole and applied once, `lagging` and
  `replaced` closes resuming the provider, a delta reads only two keys (P09); S25's three items.
  User-sim ACCEPTED on run 2 (a deferred provider stayed deferred after a kernel close);
  adversary BREACHED 1 then 1 (past-quota `turn` entries), fixed, HELD. Implementer runs 1 each
  of 7.

- **S28** no session is left running with nothing driving it (PR #32, merged 2026-11-21 06:07 UTC, ~35 min). A provider
  `ended` mid-turn closes the open asks and ends the turn `failed {provider-ended}`; the next `say`
  resumes the Claude session; the console shows "Try again". `ended` never fails on quota (charged
  past the ceiling, as the restart firing). The past-quota flag is a `Past` value only
  `Past::load` makes. Case 76. User-sim ACCEPTED (run 1); adversary BREACHED 1 (the new `turn`
  entry made `ended` fail at the ceiling, leaving the session running), fixed, HELD. Implementer
  runs 1/1/1. Kernel 342 tests in 3.8 s; gate 28 s.

- **S29** ending a turn never fails at the ceiling (PR #33, merged 2026-11-21 06:51 UTC, 0.6 h). At and past the ceiling,
  `cancel`, `interrupt` without a replacement, provider items while past and `ended` succeed and
  free what they hold, charged into the reserve; new work is refused `quota`. Past quota a decision
  is the short form `{id, kind, ask-i, decision}` (counted at each write), so a closing call with 64
  open asks of 300 KiB writes 9.4 KB; case 77. User-sim ACCEPTED (run 1); adversary HELD, BREACHED 1
  (a pending id's decided ask never closed, so repeats stored without limit), fixed, HELD.
  Implementer runs 1 each of 4. Kernel 369 tests in 3.9 s; gate 29 s.

- **S31** the broker (PR #34, merged 2026-11-21 07:31 UTC, 0.6 h). Session allows answer later
  matching Claude asks (`by: session-allow`), with `access/` reread before each; `take-back`; no
  owner stream or an `agent:` owner denies at once (`no-owner`); the OpenAI rule matcher as a pure
  function; a `Bash` allow is the exact command (option a of Waiting). User-sim ACCEPTED (run 1);
  adversary BREACHED 1 (a Bash allow covered `dangerouslyDisableSandbox`), fixed, HELD.
  Implementer runs 1 each of 4. Kernel 400 tests in 3.2 s; gate 31 s.

- **S23** the console after S7 (PR #35, compared and armed by the PO, merged 2026-11-21 08:23 UTC; ~3.3 h). An answered ask
  shows "Answered, waiting" (or "Checking with your judge") until its decision, with no second
  click; a scrollbar press or the card's text no longer carries the ask out of view; code
  highlighting cannot freeze the page; two open asks behave; status line, sidebar and pills agree;
  each view is tied to its subscription, so an answer never reaches another session. User-sim
  ACCEPTED on run 3 (72-run matrix); adversary breached on 8 of 10 passes (answers reaching a moved
  or switched ask), each fixed with a red test and a randomized open-ask invariant, HELD on passes
  7 and 10. PO check at 1440 px: look unchanged; "Answered, waiting" shown.

- **S33** session allows fail closed (PR #36, merged 2026-11-21 08:24 UTC, 0.3 h). Bare-name allows
  only for `TodoWrite` and `mcp__watane__<tool>`; a Bash reach pins `timeout` and
  `run_in_background`; a Glob reach needs a relative pattern; 4.1: the host writes
  `allowUnsandboxedCommands: false`; the gate runs `cargo fmt --check`. User-sim ACCEPTED; adversary
  BREACHED 1 (an absolute Glob pattern escaped a workspace allow), fixed, HELD. Kernel 411 tests.

- **S1** the open probes of section 18 (PR #37, local foreman, merged 2026-11-21 08:31 UTC,
  1 h 39 min). A maker runs `python3 -I proto/claude-probes/probes.py subagent-slash loopback` on
  the Mac: V26 held (a leading `/` in a subagent prompt runs no skill; a `Skill` deny stops it);
  V27 BREACH (an allowlisted `localhost` reaches loopback through the sandbox's proxy; LAN and
  link-local behind any allowlisted name too), under Waiting. User-sim ACCEPTED; adversary
  BREACHED 1 (recorded as the finding), HELD on the fix. Gate 247 s on the Mac.

- **S34** scope keys for session allows, and `host-info.generation()` (PR #39, merged 2026-11-21
  09:25 UTC). The human's rule built: a session allow holds one (tool, scope key) computed afresh
  from the input that will run (Bash: the exact command and its pins; file tools: the workspace;
  WebFetch: `domain:<host>`; Skill: its name; every other tool and any unknown input key: no key,
  always asks); writes to dot-files, non-ASCII paths and named build files get no key; restart
  detection is `host-info.generation()`; `timer-error` gains `bad-delay` (30-day cap); 4.1 gains
  `profile`; 15 gains the H2 rows. User-sim ACCEPTED; adversary HELD, BREACHED 2 (a Unicode
  `Makefile`; an uncharged timer firing at the ceiling), fixed, HELD.

## Deputy decisions

- 2026-11-20 S2's user-sim rejection is accepted: the failing outcome is a gap of the proposal,
  now named as a precondition of stage B (TECHNICAL 4.6), not a defect of the slice.
- 2026-11-20 Adversary RISK 3 of S2: the checked provider's first judge call waits until the
  workspace consent is held (option b; one TECHNICAL 10 line to undo). Written in by S5.

- 2026-11-20 The performance review (68a27f3, another session, on the human's ask) becomes
  slices S5 (must-dos P01-P06, P13, P15), S6 (P07-P12), S7 (P14, console). Its record and the
  AGT-F-8 edit reach main with the PO's next push.

- 2026-11-20 The human is away overnight (standing orders from the session that started this
  PO): no questions to the human until morning; the PO decides and records here.
- 2026-11-20 The kernel's pure logic is a plain Rust crate in `kernel/`, tested natively, with no
  WIT until watane has the 2.1 additions; the WASM component wraps it later (keeps R1 and the
  host interfaces open).
- 2026-11-20 One gate script `factory/gate.sh` (S3) runs every part that exists: kernel, console,
  mod contract. Later slices add no gate lines.
- 2026-11-20 The console is built against a scripted fake kernel feed of ACP v1 plus
  `_meta.watane` (N6), so it needs no host.
- 2026-11-20 S1 runs locally: its outcomes need the Mac's logged-in `claude`.

- 2026-11-20 S3's spec debts (real kernel file names in TECHNICAL 1 and 14; tools, ask-expiry,
  overlay keys, frontmatter, Bash allow grammar) go into S6, which owns `design/` next; the code
  follow-ups (drop `yarn ls`) go to the next kernel slice after S8.
- 2026-11-20 S3's foreman refused any `*` (and `:*`) in an allow rule for a package runner:
  fail-closed, so `Bash(npm test:*)` is refused. Kept; the wider question is under Waiting.
- 2026-11-20 Next kernel slice is S8, the adapters of 9.8: pure functions, disjoint from S4
  (console) and S6 (design), needed by stage B.

- 2026-11-20 S4 is accepted as behaviour; its look goes back as S9 (the frames give layout and
  words, TECHNICAL 18.4). S7 waits for S9 (same files). S4's wire assumptions and spec notes
  become S10 after S6 (same files). Its decisions stand meanwhile: decision JSON
  `{"allow":true[,"for":"session"]}` / `{"deny":true,"reason"}`; acts with no 2.3 call shown off.

- 2026-11-20 S8 BLOCKED on 4.5 (ask/check items lack the call id; a `claude` stdout line typed
  like a host item would pass as an ask; `expires-at` on the kernel's clock). Decided: merge S8 as
  built (it reads `tool_use_id` when present, null otherwise, never guessed), and write the three
  rules into 4.5 in S10 (option 1). Also under Waiting: it is a wire rule of the watane host.

- 2026-11-21 S6 settled the Bash boundary as option (a) of Waiting (TECHNICAL 14 "Not protected":
  no refused-word list is complete; a missed word runs only inside the shell grant's sandbox, for
  a profile the participant made or adopted). Kept; the human may still choose (b).
- 2026-11-21 S6 open RISKs: kept keys and timers per owner are accepted limits named in 14 (same
  pattern as slots); finding running sessions at restart without a prefix scan goes to stage B's
  first kernel slice; the session allow on `Bash` is under Waiting.
- 2026-11-21 Next kernel slice after S11 is S12: S6's code to change (runner names, `yarn ls`,
  frontmatter `:`), then `expires-at` once S10 writes it.

- 2026-11-21 S11 BLOCKED on 9.4: "read with an after inside a group starts at its first part"
  loops forever on entries over a page (cases 34, 49). Decided (a), as built: only `attach` moves
  `after` to the group's start; `read` takes `after` as given (a client that lost the start reads
  with `after = i - n`). Undo: one call, one test, one sentence; (c), a `restart` flag on `read`,
  can come on top. The sentence goes to S13; the RISKs to S12 and S13.

- 2026-11-21 S10's foreman decisions kept: keys ACP has keep its spelling (`toolCallId`), the
  kernel's own are kebab-case (`expires-at`); a decision is a later `ask` entry repeating the ask;
  an ask goes out live once, as its entry; the wrap list stays a blocklist (each new item type is
  added to 4.5), the envelope option still declined. S10's code lists become S14 (kernel) and S15
  (console); S12 was already out, so S14 follows it.

- 2026-11-21 S14's open points, decided fail-closed: an unknown `by` on `ask-resolved` is
  `unknown` and the ask stays open; a non-object `input` or a negative or fractional `expires-at`
  is `unknown`; a joined whole of kind `part` is `unreadable` (S19). Accepted limits, named in 14
  by S18: a first-key JSON reader would read a duplicate `i` differently (every reader here reads
  the last); a nested duplicate key in `input` is safe only while the host serializes the item.

- 2026-11-21 S19 consequence: with an unknown `by` read as `unknown`, every answered ask would
  add an `unknown` entry, since 4.5 names no `by` for a person's answer. Kept fail-closed; S18
  lists the `by` values, and S20 makes the answer ones give nothing.

- 2026-11-21 S15's open risks: (a) two entries on one `toolCallId` and (c) the ask's `toolCall`
  without `kind` or diff go to S20 (kernel and 9.8, fail-closed); (b) a part group over 256 parts
  read mid-group shows one unknown row that can only expire: accepted (fails safe; entries over
  64 MiB do not occur under the 256 KiB-per-part rule's caps).

- 2026-11-21 S22 RISK 1: open Claude asks and checks are capped at 64 per session; one past it
  is denied at once (`busy`), so every closing call writes at most 64 decisions (S20).

- 2026-11-21 S20: a `for: session` allow given before `access/` is revoked must not keep allowing
  calls: the slice that builds session-allow matching (the broker, 9.6) reads `access/` before
  answering from a session allow, failing closed. The host keeps checking `allow` + `input` (2.3).

- 2026-11-21 S25 RISKs: `delete` skips a failing restart step; at most one `turn` entry per turn
  past quota; the quota refusal stops naming the kernel's shared account (all to S26). Accepted:
  duplicate lines from a restart step that fails in a timer firing (the real host rolls a call
  back whole, 7; only `MemState` lacks it); no migration (nothing deployed).

- 2026-11-21 S26 RISKs: a provider `ended` during an active turn ends it `failed {provider-ended}`
  (S28); the past-quota flag is made unreadable without the accounts read (S28). Accepted: the
  transcript's last `turn` entry may trail the record past quota (attach and pushes carry the
  record's turn); decisions use the chunk cap's kept room (older); for a moment after `cancel` the
  host may run one more `claude` than the kernel counts (the host's own caps bound it).

- 2026-11-21 S28 RISKs: every call that ends a turn or a session's work (`cancel`, a turn ended
  `stopped {quota}`, `ended`) succeeds at the ceiling, charged into the reserve; past the ceiling a
  decision carries the ask's index, not its body (S29). Accepted: a deferred `ended` that the host's
  backstop ends `consumer-failed` (host H1, stage B); `ended` on an older-boot record (only if the
  host breaks R1).

- 2026-11-21 S29 RISK: the reserve is shared, the bound per call: about 45 owners crossing at once
  with full sets of open asks could pass the 32 MiB reserve, then the host's 256 MiB limit refuses
  the writes. Accepted as a named limit (9.3 states the bound). Also accepted: a 256-byte `reason`
  can escape to ~1.5 KiB; an adapter-made decided ask below quota is stored as given (no adapter
  makes one; build it from the record when one does).

- 2026-11-21 The human trusted the agentic-harness folder (via the parent session); S1 goes to a
  local foreman.

- 2026-11-21 S31 RISKs: no session allow for `Task`, `Skill`, `WebFetch`, `WebSearch`; a Bash
  reach pins `run_in_background` and `timeout`; the host sets `allowUnsandboxedCommands: false`
  explicitly (all S33; also under Waiting, as a security boundary). Accepted: `allows` changed shape
  with no migration (nothing deployed).

- 2026-11-21 The human: slices were cut by layer (kernel-only, console-only, spec-only), not as
  stories across components. Corrected: every slice is a story a person can see, crossing watane's
  host, the kernel mod and the console thinly; spec lines and risk fixes go into the slice that
  builds the feature. S30 (console-only) stopped and folded into S32; S34 (the human's scope-key
  rule, kernel and spec) finishes. Stage A is one story in two repositories (watane S96 with S32).

## Human answers (2026-11-20, through the PO)

S2 writes each into the document that owns it.
- **N1** Two Claude Code grants: file tools in confined folders, and shell-and-web inside Claude's sandbox.
- **N2** `auto` permission mode stays out.
- **N3** All listed changes to watane's contracts are accepted, F-AGENT-2/W6 included.
- **N4** Keep full fsync per write; decide after V8 (stage A) measures it.
- **N6** Stay on ACP v1, with the kernel's own entry ids, turn state and replay end.
- **N7** Workspace instructions off; a later opt-in profile field.
- **N8** watane gets MIT OR Apache-2.0 for all of it. A watane change: cross-project, not ours.
- **N9** Subagents stay `--agents=` flags, capped at 64 KiB.
- **N10** The host keeps the judge's key as an account bound to the judge's origin.
- **N11** The console shows only its own sessions; F-AGENT-6 stays as it is (asked after commit 3926a0d raised it).
- **R1** This repository is the kernel `mods:agent`; each provider has its own repository.

- **2026-11-21, the human:** (1) the three host-wire additions are accepted, with restart detection
  as a new interface `host-info` with `generation()` (in place of `caller.boot()`); `outcome` on
  `answered` and checks closed by the host with `expired` as written. (2) Session allows use a scope
  key per tool: a pure scope function turns a call's input into the smallest part that decides
  what it can reach and that a person can judge at a glance (Bash: the exact command; file edits:
  the workspace; a web fetch: the host; a skill: its name). A tool without one, or an input that
  cannot be scoped safely, gets no key and always asks. The kernel computes the key from the input
  that will run and shows it on the ask; "allow for this session" stores (tool, key); a later call
  is allowed only when its fresh key equals a stored pair exactly; take-back and session end remove
  it. (3) Bash: the exact command. (4) The sandbox is the boundary for Bash allow rules.

## Waiting on the human

- **N5 The factory's implementer needs a shell.** The human chose to decide at stage F. Options:
  (a) Claude grant with shell and web (nothing new; (b) can follow); (b) a `codex app-server`
  backend in the host, a third native process kind (a new host boundary, costly to remove once
  mods use it). Rec: (a). Blocks only stage F.

- **V27: the shell grant's network allowlist reaches loopback through Claude's sandbox proxy**
  (S1, PR #37, security boundary). With `allowedDomains: ["localhost"]`, a command reaches a
  loopback server through the sandbox's own proxy (direct connections stay blocked); the proxy also
  reaches LAN and `169.254.169.254` addresses behind any allowlisted name. TECHNICAL 4.2's
  "loopback denied" holds only if no allowlisted name names or resolves to such an address.
  Options: (a) the kernel refuses allowlist entries that are `localhost`, `*.localhost`, IP
  literals, or private, loopback or link-local ranges (cheap, at profile check; does not stop
  DNS names that resolve there or rebinding); (b) the host puts its own egress proxy in front of
  the sandbox that checks the resolved address at connect time and refuses loopback, private and
  link-local (robust; a watane host part, stage C with the shell grant); (c) name it in 14 "Not
  protected" (the participant chose the allowlist). Rec: (a) now plus (b) before the shell grant
  ships. Undo: (a) one refuse-table row; (b) a host component.

## Friction

- 2026-11-20 po skill 0: "the order of work is the vision's steps"; this vision has none, the
  order is in TECHNICAL 17. The skill should allow "or the order of work the spec chain names".
- 2026-11-20 config `boundaries` points at TECHNICAL 14, which names sections, not files; the
  foreman's adversary rule matches diff files against it, so it can never match. adopt.sh (or
  the skill) should say the section must name files. S2 fixes it here.
- 2026-11-20 gate `check-mod.sh` passes with nothing checked while there is no `wit/world.wit`:
  design and probe slices merge on a vacuous CI. Fine for now; a design-only fallback check
  (links, `//!`-style headers) would give such slices a real gate.
- 2026-11-20 po skill 2/README "Slices that span projects" assume both projects run this factory;
  nothing says what to do when the provider (watane) runs its own older factory. Recorded as a
  cross-project dependency instead.
- 2026-11-20 PLAN template has no place for the human's answers (only `Deputy decisions` and
  `Waiting on the human`); added `Human answers`.
- 2026-11-20 po skill 4: "use AskUserQuestion only on a turn the human's own message started";
  a PO started by another session on the human's behalf cannot tell. Worked here; worth a line.

- 2026-11-20 a second session committed a design change (3926a0d: AGENTS.md, console, N11) to
  this checkout 14 s after the PO's plan commit, overlapping S2's sketch (AGENTS.md, drafts). The
  skill assumes the PO alone writes `main` besides merges; nothing tells another session that a
  slice owns those files. A `Touches` lock visible outside PLAN.md (or a PR for every design
  change) would have caught it.

- 2026-11-20 PO error: `git commit -a` in a checkout another session also edits swept that
  session's uncommitted `design/TECHNICAL.md` edits into plan commit 5e46864, on main. Nothing
  lost; S2 will rebase over it. The po skill should say: commit `factory/PLAN.md` by path only.

- 2026-11-20 po skill 3 (local): `claude --bg` in a fresh worktree stops on the workspace trust
  prompt; the skill does not say so, and an unattended PO cannot start any local foreman. The
  skill (or adopt.sh) should have the human trust the project once, or create local worktrees
  where trust is inherited.

- 2026-11-20 S2 foreman: user-sim and adversary are built around red code tests; on a documents
  slice they report checks instead (worked, outside their format). The skill's `gh` steps do not
  apply in the cloud (no `gh`; it used the GitHub MCP tools). It reported "about 2 hours in
  flight" for 20 minutes: a cloud foreman cannot tell elapsed time either.

- 2026-11-20 watch-cloud.sh missed S3 and S5 (the PO stayed idle for 1.5 h until the parent
  woke it). The foremen pushed `DELIVERED <id>` as an empty commit after arming auto-merge; the
  rebase merge drops empty commits and `delete_branch_on_merge` deletes the head, so CI (17-40 s)
  merged and erased the report before the script's 120 s poll saw it. The watchers were running
  and never exited. Fix: watch-cloud.sh should also return when `gh pr list --head slice/<id>
  --state merged|closed` finds the PR (or the foreman pushes DELIVERED before it opens the PR, and
  the report lives in the PR body). The PO now watches with an inline loop that checks both.
- 2026-11-20 S5 foreman: a design slice's user-sim fixes are new spec text, so it rejected three
  times on readings the previous fix introduced; the adversary can only propose cases as text.
- 2026-11-20 S3 foreman: it could not fix the spec-path gap (TECHNICAL 1 and 14 name `src/` at the
  root) because `design/` was locked by S5; it put it in the report. Worked as intended.

- 2026-11-20 po skill 5 vs auto-merge: a cloud foreman arms auto-merge as it delivers, so a UI
  slice merges (S4: 38 s of CI) before the PO can compare it with the design; `--disable-auto`
  came too late. A UI slice should open its PR with auto-merge off, and the PO arms it after
  the comparison (or the foreman skill names UI slices as "PO arms").
- 2026-11-20 watch-cloud.sh miss: fixed in the factory (watane-mods aaf89b6, synced by PR #7).

- 2026-11-21 Flake: S4's `ac1_app_shows_sidebar_header_rows_dock_on_the_played_story` races the
  played story (PR #9, rerun once). Weighed: *guard*, hits any PR; folded into S9 (owns
  `console/`, in flight, merge armed by the PO) rather than a separate slice on the same files.
  A cloud foreman cannot be told mid-run, so a late outcome is checked at acceptance.

- 2026-11-21 S9: every console task rebuilds the committed `ui/`, so parallel tasks always conflict
  there on rebase; and fix tasks for the user-sim reopened adversary breaches twice (a brief that
  changes the ask card should list the adversary's held cases as acceptance).

- 2026-11-21 S21: a validator committed a 550 MB build copy (`kernel/target-base`); the foreman
  force-pushed it away. The validator's instructions (factory) or `.gitignore` should stop it.
  Parallel kernel tasks all edit `tests/it/main.rs` and `error.rs`: trivial rebase conflicts.
  User-sim and adversary red tests should pass clippy themselves.

- 2026-11-21 S7: the `npm test` budget moved three times within the slice (8.6, 11, 12.5, 14.5 s);
  a budget should be fixed at the slice's end, not per task. Parallel console tasks conflict in the
  committed `ui/` every time (S9, S15, S7): a factory rule to build `ui/` once at integration.

- 2026-11-21 S26: briefs that paraphrase a spec rule missed two of them (user-sim and adversary
  caught both); a brief should quote the rule. Implementers still skip the red phase on some
  tasks, and `cargo mutants` is not in the cloud image.

- 2026-11-21 S28: a brief that changes what a call writes should quote the ceiling rule (9.3
  "Past"); the adversary found the gap. A brief's AC that says "existing tests show it" was false.

- 2026-11-21 S29: a load-sensitive timing test (`long_S7`) in the default gate fails parallel
  tasks' checks for code they did not touch (S30 moves it). No `cargo mutants` or `/usr/bin/time`
  in the cloud image.

- 2026-11-21 Driving a second project (watane, the bridge): its PLAN has its own format (Status
  markers, steps, Foremen) and a long history; the PO must read its README, PLAN and VISION first.
  Its checkout holds another session's uncommitted blog files, so `git pull --rebase --autostash`
  is unsafe there; `git merge --ff-only origin/main` works when the PO has no local commits. Its
  branches are lowercase (`slice/s95`) while ids are `S95`: the watcher must match both. A slice
  of one project that waits on another's merge has no watcher that spans repositories.
- 2026-11-21 po skill 3 Local: the hand-out's `cd` into the worktree moved the PO's Bash cwd there
  for the rest of the session; run `claude --bg` in a subshell.

- 2026-11-21 PO error, recurring (the human caught it): from S3 on, the PO cut slices by folder
  (kernel on an in-memory fake host, console on a scripted fake kernel, spec debts on their own),
  because the watane host parts did not exist and folders kept slices apart. Each layer then made
  more risks and spec debts, which became more layer slices. The po skill's "crosses every part
  it needs, thinly" was read as "within this repository"; a story that needs another project
  should be one story with a half per project, started together. Worth a line in the po skill:
  a slice whose story names only a test run ("a kernel maker runs `cargo test`") is a layer slice.

- 2026-11-21 S1 (local): `test:slow` not run on the Mac (no global playwright); `probes.py` exits 0
  even when an expectation fails. Local foreman worked once the folder was trusted.

- 2026-11-21 PO: S32 (stage A, end to end) shares `kernel/` and TECHNICAL with S34, still in flight;
  handed out anyway to start the end-to-end path the human asked for. S34 should merge first and
  S32 rebases on it. A cloud foreman cannot be told mid-run.

## Suite time

- S4: console `npm test` 167 tests 6.2 s; `test:slow` 7.5 s; gate with kernel 24 s (2026-11-20).

- S3: kernel `cargo test` 57 tests in 0.05 s; CI gate job 17 s (2026-11-20, macos-15).
