# Skills, subagents and delegation in other harnesses (2026-11-20)

A record, not a contract. It compares how other agent harnesses handle group 3
of the agentic layer: skills, subagents and multi-agent delegation. It feeds
`ARCHITECTURE-PROPOSAL.md` (revision 3). Section numbers like "4.1" or "10"
refer to that proposal.

Method: shallow clones, read only; nothing was built or run. Docs, issues and
changelogs were read on the web on 2026-11-20. GitHub's search rate limit cut
some issue searches short, so issue lists are samples, not complete. Each
claim cites `repo@commit:path:line`. Line numbers of files read through the web
(OpenHands SDK, Claude Code docs) are approximate.

## 0. Sources and licenses

| Short name | Repo | Commit | License | Code reusable by watane |
|---|---|---|---|---|
| codex | openai/codex (codex-rs) | bb5b9c6 | Apache-2.0 | yes, with notice |
| agents-py | openai/openai-agents-python | 6c4e4a2 | MIT | yes (Python: ideas) |
| claude-sdk | anthropics/claude-agent-sdk-python | 69846cd | MIT | yes (Python: ideas) |
| cc-acp | zed-industries/claude-code-acp | a4f0f78 | Apache-2.0 | yes, with notice (TS) |
| agentskills | agentskills/agentskills (+ skills-ref) | 83021e2 | Apache-2.0 | yes, with notice |
| gemini | google-gemini/gemini-cli | d4cb171 | Apache-2.0 | yes, with notice (TS) |
| opencode | sst/opencode (now anomalyco) | b7f0b90 | MIT | yes (TS) |
| pi | badlogic/pi-mono | e5d93ea | MIT | yes (TS) |
| cline | cline/cline | 20b0427 | Apache-2.0 | yes, with notice (TS) |
| goose | block/goose (now aaif-goose) | 80ce943 | Apache-2.0 | yes, with notice (Rust) |
| openhands | All-Hands-AI/OpenHands; agent code in OpenHands/software-agent-sdk (web) | 3df9648 | MIT | yes (Python: ideas) |
| rig | 0xPlaygrounds/rig | 0019e24 | MIT | yes (Rust) |
| forge | antinomyhq/forge | a22b492 | Apache-2.0 | yes, with notice (Rust) |
| swiftide | bosun-ai/swiftide | 92e1a30 | MIT | yes (Rust) |
| autoagents | liquidos-ai/AutoAgents | d1f8cc5 | MIT OR Apache-2.0 | yes (Rust) |
| golem-ai | golemcloud/golem-ai | 0923caf | Apache-2.0 | yes, with notice (WIT, Rust) |
| vibe-kanban | BloopAI/vibe-kanban | 3f095f0 | Apache-2.0 | yes, with notice (Rust) |
| zed | zed-industries/zed | 10d1b2a | GPL-3.0-or-later (agent crates) | **no**: ideas only |

