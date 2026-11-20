# Restate: what watane's agentic layer can take from it (2026-11-20)

A record, not a contract. It compares Restate (a durable execution engine) and
its agent work with `ARCHITECTURE-PROPOSAL.md` (revision 3) and with the
seven earlier comparisons (`COMPARISON-harness-summary-2026-11-20.md`).
Nothing was built or run. Every citation names repo@commit.

## Sources and licences

| Repo (github.com/restatedev) | Commit | Licence | What it is |
|---|---|---|---|
| `restate` | fa4262f | **BSL 1.1** (Apache-2.0 four years after each release); `service-protocol/dev/restate/service/*.proto` say MIT in their headers | the server, Restate Lite (`lite/`, the server as a library) |
| `sdk-shared-core` | 75dc089 | MIT | the sans-IO journal state machine every SDK uses (crate `restate-sdk-shared-core` 8.7.2) |
| `sdk-rust` | 968ecd0 | MIT | Rust SDK 0.13.0, test services, testcontainers |
| `sdk-typescript` | b484a78 | MIT | TS SDK; shared core as wasm-bindgen; Cloudflare Workers build |
| `agent` | 4080257 | **none** (no LICENSE file, no `license` field) | the reference agent architecture (TS): controller, turn runtime, approvals, steering, schedules, sub-agents |
| `agentlib` | 2bd4d81 | MIT | agent library on Restate (TS), its vocabulary (`CONTEXT.md`) and tests |
| `ai-examples` | daf0283 | **none** | agent examples for Vercel AI, OpenAI Agents, ADK, ... |
| `vercel-ai-middleware` | 62eeb7b | MIT | `durableCalls` middleware for the Vercel AI SDK |
| `pubsub` | 065aabb | MIT | durable pub/sub object with offsets and long-poll |
| `sdk-test-suite`, `e2e` | 5692c95, 4ce3c05 | MIT | conformance and end-to-end tests (Kotlin) |
| `e2e-verification-runner` | ef95254 | no LICENSE file; file headers say MIT | random-program chaos verification |
| `jepsen` | 828262a | MIT; `src/jepsen_patched/` EPL-1.0 | Jepsen workloads |

**watane's licence.** The repo has no `LICENSE*` at its root. `net` and `log`
declare `MIT OR Apache-2.0` in their `Cargo.toml`. MIT code from Restate fits
that (keep its copyright notice). BSL code does not: the server is read-only
for us. `agent` and `ai-examples` have no licence, so all rights are reserved:
read them for design, copy nothing.

Docs read: docs.restate.dev (versioning, invocation management, HTTP
invocation, human-in-the-loop, streaming responses, services), the Restate 1.2
announcement (benchmarks). Issues: restatedev/restate #3216, #3603, #4344,
#5099, #5152, #5375.

## Restate in one page

- **Handlers and a journal.** A handler is ordinary async code. Every
  context operation is a **command** in the invocation's journal; results
  arrive as **notifications** (completions, signals). Commands: input,
  output, get/set/clear state, sleep, call, one-way call (with
  `invoke_time`), send signal, run, get/peek/complete promise, attach,
  get invocation output, cancel (`protocol.proto`, restate@fa4262f).
- **Replay.** After a crash, a retry or a suspension, the server streams the
  journal back and the SDK re-executes the code; recorded commands return
  their recorded results. A command that differs from the record is a
  journal mismatch (code 570) (`sdk-shared-core` `src/lib.rs`
  `NonDeterministicChecksOption`, `src/tests/failures.rs`).
