# agentic-harness: where things are, for agents

The agent layer of watane: a kernel mod that runs Claude Code and ChatGPT
sessions for other mods, its providers, and the console, the kernel's own
page. Its small host part lands in watane (`../../watane`), never here.

## Find the right document

| You need | Read |
|---|---|
| Why, what comes first, boundaries | `design/VISION.md` |
| What it does, journeys | `design/FUNCTIONAL.md` |
| How: interfaces, state, rules, tests, build stages | `design/TECHNICAL.md` |
| What the console looks like | the canvas row "Revision 2" of [watane agent frontend](https://claude.ai/artifact/8jz2aHuaPo2REtKQyMt89e), with `TECHNICAL.md` section 18 |
| What is built, what is next | `factory/PLAN.md`; one task's brief: `factory/tasks/<id>.md` |
| How the build process works | `.claude/factory/README.md` |
| watane's own rules (host, pkg, app, dev) | `spec/` and `<sub>/TECHNICAL.md` in the watane checkout |

## How to treat them

- **The chain decides.** `design/VISION.md`, then `design/FUNCTIONAL.md`,
  then `design/TECHNICAL.md`, then code. When two disagree, fix the lower one
  at once, and fix each file that cites it. The open points are N5
  (`factory/PLAN.md` "Waiting on the human", stage F) and any later item the
  PLAN lists under "Waiting on the human"; they are the human's, and a slice
  that depends on one waits for the answer. The answered ones (N1–N4,
  N6–N11, R1) are rules in the specs.
- **Backend first.** Sections 1–17 of `TECHNICAL.md` (and 3.1–3.10 of
  `FUNCTIONAL.md`) are the backend. When a choice or the time left pits it
  against the console, the backend wins. The console uses only the kernel
  interface every client has. If it needs more, add that to the backend
  sections first.
- **The console's scope is the canvas target**, not more. Anything past it is
  `FUNCTIONAL.md` section 5.
- **Contracts and records.** The three `design/` specs and this file are
  contracts. These are records: read them for why, never cite them as the
  rule, and do not rewrite their history:
  - `design/ARCHITECTURE-PROPOSAL.md` and `design/REVIEW-*.md`;
  - `design/COMPARISON-*.md`;
  - `design/FRONTEND.md` and `design/FRONTEND-2.md`;
  - `research_notes/` and `proto/`.
- **Ask the human** only to go against `VISION.md`, to weaken a security
  check (`TECHNICAL.md` section 14), or to settle an open point (`N<n>`).

## Tests

Each behaviour is proven at the lowest level that shows it
(`.claude/factory/README.md`, test pyramid).

| Level | Command |
|---|---|
| Gate (doctor, contract tests) | `.claude/factory/check-mod.sh` |
| Kernel and providers | `watane-dev test --json` |
| Console views | the component test run that the first console slice adds, and names here |
| Probes of the real programs | `proto/claude-probes/probes.py`, `proto/agent-spike/chatgpt-probes.py` (by hand, after a `claude` update) |

## Do not touch

- `.claude/skills/foreman`, `.claude/agents/`, `.claude/factory/`,
  `.github/workflows/factory-ci.yml`: the factory's copy, written by
  `watane-mods/factory/bin/sync.sh`.
- `ui/` by hand: the console build writes it from `console/`.

When the first slice adds the mod code, `watane-dev` writes its own guide to
a mod folder. Merge that guide into a section of this file; never let it
replace this file.