**License rule.** watane's crates that state a license use `MIT OR
Apache-2.0` (`log/Cargo.toml:6`, `net/Cargo.toml:7`); `deny.toml` allows
permissive licenses only. MIT code fits as is. Apache-2.0 code can be ported,
but a copied file stays Apache-2.0 only: keep its notice, and do not relabel
that file as MIT. Rewriting from the idea avoids this. Zed's agent crates are
GPL-3.0: do not copy them.

**Repo drift seen while reading.** The OpenHands repo at 3df9648 is now a
TypeScript UI ("Agent Canvas"); its agent code lives in
`OpenHands/software-agent-sdk`. opencode at b7f0b90 is the v2 "core" rewrite:
its old `task` tool became `subagent`. Cline at 20b0427 removed `new_task`
and `use_subagents`; it now has `spawn_agent` and team tools in its SDK.

## 1. What watane needs from this group

From proposal sections 1, 2, 4.1, 10, 11, 12, 13, and from the factory
(`factory/README.md`, `.claude/skills/foreman`, `.claude/skills/product-owner`,
`.claude/agents/`):

| Capability | What watane needs (one line) |
|---|---|
| Skill format | One format that `claude --plugin-dir` reads and the OpenAI loop can serve as a `skill` function (4.1, 11) |
| Discovery, disclosure | Skills come from offers and profiles, never from the disk; only name and description in the prompt; body on demand (10, 11) |
| Skill trust | A skill is configuration: shown to and adopted by the participant, cut to `name` and `description`, no scripts run (4.1, 10, review 3) |
| Subagent definitions | `{name, description, prompt, tools ⊆ tools, model}` per profile; refused fields stay refused (4.1, 11) |
| Delegate and collect | A child runs inside the 10 s call model: start returns an id, the result arrives later as an item (6, 12) |
| Parallel, limits | Depth, concurrency, turns and spend capped by the host or kernel, never by a prompt |
| Handoffs | Passing a session's history to another profile must not widen reach or carry injected text into a stronger agent |
| Recipes, profiles | Profiles are the reusable configuration; only the participant writes them; overlays only narrow (10) |
| Workspace isolation | The factory needs one git worktree per foreman and per parallel task, made and cleaned by the host (4.4) |
| Meta-harness | The factory as mods: start background sessions with a profile and a workspace, message them by name, see "waiting on a prompt", resume, collect a typed report (13) |

The factory today does this by hand: `claude --bg` per foreman, `SendMessage`
by session name, `factory/watch.sh` polling `claude agents --json` for
`blocked`, `codex exec resume <thread>` with feedback on stdin, APFS clones of
`target/` and of git-ignored folders into each worktree, and a run ledger
(`factory/impl.sh:1-60`, `factory/watch.sh:1-15`).

## 2. Skill format

**Agent Skills spec** (agentskills@83021e2:docs/specification.mdx:27-172).
```
name         required, <= 64, lowercase a-z 0-9 and '-', no leading/trailing '-', no '--', = folder name
description  required, <= 1024, non-empty
license, compatibility (<= 500), metadata (string map), allowed-tools (experimental)
```
Folder: `SKILL.md` plus optional `scripts/`, `references/`, `assets/`
(:11-15). The reference validator `skills-ref` rejects unknown fields
(skills-ref/src/skills_ref/validator.py:104-115), but its name check accepts
any Unicode alphanumeric after NFKC (:25-67), wider than the spec text. It
does not check `metadata` value types or `allowed-tools` syntax. Tests: pytest
with `tmp_path` fixtures (tests/test_validator.py, 21 tests; test_parser.py;
test_prompt.py).

**Claude Code** (code.claude.com docs, read 2026-11-20). Adds `allowed-tools`,
`disallowed-tools`, `context: fork`, `agent`, `disable-model-invocation`,
`user-invocable`, `arguments`, `paths`, `hooks`, `model`, `effort`, `shell`.
`` !`cmd` `` in a body runs a shell command on load unless the setting
`disableSkillShellExecution` is on. `metadata`, `license` and `compatibility`
are accepted and ignored.

**Others.**
- Codex: `name`, `description`, `metadata.short-description`; a sidecar
  `agents/openai.yaml` with `interface`, `dependencies.tools` (MCP servers) and
  `policy.allow_implicit_invocation` (codex@bb5b9c6:codex-rs/skills/src/parser.rs:4-28,
  codex-rs/skills/src/metadata.rs:26-58). No `allowed-tools`: a skill cannot
  grant anything.
- Gemini reads only `name` and `description` and ignores `allowed-tools`
  (gemini@d4cb171:packages/core/src/skills/skillLoader.ts:42-134).
- opencode, pi, cline, goose, rig, forge all read Agent Skills `SKILL.md`. pi
  enforces the spec's name grammar and lengths
  (pi@e5d93ea:packages/coding-agent/src/core/skills.ts:11-14,95-122).
- OpenHands SDK also reads legacy microagents, `.cursorrules`, `AGENTS.md`,
  `CLAUDE.md` and `GEMINI.md` as skills, with keyword, task and path triggers
  (software-agent-sdk `openhands-sdk/openhands/sdk/skills/skill.py`, web).

**Pitfalls.**
- `allowed-tools` does not reliably grant in Claude Code
  (anthropics/claude-code#14956, #98189, #99353), and watane's probe V15 saw it
  do nothing under `--plugin-dir`. Nobody should rely on it either way.
- A YAML colon broke descriptions in Claude Code until 2.1.69 (CHANGELOG).
  Gemini added a fallback line parser for the same reason
  (skillLoader.ts:42-134).
- `` !`cmd` `` bodies run shell on load in Claude Code and in the OpenHands SDK
  (`render_content()`). V15 saw no run under `--plugin-dir`, but that is one
  probe on one version.

**Lessons for watane.**
- **Adopt** the Agent Skills grammar exactly as watane already does in 4.1
  (`[a-z0-9-]{1,64}`), and add the spec's two extra rules: no leading or
  trailing `-`, no `--`. Add the spec's 1024-character description limit.
- **Adopt** the frontmatter cut to `name` and `description` (4.1). Every
  harness that reads more fields gained bugs or grants from them.
- **Proposal change (4.1, host settings):** the host's `--settings=` also sets
  `disableSkillShellExecution: true`. Defence in depth beside the frontmatter
  cut; a new probe checks it (section 14, T3).

## 3. Discovery and progressive disclosure

**The common shape.** Every harness puts only name, description and
sometimes a location into the prompt, and loads the body on demand:
- Agent Skills client guide: about 100 tokens per skill at start, body on
  activation (< 5000 tokens advised), resources on demand
  (agentskills@83021e2:docs/specification.mdx:216-224). Project skills win over
  user skills on a name clash, with a warning
  (docs/client-implementation/adding-skills-support.mdx:81-87).
- Two ways to load the body: a dedicated tool (Gemini `activate_skill`,
  opencode `skill`, goose `load_skill`, forge `skill_fetch`, OpenHands
  `invoke_skill`, cline `skills`) or a file read of the listed location
  (pi, rig, Codex).

**Budgets.**
- Claude Code: the listing gets 1% of the context window; each entry is capped
  at 1,536 characters; over budget, the descriptions of the least-used skills
  are dropped (docs).
- Codex: 2% of the window, else 8,000 characters; descriptions cut to 1,024;
  over budget, descriptions go first, then whole skills, with a warning
  (codex@bb5b9c6:codex-rs/ext/skills/src/render.rs:23-33,169-195). An injected
  body is capped at 8,000 bytes (render.rs:25).

**Dedupe.** Codex dedupes by `SKILL.md` path, not name, so two skills with one
name both appear (codex-rs/ext/skills/src/loader/host_merge.rs:228-268). goose
puts project skills before global ones, so a cloned repo shadows a user's
skill of the same name (goose@80ce943:crates/goose/src/skills/mod.rs:324-372,
test at :567).

**Interface worth copying.** goose's skill module compiles for wasm32; only
the file-system discovery is native
(goose@80ce943:crates/oi-kit-agent/src/skills/mod.rs:5). Its prompt section is
"name: description" lines, and `load_skill("skill/rel/path")` returns a listed
supporting file only (crates/oi-kit-agent/src/skills/operation.rs:25,124,311-326).
opencode sends a diff message when the list changes instead of a new list
(opencode@b7f0b90:packages/core/src/skill/instructions.ts:40-45).

**Pitfalls.**
- Descriptions silently dropped or truncated over budget:
  anthropics/claude-code#13343, #56448, #57168; compaction drops skills
  (#96168).
- opencode puts descriptions into its XML list unescaped
  (packages/core/src/skill/instructions.ts:16-23): a description can close the
  tag and inject text. pi and skills-ref escape
  (pi skills.ts:358-397; skills-ref prompt.py:9-58).
- An inline mention loaded a denied skill's body without the permission check
  (opencode#49891).

**Lessons for watane.**
- **Adopt** skills-ref's listing format (`<available_skills>` with escaped
  name and description) for the OpenAI loop's `skill` function (11). Escape
  every field.
- **Adopt** a budget check in the compiler: when a profile's skill
  descriptions exceed what the backend lists, the kernel says so on its page
  instead of letting the backend drop them silently.
- **Avoid** dedupe by path and silent shadowing. **Proposal change (10):** the
  compiler refuses a launch in which two skills (profile, overlay, adopted
  offers) share a name. No precedence rule, so nothing can shadow.

## 4. Skill trust and sandboxing

**What others do.**
- **Gemini**: workspace skills load only in trusted folders
  (packages/core/src/skills/skillManager.ts:61-94); `activate_skill` asks for
  confirmation (packages/core/src/tools/activate-skill.ts:72-151). A changed
  extension skill set asks for consent again (consent snapshots in
  `packages/cli/src/config/extensions/__snapshots__/`). Project agents need an
  acknowledgement **bound to a content hash**: a changed file must be
  acknowledged again (packages/core/src/agents/acknowledgedAgents.ts:66-83,
  registry.ts:174-234; test registry_acknowledgement.test.ts).
- **pi**: project resources pass a project-trust gate
  (packages/coding-agent/src/core/resource-loader.ts:541-563). But its
  subagent example lets the **model** pass `confirmProjectAgents: false`
  (examples/extensions/subagent/index.ts:602,671).
- **Codex**: untrusted projects lose `.codex/config.toml`, hooks and exec
  policies (codex-rs/config/src/loader/mod.rs:138-146), but `.agents/skills`
  loads without a trust check (codex-rs/ext/skills/src/host_roots.rs:192+).
- **goose, cline, OpenHands SDK**: no gate. OpenHands repo skills can start MCP
  servers (`mcp_tools`, `.mcp.json`) and run `` !`cmd` ``. Real attacks on
  OpenHands through untrusted content: embracethered.com "ZombAI" and "lethal
  trifecta" write-ups (2025).
- **goose recipes**: `check_for_security_warnings` flags Unicode tag
  characters, i.e. hidden text (goose@80ce943:crates/goose/src/recipe/mod.rs:280).
- **Claude Code**: plugin agents ignore `permissionMode`, `mcpServers`,
  `hooks` and `initialPrompt` (docs). Project frontmatter hooks need folder
  trust; `--trust-workspace false` stops skill hooks since 2.1.319 and 2.1.323.

**Lessons for watane.** watane's design is already stricter than all of them:
skills never come from a disk the agent can write, an offer needs adoption,
and `--restricted` loads nothing from the workspace (V10).
- **Adopt** Gemini's hash binding. **Proposal change (10, Offers):** an
  adoption records the offer's content hash. A changed offer is not used until
  the participant adopts it again; the kernel shows the diff.
- **Adopt** goose's hidden-text check. **Proposal change (10):** the kernel's
  adoption page shows invisible and bidirectional Unicode characters visibly,
  or refuses offers that contain them. Showing content is the consent; hidden
  text defeats it.
- **Avoid** pi's mistake: no trust decision is ever a model-settable
  parameter. Today no tool in 6 or 13 takes one; keep it that way.

## 5. Subagent definitions

**Fields per harness.**

| Harness | Format | Prompt | Tools | Model | Limits | Other |
|---|---|---|---|---|---|---|
| Claude Code | `--agents` JSON, `agents/*.md`, SDK `AgentDefinition` | `prompt` | `tools`, `disallowedTools` | `model` or `inherit` | `maxTurns` | `permissionMode`, `skills`, `mcpServers`, `hooks`, `isolation: worktree`, `background`, `effort` (claude-sdk@69846cd:src/claude_agent_sdk/types.py:108-126; docs) |
| Codex | `[agents.<n>]` + role `*.toml` | developer instructions | can only turn features off | model, effort | none per role | sandbox, approval and MCP in a role are ignored (codex-rs/core/src/agent/role.rs:1-4,76-113) |
| Gemini | `.gemini/agents/*.md`, zod `.strict()` | body | `tools` with wildcards | `model` | `max_turns`, `timeout_mins` | `kind: remote` for A2A (packages/core/src/agents/agentLoader.ts:92-225) |
| opencode | `agents/**/*.md`, JSON | `system` | permission rules, not a list | `model` | `steps` | `mode: subagent\|primary\|all`, `hidden` (packages/schema/src/agent.ts:23-35) |
| goose | `.goose/.claude/.agents/agents/*.md` | body | **ignored** | `model` | max_turns via recipe | only name, description, model read (crates/goose/src/agents/platform_extensions/summon.rs:97-138) |
| OpenHands SDK | `.md` frontmatter | body | `tools` (narrow only) | `model` or inherit | `max_iteration_per_run`, `max_budget_per_run` | `permission_mode`, own `mcp_config` ignored |
| forge | `agents/*.md` | body | `tools` | `model` | `max_turns` (**parsed, not enforced**), `max_requests_per_turn` | handlebars `user_prompt` (forge@a22b492:crates/forge_domain/src/agent.rs:110-168) |
| cline | `.md` frontmatter | body | `tools`, `skills` | provider, model | `maxIterations` | (sdk/packages/core/src/extensions/tools/team/configured-agent-config.ts:8-16) |

**Codex's rule is the one to copy.** Its role file header:
```rust
//! Roles may customize the child or reduce its capabilities, but never replace the parent
//! session's authority.
```
(codex@bb5b9c6:codex-rs/core/src/agent/role.rs:1-4). The child config is
built in a fixed order: parent's effective config, role layer, then the
parent's runtime sandbox and approval policy re-applied on top
(codex-rs/core/src/agent/child_config.rs:115-221). Tested in
`spawn_agent_reapplies_runtime_sandbox_after_role_config`
(codex-rs/core/src/tools/handlers/multi_agents_tests.rs:2268).

**Claude Code's inheritance** (docs): when the parent runs `bypassPermissions`,
`acceptEdits` or `auto`, the subagent runs in the same mode. When the parent
runs `plan`, it stays in plan. When the parent runs `default`, the subagent
uses **its own** `permissionMode`. So a subagent definition with a weaker mode
widens a `manual` session.

**How the SDK passes agents.** The Python SDK no longer uses `--agents`:
```python
# Agents are always sent via initialize request (matching TypeScript SDK)
# No --agents CLI flag needed
```
(claude-sdk@69846cd:src/claude_agent_sdk/_internal/transport/subprocess_cli.py:675-676).
The reason, from its CHANGELOG (:1225, #468): large agent definitions
"would silently fail to register due to platform-specific CLI argument size
limits (ARG_MAX)". Its e2e test sends 20 agents of 12 KB each
(e2e-tests/test_agents_and_settings.py:17).

**Pitfalls.**
- Claude Code `-p` ignored an agent's `tools` and `disallowedTools` before
  2.1.119; MCP specs in `disallowedTools` were ignored until 2.1.178; the
  `Artifact` tool leaked past `disallowedTools` until 2.1.326 (CHANGELOG).
  `--agents` ignored invalid JSON silently until 2.1.243.
- The built-in Plan agent ignored the parent's permissions
  (anthropics/claude-code#10906).
- Codex role files that set sandbox or MCP are silently ignored
  (openai/codex#40130). goose ignores a `tools:` list silently. forge parses
  `max_turns` and never enforces it. **Silently ignored restrictions are the
  most common bug in this group.**

**Lessons for watane.**
- **Adopt** narrow-only as a checked rule, and refuse instead of ignore (G4
  already says refuse per field).
- **Proposal change (4.1, `subagents`):** the grammar lists the allowed keys
  and refuses every other key, naming `permissionMode`, `hooks`,
  `mcpServers`, `isolation`, `background`, `skills` and `initialPrompt` in the
  refusal test. Add `max-turns` (a positive integer, Claude `maxTurns`) so a
  subagent cannot run without a bound (anthropics/claude-code#28148: no
  `max_turns`, a 153 GB output file).
- **Proposal change (4.1):** do not pass subagents in argv. Either put them in
  the host's own `initialize` (the host writes it already, 4.3), or write them
  as `agents/<name>.md` into the host-written plugin folder next to the
  skills. The plugin route has a bonus: Claude Code ignores
  `permissionMode`, `mcpServers` and `hooks` for plugin agents by its own
  rule. Either route needs a new probe (T1). G4 then reads "a flag, a settings
  key or a field of the host's `initialize`".
- **Proposal change (4.1, minimum version):** the minimum `claude` version is
  at least the one that enforces subagent tools in `-p` (2.1.119) and closes
  the `Artifact` leak (2.1.326). The init check (4.3) cannot see subagent
  tools, so a probe must (T2).

## 6. Delegating a task and collecting the result

**Three shapes.**
1. **Blocking tool call.** The parent's tool call runs the child and returns
   its final text. Claude Code foreground `Task`/`Agent`, Gemini
   `invoke_agent`, cline `spawn_agent` (5 min timeout), OpenHands `task`,
   forge, rig `into_tool`. Simple, but the parent's call holds for the child's
   whole run.
2. **Spawn now, result later.** The tool returns an id at once; the result
   arrives later as a message. opencode `subagent` returns
   `{sessionID, status: "running"}` and later posts a
   `<background-result kind="subagent">` with the child's last assistant text
   (opencode@b7f0b90:packages/core/src/tool/plugin/subagent.ts:19-44;
   packages/core/src/session/execution/subagent-report.ts:56-73). rig `task`
   returns an id, the child's answer comes back as a `Report` message, and
   `wait` is refused when two agents would wait on each other
   (rig@0019e24:plugins/rig-basics/src/subagents.rs:1-40,
   plugins/rig-basics/src/subagents/tools.rs:24). goose `delegate` persists a
   child session and returns "Delegated to subagent X"; the result comes back
   as a hidden user message (goose@80ce943:crates/goose/src/agents/platform_extensions/summon.rs:476-547,928-971).
   Claude Code background subagents return a `task_notification` later.
3. **Spawn, then explicit wait.** Codex V1: `spawn_agent`, `send_input`,
   `wait_agent{targets, timeout_ms}`, `close_agent`, `resume_agent`
   (codex-rs/core/src/tools/handlers/multi_agents_spec.rs:166-359,610-647).
   `wait_agent` returns on the first final status or empty on timeout;
   timeout 30 s default, 10 s minimum, 1 h maximum
   (codex-rs/core/src/config/mod.rs:279-289). `close_agent` closes all
   descendants. Codex V2 adds task paths (`/root/task1/task_3`) and a mailbox.

**What returns.** Almost all return the child's last assistant text plus a
status. goose requires a final-output tool call and defaults to
`{summary: string}`; Gemini requires `complete_task`, else
`ERROR_NO_COMPLETE_TASK_CALL` (packages/core/src/agents/local-executor.ts:446-531).
Claude Code stream-json tags child messages with `parent_tool_use_id` and
`agent_id`, and forwards only `tool_use`/`tool_result` blocks unless
`forwardSubagentText` is set in `initialize`
(claude-sdk types.py:2669-2677; query.py:389-390). Task lifecycle lines:
`task_started`, `task_progress`, `task_notification`, `task_updated`
(types.py:1595-1680). cc-acp maps these to ACP child sessions
(cc-acp@a4f0f78:src/acp-subagents.ts:15-25; tests
src/tests/native-subagents.test.ts, subagent-history.test.ts).

**What the child sees.** Gemini, goose, pi, cline and opencode give the child
only its prompt, not the parent's history. Codex copies history only with
`fork_context`/`fork_turns: "all"` and then refuses a different `agent_type`
(child_config.rs:185-191). Zed's `spawn_agent` doc tells the model each task
needs a disjoint write set (zed@10d1b2a:crates/agent/src/tools/spawn_agent_tool.rs:47; GPL, idea only).

**Tests.** Codex: wiremock Responses SSE server (`start_mock_server`,
`mount_sse_once`) and `multi_agents_tests.rs`. opencode: bun with a scripted
`TestLLM` and `tmpdirScoped` (packages/core/test/tool-subagent.test.ts:919,1000
for cancellation). goose: `FakeRunner` with `RunningChild` channels
(crates/oi-kit-agent/tests/subagent.rs). Gemini: vitest with a mocked chat
stream (local-executor.test.ts:786 recursion filter, :1399 missing
`complete_task`). agents-py: a scripted fake model that fails on unexpected or
unconsumed steps (src/agents/testing/model.py).

**Pitfalls.**
- **Asks from a child hang the parent.** opencode#35073, #44747 (a dropped
  permission request left the parent busy forever, stop did nothing), #36868,
  #64032 (a rejected child ask was never reported to the parent).
- **Results go astray.** anthropics/claude-code#75043 and #83599 (child
  completions never reach a subagent parent); a forked skill's result went to
  the wrong agent (fixed 2.1.295); cc-acp notes that a forked skill reuses
  `forked-command-<skill>` as its `parent_tool_use_id` for every run
  (src/native-subagents.ts:686-694).
- **Truncated reports.** anthropics/claude-code#90544: a subagent report cut at
  about 2,500 characters.
- **Turn end is not process end.** With background tasks, stdin must stay open
  until the session is idle (claude-sdk CHANGELOG #1103, #1190). vibe-kanban
  never sees the Claude executor finish, because stream-json keeps the process
  alive after `result` (BloopAI/vibe-kanban#2495; `exit_signal: None` at
  crates/executors/src/executors/claude.rs:714).
- **Resume lost.** OpenHands SDK forgets tasks when its manager closes; pi
  subagents run with `--no-session`; Gemini/Qwen follow-ups failed in
  vibe-kanban (#3500, #3501).

**Lessons for watane.**
- **Adopt shape 2** for the kernel's OpenAI `task` function (11). It is the
  only shape that fits the 10 s call: `task` returns a child session id at once,
  like `agent-tools.call` returns a job (6), and the child's final item arrives
  on the parent's stream. The OpenAI loop then continues when every pending
  function call, task or tool, has its item (S5 already waits for "every job's
  item").
- **Adopt** "the result is the final assistant text, a status and the usage",
  plus the structured output when the profile has `json-schema`. The full
  text, never a cut: the kernel stores it in the transcript.
- **Adopt** cc-acp's mapping. **Proposal change (10, Adapters):** the Claude
  adapter turns `parent_tool_use_id`/`agent_id` lines and `task_*` system lines
  into child entries of the session's transcript, each with its own index, so a
  client sees which subagent said what.
- **Proposal change (4.3):** the host treats a turn as over at `result` **and**
  no running task (the session's idle state), not at the first `result`. It
  keeps stdin open until then.
- **Proposal change (12, S4):** an ask from a child carries the child's id
  and goes to the same owner. A denied or timed-out child ask ends as a
  `deny` item that the parent sees. Test the hang cases (T9).

## 7. Parallel subagents and their limits

| Harness | Depth | Concurrency | Turns or time | Spend |
|---|---|---|---|---|
| Claude Code | 3 by default (`CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`), changed in 2.1.217 and 2.1.219 | 20 (`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`) | `maxTurns` per agent | `--max-budget-usd` per session |
| Codex V1 | `agents.max_depth`, default 1; "Agent depth limit reached. Solve the task yourself." | `max_threads`, default 6; atomic slot reservation (codex-rs/core/src/agent/registry.rs:97-122) | `wait_agent` 10 s to 1 h | shared tree budget (codex-rs/core/src/rollout_budget.rs) |
| Codex V2 | **ignores** `max_depth` (config_toml.rs:816) | 4 minus 1 | | |
| opencode | `subagent_depth`, default 1, walks `parentID` | none (#63303 closed) | none by default (#64733) | none |
| goose | 1: `delegate` is not listed for children, and the call is refused (summon.rs:876) | 5 (`GOOSE_MAX_CONCURRENT_SUBAGENTS`, PR #12946) | max_turns per child | none |
| Gemini | 0: agent tools are filtered from children (local-executor.ts:204-210) | | 30 turns, 10 min, then one 60 s grace turn | |
| pi example | none checked | 8 tasks, 4 at once | | |
| cline | `spawn_agent` is given back to children when enabled (spawn-tool.ts:132-146) | teams: 2 runs | 5 min per spawn | |
| Zed | `MAX_SUBAGENT_DEPTH = 1` (zed@10d1b2a:crates/agent/src/thread.rs:78) | | | |
| forge | tools removed from non-root agents only | unbounded `join_all`, "Agents should not timeout" (crates/forge_app/src/tool_registry.rs:109-130) | `max_turns` not enforced | |
| AutoAgents | | | `max_turns` 10 | `BudgetGuard`: reserve before, settle after, refuse once over (crates/autoagents-guardrails/src/guards/cost_budget.rs:1-30) |

**Pitfalls (runaway subagents).**
- anthropics/claude-code#68619 (infinite recursion), #77361 (877 agents
  despite the depth limit); spawn depth was not restored on resume until
  2.1.187.
- openai/codex#32027 (`max_depth=1` still lets a grandchild spawn), #33447 (V2
  bypasses `max_threads`), #35984 (doc regression on the default), #35463 (a
  week's quota drained overnight), #61406 (54 children, 305 M tokens, no tree
  total), #40995 (a delegated builder ran `shutil.rmtree(".")`).
- opencode#18100 (infinite recursion through the task tool), #25681
  (`task: true` in frontmatter allowed self-spawning), #64733 (about $35 spent
  on subagents with no bound).
- cline: the docs say subagents cannot spawn subagents, but the code gives
  `spawn_agent` back to children by default (from reading, not run).

**The pattern.** Every limit that lived in a prompt, a per-agent tool list or a
second code path (Codex V2) failed at least once. The limits that held are a
counter checked where the spawn happens, plus the tool missing from the child.

**Lessons for watane.**
- **Adopt** goose's double guard for the OpenAI loop: the child's request has
  no `task` function, and the kernel refuses a `task` call from a child
  session anyway (depth counted from the session tree in the kernel's state).
- **Adopt** Codex's atomic slot reservation and tree budget in the kernel: a
  per-session tree cap on live children and a token total across the tree,
  reported in the transcript.
- **Proposal change (4.1, host always adds):** the host sets
  `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` and
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` in the allowlisted environment, from
  a profile field `subagent-limits {depth 1..3, concurrent 1..20}` with
  defaults 1 and 4. The env names come from the docs and need a probe (T4).
- **Proposal change (11):** add `max-turns` to the profile (Claude
  `--max-turns=`, the OpenAI loop's own counter). Both backends can honour it,
  so neither refuses it.

## 8. Handoffs between agents

**openai-agents-python** is the reference. A `handoff(agent, input_filter,
nest_handoff_history, ...)` hands the whole conversation to another agent
(src/agents/handoffs/__init__.py:243-294). By default the new agent sees all
history. `input_filter` can trim it; `nest_handoff_history` (beta, off)
squashes it into a summary, and the docs say that is not redaction.
`Agent.as_tool(...)` is the other option: the child sees only its input
(src/agents/agent.py:646-677). `DEFAULT_MAX_TURNS=10` applies per run, not
across nested `as_tool` calls; there is no depth limit (issue #5453).

Bugs: #4614 (`remove_all_tools(nest_handoff_history(...))` leaks tool data),
#5678 (nesting breaks after a state round trip), #5392 (invalid handoff
arguments end the run).

**Cline's `new_task`** handed a summary (plan, work done, files, next steps) to
a fresh task after the user approved it. It is gone: `/newtask` is now an alias
of `/compact` (cline@20b0427:apps/vscode/src/shared/slashCommands.ts:9-18).
**Codex** has `thread/fork` with `last_turn_id`/`before_turn_id`
(codex-rs/app-server-protocol/src/protocol/v2/thread.rs:571-610).
**vibe-kanban** follows up on Codex by forking the thread, not resuming it
(crates/executors/src/executors/codex.rs:606-631), and on Claude with
`--resume <id> --resume-session-at <uuid>` to rewind (claude.rs:371).

**Lessons for watane.** A handoff moves text that may hold injected
instructions from one agent into another. If the receiver has more reach than
the sender, the injection gains reach.
- **Adopt** the as-tool shape (fresh context, only the prompt) as the default
  for child sessions.
- **Proposal change (10, S7):** `session: {fork: uuid}` and any future
  handoff are allowed only into a profile whose reach (tools, allow rules,
  workspaces, toolsets, mode) is the same or narrower than the source
  session's. The compiler already has narrow-only logic for overlays; reuse it.
- **Avoid** history filters as a security tool; agents-py's own docs say they
  do not redact.

## 9. Recipes and profiles as reusable agent configurations

**goose recipes** are the richest format
(goose@80ce943:crates/goose/src/recipe/mod.rs:44-187): instructions, prompt,
extensions, settings (provider, model, temperature, max_turns), typed
parameters (`required`, `optional`, `user_prompt`; `string`, `number`,
`boolean`, `date`, `file`, `select`), `response.json_schema`, `sub_recipes`,
`retry`. Validation (crates/goose/src/recipe/validate_recipe.rs) caps 32
parameters, 200 select options and 128 KiB of parameters (:15-17); every
template variable must be declared; and a `file` parameter may not have a
default, so a recipe cannot quietly import a user's file (:664). Fixed
sub-recipe values beat what the model passes (summon.rs:71). `retry` runs
shell commands on failure: an injection surface.

**Codex** has `[profiles.<name>]` (model, approval, sandbox, effort, tools) for
a whole session (codex-rs/config/src/profile_toml.rs:25-73), separate from
narrow-only roles. **vibe-kanban** has executor profiles with variants
(`ExecutorProfileId {executor, variant}`,
crates/executors/src/profile.rs:58-62), and every default is "yolo"
(`dangerously_skip_permissions`, `danger-full-access`). **Zed** profiles are a
tool on/off map plus context servers and a default model
(crates/agent_settings/src/agent_profile.rs:104; GPL).

**Lessons for watane.** watane's profile (10, 11) is closer to Codex profiles
than to recipes, and that is right: a profile is authority, set by the
participant only.
- **Adopt** goose's typed parameters later, only if a client needs to fill a
  template. Keep the `file`-without-default rule: a parameter never names a
  path; a workspace comes only from a consent (8).
- **Avoid** a `retry` or "checks" field that runs commands. The factory's
  `check.sh` belongs to a tool mod, not to a profile.
- **Avoid** vibe-kanban's yolo defaults. watane has no bypass mode (4.1);
  keep it.

## 10. Worktree and workspace isolation per agent

**Who isolates.**
- **None of the in-process harnesses isolates subagents.** Codex, opencode,
  goose, OpenHands, cline, rig and forge children share the parent's cwd.
  goose lets a child narrow to a subfolder and rejects paths outside the parent
  (`resolve_working_dir`, summon.rs:1512, tests :2089-2130). pi lets each task
  set a `cwd`. openai/codex#18969 asks for a `cwd` per child so each worker can
  use its own worktree.
- **Claude Code** has `isolation: worktree` for subagents, with a stream of
  bugs: anthropics/claude-code#104008 (kept worktrees committed as gitlinks),
  #99270, #104925, #97799; the guard was escaped by awk, zsh arithmetic and
  locale-prefixed commands (2.1.299, 2.1.309); a worktree was removed while a
  background child still ran in it (2.1.311).
- **Codex** `exec --worktree` makes a detached worktree bound to the thread,
  and refuses removal when it has local changes; it cannot be combined with
  `resume` (codex-rs/worktree/src/lib.rs:62-110,274-285;
  codex-rs/exec/src/lib.rs:339-424).
- **vibe-kanban** has the best worktree manager
  (crates/worktree-manager/src/worktree_manager.rs): a lock per path (:16),
  `ensure_worktree_exists` that checks both the folder and its registration in
  `.git/worktrees` and recreates it (:88,156-224), forced metadata cleanup and
  `git worktree prune` (:230), one retry after cleanup (:303), and a base
  folder of its own "so orphan cleanup never touches user's existing folders"
  (:511). An orphan scan removes folders with no record
  (crates/workspace-manager/src/workspace_manager.rs:538-620). Branches are
  `{prefix}/{short_uuid}-{slug}` (crates/services/src/services/container.rs:790).
  Git-ignored files are copied by glob (crates/local-deployment/src/copy.rs:18,
  tests :125-240). Its tests include a repo that is itself a worktree
  (worktree_manager.rs:573).
- **Containers**: container-use (Dagger) and Sculptor (Imbue) give each agent
  a container plus a branch.

**Pitfalls.** vibe-kanban#1571 (a process kept running after its worktree was
deleted), #765 and #2907 (disk), #2655 (manual branch changes not detected),
#1425 (worktree from the wrong branch), #3255 (repo rename breaks sessions).
watane's factory hit the disk problem too: Colos worktrees had `target/` of
11 to 61 GB (`factory/README.md`, section 1, item 7).

**Lessons for watane.** A mod cannot run git or touch files, so worktrees are a
host effect, like the scratch workspace (4.4).
- **Avoid** Claude Code's `isolation: worktree` and `--worktree`; 4.1 already
  refuses them. Keep that.
- **Proposal change (4.4, new workspace kind):** a **worktree workspace**:
  the origin client asks the host for a worktree of a picked workspace (a git
  repo), at a base ref, on a branch the host names
  (`watane/<session-short-id>`), under `<home>/worktrees/`. The host may clone
  listed git-ignored paths into it (copy-on-write where the file system allows,
  like the factory's `cp -c -R`). It belongs to a session like a scratch
  workspace. Removal kills the session's process group first (vibe-kanban#1571)
  and refuses when the tree has uncommitted changes unless the owner says
  discard (Codex). At open, the host prunes worktrees whose session is gone,
  only under its own base folder. This needs consent like any workspace (8).
- **Port** vibe-kanban's worktree manager and copy logic for this (native host
  code, Apache-2.0).

## 11. Orchestration of several CLI agents (meta-harness), and the factory as mods

**vibe-kanban** is the Rust reference. One trait per agent CLI
(crates/executors/src/executors/mod.rs:311): `spawn`, `spawn_follow_up(session_id,
reset_to_message_id)`, `normalize_logs`, `use_approvals`. The spawned child has
two channels:
```rust
pub struct SpawnedChild { child: AsyncGroupChild,
    exit_signal: Option<oneshot::Receiver<ExecutorExitResult>>,   // executor -> host: done
    cancel: Option<CancellationToken> }                            // host -> executor: stop nicely
```
(mod.rs:420). Claude runs with stream-json both ways and
`--permission-prompt-tool=stdio` (claude.rs:149-197); `can_use_tool` asks are
answered in claude/client.rs:279, and a timed-out ask becomes a deny
(:134-138). **Codex runs as `codex app-server` (JSON-RPC), not `codex exec`**
(codex.rs:451), because app-server has approval requests
(`CommandExecutionRequestApproval`, `FileChangeRequestApproval`,
codex/client.rs:309,344); `codex exec resume` has no approval channel. Other
CLIs go through ACP (crates/executors/src/executors/acp/). Logs are normalized
into one entry type (crates/executors/src/logs/mod.rs:73). Process groups are
stopped with SIGINT, SIGTERM, SIGKILL two seconds apart
(crates/utils/src/process.rs:5). At start, runs left "running" are marked
failed (crates/services/src/services/container.rs:277); pids are not kept, so
orphans are not killed. Its MCP server lets an agent create a workspace,
start a session, prompt it and poll its status
(crates/mcp/src/task_server/tools/task_attempts.rs:94, tools/sessions.rs:255,320).
A fake executor emits Claude JSON for end-to-end tests (`qa_mock.rs`).

**Others.** ccmanager and claude-squad detect "busy / waiting / idle" by
matching terminal output; their own docs call this imperfect. Conductor gives
each workspace a worktree, setup and archive scripts and its own port.
ComposioHQ agent-orchestrator has slots for runtime, agent, workspace, tracker,
SCM and notifier, and re-prompts an agent on events like `ci-failed`: the
closest to the product owner and foreman loop. Zed's `create_thread` tool
starts a sibling thread with `{agent, model, use_new_worktree, base_ref}`
(crates/agent/src/tools/create_thread_tool.rs:61; GPL). Zed's
`agent_servers/src/e2e_tests.rs:20-380` runs one test list against every
external agent: basic, path mentions, tool call, tool call with permission,
cancel, thread drop. **Golem** gives WASM agents durable agent-to-agent calls
(`invoke`, `async-invoke-and-await` with a pollable future, `schedule`), and
warns that A awaiting B while B awaits A deadlocks
(golem-ai@0923caf:test/llm/.wit/wit/deps/golem-rpc/wasm-rpc.wit:215-249;
learn.golem.cloud v1.5 docs).

**What the factory needs, mapped onto the proposal.**

| Factory today | As mods | Status in the proposal |
|---|---|---|
| `claude --bg` foreman per slice, own worktree | A client (or an agent, 13) starts a session with the foreman profile on a worktree workspace | Worktree workspace missing (section 10 above) |
| `SendMessage` to a session by name | `sessions.say(session, text)` | Present (10) |
| `watch.sh` polls for exit or `blocked` | An `ask` item on the owner's stream; an `ended` item | Present and better: structured, with the question text (S4, S6) |
| `codex exec resume <thread>` with feedback | `say` on an idle session; resume after restart (S7) | Present for Claude; **no Codex CLI backend** |
| Sol needs a shell (`cargo test`) | OpenAI loop in the kernel has no shell tools; `tools` is refused for OpenAI (11) | **Gap**: the implementer cannot run on the OpenAI backend as designed |
| Validator, adversary, user-sim as Claude subagents with their own tools and model | Profile `subagents` (in-Claude), or child sessions the foreman starts | In-Claude subagents present; agent-started child sessions need a rule (below) |
| DELIVERED / BLOCKED reports | `json-schema` output on the session's last turn | Present (4.1, 11) |
| `ledger.jsonl` with usage per run | Usage per turn in the transcript; a tree total | Tree total missing (section 7) |

**Lessons for watane.**
- **Adopt** structured "waiting" signals over screen scraping. watane has them
  already (ask items); keep the question text in the item, as S4 does.
- **Adopt** vibe-kanban's two-channel child and its signal ladder for the host's
  process groups (S6), and **fix** what it lacks: keep pgids on disk (4.4 does,
  `<home>/runs/pids`) and kill orphans at open.
- **Proposal change (10, 13, agent-owned sessions):** a profile gains
  `delegates: [profile names]`, the profiles that its session's agent may start
  through the `agent:sessions.*` toolset. The kernel refuses a delegate whose
  reach is wider than the parent's, counts the depth through the session tree,
  caps live children per tree, and sums usage per tree. Cancelling a session
  cancels its children (Codex `close_agent`, pi's durable ownership). Today 13
  gives an agent `agent:sessions.start` with no rule for which profiles it may
  start; its owner identity `agent:<handle-name>` is random per session, so the
  participant cannot grant it profile by profile.
- **Proposal change (13):** add `agent:sessions.wait(session, after,
  timeout)`, run as a job (6): it returns when new items or an ask arrive, or
  at the timeout (10 s to 1 h, like Codex `wait_agent`). Polling `read` alone
  makes an agent busy-loop and spend turns.
- **Open point (15, new N5): a Codex CLI backend.** The factory's implementer is
  Codex with a shell. Either a host part like H2 for `codex app-server`
  (JSON-RPC, approval requests, `thread/start`, `turn/start`, `thread/fork`,
  per vibe-kanban and codex-rs/app-server-protocol/src/protocol/common.rs:580-620),
  or the implementer moves to the Claude shell grant (4.2). That choice is the
  human's: it adds a second native agent to the host.

## 12. Proposal changes, in one list

| # | Section | Change | From |
|---|---|---|---|
| P1 | 4.1 `subagents` | Closed key list; refuse `permissionMode`, `hooks`, `mcpServers`, `isolation`, `background`, `skills`, `initialPrompt`; add `max-turns` | Claude docs, CC#28148, Codex#40130 |
| P2 | 4.1, G4 | Subagents not in argv: the host's `initialize` or `agents/*.md` in the plugin folder | claude-sdk #468 (ARG_MAX) |
| P3 | 4.1 | Minimum `claude` version covers subagent tool enforcement in `-p` (2.1.119) and the `Artifact` leak fix (2.1.326) | CC CHANGELOG |
| P4 | 4.1 | Host settings set `disableSkillShellExecution: true` | Claude docs, OpenHands `` !`cmd` `` |
| P5 | 4.1 | Host env sets subagent depth and concurrency from a profile field `subagent-limits` (defaults 1 and 4) | CC#68619, #77361 |
| P6 | 4.1 skills | Name grammar adds "no leading or trailing `-`, no `--`"; description at most 1024 | Agent Skills spec |
| P7 | 4.3 | Turn over at `result` and no running task; stdin open until idle | claude-sdk #1103/#1190, vibe-kanban#2495 |
| P8 | 4.4 | A worktree workspace kind, host-made, host-pruned, process group killed before removal | vibe-kanban, Codex, Colos disk |
| P9 | 10 Offers | Adoption bound to a content hash; a changed offer needs re-adoption; hidden Unicode shown or refused | Gemini acknowledgedAgents, goose recipe check |
| P10 | 10 Compiler | Refuse duplicate skill names across profile, overlay and offers | Codex dedupe, goose shadowing |
| P11 | 10 Adapters | Claude subagent lines become child entries of the transcript | cc-acp |
| P12 | 10, 11 | OpenAI `task` = spawn now, result later; child has no `task`; kernel counts depth and tree usage | opencode, rig, goose, Codex |
| P13 | 11 | Add `max-turns` to the profile, honoured by both backends | Gemini, Codex, opencode#64733 |
| P14 | 10, 13 | `delegates` per profile; narrow-only; tree caps; cancel cascades | Codex roles, goose, pi durable |
| P15 | 13 | `agent:sessions.wait` long-poll job | Codex `wait_agent` |
| P16 | 10, S7 | A fork or handoff only into a profile with the same or narrower reach | agents-py handoffs |
| P17 | 12 S4 | A child's ask carries the child id; a denied or expired child ask reaches the parent as an item | opencode#44747, #64032 |
| N5 | 15 | Open: a Codex app-server backend for agents that need a shell, or move them to the Claude shell grant | vibe-kanban, factory |

None of these weakens a security check. P2 changes how G4 is worded, and N5
adds a native agent to the host: those two go to the human.

## 13. Who does it best

| Capability | Best | Runner-up | Notes |
|---|---|---|---|
| Skill format | Agent Skills spec + skills-ref | pi (enforces it) | Codex's `allow_implicit_invocation` sidecar is a neat extra |
| Discovery, disclosure | Codex (budgeted renderer, warnings) | goose (wasm32-ready, supporting files listed) | opencode's diff of the list |
| Skill trust | Gemini (folder trust, hash-bound acknowledgement, consent on change) | Claude Code (plugin agents lose risky fields) | goose, cline, OpenHands: none |
| Subagent definitions | Codex roles (narrow-only, re-applied sandbox) | Gemini (`.strict()` schema, turn and time limits) | Codex fails to enforce role sandbox (#40130) |
| Delegate and collect | opencode v2 (async, outbox survives restart, resume by id) | rig (async, refuses deadlocking `wait`) | Gemini's required `complete_task` |
| Parallel, limits | goose (double depth guard, cap 5, cancel token) | Codex V1 (slots, tree budget) | Codex V2, forge, cline: unbounded paths |
| Handoffs | openai-agents-python | Codex `thread/fork` | No Rust harness has handoffs |
| Recipes, profiles | goose recipes (typed, validated) | Codex profiles | Zed profiles (GPL) |
| Workspace isolation | vibe-kanban worktree manager | Codex `exec --worktree` | Claude `isolation: worktree` is buggy |
| Meta-harness | vibe-kanban (Rust, executors, approvals relay, MCP) | agent-orchestrator (event reactions) | Screen-scraping tools are fragile |

## 14. Test cases watane should write

Probes (rerun after each `claude` update, in `proto/claude-probes/`):
- **T1** Subagents from the host's `initialize` and from `agents/*.md` in the
  plugin folder both register; 20 agents of 12 KB each register (the
  claude-sdk e2e case).
- **T2** A subagent with `tools: [Read]` cannot `Write`, in `-p` under
  `--restricted`; a subagent definition carrying `permissionMode:
  bypassPermissions` (if it ever got through) does not widen a `manual` parent.
- **T3** `disableSkillShellExecution: true` stops `` !`cmd` `` in a plugin
  skill.
- **T4** `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=1` stops a subagent from
  starting another; the concurrency variable caps parallel `Task` calls.
- **T5** With a background subagent running, the host sees `result`, keeps
  stdin open, and ends the turn only at idle.

Unit and integration (host, kernel):
- **T6** Launch grammar: every refused `subagents` key is refused by name;
  skill names `-a`, `a-`, `a--b`, `A`, 65 characters are refused; a 1025-character
  description is refused; `--agents`-style JSON in a value cannot become a
  flag.
- **T7** Compiler: two skills with one name (profile and offer) are refused; a
  profile whose skill descriptions exceed the listing budget is reported.
- **T8** OpenAI `skill` listing escapes `</description>` and `<` in names and
  descriptions (opencode's bug).
- **T9** A child's ask: answered by the owner; denied; never answered (denied
  after the timeout, and the parent gets an item); the parent cancelled while
  the ask waits (no hang; opencode#44747, #64032).
- **T10** Depth: an OpenAI child's request has no `task` function, and a forged
  `task` call from a child is refused with a clear item (goose double guard).
- **T11** Concurrency: N+1 parallel `task` calls start N; the last waits or is
  refused, never dropped (goose `subagent.rs:184`, Codex slot test).
- **T12** Cancel cascades: cancelling a parent ends every child stream and
  kills each child's process group; a restart then reports children as ended,
  not running (pi #8250, vibe-kanban start cleanup).
- **T13** Tree usage: usage of all children adds up in the parent's transcript
  (Codex#61406).
- **T14** Narrow-only: a `delegates` profile, a fork, or a resume into a profile
  with an extra tool, allow rule, workspace, toolset or weaker mode is refused.
- **T15** Offer change: an adopted offer whose content changes is not used until
  re-adopted; an offer with a Unicode tag character or a bidi override is shown
  flagged or refused.
- **T16** Worktree workspace: create twice at once (one wins, one waits);
  remove with a running session (process killed first); remove with
  uncommitted changes (refused); prune at open touches only `<home>/worktrees/`;
  a repo that is itself a worktree works (vibe-kanban :573).
- **T17** Result: a child's 50 KB final text reaches the parent whole
  (claude-code#90544); a profile with `json-schema` returns the parsed object.
- **T18** Adapter: a recorded stream-json with `parent_tool_use_id`, `agent_id`
  and `task_*` lines becomes child transcript entries; a forked-skill id that
  repeats does not merge two runs (cc-acp). Use vibe-kanban's real
  `can_use_tool` JSON as a fixture (claude.rs:3286).
- **T19** One contract list for every backend, after Zed's e2e list: basic turn,
  tool call, tool call with ask, cancel, session drop, resume.

Fixture style to copy: Codex's wiremock Responses SSE server for the OpenAI
loop; a scripted fake model that fails on unexpected or unconsumed steps
(agents-py `testing/model.py`, Gemini, opencode `TestLLM`); a fake `claude`
that replays recorded stream-json (vibe-kanban `qa_mock.rs`; watane's own
"fake `claude` in dev", 8).

## 15. Reusable code

| What | Path | License | wasm32-wasip2 | Use in watane |
|---|---|---|---|---|
| Skill validator and listing | agentskills@83021e2:skills-ref/src/skills_ref/{validator,parser,prompt}.py | Apache-2.0 | port (Python, about 1000 lines) | kernel: offer validation, OpenAI listing |
| Skill discovery, parse, listing | rig@0019e24:plugins/rig-basics/src/skills.rs:123-397 | MIT | parse and listing yes (std, serde, yaml); discovery needs a file system | kernel: parse and list only |
| Skills, `load_skill`, arguments | goose@80ce943:crates/oi-kit-agent/src/skills/ | Apache-2.0 | yes except discovery (mod.rs:5) | kernel |
| Concurrency-capped subagent step | goose@80ce943:crates/oi-kit-agent/src/subagent.rs | Apache-2.0 | reportedly wasm32-friendly; check deps | kernel: OpenAI `task` |
| Recipe parameter validation | goose@80ce943:crates/goose/src/recipe/validate_recipe.rs | Apache-2.0 | port the functions (crate uses tokio) | kernel, if profiles get parameters |
| Budgeted skill catalog | codex@bb5b9c6:codex-rs/ext/skills/src/render.rs | Apache-2.0 | likely; check deps | kernel budget check |
| Child config order, slot registry | codex@bb5b9c6:codex-rs/core/src/agent/{child_config,registry}.rs | Apache-2.0 | no (tokio); port logic | kernel |
| Depth walk, report outbox | opencode@b7f0b90:packages/core/src/session/{subagent-depth,execution/subagent-report}.ts | MIT | port (TS) | kernel |
| Hash-bound acknowledgement | gemini@d4cb171:packages/core/src/agents/acknowledgedAgents.ts | Apache-2.0 | port (TS) | kernel offers |
| Executor loop with `complete_task` and grace turn | gemini@d4cb171:packages/core/src/agents/local-executor.ts:109,446-531 | Apache-2.0 | port (TS) | kernel OpenAI loop |
| Subagent lines to child sessions | cc-acp@a4f0f78:src/acp-subagents.ts, src/native-subagents.ts | Apache-2.0 | port (TS) | kernel adapter |
| Cost guard | autoagents@d1f8cc5:crates/autoagents-guardrails/src/guards/cost_budget.rs | MIT OR Apache-2.0 | likely yes | kernel tree budget (tokens, not USD) |
| Continue after a cut stream | golem-ai@0923caf:llm/llm/src/durability.rs | Apache-2.0 | targets wasip3; port logic | kernel OpenAI loop |
| Agent RPC shape | golem-ai@0923caf:test/llm/.wit/wit/deps/golem-rpc/wasm-rpc.wit:215-249 | Apache-2.0 | WIT | reference for kernel and host interfaces |
| Task graph with pause and resume | swiftide@92e1a30:swiftide-tasks/ | MIT | swiftide-tasks yes, but swiftide-core pulls tokio "full": vendor | maybe, for a factory mod |
| Worktree manager | vibe-kanban@3f095f0:crates/worktree-manager/src/worktree_manager.rs | Apache-2.0 | no (native host, tokio, git) | host: worktree workspace (P8) |
| Copy ignored files by glob | vibe-kanban@3f095f0:crates/local-deployment/src/copy.rs | Apache-2.0 | no (native) | host |
| Process group stop ladder | vibe-kanban@3f095f0:crates/utils/src/process.rs:5 | Apache-2.0 | no (native, command-group, tokio) | host S6 |
| Approvals trait with timeout | vibe-kanban@3f095f0:crates/executors/src/approvals.rs:31 | Apache-2.0 | shape only | host and kernel asks |
| Normalized entry types | vibe-kanban@3f095f0:crates/executors/src/logs/mod.rs:73 | Apache-2.0 | yes (types) | kernel transcript, next to ACP |
| Agent file format | forge@a22b492:crates/forge_repo/src/agent.rs:177, fixtures/agents/advanced.md | Apache-2.0 | parsing yes (gray_matter); rest tokio | reference only |
| Zed `spawn_agent`, `create_thread`, profiles, e2e list | zed@10d1b2a:crates/agent, crates/agent_servers | GPL-3.0-or-later | not usable | ideas only |

## 16. Gaps in this research

- Issue searches were rate-limited; no reverted PRs were found for any repo.
- The Claude Code environment variables for subagent depth and concurrency,
  `disableSkillShellExecution`, and the plugin-agent field rules come from the
  docs read on 2026-11-20, not from probes. T1 to T5 check them.
- Claude Code CHANGELOG version numbers (2.1.119, 2.1.326 and the others)
  were read from the live CHANGELOG once; check them again before fixing the
  minimum version (P3).
- The OpenHands SDK was read on the web only; its line numbers are approximate.
- cline's unbounded nested `spawn_agent` is from reading the code, not from a
  run.
- Claude Code docs and its CHANGELOG disagree on a per-session spawn cap (200
  since 2.1.212 in the CHANGELOG; "no total limit" on the docs page).
