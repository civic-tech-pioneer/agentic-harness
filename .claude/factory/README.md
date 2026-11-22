# The factory

One pipeline for watane and each mod: a product owner sketches vertical slices, a foreman per slice
builds it, a review team of the other vendor reviews its pull request round by round, and the product
owner merges it on the reviewer's approval and green CI. Derived from watane's own factory (`watane/factory/README.md` tells why
it looks like this; that file is a record, this one is the contract for every project that uses it).

## Where things live

| What | Where | Who edits it |
|---|---|---|
| The pipeline: skills, agents, scripts, CI | `watane-mods/factory/` | the human, or the product owner on the human's word |
| Its copy in a project | `<project>/.claude/skills/foreman`, `.claude/agents/`, `.claude/factory/`, `.github/workflows/factory-ci.yml` | nobody: `factory/bin/sync.sh` writes it; commit it in the project |
| The product owner | `~/.claude/skills/po`, a link to `watane-mods/factory/skills/po` | as the pipeline |
| A project's settings | `<project>/factory/config.toml` | the project |
| Its plan, briefs and run logs | `<project>/factory/PLAN.md`, `tasks/`, `runs/` (git-ignored) | product owner, foreman, scripts |
| Its why, what and how | the files `spec` in its config names, in that order | foremen, by the spec chain |

The product owner runs only on the Mac, so it is a user skill and never copied. Every team runs in a
cloud, in the project's own clone. A cloud session clones only that project and loads no
plugins (probes of 2026-11-20), so everything a foreman needs is in the project's copy.

`factory/bin/adopt.sh <project folder> <owner/repo>` sets a project up once: the copy, a starting
`factory/config.toml` and `PLAN.md`, the git-ignore lines, and the repository's ruleset. After a change to the pipeline: `factory/bin/sync.sh <project folder>` for each project
in `projects.toml`, and commit there.

## Config

`<project>/factory/config.toml`; `factory/config.example.toml` explains each key. The scripts read
the simple `key = "value"` lines with `sed`, so keep each such value on one line.

**The agents.** `[agents]` names the agent for each role. The defaults are the factory's own
(`implementer`, `validator`, `user-sim`, `adversary`). To change one for a project, put an agent
with another name in its `.claude/agents/` and name it here; `sync.sh` writes only the factory's
own four, so the project's agent survives every sync. An empty `adversary` skips that step.

## Slices that span projects

A story that needs code in two projects (a mod and the host part it needs in watane) becomes one
slice per project. The product owner fixes the interface between them in both sketches, hands the
provider's slice out first, and the other once the provider merged or a fake of it is in the
sketch. One foreman never works in two repositories. A provider project that is not on this
factory yet (watane, today) is a cross-project dependency in `PLAN.md`: the consumer's slice
fakes it, and the provider's part waits for the human or that project's own product owner.

## One writer on main

While a project has slices in flight, its `main` takes only the product owner's `PLAN.md`
commits and merged PRs. Any other change (a design edit by hand, a factory sync) goes through a
PR, after a look at the `Touches` of the slices in flight: a direct commit to files a slice owns
makes that slice's rebase someone else's problem.

## Test pyramid

Each behaviour is proven at the lowest level that can show it; a higher level only proves that
the parts work together. A project's `AGENTS.md` names its levels and their commands. The rules:
- the project's default test run stays under 2 minutes; a test that needs more is marked slow,
  and its reason names what no cheaper test proves;
