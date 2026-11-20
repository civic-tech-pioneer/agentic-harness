# Rust agent harnesses and meta-harnesses: what to mine (2026-11-20)

A record, not a contract. It catalogues Rust projects that run an agent loop
(harnesses) or drive other agent CLIs (meta-harnesses), for the agentic layer
in `ARCHITECTURE-PROPOSAL.md` (M1 providers, M2 kernel, H2 Claude Code host
part). It adds to `research_notes/calling-agents.md`, which already checked
rig, genai, async-openai, llm, rmcp, wstd, claude-codes and
agent-client-protocol for `wasm32-wasip2`.

Method: crates.io search API (agent, llm, claude, codex, harness, orchestrator,
mcp, acp, sans-io and more), GitHub search (Rust, by stars), then shallow
clones read by hand. No build script or binary of any clone was run. Clones
are in the session scratchpad, `research/<repo>`. Stars, versions and dates
are from 2026-11-20.

## 1. Catalogue

Kind: **H** harness (own loop), **M** meta-harness (drives other CLIs),
**F** framework, **S** SDK or protocol types. Capabilities: L loop, T tools,
MCP, C compaction, Sk skills, Sub subagents, P permissions or hooks, R
sessions and resume, Pr client protocol, V provider abstraction, D drives CLI
agents.

