# Event protocol and provider abstraction: what other harnesses did (2026-11-20)

A record, not a contract. Group 6 of the harness comparison: the event
protocol from the kernel to clients, and the provider abstraction behind it.
It feeds `ARCHITECTURE-PROPOSAL.md` (revision 3); section numbers below
(§n) are that document's. Nothing here is decided until a design slice takes
it into the contracts.

## Sources

Every citation is `<key> <path>:<line>`, at the commit below. A path is
relative to the repository root, or to the sub-folder that the table names.
Shallow clones, read only; nothing was built or run.

| Key | Repository | Commit | Licence | Language |
|---|---|---|---|---|
| ACP | agentclientprotocol/agent-client-protocol (schema crate 1.19.0, docs, RFDs) | 65ee6ae | Apache-2.0 | Rust |
| CCA | agentclientprotocol/claude-agent-acp (was zed-industries/claude-code-acp) | a4f0f78 | Apache-2.0 | TS |
| CXA | zed-industries/codex-acp | 296069e | Apache-2.0 | Rust |
| PIA | svkozak/pi-acp | 04d0a15 | MIT | TS |
| CDX | openai/codex (`codex-rs/`) | bb5b9c6 | Apache-2.0 | Rust |
| OC | sst/opencode | b7f0b90 | MIT | TS |
| GO | block/goose | 80ce943 | Apache-2.0 | Rust |
| PI | badlogic/pi-mono (`packages/ai`) | e5d93ea | MIT | TS |
| OH | OpenHands/software-agent-sdk (`openhands-sdk/openhands/sdk`); the OpenHands repo itself is now the front end only | d5b327b | MIT | Python |
| GEM | google-gemini/gemini-cli | d4cb171 | Apache-2.0 | TS |
| CL | cline/cline (`sdk/packages/llms`) | 20b0427 | Apache-2.0 | TS |
| RIG | 0xPlaygrounds/rig (`crates/`) | 0019e24 | MIT | Rust |
| GEN | jeremychone/rust-genai | 2a8a5b3 | MIT OR Apache-2.0 | Rust |
| GAI | golemcloud/golem-ai (`llm/`) | 0923caf | Apache-2.0 | Rust |
| FG | antinomyhq/forge (`crates/`) | a22b492 | Apache-2.0 | Rust |
| ZED | zed-industries/zed (`crates/language_model`, partial checkout) | 10d1b2a | GPL-3.0-or-later | Rust |
| CC | crate claude-codes 2.1.330 (cargo registry) | – | Apache-2.0 | Rust |

watane's crates are `MIT OR Apache-2.0`, and `deny.toml` allows permissive
licences only. Everything above except ZED may be copied with its notice. ZED
is reference only.

Issues are cited as `<owner/repo>#n`, read on 2026-11-20.

## The finding that frames the rest

ACP has two versions now. v1 is stable. **v2 is a draft since 2026-07-20**
(ACP `docs/announcements/acp-v2-draft.mdx`), shipped in the same schema crate
behind the `unstable_protocol_v2` feature (ACP
`agent-client-protocol-schema/Cargo.toml`). The spike (`proto/agent-spike/`)
maps to **v1** types. v2 changes exactly the parts that §3, §10 and §12 worry
about: replay, partial messages on attach, failures after a turn started,
pending asks after cancel, and unknown variants. See capability 1 and 7.

---

## 1. Event and message model

**watane needs:** one item format on stream C (§2.2) that carries deltas, whole
messages, tool steps, plans, asks, verdicts, usage and the end of a turn, and
that the kernel can store per finished entry and replay (§3, §10).

### ACP v1 (what the spike uses)
- **Interface.** `SessionUpdate` is a closed tagged union on `sessionUpdate`:
  `user_message_chunk`, `agent_message_chunk`, `agent_thought_chunk`,
  `tool_call`, `tool_call_update`, `plan`, `plan_update`, `plan_removed`,
  `available_commands_update`, `current_mode_update`, `config_option_update`,
  `session_info_update`, `usage_update`, `notice`, `compaction_*` (ACP
  `agent-client-protocol-schema/src/v1/client.rs:105-155`).
- The turn ends in the **response** to `session/prompt`, with `stopReason`
  `end_turn | max_tokens | max_turn_requests | refusal | cancelled` (ACP
  `.../v1/agent.rs:4852-4868`). A failure is a JSON-RPC error on that
  response.
- Asks are a separate client request, `session/request_permission`, with
  `options` of kind `allow_once | allow_always | reject_once | reject_always`.
- **No fallback variant.** The v1 Rust enum has no `Other`: a receiver built
  against an older schema fails on a new `sessionUpdate`.
- `tool_call_update` replaces `content` whole; there is no append.

### ACP v2 (draft)
- **Every entity has an id and patch semantics.** Messages carry a required
  `messageId`; `agent_message` / `user_message` / `agent_thought` are
  whole-message upserts; chunks append; `null` clears; omitted keeps (ACP
  `docs/protocol/v2/migration.mdx` "Messages and message IDs").
  `MaybeUndefined<T>` models the three states (ACP
  `agent-client-protocol-schema/src/serde_util.rs:613`).
- `tool_call` is gone; the first `tool_call_update` for an id creates it;
  `tool_call_content_chunk` appends one content item.
- **`state_update`**: `running | idle | requires_action | unknown`, with the
  stop reason on `idle`, a new `error` stop reason carrying the JSON-RPC error,
  and an optional `background` list.
- `plan_update` with `planId` and `type: items | markdown`; `subagent_update`
  for child sessions; `replay_complete` after a replay.
- Permission requests get their own `title`, `description` and an open
  `subject` (`tool_call` or `command`), instead of rewriting the tool call's
  title.
- **Open enums**: `SessionUpdate::Other(OtherSessionUpdate)` (ACP
  `.../v2/client.rs:196`); `_`-prefixed values are implementation extensions.