- **Side effects.** `ctx.run(closure)` runs the closure once and proposes its
  result; the server stores it, then acks (`ProposeRunCompletionMessage`).
  A closure that finished before its result was stored runs again
  ("an external effect that completed before its result was recorded may
  run again", `agent` `docs/architecture.md`).
- **Determinism.** No clock, random or unordered iteration outside the
  journal; `ctx.rand()` is seeded from `StartMessage.random_seed`; await
  `ctx.run` at once (`sdk-rust` `src/context/mod.rs` doc of `run`).
- **Suspension.** An invocation that awaits something not yet completed
  sends `SuspensionMessage` with the awaited future tree and ends. In
  request/response mode (Lambda, Workers) it suspends at every such await,
  and every resume replays the whole journal.
- **Keys.** A virtual object runs at most one exclusive handler per key;
  shared handlers are read-only and run alongside. A workflow's `run` runs
  once per key; its shared handlers resolve durable promises.
- **Limits and defaults** (restate@fa4262f `crates/types/src/config/`):
  message (journal entry) limit 32 MiB (`networking.rs:23`); inactivity
  timeout 60 s, then a suspend request, abort after 600 s (`worker.rs`);
  invocation retry 500 ms × 2, max interval 60 s, 70 attempts, then
  **pause** (`invocation.rs`); journal retention 1 day; idempotency results
  kept 24 h; large entries can go to a blob table above 4 KiB (experimental,
  `release-notes/unreleased/journal-entry-blob-storage.md`).

**The mapping in one line each.** A watane H5 request is Restate's journaled
call or send: the runtime performs it after the record is durable (exactly
one dispatch), not `ctx.run` (at least once). A watane stream item is a
notification. Kernel state per session is virtual-object state. The kernel's
OpenAI step is the `agent` repo's `agentStep`. The big difference: Restate's
notifications are durable; watane's streams live in memory (proposal 3,
Restart).

## 1. The loop as durable steps (10 s calls)

