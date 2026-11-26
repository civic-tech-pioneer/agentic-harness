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
| E | Checked provider (needs a judge account, V7) | `provider-checked/` (R-B1); watane half done (S122) | S127 in flight; V7 on Jev: p50 173 ms, p95 267 ms |
| F | Delegation and counters (kernel 9.7), factory profiles | kernel | N5 = (a) for now |

**Cross-project dependency.** Stages A to D put code in watane (`../../watane`: host, dev, pkg,
app), which runs its own older factory and is not on this generic one. Each such stage becomes
one watane slice (provider, first) plus one slice here or in a provider repo (consumer). The
interface between them is `design/TECHNICAL.md` section 2.1 (`watane:host@0.1.0` additions) and
the watane contract rows of section 15. None of them is this product owner's to hand out until
watane is on the generic factory, or the human hands stage A to watane's own product owner. Stage
A's backend is pure watane; its console part (TECHNICAL 17) is this repository's.

## In flight

S138 (consumer half of watane S133/S136, Codex build); S139 merged.

## Sketched
### S138 the kernel acts on a timer firing only under its session owner's authority (consumer half of watane S133/S136; F6.1)
(sketched 2026-11-26 by Deputy Scissors, the main line; builder codex, reviewer claude by rule 3 (last two S137 claude,
S136 codex); in flight, cloud, Codex task https://chatgpt.com/local/6b08156b-80bc-83eb-94e0-70035ce5604a, 11:22 UTC; DELIVERED as PR #61 at 590c639 (+546/-12); r1: Claude routine `trig_017eS5rRjpFSfNeaRv74NHJN`, 11:51 UTC; spec-check not run: the contract is merged in watane (PR #48 design, PR #50 build, host 19.1, 19.6).)
Why: the guard in Upkeep (the human's (b) on S40 F6.1): a timer firing of an OpenAI session whose owner mod was disabled
during a timed pause may send one more model request under the kernel's origin. watane now gives a mod's own top-level
`timer-sink.fired` the read `timers.firing-origin() -> option<string>` (the resolved origin identity, the timer owner's
own identity after the client is disabled or removed, `none` elsewhere).
Outcomes (user-sim on the kernel's tests with the fake host, and on `watane-dev serve` where this cloud has it):
1. A timed resume or restart retry whose firing reads the session owner's identity runs as before.
2. A firing that reads `none`, the kernel's own identity, or any identity other than the owner bound at arm time does
   only local cleanup: no model request, no toolset operation, no consent; the session shows why (a test per case).
3. The owner disabled during a timed pause: the firing sends no model request (F6.1's known limit is removed from 9.5
   and the Upkeep guard is closed).
4. The fake host (`FakeHost`) answers `firing-origin`; the Harness default `none` and a `Fakes` override are used in the
   tests that run on watane-dev.
Touches: `kernel/` (timer firing paths in `openai/run.rs`, `sessions.rs`, the host adapter and `FakeHost`),
`kernel/tests/`, `design/TECHNICAL.md` (2.1, 9.5, 15, case 22). Not `console/` (S139).
Constraints: copy verbatim the section "Copy verbatim into agentic-harness design/TECHNICAL.md" of
https://github.com/civic-tech-pioneer/watane/pull/48 (its body) into 2.1 and 15; do not change the WIT shape (a gap is
BLOCKED). Settle S133 r1 P1.5 (the expected owner is the arming call's `caller.identity()`, never the kernel itself; a
mismatch through a front mod fails safe) and P1.12 (no install generation: a known limit, or a Door C note) in 9.5 with
spec-change (https://raw.githubusercontent.com/mboogerd/claude-plugin/main/skills/spec-change/SKILL.md). Case 22's
idempotent half stays a known limit (S133 gives no restart callback). Adversary: a front mod or another owner whose
firing tries to make the kernel act for a different owner (TECHNICAL 14).

### S35: stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude` (DONE 2026-11-23: PR #43 merged with --admin at 042b0dd on the human's word after the extra round, r6 changes 2 answered (F6.1 done on main, F6.2 fixed); was in flight, builder claude, reviewer codex; continuation DELIVERED 21:31 UTC at 8fc6db6 on watane c2a7d4d (routine `trig_01NpPnJxwNjH279WAFwnziDk`), screenshots in `factory/runs/S35/screens/`; r1: Codex review dispatched 21:40 UTC, https://chatgpt.com/local/6b035f75-9cbc-83eb-953f-4ea568e56065; r1: 8 Codex findings (validator RETURN, adversary BREACHED, user-sim REJECTED) plus deputy design findings D1.1, D1.2, answer dispatched 22:04 UTC (session `cse_01LxwP923RKBfVNTWsTUagAQ`); r1 answered at 4522428 (all 10 fixed); r2 (22:37): Codex `changes (6)`, validator RETURN, adversary HELD, user-sim ACCEPTED; answer r2 dispatched 23:10 UTC (session `cse_01LxwP8yPPmPVipCPszcmKqW`; a 23:00 fire carried the stale r1 brief and pushed nothing); r2 answered at ea98cb9 (all 6 fixed); r3: Codex review dispatched 23:29 UTC; r3 (23:58): Codex `changes (3)` (validator RETURN, adversary HELD, user-sim ACCEPTED); answer r3 dispatched 00:01 UTC (session `cse_01LxwP8zfZtC4o6shbfqRvp7`); r3 answered at d0dfddd (all 3 fixed); r4: Codex review dispatched 00:20 UTC; r4 (00:26): Codex `changes (3)` (validator RETURN, adversary HELD, user-sim REJECTED), all on the composer's kept draft; deputy ruling (same finding 4 rounds running) and answer r4 dispatched 00:51 UTC (session `cse_01LxwP92VSjjigxaiXQaSyiJ`); r4 answered at 8177ca5 (by the ruling); r5 (last): Codex review dispatched 01:12 UTC; r5 (01:18): `changes (1)` (validator RETURN, adversary HELD, user-sim ACCEPTED): **round cap reached, held for the human** (Waiting: S35-CAP); then the human's R-B1 before merge; first routine `trig_01XowNDWUGosFjLtAtbGFBwc`)

Story: Alice runs `watane-dev serve` with the agent kernel and the unchecked provider installed,
opens the console, starts a session in a scratch workspace (4.6, no consent steps), types a
message and watches the answer stream in from dev's fake `claude`: text deltas, folded thinking,
finished entries, the status line Working then Idle. She reloads and sees the same transcript,
nothing doubled. One story in two repositories, handed out together: watane S99 is the host half
(H2 `claude-code.start` / `claude-session.say` with the unchecked path and
`allowUnsandboxedCommands: false`, `host-info.generation` as S97 writes it, dev's fake `claude`,
pkg). Interface, named the same in both sketches: the `watane:host@0.1.0` H2 and `host-info`
additions of TECHNICAL 2.1 as watane S97/S99 write them in host 19.
Outcomes:
1. S32 R1 first: a kernel call that fails commits none of its writes on watane's host (a test that
   fails today).
2. The unchecked provider (10) starts a fake `claude` session and hands its stream to the kernel;
   the kernel runs the launch and one `say` (9.3), items through the adapters (9.8, R2) into the
   transcript.
3. The console shows the turn live and after a reload; the status line follows the turn (fixes
   the S32 "felt wrong" Idle line).
4. The stage B cases of 17 that a single turn covers (5, 6, 8 to 12, 58 reattach part) pass on
   watane's host.
Carry list items folded in: R1, R2, S34 RISK 1 (the first `workspace`), the felt-wrong list.
Touches: `kernel/`, `provider/` (see Waiting: its own repository), `wit/`, `mod.toml`, `console/`,
`ui/`, `design/TECHNICAL.md` (2.1, 4.1, 4.6, the 15 row 4.6 names). Fakes nothing once S99
merges; until then it builds against S99's branch. Protects: *core*. `PO arms`. Cloud.

### S37: spike: what can enforce a profile's local-network setting for the shell grant (V27) (DONE 2026-11-23: report read, PR #48 closed unmerged, answer under V27; was in flight, cloud, builder codex, no review round (spike: the deputy reads it, closes the PR unmerged, and puts the answer under V27); Codex task https://chatgpt.com/local/6b03eab5-4b88-83eb-8f84-d9cc7bb8606e, 2026-11-23 ~07:15 UTC)

Question: which mechanisms can enforce a profile setting "local network access: denied (default) / allowed" for
the shell grant, on macOS and Linux? Local network means loopback (127.0.0.0/8, ::1), private ranges (10/8,
172.16/12, 192.168/16, fc00::/7), link-local (169.254/16, fe80::/10) and the metadata address 169.254.169.254.
Candidates: (a) the kernel refuses allowlist entries at profile check (`localhost`, `*.localhost`, IP literals in
those ranges); (b) Claude Code's sandbox settings (what its network proxy and `allowedDomains` can and cannot
refuse, per platform: Seatbelt on macOS, bubblewrap on Linux); (c) a watane host egress proxy in front of the
sandbox that checks the resolved address at connect time; (d) anything else found (OS firewall rules per
process, a resolver that refuses such answers). For each: what it stops and what it doesn't (DNS rebinding,
names that resolve to private addresses, IPv6 and IPv4-mapped forms, the metadata address, a command that
bypasses the proxy), its cost, and where it lives (kernel, host, Claude config). Probe on the Mac and in a Linux
container where a claim can be tested; mark each claim probed or from docs.
Outcome: `design/SPIKE-local-network.md`, a write-up with a recommendation and the probe scripts; no product
code. The human decides V27 from it. Touches: `design/SPIKE-local-network.md`, `probes/` only. Builder codex,
reviewer claude. Cloud.

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