| Project | Source | Kind | ★ | Last release | License | Capabilities | wasip2 core |
|---|---|---|---|---|---|---|---|
| codex-rs | openai/codex @bb5b9c6 | H | 136k | rust-v0.171.0, 2026-11-20 | Apache-2.0 | L T MCP C Sk Sub P R Pr(app-server) V | no: tokio full, reqwest, process |
| goose | block/goose (now aaif-goose) @80ce943 | H M | 56.8k | v1.60.2, 2026-11-20 | Apache-2.0 | L T MCP C Sk Sub P R Pr(ACP) V D(claude, codex, gemini, cursor) | no: sqlx, tokio |
| grok-build | xai-org/grok-build @2bdd1d6 (SOURCE_REV 559751f) | H | 27.8k | no tags; synced 2026-09-29 | Apache-2.0 | L T MCP C Sk Sub P R Pr(ACP) | compaction core nearly (tokio time) |
| jcode | 1jehuang/jcode @553db90 | H | 21.9k | v1.0.1, 2026-11-18 | MIT | L T MCP C Sub(swarm) R V | no |
| forgecode | tailcallhq/forgecode @a22b492 | H | 7.6k | v2.14.0, 2026-10-01 | Apache-2.0 | L T MCP C Sk P R V | domain crate maybe; app no |
| VT Code | vinhnx/VTCode @059d4e0 | H | 0.9k | 0.184.0, 2026-11-19 | MIT OR Apache-2.0 | L T MCP C Sk P R Pr(ACP, A2A) V | no |
| rig coding agent | 0xPlaygrounds/rig @0019e24: rig-ecs, rig-harness, rig-agent 0.46.0 | H F | 9.2k | 0.46.0, 2026-10-31 | MIT | L T MCP C Sk Sub P R Pr(ACP) V | not tried; rig-core has a known wasip2 bug |
| lash | Ascending-AI/lash @ff2f338; crates 0.1.0-alpha.113 | H F | 8 | alpha.113, 2026-07-25 | MIT | L T MCP C Sub P R V | **lash-sansio builds** |
| adk-rust | zavora-ai/adk-rust @65c15c0 | F H | 760 | 2.3.0, 2026-10-09 | Apache-2.0 | L T MCP Sk Sub P R Pr(ACP) V | no: reqwest, sqlx |
| swiftide | bosun-ai/swiftide 0.34.0 | F | 805 | 2026-11-06 | MIT | L T MCP | no: tokio |
| AutoAgents | liquidos-ai/AutoAgents 0.4.0 | F | 790 | 2026-07-08 | MIT OR Apache-2.0 | L T MCP V; wasmtime tool sandbox | browser wasm only |
| langchain-rust / llm-chain / kalosm | 8.0.0 / 0.13.0 / 0.4.0 | F | 1.4k / 1.6k / 2.2k | 2026-11 / 2023 / 2025-02 | MIT / MIT / MIT-Apache | L T V | no; last two stale |
| mistral.rs | EricLBuehler/mistral.rs 0.8.1 | F (local inference) | 7.9k | 2026-04-02 | MIT | V MCP | no |
| golem-ai | golemcloud/golem-ai @0923caf, 0.5.5 | S | 45 | 2026-10-22 | Apache-2.0 | V | builds, wasi:http 0.3 (known) |
| vibe-kanban | BloopAI/vibe-kanban @3f095f0 | M | 28.8k | v0.1.46, 2026-11-15 | Apache-2.0 | D(claude, codex, gemini, cursor, amp, opencode, qwen, copilot, droid, ACP) P R worktrees | no |
| agent-of-empires | agent-of-empires/agent-of-empires @cfca1ca | M | 4.6k | v1.25.0, 2026-11-18 | MIT | D(via ACP adapters) P R worktrees, containers, tmux | no |
| zeroshot | the-open-engine/zeroshot @8ebba1a | M | 2.2k | v16.1.0, 2026-11-18 | MIT | D, review graph | not read in depth |
| mango-external-agents | juliopolycarpo/mango-external-agents @82d4f45, 0.11.0 | M | 1 | 2026-11-17 | MIT | D(claude stream-json, codex app-server, ACP) P R | no: ring, tokio fs |
| claude-wrapper | joshrotenberg/claude-wrapper @7e90f54, 0.14.11 | M S | 11 | 2026-11-09 | MIT OR Apache-2.0 | D(claude) P R worktrees | no |
| oneharness | nickderobertis/oneharness @e632045, 0.31.0 | M | 1 | 2026-11-18 | MIT | D(8 CLIs, one-shot) | no |
| nexo-rs drivers | lordmacu/nexo-rs @052ee4a: nexo-driver-claude, nexo-driver-permission 0.1.9 | M | 9 | 2026-05-17 | MIT OR Apache-2.0 | D(claude) P R | no |
| embacle, gate4agent, agent-abstraction, zag-agent | crates.io 0.47.0, 0.5.19, 0.6.0, 0.18.0 | M | <10 | 2026 | Apache-2.0 / MIT / MIT / MIT | D | not read |
| worktrunk, workmux | crates.io 0.82.0, 0.1.283 | M (worktrees, tmux) | 12.7k, 3.1k | 2026-11 | MIT OR Apache-2.0, MIT | worktrees only | n/a |
| claude-codes, codex-codes | meawoppl/rust-code-agent-sdks @82a130a; 2.1.330, 0.169.8 | S | 30 | 2026-11-20 | Apache-2.0 | stream-json and app-server types, clients | **types build** |
| cc-sdk, claude-agent-sdk-rs, anthropic-agent-sdk, claude-code-sdk-rust | 0.8.1, 0.6.4, 0.2.75, 0.4.1 | S | 178, 78, 10, 1 | 2025-12 to 2026-07 | MIT | D(claude) P | no (tokio process) |
| agent-client-protocol (+ conductor) | agentclientprotocol/rust-sdk @8d91a29, 5.4.0 | S | 248 | 2026-11-18 | Apache-2.0 | Pr, proxy chains | schema builds (known) |
| ahp, ahp-types | microsoft/agent-host-protocol @e36b0a2, 1.1.0 | S | 465 | 2026-11-13 | MIT | multi-client session state, reducers | **ahp-types builds** |
| tower-mcp-types | joshrotenberg/tower-mcp @367caa3, 0.27.0 | S | 14 | 2026-11-17 | MIT OR Apache-2.0 | MCP types incl. 2026-07-28, skills, tasks | **builds** |
| rmcp | modelcontextprotocol/rust-sdk 3.11.0 | S | 4.1k | 2026-11-17 | Apache-2.0 | MCP | builds, traps when waiting (known) |
| llmsim | chaliy/llmsim 0.9.0 | test tool | 8 | 2026-11-17 | MIT | fake LLM APIs | n/a |

