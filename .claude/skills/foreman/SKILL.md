---
name: foreman
description: Builds one vertical slice of a factory project (watane or a mod) as its build team. Writes the task briefs, implements each task, opens the slice's pull request, and answers each review round of the other vendor's review team with one push. Use when started as "/foreman <slice id>", or when a brief says "follow .claude/skills/foreman/SKILL.md", in Claude Code or in Codex.
---

# /foreman <slice>

You build one slice, in one project: the repository you run in. The product owner (PO) gave you its
sketch (the story, the outcomes, what it touches, the interfaces). You work on branch `slice/<id>`.
Do not edit `factory/PLAN.md`; the PO owns it. Never push `main`, never force-push, never merge: the
PO merges.

Read first: `factory/config.toml` (this project's settings), `.claude/factory/README.md` (the
process), the files of the config's `spec` list in order (why > what > how: the priority of truth),
and `AGENTS.md`. The config's `[agents]` names the agent for each role below; this text calls them by
role.

## 0. Where you run

In a cloud, always: a Claude Code cloud session (`CLAUDE_CODE_REMOTE` is `true`) or a Codex Cloud
task. You are the build team: you brief and build (sections 1 and 2), deliver (section 5), and end.
The review team, usually the other vendor, reviews your PR, and the PO brings each review back to
you (section 6). You do not run the validator, adversary or user-sim; they are the review team's.
You cannot message the PO: you report through the PR and the branch.

## 1. Brief each task

At most 4 tasks, cut by folder and by file, in dependency order. Write `factory/tasks/<slice>-<n>.md`:
```
Folder: <path from the project root>|.
Effort: <low|medium|high|xhigh>
Depends on: <task ids>|none
Slice: <id> — <story>
## Goal            one paragraph: what exists after this task, in user terms where possible
## Acceptance      numbered, observable criteria (AC1, AC2, ...): input -> expected result
## Spec            the sections to follow (file and section numbers), not copies of them
## Interface       what this task provides to / takes from a sibling: exact names and types (fixed)
## Files           the files this task may touch
## Changed tests   each existing test that asserts a behaviour this task changes, and what it
                   asserts after; "none" only after you searched the tests, slow ones too
## Out of scope    what a later slice does
```
Effort: `medium` unless a rule says otherwise. `low` for mechanical work (a fixture, a rename, a
test for existing behaviour). `high` only when the task decides something the spec leaves open at a
security boundary; name that decision in the Goal. `xhigh` only after two failures at `high`. Raise
effort after a failed run, never up front.

A task depends on another only if it needs that task's code to compile or its tests to run. Cut the
briefs so most say `none`, and fix the interface in both briefs' `## Interface`. Commit the briefs
first; the review team checks the work against them.

## 2. Build each task

1. Implement each task, test first, in dependency order, by its brief and the standing rules of
   `.claude/factory/implementer.md`:
   - Codex Cloud: you implement each task yourself.
   - Claude cloud: run the agent named by `implementer_cloud` with the task id, one task at a time
     in this checkout.
2. After each task: `.claude/factory/check.sh <task> <commit before it>`. `CHECK FAIL`: fix (or
   send the failing part of the log back to the implementer) before the next task. `BLOCKED` or
   `QUESTIONS` from an implementer go to section 4.
3. A check that only a Mac or other hardware can run (macOS, a device, timing) does not run here
   or on the human's Mac: it becomes a CI job (`.claude/factory/README.md`, "Nothing runs on the
   Mac"), as a task of this slice.
4. Past 3 implementer runs on one task, a flake past one diagnosis, or a slice in flight for more
   than a working day: weigh what it protects (`.claude/factory/README.md`, "Time and value")
   before the next round, and write the answer as a decision line.

## 3. The adversary rule (the review team applies it)

The review team runs the adversary when this rule holds; you name in your report what it matched.