### Upkeep

- **(S139 r1 polish)** P1.1 to P1.7 of https://github.com/civic-tech-pioneer/agentic-harness/pull/62 (review r1): tests for `cannotInterrupt` on the taken-row retry and the control-ack fold; the older `workspace:/abs` consent form; `stage_c.mjs:316` asserts `q.what`; P1.5 spec drift in `design/`; the 240-char preview as a slice; a folder with a newline.

- **(from S131 F1.8)** a stage D load run lost 1 item and 4 tool calls of one OpenAI session at the page (1 of 3 runs, undiagnosed; maybe its last step cut by Stop). Diagnose before the stage D Budget row can close.
- **(after S127)** `test:host:e`: stage E's outcomes on `watane-dev serve` with dev's scripted judge (watane S129 has merged).
- **(S127 polish)** P4.1 `kernel/tests/it/s127_judge.rs:333`: assert that the replacement launch carries the original follow-up.
- **(from S131 F1.2)** the console's consent activity does not name the folder the app asks for (`console/src`); S131's `stage_c.mjs` keeps it as a named known gap.
- **(guard, the human's (b) on S40 F6.1)** a timer firing of an OpenAI session whose owner mod was disabled during a timed pause may send one more model request under the kernel's origin. Fix: read the host-resolved origin of a timer firing through the production adapter (watane reports the caller of a timer callback as `host` and keeps the resolved origin apart), stop only foreign firings, and test it through the real adapter: an enabled owner's pause resumes by itself, a disabled owner's stops with no work.

- **(guard, second sighting) `installed-agent` CI flake:** `console/test/host/installed.mjs:216` 'Timed out waiting for Claude Code settings' after AC1 passes; on S40 at 890caad and 10ed0a3, green on rerun. Find the race or give the wait a bound that measures it.

- **Case 22's idempotent half (from S40 r2):** after a restart, reissue an idempotent job under its key, with the toolset reopened under the session owner's origin and the job sent only once the handle is usable.

- **S40 on `watane-dev serve` (after watane S114 merges):** turn the SKIP lines of `test:host:d` into checks, run S40 outcomes 2 to 4 on a preview with a scripted OpenAI stream, and add the stage D load test.

- **From S39's review (2026-11-24), console, earlier slices:** after Stop the transcript shows 'An event this console does not know' (claude's `control_response` to the interrupt); a stopped turn's footer reads 'Completed'; with no Claude Code binary chosen the error does not point to Settings → Claude Code; S39 polish P1.2 to P1.5 (exit 2 when binaries are missing, full grant lists in the consent check, a commit id in `provider/BOUNDARIES.md`, `node -e` instead of `python3`). For watane: the consent window's Call line reads as an interface name, not words.