Not catalogued further: claude-squad (Go, AGPL), steer (AGPL), mjolnir (GPL),
codeg, arbor, golutra, openhuman, claurst (GPL). The crate `claude-agent-sdk`
0.1.1 names `anthropics/claude-agent-sdk-rust` as its repository, but that
repository does not exist: treat it as unofficial.

Zed: the clone holds only `acp_thread`, `agent_servers` and `agent_ui`
(sparse). Zed's native agent crates were not read. Zed is GPL; read only.

## 2. Per capability: the best Rust source to mine

### 2.1 Agent loop that fits a fresh instance per call

- **lash-sansio** (`crates/lash-sansio/src/sansio/mod.rs`, `turn_protocol.rs`,
  `machine_state.rs`, `turn_machine.rs`). A real sans-IO turn machine. The
  host calls `poll_effect()` and gets an `Effect`: `LlmCall`, `ToolCalls`,
  `ExecCode`, `Checkpoint`, `SyncExecutionEnvironment`. It answers with
  `handle_response(Response::LlmComplete | ...)`. `MachineState` and the
  driver state are serde types, and an input journal replays a turn. This is
  the shape the OpenAI loop in M2 needs: one effect out per short call, the
  state saved between calls.

  ```rust
  pub enum Effect<M: TurnProtocol = UnitTurnProtocol> {
      SyncExecutionEnvironment { id: EffectId },
      LlmCall { id: EffectId, request: Arc<RetainedLlmRequest> },
      ToolCalls { id: EffectId, calls: Vec<PendingToolCall>, expansion: ToolExpansionPlan },
      ExecCode { id: EffectId, language: String, code: String },
      Checkpoint { .. },
  }
  ```
- **goose** `crates/goose/src/agents/state_machine/` (`mod.rs`, `ops_llm.rs`,
  `ops_toolcalling.rs`, `tool_approval.rs`, `ops_subagent.rs`): "an ordered,
  re-entrant pipeline over persisted conversation state". Readable; each step
  is an op over stored messages.
- **rig-ecs** `crates/rig-ecs/src/` (`turn.rs`, `effects.rs`, `session_log.rs`):
  turns and calls as Bevy entities, one recorded effect path. Interesting but
  Bevy is a large dependency for a mod.
- codex-rs `core/src/tasks/regular.rs`, `core/src/session/`: the reference
  for a production Responses loop; too coupled to copy.

### 2.2 Context and compaction

- **grok-build** `crates/common/xai-grok-compaction/src/` (`lib.rs`,
  `code_compaction/`, `intra_compaction/`, `inter_compaction/`, `select.rs`).
  "Transport-agnostic": the host supplies `CompactionItem`, `ItemTokenCounter`
  and `CompactionSampler` (the model call). Depends only on serde, tokio
  (time, macros), async-trait. Best port candidate.
- codex-rs `core/src/compact.rs`, `compact_token_budget.rs`,
  `compact_retained_tool_outputs.rs`; tests `core/tests/suite/compact*.rs`.
  Shows what to keep across a compaction (pending input, tool outputs).
- lash `crates/lash-plugin-standard-compaction/src/lib.rs`: compaction as a
  plugin, at a context-pressure threshold and on overflow recovery.
- jcode `crates/jcode-compaction-core/src/lib.rs`: the simple version (budget,
  80 % trigger, hard drop above a ceiling).

### 2.3 Skills

- **codex-rs** `skills/src/` (`parser.rs`, `loading.rs`, `selection.rs`,
  `mentions.rs`, `invocation.rs`). SKILL.md front matter with serde_yaml;
  separate selection and mention handling. Pure apart from `codex-protocol`.