**watane plans.** A sans-IO loop in the kernel; each step is a short call
driven by stream items; state saved between calls (proposal 10 "OpenAI
loop", 12 S5; summary lesson 3).

**What Restate does.** The loop is straight-line code; the journal is the
state. The agent reference splits it in two keyed objects with the same key
(`agent` `docs/architecture.md`):

| Object | Owns | Lock held |
|---|---|---|
| `Agent` (controller) | active turn id, FIFO input queue, profile, approvals, schedules, notification versions, children | short: never waits for a turn |
| `AgentSession` | append-only transcript, `doTurn` (one turn = one invocation, its id is the `turnId`) | the whole turn |

One `agentStep` is "model call, guardrail gate, optional approval wait, and
allowed foreground tool batch. It owns no task after returning"
(`docs/turn-runtime.md`). Rules worth keeping:
- At most 50 steps per turn; at the bound, stop pending work and make one
  tool-free final call; outcome `stopped` with `step_limit`, not a failure.
- Outcomes are exactly `completed`, `interrupted`, `stopped`, `failed`, each
  with `consumedSteering`.
- On `finishReason: length` the whole generation is rejected before any tool
  call runs; one recovery call with a doubled budget, derived from the
  journaled result, not the current env (`src/model/inference.ts`).
- Invalid or empty output becomes corrective feedback; three unusable
  responses in a row fail the turn.
- The profile is snapshotted at turn start; edits apply to the next turn.
- Tool-call and tool-result messages are committed together to the model
  context; a step's tool failure is an observation, not a sibling's loss.

`agentlib` (MIT) names three failure classes (`CONTEXT.md`): a
**model-handled failure** (a `TerminalError` in a tool goes to the model as
the result, never retried); an **agent-terminal failure** (the run and every
enclosing run fail); a **retryable failure** (Restate retries, then pauses
the invocation).

The Vercel middleware journals one entry per model call:
`wrapGenerate: ({doGenerate}) => ctx.run(..., doGenerate)`; it has no
`wrapStream` (`vercel-ai-middleware` `src/lib/ai_infra.ts:60`).

**Tested.** `agent` `test/harness.mjs` drives the real handler over hand-built
protocol frames, replaying a recorded journal without a server.
`agentlib` `packages/tests/src/lifecycle.test.ts` (MIT): "steering during
inference discards a stale final answer", "interruption preserves completed
results, closes unfinished calls, and finalizes without tools", "a spent
tool-call budget stops the run with a summary instead of failing it".
`sdk-test-suite` runs every SDK in an `alwaysSuspending` mode
(`sdk-rust` `test-services/exclusions.yaml`): the server suspends at every
await, so each step resumes from the journal alone.

**Worth copying.** The outcome set and step bound; the three failure
classes; the truncation rule; the controller/turn split as a rule (the
exclusive section never spans a turn). Read `agent` (no licence); port
`agentlib` test ideas (MIT).

**Pitfalls.** Replay cost grows with the journal: in request/response mode
every resume re-runs every recorded command. A retry counter is "not durably
stored ... might get reset" (`protocol.proto`, `StartMessage`). Code changes
break in-flight journals (section 9).

**Do not embed the journal VM.** `sdk-shared-core` is pure Rust with default
features (prost, bytes, serde, thiserror, tracing) and the TS SDK ships it as
wasm32-unknown-unknown. A kernel could, in principle, write the loop as async
code and replay a journal kept in redb on each call. That would make every
call O(journal), add determinism rules, and break in-flight turns on each
kernel update, because watane keeps one installed version and cannot pin old
code. The `agent` repo itself says: "Deploy with affected in-flight turns
drained/interrupted". Keep the explicit state machine (summary lesson 3).

## 2. Approvals and long waits

**watane plans.** An ask is an item with an ask id; the kernel's broker or the
owner answers; no answer in 10 minutes is a deny (proposal 3 Ends, 12 S4).

**What Restate does.**
- **Awakeable:** an id the code hands out, resolved from outside
  (`/restate/awakeables/{id}/resolve`); `promise.orTimeout({hours: 3})`; a
  timeout takes a fallback path, here "not approved" (`ai-examples`
  `vercel-ai/tour-of-agents/src/human-approval-agent.ts`; docs
  human-in-the-loop). A waiting invocation suspends and costs nothing.
- **Named signals** on an invocation id (newer):
  `ctx.signal::<String>("approval").await` and
  `ctx.invocation_handle(id).signal("approval").resolve(..)`
  (`sdk-rust` `examples/signals.rs`). Repeated resolutions of one name form a
  FIFO: "the k-th await of a signal name must observe the k-th delivery"
  (`e2e` `SignalsFuzz.kt`).
- **Durable promise** (workflows): named, first completion wins, can be
  resolved before anyone awaits it, `peek` reads without waiting.
- **The agent's approval registry** (`agent` `src/agent/approvals.ts`):

```ts
*requestApproval(request) {   // idempotent for an identical request
  if (!(yield* activeTurn.accepting(request.turnId))) return false;
  ...existing.turnId === request.turnId && existing.question === request.question
*resolveApproval({approvalId, decision}) {  // only while that turn is active
  restate.invocation(request.turnId).signal(approvalSignalName(approvalId)).resolve(...)
```

  The registry (controller) and the waiter (turn) are separate. An
  interrupted wait withdraws its request (`src/session/approvals.ts`). A
  turn's end clears its approvals. Steering clears earlier guardrail
  approvals; an approval covers one action, never "all later calls".

**Tested.** `agent` `test/approvals.test.mjs`: "an approval is registered once
and resolved into a signal to its turn", "a conflicting request reusing an
approval ID is rejected", "only the active, non-interrupting turn can register
or receive approvals", "resolving an unknown approval, or cancelling another
turn's, changes nothing". `sdk-shared-core` `src/tests/signals.rs`,
`src/tests/promise.rs`.

**Worth copying.** Bind an ask id to its turn; refuse answers for a stale
turn; same id and same content is idempotent, a different content conflicts;
withdraw on interrupt; answers race-free (first wins). Read (no licence).

**Watane difference.** A Claude ask holds a live process, so a short expiry
fits. A kernel (OpenAI) ask holds nothing: the turn is just state between
HTTPS streams, like a suspended invocation. It could wait for days.

## 3. Sessions, transcripts, replay, attach

**watane plans.** A transcript indexed per finished entry; `attach(after)`
replays then goes live; `read(after, max)` for polling; resume after restart
(proposal 10, 12 S1/S7; summary lesson 5).

**What Restate does.**
- **Chat session as a virtual object.** The simple version keeps the whole
  message list in one key and rewrites it each turn (`ai-examples`
  `vercel-ai/tour-of-agents/src/chat-agent.ts`, exclusive `message`, shared
  `getHistory`). The reference agent does better: chunks of 32 entries, one
  key per chunk, a meta key with `nextSequence`, a summary checkpoint
  (`agent` `src/session/history.ts`: `CHUNK_SIZE = 32`,
  `COMPACT_AFTER_MESSAGES = 32`, `KEEP_RECENT_MESSAGES = 8`). A turn loads
  history once and then only appends to the tail chunk.
- **History API.** `history({fromSequence, limit})` returns entries and
  `nextSequence`, from 1. "Notifications invalidate data; they do not carry
  it." "Capture a notification watermark **before** reading authoritative
  state, drain history pages, and then watch" (`agent` `docs/protocol.md`).
- **Public transcript vs working context.** "Raw reasoning, tool arguments,
  and tool results do not enter the canonical transcript"; they live in the
  turn's journal and model context (`agent` `docs/agent-guide.md`,
  invariant 12). Compaction writes a checkpoint, never rewrites the log.