- **From watane S103 r1 (2026-11-24):** Stop before `initialized` (while a folder consent waits) should cancel the stream, not send `interrupt`: an interrupt leaves the app's question open, and a later yes answers into an idle session. The host's cancel path drops the question and prevents the launch.

- **Polish from S38 (merged 2026-11-24):** P1.2 (assistant line without `session_id` in `kernel/tests/it/s38.rs`), P2.2 (TECHNICAL 18.5 words 'Workspace access was refused for …', 'Connection to Claude Code ended'), P2.3 (picked folders get no file write key at all: say so in 9.6 and 14), P2.5 (`start-in-folder` shares `start`'s key kind; its own refusals in 2.3/4.6), P2.6 ('Open the question' does nothing; show the folder in the wait, per `States-2`); stale half-streamed message after `gone` until reload (stage B carry).


- **S36 accepted limits (r7, the human merged it as it stood, 2026-11-24):** F7.1 and F7.2 are the S35 Send limit (TECHNICAL 18.6): an Edit send refused after leaving the session keeps only the original text, and a reload while an accepted take's reply is in flight can offer it once more. F7.3: the load check's 2x spacing failed once on timer jitter (40.585 ms delta gap); widen to 3x (60/600 ms) if it fails again. Also P7.1: exact just-over-limit fixtures in `console/test/host/load_check.test.ts`.

- **Polish from S36's review r1 (2026-11-23):** P1.1 `kernel/tests/it/s36_firing.rs:49, :96` (assert the write delta,
  the retry delay and a later successful call; the ceiling fixture at the actual ceiling); P1.2
  `console/src/replaced_S36_3.test.tsx` (exact replay cursor, notice timing, enabled controls around takeover, session
  switch on one mounted app); P1.3 the rest of `console/test/host/stage_b.mjs` browser assertions (card input and expiry,
  scoped grant and removal, Deny with reason, queue and Stop transitions).
- **Felt wrong in S36's user-sim:** queue rows show a whole 255 KiB message; Edit and Undo re-queue at the end; Stop and
  send is offered on a queued row while the session reads Not running.