- **adk-rust** `adk-skill/src/` (`parser.rs`, `index.rs`, `injector.rs`,
  `progressive.rs`). Progressive disclosure: index first, body on use. Fits the
  kernel's "offers" (W34) well.
- tower-mcp-types `src/skills.rs`: the MCP skills types, if skills travel over
  MCP.
- Also: rig `plugins/rig-basics/src/skills.rs`, VT Code `vtcode-skills`.

### 2.4 Subagents and delegation

- codex-rs `core/src/agent/` (`registry.rs`, `role.rs`, `control.rs`,
  `child_config.rs`), `core/src/thread_spawn.rs`, `core/src/codex_delegate.rs`.
  Roles, child config narrowing, a mailbox between agents.
- grok-build `crates/codegen/xai-grok-subagent-resolution/src/` (`definition.rs`,
  `overrides.rs`, `resume.rs`). Definition plus overrides plus resume, small.
- goose `agents/subagent_handler.rs`, `agents/subagent_task_config.rs`.
- Zed `crates/acp_thread/src/subagents.rs`: how an ACP client shows a child
  session under a parent (title, can_cancel, cwd).
- jcode `crates/jcode-swarm-core`: coordinator and workers; read only.

### 2.5 Permissions, approvals, hooks

- **grok-build** `crates/codegen/xai-grok-permission-rules/src/`
  (`claude_settings.rs`, `policy.rs`, `bash_command_splitting.rs`,
  `resolution.rs`). Parses `.claude/settings.json` permission rules and modes,
  and splits bash commands with tree-sitter-bash before matching. The closest
  match to the kernel broker's `Name(arg)` matcher (section 10).
- **mango** `crates/mango-agent-claude/src/permissions.rs`: maps product axes
  onto Claude's single `--permission-mode` and refuses pairs it cannot
  express, instead of rounding. The same rule as G4 ("refuses otherwise").
- goose `crates/goose/src/permission/permission_judge.rs`, `security/`,
  `tool_inspection.rs`: an LLM judge and inspectors before a tool runs. The
  same idea as the checked provider (section 9).
- nexo `crates/driver-permission/src/` (`bash_destructive.rs`,
  `sed_validator.rs`, `path_extractor.rs`, `decider.rs`): a
  `permission_prompt` MCP tool with rule-based deciders.
- codex-rs `execpolicy/` (Starlark rules) and `core/src/exec_policy.rs`:
  powerful, heavy; read only.
- Zed `crates/acp_thread/src/acp_thread.rs` (`ToolCallStatus::WaitingForConfirmation`,
  `AuthorizationKind`): a client-side model for an ask.

### 2.6 Sessions, transcripts, resume

- codex-rs `rollout/` (JSONL rollouts), `thread-store/`, `core/src/thread_rollout_truncation.rs`.
- grok-build `xai-grok-session-events` (typed per-session JSONL event log) and
  `xai-grok-foreign-sessions` (metadata-only discovery of Claude and Codex
  sessions on disk).
- claude-codes `claude-codes/src/transcript.rs`: types for Claude's own
  `~/.claude/projects` JSONL.
- lash `crates/transcript-contract`, `lash-sqlite-store`: durable cursors so a
  late follower catches up. Same need as the kernel's `attach(session, after)`.

### 2.7 Client protocol and event model

- ACP stays the event model (`agent-client-protocol-schema` 1.19.0). The
  ACP rust-sdk now has `pub mod v1` and `pub mod v2`
  (`src/agent-client-protocol/src/lib.rs`), and Zed's `acp_thread` imports
  both schemas. Which version the kernel maps to is still open.
- **ACP conductor** (`acp-rust-sdk/src/agent-client-protocol-conductor`,
  `md/conductor.md`, `md/proxying-acp.md`): `Editor → Conductor → Proxy … →
  Agent`, with a `proxy/successor` envelope. A checked provider is exactly
  such a proxy.