- **Invocation ids and attach.** Every call gets an `inv_` id at once
  (`CallInvocationIdCompletionNotificationMessage`). `GET
  /restate/attach/{id}` waits for the result; `/restate/output/{id}` answers
  "not ready" while it runs; both also take an idempotency-key target.
- **Idempotency.** `Idempotency-Key` header: the same key in 24 h returns the
  stored response without re-running.
- **Restart from prefix** (UI): a new invocation replays part of an old
  journal and continues: Restate's "resume at".

**Tested.** `agent` `test/history.test.mjs`: "history pages across chunk
boundaries and reopened turns append after the tail", "the verbatim tail never
starts at an assistant reply", "a failed summary clears the reservation".
`e2e` `JournalRetentionTest.kt`, `KillRuntime.kt`
(`startAndKillRuntimeRetainsTheState`).

**Pitfalls.**
- **#5375** (closed 2026-09): a notification appended while an attempt
  starts was sent twice, once in the replay and once live: "Nothing
  correlates the two." This is exactly watane's attach race.
- **#4344**: all state is sent eagerly at start; large state needs streaming.
  One growing key per chat is the trap.
- **#5152**: ingress mixed transport errors and the invocation's own error
  codes, so clients could not tell retry, reattach or stop apart.

## 4. Delegation and agent-to-agent calls

**watane plans.** Profiles name what they may start (`delegates`), narrow
only; a parent's cancel cancels its children; the OpenAI child has no `task`
function; jobs return an id at once (proposal 10; summary "10 Kernel").

**What Restate does.**
- **Sub-agents as separate invocations** (`agent` `docs/protocol.md`,
  `src/agent/sub-agents.ts`). The parent stores its child list; the child
  stores its parent. A child copies the parent's instructions, guardrails and
  tools at creation, then narrows; it does not follow later changes; it
  starts with empty memory. Children cannot create children or schedules. A
  child conversation refuses direct `ask`, `steer` and profile edits. The
  parent's session waits on "the exact child turn".
- **Retry-safe child identity.** "Agent derives its parent and stable child
  identity from turn/tool call"; "child initialization is atomic,
  retry-safe"; "deletion tombstones the ID, and late creation cannot
  resurrect it" (`agent` `test/sub-agent.test.mjs`).
- **agentlib** keeps three words apart (`CONTEXT.md`): **handoff** (change
  the active agent inside a run, keep history and budgets), **sub-agent** (a
  fresh run inside the same invocation; its answer is the tool result),
  **fork** (a copy of the run from the calling step; saves nothing; cannot
  fork again).
- **Cancellation propagates by recorded ids.** On the cancel signal the SDK
  cancels every child call whose id it knows. If the id has not arrived yet,
  it suspends until it has (`sdk-shared-core`
  `src/tests/implicit_cancellation.rs`: `call_then_cancel_without_invocation_id`).
  Calls made after the cancel was caught (compensation) are not cancelled
  (`saga_compensation_call_is_not_cancelled`). One-way and delayed sends are
  detached and survive the caller's cancel (docs, managing invocations).
- **Scope and limit keys** (protocol V7, `CallCommandMessage.scope`,
  `limit_key`): concurrency limits per key, the shape of "4 Claude
  processes per origin client".

**Worth copying.** Child key derived from (parent session, call id); creation
idempotent; tombstones; children read-only to clients; depth 1 by default;
cancel needs the child id first. Read `agent`; port the `sdk-shared-core`
cancel tests' cases (MIT).

## 5. Streams to clients

**watane plans.** One-consumer streams with batching and backpressure;
deltas only live, never stored (proposal 3).

**What Restate does.** It does not stream tokens through the journal.
"Token-by-token streaming is not yet supported but is on the roadmap" (docs,
streaming responses). The options are:
- **pubsub** (MIT, `pubsub` `packages/pubsub/src/lib/pubsub.ts`): each
  `publish` writes one state key `m_<offset>`; `pull({offset})` returns the
  messages after `offset`, or registers an awakeable and waits with
  `orTimeout(30 s)`; a client turns this into SSE. Each publish needs an
  idempotency key, or "a replay or retry produces duplicate events".
- **The agent's notifications**: `watch({afterRevision, timeoutSeconds})`
  long-polls per-topic version counters; clients then re-read only the
  changed topics. Keep the same idempotency key when retrying one poll
  window (`agent` `docs/protocol.md`).
- **agentlib progress events**: the run waits for its progress handler, and
  "replay delivers the same events again, so external effects belong in a
  journaled step" (`CONTEXT.md`).

