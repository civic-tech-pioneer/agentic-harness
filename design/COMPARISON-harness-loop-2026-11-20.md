# The agent loop and tool execution: what other harnesses do (2026-11-20)

A record, not a contract. It compares how existing agent harnesses run the
turn loop and execute tools. It serves the OpenAI loop in the kernel
(`ARCHITECTURE-PROPOSAL.md` 10, S5) and the Claude Code adapter (9, 10). Claude
runs its own loop inside `claude`, so for Claude only the edges matter: limits,
results, usage and errors on stream-json.

Method: shallow clones, read by hand; nothing was built or run. GitHub issues
and PRs were read with `gh`, read-only. Clones live in the session scratchpad,
`research/<repo>`.

| Harness | Repo @ commit | License | Reuse in watane |
|---|---|---|---|
| Codex | openai/codex @bb5b9c6, `codex-rs` | Apache-2.0 | yes, with NOTICE |
| OpenCode | sst/opencode @b7f0b90 (now anomalyco) | MIT | yes (TypeScript: port) |
| Goose | block/goose @80ce943 (now aaif-goose) | Apache-2.0 | yes, with NOTICE |
| Pi | badlogic/pi-mono @e5d93ea (now earendil-works/pi) | MIT | yes (TypeScript: port) |
| OpenHands | OpenHands/OpenHands @3df9648 (frontend only); the loop is in OpenHands/software-agent-sdk @d5b327b | MIT | yes (Python: port) |
| Gemini CLI | google-gemini/gemini-cli @d4cb171 | Apache-2.0 | yes (TypeScript: port) |
| Cline | cline/cline @20b0427 (`sdk/` rewrite) | Apache-2.0 | yes (TypeScript: port) |
| rig | 0xPlaygrounds/rig @0019e24 | MIT | yes |
| golem-ai | golemcloud/golem-ai @0923caf | Apache-2.0 | yes |
| Zed agent | zed-industries/zed @10d1b2a, `crates/agent` | **GPL-3.0-or-later** | **no code**: ideas only |
| Forge | antinomyhq/forge @a22b492 (now tailcallhq/forgecode) | Apache-2.0 | yes |
| swiftide | bosun-ai/swiftide @92e1a30 | MIT | yes |
| AutoAgents | liquidos-ai/AutoAgents @d1f8cc5 | MIT or Apache-2.0 | yes |

watane's crates carry no licence field. `deny.toml` allows only permissive
licences (MIT, Apache-2.0, MPL-2.0 and similar). Copied Apache-2.0 code keeps
its licence header and needs a NOTICE line. Zed's GPL code must not be copied.

Path prefixes used below: pi = `packages/`, OC = `packages/`, GC =
`packages/core/src`, CL = `sdk/packages`, OH-SDK = `openhands-sdk/openhands/sdk`.

## 1. The turn loop and its state machine

**watane needs:** a loop that advances one small step per call, from state that
survives between calls (10 s per call, a fresh instance each time).

- **rig** (`crates/rig-agent/src/run/mod.rs`). `AgentRun` is a serializable,
  sans-I/O state machine. The driver asks for the next action and feeds results
  back. It owns no model, tool or hook.
  ```rust
  pub enum AgentRunStep { CallModel { prompt, history, turn }, CallTools { calls: Vec<PendingToolCall> }, Done(PromptResponse) }
  pub fn next_step(&mut self) -> Result<AgentRunStep, PromptError>;
  pub fn model_response(&mut self, turn: ModelTurn) -> Result<ModelTurnOutcome, PromptError>;
  pub fn answer(..); pub fn answer_all(..);
  ```
  `next_step` is idempotent while it waits: after a restart it re-issues the
  same model call without spending a turn, or the tool step with only its
  unanswered calls (mod.rs:805-830). The envelope has a `format` number and
  refuses any other (`RUN_FORMAT = 7`, mod.rs:338). Tests: `run/tests.rs`
  (2597 lines) drive the machine with literal `ModelTurn`s; `rig-cassette`
  records and replays traffic. **Pitfall:** the format went from 5 to 7 within
  a month (CHANGELOG 0.46: "drain or discard older persisted runs"). Copy the
  design, not the crate.
- **Goose** (`crates/oi-kit-agent/src/machine.rs`). A re-entrant pipeline of
  `Operation`s over the persisted conversation. `step` runs operations in
  order until one applies, then returns effects; `run` loops load, step,
  apply. State is derived from the conversation, not kept in counters.
  ```rust
  pub enum RunStatus { Continuing, Yielded, Cancelled }
  pub async fn step(&self, session: &S, emit: &Emitter) -> Result<StepResult<E>>;
  ```
  The crate has `cfg(target_arch = "wasm32")` paths (`async_trait(?Send)`,
  `MaybeSend`). Tests: `crates/goose/src/agents/state_machine/tests/` with a
  wiremock `dummy_api.rs` that scripts replies, tool calls, output limits and
  HTTP errors.