- **Polish, low priority (the human, 2026-11-23):** a sent message's text stays recoverable until the kernel
  has accepted it, even across a reload (S35's composer ruling dropped this; Codex F1.4 to F4.3). Pick it up only
  when cheap, e.g. alongside other composer work.

## Done

- **S131** (upkeep) the host tests check what they skip: stage C runs on a participant home with a fake Confirm and its budgets are measured; the expiry check requires deny by host for `expired` at or after the deadline; S128/S36/S38 test polish done; the console's folder name in consent is a named KNOWN GAP (Upkeep). PR #58, squash-merged 2026-11-26 ~08:58 UTC at 05cda43 by Deputy Paper, no bypass; builder claude, reviewer codex, 3 rounds (8/2, 1/1, 0/0); ruled by Deputy Rock at r1 (F1.1/P1.1 sketch amended, F1.2 a known gap, F1.7/F1.8 narrowed); boundary HELD; Codex could not run the host commands (no watane-dev), judged from the builder's red-first runs; ~2.5 h in flight. Sketch in factory/DONE.md.
- **S132** (design) stage F, delegation and counters, made one consistent contract: AGT-J5 runs on an OpenAI root with two sibling children (depth 1); Claude keeps its launch-time subagents; `task` returns one terminal result, a child shows as an indexed `child` entry; counters per turn; §14 child admission spelled out (PR #59, rebase-merged 2026-11-26 ~08:30 UTC at 95ec3bc7 by Deputy Rock, no bypass; builder codex, reviewer claude, 1 round 0/12; ~25 min in flight). **Door C, reported to the human for veto:** 9 choices in PR #59's body; two narrow a FUNCTIONAL promise: AGT-F-36 counters per turn instead of per session, and Claude refuses `max-tokens` and `max-tool-errors` (only its native `max-turns`); both fail closed. Polish P1.1 to P1.12 ride with S134's brief. Sketch in factory/DONE.md.
- **S127** (stage E) Alice's Claude session runs through the checked provider with Jev as its judge: the console shows `checking` before each tool call, a safe call goes on, an unsafe or unsure one is denied with a reason she can read, and repeated denials trip the breaker, which says why (PR #56, rebase-merged 2026-11-26 07:57 UTC at 8e92d1b4 by Deputy Rock, no bypass; builder claude, reviewer codex, 4 rounds 4/2, 2/1, 1/1, 0/1; ~13 h in flight). Cut at r2 (deputy-7): F2.1 a known limit in TECHNICAL 10 (a verdict's reason can be lost under backpressure at teardown); r3's F3.1 fixed on existing state (TECHNICAL 9.4). Boundary HELD, outcomes ACCEPTED on fakes. Codex's cyber filter refused the adversary twice in r1 (worded as a boundary check since). Sketch in factory/DONE.md.
- **S128** (upkeep) the `installed-agent` flake is a fixed race (the test now waits for the install to finish routing), `kit/s7_fix1` counts row reads instead of timing them, S40 runs on `watane-dev serve`, and `test:load:d` measures the stage D budgets at the page with a Budget record row (PR #57, rebase-merged 2026-11-25 at a19a90f3 by deputy-6, no bypass; builder codex, reviewer claude, 2 rounds (1/9, 0/6); ~1.3 h in flight). Cut: F6.1 and case 22 stay known limits (watane timer-origin contract held, watane PLAN Unsettled). P1.2 accepted (~1 s first-request delay). Polish in Upkeep. Sketch in factory/DONE.md.
- **S40** Alice runs a profile on her ChatGPT plan in the console: the kernel refuses what OpenAI cannot honour and names each field, an OpenAI session streams the same events as Claude, and a usage limit pauses it until the reset and resumes by itself (PR #54, squash-merged 2026-11-25 ~18:45 UTC at b494cc34 by deputy-6, no bypass; builder claude, reviewer codex, 7 rounds; r7 reverted to the reviewed 553c95f plus one 9.5 line, checked mechanically; ~24 h in flight). F6.1 a known limit (the human's (b); Upkeep guard). CI: `installed-agent` red twice on the doc-only head with the known 'Timed out waiting for Claude Code settings' flake (not required; green on 553c95f). Sketch in factory/DONE.md.
- **S39** the agent mod installs and runs in the watane app (PR #53, merged 2026-11-24 ~20:00 UTC without the admin bypass at 893f14a3, the first merge under the changed ruleset; builder codex, reviewer claude, 2 rounds 0/5, 0/0; ~7.5 h in flight, of which ~3 h waiting for watane S103 under the new 'dependent slices do not wait' rule). One command builds and publishes the agent mod and the Claude Code provider; one consent installs both from a link in the app; the launcher opens the console, which streams, asks, stops and keeps its transcript across a restart (CI `installed-agent` on watane main). Outcome 4, the human's real Claude login on a Mac: commands in the PR body. Risks A1.1, A1.2 accepted; polish under Upkeep.

- **S38** stage C consumer half (PR #52, merged 2026-11-24 ~09:45 UTC with `--admin` on the human's word, main 499c254; builder codex, reviewer claude, 4 rounds 2/8, 1/7, 1/2, 0/0; ~1.9 h in flight). Alice starts an agent session in a folder she picks; the console shows the consent wait, names a refusal and a forbidden folder, and a withdrawal ends the session with its transcript kept (shown as 'Connection to Claude Code ended', R1.1 accepted); tested against a double of watane host 19.7 until S103 merges. Decisions: Q38 left to the human (the spec cites 19.7), R1.1, R2.2 accepted. Friction: update+run of a routine fired a stale brief.

- **S32** stage A end to end (PR #41, merged 2026-11-22 ~14:10 UTC by the PO with `--admin`; 2026-11-21 08:59-10:47,
  then 2026-11-22 12:06-13:42 on watane's real host, and a 15-min fix after the PO's comparison). Alice runs
  `watane-dev serve` (watane 4f105a9) with the agent kernel and watches a real kernel stream's numbered items in
  the console, live, after a reload, a host restart and `kill -9`, nothing doubled; a refused send keeps her text
  and says why (18.6 "Send"); Deny… opens its reason form in view (18.6 "Asks"). `npm run test:host` (by hand):
  7 pass, 2 honest skips (case 7 and 36's defer half need a provider stream: stage B). User-sim ACCEPTED (real-host
  run 2); adversary HELD (one earlier BREACH fixed). World stays staged in `kernel/wasm/wit/` until a watane release
  with host 19 (CI installs v0.1.0). Gate 55 s; kernel 459 tests 3.1 s; console 496 tests; `test:slow` 28 s.
  Friction: no IPv6 loopback in the cloud (patched host for the run); SessionStart installs watane-dev v0.1.0 (12 min
  builds by hand); the rebase dropped a DELIVERED commit. Felt wrong, to S35/S36: probe composer invites a message,
  raw error wording in the alert, reload drops typed text, blank flash after reload, new port per `serve` restart,
  `serve --stop` deletes preview data, "(18.8)" shown to the person.
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
- **S134** stage F: an OpenAI root delegates to two named sibling children (AGT-J5), each a `child` link in the console with its own transcript; a child that would widen its root (calls `task`, a name outside `delegates`, a wider rule) is refused with no session; stopping, cancelling or deleting the root stops its children, live and stored; every OpenAI turn stops at `max-turns`, `max-tool-errors` or `max-tokens` and names it; Claude reports its native `max-turns` and refuses the other two. PR #60, squash-merged 2026-11-26 11:16 UTC at f8c0fe0d by Deputy Scissors, no bypass; builder claude, reviewer codex, 4 rounds (9/1, 3/1, 1/1, 0/0); ~2.7 h in flight. Adversary BREACHED in r1 (F1.1 to F1.8), every breach fixed by r4; outcome check on scripted Responses, `watane-dev` absent (page-level budgets a known limit). Accepted: F1.5 (`persist: false` not built for OpenAI). F1.4 on the builder's reading. F3.1 ruled (no path drops `Work.stopped`). CI: one gate timeout at 15 min on 5535d7f, green on rerun (Friction). Sketch in factory/DONE.md.
- **S139** (upkeep) the console names the folder a consent waits for (stage C checks it), a stopped turn reads stopped (no unknown-event row, no "Completed" footer), a missing Claude Code binary points to Settings → Claude Code, S36 polish done, and the `installed-agent` wait is bounded and measured (both red runs predated S128's guard). PR #62, rebase-merged 2026-11-26 ~12:20 UTC at 41353b37 by Deputy Scissors, no bypass; builder codex, reviewer claude, 1 round (0/7); ~1 h in flight. Polish P1.1 to P1.7 in Upkeep. Sketch in factory/DONE.md.

## Deputy decisions

- 2026-11-26 (Deputy Scissors, rung 2, Door C, for the human's veto) S138 F1.1: a console (page-owned) session's timer firing carries its owner's authority when it reads the kernel's own installed identity, its origin client (TECHNICAL 8); every other owner stays strict, so F6.1 stays closed. Codex and Claude agreed blind. Record: factory/decisions/D001-page-timer-authority.md. Undo: one comparison and two sentences in 9.5.

- 2026-11-26 (Deputy Scissors) S134 F1.5 accepted as a known limit: OpenAI `persist: false` is not built for any session (root and child alike); no VISION or FUNCTIONAL clause promises it; recorded in TECHNICAL 11. Undo: an upkeep slice that builds it. S134 F1.4 settled on the builder's reading (toolsets carry no adopted revision; host consent decides).

- 2026-11-26 (deputy-7) S127 cut at r2: F2.1 accepted (the denial and cancellation stay immediate and every §14 row HELD; only the reason text can be lost when the kernel-facing stream is full at a launch deadline or Claude ends before ready; two unusual events together). Clause: the stall rule (findings in code the last answer added). undo: an upkeep item, drain the outbox before close.

- 2026-11-25 (deputy-6, S127, **Door C, for the human's veto**) judge wire shape: Jev (the human's choice) speaks TypeSafe
  System One, and no operator may sit between (VISION), so the checked provider's judge call is `POST
  <origin>/v1/systemone` with `noul` questions in one request; it maps probabilities to `safe | unsafe | unsure` by
  thresholds on its own page (default 0.7/0.3, the docs' starting values) and writes the reasons itself. Origin: entered
  in the app only (TECHNICAL 5 wins over 10's "judge URL" setting). `[http]`: `api.typesafe.ai`; "a judge they chose"
  (VISION V6) holds by choosing or deriving a provider, so no VISION edit. Clauses: Human answers 2026-11-25 (2), (4);
  TECHNICAL 5, 10; dev TECHNICAL 17 item 17 (the provider fixes its shape and scripts `Raw`). Against: TECHNICAL 10's
  `{safe|unsafe|unsure, reasons}` judge answer, fixed in S127. Undo: a second request shape in the provider (an adapter
  per judge API), before any participant relies on it.

- 2026-11-25 (deputy-4, S40 r4) F4.3 accepted as a known limit: a message finished in the window between its transcript write and `response.completed` when the host restarts is shown but missing from the model's history; Alice repeats it. Clause: rulings narrow; same kind as the accepted no-auto-continue limit (S40-CUT). Undo: write the finished message to request history with the transcript, a later slice.

- 2026-11-24 (S40 r2) after a host restart every open job of an OpenAI session in `tools` is aborted, idempotent ones included; the kernel reopens no toolset and reissues no job (TECHNICAL 9.5 and the restart table say so; case 22's idempotent half not yet built). Clause: rulings narrow (stall by area: two blocking findings in code r1's answer added, one a breach). Undo: reissue under the session owner's origin once the handle is usable, a later slice.

- 2026-11-24 (S39 r1) accepted: A1.1 a provider release built other than by `publish-test-mods.sh` trusts `local/agent` (needs a hand-published provider and a local mod with the slug `agent`; the four-line pin test closes the script path); A1.2 the script checks the channel's publisher only after signing the provider (operator-only). Whether a release build must refuse the dev default is a TECHNICAL 2.2/10 question for later.

- 2026-11-24 (S38 r2) F2.1 replaces the r1 ruling on F1.2: TECHNICAL 8's two 'Kept for' cells cite watane host 19.7 and name Q38 as open, restating no key; the doubles use 19.7's `workspace:<folder>` as the main case. Clause: rulings narrow to the spec's literal words. Undo: one cell each.
- 2026-11-24 (S38 r2) accepted: R2.2 (a late `consent-pending` needs a forged wrapped line and a disagreeing host check: two unusual events); Q2.1 (Stop during a consent wait leaves the app's question open; watane 19.3 does not say what `interrupt` does before `initialize`) and Q2.2 (forbidden path refused at call or at apply) go to watane S103's review.
- (superseded by S38 r2) 2026-11-24 (S38 r1) F1.2: watane Q38 (consent key with `via`) is the human's and open; S38 keeps option (b) in TECHNICAL 8 and the doubles, marked pending, as the reversible option ((b) to (a) keeps every consent valid; host 18 Q38). Clause: the deliberation ladder's 'build the option that keeps the others open'. Undo: drop `via` from two rows and the doubles.
- 2026-11-24 (S38 r1) R1.1 accepted: a withdrawn consent reads as 'Connection to Claude Code ended'; host 19.7/19.2 end it with a plain `gone`, and AGT-J1 / AGT-F-29 ask only that the stream end. A distinct host ending would be a watane interface change (Door C), not planned. Undo: a `consent-withdrawn(what)` ending in watane and one console line.

- 2026-11-23 (deputy-2, S36 F3.1, the same finding a third time) Take back first, replacing the F2.1 rule below, which let the original and its replacement both run: Edit, Stop and send, and remove call `unqueue(n)` first (the kernel's atomic take, TECHNICAL 9.3); `unknown-input` leaves everything alone; a later refused `say` or `interrupt` puts the text in the composer (18.6 Send). Clauses: AGT-F-41 "take one out before a turn takes it", AGT-F-50 "take a queued input back to edit it"; none against. Cost: a refused re-send loses its queue place, and its text lives in the composer under that rule's reload limit. Not deliberated on the PR: the clause decides it (Kind rule, Door none). undo: rewrite 18.6 Queue and the console's Dock again. PR #50 comment 7114381778.
- (withdrawn by the line above) 2026-11-23 (S36 F2.1, r1's F1.5 back a second time) The console's queue keeps an item until its transfer is accepted: Edit, Undo and Stop and send never remove a queued item before the replacement say (or interrupt and say) succeeds, so a refusal changes nothing and needs no recovery state; a newer draft is left alone.
- 2026-11-23 (S35, the composer's kept draft, F1.4 to F4.3): no spec asks for a draft that survives a reload while its send is pending; each fix opened the next race. Rule: the composer keeps unsent typing per session across a reload; Send removes the text from composer and kept draft at once; a refusal while the sending composer is mounted puts it back (newer typing wins); a refusal after a reload or unmount is lost from the composer (known limit in TECHNICAL 18). PR #43 comment 7088686568.

- 2026-11-23 (watane S100, for S36): the host gives the resolved path (9.6's open point) as a top-level `"path"` on a file-tool `ask`/`check` item (absolute, symlinks resolved; `input` as `claude` sent it; absent when unresolvable or refused). S36 writes it into 4.5's item table and has the kernel offer a file key only from `path`. A request before `system/init` passed is denied by the host and pushes nothing.

- 2026-11-22 (S35): stage B cases 9 (asks), 10 (hooks) and 11 `compact` move to S36; case 12's depth-0 half (removing Task) moves to stage F (9.7, delegation). S35's continuation named them as not covered; none is a single-turn case S35 needs.

- 2026-11-22 (deputy): R-B2 settled without the human: `codex_env = "watane"` is in `factory/config.toml` since the factory sync (PR #45), and the `watane` Codex environment holds this repository. Removed from Waiting.

- 2026-11-22 S35's user-sim round-2 rejection (an attach mid-message shows Working, not Writing, until the
  entry arrives) is accepted as a polish limit (9.4): setting `synced.writing` costs a state write per delta-only
  call, which case 56 and P09 rule out. Its red test stays `#[ignore]` with that reason.
- 2026-11-22 R-B3 (S35): the provider compares its caller with `local/agent` while the kernel is unpublished;
  the publisher's name replaces it when the kernel is published (one 2.2 line).
- 2026-11-22 (the human's answer) Auto-merge is retired: the `codex-push` ruleset (27841001: only
  `slice/*` without bypass, for the Codex push App) blocks it, so the PO merges each green PR with
  `gh pr merge <n> --admin --rebase`. Foremen open their PR and do not arm it.
- 2026-11-21 Stage B is two end-to-end slices (S35 one turn, S36 steering, restart, budgets), each with
  a watane half (S99, S100); S35 builds the unchecked provider in `provider/` as a self-contained mod
  until R-B1 is answered.
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

- **2026-11-25, the human (to deputy-5): no test run in a cloud (CI or a cloud team) calls an external provider** (OpenAI, Jev or another judge, GitHub's API, any real service). Unit and integration tests use fakes: dev's scripted Responses server and scripted judge, fake `claude`, fake-watane. A check against a real provider is a deputy probe on the Mac (V7 on Jev). Checked 2026-11-25: no workflow in watane or agentic-harness passes a provider secret to a test.

- **2026-11-25, the human (to deputy-5):** (1) **S40-F6.1: (a)**, one more answer round with the narrow rule; if it does not succeed, (b): accept it as a known limit and fix it in Upkeep. (2) **V7: the judge is Jev.** Its API key is in the environment variable `TYPESAFE_API_KEY` on the Mac (never read it into a context or a log); spending limits are set at Jev, so the factory adds none. (3) **N5: (a) for now**, the Claude grant with shell and web; (b) may follow later. (4) **The Jev key stays on the Mac only**: no GitHub or cloud secret. V7 and every check against the real judge run on the Mac, by the deputy, as a read-only probe with the key taken from the environment (never printed); cloud teams build and test against dev's scripted judge.

- 2026-11-24 watane host 18 **Q38: (b)**, the human: a consent is kept per (origin client, provider, folder or account). TECHNICAL 8 cites watane host 19.7, which S103 writes to match; no change here.

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

- **2026-11-23, the human, on S35-CAP** (PR #43 at the round cap): "Yes, one more round". Option (a): one more
  answer round for F5.1 by the builder (Claude), then a sixth review by Codex. The merge needed R-B1 too (answered below).

- **2026-11-23, the human, on R-B1:** (b). "b is fine with me ... I favor the pragmatic approach here and keep it
  together until there's real evidence that they ought to be apart." The provider stays as `provider/` in this
  repository; TECHNICAL 17 (stage B) gets one boundary list and drops "its own repository". S35's r5 answer
  round makes that change, so review r6 covers it. S35 merges on an approving r6 and green CI.

- **2026-11-23, the human, on the S35 composer ruling:** "I agree with the reviewers that it ought to work that
  way. But at the same time, if it is a lot of hassle to get it to work, it's not the kind of feature that needs
  much of our attention, especially because it's fairly rare." The ruling stands for S35; an Upkeep line (polish,
  low priority) keeps the recoverable sent text.
- **2026-11-23, the human, on watane S100's top-level resolved `path`** (Deputy decisions, for S36): "Sounds good.
  Approved." No longer a pending veto.

- **2026-11-23, the human, on the R-B1 vision edit:** "That should not have lived in vision, because it's way too
  specific, And only belongs to the technical documentation, as an option. So good to remove it there." VISION drops
  the repository rule entirely; TECHNICAL keeps a provider's own repository as an option for later.
- **2026-11-23, the human, on R-B4** (the built-in `scratch` profile): "Sounds good to me." Option (a), as S35 built it.
- **2026-11-23, the human, on V27:** "In principle, what I would like is that the profile at least has the option to
  block or allow access to localhost via whatever mechanism that may be. I could imagine that it's complex though, so
  I'm willing to take a best effort and a report on exactly what can be done and what are the limitations. I'll take
  the decision from there." Read as: a profile setting for local network access (loopback, private and link-local
  ranges), denied by default. Spike S37 reports the mechanisms; V27 is decided after it.

- **2026-11-23, the human, on N5:** "I'm leaning towards B as Well. But let's revisit this only once we have actually
  started at E stage E, that is." A leaning, not a decision: (b), a `codex app-server` backend in the host, keeps a
  second vendor for cross-checking. Revisit when stage E starts.

- **2026-11-23, the human, on S35-MERGE:** "Merge": merge PR #43 once CI on 8a9e512 is green, with no seventh review;
  then hand out S36. The user confirmed it in the deputy's session ("Yes, merge S35"); merged at 042b0dd.

- **2026-11-23, the human, with the monitor, on reviews** (watane-mods PR #5): the review lead triages blocking versus
  polish (polish never holds a merge; it goes under Upkeep); the lead may propose to accept a blocking finding (security
  only if cheap to undo and outside the vision's promises, else the human); no fixed round cap, each review ends with the
  trend and the lead's call, and from round 4 the deputy writes why one more round is worth it; the deputy rules when a
  finding comes back twice. "find a balanced perspective on building out many features of the product versus building
  few features perfect".

## Waiting on the human

- **N5 (2026-11-26, Deputy Rock): stage E has merged, so N5 is due** (your 2026-11-23 words: revisit once stage E starts). The factory's implementer needs a shell. (a) the Claude grant with shell and web: nothing new, recommended by TECHNICAL 17; (b) a `codex app-server` backend in the host, a third native process kind, which keeps a second vendor for cross-checking (your leaning). Holds only stage F's factory profiles; S132 (delegation and counters) goes on without it.

- **S40-CUT (2026-11-24): restart recovery of OpenAI sessions has stalled three rounds** (each fix opened a new path where the global restart pass acts under the wrong origin). Proposed cut, by J11's own words for Claude ("Notes shows the session as not running, with everything up to the last finished message; she sends 'continue'"): after a host restart, an OpenAI session that was mid-turn shows not running with its transcript up to the last finished message; its open and pending calls are aborted; the restart pass sends nothing, opens nothing and asks nothing for it; the owner's next message continues it under her own origin. Keeps: everything else in S40 (profiles, refusals, streaming, tools, pause until reset, the meter). Becomes a known limit: an OpenAI turn does not continue by itself after a restart (TECHNICAL 9.5's restart row and case 22's reissue, Upkeep). Undo: an owner-scoped recovery in a later slice. Recommended: take the cut. DECIDED 2026-11-25 by the deputy under the new cut-early rule (mboogerd/claude-plugin#3, asked by the human): cut taken; answer r3 with the cut to Claude, routine `trig_014rQUrVhYxDfSZTELMCedUR`, fires 06:37 UTC. The human may veto; the undo is an owner-scoped recovery slice.

- **S35-MERGE: PR #43 after the extra round (r6: `changes (2)`, adversary HELD, user-sim ACCEPTED, F5.1 closed, CI gate
  green).** F6.1 (PLAN.md on the branch still shows R-B1 open) is done on main; the slice does not touch PLAN.md. F6.2 is
  a one-line citation in `provider/BOUNDARIES.md` (cite 3629c5c, not 866d325); the builder fixes it now (routine, 08:45
  UTC). Options: (a) the deputy checks F6.2 with Codex's own command (`git show <sha>:design/TECHNICAL.md | grep -Fx` each
  copied row, exit 0) and green CI, then merges with no seventh review (rec: a citation, checked mechanically); (b) one
  more Codex review round first. Undo: (a) a revert of the merge.

- **V27: the S37 spike report is in; your decision** (the human wants a profile setting for local network access, denied
  by default). Report: `design/SPIKE-local-network.md` at 5f7392a on `slice/S37` (PR #48, closed unmerged). Its findings:
  (A) the kernel refusing allowlist entries (`localhost`, `*.localhost`, local IP literals incl. IPv4-mapped) is cheap
  and probed, but misses DNS names that resolve local, rebinding, and direct dials; (B) Claude's sandbox: on macOS
  `["localhost"]` reached loopback through the sandbox proxy (S1, probed); Anthropic's sandbox-runtime has a
  resolved-address guard at dial time (loopback, link-local, metadata by default; private and ULA ranges need extra
  config) but whether the pinned Claude build exposes it is unverified; (C) a watane host egress broker that resolves
  once and dials only a checked address covers rebinding, mixed answers, IPv6 and the metadata address (probed in a
  Linux fixture), at the cost of a host component that must own both proxy paths; (D) OS egress rules (Linux cgroup or
  nftables, macOS Network Extension) or a filtering resolver: costlier, admin-level, or partial. Recommendation: keep
  the domain allowlist, add (A) as a first line, and call the setting enforced only with a dial-time address check (B if
  the Claude build exposes it, else C); if neither is available, fail closed for a profile that promises local denial,
  or label it best effort with the gaps named. Decide in stage C, with the probes in CI on Linux and macOS.
  Was: V27 (decide after the S37 spike report; the human wants a profile setting for local network access, denied by default):
  the shell grant's network allowlist reaches loopback through Claude's sandbox proxy**
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

- (2026-11-26, Deputy Scissors, S134) CI `gate` was cancelled at its 15-min limit on 5535d7f (runs before took ~7 min); green on rerun. Watch for a second sighting.

- 2026-11-24 (S38 r2) `RemoteTrigger` update then run fired the routine's old brief in a new session (second time after S35 r2): for a later round, create a new routine instead of update+run. (deputy-3)

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

- S131 (2026-11-26, `Linux 6.18.44-fc-v880 x86_64`, 4 cpus, a cloud VM; load average 2.6 to 4.3 from the runs themselves): console
  `npm test` 635 tests 38 s, `test:slow` 21 s; kernel `cargo test` 621 + 6 + 1 in 5 s (warm); gate 79 s. By hand: `test:host`
  25 pass, 7 skip, 212 s; `test:host:c` 6 pass, 36 s (warm); `test:host:d` 9 pass.
  - **Stage C budgets** (TECHNICAL 17 row C, the kernel and host part, on dev's fake `claude`; the real `claude` run is the
    human's): `npm run test:load:c`, the stage B load (8 sessions, 4 `page` in Chromium and 4 `local/loadclient`, each 50 deltas
    and 5 entries of 4 KiB a second) on a participant home of watane c52d2aa (S129) whose app is the fake `Confirm` of
    `console/test/host/home`, each session started with `start-in-folder` in a folder new to it (8 consents asked and granted per
    run). r1 (F1.7), the home built with `console/test/host/home/commit-clock.patch`, load average 1.4 to 2.7: item delay p95
    5.0, 4.9, 6.0 ms (budget 100; max 122, 114, 149 ms); Stop to `idle {interrupted}` at the page p95 16.8, 12.0, 11.0 ms (budget
    2 000); writer busy share over the common window 19.9, 20.8, 21.4% (about 3 130 commits; budget 50). State bytes per entry:
    as B (the crate's `s36_load`). The first three passes (before r1, stock home: writer not measured): item p95 6.8, 4.5, 5.0 ms,
    Stop p95 11.9, 13.9, 13.8 ms. Stage B on the same machine and host, same day: item p95 12.0 ms, Stop 19.5 ms.
  - **Stage D budgets, remeasured** (S128 P2.5): `npm run test:load:d` on watane 548b4e1 with `stage-d-tools/watane.patch` (which
    builds `watane-host` and `watane-dev` at opt-level 2 in the dev profile; each Claude delta carries a marker ~37 bytes longer
    than the stage B load's). Five runs: item p95 17.5, 17.2, 18.3, 23.2, 24.1 ms (budget 100); Stop p95 44.9, 44.0, 38.4, 36.3,
    46.8 ms (budget 1 000); writer busy 28.9, 27.7, 29.0, 30.5, 30.8% (budget 60); cadence 8/8 OpenAI sessions at one step a second
    (runs 4 and 5 print it; 1 to 3 had no cadence fault). Runs 1 and 3 pass; runs 2, 4 and 5 fail only the source check: one fake
    `claude` wrote no delta for 40.9 to 46.4 ms, over the 2x spacing of 40 ms (S36 F7.3, the second sighting: the deputy's
    "widen to 3x if it fails again"; the rule is also TECHNICAL 16's, which this slice may not touch).
    r1 (F1.8, the deputy's ruling: the rule kept at 2x, P2.1's second-step cadence window in force for all eight runs), three
    more runs on a quieter start (load 1.6 to 2.1): item p95 19.6, 18.9, 20.0 ms; Stop p95 58.9, 34.6, 40.0 ms; writer busy 29.0,
    28.7, 29.3%; cadence 8/8 each. Runs 2 and 3 pass; run 1 fails on completeness, not spacing: one OpenAI session (`s9`) had one
    source item and 4 of its 32 tool calls missing at the page within the window (undiagnosed: possibly its last step cut by the
    Stop at the window's end; reported in PR #58). **The stage D Budget row stays open**: 4 of 8 runs pass, every budget held in
    all 8.

- S4: console `npm test` 167 tests 6.2 s; `test:slow` 7.5 s; gate with kernel 24 s (2026-11-20).

- S3: kernel `cargo test` 57 tests in 0.05 s; CI gate job 17 s (2026-11-20, macos-15).
