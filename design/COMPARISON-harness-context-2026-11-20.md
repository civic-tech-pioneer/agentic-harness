# Context assembly in other agent harnesses (2026-11-20)

A record, not a contract. It compares how other harnesses build what the
model sees, so watane's kernel (`ARCHITECTURE-PROPOSAL.md` 10) and the launch
document (4.1) do not reinvent it. Group 2 of the harness study: system prompt,
project instructions, memory, compaction, prompt caching, reasoning items, and
attachments.

## Sources

Shallow clones in the session scratchpad (`research/<repo>`), read 2026-11-20.
Citations are `repo path:line` at the commit below.

| Repo | Commit | Licence | Language |
|---|---|---|---|
| openai/codex (`codex-rs`) | bb5b9c6 | Apache-2.0 | Rust |
| sst/opencode | b7f0b90 | MIT | TS |
| block/goose | 80ce943 | Apache-2.0 | Rust |
| badlogic/pi-mono | e5d93ea | MIT | TS |
| All-Hands-AI/OpenHands (now a frontend only) | 3df9648 | MIT | TS |
| OpenHands/software-agent-sdk (the agent core) | d5b327b | MIT | Python |
| google-gemini/gemini-cli | d4cb171 | Apache-2.0 | TS |
| cline/cline (new `sdk/`) | 20b0427 | Apache-2.0 | TS |
| 0xPlaygrounds/rig | 0019e24 | MIT | Rust |
| golemcloud/golem-ai | 0923caf | Apache-2.0 | Rust |
| zed-industries/zed (`crates/agent`, `prompt_store`) | 10d1b2a | **GPL-3.0-or-later** | Rust |
| antinomyhq/forge | a22b492 | Apache-2.0 | Rust |
| bosun-ai/swiftide (`swiftide-agents`) | 92e1a30 | MIT | Rust |
| liquidos-ai/AutoAgents | d1f8cc5 | MIT OR Apache-2.0 | Rust |
| zed-industries/claude-code-acp | a4f0f78 | Apache-2.0 | TS |
| agentclientprotocol/agent-client-protocol | 65ee6ae | Apache-2.0 | Rust |

Web: OpenAI compaction guide (developers.openai.com/api/docs/guides/compaction),
Claude Agent SDK commands page (code.claude.com/docs/en/agent-sdk/slash-commands),
codex issues #5957, #6022, #8759.

**Licences.** watane's crates are `MIT OR Apache-2.0`, and `deny.toml` allows
both. Code copied from an MIT source can stay dual-licensed. Code ported from
an Apache-2.0-only source makes that file Apache-2.0 only, and needs the
source's notice kept. Prefer MIT sources when two are equal. Zed's agent
crates are GPL: read for design only, never copy.

**Two backends, two depths.** For Claude, `claude` assembles the context
itself; watane controls only what the launch document lets through (4.1), and
`--restricted` loads no workspace `CLAUDE.md` (18, V10). For OpenAI, the
kernel builds every request itself (10, OpenAI loop). So most lessons below
land in the kernel's OpenAI loop; a few land in the launch document and in
the host's stdin filter (4.3).

---

## 1. System prompt composition

**watane needs:** one profile prompt that compiles to `--system-prompt` /
`--append-system-prompt` for Claude and to `instructions` for OpenAI, stable
in bytes from turn to turn.

### Per harness

- **Codex.** The base prompt is per model (`prompts/src/model_instructions.rs`,
  model catalog `instructions_template`). Everything else is a typed
  *context fragment* sent as a `developer` or `user` message, not as
  instructions: environment, AGENTS.md, permissions, memory
  (`core/src/context/*.rs`, `context-fragments/`). Each fragment has start and
  end markers so it can be found again in history:
  ```rust
  fn type_markers() -> (&'static str, &'static str) {
      ("# AGENTS.md instructions", "</INSTRUCTIONS>")   // user_instructions.rs:31
  }
  ```
  **World state** (`core/src/context/world_state/mod.rs:361-420`): each section
  has a stable id and a JSON snapshot. Next turn it renders only the change
  (an RFC 7386 merge patch decides; `:535`). The environment changed? One new
  `<environment_context>` item. Nothing changed? Nothing is sent.
  Tests: insta snapshots of every `previous -> current` case
  (`world_state/snapshots/*agents_md*.snap`), e.g. removal renders "The
  previously provided AGENTS.md instructions no longer apply."
- **OpenCode.** One prompt file per model family, picked by id substring
  (`packages/core/src/plugin/optimize.ts`: `gpt.txt`, `anthropic.txt`
  appended, `kimi.txt`, ...). Instructions are keyed state with
  initial/changed/removed renderers; a change is sent as a unified diff when
  the diff is shorter than the full text (`instruction-discovery.ts:140-165`).
  The rendered update is frozen into a durable event
  (`session/instruction-state.ts:41-60`).
- **Pi.** Plain string sections, each wrapped in `<name>…</name>`
  (`coding-agent/src/core/system-prompt.ts:128-192`). Only `cwd`, no date
  or OS. Mid-session changes are appended as a *system message patch*
  (`sections`, `toolsAdded`, `toolsRemoved`; `packages/ai/src/types.ts:560-585`)
  so the cached prefix is never rewritten.