- **OpenCode** (OC `core/src/session/runner/llm.ts:63-225`, `runner/step.ts:30-41`).
  `SessionStep.attempt` returns a tagged outcome:
  `Completed{needsContinuation} | Retry | RecoverFull | Compacted | OutputLimit`.
  A drain returns `Complete | Moved | Reloaded{continuation:{step}}`. Fake:
  `TestLLM` (`ai/src/testing.ts`); 313 recorded HTTP cassettes in
  `ai/test/fixtures/recordings`.
- **Pi** (pi `agent/src/agent-loop.ts:172-330`). Two nested loops over
  `AgentContext.messages`; no hidden state, so `agentLoopContinue` resumes from
  the messages (80-109). Fake: `ai/src/providers/faux.ts` (queued responses,
  `fauxToolCall`).
- **Codex** (`core/src/session/turn.rs:227`, `run_sampling_request` 2100,
  `try_run_sampling_request` 3095). A long async function per turn; the model
  step reports `needs_follow_up`. Not resumable mid-turn. Tests: wiremock with
  SSE builders (`core/tests/common/responses.rs`: `ev_function_call`,
  `ev_completed_with_tokens`, `sse_failed`, `mount_sse_sequence`), suites
  `tool_parallelism.rs`, `retry_after.rs`, `abort_tasks.rs`, `truncation.rs`.
- **Cline** (CL `agents/src/agent-runtime.ts:801-1106`). One `while` over
  iterations; status `idle|running|completed|aborted|failed`. Fake:
  `ScriptedModel` (agent-runtime.test.ts:30-51), one scripted stream per
  request, and it records each request.
- **Gemini CLI**. The tool loop lives in the caller
  (`packages/cli/src/nonInteractiveCli.ts:358-711`); core runs one model call
  (GC `core/turn.ts:270`). Per-call states: Validating, Scheduled, Executing,
  AwaitingApproval, Success, Error, Cancelled (GC `scheduler/types.ts:89-206`).
  Fake: `FakeContentGenerator` reads JSONL responses (`--fake-responses`,
  `integration-tests/*.responses`).
- **OpenHands SDK**. `ConversationExecutionStatus`: idle, running, paused,
  waiting_for_confirmation, finished, error, stuck (OH-SDK
  `conversation/state.py:51-63`). Fake: `TestLLM` served as an OpenAI endpoint
  with named trajectories (OpenHands `tests/e2e/mock-llm/scripts/mock-llm-server.py`).
- **Zed** (GPL). `run_turn_internal` races model events, tool completions and
  cancellation with `select_biased!` (thread.rs:3086-3130). Fake:
  `FakeLanguageModel` in `crates/agent/src/tests/mod.rs`.
- **swiftide** (`swiftide-agents/src/state.rs`). `State { Pending, Running,
  Stopped(StopReason) }`, serializable. **AutoAgents**
  (`crates/autoagents-core/src/agent/executor/turn_engine.rs:148`):
  `run_turn` returns `TurnResult::Continue | Complete`.

**Best:** rig's step protocol, with Goose's rule that state comes from the
conversation. Both fit a kernel that does one step per call.

## 2. Stop conditions and max turns

**watane needs:** a bound per user input, a stop on loops, and a clear outcome
the owner can show.

