# spec-check, stage F (delegation and counters), agentic-harness main @ 10abec0

Scope: design/VISION.md (no delegation clause beyond "limits live in code", Later list), FUNCTIONAL 3.9, AGT-J5, 3.11 console rows tracing here; TECHNICAL 9.7 (+9.1-9.5, 2.3, 14, 16 cases 23/24/52, 17 stage F + budget row).
Run: spec-lint (plugin copy; repo has none, not adopted): 0 errors, 138 warnings, none about this scope except the M2/M4 noise (V-ids, F-AGENT-5..9).
Deviations from the skill: judged by me in one context (no per-pair subagents); no Codex confirmation (no second judge run), so nothing here is "confirmed"; all 18 candidate quotes passed spec-quotes.sh. N5 and factory profiles left out as asked.
Grades: BUILD = will come back as a question during the build; LATER = survivable.

## High (BUILD)

1. AGT-J5 cannot run on any backend.
   FUNCTIONAL.md:199 "slice; the foreman starts a validator child."
   FUNCTIONAL.md:126 "An OpenAI child shall not start children."
   TECHNICAL.md:1681 "request has no `task` tool, and a `task` call from a child is refused."
   TECHNICAL.md:1682 "Claude: subagents from the launch, depth from `subagent-depth`."
   PO -> foreman -> validator needs a child that starts a child. OpenAI children cannot; Claude agents have no tool to start kernel child sessions (9.7 gives only the OpenAI `task`). Stage F "Done when" is AGT-J5, so it is unreachable as written. Also "refuses a fourth level" needs depth >= 2 while the default is 1.

2. AGT-F-37 describes child sessions for both backends; TECHNICAL gives them to OpenAI only.
   FUNCTIONAL.md:126 "An agent shall start a child session only with a profile that its own profile names."
   TECHNICAL.md:1682 "Claude: subagents from the launch, depth from `subagent-depth`."
   (also TECHNICAL.md:1659 Claude subagents "are launch fields of the profile (4.1), not profiles"). For Claude there is no `delegates` check, no live-children count, and the limit is a different knob (see 4). Question: is AGT-F-37 OpenAI-only, or does Claude need a child-session tool?

3. "Narrow" has no rule for a child.
   FUNCTIONAL.md:126 "A child shall only narrow its parent."
   TECHNICAL.md:1655 "counts against `subagent-limits {depth 1..3 (default 1), live"  (9.7 says "narrows the parent", 1654)
   Narrowing is defined only for overlays (9.2, 766-773). Which profile fields are compared (toolsets, tools, allow, mode, limits, workspace), where it is checked (profile save or `task`), and which refusal field it gives are not stated. `task(profile, input)` (1679) carries no overlay or folder, so a child's workspace and its folder consent (AGT-F-28, FUNCTIONAL.md:110) are also undefined.

4. Two depth knobs, and whose limit counts is unsaid.
   TECHNICAL.md:1682 "Claude: subagents from the launch, depth from `subagent-depth`."
   TECHNICAL.md:1655 "counts against `subagent-limits {depth 1..3 (default 1), live"
   `subagent-depth` (0..3, 381/377, overlay-lowerable) vs `subagent-limits.depth` (1..3, not overlay-lowerable). Not stated: whether depth counts the root, whether `live` is per parent or per tree, and whether root's, parent's or child's profile supplies the limit. AGT-J5's "a fourth level" fits only depth 2 counting the root.

5. Counters: scope and Claude enforcement.
   FUNCTIONAL.md:125 "the tokens of each session, for both backends, and stop the turn at a limit"
   TECHNICAL.md:1684 "each turn: `max-turns`, `max-tool-errors` in a row, `max-tokens`. A limit"
   per session vs per turn (does `max-tokens` accumulate across turns; does a spent limit stop every later input at once; does the failure streak survive a turn end).
   FUNCTIONAL.md:125 "At the turn limit the agent gets one last turn without tools to sum up."
   TECHNICAL.md:381 "| `max-turns` | 1..1000 | `--max-turns=` |"
   For Claude `max-turns` is Claude's own flag, so the kernel cannot give a last turn without tools. Also "turn" is the FUNCTIONAL term for one run from input to stop (1: Terms), but 9.7 counts something per model step.
   FUNCTIONAL.md:125 "stop the turn at a limit of the profile, with the limit named."
   TECHNICAL.md:1705 "| `result` | usage into the counters; `idle {completed}` only when `claude` is idle |"
   Claude usage reaches the counters only at the end of the turn, so `max-tokens` cannot stop a Claude turn; how the kernel stops one (interrupt?) and the limit names are not given.

