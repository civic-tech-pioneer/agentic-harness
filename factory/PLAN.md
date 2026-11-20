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

## Sketched

### S1: the open probes of section 18 (local: needs the participant's `claude` login)

Story: a maker runs `python3 proto/claude-probes/probes.py v26 v27` on the Mac and sees, for the
current `claude`, (v26) whether a subagent prompt that starts with `/` runs a skill the profile
denies, and (v27) whether the shell grant's sandbox blocks a connection to loopback.
Outcomes:
1. `probes.py` runs each new probe by its id, alone, and prints a result line like the others.
2. Proposal section 18 gains rows V26 and V27 with the result, the `claude` version and the
   effect on the design; TECHNICAL 14's "not probed" line is not edited here (S2 owns it).
3. A breach (either one runs or connects) is written as the design effect and flagged to the PO.
Touches: `proto/claude-probes/probes.py`, `design/ARCHITECTURE-PROPOSAL.md` section 18 only.
Leaves out: V7 (judge account), V8 (stage A), V13 (unit test of host code).
Protects: *guard*. Stays local: its outcomes need the Mac's logged-in `claude`.

## Done

## Deputy decisions

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

## Suite time
