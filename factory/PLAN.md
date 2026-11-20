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

### S4: the console on a scripted session (in flight (cloud), routine trig_014DLovebAdvPQZZ2vujFUcB)

Story: Alice opens the console (`console/`, served by its dev server) on a scripted fake kernel
and sees a session as the first target describes (TECHNICAL 18): her message, thinking folded,
reads folded to "Explored · n reads", an edit as a card with its diff, an open ask with its bar
and "Needs you (1)", the status line, and after a reload the same transcript with nothing doubled.
Outcomes:
1. `npm test` in `console/` passes and covers each row of 18.3 from a scripted ACP v1 feed with
   `_meta.watane` items.
2. Reload mid-session: the console attaches after its last index; replayed items before `synced`
   double nothing (18.3).
3. `npm run build` writes `ui/` with no inline script, no `eval`, no outside URL (18.2); a test
   greps the output for these.
4. The layout values of 18.4 hold at 1280 px and under 760 px (sidebar hidden).
Touches: `console/` (incl. `console/kit/`), `ui/`. Not `kernel/`, not `factory/`, not `design/`.
Fakes: the kernel: a scripted feed in `console/` behind the same shape as `attach`/`read`
(TECHNICAL 2.3, 9.4). Leaves out: profiles and offer pages, the real host.
Protects: *core*. The PO compares it with the frames before acceptance.

### S6: the review's should-dos P07 to P12, and S3's spec debts (in flight (cloud), routine trig_014PPn8yaasuqjGjGycvXqCg)

Story: a foreman of stage B or D reads TECHNICAL and finds the remaining bounds of the performance
review, and the rules S3 decided in code, written where they belong.
Outcomes:
1. P07 (active history after compaction), P08 (chunk rewrite cost), P09 (stream-to-session key),
   P10 (timer budget), P11 (host queue memory) and P12 (kernel quota) are rules in the sections
   that own them, each with a numbered case in 16, as the record states or a better fix named.
2. TECHNICAL 1 and 14 name the real kernel files (`kernel/src/profile.rs`, `overlay.rs`,
   `refuse.rs`, `check.rs`, `error.rs`), so the adversary rule matches them.
3. S3's code decisions are spec text (4.1, 4.2, 9.2): an absent `tools` and `[]` both mean no
   built-in tools; per-session fields refused in a profile; `ask-expiry` in seconds; the overlay
   keys and limits; strict skill frontmatter (`name`, `description`, ASCII); the Bash allow-rule
   grammar (shell-safe characters, no path or `~` first word, `:*` only trailing, bare `Bash`
   refused) and the package-runner table (npm, pnpm, yarn, uv, devbox: fixed second words, no
   option word, no `*`), with `yarn ls` dropped from yarn's list (it runs a script named `ls`).
   Where code and the new text differ, the text names the code change for a later kernel slice.
Touches: `design/TECHNICAL.md`, `design/FUNCTIONAL.md`. Not the record, not `kernel/`.
Protects: *core* (*guard* for 2 and 3).

### S8: the kernel's adapters: Claude and Responses lines to ACP v1 (in flight (cloud), routine trig_017iKEQyBp4bB3RMVZnPDTsf)

Story: a kernel maker runs `cargo test` in `kernel/` and sees each row of 9.8 turn a recorded
Claude stream-json line or Responses event into the ACP v1 update and transcript entry it names.
Outcomes:
1. Golden fixtures under `kernel/tests/fixtures/adapters/`: one per row of 9.8, input lines and
   the expected updates and entries as JSON; a test runs them all.
2. An unknown or unparsable Claude line becomes an `unknown` entry and never an error (9.8).
3. `TodoWrite` gives a `Plan` and a `tool` entry; `result` gives usage and `idle {completed}` only
   when `claude` is idle; `compact_boundary` gives a `compaction` entry.
4. The adapters are pure functions (no state I/O), so 9.4 and the console can reuse them.
Touches: `kernel/` only (a new adapter module, fixtures, tests). Not `design/` (S6 owns it): a
spec question is a `BLOCKED`. Libraries per 9.8 and 13 (`claude-codes` types only,
`agent-client-protocol-schema` v1) if they build; else serde types of the same shape, named in
the delivery.
Fakes: nothing; no host, no WIT. Leaves out: entry storage and paging (9.4), counters (9.7).
Protects: *core*.

## Sketched

### S7: the console stays fast on a long session (cloud, after S4 merges)

Story: Alice opens a session of 5 000 entries with a fast stream running and scrolls, types and
presses Stop without lag (P14).
Outcomes: the transcript is virtualized; at most one render per animation frame; code is
highlighted once its block is complete and visible; a large-transcript fixture with limits for
input, Stop and scroll latency runs in `npm test`.
Touches: `console/`, `ui/`. Protects: *core*.

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

- **Where the Bash allow-rule boundary is** (S3, security boundary). The kernel refuses allow rules
  from a list of interpreters and runners; a list cannot be complete (`make`, `cargo run`,
  `git -c` still pass). Options: (a) the spec says Claude's sandbox (N1, the shell grant) is the
  only boundary for Bash, and the refuse list is a best-effort guard; (b) Bash allow rules become a
  full allow list of named commands (safer, makers write each command, more friction).
  Rec: (a), since N1 already requires the sandbox with every shell grant. Undo: (a) to (b) later
  breaks profiles that rely on prefixes. Blocks nothing now: the current fail-closed list stays.

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

## Suite time

- S3: kernel `cargo test` 57 tests in 0.05 s; CI gate job 17 s (2026-11-20, macos-15).
