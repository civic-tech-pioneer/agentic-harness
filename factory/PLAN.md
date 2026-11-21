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

### S21: sessions, turns and the active index on the in-memory state (in flight (cloud), routine trig_013z6HkuQwsoLGpkNY4ZQhno)

Story: a kernel maker runs `cargo test` in `kernel/` and drives sessions through 9.3 on the
in-memory host-7 state of S11, with a fake `caller.boot()`: create, turn changes, queue, the
active index and the restart pass, each call doing bounded work.
Outcomes:
1. The session record of 9.3 with its caps (`key` 256 B, `title` 256 B, `allows`, `chunks`,
   `streams`, `deadlines`; record at most 512 KiB), the turn JSON of each state, `turn` entries.
2. The queue (`queue/<id>`): 32 inputs, 2 MiB, 256 KiB per input; `queued`, `unqueue`
   (`unknown-input`); the `{"type":"queue"}` push as a value the caller returns.
3. The active index (A2, A3): moves in and out of running, waiting, paused write one page and
   `slots`; a change among them writes none; at 1 024 sessions `start`/`say` is refused `busy`.
4. Restart (R1-R6) with a fake boot number: detection, the pass in at most `A + 1` calls, one
   session each, Claude outcomes (asks denied `restart`, turn `unknown`, `paused` unchanged),
   R5 (`say` on `unknown` resumes), R6 (a call on a stale record steps it first); cases 67, 68,
   69 and 71 as kernel tests. OpenAI outcomes (case 70) wait for stage D.