- **AHP** (`agent-host-protocol/clients/rust/crates/ahp-types`,
  `crates/ahp/src/reducers.rs`): one session server, many clients, immutable
  state, pure reducers over `StateAction`. VS Code's agent host speaks it.
  Useful if the kernel keeps a synchronized view for several pages.

### 2.8 Provider abstraction and the ChatGPT route

- lash `crates/lash-provider-openai/src/codex.rs` and `codex/`: a Codex
  provider for ChatGPT accounts, over WebSocket and HTTP, with cached-context
  continuation.
- goose `providers/chatgpt_codex.rs` and codex-rs `login/`: the same route.
  Both use the Codex CLI's own OAuth client id against
  `chatgpt.com/backend-api/codex`. watane uses Sign in with ChatGPT with its
  own registration against `api.openai.com`; read only, do not copy the id.
- Do not copy: jcode (`jcode-base/src/auth/oauth.rs`) and rig's
  `plugins/rig-claude-cloak` call Anthropic with a Claude subscription OAuth
  token. That breaks G5 and the terms in `calling-agents.md`.

### 2.9 Tests

| Approach | Best source |
|---|---|
| Scripted provider wire scripts, schema-versioned | lash `crates/lash-provider-fixtures/src/provider/wire_script.rs` (`lash.provider-wire-script.v1`) |
| Deterministic simulation, crash and replay matrices | lash `crates/lash-sim/src/` |
| Record, replay, scrub provider HTTP and effect logs | rig `crates/rig-cassette/src/` |
| Mock Responses server with SSE, request assertions | codex-rs `core/tests/common/responses.rs`, `streaming_sse.rs` (wiremock), snapshots with insta |
| Record and replay keyed by a hash of the input | goose `crates/goose/src/providers/testprovider.rs`, `tests/mcp_replays/` |
| Mock inference server, SSE generators, scripted ACP client | grok-build `crates/codegen/xai-grok-test-support/src/` |
| Fake CLI launcher and harness; `--help` text per CLI version | mango `crates/mango-external-agents/src/testing/`, `fixtures/claude/help/`, `fixtures/codex/*.jsonl` |
| Captured CLI output, schema drift checks against the binary | claude-codes `test_cases/`, `RESNAPSHOTTING.md`, `scripts/check_claude_schema_drift.py` |
| Fake agent binary | oneharness `crates/oneharness-mock-harness`, vibe-kanban `executors/qa_mock.rs` |
| Fake LLM HTTP API | llmsim 0.9.0 |

## 3. Meta-harness patterns

### 3.1 How they start `claude`

The common core, in vibe-kanban, goose, mango, claude-wrapper and nexo:

```
claude -p --input-format stream-json --output-format stream-json --verbose
       --include-partial-messages
```

Additions seen:
- vibe-kanban `crates/executors/src/executors/claude.rs:157-194`:
  `--replay-user-messages`; `--permission-prompt-tool=stdio` with
  `--permission-mode=bypassPermissions` when approvals are on;
  `--disallowedTools=AskUserQuestion` when they are off; `--resume <id>` and
  `--resume-session-at <message>` for a fork.
- goose `crates/goose/src/providers/claude_code.rs:338-399`:
  `--system-prompt-file`, `--mcp-config <file> --strict-mcp-config`,
  `--permission-prompt-tool stdio`, `--model`.
- mango `crates/mango-agent-claude/src/argv.rs`, `cli_surface.rs`: parses
  `claude --help` and adds a flag only if this build declares it (fail
  closed): `--permission-prompts` (from 2.1.259), `--json-schema`,
  `--forward-subagent-text`, `--mcp-config`. It mints the `--session-id`
  UUID itself. One child process per turn, `--resume` between turns.