- **Goose.** One minijinja template `prompts/system.md`; extension
  instructions sorted by name "for multi session prompt caching"
  (`agents/prompt_manager.rs:114`). Date cut to the hour for the same reason
  (`prompt_manager.rs:194-196`). Volatile facts (time, cwd, turn budget) go
  in a per-turn `<turn-context>` user block, not in the prompt
  (`agents/moim.rs:9-23`). All text passes `sanitize_unicode_tags` (NFC, then
  strips Unicode tag characters; `utils.rs:38`). Snapshot tests:
  `agents/snapshots/goose__agents__prompt_manager__tests__*.snap`.
- **OpenHands SDK.** Sections carry a `CacheTier`: `STATIC` (same for every
  conversation) or `DYNAMIC` (datetime, repo context, skills, suffix); they
  map onto two system content blocks
  (`openhands-sdk/openhands/sdk/context/prompts/section.py:25-31`,
  `sections/dynamic.py`).
- **Gemini CLI.** `snippets.ts` vs `snippets.legacy.ts` by model generation
  (`prompts/promptProvider.ts:81`). Each section can be turned off with
  `GEMINI_PROMPT_<KEY>`. Environment (date, OS, tree) goes in the **first user
  message** as `<session_context>`, not the system prompt
  (`utils/environmentContext.ts:55-111`). Snapshot test:
  `core/src/core/__snapshots__/prompts.test.ts.snap`.
- **Cline (sdk).** One template with `{{CWD}}`, `{{CURRENT_DATE}}`,
  `{{CLINE_RULES}}` (`sdk/packages/shared/src/prompt/cline.ts:152-213`). No
  per-model variants at this commit. Tools only as native schemas.
- **Zed (GPL).** Handlebars `system_prompt.hbs` with a `ProjectContext`
  (worktrees, rules files). Design reference only.
- **Forge.** Handlebars templates (`templates/forge-partial-system-info.md`)
  and insta snapshots of the whole prompt
  (`forge_app/src/orch_spec/snapshots/*system_prompt*.snap`).
- **rig, swiftide, AutoAgents, golem-ai.** A `preamble` string or a system
  message. No environment block, no variants.

**Convergence.** Tool schemas go in the API `tools` field everywhere; none
repeats them in prose (Gemini lists names only in plan mode). Every mature
harness keeps the system prompt fixed for the session and moves change into
appended messages.

### Pitfalls

- Date with seconds or minutes in the prompt breaks the cache every turn
  (goose's hour cut; pi leaves the date out).
- Unordered tool lists or map keys move the prefix (goose sorts; rig tests it,
  section 5).
- OpenHands frontend: a skill deny-list "puts every future addition into
  every system prompt" (OpenHands#16302, `agent-server-adapter.ts:930`).
  They moved to an allow-list.

### Lessons for watane

- **Adopt** a two-part prompt for the OpenAI loop: `instructions` = the
  profile's text only (static, byte-stable). Session facts (workspace name,
  date) go in a first `developer`/`user` item, Codex-style, never in
  `instructions`. Proposal 11 maps `system-prompt` to `instructions`; keep it
  that way and add nothing volatile.
- **Adopt** Codex's typed fragment with markers for every kernel-injected
  block, so compaction and diffing can find and replace it.
- **Adopt** goose's `sanitize_unicode_tags` on every text that a contributor
  or a workspace supplies (adopted offers, skills, AGENTS.md). Small, pure,
  wasm-safe.
- **Avoid** per-model prompt files in the kernel. The profile owns the prompt
  (G3); a model variant is a second profile.

---

## 2. Project instructions (AGENTS.md, CLAUDE.md, GEMINI.md)

**watane needs:** a decision on whether a picked workspace's instruction files
reach the agent at all, and if so how, with the same result on both backends.

### Per harness