- **OpenCode**: per-agent `steps`. At the limit it adds a "max steps" assistant
  message and sets `toolChoice: "none"` but keeps the tool list, so the prompt
  cache holds (llm.ts:261-280). After 3 `length` finishes in a row it fails
  (llm.ts:38-39, 346-376). Its old `doom_loop` setting is a no-op: one subagent
  made 364 identical grep calls in 50 minutes (anomalyco/opencode#45442,
  #59339).
- **Goose**: `MaxTurnsOperation` (`oi-kit-agent/src/max_turns.rs`) counts
  assistant turns since the last user input. From half the budget it adds
  `<turn-budget>7/10 used</turn-budget>` to the prompt. At the limit it yields
  "I've reached the maximum number of actions…".
- **Gemini CLI**: `MAX_TURNS = 100` (GC `core/client.ts:86`).
  `LoopDetectionService`: the same tool call 5 times, the same content chunk 10
  times, an LLM judge after 30 turns. First detection sends feedback; the
  second stops (client.ts:791-807). **Pitfall:** recovery replaced a real tool
  result with "lost or interrupted", so the model redid side effects
  (gemini-cli#29815).
- **Forge**: `max_requests_per_turn` and `max_tool_failure_per_turn`
  (`crates/forge_app/src/orch.rs:399-430`). Each failed tool result gets an
  "attempts left" note. `DoomLoopDetector` (`forge_app/src/hooks/doom_loop.rs`)
  finds `[A,A,A]` and `[A,B,C][A,B,C][A,B,C]`, threshold 3, from the history.
- **Cline**: identical consecutive calls, soft 3 and hard 5
  (CL `core/src/runtime/safety/loop-detection.ts:66-87`).
- **OpenHands SDK**: `max_iteration_per_run = 500`, a cost budget, and a
  `StuckDetector` (repeated action/observation, repeated errors, monologue)
  that ends in status `stuck` (OH-SDK `conversation/impl/local_conversation.py:295, 858-920`).
- **swiftide**: `limit` on loops and `tool_retry_limit` per hash of tool name
  and arguments (`agent.rs:96-114, 780`).
- **rig**: `max_turns` raises `PromptError::MaxTurns` with the history.
- **Pi, Codex**: no turn limit.
- **Claude Code**: `--max-turns` ends with `subtype: "error_max_turns"`. It was
  not enforced consistently (claude-code#105974: cap 12, 26 turns, `success`).
  `--max-budget-usd` is checked after each call, so a $1 cap stopped at $1.38
  (claude-code#100111).

**Best:** OpenCode's soft stop (tools off, cache kept) plus Forge's
history-based loop detector.

## 3. Tool-call dispatch and result order

**watane needs:** many `agent-tools.call` jobs in flight, results in a fixed
order, and exactly one result for every call.

- **Codex** (`core/src/tools/parallel.rs:222-360`). Each tool says whether it
  supports parallel calls. A parallel call takes a read lock, an exclusive one
  a write lock, so exclusive calls wait for the others. Results are collected
  in call order with `FuturesOrdered` (turn.rs:3220). Unknown tool:
  `"unsupported call: {name}"` (`tools/registry.rs:1290`).
- **Pi** (agent-loop.ts:525-692). Parallel by default; sequential if the config
  or any called tool says so. Preparation (validation, hooks) runs in call
  order; execution runs together. Events come in completion order; results go
  to history in call order (test "emit tool_execution_end in completion order
  but persist tool results in source order", agent-loop.test.ts:721). If the
  response stopped on `length`, no call runs: "salvaged arguments may be
  silently incomplete" (488-513).
- **OpenCode**: a tool starts as soon as its call has streamed in
  (step.ts:145-159); history is ordered by assistant content
  (`to-llm-message.ts:217-222`).
- **Gemini CLI**: contiguous parallel-safe calls run together; edit tools are
  never parallel; the model can pass `wait_for_previous: true`
  (GC `scheduler/scheduler.ts:545-588`, `utils/tool-utils.ts:29-41`).
- **Cline**: per-tool `executionMode`, default sequential; sequential calls are
  barriers (agent-runtime.ts:2370-2405).
- **Forge**: `task` calls run in parallel; all others run one after another
  (orch.rs:58-110).
- **Goose**: `ToolCallBatch` merges the tool streams with `select_all` and
  records each result as it finishes, so a cancelled turn keeps what completed
  (`oi-kit-agent/src/mcp/batch.rs:42-100`).
- **swiftide**: `tokio::spawn` per call, awaited in call order (agent.rs:639-700).

Every harness turns a tool failure into a tool result with an error flag. None
lets a failure end the turn, except swiftide and Forge after a retry limit.

**Pitfalls.** A call without a result makes every later request fail with 400
("No tool output found for function call", codex#586, PR #611; opencode#57373
for an errored call). Claude Code has 150+ reports of the same mismatch
(claude-code#6836).

**Best:** Codex's per-tool parallel flag and Pi's "prepare in order, run
together, record in order".

## 4. Tool schemas and argument validation

**watane needs:** schemas from the WIT of tool mods, checked before a job
starts, with errors the model can act on.

- **Pi** (`ai/src/utils/validation.ts:15, 54-69`): JSON Schema compiled with
  coercion (`"42"` to number, one value to an array, null to omitted).
  Malformed JSON goes through `parseJsonWithRepair` (`json-parse.ts:106-119`).
  The error text includes the received arguments.
- **rig** (`crates/rig-core/src/tool/validate.rs`, `tool/feedback.rs`): its own
  validator. Problems name a JSON pointer: `"/limit: expected an integer, got a
  string"`, `"/path: missing"`. Feedback helpers: `invalid_arguments_feedback`,
  `unknown_tool_feedback`, `not_executed`. `max_invalid_tool_call_retries` and
  `max_consecutive_malformed_tool_calls` bound repeats.
- **OpenCode** (OC `core/src/tool/runtime.ts:100-118`): lists the first 5
  issues and ends with "Update the arguments and call the tool again".
  Malformed JSON gets an excerpt of at most 2048 characters.
- **Gemini CLI** (GC `utils/schemaValidator.ts:85-101`): Ajv. **Pitfall:** if
  the schema does not compile, validation is skipped silently.
- **Codex**: no schema check at dispatch; each handler parses with serde and
  answers `"failed to parse function arguments: {err}"`. It cleans incoming
  schemas (`tools/src/json_schema.rs`: sanitize, prune unused `$defs`, compact
  large ones).
- **Forge** (`crates/forge_json_repair`): `json_repair` and
  `coerce_to_schema`; depends only on serde_json, regex, schemars, json5.
- **Cline**: no central check; each tool validates with zod.

**Best:** rig's pointer-named problems with Pi's coercion.

## 5. Tool output truncation

**watane needs:** a bound on what reaches the model, while the full result
stays in the transcript.

| Harness | Limit | Shape | Full output |
|---|---|---|---|
| Codex | per model, default 10 000 bytes (`models-manager/src/model_info.rs:127`), or a token budget | head and tail, cut on char boundaries, "…N tokens truncated…" (`utils/string/src/truncate.rs`) | rollout only |
| OpenCode | 2000 lines or 50 KiB | head, "[showing lines 1-N of M; full output saved to …]" | file, kept 7 days (OC `core/src/tool-output.ts:13-166`) |
| Pi | 2000 lines or 50 KB; grep lines 500 chars | tail for bash | temp file path in the text (`coding-agent/src/core/tools/truncate.ts:11-13`) |
| Gemini CLI | min(40 000 chars, 4 × free context) | first 20 %, last 80 % | file (GC `utils/fileUtils.ts:814-841`) |
| Cline | 48 000 chars; 8 000 per result in the request | middle cut, "refine the command" | none (CL `core/src/extensions/tools/executors/output-limits.ts:20-60`) |
| Goose | 200 000 chars | whole text to a file | file (`agents/large_response_handler.rs:5`) |

**Pitfalls.** UTF-8 boundary panics (codex#35381, #34282); a notice that said
"0B" (pi#10015); Claude Code cuts MCP errors above ~11k chars to 5 000 + 5 000
with nothing saved (claude-code#97623). Cline's comment: an oversized result
costs "quadratically" over a run, because it is resent each turn.

**Best:** Codex's `truncate_middle_with_marker` (hard byte limit, char
boundaries, pure Rust).

## 6. Cancellation and interrupt mid-turn

**watane needs:** `cancel` that stops the HTTPS stream, drops late job results
and leaves a history the next request accepts.

- **Codex**: an interrupted call gets the output `"aborted by user after
  {secs}s"` (`tools/parallel.rs:535-537`). Before every request,
  `ensure_call_outputs_present` adds `"aborted"` for any call without output and
  `remove_orphan_outputs` drops outputs without a call
  (`core/src/context_manager/normalize.rs:21-243`). PR #60519 does the same for
  restored and forked histories.
- **Goose**: `finalize` writes "Tool call was interrupted before completing"
  for every pending call, then lets each operation clean up
  (`oi-kit-agent/src/machine.rs:84-96, 260-300`).
- **OpenCode**: a stop asks each tool to cancel, then interrupts it after 5 s;
  pending calls fail with "Tool execution interrupted" (step.ts:66-117). On the
  next start, stale running tools are failed (llm.ts:389-442).
- **Pi**: once aborted, every remaining call gets "Operation aborted" so "no
  call is left without one" (agent-loop.ts:716-724); at send time orphan calls
  get "No result provided" (`ai/src/api/transform-messages.ts:179-244`).
- **Gemini CLI**: `hardenHistory` adds sentinel results and calls
  (GC `utils/historyHardening.ts`); the comment says an unpaired call "corrupts
  every subsequent request".
- **rig**: "Cancellation histories close unanswered calls" (CHANGELOG 0.46).
- **Cline**: missing results are filled at request time: "Tool execution was
  interrupted before a result was produced." (`message-builder.ts:55`).

**Best:** Codex: a synthetic result at cancel time and a normaliser before
every request, so a bug in one path cannot poison the session.

## 7. Retries and backoff

**watane needs:** retries that never sleep inside a 10 s call, never repeat a
tool call, and never stack.

- **Codex** (`core/src/responses_retry.rs`, `protocol/src/error.rs:481-540`,
  `codex-client/src/retry.rs:43`). Each error has a policy:
  `Never | ServerDirected | Backoff | FallBackToHttp`. The delay is the
  server's (`retry-after`, or "try again in 1.898s" parsed from the message,
  `codex-api/src/sse/responses_error.rs:121`), else `base × 2^(n-1)` with
  ±10 % jitter. Defaults: 4 request retries, 5 stream retries, a stream idle
  timeout of 300 s (`model-provider-info/src/lib.rs:73-80`). Terminal outcomes
  are kept across retries, steering and fallback: `ResponseIncomplete`,
  `UsageAccountingFailed`, budget and spend limits
  (`is_terminal_sampling_outcome`). When retries run out it stores the server's
  `retry_at` for later. A retry resends the same request; tool calls of a
  failed attempt never ran, because tools run only from items of a stream.
- **OpenCode** (OC `core/src/model-retry.ts`, `ai/src/schema/errors.ts:197-229`):
  from 2 s, at most 10 s per gap, 10 retries; `retry-after` capped at 15
  minutes; never retries auth, quota, content policy, invalid request. A retry
  is silent only if no output had started; otherwise the attempt stays and a
  fresh message starts (step.ts:255-318).
- **Pi** (`ai/src/utils/provider-retry.ts`, `retry.ts`): honours
  `retry-after-ms`, `retry-after` (seconds or date); a server delay above 60 s
  fails instead of sleeping. Comment: SDK retry timers ignore the abort
  signal, so call the SDK with `maxRetries: 0`.
- **Cline**: an SDK `RetryError` is terminal, so the SDK's 3 retries and the
  runtime's 3 do not multiply (CL `llms/src/providers/error-classification.ts:436-458`).
- **Gemini CLI**: 10 attempts, 5 s to 30 s, ±30 % jitter; a separate budget of
  4 for mid-stream errors (GC `utils/retry.ts`, `core/geminiChat.ts:117-121`).
- **Zed** (GPL): per error, `ExponentialBackoff` (4 tries, base 5 s) or
  `FixedDelay`; a stream that "ended unexpectedly" gets one retry. After a
  retry with no tool results it appends a `Resume` message (thread.rs:182-210,
  3300-3330, 4841-4890).
- **Goose** (`oi-kit-provider-types/src/retry.rs`): 3 retries, 1 s, ×2, at
  most 30 s, ±20 % jitter. Some 4xx are permanent because the same payload is
  rebuilt every time, for example "encrypted content … could not be verified".
- **golem-ai** (`llm/llm/src/durability.rs:26-78`): after a crash mid-stream it
  replays the recorded deltas, then asks the model to "continue from where you
  left off", with the partial answer pasted in as text.

**Pitfalls.** "stream disconnected before completion" (codex#62169, #14613); a
silent stream found only after 300 s (codex#33051); Pi retried 400 and auth
errors as network errors (pi#12310); a missing terminal event led a client to
auto-continue and duplicate output (opencode#43617); codex lost the turn input
on retry (codex#586).

**Best:** Codex's per-error policy with server-directed deadlines; OpenCode's
"no silent retry after output started".

## 8. Rate limits and subscription usage limits

**watane needs:** to tell "wait and retry" from "the plan is used up until a
time", and to show the reset time.

- **Codex** parses the ChatGPT 429 body into a typed error
  (`codex-api/src/api_bridge.rs:231-270`):
  ```json
  {"error":{"type":"usage_limit_reached","plan_type":"plus","resets_at":1793390439,
            "limit_window_minutes":300,"resets_in_seconds":4454}}
  ```
  Headers `x-codex-primary-used-percent`, `-window-minutes`, `-reset-at`, the
  same for `secondary` (the weekly window), `x-codex-credits-*`,
  `x-codex-active-limit`, `x-codex-promo-message` (`codex-api/src/rate_limits.rs`).
  `usage_not_included` and `insufficient_quota` are separate. All are
  `RetryPolicy::Never`; `rate_limit_exceeded` and `server_is_overloaded` are
  retried. Body captured in codex#58413. **Pitfalls:** the same limit inside a
  streamed `response.failed` lost its reset time (codex#62416); headers were
  dropped when `x-codex-active-limit` named another limit.
- **Pi** (`ai/src/api/openai-codex-responses.ts:1648-1668`): "You have hit your
  ChatGPT usage limit (plus plan). Try again in ~N min." Non-retryable codes:
  `subscription_sharing_usage_limit_exceeded`, `insufficient_quota` and more
  (`ai/src/utils/retry.ts:7-35`). Lost the reset time on websockets (pi#10205).
- **OpenCode**: "ChatGPT usage limit reached. Check ChatGPT Settings → Usage"
  (OC `core/src/session/to-session-error.ts`); vetoes retry for
  `subscription_sharing_*` (`core/src/plugin/provider/chatgpt.ts:58-67`).
- **Gemini CLI** (GC `utils/googleQuotaErrors.ts:210-418`): a table of
  retryable versus terminal quota errors; a retry delay above 300 s is
  terminal.
- **Claude Code** stream-json emits `rate_limit_event`
  (claude-code#91857, #103820):
  ```json
  {"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1788465600,
   "rateLimitType":"five_hour","overageStatus":"allowed","isUsingOverage":false}}
  ```
  `status` is `allowed`, `allowed_warning` or `rejected`. **Pitfalls:** at the
  weekly limit `-p` exited 1 with no `result` at all (#103820); a rate-limit
  failure came as `subtype: "success"` with `is_error: true` (#79500); a
  `rate_limit_event` was once written inside another JSON line (#49640).

We have not checked whether `api.openai.com/v1/responses` under Sign in with
ChatGPT sends the `x-codex-*` headers or the same 429 body as
`chatgpt.com/backend-api/codex`.

**Best:** Codex's typed `UsageLimitReachedError`, parsed on both the HTTP and
the SSE path.

## 9. Token and cost accounting

**watane needs:** usage per finished response in the transcript; no price for
plans.

- **Codex** (`protocol/src/protocol.rs:2604`; mapping in
  `codex-api/src/sse/responses.rs:123-149`):
  `input_tokens, cached_input_tokens, cache_write_input_tokens, output_tokens,
  reasoning_output_tokens, total_tokens`, plus `last_token_usage` and
  `non_cached_input()`. It comes from `response.completed.response.usage`
  (`input_tokens_details.cached_tokens`, `output_tokens_details.reasoning_tokens`).
- **OpenCode** (OC `ai/src/schema/events.ts:64-93`): also
  `visibleOutputTokens = output − reasoning`; a provider cost, even 0, beats the
  price table.
- **Pi** (`ai/src/types.ts:467-495`): input, output, cacheRead, cacheWrite,
  reasoning; cost per field.
- **Cline**: cache-write price defaults to 1.25 × input.
- **golem-ai**: only input, output, total, each `Option<u32>`. Too thin.

**Pitfalls.** Cached tokens counted twice after an SDK change
(opencode#19757); Claude Code writes one response as several lines that repeat
the same `usage`, so sums double unless deduped by `message.id`
(claude-code#110387); rig summed a missing counter as 0 (rig#3098).

**Best:** Codex's `TokenUsage`, with "unknown" kept apart from 0.

## 10. Structured output

**watane needs:** `json-schema` for both backends (proposal 11), and a clear
failure when the answer does not fit.

- **Codex**: `text.format = {type: json_schema, strict: true, name:
  "codex_output_schema"}` (`codex-api/src/common.rs:535-552`). Pitfalls: the
  schema applied to intermediate messages too (codex#19816); `minLength`
  made some models stream whitespace forever (codex#54285).
- **Goose** (`oi-kit-agent/src/final_output.rs`): a `final_output` tool,
  checked with the `jsonschema` crate; it must be called alone; if the model
  stops without it, "You MUST call the `final_output` tool NOW".
- **rig**: `with_output_validation(schema)`; text or output-tool arguments are
  checked; `max_output_retries` re-prompts; then `OutputSchemaMismatch`.
- **OpenCode**: a hidden answer tool with `toolChoice` set to it.
- **Cline**: `submit_and_exit` with `completesRun`, plus reminders.
- **Gemini CLI**: subagents call `complete_task`, checked with zod.
- **Claude Code** `--json-schema`: the answer is in `result.structured_output`.
  Pitfalls: `error_max_structured_output_retries`; a loose schema accepted
  placeholder text (claude-code#77026); a `tools` allowlist dropped the
  internal tool and the run returned `success` without `structured_output`
  (#82258). Treat a missing key as failure.

**Best:** native `text.format` for OpenAI plus the kernel's own check and a
bounded re-prompt (rig); for Claude, check the key.

## 11. Splitting the loop into short calls

**watane needs:** each step within one call, nothing slept, the state written
only at transitions (proposal 3: forwarding writes nothing).

- **rig**'s `AgentRun` does exactly this: the run is data; `next_step` says
  "call the model" or "run these tools"; answers are fed back one by one. A
  re-issued model call is the same attempt and costs no turn.
- **Goose** derives turn counts, retries and pending calls from the
  conversation (`assistant_turn_count`, `pending_tool_requests`,
  `response_recovery_attempts`). Nothing else needs saving.
- **OpenCode** keeps a durable per-turn resume counter (at most 10) and, after
  a restart, tells the model "The server restarted while you were working.
  Continue from where you left off" (OC `core/src/session/execution/restart.ts:30-123`).
- **Cline** writes the whole message list after each iteration; a crash loses
  the current turn's tool results.
- **golem-ai** records every stream poll in an oplog and replays it into a
  fresh instance. watane has no oplog; its transcript keeps finished entries
  only.

A Responses stream with `store: false` cannot be resumed. An attempt cut off
mid-stream must be discarded and sent again.

## 12. Other findings: Responses with `store: false`

- Reasoning items must carry `encrypted_content`. A bare `{id:"rs_…",
  summary:[]}` gives "Item with id 'rs_…' not found. Items are not persisted
  when store is set to false" (pi#6950, opencode#8746). Once it happens, every
  later request fails.
- Replaying item ids broke requests behind a load balancer. Pi now strips ids
  and replays only `encrypted_content` (pi#9966).
- Encrypted reasoning fails after a model switch ("could not be verified").
  Goose classifies it as permanent and retries once without reasoning items
  (`oi-kit-providers/src/formats/openai_responses.rs:91-102`). OpenCode replays
  reasoning only to the same model (`to-llm-message.ts:197-212`).
- `response.incomplete` with reason `max_output_tokens`: Codex fails the turn
  with the partial output (`codex-api/src/sse/responses.rs:474-512`); Pi runs
  none of its calls.
- The spike (`proto/agent-spike/guest/src/lib.rs:140`) appends the raw output
  items, ids included, to the next request. It worked once, but it is the
  shape that broke Pi.

## Lessons for watane

### Adopt

1. **A run record in the kernel's state** (10). Model it on rig's `AgentRun`:
   `AwaitingModel{attempt}`, `AwaitingTools{calls, answers by call id}`,
   `RetryAt{time, attempt}`, `Done`, `Failed{reason}`. Each item call advances
   it. Write it only on a transition (a completed response, a job result, an
   end), never per delta. Derive counts from the transcript where possible
   (Goose). Put a format number in it and refuse others.
2. **One result for every call, in call order.** Jobs may finish in any order;
   the kernel stores answers by call id and appends them in the order of the
   response. A denial, an unknown tool, invalid arguments, a failed job, a
   cancel: each becomes a `function_call_output`. Before each request, a
   normaliser adds `"aborted"` for any call without output and drops orphan
   outputs (Codex `normalize.rs`).
3. **Validate arguments before `agent-tools.call`.** Report problems by JSON
   pointer (rig), list at most 5, end with "fix the arguments and call again".
   Malformed JSON gets an excerpt. Calls from a response cut off by
   `max_output_tokens` never run.
4. **Truncate tool results for the model, keep them whole in the transcript.**
   Middle cut on char boundaries with a marker that says how much was cut
   (Codex). Default about 10 000 tokens.
5. **Classify each error once** (Codex): retry with backoff (5xx, overloaded,
   `rate_limit_exceeded`, a stream that ended without `response.completed`),
   retry after the server's time, or terminal (400, auth, `usage_limit_reached`,
   `usage_not_included`, `insufficient_quota`, `ResponseIncomplete`). Retry
   only the model request, never a tool. At most 4 retries per turn. A retry
   of a stream that had already sent deltas tells the owner to discard them.
6. **Usage limits are a typed item.** Parse `usage_limit_reached` from the 429
   body and from `response.failed`; keep `resets_at`, `plan_type` and the
   window; read the `x-codex-*` headers if they come. Map Claude's
   `rate_limit_event` with `status: rejected` to the same item. The session
   goes idle with the reset time; nothing retries it.
7. **Usage per finished response** in the transcript entry: input, cached,
   output, reasoning; an unknown field stays absent, not 0. Dedupe Claude
   usage by `message.id`.
8. **Loop limits from the history:** a step cap per input (with a soft stop:
   `tool_choice: none` for one last answer), Forge's repeat detector
   (threshold 3), and a limit on consecutive tool errors.
9. **`store: false` hygiene:** replay reasoning only with `encrypted_content`;
   strip `id` from replayed items; on "could not be verified", retry once
   without reasoning items.
10. **Claude results:** judge failure by `is_error`, not `subtype`; treat a
    missing `result` before exit as a crash; skip a line that does not parse;
    accept events after `result`; a missing `structured_output` is a failure.

### Avoid

- Sleeping in a call for backoff (Pi and Codex sleep; watane cannot).
- Retry layers that multiply (Cline's lesson).
- Replaying a partial answer as a "continue" prompt (golem-ai, Zed `Resume`).
  The transcript holds finished entries only; discard and resend.
- Replacing a real tool result after loop detection (gemini-cli#29815).
- Trusting `--max-turns` or `--max-budget-usd` alone.
- Depending on rig-agent or oi-kit-agent as crates: their formats and APIs
  still move every month.

### Proposal changes

- **5 (H3) and 7 (H5): a deferred request.** `https-stream.start` takes
  `not-before` (a delay in ms, capped, for example 15 minutes). The host holds
  the request and sends it then. The kernel never waits in a call, and the
  retry is still a request that applies only on commit.
- **3 and 5: an idle timeout for host streams.** D2 keeps no total limit, but
  an HTTPS stream with no event for N seconds (a host setting; Codex uses
  300 s, which proved too long, codex#33051) ends with `ended(idle-timeout)`.
- **6 (H4): `describe` returns `parallel: bool` per tool.** The kernel starts
  parallel jobs together and runs the others one at a time, in call order
  (Codex's read and write lock). Default false.
- **4.1 and 11: a `max-turns` field.** Claude: `--max-turns=`, and the kernel
  also counts and interrupts. OpenAI: the kernel's step cap. Add
  `max-tool-errors` the same way for OpenAI; Claude refuses it.
- **10 and 12 S5:** step 3 adds "every call gets one output, in call order;
  late job results after a cancel are dropped". S6 adds "the kernel writes
  `aborted` outputs for open calls". S7 adds "an OpenAI turn in flight at a
  restart is discarded and resent once; a durable counter stops a crash loop"
  (OpenCode).
- **11 `json-schema` row:** OpenAI: `text.format` with `strict: true`, plus the
  kernel's own check of the final answer and one re-prompt. Claude: a missing
  `structured_output` fails the session.
- **8 Previews:** `dev` serves a scripted Responses stream, as it runs a fake
  `claude` (W5).

## Who does it best

| Capability | Best | Runner-up | Note |
|---|---|---|---|
| Turn loop, state machine | rig (`AgentRun`) | Goose (`oi-kit-agent`) | both resumable from data |
| Stop conditions, max turns | OpenCode (soft stop) | Forge (doom loop), Gemini CLI | Pi, Codex: none |
| Dispatch and order | Codex (parallel flag, ordered results) | Pi | Cline: sequential default |
| Schemas and validation | rig (pointer problems) | Pi (coercion, repair) | Gemini skips bad schemas |
| Output truncation | Codex (char-safe middle cut) | OpenCode (file spill) | |
| Cancellation | Codex (aborted output and normaliser) | OpenCode, Goose | |
| Retries | Codex (per-error policy, server deadline) | OpenCode | Zed similar (GPL) |
| Usage limits | Codex (typed, headers) | Pi | Gemini for Google quotas |
| Token accounting | Codex `TokenUsage` | OpenCode | golem-ai too thin |
| Structured output | rig (check and re-prompt) | Goose (`final_output`) | Claude: check the key |
| Resumable split | rig | Goose, OpenCode | golem-ai replay: avoid |

## Test cases watane should write

Fake: a scripted `https-stream` that replays SSE events per request (Codex
`ev_*` builders and `mount_sse_sequence`; Cline `ScriptedModel`), and a fake
`agent-tools` that answers jobs in a chosen order and delay.

1. Two calls; the second job finishes first. The next request has both outputs
   in call order.
2. Cancel with two jobs open. The history has two `aborted` outputs; a late job
   result is dropped; the next `say` succeeds.
3. A history restored with a call and no output. The request gets a synthetic
   output, and no orphan output is sent.
4. A broker denial, an unknown tool, invalid arguments, malformed JSON: each
   gives one output; no job starts.
5. `response.incomplete` (`max_output_tokens`) with a call: no job starts.
6. A stream that ends without `response.completed`: retried; deltas already
   sent are marked discarded; after 4 failures the session fails.
7. A 429 `rate_limit_exceeded` with `retry-after: 2`: the next start carries
   `not-before` ≥ 2 s; no call sleeps.
8. A 429 `usage_limit_reached` body, and the same inside `response.failed`:
   one usage-limit item with `resets_at`; no retry.
9. A 400 is not retried.
10. Kernel restart between a completed response and its job results: the run
    record resumes and the turn finishes once, without re-running a tool.
11. Kernel restart mid-stream: the attempt is resent once; a second crash at
    the same point ends the session (counter).
12. The step cap: at the cap the request has `tool_choice: none`; then the
    session ends with a max-turns outcome.
13. Three identical calls in a row: the loop item is raised.
14. A 1 MiB tool result with multi-byte text: the model sees the head, the
    marker and the tail, cut on char boundaries; the transcript has it whole.
15. Replayed history: no reasoning item without `encrypted_content`, no `id`
    fields.
16. Usage summed over three responses with one missing `cached_tokens`: the
    sum marks it unknown.
17. Claude lines: `is_error: true` with `subtype: success`; no `result` before
    exit; a `rate_limit_event` after `result`; one line that does not parse;
    a `result` without `structured_output` under `json-schema`.
18. A parallel tool and an exclusive tool in one response: the exclusive job
    starts only after the parallel one ends.

## Reusable code

| What | Path @ commit | License | wasm32-wasip2 | How |
|---|---|---|---|---|
| Char-safe middle truncation, token estimate | codex `codex-rs/utils/string/src/truncate.rs` @bb5b9c6 | Apache-2.0 | yes: std, serde, regex-lite | copy with header; not on crates.io |
| Error classes and retry delay | codex `codex-rs/protocol/src/error.rs:481-540`, `codex-client/src/retry.rs:43` | Apache-2.0 | logic yes; the crates pull tokio | port |
| SSE error parsing, "try again in" regex | codex `codex-rs/codex-api/src/sse/responses_error.rs` | Apache-2.0 | logic yes | port |
| Usage-limit body and `x-codex-*` headers | codex `codex-api/src/api_bridge.rs:215-270`, `rate_limits.rs` | Apache-2.0 | logic yes (`http::HeaderMap` builds) | port |
| `TokenUsage` and its mapping | codex `protocol/src/protocol.rs:2604`, `codex-api/src/sse/responses.rs:123-149` | Apache-2.0 | yes | port |
| History normaliser | codex `core/src/context_manager/normalize.rs:21-243` | Apache-2.0 | logic yes | port onto our items |
| Codex SSE test builders | codex `core/tests/common/responses.rs:799-1206` | Apache-2.0 | test side, native | port |
| Sans-I/O run machine | rig `crates/rig-agent/src/run/` @0019e24 | MIT | likely: rig-core now has no tokio or reqwest by default; not built (the earlier probe failed with defaults on) | copy the design; maybe code |
| Schema validator with pointers; feedback texts | rig `crates/rig-core/src/tool/validate.rs`, `tool/feedback.rs` | MIT | likely, not built | copy |
| Step pipeline, max turns, pending calls, final output | goose `crates/oi-kit-agent/src/{machine,max_turns,tool_call_state,final_output,empty_response}.rs` @80ce943 | Apache-2.0 | partly: has wasm32 cfgs, but uses tokio `select!` and rmcp types | port |
| Retry config and permanent-4xx markers; encrypted-reasoning check | goose `crates/oi-kit-provider-types/src/retry.rs:14-120`, `oi-kit-providers/src/formats/openai_responses.rs:91-102` | Apache-2.0 | logic yes | port |
| JSON repair and schema coercion | forge `crates/forge_json_repair` @a22b492 | Apache-2.0 | likely: serde_json, regex, schemars, json5 | depend or copy |
| Doom-loop detector | forge `crates/forge_app/src/hooks/doom_loop.rs` | Apache-2.0 | logic yes | port |
| `StopReason` enum | swiftide `swiftide-agents/src/state.rs` @92e1a30 | MIT | yes (types) | copy the shape |
| JSON Schema checks | crate `jsonschema` 0.61, `default-features = false` (as goose uses it) | MIT | likely, not built | depend |
| Retry classifier lists, retry-after parsing | pi `ai/src/utils/retry.ts`, `provider-retry.ts` @e5d93ea | MIT | TypeScript | port the lists |
| Tool output limits, cooperative stop | opencode `core/src/tool-output.ts`, `runner/step.ts` @b7f0b90 | MIT | TypeScript | port the rules |
| Loop detection | gemini-cli `core/src/services/loopDetectionService.ts` @d4cb171 | Apache-2.0 | TypeScript | port the rules |
| Turn loop with retries | zed `crates/agent/src/thread.rs` @10d1b2a | GPL-3.0 | – | ideas only |

"Likely" means the dependencies look portable, but nobody built it for
`wasm32-wasip2`. The probe in `research_notes/calling-agents.md` failed for
rig-core 0.46 with default features; `default-features = false` is untested.