- Zed `crates/agent_servers/src/acp.rs:1540-1575` (test): sets
  `CLAUDE_CONFIG_DIR` to a scratch folder and blanks `ANTHROPIC_API_KEY` and
  `CLAUDE_CODE_OAUTH_TOKEN`.

### 3.2 Environment

mango (`docs/compliance.md`, `src/env.rs`) passes a positive allowlist:
`PATH`, temp, `HOME` and XDG folders, locale, `NO_COLOR`, version-manager
roots, plus the keys a harness names. It never forwards `*_TOKEN`,
`*_API_KEY`, `*_SECRET`, proxies, or inline config. It never reads a
credential file; login state it cannot see is `Unknown`. This matches G5 and
is stricter than the spike (which removes only `ANTHROPIC_API_KEY`).

### 3.3 The control protocol

- vibe-kanban `executors/claude/protocol.rs`, `client.rs`, `types.rs`:
  `initialize` with hooks; `can_use_tool` → `control_response`;
  `hook_callback`; `interrupt`; `set_permission_mode`. The trick
  (`client.rs:323-380`): a PreToolUse hook callback answers
  `permissionDecision: "ask"`, which makes the CLI send `can_use_tool`, so
  every tool call reaches the host even under `bypassPermissions`. A Stop hook
  callback blocks the end of a turn while git has uncommitted changes.
- goose `claude_code.rs`: `initialize`, `set_model`, `can_use_tool`.
- claude-codes `src/io/control.rs`: typed `ControlRequestPayload`
  (`CanUseTool`, `HookCallback`, `Interrupt`, …). Already chosen; it covers
  the whole control protocol.
- claude-wrapper `src/duplex.rs`: one long-lived child per conversation, a
  `PermissionHandler`, and `--permission-prompt-tool stdio` added when a
  handler is set.

For watane: this is evidence for the checked provider's `pre-tool-check`
(section 9) without `--restricted` problems, and for V9.

### 3.4 Codex

mango `crates/mango-agent-codex/src/` and codex-codes `src/client_async.rs`
drive `codex app-server` (JSON-RPC over lines). Approvals arrive as
server-to-client requests (`CommandExecutionApprovalResponse`,
`FileChangeApprovalResponse`). `approval_policy` and sandbox go in as `-c`
overrides (`codex-codes/src/cli.rs`). Fixtures:
`mango-external-agents/fixtures/codex/approval.jsonl`, `interrupt.jsonl`,
`subagent.jsonl`.

### 3.5 Version drift

- claude-codes extracts the wire schemas from the shipped `claude` binary and
  diffs a fingerprint nightly; the crate version means "tested against".
- codex-codes generates `src/protocol_generated/` from Codex's own schema.
- mango keeps `claude --help` text per version as fixtures and tests its flag
  detection against each.

### 3.6 Workspace isolation

Git worktrees: vibe-kanban `crates/worktree-manager`, agent-of-empires
`src/git`, claude-wrapper `src/worktrees.rs`, codex-rs `worktree/`, grok-build
`xai-fast-worktree`, worktrunk, workmux. Containers: agent-of-empires
`src/containers` (Docker, Podman, Apple containers), tsk-ai. mango does not
isolate; the host authorizes a working folder through a port.

### 3.7 Normalization and the ACP route

- vibe-kanban: each executor has `normalize_logs` into its own log model
  (`executors/codex/normalize_logs.rs`).
- mango: reducers per vendor (`mango-agent-claude/src/reducer.rs`,
  `mango-agent-codex/src/turn_reducer.rs`) into one event enum.
- Zed and agent-of-empires reach Claude through `claude-agent-acp`
  (`acp-worker/adapters/claude-agent-acp`). That adapter uses the Agent SDK,
  which `calling-agents.md` rules out for watane.

## 4. Dependency, port, or read only