5. Keys (`key/<owner>/<key>`): a call that changes nothing is not kept (9.1).
Touches: `kernel/` only. Fakes: the host state (S11's), `caller.boot()`, timers as recorded
calls. Leaves out: streams and providers, the broker (9.6), quotas beyond counting, OpenAI (9.5).
Protects: *core* (*guard* for keys and caps).

### S7: the console stays fast on a long session (in flight (cloud), routine trig_01QKd2ZG4iQvqxnAxHagNAqc)

Story: Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and
presses Stop without lag (P14).
Outcomes: first move tests down the pyramid (S9: `npm test` 230 tests 7.8 s, over the 20%
budget; `test:slow` 23 s because each of 4 scripts runs its own `npm run build`: share one build);
the transcript is virtualized; at most one render per animation frame; code is
highlighted once its block is complete and visible; a large-transcript fixture with limits for
input, Stop and scroll latency runs in `npm test`.
Also (S15 user-sim): the status line repeats the kind before the title ("Edit Edit s.ts"); a
check decided `none` with no tool entry shows "Pending"; an open check sharing a card with an
ask shows no "Checking" state. Also (S17): a Claude `ToolCall` a page saw live before a restart is never closed by an entry;
after the restart's `unknown` turn the console shows it as ended, not running.
Touches: `console/`, `ui/`. Protects: *core*.

## Sketched

### S20: asks as the kernel records and shows them (cloud, after S21 and S22 merge: same files)

Story: a kernel maker runs `cargo test` in `kernel/` and asks are recorded and shaped as the spec
says, with the spec fixed alongside.
Outcomes:
1. `ask-resolved` as 9.8 rule 4 says (S18, PR #22): no entry for the kernel's own answer; another
   caller's answer gives the decision from `outcome` (`by: checked` on a `direct: false` stream,
   `host` otherwise); a bad `outcome`, `none` on an ask, `allow` on a check is `unknown`, the ask
   open; plus S22's 9.4 "Code to change (S20)" list (PR #24): the pending answer and the decision
   at close (D1), D2, `ended` closing every open or pending ask (D5), `cancel` and R4 in every
   state, `pending-answers` capped at 64 with `busy`; case 73.
2. S15 RISK (c): the kernel's ask `toolCall` carries ACP `kind` and, for edits, the diff content
   derived from the input (9.8), so a real Edit ask's card shows its diff, not "Other <path>".
3. S15 RISK (a): a second open ask or check on a `toolCallId` that already has an open one with a
   different `tool` or `input` is `unknown` (9.8), or the spec says why the console's handling is
   enough; decide in the slice, failing closed.
4. Spec: 9.4 "Code to change" drops the ten console bullets S15 built; 9.8 gains 2 and 3.
5. Polish: the refusal text of a bad `append` kind (`kind (, ): ...`) gets its field and reason.
6. S22 RISK 1 (PO decision): at most 64 open Claude asks and checks per session; one past that is
   denied at once (`by: kernel`, reason `busy`), so `cancel`, R4 and `ended` write at most 64
   decisions in one call; spec in 9.3/9.6 with a case. Also S22's wording: R4's Claude row and
   9.8 rule 4 no longer name a kernel deadline or `ask-expiry` for Claude asks.
Touches: `kernel/`, `design/TECHNICAL.md` (after S22 merges). Protects: *guard* (asks).

### S23: the console shows an answered ask as waiting (cloud, after S7 merges: same files)

Outcomes: an ask with a pending answer (S22 D1: decided at `answer`, recorded at `ask-resolved`)
shows "Answered, waiting" in place of its bar until its decision entry arrives, with no second
click; in checked mode the wait shows "Checking with your judge" (18.5). A UI slice: PO-armed.
Touches: `console/`, `ui/`. Protects: *guard* (asks) and *polish*.

### S1: the open probes of section 18 (local; blocked: worktree trust, see Waiting on the human)

Story: a maker runs `python3 -I proto/claude-probes/probes.py subagent-slash loopback` on the Mac and sees, for the
current `claude`, (subagent-slash) whether a subagent prompt that starts with `/` runs a skill the profile
denies, and (loopback) whether the shell grant's sandbox blocks a connection to loopback.
Outcomes:
1. `probes.py` runs each new probe by its name, alone, and prints a result line like the others.
2. Proposal section 18 gains rows V26 and V27 with the result, the `claude` version and the
   effect on the design; TECHNICAL 14's "not probed" line is not edited here (S2 owns it).
3. A breach (either one runs or connects) is written as the design effect and flagged to the PO.
Touches: `proto/claude-probes/probes.py`, `design/ARCHITECTURE-PROPOSAL.md` section 18 only.
Leaves out: V7 (judge account), V8 (stage A), V13 (unit test of host code).
Protects: *guard*. Stays local: its outcomes need the Mac's logged-in `claude`.

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

- **S22** one decision per ask in the record (PR #24, 14 min). A Claude ask's decision is written
  once, when it closes: the kernel's own answer is a pending answer until `answered`, and an
  outcome that differs (an unsafe verdict after an owner's allow) gives one `deny, by: checked`;
  the host closes a check with `expired` too (watane, 15); `ended` closes every open or pending
  ask; `pending-answers` capped at 64; a forging checked provider named in 14 (record only);
  case 73. User-sim ACCEPTED (run 1); adversary HELD twice (RISKs 1-5 fixed in S22-fix1).

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

## Waiting on the human

- **S1 cannot start a local foreman.** `claude --bg` in a new worktree refuses: "Workspace not
  trusted. Run `claude` in <worktree> once and accept the trust prompt". Accepting it is yours;
  the PO does not bypass a trust prompt. Options: (a) trust `watane-mods/agentic-harness` so its
  worktrees inherit it, then the PO starts S1; (b) run S1's probes by hand; (c) drop S1 (it is
  *guard*, two probes). Rec: (a). Undo: none needed.

- **N5 The factory's implementer needs a shell.** The human chose to decide at stage F. Options:
  (a) Claude grant with shell and web (nothing new; (b) can follow); (b) a `codex app-server`
  backend in the host, a third native process kind (a new host boundary, costly to remove once
  mods use it). Rec: (a). Blocks only stage F.

- **Bash boundary** (S3, S6): S6 wrote option (a): Claude's sandbox is the boundary for Bash, the
  refused-word list best effort. Your call to keep it or ask for (b), an allow list of named
  commands (stricter; makers write each command). Undo (a) to (b): profiles that rely on
  prefixes break.
- **Session allow on `Bash`** (S6 RISK, security boundary). An `{allow, for: "session"}` answer to
  a Bash ask allows any command for the rest of the session. Options: (a) hold the session allow
  to the 4.1 allow-rule grammar (it allows only that command's prefix, refused words refused);
  (b) name it in 14 "Not protected". Rec: (a), stricter and cheap now. Not blocking: no host yet.
- **4.5 host items (S8, wire format, decided as option 1, veto welcome).** Ask and check items
  gain `tool_use_id` and `expires-at` (host clock), and the host never passes through a stdout line
  typed like its own items. Alternatives: (2) leave it to stage B's host slice; (3) a source marker
  on host items, a wire change in both repos. Undo of 1: a few spec lines, one adapter field.

- **`caller.boot()`, a new watane host import** (S17, interface between projects, not in N3).
  The kernel cannot tell a restart from its own state, so it reads the host's start number on its
  first call. Options: (a) `caller.boot() -> u64`, a pure read (as written); (b) a host export
  `started()` called once per start. Rec: (a), no new call into mods at start. Undo: one 2.1 line,
  one 15 row, rule R1. Blocks only stage B's host part.

- **`answered` carries `outcome`; the host closes checks with `expired` too** (S18, S22: two small
  additions to watane's H2 wire, both on the H2 line in 15).
  Without it the kernel cannot record what a checked provider decided. Alternative: leave the wire
  as it is and design another way to learn the outcome at stage E. Undo: one field. Blocks only
  stage B's host part.

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

## Suite time

- S4: console `npm test` 167 tests 6.2 s; `test:slow` 7.5 s; gate with kernel 24 s (2026-11-20).

- S3: kernel `cargo test` 57 tests in 0.05 s; CI gate job 17 s (2026-11-20, macos-15).