Skip the adversary only if the config's `adversary` is empty or `boundaries` is
empty; write `adversary: not run (<which>)`. Otherwise read the section `boundaries` names. The slice
touches a boundary if any of these holds; check all three:
- a brief has `Effort: high`; or
- the sketch or a brief names a boundary of that section; or
- `git diff --name-only main...slice/<id>` lists a file that section names for a boundary.
If it does, the adversary runs before user-sim, on the boundaries that matched. If none holds:
`adversary: not run (no boundary)`; never skip it for time. A `BREACHED` or `REJECTED` comes back to
you as findings with red tests (section 6); a fix that touches files the adversary attacked keeps
its `HELD` attacks holding.

A file that holds a boundary and is missing from that section: add it in the same commit.

**A documents-only slice** (no code in its diff): the adversary is `not run (no code)`, unless the
slice changes the boundaries section itself; then it reads the change for a weakened boundary and
reports in its format, with checks instead of tests. The user-sim checks each outcome by reading and
by shell checks (`grep`, `.claude/factory/check-docs.sh`).

## 4. Questions

Decide yourself what stays inside this slice and is cheap to undo: a choice inside one folder, a spec
that is unclear but whose intent is plain. Edit the spec that owns the fact, in place, and note the
decision for your report.

Ask the PO only for what stops your progress: a public interface, a format on disk or on the wire, a
security boundary, an interface with another project, or anything outside the sketch, that no option
you can build keeps open. One question, its options, your recommendation and the cost to undo each.
Every other such question you settle yourself: build the option that keeps the others open, and list
it under decisions. To ask: commit and push all work, the briefs too, then push an empty commit
`git commit --allow-empty -m "BLOCKED <id>" -m "<the question>"` to `slice/<id>` and end. The PO
answers in a new round on that branch.

**Time.** You do not feel time pass: run `date` when you start and write the start time in your first
brief's commit message; every time you report is `date` minus that, never an estimate.

Send no progress updates. While a background run works, end your turn with no message.

## 5. Deliver

1. Rebase `slice/<id>` on `origin/main` (`git fetch` first). Run the project's whole test set once,
   the slow tests too (`AGENTS.md` says how), and the gate; they must pass.
2. Push `slice/<id>` and open its PR, base `main`, title `<id>: <outcome>`, body the report below:
   `gh pr create`, or this session's GitHub tools (a Claude cloud session: the GitHub MCP server; a
   Codex task: its GitHub integration). Never arm auto-merge: the PO merges. If no tool can open a
   PR, put `no PR: <why>` and the report in the body of your last commit; the PO opens it.
3. The report:
   - `DELIVERED <id>`, the project, the branch, the builder (`claude` or `codex`);
   - what a person can now do (one line);
   - the boundaries your briefs or diff touch, for the review team's adversary rule (section 3);
   - each check that needs a Mac or hardware, and the CI job that runs it;
   - decisions you made, one line each;
   - friction: one line each, for anything in the process that got in the way;
   - implementer runs per task (`factory/runs/ledger.jsonl`, or your own count);
   - suite times, default and slow, before and after, and the hours the slice was in flight.
4. End. The PO watches CI and brings a red job back to you as a finding.

## 6. Answer a review round

The PO starts you with an answer brief: the PR, the round `r`, and the review (a PR comment that
ends with `VERDICT: changes (n)`, its findings numbered `F<r>.<k>`).
1. `git pull` `slice/<id>`; read the review and the earlier rounds on the PR (the thread is the
   memory of a fresh session).
2. Answer every finding, none skipped:
   - **fix it**: a finding that carries a red test: commit the test first, unchanged, then the fix;
     or
   - **disagree**: only with a reason from the spec chain or the brief (the review team may be
     wrong; say where).
3. Run the touched tests and the gate (`check.sh`); they must pass.
4. Push once per round. The last commit's subject is `<id> r<r>: answers`; its body has one line
   per finding: `F<r>.<k> fixed in <sha>` or `F<r>.<k> disagree: <why>`. Post the same lines as a PR
   comment too, if this session can comment. Then end. Your push starts the next review.

Never argue with the verdict line itself, never resolve or hide a review comment, and never push
twice in one round: a second push races the review that the first one started.