**Pitfall.** #5099 (open): users ask how to stream AG-UI events from a durable
agent; there is no built-in answer.

**Lesson.** watane's live stream already does what Restate lacks: deltas that
cost nothing durable. Keep it. Take two things: a sequence per item so a
consumer can drop duplicates, and invalidation-only notifications for agents
that poll (proposal 13).

## 6. Retries without sleeping in a call; timers

**watane plans.** `https-stream.start` takes a `not-before` delay (summary
"5 HTTPS"); asks expire after 10 minutes; no general timer.

**What Restate does.**
- **The runtime sleeps, not the handler.** A failed attempt returns
  `ErrorMessage{next_retry_delay, behavior: RETRY | PAUSE | FAIL}`; the
  server waits and re-invokes (`protocol.proto`). Per-`run` policy:

```rust
pub enum RetryPolicy { Infinite, None,
  FixedDelay { interval, max_attempts, max_duration, on_max_attempts },
  Exponential { initial_interval, factor, max_interval, max_attempts, max_duration, on_max_attempts } }
pub enum OnMaxAttempts { FailAsTerminal, Pause }   // sdk-shared-core src/retries.rs
```

  `next_retry` computes `initial * factor^(n-1)`, capped, with an overflow
  guard and a test to 200 attempts (`exponential_policy_does_not_panic_on_overflow`).
- **Pause, not fail.** By default an invocation that runs out of attempts is
  paused, so it can resume after the cause is fixed (server default 70
  attempts; `agentlib` "Retryable failure").
- **Durable timers.** `ctx.sleep`, `OneWayCall.invoke_time` (delayed send),
  `orTimeout`. The agent's schedules are delayed `Agent.fire` calls; each
  schedule stores the id of its next timer, so "replaced or cancelled timer
  invocations cannot deliver, because their invocation ID no longer
  matches". Recurring deliveries coalesce: a firing is dropped while the
  previous one from the same schedule is queued or running
  (`agent` `docs/schedules.md`).
- **Model retries.** `{maxAttempts: 4, initialInterval: 500, maxInterval:
  5000, factor: 2}`; provider SDK retries are disabled so only one layer
  retries (`agent` `src/model/inference.ts`).

**Tested.** `sdk-shared-core` `src/retries.rs` tests; `e2e` `RunRetry.kt`,
`SleepWithFailures.kt` (`sleepAndKillServiceEndpoint`); `e2e`
`PauseResumeTest.kt`.

**Worth copying.** `RetryPolicy` and `next_retry` (about 100 lines, MIT): port.
Stale-timer check by id; coalescing; pause on exhaustion; one retry layer.

**Pitfalls.** #3216: infinite retries of a non-retryable error. The retry
count in `StartMessage` is not durable.

## 7. Per-step cost

**watane.** A writing call costs about 4 ms (one `F_FULLFSYNC`); one writer for
the host, about 240 writing calls per second (proposal 3, V6).

**Restate.** Restate 1.2 on 3 nodes, 3-way replicated, low load: 3 steps
p50 15 ms, 9 steps p50 31 ms (about 3.4 ms a step); high load 17 000 req/s,
"84 000 actions per second" at p90 76 ms (restate.dev/blog/announcing-restate-1.2;
hardware not stated). Throughput comes from batching many invocations into
one log write. Each step is one journal entry; the agent journals a model
call as one entry and tool events as short records without raw arguments.

**Lesson.** Per step, watane and Restate cost about the same. Restate's
journal is as coarse as watane's transcript plan: one entry per model call or
tool result, never a token. The difference is throughput under load:
batching commits. A host-level group commit (several calls, one fsync) is
the lever if 240 writes per second ever binds; not a proposal change now.

## 8. Cancellation, interruption, kill

**What Restate does.** Three levels:
- **Interrupt** (agent): stop the turn, join unfinished work, keep completed
  results, mark unfinished calls honestly, one tool-free final call; an
  optional replacement message becomes the next turn's input
  (`docs/turn-runtime.md`).
- **Cancel** (runtime): a cancel signal (`CANCEL_SIGNAL_ID`) reaches the
  leaves first; each handler gets a terminal error at its next await, runs
  compensation, and passes it up. "Cancellation is non-blocking" and may need
  a retry (docs).
- **Kill**: stop the tree at once, no compensation, "Only use as a last
  resort".

Steering is not cancellation: "Steering never cancels a step or existing
pending operation"; it is read after the current tool batch settles.