- the implementer runs the tests it works on, then the default run once; the build team runs
  `check.sh` (the project's gate) after each task and everything once before it delivers; the
  review team runs the gate once per round (the build team's logs are not in its clone); the
  user-sim runs no suite, it uses the programs;
- a failure the user-sim or the adversary finds becomes a test at the lowest level that shows it.

## Time and value

Agents do not feel time pass. These checkpoints stand in for that feeling:
- **Weigh what the work protects** before a fix slice, a new slow test or another round:
  *guard* (authorization, consent, isolation, data loss: worth what it takes), *core* (a journey
  step, a feature a person relies on), *polish* (wording, layout, convenience: one attempt, then
  an accepted limit in `PLAN.md`).
- **Suite time is a budget.** Each delivery reports the default and slow suite times before and
  after; the product owner keeps them in `PLAN.md`. A default run over 2 minutes or a growth over
  20% in one slice: the next slice of that project moves tests down the pyramid first.
- **Elapsed time is a signal.** A task past 3 implementer runs, a flake past one diagnosis, or a
  slice in flight for more than a working day: stop, weigh, and write the answer as a decision.

## Two teams, one PR

A cloud slice has two teams of two vendors (Claude and Codex), and the slice's pull request is the
only channel between them. The product owner, on the Mac, dispatches every round and picks who
builds (the PO skill, "Who builds, who reviews": Codex two slices in three).

| Team | Who | Does | Ends each round with |
|---|---|---|---|
| Build | the foreman (`skills/foreman`): briefs and implementation | pushes `slice/<id>`, opens the PR, answers every finding | one push; last commit `<id> r<r>: answers`, a line `F<r>.<k> fixed in <sha>` or `F<r>.<k> disagree: <why>` per finding |
| Review | the other vendor, acting as `validator`, `adversary` and `user-sim` (`.claude/agents/`) | reads the PR and runs the programs; pushes nothing | a PR comment: numbered findings, then one line `VERDICT: approve` or `VERDICT: changes (<n>)` |

```
PO: build brief ──> builder pushes slice/<id>, opens PR
PO: review brief r ──> reviewer posts review r ──┬─ VERDICT: approve + CI green ──> PO: gh pr merge --admin --rebase
                                                  └─ VERDICT: changes (n) ──> PO: answer brief r ──> builder pushes once ──> r+1
                                                     after review 5 without approve ──> the human
```

- **Dispatch.** Claude side: a one-off claude.ai routine (`RemoteTrigger`), later rounds of a team
  into its earlier session where the API allows. Codex side: the product owner's built-in browser
  pane, signed in to chatgpt.com, in the Codex Cloud environment named by `codex_env`; Codex pushes
  and opens or updates the PR itself, as the human's account. The Codex CLI cannot reach that
  environment (openai/codex#50182), and nobody uses computer use for it. The PO skill holds the
  briefs, the steps and their failure modes.
- **Waking.** No GitHub triggers: the product owner runs `scripts/watch-pr.sh` per slice, which
  returns on a push, a comment or a closed PR. A Codex review shows only on its task page; the
  product owner copies it to the PR verbatim, so the PR thread stays the whole record.
- **Memory.** Each round may be a fresh session: reviews list each finding in full, answers name
  each finding, and the PR thread carries the rest.
- **Merge.** Every PR is authored by the human's account, so nobody can give a GitHub approval and
  auto-merge is retired. The verdict line stands in for approval; the product owner merges with its
  admin bypass, after it checked CI itself.

## Nothing runs on the Mac

The Mac runs only the product owner: no team, build or check. A check that needs macOS or
hardware the clouds lack (a device, timing, a GUI) runs in CI: `factory-ci.yml` runs the gate on
`macos-15`, and a project adds its own workflow for more. The slice that first needs such a check
adds its CI job as one of its tasks, so the check runs on every PR from then on. Until a job can
exist (hardware no runner has), the check is a line under `Upkeep` in `PLAN.md` with what it
protects, and the outcome it covers waits under `Waiting on the human`.

## Two feedback loops

| Loop | Sensors | Where | Before |
|---|---|---|---|
| Fast | `check.sh` (the project's gate) by the builder; validator, adversary, user-sim by the reviewer | the teams' clouds | each push, then each review |
| Slow | CI, `.github/workflows/factory-ci.yml` (the gate on a clean runner) and any workflow of the project's own | GitHub, through the pull request | the merge to `main` |

The repository's ruleset ("main: CI green before merge") requires the `gate` check; the product
owner merges with `gh pr merge <n> --admin --rebase` only on `VERDICT: approve` and green checks, and
pushes its own `PLAN.md` commits straight to `main` through the same admin bypass. A ruleset on a
private repository needs a paid GitHub plan, so a project that uses the factory is public, or its
org pays.
