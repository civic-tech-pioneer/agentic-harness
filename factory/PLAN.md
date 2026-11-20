# Plan

Kept by the product owner (`/po`). The factory's process: `.claude/factory/README.md`.

## Order of work

The vision (`design/VISION.md`) has no numbered steps; the order is `design/TECHNICAL.md`
section 17. All three design files are drafts: no build slice depends on an open point (N1 to
N10) until the human answered it and a design slice made the drafts the contract.

| Stage | Builds | Where | State |
|---|---|---|---|
| 0 | Design slice: drafts become the contract (S2); open probes of proposal section 18 (S1) | this repo | sketched |
| A | H1 streams, H5 durable requests, H7 timers; V8 prototype | watane host | cross-project, waits (below) |
| B | H2 + unchecked provider + kernel Claude sessions; dev's fake `claude` | watane host, dev; kernel; provider | cross-project, waits |
| C | H6 consents, app words, `claude` binary setting, plan line | watane app, pkg | cross-project, waits |
| D | H3 accounts, OpenAI loop (kernel 9.5), scripted Responses server | watane host, dev; kernel | cross-project, waits |
| E | Checked provider (needs a judge account, V7) | provider repo; host if N10 = host | waits on V7 |
| F | Delegation and counters (kernel 9.7), factory profiles | kernel | needs N5 |

**Cross-project dependency.** Stages A to D put code in watane (`../watane`: host, dev, pkg,
app), which runs its own older factory and is not on this generic one. Each such stage becomes
one watane slice (provider, first) plus one slice here or in a provider repo (consumer). The
interface between them is `design/TECHNICAL.md` section 2.1 (`watane:host@0.1.0` additions) and
the watane contract rows of section 15. None of them is this product owner's to hand out until
watane is on the generic factory, or the human hands stage A to watane's own product owner. Stage
A has no consumer here at all: it is pure watane.

## In flight

### S2: the agent layer's contract (in flight (cloud), routine trig_01D59fStFZruNVCJ2QeRNPpi)

Story: a foreman of stage A or B (a coding agent) opens this repo and reads the agent layer's
`VISION.md`, `FUNCTIONAL.md` and `TECHNICAL.md` as contracts: no draft marks, every open point
answered where its fact lives, and an `AGENTS.md` that tells it where things are.
Outcomes:
1. No "draft" marks left in `design/VISION.md`, `FUNCTIONAL.md`, `TECHNICAL.md`; each of N1 to
   N10 but N5 (and R1) is stated as a rule in the document that owns it, as the human answered (below),
   and no "N<n> is open" wording remains.
2. TECHNICAL section 14 names, for each boundary, the files (present or planned, in this repo or
   watane's) that hold it, so `factory/config.toml` `boundaries` finds files.
3. TECHNICAL section 15 lists the watane contract changes (N3) as a checklist a watane slice can
   apply row by row, each with its target document and section.
4. A new `AGENTS.md` at the root (`CLAUDE.md` pointing to it): where things are, which files are
   contracts and which records (proposal, reviews, comparisons), the gate, and that host code
   lives in watane.
5. The order of work in TECHNICAL 17 names, per stage, which repository builds which part.
Touches: `design/VISION.md`, `design/FUNCTIONAL.md`, `design/TECHNICAL.md`, `AGENTS.md`,
`CLAUDE.md`. Not the proposal, reviews or comparisons (records). Not watane.
Leaves out: any code; editing watane's `spec/`.
Protects: *core*. N5 stays open: write it as open, to be decided at stage F.

### S3: the kernel's core: profiles, overlays, the refuse table (in flight (cloud), routine trig_019eJBAcTCobEJRhVgtoZkCd)

Story: a client mod's maker (a coding agent) runs
`cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>`
and sees every refusal at once, as `refusal {field, backend, model, reason}` JSON lines, or none.
Outcomes:
1. An overlay that adds tools, allow rules, workspaces, subagent profiles or a weaker mode is
   refused, naming each field (AGT-F-3, case 26); one that appends instructions, adds skills,
   removes tools, adds deny rules or lowers limits compiles.
2. A profile set to OpenAI with file tools, a workspace, `max-budget-usd` or a path rule is
   refused, every field named at once (AGT-F-4, TECHNICAL 11); nothing is dropped silently.
3. `check` starts nothing and needs no host: the crate builds and tests natively.
4. Kernel errors (TECHNICAL 12, kernel rows) are one Rust type with a fixed code and a message
   naming the cause (AGT-F-40).
5. `factory/config.toml` `gate` becomes `factory/gate.sh`: `cargo test` and
   `cargo clippy --all-targets -- -D warnings` in `kernel/`; `npm ci && npm test` in `console/`
   when `console/package.json` exists; `.claude/factory/check-mod.sh` when `wit/world.wit` exists.
Touches: `kernel/` (new crate `agent-kernel`, lib plus `examples/check.rs`), `factory/gate.sh`,
`factory/config.toml` (`gate` line only). Not `design/` while S2 is in flight: a spec question is
a `BLOCKED`.
Fakes: nothing of the host; no WIT yet (the `watane:host@0.1.0` additions do not exist).
Leaves out: offers (adoption, hashes), sessions, state, the launch-document grammar of 4.1 beyond
the fields the refuse table names.
Protects: *guard* (overlays only narrow).

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

## Sketched

### S5: the performance review's must-dos in the contract (cloud, after S2 merges)

Story: a foreman of stage A, B or D reads TECHNICAL and finds a bound for every size and wait the
performance review (`research_notes/performance-review-2026-11-20.md`) found missing, each with
the test case that proves it.
Outcomes:
1. P01 to P06 are rules in the documents that own them (TECHNICAL 3, 9.1, 9.3, 9.4, 9.5, 10, 16;
   FUNCTIONAL where a requirement changes), each with its fix as the record states or a better one
   named in the delivery.
2. P15: TECHNICAL 17 gives each stage pass/fail budgets (p95 item delay, p95 Stop latency,
   writer busy share, bytes per step), so "V8 measured" has a passing number.
3. P13: `read` pages backwards (`before` cursor), so the console's "newest first" holds.
4. Each new rule has a numbered case in TECHNICAL 16.
Touches: `design/TECHNICAL.md`, `design/FUNCTIONAL.md`. Not the record.
Leaves out: P07 to P12 (S6), P14 (S7). Protects: *core* (*guard* for P01, P02: a cap that holds).

### S6: the review's should-dos P07 to P12 (cloud, after S5)

Story and outcomes as S5, for P07 (active history after compaction), P08 (chunk rewrite cost),
P09 (stream-to-session key), P10 (timer budget), P11 (host queue memory), P12 (kernel quota).
Touches: `design/TECHNICAL.md`. Protects: *core*.

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

## Deputy decisions

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

## Suite time