## 9. Versioning while turns are in flight

Deployments are immutable; "Restate then makes sure that requests start and
end on the same version". A fix to an in-flight invocation needs pause,
register, resume `--deployment`, and still must match the recorded journal.
Restate advises against handlers that run for days. Object state persists
across versions and must stay backward compatible (docs, versioning).

watane keeps one installed version of a mod and ends its streams on update
(proposal 3 Ends). So it cannot pin. The kernel's own state must be readable
by the next version, and a turn in flight must end cleanly.

## 10. WASM, edge and the Rust SDK

- `restate-sdk` (Rust) needs a Restate server; it builds for
  wasm32-unknown-unknown (Cloudflare Workers; `Cargo.toml` uses `web-time`
  and `getrandom` backends) but not as a WASI component with watane's world.
  Not usable in a mod.
- The TS SDK runs on Workers, Deno, Bun, Lambda in request/response mode:
  suspend at every await, replay on resume. That is the closest analogue of
  watane's fresh instance per call, and its cost is the full replay.
- **Restate Lite** (`restate` `lite/src/lib.rs`, BSL) embeds the server in a
  process (tokio, RocksDB). Embedding it in watane would bring BSL, a second
  storage engine beside redb, an HTTP/2 service protocol between host and
  mods, and journal replay. Avoid.

## Lessons for watane

### Adopt

1. **Bounded turns with four outcomes** (`completed`, `interrupted`,
   `stopped`, `failed`), each recording how many steered messages it consumed.
   `max-turns` stops with a tool-free summary, not a failure. (10, 12)
2. **Busy input is explicit.** `say` while a turn runs queues FIFO and
   returns `queued`; separate `steer` and `interrupt(reason, replacement?)`;
   at most 32 queued and 32 steered messages, then `busy`. (10)
3. **Idempotent starts and children.** A client-chosen session key, unique
   per owner; a child's key derived from (parent session, call id);
   tombstones for deleted keys. (10)
4. **Asks bound to turns.** Same id and content is idempotent; a different
   content conflicts; answers to an ended turn are refused; first answer
   wins; interrupt withdraws. (9, 12 S4)
5. **Retries by the host, not in a call**, with Restate's policy shape and
   pause on exhaustion. (5, 7)
6. **Transcript in chunks** of N entries per key plus a meta key; the public
   transcript holds concise entries; tool I/O lives in the private wire
   history. (10)
7. **Three error classes** for tools and steps: model-handled,
   agent-terminal, retryable. (10, summary lesson 6)
8. **Tests in always-suspend form** (below): every step a separate call,
   kernel rebuilt from state alone.

### Avoid