## Medium (BUILD)

6. `task` returns twice.  TECHNICAL.md:1679 "`task(profile, input)` starts a child and returns its id at once;" vs 1680 "the child's final message arrives later as the call's output." One tool call has one output. Whether the parent's step waits on the child, whether a live `task` holds a job slot, and how the parent learns the child id are open. No entry kind in 9.4 carries a child, yet 2635 says "children in the transcript" and 803 says the id "stays in the parent's transcript".

7. Usage: stage and Claude source.  TECHNICAL.md:2771 "from stage D (9.7, the stages table), so a stage B session has no meter." vs 2635 "| F | Delegation and counters (9.7); factory profiles |" (9.7 owns Usage, but D shows the meter and D's AGT-J6 needs it). FUNCTIONAL.md:127 "The kernel shall report, after each step, how much of the model's context a session uses" vs TECHNICAL.md:1705 (Claude usage only at `result`); 9.7 says `context-window` "from the model" with only OpenAI's 272 000 given. Who builds `usage`, in which stage, and for Claude from where, will come up.

8. Budget row F against slot caps.  TECHNICAL.md:2655 "| F | D, each OpenAI session with 2 live children | as D |" with 2653 "plus 8 OpenAI sessions on the scripted Responses server, each one step/s with 4 tool calls" and 1344 "A child session counts against its root's owner." Kernel holds 8 request slots, 4 per owner, 2 owners (2646): D already uses them all, so 16 children's requests are `busy` or queue and "as D" for delay cannot hold unless parents idle while children run. State the intended concurrency.

9. Children after a restart are unspecified.  FUNCTIONAL.md:131 "After a restart of watane, the owner shall reattach to each session and continue it." TECHNICAL.md:1012 restart row (turn becomes `unknown`, every job `aborted`) and 1687 "Cancelling a session cancels its children, depth first." say nothing about live children of a restarted parent. Case 24 and 52 do not cover it.

10. Console has no clause for children.  TECHNICAL.md:2635 "children in the transcript; stopped at a limit" vs FUNCTIONAL.md:143 "inputs, thinking, tool calls, asks, verdicts, decisions with who made them" and :141 "show and change only the sessions it started". No 3.11 requirement, no 18.3 row, `session-info` (2.3:292) has no parent field, and the child's owner is not stated (can the console read it?). "Stopped at a limit" is covered (18.5 row, 2791).

11. OpenAI `skill` tool lands at stage F but is not in the stage F row.  TECHNICAL.md:2044 "a `skill` function, which stage F adds" vs 2635 "| F | Delegation and counters (9.7); factory profiles |".

## Low

12. AGT-F-3 says overlays may lower "limits" (FUNCTIONAL.md:62 "adds deny rules or lowers limits"); the overlay table (TECHNICAL.md:766) cannot lower `subagent-limits`, and 768 "Every other key is refused as `overlay.<key>`."
13. FUNCTIONAL.md:60 "say which client may use which profile" vs TECHNICAL.md:1659 child "cannot widen its parent": a delegated profile needs no `access/` entry (1658). Intended, but AGT-F-1 reads absolute.
14. FUNCTIONAL.md:147 "It shall show how full the context is and warn before the kernel makes room." vs TECHNICAL.md:1692 "Claude, `compact-at` is absent: Claude decides when it compacts." (AGT-F-44 allows it; AGT-F-51 does not.)
15. Usage example TECHNICAL.md:1242 `"compact-at":180000` with window 200000 matches neither OpenAI (1414 `"context-window": 272000, "compact-at": 200000}`) nor Claude (no `compact-at`).
16. Section 14 has no delegation row except the runaway one (2120); "child cannot widen", "profile not named refused" and the depth/live refusal field (no refusal field is named for them; only `{field: "profile"}` for revocation, 1667) rest on cases 24 only.

Not judged: VISION has no delegation clause to pair with (only "Limits live in code", "Sessions, delegation, tools, limits and resume are what the factory needs"); 3.11 requirements 45-52 beyond those above; J2/J7 pairs; ACCEPTED.md absent, nothing skipped.