| Harness | Files | Discovery | Placement | Limit |
|---|---|---|---|---|
| Codex | `AGENTS.override.md`, `AGENTS.md`, configured fallbacks | project root (`.git` marker) down to cwd, concatenated in that order; never above the root; global `~/.codex/AGENTS.md` first (`core/src/agents_md.rs:1-18`) | `user` message `# AGENTS.md instructions for <dir>` | 32 KiB total, then truncated with a warning (`agents_md.rs:132-195`, `config/mod.rs:278`) |
| OpenCode | `AGENTS.md`; `CLAUDE.md` only where no AGENTS.md (`instruction-discovery.ts:127-136`) | global, then nearest to farthest | keyed instruction state; diffs on change | none found |
| Pi | `AGENTS.override.md`, `AGENTS.md`, `CLAUDE.md` (one per dir) | global, then root → cwd; no subdirs (`resource-loader.ts:214-303`) | system prompt `<project_instructions path=…>` | none |
| Goose | `.goosehints`, `AGENTS.md` (configurable) | global, git root → cwd; **subdirectories lazily** when a tool touches a path there (`hints/load_hints.rs:26-110`) | system prompt "Additional Instructions" | `@file` imports: depth 3, 64 references, 1 MiB (`hints/import_files.rs:14-17`) |
| Gemini CLI | `GEMINI.md` (+ configured names) | global; walk up to the git root; subdirectories JIT after read/ls/edit (`utils/memoryDiscovery.ts:405-648`, `tools/jit-context.ts`) | three tiers: global in system; project in first user message; JIT inside tool output | `@path` imports depth 5 |
| Cline | `AGENTS.md`, `.clinerules`, `.cline/rules` | workspace root only; no walk (`shared/src/storage/paths.ts:529-600`) | system prompt `# Rules`, sorted by name | none |
| Zed (GPL) | `.rules`, `.cursorrules`, …, `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, first one wins (`prompt_store/src/prompts.rs:22-32`) | worktree root | system prompt | – |
| Forge | `AGENTS.md` | base dir, git root, cwd (`forge_services/src/instructions.rs:6-10`) | system prompt custom rules | – |
| OpenHands SDK | `AGENTS.md` as an always-on `Skill`; keyword-triggered skills | SDK side | system `DYNAMIC` block | – |

**Change during a session.** Codex, OpenCode and pi re-read the files and
send a delta message ("These AGENTS.md instructions replace all previously
provided…"). Gemini and goose append newly found subdirectory files to the
tool result that found them.

### Tests

- Codex: `core/tests/suite/agents_md.rs`, `agents_md_refresh.rs`
  (wiremock Responses server, asserts the request body).
- OpenCode: `packages/core/test/instruction-discovery.test.ts`,
  `instruction-state.test.ts`.
- Gemini: `utils/memoryDiscovery.test.ts`, `memoryImportProcessor.test.ts`.

### Pitfalls

- Instructions lost or ignored after compaction: codex #5957 (open, "forgets
  it is mid-task"), #6022 (closed, ignores AGENTS.md rules). The fix in every
  mature harness is the same: instructions are never part of the summary and
  are re-sent fresh after it (section 4).
- Global file not found by default: codex #8759.
- Cline: rules dropped when a host hard-coded one layout (#14186); global
  rules missed on WSL and OneDrive (`paths.ts:546-580`).
- Codex refuses fallback names with path syntax, because "resolving a Windows
  network path can send ambient credentials even during metadata probes"
  (`agents_md.rs:290-296`).

### Lessons for watane

- **Today the Claude path loads none** (V10, `--restricted`), and the kernel
  cannot read a workspace. That is a safe default: a repository's AGENTS.md
  is untrusted text that steers the agent.
- **Proposal change (4.1, 10, 11).** Add an opt-in profile field
  `project-instructions: bool`. The host reads `AGENTS.md` (then `CLAUDE.md`
  where none) from the workspace root down to the session's folder, Codex
  rules, 32 KiB in all, symlinks resolved inside the workspace only. For
  Claude it goes out as part of `--append-system-prompt=`; for OpenAI the
  kernel receives the text and sends it as a marked `user` fragment (Codex
  placement: below the profile's own `instructions` in authority). The
  participant sees in the session view which files were read.
- **Avoid** `@file` imports in instruction files (goose, Gemini): they turn a
  workspace file into a reader of other files.
- **Avoid** JIT subdirectory discovery for now; it needs the host to watch
  tool paths, and doubles the injection surface.

---

## 3. Memory across sessions

**watane needs:** a position. Persistent notes are state the agent writes and
a later session reads; that is a channel across sessions and clients (G6).

### Per harness

- **Codex.** A two-phase background pipeline (`memories/README.md`). Phase 1
  extracts a `raw_memory` and `rollout_summary` from idle past sessions with a
  model, redacting secrets. Phase 2 consolidates into `~/.codex/memories/`
  (a git-tracked folder) with a sub-agent that has no network and no
  approvals. The read side is a bounded developer fragment, "capped below 10k
  tokens" (`core/src/context/memory.rs:10`).
- **Gemini CLI.** No memory tool any more: the prompt tells the model to edit
  markdown files, with four tiers and routing rules (`prompts/snippets.ts:866-880`).
  A private `MEMORY.md` index lives in the project temp dir. An experimental
  extraction agent mines idle sessions (`services/memoryService.ts`).
- **OpenHands SDK.** Two `MEMORY.md` indexes (user, project), 6000 characters,
  the top lines dropped first; daily logs are not loaded
  (`context/memory.py:1-40`).
- **Goose.** An MCP extension with `remember_memory`, `retrieve_memories`,
  by category, local or global (`goose-mcp/src/memory/mod.rs`).
- **Cline.** A convention only ("memory bank" files kept by a rule).
- **Pi, OpenCode, Forge, Zed.** None found (sessions resume, nothing more).
- **AutoAgents, rig.** "Memory" means the conversation window
  (`autoagents-core/src/agent/memory/sliding_window.rs`, `rig-memory`).
- **Claude.** Claude Code has its own memory files and auto memory; under
  `--restricted` the workspace's are not loaded (V10). User-level memory was
  not probed.

### Lessons for watane

- **Avoid** automatic memory in the first version. An extraction agent that
  reads past transcripts mixes sessions of different clients (G6, G7).
- **Adopt** the simple shape when it comes: an index file of at most a few KB,
  loaded as one marked fragment, written only through a tool call the broker
  sees. In watane that is a function in a toolset (proposal 6), owned by a
  contributor mod, scoped to one profile and one workspace.
- **Check (proposal 18).** Add a probe: does `--restricted` stop Claude Code
  from reading and writing `~/.claude/projects/<cwd>/memory` and
  `~/.claude/CLAUDE.md`? If not, the host needs a setting that turns auto
  memory off, or a session leaks into the participant's own Claude use.

---

## 4. Context window management and compaction

**watane needs:** for OpenAI, a compaction the kernel runs in short calls,
that keeps tool pairs and reasoning valid, re-sends instructions, and never
loses the transcript. For Claude, a way to see and ask for `claude`'s own
compaction.

### Trigger

| Harness | When |
|---|---|
| Codex | 90 % of the window, or a configured lower limit (`protocol/src/openai_models.rs:584-596`); also before sampling when the model changes to a smaller window; on `ContextWindowExceeded` while compacting, drop the oldest item and retry, "to preserve cache (prefix-based)" (`core/src/compact.rs:359-366`) |
| OpenCode | window minus max(16k, 15 %); a background summary starts 60k tokens early, and only while ≥30 s of the prompt cache lifetime is left (`core/src/session/compaction.ts:124-168`) |
| Pi | `tokens > window − 16384`; also on an overflow error (compact, retry once) and on a "silent overflow" (`compaction/compaction.ts:126-130, 267`; `agent-session.ts:3121-3255`) |
| Goose | 80 %, capped at 225k tokens "below OpenAI's long-context price tier" (`context_mgmt/mod.rs:27-57`) |
| Gemini CLI | 50 % (`context/chatCompressionService.ts:45`) |
| Cline | 90 % of 90 % of the window; skipped when the window is unknown (`compaction-shared.ts:15-37`) |
| Zed (GPL) | only for windows ≥ 80k tokens (`agent/src/thread.rs:121-124`) |
| Forge | min(token count, % of window), or turn or message count (`forge_domain/src/compact/compact_config.rs`) |
| OpenHands SDK | more than 240 events, or a token cap (`condenser/llm_summarizing_condenser.py:64-67`) |
| claude-code-acp | its own "idle compaction": `/compact` at 54 min idle, so the next prompt does not rewrite a whole expired 1 h cache (`src/idle-compaction.ts:29-62`) |

### What is kept

- **Codex.** The newest real user messages up to 20k tokens, then the summary
  as a user message with a fixed prefix, then the *current* initial context
  (instructions, environment) inserted before the last user message
  (`compact.rs:69, 670-858`). Tool outputs are not kept, except ones marked
  retained.
- **Pi** (and its Rust port in rig-memory). Walk back about 20k tokens; cut
  only at a user, assistant or summary message, **never at a tool result**. A
  cut inside a turn makes a separate "turn prefix" summary
  (`compaction.ts:351-406, 929-1019`). Read and changed file lists are
  carried from summary to summary.
- **Gemini CLI.** Newest 30 % by characters; split only at a user turn that
  is not a function response (`chatCompressionService.ts:367-418`).
- **Cline.** ≥20k recent tokens; never cut past the latest typed user prompt.
- **Forge.** No model call: a Handlebars "summary frame" lists each compacted
  message's text and each tool call as one line (`**Read:** path`,
  `**Execute:** cmd`; `templates/forge-partial-summary-frame.md`). The end
  index steps back so a tool call is never split from its result
  (`forge_domain/src/compact/strategy.rs:78-140`).
- **Goose.** Separately, old tool pairs are summarised ten at a time, never
  the current turn's (`context_mgmt/mod.rs:229-266`).
- **Swiftide.** A `ChatMessage::Summary` in an append-only history; the view
  drops everything before the last summary except system messages
  (`swiftide-agents/src/default_context.rs:258-275`).

### The summary prompt

- Codex: 9 lines, "CONTEXT CHECKPOINT COMPACTION … handoff summary for
  another LLM" (`prompts/templates/compact/prompt.md`).
- Pi, Cline: sections Goal / Constraints / Progress (Done, In Progress,
  Blocked) / Decisions / Next / Files. Update mode merges into the previous
  summary.
- OpenCode: Objective / Requirements / Decisions / Work State / Next Move /
  Relevant Files (≤15) / Important Context, and: "Leave out … instruction
  files such as AGENTS.md … The next agent receives current versions of all of
  these separately" (`compaction.ts:170-243`).
- Goose: a JSON schema rendered back to markdown by a template; quote errors
  verbatim (`oi-kit-context-management/src/prompts/compaction.md`).
- Gemini: an XML `<state_snapshot>`, an anti-injection rule, then a second
  "probe" call that critiques and regenerates it (`chatCompressionService.ts:774-801`).

### Tool output pruning (before or instead of a summary)

- Codex: every tool output is truncated when recorded, with a header
  `Warning: truncated output (original token count: N)`
  (`utils/output-truncation/src/lib.rs:26-50`).
- Gemini: the newest 50k tool tokens are kept; older outputs go to a file and
  become a 250+250 character head and tail (`toolOutputMaskingService.ts`).
- Cline: a later read of the same file turns older ones into
  `[outdated - see the latest file content]`, but only in batches of ≥64 KB,
  to keep the cache (`session/services/message-builder.ts:29-66`).
- Goose: summariser input loses tool responses from the middle outwards, in
  steps 0, 10, 20, 50, 100 % (`oi-kit-context-management/src/summarize.rs:13, 35-170`).
- OpenCode: five levels per exchange: whole; outputs cut; outputs removed;
  user text only; omitted (`compaction.ts:141-147`).

### Server-side compaction (OpenAI)

The Responses API compacts on the server: `context_management` with
`compact_threshold` on a normal request, or a `{"type":"compaction_trigger"}`
input item, or `POST /responses/compact`. The output is an opaque `compaction`
item; with stateless chaining the client appends it and may drop everything
before it. It works with `store: false` (OpenAI guide). Codex uses it for
OpenAI providers (`ResponseItem::Compaction { encrypted_content }`,
`protocol/src/models.rs:1398-1424`; `core/src/compact_remote_v2.rs`); OpenCode
implements `/responses/compact` (`packages/ai/src/protocols/utils/responses-compaction.ts`).
Not known: whether Sign in with ChatGPT plan tokens may use it.

### Claude

`claude` compacts by itself. The stream shows a `system` line with subtype
`compact_boundary` and `compact_metadata` (`trigger`, `pre_tokens`,
`post_tokens`). Sending `/compact` as a user message runs it (Agent SDK
docs). claude-code-acp maps both to ACP compaction updates
(`src/v1/acp-agent.ts:4855-4900`). ACP v1 has stable
`CompactionUpdate` and `CompactionSummaryChunk` session updates
(`agent-client-protocol-schema/src/v1/client.rs:146-155`), behind the client
capability `session.compaction`.

### Tests

- Codex: `core/tests/suite/compact.rs` (wiremock SSE helpers
  `ev_completed_with_tokens`, `ev_reasoning_item` in `core/tests/common/responses.rs:799-968`):
  `auto_compact_runs_after_token_limit_hit`,
  `additional_context_survives_compaction`, `failed_compaction_preserves_history_after_restart`,
  `pre_sampling_compact_runs_on_switch_to_smaller_context_model`;
  readable request snapshots in `core/tests/common/context_snapshot.rs`.
- Pi: a scripted *faux provider* (`packages/ai/src/providers/faux.ts`), pure
  tests over `fixtures/large-session.jsonl` (`coding-agent/test/compaction.test.ts`).
- Goose: an `OverflowingModel` that fails until enough tool responses are
  dropped (`summarize.rs` tests).
- Gemini: canned `.responses` files for the whole CLI
  (`integration-tests/context-compress-interactive.{compress,compress-empty,compress-failure}.responses`).
- Forge: insta snapshots of the summary frame; a test that only the last
  reasoning survives.
- claude-code-acp: `src/tests/context-compaction.test.ts`,
  `tests/v1/idle-compaction-agent.test.ts`.

### Pitfalls (from changelogs and comments)

- **Token estimates.** Cline: after the first compaction the estimate was 7×
  too high, so it compacted every few tool calls; images counted as text; the
  character estimate undercounted dense content. All now prefer the
  provider's reported usage. Pi: stale pre-compaction usage re-triggered
  compaction (`agent-session.ts:3137-3145`); providers without usage never
  compacted (CHANGELOG :1207).
- **False overflow.** Pi compacted at 20 % of the window on a misread
  OpenRouter error (#10497). Pi keeps ~29 provider regexes plus exclusions for
  rate limits (`packages/ai/src/utils/overflow.ts:39-85`).
- **Summary failures.** Reasoning models spent the summary budget and returned
  nothing (Cline); summaries were truncated at the token cap and stored (pi
  :1194, :370); a summary larger than the input (Gemini
  `COMPRESSION_FAILED_INFLATED_TOKEN_COUNT`, then truncation only).
- **Summary bloat.** OpenCode's old template restated AGENTS.md and grew an
  "Additional Context" section on every update; they rewrote it (#48058
  comment, `compaction.ts:227-231`).
- **Lost state.** Codex #5957: the model forgot it was mid-task. Pi: extension
  handlers dropped prompt and tools after compaction (#9789); forks lost the
  boundary (:1068).
- **Reasoning after compaction.** Pi: OpenAI rejected the next request with a
  prefix mismatch because reasoning items were dropped (CHANGELOG :25). Forge
  keeps only the *last* reasoning block and moves it to the first remaining
  assistant message, "preventing exponential accumulation"
  (`forge_app/src/compact.rs:109-170`).
- **Usage display.** claude-code-acp: after compaction the client showed the
  old size; it now reads `post_tokens` from the boundary.

### Lessons for watane

- **Adopt** an append-only transcript where compaction is one more item
  (swiftide, pi, Cline, Codex all converge). The request is a *projection*:
  last compaction item, the kept tail, fresh instructions. Proposal 10 already
  keeps "a transcript with an index per entry"; add "a compaction entry; the
  request starts at the last one". `attach` replay stays complete.
- **Adopt** the cut rule: never between a function call and its output, never
  past the latest user message (pi, Forge, Cline). Port pi's cut and prompts
  through rig-memory (MIT, Rust, pure).
- **Adopt** "instructions are not summarised; they are re-sent" (OpenCode
  prompt line, Codex re-injection). This is the fix for codex #5957/#6022.
- **Adopt** truncation at record time for every agent-tool result, with
  Codex's header, so one result cannot fill the window.
- **Prefer server-side compaction for OpenAI if plan tokens allow it.** It
  keeps reasoning valid and needs no summary prompt. Fallback: a client
  summary with pi's template. **Proposal change (18):** add a probe V16:
  does `/responses/compact` or `context_management` work on Sign in with
  ChatGPT plan usage with `store: false`?
- **Proposal change (10).** Each compaction step must fit one short call.
  The summary request is one HTTPS stream like any turn, so it fits the
  existing loop ("each step is a short call"); state it, and count it in the
  usage shown to the participant.
- **Proposal change (10, Adapters).** Map `compact_boundary` and the OpenAI
  `compaction` item to ACP `CompactionUpdate`; map Claude's `status:
  compacting` to `in_progress`.
- **Proposal change (4.3).** `say` writes a user message, and `claude`
  treats a leading `/` as a command: `/compact`, `/clear`, and any skill by
  name, **"even when your `skills` list omits it"** (Agent SDK docs). A
  client could run a skill the profile denied. The host must refuse or escape
  a leading `/` in `say`, and offer `compact` as a typed operation instead.
  Add a probe: does `/<denied-skill>` run under `--restricted` with a
  `Skill(<name>)` deny rule?
- **Consider (4.1).** A launch field `auto-compact: bool` set through the
  host's `--settings`, so a profile can turn `claude`'s compaction off.

---

## 5. Prompt caching

**watane needs:** for Claude, nothing it can control (claude places the
breakpoints). For OpenAI, a byte-stable prefix and a cache key.

### Per harness

- **Pi (Anthropic).** `cache_control` on the system blocks, the last tool,
  and the last block of the last user message; `ttl: "1h"` on request. Tool
  changes go in as `tool_addition`/`tool_removal` blocks "so the cached prefix
  survives every tool change" (`packages/ai/src/api/anthropic-messages.ts:82-106, 1488-1530, 1795-1820`).
  OpenAI: `prompt_cache_key` = session id (`openai-responses.ts:369`).
- **Cline.** One breakpoint, on the last user message
  (`llms/src/providers/ai-sdk.ts:413-437`). Bedrock's format once gave zero
  cache reads (CHANGELOG :450).
- **Codex.** `prompt_cache_key` per thread. On every request it compares the
  request's fixed fields with the previous one and names the first that moved
  (`model_changed`, `tools_changed`, `reasoning_changed`, …;
  `core/src/client.rs:495-560`). Synthetic outputs get stable ids "to
  preserve prompt-cache reuse" (`context_manager/normalize.rs:156-162`).
  Tests: `core/tests/suite/prompt_caching.rs`
  (`per_turn_overrides_keep_cached_prefix_and_key_constant`).
- **Gemini CLI.** No cache API; retry nudges are appended "rather than
  modifying systemInstruction" (`core/geminiChat.ts:1168`).
- **OpenCode.** Compaction sends the *whole unchanged conversation* plus the
  summary instruction, so the summary request reads the cache
  (`compaction.ts:515-519`); it waits for a model switch's first response, and
  skips a background summary when the cache is about to expire.
- **rig.** A test that consecutive recorded requests never rewrite, reorder or
  drop a block already sent; a deliberate move needs a written reason
  (`crates/rig-cassette/tests/cassette_cache_prefix.rs`,
  `test-support/rig-test-support/src/cache_prefix.rs`, ported from
  pydantic-ai).
- **Goose, OpenHands SDK.** Prefix stability by ordering and the
  STATIC/DYNAMIC split (section 1).
- **golem-ai, swiftide, AutoAgents.** Count cached tokens only.

### Pitfalls

- Pi: a thinking-level change broke the Codex cache (:70); mid-session tools
  broke Kimi's (:402).
- Cline: cached tokens counted ~5× too high in telemetry.
- Cline: rewriting old file reads each turn breaks the cache; hence batches.

### Lessons for watane

- **Adopt** `prompt_cache_key` = the kernel's session id, if plan usage accepts
  the field (add to probe V16).
- **Adopt** rig's prefix test in the kernel's test suite: record each request
  of a scripted session and assert each is a prefix-extension of the last,
  except across a compaction entry.
- **Adopt** Codex's cache-break reason as a debug field on the kernel's turn
  item. Cheap, and it explains a sudden drop in plan usage.
- **Avoid** rewriting old items (stale reads, masks) unless in large batches.
- **Note (4.1).** The host always adds `--system-prompt-snapshot=off`. Check
  that it does not change caching; the probe runs once per `claude` update
  anyway (18).

---

## 6. Reasoning items across turns

**watane needs:** the OpenAI loop runs with `store: false` and resends the
whole history (research note). Reasoning must survive that.

### Per harness

- **Codex.** Always `include: ["reasoning.encrypted_content"]`
  (`core/src/client.rs:1326`), replays reasoning items in place. Tests:
  `reasoning_replay_metrics.rs`.
- **Pi.** Same for Responses. Across models: thinking from **another model is
  dropped**, not turned into text (`packages/ai/src/api/transform-messages.ts:128-138`;
  CHANGELOG :215 records the switch, #10995). Anthropic signatures replayed;
  `redacted_thinking` passed back opaque. Orphaned tool calls get a synthetic
  "No result provided" result.
- **Gemini CLI.** Strips `thought` parts from history but keeps
  `thoughtSignature` on function calls; injects a placeholder when the first
  call lacks one, to avoid a 400 (`core/geminiChat.ts:1518-1560`).
- **Cline.** Replays Anthropic `signature`/`redactedData` and Gemini
  `thoughtSignature`; drops thinking from compacted answers.
- **Forge.** Keeps only the last reasoning block across a compaction (section 4).
- **rig.** Keeps `encrypted_content` through streaming
  (`rig-core/src/providers/openai/responses_api/streaming.rs:816-824`).
- **golem-ai.** `ContentPart` is `Text | Image` (`llm/llm/src/model.rs:125-130`):
  no place for an opaque reasoning item. Porting its model would lose
  reasoning.
- **Summaries.** Pi omits thinking from summary input, since it could make the
  request "larger than the context it summarizes".

### Lessons for watane

- **Proposal change (10, Sessions).** The OpenAI loop needs a *wire history*
  beside the ACP transcript: the exact Responses input items, including
  `reasoning` with `encrypted_content`, function calls and outputs, and
  `compaction` items. It is the kernel's private state, never sent through
  `attach` or `read`, and resent verbatim. The ACP transcript cannot be the
  source, because ACP has no opaque reasoning field.
- **Adopt** pi's rule: on a model change inside a session, drop reasoning
  items from the other model; never convert them to text.
- **Adopt** a repair pass before each request: every function call has an
  output (synthetic "no result" if cancelled), no output without its call.
  Codex `ensure_call_outputs_present` / `remove_orphan_outputs`
  (`context_manager/normalize.rs:21, 171`) is the reference.

---

## 7. Attachments and file mentions

**watane needs:** a shape for `say` input richer than one string, without the
kernel reading files.

### Per harness

- **Gemini CLI.** `@path` in the prompt: files read with `read_many_files`,
  ignore files honoured, a glob fallback when not found; directories become a
  path, not content; 20 MB per file, 2000 lines, 2000 characters per line
  (`cli/src/ui/hooks/atCommandProcessor.ts`; `core/src/utils/constants.ts:10-12`).
- **Cline.** VS Code: `@/path` → `<file_content>` / `<folder_content>`,
  `@url` → fetched markdown, `@problems`, `@terminal`, `@git-changes`. SDK:
  `@token` matched against the file index; caps have no defaults.
- **Pi.** CLI `@file` → `<file name="…">…</file>`; images resized; non-vision
  models get placeholders.
- **Codex.** Typed user input (`ResolvedUserInput`), images resized before
  sending (`core/src/image_preparation.rs`), stripped when the model has no
  image input (`normalize.rs:344-390`).
- **ACP.** Prompts are `ContentBlock` lists: `text`, `image`, `resource_link`,
  embedded `resource`.

### Lessons for watane

- **Proposal change (10).** `say(session, input)` takes ACP `ContentBlock`s
  as JSON, not a plain string. The kernel maps `text` and `image` directly
  (Claude stream-json user content; Responses `input_text`/`input_image`).
  A `resource_link` stays a path in text: the agent reads it with its own
  permission-checked tool. An embedded `resource` becomes a marked text
  fragment with a size cap. The kernel never reads a file itself.
- **Avoid** eager folder expansion and URL fetching in the kernel (Cline):
  both move I/O out of the broker's sight.
- **Keep** `--file` excluded (4.1).

---

## Who does it best

| Capability | Best | Runner-up | Why |
|---|---|---|---|
| System prompt | Codex | OpenCode | typed fragments, diffed world state, snapshot-tested |
| Project instructions | Codex | OpenCode | clear root→cwd rule, byte budget, deltas on change |
| Memory | OpenHands SDK (simple) | Codex (complete) | bounded index; Codex's pipeline is thorough but heavy |
| Compaction | OpenCode | pi (via rig-memory in Rust) | cache-aware, instruction-free summaries; pi is clean and portable |
| Tool output pruning | Gemini CLI | Codex | protected recent window, head/tail to file; Codex truncates at record |
| Prompt caching | pi | rig (tests) | breakpoints and tool deltas; rig's prefix test |
| Reasoning items | pi | Codex | cross-model rules, orphan repair |
| Attachments | Gemini CLI | Codex | limits and ignore rules; typed input |
| Server compaction (OpenAI) | Codex | OpenCode | `compaction_trigger`, `/responses/compact` in production |

Harnesses with nothing notable for this group: golem-ai (no reasoning, no
compaction), AutoAgents (a sliding window; its `TrimStrategy::Summarize`
leaves the summary to the caller), swiftide (only the summary marker), rig
core (preamble only; rig-memory is the exception).

## Test cases watane should write

Kernel (OpenAI loop), against a scripted fake Responses stream (Codex's SSE
helpers, pi's faux provider):
1. **Prefix stability.** Ten scripted turns; each request is the previous one
   plus new items. Fails on any reorder or rewrite (rig).
2. **Instructions after compaction.** Compact, then the next request holds the
   current profile prompt and project instructions once, and the summary does
   not contain them (OpenCode rule; codex #5957).
3. **Cut rule.** A compaction never separates a function call from its output
   and never drops the latest user message (pi, Forge).
4. **Reasoning replay.** Encrypted reasoning items come back verbatim and in
   order; after a model switch the other model's reasoning is gone (pi).
5. **Orphans.** A cancelled tool job yields a synthetic output; an output
   without a call is removed (Codex).
6. **Overflow error.** A context-length error from the stream triggers one
   compaction and one retry, not a loop; a rate-limit error does not (pi's
   regexes).
7. **Estimates.** After a compaction the trigger uses reported usage, not the
   pre-compaction count (Cline, pi pitfalls).
8. **Truncation.** A 1 MB tool result is stored truncated with the header and
   the original token count.
9. **Summary failure.** Empty or longer-than-input summary: the transcript is
   unchanged and the session reports it (Gemini).
10. **Snapshot.** Insta snapshot of the first request of each profile
    (Codex, goose, Forge).

Host and Claude path:
11. `say` with a leading `/` is refused or escaped (4.3).
12. `compact_boundary` becomes one ACP `CompactionUpdate` with `completed`
    and the post-compaction size.
13. Probe V16 (plan tokens with `/responses/compact`, `context_management`,
    `prompt_cache_key`) and the memory probe (section 3), in
    `proto/claude-probes` and a spike of the OpenAI loop.

## Reusable code

| What | Path | Licence | wasm32-wasip2 |
|---|---|---|---|
| Pi compaction ported to Rust: when, where to cut, prompts, file lists | rig `crates/rig-memory/src/compaction.rs`, `compaction/pi.rs` (~1100 lines) | MIT (carries pi's MIT notice) | pure; the crate depends on rig-core, which did not build for wasip2 (research note), so port the functions over the kernel's own types |
| Cache-prefix rule and test | rig `test-support/rig-test-support/src/cache_prefix.rs` | MIT | test-only, native |
| Overflow error patterns | pi `packages/ai/src/utils/overflow.ts:39-188` | MIT | port to Rust with `regex` (builds for wasm) |
| Summary templates | pi `compaction.ts:507-579`; OpenCode `compaction.ts:170-243` | MIT | text |
| Tool output truncation with header | codex `codex-rs/utils/output-truncation/src/lib.rs` | Apache-2.0 | logic is pure; depends on `codex-protocol`, so port the functions |
| AGENTS.md discovery and budget | codex `core/src/agents_md.rs:132-304` | Apache-2.0 | host side (needs the file system) |
| World-state section diffing | codex `core/src/context/world_state/mod.rs` | Apache-2.0 | design to copy; code is tied to Codex types |
| Middle-out tool response removal, retry on overflow | goose `crates/oi-kit-context-management/src/summarize.rs:13-170` | Apache-2.0 | pure; the crate pulls `oi-kit-provider-types` (reqwest, tokio, zstd), so port |
| Unicode tag sanitiser | goose `crates/goose/src/utils.rs:38` | Apache-2.0 | pure, `unicode-normalization` builds for wasm |
| Eviction range that keeps tool pairs | forge `crates/forge_domain/src/compact/strategy.rs:78-140` | Apache-2.0 | pure |
| Summary-marker projection | swiftide `swiftide-agents/src/default_context.rs:258-275` | MIT | pure, trivial |
| Instruction diff rendering | OpenCode `packages/core/src/instruction-discovery.ts:139-165` | MIT | port with `similar` (MIT/Apache, wasm-safe) |
| Compaction updates for the event model | `agent-client-protocol-schema` `CompactionUpdate`, `CompactionSummaryChunk` | Apache-2.0 | already in the spike's guest (research note) |
| Zed agent prompt, compaction | `crates/agent`, `prompt_store` | GPL-3.0 | **do not copy** |

## Proposal changes, by section

- **4.1** Optional `project-instructions: bool` (host reads AGENTS.md /
  CLAUDE.md, 32 KiB, appended to the system prompt); consider
  `auto-compact: bool` through the host's settings.
- **4.3** Refuse or escape a leading `/` in `say`; add a typed `compact`
  operation.
- **10 Sessions** A private wire history for the OpenAI loop (encrypted
  reasoning, compaction items) beside the ACP transcript; compaction is an
  appended entry and the request a projection from the last one.
- **10 Adapters** Claude `compact_boundary` and OpenAI `compaction` items map
  to ACP `CompactionUpdate`.
- **10 `say`** Input is a list of ACP `ContentBlock`s.
- **11** `project-instructions` honoured by both backends; reasoning effort
  changes inside a session are a cache break, note it.
- **18** Probe V16 (server compaction and `prompt_cache_key` on plan usage);
  probe for Claude user memory under `--restricted`; probe for `/<skill>`
  dispatch past a deny rule.