- Journal replay of straight-line code inside a mod (section 1 above).
- One growing state value per chat (`ai-examples` chat-agent; #4344).
  watane's 4 MiB `value_bytes` makes it a hard failure.
- Streaming tokens through anything durable (pubsub writes a key per event).
- Restate's non-durable retry counter: keep attempt counts in state.
- Embedding Restate or Restate Lite (BSL, second engine).

### Proposal changes, by section

- **3 Streams.** Each item carries a per-stream sequence number. State the
  delivery rule when a `stream-sink.items` call fails: retry the same batch
  (at least once) and let the consumer drop seen sequences, or drop it; today
  the section is silent. Attach: read the transcript and register for live
  items in one call, then send `synced {through}`; a live item at or below
  `through` is dropped (#5375 hit exactly this).
- **5 HTTPS / 7 Requests.** Replace the HTTPS-only `not-before` with a host
  timer request: `timers.after(delay, tag) -> timer-id` and
  `timers.cancel(id)`, delivered as a call to the mod's own export, kept
  across restart. One mechanism for retry backoff, ask expiry, profile
  schedules and idle timeouts. A mod ignores a firing whose id it no longer
  holds.
- **6 Agent tools.** `describe` returns `idempotent: bool` (default false),
  beside `parallel`. `call` passes the model's call id as an idempotency key.
  After a restart, a call with no result is re-run only if idempotent;
  otherwise it gets an `aborted` result (summary lesson 2).
- **9 Providers / 12 S4 Asks.** Expiry per backend: a Claude ask keeps
  10 minutes (it holds a process); a kernel ask on the OpenAI loop holds
  nothing and takes the profile's `ask-timeout` (default 10 minutes, longer
  allowed), kept in kernel state across restart. Approvals cover one action.
- **10 Kernel.** `start(key, profile, overlay, input)`: idempotent by owner
  and key. This closes a gap: the kernel's writes commit when its call
  succeeds even if the client's call fails afterwards (host 6, "The writes of
  P commit when P's call succeeds, even if C fails later"), while the host
  effect (H5) is dropped; today that leaves a session the client never
  learns and whose stream never started. Add `steer` and `interrupt`; `say`
  returns `started | queued`. A turn snapshots its profile. Transcript stored
  in chunks; the outcome entry of each turn holds `consumed-steering`.
- **12 S6.** Name three levels: `interrupt` (end the turn, synthesise
  results, keep the session), `cancel` (end the session, answer asks with
  deny, close the handle, cancel children), kill (process group, last
  resort, what the host does when a mod is removed).
- **12 S7.** On a kernel update or restart, a turn in flight ends as
  `interrupted` with reason `updated` or `restarted`; missing tool results
  follow the 6 rule. Entries stay readable by the next version (summary
  "entries versioned per kind").
- **13 Agents as clients.** `agent:sessions.wait(after-revision, timeout)`
  as a host-held MCP request: it returns changed topic versions only
  (history, asks), never data; the agent then calls `read`. A mod call
  cannot block, so the host must hold the wait.

### What challenges a watane choice

- **One call per mod.** Restate locks per key and runs read-only handlers
  beside the writer. watane's kernel is one mod, so all sessions of all
  clients share one lane, and `read`/`attach` wait behind writes. Fine at
  today's sizes because no kernel call waits; measure before changing. A
  later host option: read-only exports on a redb read snapshot, without the
  mod lock.
- **Memory-only streams.** Restate's completions survive a crash; watane's
  stream items do not. A host effect whose result was not delivered before a
  restart is lost (at most once). S7 accepts that; the proposal should say
  so in one line.
- **H5 is stronger than `ctx.run`.** Restate runs a side effect, then
  records it, so it may repeat. watane records the request, then the host
  performs it after commit. Keep H5; do not add a `run`-like host call.

## Test cases we should write

| # | Case | Source of the idea |
|---|---|---|
| 1 | Always-suspend: run an OpenAI turn where every step is a separate call and the kernel keeps nothing in memory; the transcript equals the in-memory run | `sdk-test-suite` `alwaysSuspending` |
| 2 | The same item batch delivered twice changes nothing and starts no second job | #5375; `agentlib` "replay delivers the same events and journaled side effects run once" |
| 3 | An entry appended during an attach replay reaches the client once | #5375 |
| 4 | Steering fuzz: random `say`/`steer` while busy and idle; the k-th steer is consumed k-th; `consumed-steering` matches | `e2e` `SignalsFuzz.kt`; `agent` `turns.ts` |
| 5 | The 33rd queued message gets `busy` | `agent` `docs/protocol.md` |
| 6 | Interrupt keeps finished tool results, closes unfinished calls with results, makes one tool-free call | `agentlib` `lifecycle.test.ts:227` |
| 7 | Interrupt during inference prevents its proposed tools | `lifecycle.test.ts:187` |
| 8 | A steer during inference discards the stale final answer | `lifecycle.test.ts:116` |
| 9 | `max-turns` ends `stopped` with a summary, not `failed` | `lifecycle.test.ts:708` |
| 10 | `finish_reason: length` runs no tool call and retries once with double budget | `agent` `inference.ts` |
| 11 | Three unusable responses in a row fail the turn | `agent` `docs/turn-runtime.md` |
| 12 | Ask: identical re-register is accepted, conflicting is refused, an answer for an ended turn is refused, a second answer gets `already-resolved` | `agent` `approvals.test.mjs` |
| 13 | Interrupt withdraws pending asks; clients see them resolved | `agent` `session/approvals.ts` |
| 14 | Kernel ask expiry fires after a restart (timer kept) and denies | Restate durable timers |
| 15 | Retry backoff follows the policy, survives restart, never sleeps in a call; exhaustion pauses the session | `sdk-shared-core` `retries.rs` |
| 16 | Backoff computation does not overflow at 200 attempts | `exponential_policy_does_not_panic_on_overflow` |
| 17 | A stale timer id fires and is ignored | `agent` `docs/schedules.md` |
| 18 | `start` with the same key twice gives one session; a client that fails after the kernel commits leaves no orphan | host 6 commit rule; #5152 |
| 19 | A child key from (parent, call id): a retried `task` call yields one child; a deleted key cannot be re-created | `agent` `sub-agent.test.mjs:328,445` |
| 20 | Cancelling a parent whose child start is still a pending request cancels the child once it exists | `implicit_cancellation.rs:155` |
| 21 | Cleanup after cancel (deny asks, close handle) is not itself cancelled | `saga_compensation_call_is_not_cancelled` |
| 22 | After restart, a non-idempotent tool call without result becomes `aborted`; an idempotent one runs again with the same key | Restate `ctx.run` semantics |
| 23 | Kernel update mid-turn: the turn ends `interrupted(updated)`; old entries read under the new version | docs, versioning |
| 24 | 10 000 entries: each append writes at most the tail chunk and meta; no value exceeds 4 MiB; attach from the middle of a chunk | `agent` `history.test.mjs:23` |
| 25 | Kill the host while entries commit: every acknowledged entry is present once after restart | `jepsen` set workload; `e2e` `KillRuntime.kt` |
| 26 | Random programs (say, tool, ask, timer, cancel) on many sessions under kill and restart; counters converge; a stuck-run detector reports paused sessions | `e2e-verification-runner` `src/interpreter/commands.ts`, README |
| 27 | A tool's terminal error reaches the model; an agent-terminal error fails the run and its parent; a retryable one is retried | `agentlib` `CONTEXT.md`, `errors.test.ts` |
| 28 | A recurring delivery while the previous one is queued is dropped (coalesce) | `agent` `docs/schedules.md` |

## Reusable code

| What | Path (repo@commit) | Licence | How |
|---|---|---|---|
| Retry policy and backoff (`RetryPolicy`, `OnMaxAttempts`, `next_retry`, overflow test) | `sdk-shared-core@75dc089` `src/retries.rs` | MIT | port (~100 lines, pure Rust) |
| Cancel-propagation test cases | `sdk-shared-core@75dc089` `src/tests/implicit_cancellation.rs` | MIT | port the cases |
| `VMTestCase` style (feed frames, assert emitted frames) | `sdk-shared-core@75dc089` `src/tests/mod.rs` | MIT | read; same shape for kernel tests (items in, requests out) |
| Future combinator tree (all, any, race, all-settled) for "await these jobs" | `restate@fa4262f` `service-protocol/dev/restate/service/protocol.proto` `Future` | MIT (header) | read |
| Agent vocabulary: run, step, handoff, sub-agent, fork, steering, interruption, failure classes | `agentlib@2bd4d81` `CONTEXT.md` | MIT | read; align kernel terms |
| Lifecycle, progress, replay tests | `agentlib@2bd4d81` `packages/tests/src/lifecycle.test.ts`, `progress.test.ts`, `replay.test.ts` | MIT | port test names and cases |
| Offset long-poll pub/sub | `pubsub@065aabb` `packages/pubsub/src/lib/pubsub.ts` | MIT | read |
| One journal entry per model call | `vercel-ai-middleware@62eeb7b` `src/lib/ai_infra.ts` | MIT | read |
| Controller/turn split, steering, approvals, schedules, history chunks, sub-agents, refactoring invariants | `agent@4080257` `docs/*.md`, `packages/libs/core/src/{agent,session}/` | none (all rights reserved) | read only |
| Agent examples per framework | `ai-examples@daf0283` | none | read only |
| SDK conformance and fuzz tests (`SignalsFuzz`, `KillRuntime`, `SleepWithFailures`, `NonDeterminismErrors`) | `e2e@4ce3c05` `sdk-tests/`, `sdk-test-suite@5692c95` | MIT | read; port ideas |
| Random-program chaos runner, stuck-run detector | `e2e-verification-runner@ef95254` | headers MIT, no LICENSE file | read |
| Set and register workloads under kill | `jepsen@828262a` `src/restate/jepsen/` | MIT (`jepsen_patched/` EPL-1.0) | read |
| Server, invoker, Restate Lite | `restate@fa4262f` | BSL 1.1 | read only; never a dependency |
| `restate-sdk` crate | `sdk-rust@968ecd0` | MIT | not usable: needs a Restate server; not a WASI component |

## Gaps

- No Restate code was built or run; no `wasm32-wasip2` check of
  `sdk-shared-core` (its default dependencies are pure Rust).
- The benchmark hardware is not published; the per-step figure is derived.
- Issue searches were keyword samples, not complete lists.
- The reference `agent` repo has no licence; if Restate adds one, its
  `history.ts` chunking and `turns.ts` routing become candidates to port.