### Codex app-server (CDX)
- **Thread → turn → item.** Notifications `turn/started`, `item/started`,
  `item/completed`, `turn/completed`, plus typed deltas
  `item/agentMessage/delta`, `item/reasoning/summaryTextDelta`,
  `item/reasoning/textDelta`, `item/commandExecution/outputDelta`,
  `item/fileChange/outputDelta`, `item/plan/delta`, `turn/plan/updated`,
  `turn/diff/updated`, `thread/tokenUsage/updated`,
  `account/rateLimits/updated` (CDX
  `app-server-protocol/src/protocol/common.rs:2086-2180`).
- Item kinds: `UserMessage`, `AgentMessage`, `Plan`, `Reasoning`,
  `CommandExecution`, `FileChange`, `McpToolCall`, `DynamicToolCall`,
  `WebSearch`, `ContextCompaction` and more (CDX
  `.../protocol/v2/item.rs:319`).
- Asks are server-to-client requests per kind:
  `item/commandExecution/requestApproval`, `item/fileChange/requestApproval`,
  `item/permissions/requestApproval` (CDX `.../common.rs:1939-1970`).
  **`serverRequest/resolved`** tells every client that a request was answered
  elsewhere (CDX `.../v2/notification.rs:74`).
- `TurnStatus`: `completed | interrupted | failed | inProgress` (CDX
  `.../v2/turn.rs:34`).

### Others
- **OpenCode.** HTTP API plus one SSE stream `/api/event` with a `types`
  filter (OC `packages/protocol/src/groups/event.ts:40-55`). Permission asks
  are **resources**: create, list, get, reply under
  `/api/session/:id/permission` (OC `.../groups/permission.ts:88-154`), so a
  client that reconnects can list what is still pending.
- **Gemini CLI.** An internal `AgentProtocol`: `send()` returns a `streamId`
  before `agent_start`; events `message`, `agent_start`, `agent_end`,
  `tool_request`, `tool_update`, `tool_response`, `elicitation_*`, `usage`
  (GEM `packages/core/src/agent/types.ts:14-126`). Its ACP layer translates
  from that (GEM `packages/cli/src/acp/acpSession.ts`). This is v2's
  "acknowledge, then stream" shape.
- **OpenHands.** Every event has `id`, `timestamp`, `source`
  (`agent | user | environment | hook`) and `parent_id` (OH
  `event/base.py`, `event/types.py:7`). Frozen, `extra="forbid"`.
- **Pi agent / Cline / rig** are provider-level streams; see capability 5.

### Lessons for watane
- **Adopt** stable entity ids and upserts for stream C and the transcript. The
  kernel stores "a finished message, tool step, ask or verdict" (§3, §10);
  with ids it can store the entry and send a whole-message upsert on replay.
  That also fixes §3's gap "deltas of a message still being written are not
  replayed": on `attach`, send the partial message so far as one upsert, then
  live chunks.
- **Adopt** an explicit turn state in the stream (`running`, `idle` with stop
  reason, `requires_action` while an ask waits). A client that attaches late
  (§12 S7, §13 polling `read`) then knows whether a turn is running without a
  pending RPC. v1 cannot say this.
- **Adopt** an "ask resolved" item (CDX `serverRequest/resolved`; v2 cancels
  the pending request with `$/cancel_request`). In §12 S4 the checked provider
  may answer first, an ask may expire after 10 minutes (§3), or a cancel may
  drop it. The owner must learn that its prompt is stale.
- **Adopt** OpenCode's idea that pending asks are readable state: `read` and
  `attach` replay an ask that is still open, and its resolution.