| Use as a dependency | For |
|---|---|
| agent-client-protocol-schema 1.19.0 | the event model (kept) |
| claude-codes 2.1.330, `types` | Claude stream-json and control protocol (kept) |
| codex-codes 0.169.8, `types` | a future Codex CLI provider (app-server types, approvals) |
| tower-mcp-types 0.27.0 | MCP types inside a mod; rmcp stays in the host |
| ahp-types 1.1.0 (optional) | a synchronized session state for several clients |
| lash-sansio 0.1.0-alpha.113 (spike first) | the turn machine for the OpenAI loop; alpha, one maintainer, 117 deps, crates.io 4 months behind master |

| Port (copy the idea or the code, with the license) | From |
|---|---|
| Compaction engine with host seams | grok-build `xai-grok-compaction` (Apache-2.0) |
| Claude-style permission rules and bash splitting | grok-build `xai-grok-permission-rules` (Apache-2.0) |
| SKILL.md parser and selection | codex-rs `skills` (Apache-2.0); adk-skill progressive disclosure (Apache-2.0) |
| Flag detection from `claude --help`, env allowlist, mode matrix | mango-external-agents (MIT) |
| Hook "ask" → `can_use_tool`, Stop hook check | vibe-kanban executors (Apache-2.0) |
| Wire-script fixtures, record and replay providers | lash-provider-fixtures (MIT), goose testprovider (Apache-2.0), rig-cassette (MIT) |
| Effect/response loop shape, if not the crate | lash-sansio (MIT) |

| Read only | Why |
|---|---|
| codex-rs core, goose agent, forgecode, VT Code | too coupled, native IO everywhere |
| Zed `acp_thread`, `agent_servers` | GPL; useful client model for asks and subagents |
| AHP reducers (`ahp` crate) | tokio in the client crate; reducers themselves are pure |
| ACP conductor | design of proxy chains for the checked provider |
| jcode, rig-claude-cloak | use Claude subscription OAuth outside `claude`: avoid |
| mjolnir (GPL), steer (AGPL), claude-squad (Go, AGPL) | license |

## 5. wasm32-wasip2 results

Scratch crates in the session scratchpad, `research/wasm-checks/`
(rustc 1.100.0, `cargo check --target wasm32-wasip2`, 2026-11-20). Each probe
names one public type so the crate is compiled, not only resolved.

| Crate | Result | Notes |
|---|---|---|
| lash-sansio =0.1.0-alpha.113 | check ok; `cargo build --release` ok, 15 KB cdylib | 117 crates incl. jsonschema 0.18, getrandom 0.2 and 0.3, rmp-serde; the probe used only `EffectId`, so the turn machine was not run |
| tower-mcp-types =0.27.0 | ok | 25 crates; no tokio |
| codex-codes =0.169.8, `default-features = false, features = ["types"]` | ok | 19 crates; no tokio |
| ahp-types =1.1.0 | ok | 18 crates; serde only |

Not tried, from Cargo.toml: rig-ecs (Bevy ECS and tasks; its wasm cfg is for
browser wasm), mango-external-agents core (ring, tokio fs), xai-grok-compaction
(not on crates.io; tokio time and macros only, so likely fine), adk-skill
(reqwest, google-cloud-auth), codex `skills` (pulls `codex-protocol`, which
pulls an HTTP client and a network proxy).

## 6. What this changes

- **MCP types in a mod**: tower-mcp-types instead of rmcp. rmcp's client
  traps on wasip2 when it waits; tower-mcp-types has no runtime at all.
- **A Codex CLI provider later**: codex-codes types, not our own.
- **The OpenAI loop**: model it on lash-sansio's `poll_effect` /
  `handle_response`. Spike it as a dependency before writing our own.
- **Claude flags**: detect from `claude --help` (mango), and check the new
  `--permission-prompts` flag (2.1.259+) against section 4 and V9.
- **Checked provider**: the hook-"ask" trick (vibe-kanban) routes every tool
  call to `can_use_tool`; the ACP conductor is the same shape as a judging
  proxy.
- **Kept**: claude-codes, agent-client-protocol-schema, wstd, and "no rig".