- **Avoid** resending whole tool output on each delta: codex-acp did, and
  crashed on large outputs (zed-industries/codex-acp#225, O(n²) memory). With
  v1 this is the natural mistake, because v1 has no append for tool content.
- **Proposal change §10 (Adapters) and §2.2.** Say which ACP version stream C
  speaks. Recommended: v2 entity semantics now, pinned to one exact schema
  release (see capability 7).

---

## 2. Session lifecycle methods

**watane needs:** `start`, `attach`, `read`, `say`, `answer`, `cancel` (§10),
resume after restart (§12 S7), and clear rules for a `say` during a running
turn.

### ACP
- v1: `session/new`, `session/load` (replays history), `session/resume` (no
  replay), `session/prompt` (pending for the whole turn), `session/cancel`
  (notification), `session/list`, `session/close`, `session/delete`, each
  gated by a capability.
- v2: `session/load` is removed; `session/resume` takes
  `replayFrom: {type: "start"}` and ends a replay with `replay_complete`, then
  a current-state snapshot. A baseline is required once `capabilities.session`
  is present: new, list, resume, close, prompt, cancel, update (ACP
  `docs/protocol/v2/migration.mdx` "Session setup and lifecycle").
- v2 `session/prompt` answers once the user message is **inserted**, with its
  `messageId`. A prompt during a running turn is held or inserted, never
  refused. An optional `idempotencyKey` makes a retry return the first
  `messageId` (ACP CHANGELOG 1.19.0, #2847).
- v2 `session/cancel`: the agent sends the cancelled work's last updates, then
  `idle` with `cancelled`, and cancels its own pending permission requests one
  by one. `session/close` also stops background work.

### Codex app-server (CDX)
- `thread/start`, `thread/resume`, `thread/fork`, `thread/archive`,
  `thread/list`, `thread/read`, `thread/turns/list`, `thread/items/list`,
  `turn/start`, `turn/steer`, `turn/interrupt`, a thread queue
  (`thread/queue/*`), `thread/revert` (CDX
  `app-server-protocol/src/protocol/common.rs:580-1160`).
- Steering a running turn is a method of its own, with a typed error
  `ActiveTurnNotSteerable` (CDX `.../v2/shared.rs:125`).

### OpenCode
- `session.create`, `session.prompt`, `session.interrupt`, `session.fork`,
  `session.wait`, `session.inbox.*` (queued inputs), `session.revert.*` (OC
  `packages/protocol/src/groups/session.ts:128-700`). Errors include
  `SessionBusyError` (OC `packages/protocol/src/errors.ts:56`).

### Pitfalls seen
- **Turn end before the stream ends.** codex-acp closed the turn on
  `TaskComplete` while chunks and tool results still arrived
  (zed-industries/codex-acp#302). claude-agent-acp: a prompt absorbed into a
  task-notification cycle never resolved (agentclientprotocol/claude-agent-acp#1445);
  a v2 prompt during agent-started work got its reply twice (#1611).
- **Resume without history.** "Resuming a native `claude --session-id`
  conversation silently completes without its history" (#1019); nothing read
  the Claude stream after load until the first prompt (#1621).

### Lessons for watane
- **Adopt** v2's rule: `say` acknowledges insertion and returns the entry
  index of the inserted user message; the end of the turn is an item. §10's
  `say` already returns `()` at once, so this is a small change: return the
  index.
- **Adopt** an end-of-replay marker for `attach(session, after)` (§10): items
  up to the marker are history, after it live. Without it a client cannot tell
  where history stops (CCA had to add `replay_complete` for this).
- **Adopt** an idempotency key on `say` only if `say` can be retried across a
  restart (§12 S7). The host's commit rule (§7) already makes a failed call
  push nothing, so a retry inside one session is safe without it.
- **Avoid** answering a held `say` with an error because a turn is running.
- **Proposal change §12 S7.** After restart, the kernel emits a state item
  (`idle`, or `unknown` for a stream that died mid-turn) before any new turn.
  v2 forbids replaying old states as current; the same holds for watane's
  transcript.

---

## 3. Capability negotiation

**watane needs:** a way for a client to say what it can render, and for the
kernel to say what a backend and a profile can do, without breaking older
clients.

### ACP
- v1 `initialize`: `clientCapabilities` (fs, terminal, plan, notices,
  compaction, ...) and `agentCapabilities` (loadSession, prompt content types,
  MCP transports, session sub-capabilities). Many updates are only allowed
  when the client advertised them (`plan_update`, `notice`, `compaction_*`;
  ACP `.../v1/client.rs:123-155`).
- v2: same field names both ways (`info`, `capabilities`), objects instead of
  booleans, fewer gates. Negotiating v2 does **not** enable unstable features;
  each has its own marker.
- Unstable features are Cargo features (`unstable_session_fork`,
  `unstable_end_turn_token_usage`, ...; ACP
  `agent-client-protocol-schema/Cargo.toml`).

### Codex app-server (CDX)
- `InitializeCapabilities.experimental_api: bool` opts into experimental
  methods **and fields**. Each experimental field registers itself
  (`inventory::collect!(ExperimentalField)`), and the server refuses a stable
  client that uses one with "`<reason>` requires experimentalApi capability"
  (CDX `app-server-protocol/src/experimental_api.rs:1-30`, `.../v1.rs:45-72`).
- `opt_out_notification_methods`: a client names notifications it does not
  want (CDX `.../v1.rs:66-68`).

### claude-agent-acp (CCA)
- Client profiles (`plain`, `zed`, `air`) with different capability sets drive
  different output; the scenario tests record each profile's traffic (CCA
  `src/tests/acp-scenarios/harness.ts:20-50`).

### Pitfalls seen
- A capability that an adapter cannot express silently changes behaviour:
  permission requests sent in `bypassPermissions` mode
  (agentclientprotocol/claude-agent-acp#1196); clients cannot confirm the
  effective permission mode (#1354).

### Lessons for watane
- **Adopt** a two-level split: the **WIT package version** fixes the item
  schema (one ACP version per `mods:agent@x.y`); a small list of **client
  capabilities** passed at `attach` gates optional items (plans, notices,
  verdict details). The kernel never sends a gated item to a client that did
  not ask.
- **Adopt** codex's per-field gate for experimental profile and overlay fields:
  a stable client that sets one gets a typed refusal naming the field. It
  matches §11's "refuse, never drop".
- **Adopt** reporting the effective settings after `start`: the kernel already
  checks `init` (§4.3); put the effective model, mode and tools into one
  `session_info`-like item so the owner can see them (CCA #1354 shows clients
  need this).
- **Proposal change §10 WIT.** `attach` gains a `caps: list<string>` argument,
  or `start` returns the item schema version. Without one of these, a new item
  kind breaks every older client (ACP v1 has no fallback variant).

---

## 4. Mapping vendor streams to one model (what is lost)

**watane needs:** Claude stream-json lines and Responses SSE events mapped to
ACP items (§10 Adapters), with each loss known and tested.

### Claude Code → ACP (CCA; the CLI writes the same messages as the SDK)
- **Mapped:** `text`/`text_delta` → message chunk; `thinking`/`thinking_delta`
  → thought chunk; `tool_use`, `server_tool_use`, `mcp_tool_use` → tool call,
  with `TodoWrite` → plan (CCA `src/sdk-to-acp.ts:335-420`); every
  `*_tool_result` → tool call update (`:504-511`); `result` → stop reason (CCA
  `src/result-end.ts:61`).
- **Dropped:** `redacted_thinking`, `signature_delta`, `citations_delta`,
  `document`, `search_result`, `container_upload`, `compaction`,
  `compaction_delta`, `fallback`, `mcp_tool_listing` (CCA
  `src/sdk-to-acp.ts:671-681`).
- **Partial tool input.** With partial messages on, CCA scans
  `input_json_delta` and emits a refined tool call at each top-level comma,
  then lets the consolidated assistant message finish it (CCA
  `src/sdk-to-acp.ts:782-830`).
- **Result mapping** (CCA `src/result-end.ts:50-79`):
  - any subtype with `stop_reason: "refusal"` → `refusal`;
  - `success` with `/login` text → failure "login required"; with
    `max_tokens` → `max_tokens`; with `is_error` → failure; else `end_turn`;
  - `error_during_execution` → `max_tokens` or failure or `end_turn`;
  - `error_max_turns`, `error_max_budget_usd`,
    `error_max_structured_output_retries` → failure if `is_error`, else
    `max_turn_requests`.
- **Vendor data in `_meta`:** `_claude/model` on `usage_update`,
  `_claude/rateLimit` from `rate_limit_event` (CCA scenario files, e.g.
  `src/tests/acp-scenarios/origin-main/zed/permission-denied.jsonl`).
- **Context window** is learned from each result's `modelUsage`, cached per
  (backend, model id), and cleared on logout because 1M context is
  per-account (CCA `src/context-window.ts:1-30`).

### Codex → ACP (CXA, in-process on codex-core)
- `TokenCount` → `usage_update(used, size)` only; totals and cost are not sent
  (CXA `src/thread.rs:1128-1137`).
- `StreamError` (a retry in progress) is only logged; `Error` becomes a
  JSON-RPC internal error with `codex_error_info` in `data` (CXA
  `src/thread.rs:1361-1382`).
- Ignored: raw reasoning in some paths, hooks, `TurnDiff`, settings, sub-agent
  events, deprecation notices, `RequestUserInput` (CXA
  `src/thread.rs:1476-1506`).
- `ModelReroute` (a different model served the turn) is only logged (`:1448`).

### Pi → ACP (PIA)
- Stop reasons collapse to `end_turn | cancelled | error`, and `error` is then
  reported as `end_turn` (PIA `src/acp/session.ts:45`, `src/acp/agent.ts:892`).
  A failed turn looks like success to a v1 client.

### Pitfalls seen
- **Duplicates** from partial plus consolidated messages: CCA CHANGELOG
  entries for #785, #789, #800 ("Deduplicate streamed assistant blocks by
  content"), and #1149 (still open: a prefix dedupe fails on tagged deltas).
- **New CLI frames** (`post_turn_summary`, `task_summary`,
  `permission_check_status`) hit `unreachable()` and show "Unexpected case" to
  users (agentclientprotocol/claude-agent-acp#1532).
- **Rate-limit info lost** when `rate_limit_event` arrives before the turn's
  first usage (#1127).
- **One bad tool input fails the turn**: a numeric `file_path` made the
  adapter throw, the prompt failed with -32603, the tool call stayed pending
  (#1308, closed).
- **Usage per turn vs per session** differs between adapters (#390; ACP
  agentclientprotocol/agent-client-protocol#1860, open).
- **Thought chunks duplicated** on `/review` (zed-industries/codex-acp#126).

### What the spike and claude-codes do today
- `claude_codes::ClaudeOutput` fails on an unknown `type`
  (`other => Err(unknown_variant)`, CC `src/io/claude_output.rs:996`). The
  spike's `claude_line` returns that error. So the next new CLI frame fails the
  mapping, exactly as CCA #1532.

### Lessons for watane
- **Adopt** the rule "unknown in, nothing out": peek at `type` first; an
  unknown line or block maps to no item (and a counter), never to an error.
  Same for unknown SSE event types.
- **Adopt** "a malformed tool input is a failed tool step, not a failed turn"
  (CCA #1308).
- **Adopt** dedupe by id, not by content: Claude's assistant messages and
  content blocks carry ids; with v2 upserts the consolidated message simply
  replaces the streamed one. This matters as soon as `partial` is on (§11).
- **Adopt** a written loss table per backend, kept next to the mapper and
  tested. Start from CCA's dropped list and CXA's ignored list.
- **Adopt** vendor extras under a namespaced `_meta` key (`_claude/...`,
  `_openai/...`), as CCA does, rather than new item kinds.
- **Proposal change §10 (Adapters).** Add: "An adapter never fails on input it
  does not know; it records the loss". And: the checked provider's verdicts
  (§9) travel as a namespaced `_meta` on the tool call or as a
  `_watane/verdict` extension update, not as text.

---

## 5. Provider traits

**watane needs:** behind the kernel, one shape for "a backend": stream a turn,
call tools, carry reasoning, list models, describe what each model can do, so
the compiler can refuse per field (§11).

### Interfaces
- **Goose.** `Provider::stream(model_config, system, messages, tools) ->
  MessageStream` where each item is `(Option<Message>, Option<ProviderUsage>)`
  (GO `crates/oi-kit-provider-types/src/base.rs:532-560`, `:352`). Coarse:
  partial messages, not typed deltas. Per provider: `fetch_supported_models`,
  `fetch_model_info`, `get_context_limit`, `retry_config`,
  `thinking_effort_support`, `permission_routing`, `manages_own_context`.
  `ProviderDef::RUNS_OWN_TOOL_LOOP` and `SESSION_BOUND` mark CLI and ACP
  wrappers such as `claude_code.rs` (GO
  `crates/goose/src/providers/base.rs:36-40`).
- **rig.** A part-based stream: `Start{part, kind, name}`, `Text`,
  `Reasoning`, `Arguments` (raw JSON fragments), `End{part, content}`, plus
  `Item::Unmodeled` for payloads the decoder does not model (RIG
  `rig-core/src/streaming/event.rs:65-125, 227`). A recorded sequence is
  validated: `SequenceError::{UnknownPart, EndedTwice, WrongKind}`
  (`:261-275`). Options a model cannot honour: `OnUnsupported::{Error,
  Ignore}`, default `Error`, and `UnsupportedOption{option, provider, model,
  reason}`; `CheckError::Unsupported(Vec<_>)` lists all of them in one dry run
  (RIG `rig-core/src/completion/options.rs:344-400`). Every wire must answer
  for every option or fail to compile (`.../options/mapping.rs:1-4`).
- **Pi (TS).** `AssistantMessageEvent`: `start`, `text_start|delta|end`,
  `thinking_start|delta|end`, `toolcall_start|delta|end`, `done{reason}`,
  `error{reason}`, each with the partial message (PI `src/types.ts:831-847`).
  "Request failures after a stream is returned never throw" (PI
  `README.md:1152`).
- **Cline (TS).** Chunks `text`, `reasoning` (with `signature`,
  `redacted_data`), `usage`, `tool_calls`, `done{success, error,
  incompleteReason}` (CL `src/providers/stream.ts:17-129`).
- **genai.** `ChatStreamEvent::{Start, Chunk, ReasoningChunk,
  ThoughtSignatureChunk, ToolCallChunk, End, Heartbeat}` (GEN
  `src/chat/chat_stream.rs:159-178`). The `Adapter` trait is pure: it builds
  request data and parses responses; transport is separate (GEN
  `src/adapter/adapter_types.rs:14-46`).
- **golem-ai.** `StreamEvent::{Delta{content, tool_calls}, Finish}`; no
  reasoning; `FinishReason::{Stop, Length, ToolCalls, ContentFilter, Error,
  Other}` (GAI `llm/llm/src/model.rs:50-66, 345-366`). The WIT interface of
  0.5.x is replaced by golem agentic Rust types on main.
- **Forge.** `ChatRepository::chat(model, context, provider) ->
  ResultStream<ChatCompletionMessage>` and `models(provider)` (FG
  `forge_domain/src/repo.rs:99-106`).
- **Codex.** `ResponseEvent::{Created, OutputItemAdded, OutputItemDone,
  OutputTextDelta, ToolCallInputDelta, ReasoningSummaryDelta,
  ReasoningContentDelta, Completed{token_usage, end_turn}, RateLimits,
  ServerModel, ...}` (CDX `codex-api/src/common.rs:200-255`). `ServerModel`
  reports when the server routed to another model.
- **Zed (GPL, reference).** Events include `Queued`, `Started`,
  `StartMessage{message_id}`, `Thinking`, `RedactedThinking`,
  `ReasoningDetails`, `Stop` (ZED `language_model/src/language_model.rs:460-485`).

### Model listing and capabilities per model
- **Codex** reads the backend's `/models`: per model `supported_reasoning_levels`,
  `default_reasoning_level`, `context_window`, `input_modalities`,
  `supports_reasoning_summary_parameter`, `support_verbosity`,
  `supports_search_tool` (CDX `protocol/src/openai_models.rs:429-500`).
  `ReasoningEffort` has `None..Max, Ultra, Persistent, Custom(String)` for
  values the client does not know yet (`:75-88`). The client-facing
  `model/list` keeps `supported_reasoning_efforts` and `input_modalities` (CDX
  `.../v2/model.rs:142-173`).
- **Pi**: `Model{reasoning, thinkingLevelMap, contextWindow, maxTokens, input,
  cost, compat}`; in `thinkingLevelMap`, `null` marks a level unsupported (PI
  `src/types.ts:864-895`).
- **rig**: a layered catalog: built-in from models.dev, a newer models.dev
  copy, then user overrides (RIG `rig-core/src/catalog/mod.rs:1-10`).
- **Goose**: a canonical registry built from OpenRouter (GO
  `crates/oi-kit-provider-types/src/canonical/README.md`).

### Reasoning across turns
- Signatures and encrypted reasoning are valid only for the model that made
  them. Pi drops thinking from another model and keeps signed or redacted
  blocks for the same model (PI `src/api/transform-messages.ts:129-135`).
  It also drops errored or aborted assistant turns before replay (`:216-220`)
  and inserts synthetic results for orphaned tool calls (`:179-252`).
- Tool call ids: Responses ids can be 450+ characters with `|`; Anthropic wants
  `^[a-zA-Z0-9_-]{1,64}$` (PI `src/api/transform-messages.ts:87-90`).
- Goose hands a conversation to an ACP agent without a native session as one
  budgeted, redacted text memo (GO `crates/goose/src/acp/handoff.rs:1-12`).

### Sign in with ChatGPT specifics
- rig has a ChatGPT dialect: base `https://chatgpt.com/backend-api/codex`,
  every system message becomes `instructions`, no `store` field (RIG
  `rig-core/src/providers/chatgpt/mod.rs:25-72`, `.../extension.rs:35`).
- Pi **drops** fields that token sharing rejects (`max_output_tokens`,
  `temperature`) and tests it (PI `test/openai-responses-chatgpt-sign-in.test.ts:44-60`).
  watane refuses instead (§11); rig's default agrees with watane.
- codex-acp: "model is not supported when using Codex with a ChatGPT account"
  (zed-industries/codex-acp#223). The model list differs per plan.

### Lessons for watane
- **Adopt** rig's refusal shape for the compiler (§10, §11):
  `{field, backend, model, reason}`, all refusals of a profile in one list, and
  a dry-run `check` that a page can call before saving a profile.
- **Adopt** "refuse per model, not per backend". §11 refuses `xhigh` and `max`
  for OpenAI as a whole; codex shows the efforts are a per-model fact from
  `/models`. Unknown values (`Custom`) are refused, not passed.
- **Adopt** Pi's contract: once a turn started, failures are items on the
  stream, never a trap or a lost call. In watane this is natural: the kernel
  pushes an end item with the error.
- **Adopt** Pi's history hygiene for the OpenAI loop (§12 S5), which resends
  the whole history: drop failed or cancelled turns, synthesize results for
  orphaned tool calls (after S6 cancel or S7 restart), keep encrypted
  reasoning only for the same model.
- **Avoid** a trait. watane's backends are mods and host interfaces, not Rust
  types; a trait would live in one mod. Keep the abstraction as data: a
  backend description (fields honoured, models, per-model efforts) plus the
  ACP item stream. goose's `RUNS_OWN_TOOL_LOOP` is the distinction §9 and §12
  already draw between Claude and OpenAI.
- **Avoid** switching backend inside a session. §12 S7 resumes on the same
  backend; say so, because reasoning does not survive the switch.
- **Proposal change §11.** Add a row `reasoning effort per model` (from the
  plan's model list; refuse a level the model does not list) and a row
  `model` (refuse a model the plan's list does not contain, CXA #223).

---

## 6. Error taxonomy

**watane needs:** a small set of error kinds that clients can act on, with
vendor detail kept, and a clear line between "retrying", "turn failed" and
"call refused".

### Taxonomies
- **ACP v2 JSON-RPC codes**: -32700, -32600, -32601, -32602, -32603, -32800
  (request cancelled), -32000 (auth required), -32002 (resource not found,
  with `sessionId`), -32022 (unsupported protocol version, with `requested`
  and `supported`) (ACP `agent-client-protocol-schema/src/v2/error.rs:279-334`).
  In v2, an error on `session/prompt` always means "not inserted"; a failure
  later is `idle` with `stopReason: "error"`.
- **Codex** `CodexErrorInfo`: `ContextWindowExceeded`, `UsageLimitExceeded`,
  spend limits, `RateLimitExceeded`, `ServerOverloaded`, `ModelNotFound`,
  policy refusals (`CyberPolicy`, `BioPolicy`, ...), `Unauthorized`,
  `BadRequest`, `HttpConnectionFailed{httpStatusCode}`,
  `ResponseStreamConnectionFailed`, `ResponseStreamDisconnected`,
  `ResponseTooManyFailedAttempts`, `ActiveTurnNotSteerable`, and an untagged
  `Other` (CDX `app-server-protocol/src/protocol/v2/shared.rs:75-136`).
  `ErrorNotification{error, will_retry}`: `will_retry: true` does not end the
  turn (CDX `.../v2/notification.rs:62-69`).
- **Codex SSE parsing**: `response.failed` codes map to typed errors
  (`context_length_exceeded`, `insufficient_quota`, `rate_limit_exceeded`,
  `server_is_overloaded`, `invalid_prompt`, ...); "try again in 11.054s" is
  parsed into a delay (CDX `codex-api/src/sse/responses_error.rs:40-121`). A
  stream that closes before `response.completed` is an error (CDX
  `codex-api/src/sse/responses.rs:641`).
- **OpenHands**: the only failure data that crosses the boundary is
  `{kind: auth | quota | rate_limit | config | transient | agent_action |
  internal | unknown, retryable, user_action: none | retry | settings,
  error_id}`; the raw detail is used locally to classify and never sent (OH
  `event/error_classification.py:20-48`).
- **Goose** `ProviderError`: `Authentication`, `ContextLengthExceeded`,
  `RateLimitExceeded{retry_delay}`, `ServerError`, `NetworkError`,
  `RequestFailed`, `NotImplemented`, `EndpointNotFound`, ... (GO
  `crates/oi-kit-provider-types/src/errors.rs:9-48`).
- **golem-ai**: six codes, too coarse to act on (GAI `llm/llm/src/model.rs:28-41`).
- **OpenCode**: tagged API errors such as `SessionBusyError`,
  `UnauthorizedError`, `ServiceUnavailableError`, `ProviderNotFoundError`
  (OC `packages/protocol/src/errors.ts`).
- **CCA**: a failed turn's `data` carries `errorKind` and, for `rate_limit`,
  the latest rate-limit info with `resetsAt` (CCA `src/result-end.ts:15-30`);
  login-required is detected by the text "run /login" (`:46`).

### Pitfalls seen
- Context overflow is reported in many wordings; Pi tests every provider for
  it (PI `test/context-overflow.test.ts`), OpenHands keeps a pattern list per
  vendor (OH `llm/exceptions/classifier.py`).
- codex-acp hides retries: the client sees nothing while the stream
  reconnects (CXA `src/thread.rs:1361-1369`).
- Detecting login errors by text breaks when the CLI rewords it (CCA
  `src/result-end.ts:38-47` lists three wordings already).

### Lessons for watane
- **Adopt** OpenHands' closed vocabulary plus codex's `will_retry`: one error
  item `{kind, retryable, user_action, will_retry, message}` with vendor
  detail under `_meta` (`_openai/code`, `_claude/errorKind`,
  `httpStatus`). `user_action: settings` covers "sign in again" for both
  backends.
- **Adopt** "stream ended without a terminal event is an error" for H3 SSE
  (§5) and for `claude` exiting without `result` (§4).
- **Keep** three layers apart, as v2 does: a call refused (`say` while the
  session is unknown; stream errors like `not-yours`), a turn that failed
  after it started (end item with `error`), a transient retry (error item with
  `will_retry`).
- **Proposal change §10 WIT.** The `error` type of `mods:agent/sessions` names
  its cases (not-owner, unknown-session, refused-profile with the refusal
  list, busy if ever needed) instead of a string.

---

## 7. Versioning of the protocol

**watane needs:** a way to evolve the item format and the WIT without breaking
downloaded client mods (G2, D5).

### Practice
- **ACP**: integer `protocolVersion` negotiated in `initialize`; the agent
  picks the highest version at or below the client's, or answers -32022.
  Supporting v1 and v2 side by side is the stated path; routers pick per
  connection; "share application logic, not wire shapes" (ACP
  `docs/protocol/v2/migration.mdx` "Supporting v1 and v2 side by side").
  Features move through RFDs and Cargo feature flags; v2 makes almost every
  enum open with `_`-prefixed extensions.
- **SDK pinning**: goose pins `agent-client-protocol-schema = "=1.9.1"` (GO
  `Cargo.toml:22`); codex-acp pins `agent-client-protocol = "=0.14.0"` and a
  codex git tag (CXA `Cargo.toml:21-27`). The schema moves fast: 1.15 to 1.19
  in 16 days (ACP `CHANGELOG.md`).
- **Codex**: one stable surface plus an experimental opt-in per field; the
  generated JSON Schema and TypeScript are checked in and a test fails when
  they differ from the code (CDX
  `app-server-protocol/src/schema_fixtures_tests.rs:17-30`,
  `app-server-protocol/schema/`).
- **OpenCode**: OpenAPI generated from the server schema, `info.version
  0.0.1`, an "experimental" path prefix for unstable routes (OC
  `packages/protocol/openapi.json`).
- **claude-codes** follows Claude Code's version in its own version number
  (2.1.330); watane's §4.1 sets a minimum `claude` version.

### Pitfalls seen
- ACP docs and schema drifted (agentclientprotocol/agent-client-protocol#1694,
  #1251, #1249).
- Usage semantics are ambiguous across implementations (#1860).

### Lessons for watane
- **Adopt** one item schema per WIT package version: `mods:agent@0.1.0` items
  are ACP of one exact schema release. A new release that changes the wire is
  a new WIT minor version.
- **Adopt** codex's checked-in schema fixture: generate the JSON Schema of the
  item type the kernel emits (from `agent-client-protocol-schema`'s
  `schemars`), commit it, and fail the build on a diff.
- **Adopt** open enums on the receiving side (v2's `Other`), so a client built
  against `@0.1.0` survives an extension item.
- **Decision for the human, not taken here:** v2 is a draft. watane has no v1
  clients to keep, and v2 fixes replay, attach mid-message, turn state and
  errors after insertion. The cost is churn until v2 stabilizes. Option A: v2
  draft, pinned exactly, re-checked at each schema release. Option B: v1 now,
  with v2's id and upsert rules kept in the kernel's transcript so the move is
  a projection change. This record recommends A for stream C and the
  transcript, because watane controls both ends today.

---

## Lessons for watane, by proposal section

| § | Change | Source |
|---|---|---|
| 2.2, 10 | Name the ACP version of stream C; prefer v2 entity semantics (ids, upserts, `state_update`, `tool_call_content_chunk`) | ACP v2 migration guide |
| 3, 10 | `attach(after)` replays the partial current message as one upsert, then a replay-complete marker, then live items | ACP v2 `replay_complete`; §3's gap on deltas |
| 9, 12 S4 | An "ask resolved" item when the provider, the expiry or a cancel resolves an ask; pending asks are part of `read` and `attach` | CDX `serverRequest/resolved`; OC permission resources |
| 10 Adapters | Unknown Claude lines, blocks and SSE events map to nothing and are counted; never an error | CCA #1532; CC `claude_output.rs:996` |
| 10 Adapters | A malformed tool input fails the tool step, not the turn | CCA #1308 |
| 10 Adapters | Dedupe partial and consolidated Claude messages by id | CCA #785, #789, #800, #1149 |
| 10 Adapters | Tool output streams as appended chunks, never resent whole | CXA #225 |
| 10 WIT | `say` returns the inserted entry's index; `error` is a variant; `attach` takes client capabilities | ACP v2 prompt lifecycle |
| 10, 11 | Refusals as `{field, backend, model, reason}`, all at once, with a dry-run check | RIG `options.rs:344-400` |
| 11 | Effort and model refused per model from the plan's model list, not per backend | CDX `openai_models.rs:429-500`; CXA #223 |
| 12 S5 | OpenAI history hygiene: drop failed turns, synthesize orphaned tool results, keep encrypted reasoning only for its model | PI `transform-messages.ts` |
| 12 S5, 5 | An SSE stream that ends without `response.completed` ends the turn with an error item | CDX `responses.rs:641`; PI terminal-event tests |
| 12 S6, S7 | After cancel or restart emit `idle` (cancelled) or `unknown`; never replay old state as current | ACP v2 session states |
| 12 S7 | Resume stays on the same backend and model family; say what is lost otherwise | PI, GO handoff memo |
| 10 | One error item `{kind, retryable, user_action, will_retry}` with vendor detail in `_meta` | OH, CDX |
| 10 | Usage: `usage_update` for context used/size; token totals cumulative per session and labelled so; cost absent on plans (not zero) | ACP #1860, CCA #390 |

## Who does it best

| Capability | Best | Runner-up | Notes |
|---|---|---|---|
| Event / message model | ACP v2 | Codex app-server | v2's ids and upserts; codex's typed item lifecycle |
| Session lifecycle | ACP v2 | Codex app-server | Insert-then-stream, replay cursor, idempotency |
| Capability negotiation | Codex app-server | ACP v2 | Per-field experimental gate; notification opt-out |
| Mapping vendor streams | claude-agent-acp | codex-acp | CCA maps most and tests per client profile; CXA loses errors and retries |
| Provider abstraction: stream | rig | Pi | Part lifecycle plus sequence validation; Pi's errors-as-events |
| Provider abstraction: options | rig | Pi | Refusal by default, compile-time completeness |
| Model capabilities | Codex | Pi | Live per-model efforts; Pi's `null` for unsupported |
| Error taxonomy | Codex + OpenHands | Goose | Rich typed codes plus a small closed boundary set |
| Versioning | ACP | Codex | Negotiation and dual stack; checked-in schema fixtures |
| Test harness | claude-agent-acp, Codex, Pi | rig, OpenCode | Golden scenarios; SSE fixture builders; per-provider edge tests; cassettes |

## Test cases watane should write

Mapper tests run as pure functions over recorded lines and events, in the
kernel's own crate (native `cargo test`), plus one component test per backend.

Golden mappings:
1. Claude: each recorded stream-json line of `proto/agent-spike` runs →
   expected ACP items, stored as JSONL next to the input (CCA
   `src/tests/acp-scenarios/` pattern: input script, recorded output,
   comparator that allows named differences).
2. Claude with `partial` on: deltas then the consolidated message produce one
   message, not two; same for tool calls with streamed input.
3. Claude: an unknown line type (`post_turn_summary`), an unknown content block
   and an unknown `stream_event` produce no item and no error.
4. Claude: each `result` subtype × `stop_reason` × `is_error` maps as in CCA's
   table; "run /login" text gives `user_action: settings`.
5. Claude: a tool call with an invalid input gives a failed tool step and the
   turn continues.
6. Claude: `rate_limit_event` before the first usage is kept for the turn.
7. OpenAI: SSE fixtures built with codex-style builders (`ev_response_created`,
   `ev_output_text_delta`, `ev_function_call`, `ev_reasoning_item`,
   `ev_completed_with_tokens`, `sse_failed`; CDX
   `core/tests/common/responses.rs:799-1121`).
8. OpenAI: a stream that ends without `response.completed` → error item; a
   completed stream whose function call never got `output_item.done` → error;
   parallel calls without `output_index` → refused (PI
   `test/openai-responses-terminal-event.test.ts:243-439`).
9. OpenAI: each `response.failed` code (`context_length_exceeded`,
   `insufficient_quota`, `rate_limit_exceeded` with "try again in 11.054s",
   `server_is_overloaded`, `invalid_prompt`, policy codes) → the right error
   kind and retry delay (CDX `codex-api/src/sse/responses.rs:1105-1340`, copy
   the raw lines).
10. OpenAI: `response.incomplete` with `max_output_tokens` → `max_tokens`;
    with `content_filter` → `refusal` or a non-retryable error.

Stream properties:
11. Every recorded item sequence passes an order check: no update for an id
    before its creation, nothing after the end of its message, at most one end
    of turn (RIG `SequenceError`).
12. Replay equals live: `attach(0)` after a finished turn yields the same
    entities (by type and id, after applying upserts) as the live stream did.
13. Attach mid-message: the partial message arrives as one upsert before live
    chunks; the replay marker comes once.
14. A tool with 10 MB of output streams in bounded memory (CXA #225).
15. Cancel: last updates, then `idle` with `cancelled`; a pending ask gets a
    resolved item; a late `answer` is refused.
16. Restart: an old session reports `unknown` or `idle`, never `running`.

Compiler and refusal:
17. For each §11 row, a profile that sets the field on the backend that
    cannot honour it is refused with `{field, backend, model, reason}`; a
    profile with three bad fields gets three refusals.
18. Effort per model: a level missing from the model's list is refused; a
    level the client does not know is refused.
19. Under the ChatGPT plan, no request ever contains `temperature`,
    `max_output_tokens`, `previous_response_id` or a `system` role (PI
    `test/openai-responses-chatgpt-sign-in.test.ts`; watane refuses instead of
    dropping, so the test checks the refusal and the request body).

History hygiene (OpenAI loop):
20. A cancelled turn is dropped from the next request; an orphaned
    function call gets a synthetic output; encrypted reasoning from another
    model is dropped (PI `test/transform-messages-cross-model-thinking.test.ts`,
    `test/tool-call-without-result.test.ts`).

Versioning:
21. The JSON Schema of the emitted item type equals the committed fixture (CDX
    `schema_fixtures_tests.rs`).
22. A client built against the pinned schema parses an item with an unknown
    `sessionUpdate` and an unknown `_`-prefixed enum value without failing.

## Reusable code

"wasm" means `wasm32-wasip2` under Wasmtime 48. "Built" means the spike built
it; "likely" means judged from dependencies only.

| What | Path | Licence | wasm | Use |
|---|---|---|---|---|
| ACP schema types v1 | crate `agent-client-protocol-schema` 1.19.0 (ACP `agent-client-protocol-schema/src/v1`) | Apache-2.0 | built (spike) | The item types; pin exactly |
| ACP schema types v2 | same crate, feature `unstable_protocol_v2` (`src/v2`, `MaybeUndefined` in `src/serde_util.rs:613`) | Apache-2.0 | likely: same deps as v1; build before adopting | Items with ids and upserts |
| ACP JSON Schemas | ACP `schema/v1/schema.json`, `schema/v2/schema.json` | Apache-2.0 | n/a (test data) | Validate emitted items in native tests |
| Claude stream-json types | crate `claude-codes` 2.1.330, feature `types` | Apache-2.0 | built (spike) | Parse lines; wrap to skip unknown `type` |
| Claude → ACP rules | CCA `src/sdk-to-acp.ts`, `src/result-end.ts`, `src/context-window.ts` | Apache-2.0 | port (TS) | Port the tables, not the code |
| Golden scenario harness | CCA `src/tests/acp-scenarios/` (harness, compare, JSONL) | Apache-2.0 | n/a (test pattern) | Copy the pattern and the comparator rules |
| Responses event parser and error mapping | CDX `codex-api/src/sse/responses.rs`, `responses_error.rs` | Apache-2.0 | no (tokio net, rustls, tungstenite); the parsing functions port; `regex-lite` is wasm-safe | Port event → item mapping, error codes, retry delay |
| Responses SSE fixtures | CDX `core/tests/common/responses.rs` (`ev_*`, `sse`, `sse_failed`), raw lines in `codex-api/src/sse/responses.rs` tests | Apache-2.0 | n/a (test) | Copy builders and recorded lines |
| Mock model server sequence | CDX `app-server/tests/common/mock_model_server.rs` | Apache-2.0 | native test only (wiremock) | Host-side tests of H3 |
| Item and error schema ideas | CDX `app-server-protocol/src/protocol/v2/{item,shared,notification}.rs` | Apache-2.0 | crate no (rmcp, zstd) | Reference for error kinds and item lifecycle |
| Stream sequence validator | RIG `rig-core/src/streaming/event.rs:227-275` | MIT | rig-core no (per `research_notes/calling-agents.md`); the validator is plain Rust | Port as a test helper |
| Refusal types | RIG `rig-core/src/completion/options.rs:344-400` | MIT | port | Shape of the compiler's refusals |
| SSE parser | FG `forge_eventsource_stream` (deps: nom, futures-core, pin-project-lite) | Apache-2.0 | likely | Alternative to `sse-stream` |
| JSON repair for partial tool args | FG `forge_json_repair` (deps: serde_json, regex, schemars, serde_json5) | Apache-2.0 | likely | Show partial tool input; never to run a tool |
| Provider types, `MaybeSend` | GO `crates/oi-kit-provider-types` | Apache-2.0 | no (reqwest; wasm cfgs target the browser) | Reference only |
| Record/replay provider | GO `crates/goose/src/providers/testprovider.rs` | Apache-2.0 | native test | Pattern for kernel tests with a fake backend |
| Cross-provider history rules | PI `src/api/transform-messages.ts` | MIT | port (TS) | Rules for §12 S5 |
| Edge-case tests | PI `test/openai-responses-*.test.ts`, `test/context-overflow.test.ts`, `test/streaming-tool-arguments.test.ts` | MIT | port (TS) | Test list |
| Failure vocabulary | OH `event/error_classification.py` | MIT | port (Python) | The error item's `kind` and `user_action` |
| HTTP cassettes | OC `packages/http-recorder`, RIG `crates/rig-cassette` | MIT | n/a (TS, native Rust) | Pattern for recorded OpenAI traffic |
| LLM types for components | GAI `llm/llm` | Apache-2.0 | no: wasi:http 0.3 (`wasip3 =0.7.0`) | Not suitable: no reasoning, six error codes |
| Language model events | ZED `crates/language_model` | GPL-3.0-or-later | – | Reference only, never copy |
