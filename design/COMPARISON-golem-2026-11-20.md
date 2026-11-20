# The Golem platform: what watane's agentic layer can take from it (2026-11-20)

A record, not a contract. It compares the Golem platform (the worker executor,
its oplog, its agent model, its host interfaces and its tests) with
`ARCHITECTURE-PROPOSAL.md` (revision 3). Golem is the system closest to
watane's shape: a Wasmtime component host that runs durable agents. golem-ai,
Golem's LLM library, is covered in `COMPARISON-harness-loop` and
`-sessions`; it is mentioned here only where the platform changes the picture.

Method: a shallow clone, read by hand, plus the docs site that lives in the
same repository (`docs/src/content/next/`), the repository's own agent guides
(`AGENTS.md` files and `.agents/skills/understanding-durable-execution/`),
GitHub issues, releases and older tags read with `gh`. Nothing was built or
run.

| Source | Repo @ commit | Date |
|---|---|---|
| Golem platform | golemcloud/golem @`09a6cae` | 2026-11-20 |
| Golem v1.5.10 WIT (for what was removed) | golemcloud/golem tag `v1.5.10` | – |
| Durable Streams protocol (used by Golem) | durable-streams/durable-streams @`461b402` | – |

Paths below are relative to the golem repository at `09a6cae` unless they
name another repo.

## 1. Licences

Golem's licence changed twice (from `gh api repos/golemcloud/golem/commits?path=LICENSE`):

| Period | Licence of the repository root |
|---|---|
| 2023-11-24 (`d02f710`) to 2025-05-23 | Apache-2.0 |
| 2025-05-23 (`92da374`, PR #1647) to 2026-02-26 | Golem Source License v1.0 (source-available, Apache-2.0 after 4 years) |
| since 2026-02-26 (`ef5c95e`, PR #2806) | Business Source License 1.1, with an Additional Use Grant; each version becomes Apache-2.0 four years after its release |

Per crate at `09a6cae`:

| Licence | Crates and paths |
|---|---|
| BSL 1.1 (`license-file = "../LICENSE"`) | `golem-worker-executor`, `golem-worker-executor-tests`, `golem-worker-executor-test-utils`, `golem-test-framework`, `golem-common`, `golem-service-base`, `golem-model-mock`, `golem-schema`, `golem-worker-service`, `golem-shard-manager`, `integration-tests`, the root `wit/`, `docs/`, `.agents/` |
| Apache-2.0 | `sdks/rust/golem-rust` (with its own copy of the WIT in `sdks/rust/golem-rust/wit/deps/`), `golem-rust-macro`, `golem-tool-metadata`, `golem-native-tool`, `golem-size-analysis`, `golem-openapi-client-generator` |
| Apache-2.0 (separate repo) | golemcloud/golem-ai |

Oddity: source files in the BSL crates still carry a header that says
"Golem Source License v1.1" (for example
`golem-worker-executor/src/services/golem_config/oplog.rs`). Either way they are
not open source.

watane: there is no `LICENSE*` file at the repo root. `net` and `log` declare
`MIT OR Apache-2.0`; the other crates declare nothing, and `deny.toml` allows
permissive licences only. So:
- **BSL code is read only.** Copying it would make our files derivative works
  under BSL. The "frameworks, libraries" safe harbour does not lift that.
- **Apache-2.0 code may be ported** with its NOTICE. That is the Rust SDK and
  the WIT copies inside it, and any Golem tag released before 2025-05-23
  (v1.2.x and older), which reflects a design Golem has since dropped.

## 2. Golem in one page

- **An agent** is a WebAssembly instance of an agent type with constructor
  arguments as its key, its own memory, files, oplog and wallet of permission
  cards (`docs/.../concepts/agents.mdx`). It handles one invocation at a time;
  other calls queue. Durable is the default; an ephemeral agent gets a fresh
  instance per call, like a watane reactor.
- **The oplog** is an append-only log per agent of every host interaction and
  its result, plus each invocation (`wit/deps/golem-1.x/golem-oplog.wit`, about
  60 entry kinds). A durable host call is a `start` entry and an `end` (or
  `cancelled`) entry. Recovery builds a fresh instance and re-runs the guest:
  recorded calls return their recorded results, and at the end of the log the
  agent goes live (`concepts/durable-execution.mdx`).
- **The resident runtime is disposable.** Crash, restart, suspension and
  eviction all recover the same way: new Store, replay, go live
  (`.agents/skills/understanding-durable-execution/SKILL.md`, axiom 2).
- **Runtime:** a fork of Wasmtime 50.0.1 with component-model async (WASI
  P3) (`Cargo.toml` lines 353-376). watane runs Wasmtime 48 and WASI 0.2, so
  none of the executor's host code would link to ours as is.
- **Size:** `golem-worker-executor` and its tests are about 825 000 lines of
  Rust. The rules for the durable host alone fill a 260-line
  `golem-worker-executor/src/durable_host/AGENTS.md`.

## 3. Capability: the loop as durable steps

**watane plans.** The kernel runs the OpenAI loop as short calls, one step per
call, saved between calls; tool calls are jobs (10, 12 S5; summary lesson 3).

**Golem does.** The agent loop is ordinary code inside one long invocation.
Each model call is an outgoing HTTP request and each tool call a Golem tool
call; both are recorded. After a crash the loop re-runs; completed calls come
from the oplog and only the in-flight call is sent again
(`concepts/durable-execution.mdx`). There is no wall-clock limit on an
invocation by default (`max_invocation_duration: None`,
`golem-worker-executor/src/services/golem_config/runtime.rs`).

Library authors can group effects into one durable step with a typed request
and result. The Rust SDK (Apache-2.0) shows the shape:

```rust
// sdks/rust/golem-rust/src/durability.rs
match self.begin() {
    CustomDurableInvocation::Live(invocation) => {
        let result = body();
        Self::finish(invocation, serializable_result, self.forced_commit);
        result
    }
    CustomDurableInvocation::Replayed(ref invocation) =>
        self.replay_serializable(invocation)...
}
```

Its host side is `begin-custom-durable-invocation` in
`wit/deps/golem-durability/golem-durability.wit`: it returns `live` (run the
body, then `finish`) or `replayed` (the recorded response, no body). Each step
is classified `read-local`, `write-local`, `read-remote` or `write-remote`.
An unfinished step is re-run whole on recovery, so its body must be safe to
repeat (`develop/durability.mdx`).

**How it is tested.** `golem-worker-executor-tests/tests/durability.rs`, for
example `custom_durability_crash_mid_live_invocation_reexecutes_whole_body`
and `custom_durability_recovers_with_one_completed_and_one_suspended_sibling`.

**Worth copying.** The Live/Replayed step memo, as a pattern inside the
kernel: before a step that causes an effect, look up a step record by a stable
key; if it holds a result, use it; else run and record. In watane the record
is a state key written in the same call, so commit-on-success gives the
"finish" for free.

**Pitfalls.**
- Golem removed every guest knob over durability in v1.6.0 (2026-10-19). v1.5.10
  had `set-oplog-persistence-level` (`persist-nothing`,
  `persist-remote-side-effects`, `smart`), `set-idempotence-mode`, atomic
  regions (`mark-begin-operation`) and `set-oplog-index` (a jump); at v1.6.0
  `golem-host.wit` keeps only `oplog-commit(replicas)`. The docs now say "There
  is no guest idempotence-mode switch" (`develop/durability.mdx`).
- Replay under async needs strict rules: completions are delivered at recorded
  markers, at most one event per blocked wait (`durable_host/AGENTS.md`,
  "Observe"). A missing recording is a determinism bug, not noise.

## 4. Capability: approvals and long waits

**watane plans.** No call waits. An ask is an item; the owner answers with a
new call; an unanswered ask is denied after 10 minutes (3, 7, 12 S4). Pending
asks live in the host's memory (2.3).

**Golem does.** A **promise** is a durable slot:

```wit
// wit/deps/golem-1.x/golem-host.wit
create-promise: func() -> promise-id;
get-promise: func(promise-id: promise-id) -> get-promise-result;   // get: async func() -> list<u8>
complete-promise: func(promise-id: promise-id, data: list<u8>) -> bool;  // false if already completed
```

- Only the creating agent can await it. Another agent, the REST API or a
  **webhook** (a one-time URL with an HMAC, `create-webhook` in
  `wit/deps/golem-agent/host.wit`) completes it.
- An agent that only waits is **suspended**: unloaded after `suspend_after`
  (default 10 s, `services/golem_config/runtime.rs`), with the wait recorded and
  a wake-up persisted in the scheduler. Completion loads and replays it.
- **A deadline** is ordinary code: schedule a call that completes the same
  promise with a default answer; the first completion wins
  (`concepts/communication.mdx`).
- **Identity:** a `PromiseId` is `{agent_id, fingerprint, oplog_idx}`. The
  fingerprint names the agent's incarnation, so an id from a deleted and
  re-created agent finds nothing (`SKILL.md`, "Durable versus resident state").
- **Approval as middleware.** Tool middleware, set in the manifest, wraps every
  tool call; the agent cannot list, remove or bypass it, and it may wait hours
  for a person. Middleware cannot widen: each call it passes on is checked
  against the agent's cards again, with the arguments the next layer receives
  (`concepts/tools.mdx`, `wit/deps/golem-tool/middleware.wit`).

**How it is tested.** `tests/invocation_waits.rs`, `tests/suspension_races.rs`,
`tests/scheduler_waits.rs`; the test DSL has `complete_promise`
(`golem-test-framework/src/dsl/mod.rs`).

**Worth copying.** "Complete returns false if already completed" as the rule
for asks: first answer wins, and the loser learns it. Binding ids to an
incarnation. Re-checking after a layer changes arguments.

**Pitfalls.** #3685: a `PromiseId` could not be serialised by the TS default
snapshot. Ids that cross a persistence boundary need a stable encoding.

## 5. Capability: sessions, transcripts, replay, attach

**watane plans.** The kernel keeps a transcript with an index per finished
entry, never per delta. `attach(after)` replays stored entries, then live.
After a restart, streams are gone and the session is resumed with `claude
--resume` (3, 10, 12 S7).

**Golem does.**
- The oplog is the transcript and the state at once. Status is "folded from
  the oplog"; caches are disposable (`SKILL.md`, table "Durable versus
  resident").
- **Storage is layered, not compacted.** The primary layer is indexed storage;
  older entries move to compressed layers and then to blob storage
  (`operate/persistence.mdx`). Payloads above 64 KiB go to blob storage with a
  reference in the entry (`max_payload_size`,
  `services/golem_config/oplog.rs`). Nothing is summarised.
- **Snapshots** shorten replay: periodic, every N invocations, or user
  `save-snapshot`/`load-snapshot` exports. A snapshot that fails to load is
  rejected and the previous one is tried (`durable_host/AGENTS.md`,
  "Snapshotting").
- **Fork and revert:** `fork-agent(source, oplog-idx-cut-off)` copies the
  prefix into a new agent; `revert-agent` drops the last N invocations; external
  effects are not undone (`golem-host.wit`, `concepts/agent-lifecycle.mdx`).
- **Read-only methods** may be answered from a cache without waiting in the
  agent's queue. The host rejects outgoing HTTP, agent calls and storage writes
  in them (`concepts/agents.mdx`).

**How it is tested.** `tests/durability.rs` (snapshot recovery across several
restarts, falling back to an older snapshot), `tests/fork.rs`, `tests/revert.rs`,
`tests/readonly.rs`, and `tests/goldenfiles/` (350 binary files of serialised
records, so an encoding change shows up as a diff).

**Worth copying.** Golden files for every stored format. Large values out of
line with a reference, rather than in the hot record. Read-only calls as a
declared property that the host checks.

**Pitfalls.**
- #3773 (open, 2026-08-25): updating a long-lived agent (18 400 oplog entries,
  11 hours) to a new revision, automatic or manual, "can permanently trap" it
  with no path back. A history that must be replayed against new code is a
  liability for long sessions.
- #3620: the map from idempotency key to oplog index was kept in memory for
  every invocation; it had to be bounded.
- Stream history is never reclaimed: "Golem has no configurable
  stream-retention horizon" (`operate/durable_streams.mdx`).

## 6. Capability: delegation and agent-to-agent calls

**watane plans.** Calls between mods are nested and synchronous; cycles are
refused (host 4.2). Tool calls are jobs: a stream id now, a top-level call
later (6). Subagents of an OpenAI session are child sessions through a `task`
function (11).

**Golem does.**

```wit
// wit/deps/golem-agent/host.wit (resource wasm-rpc)
invoke-and-await: func(method-name: string, input: schema-value-tree,
                       scope-card: option<borrow<permission-card>>) -> result<...>;
invoke: func(...) -> result<invocation-metadata, rpc-error>;          // trigger
schedule-cancelable-invocation: func(scheduled-time: datetime, ...) -> result<...>;
```

- **Exactly once at the callee.** The key is derived from the caller's `start`
  index and reused by every attempt; the callee dedupes on it before it queues
  the invocation (`durable_host/AGENTS.md`, "RPC"; `SKILL.md`, axiom 3).
- **Deadlock** between two agents that await each other is the developer's
  problem: "use trigger for one direction" (`concepts/communication.mdx`).
- **Authority:** an agent RPC starts a new owner, which acts with its own
  wallet; the caller may pass a `scope-card`, a narrowed card for that call
  only. A nested tool call keeps the caller's owner and bindings
  (`concepts/tools.mdx`). Derived cards only narrow; revoking a card revokes
  everything derived from it (`wit/deps/golem-permissions/derive.wit`,
  `revoke.wit`).
- **Tools** run in a fresh sandbox per call, on the agent's filesystem, with
  the agent's cards, and exactly once (`concepts/tools.mdx`). Tool metadata
  carries MCP's `read-only`, `destructive` and `idempotent` flags
  (`wit/deps/golem-tool/common.wit` lines 189-197).
- **Fork** for parallel work: `fork()` returns `original` or `forked` on each
  side, like Unix `fork`.

**How it is tested.** `tests/rpc.rs`, `tests/rpc/`, `tests/invocation_receipts.rs`,
`tests/owner_tool_isolation.rs`, `tests/scope_cards.rs`. Tests capture the key
of each attempt (`TestExecutorOverrides::wrap_rpc`), assert they are equal, and
assert one `AgentInvocationStarted` in the target's oplog
(`golem-worker-executor-tests/tests/AGENTS.md`).

**Worth copying.** A job key derived from a durable position, reused by every
attempt, checked by the receiver. The `idempotent` flag on tools.

**Pitfalls.** #3563: the RPC path re-issued "LaunchNewWorker" on replay and on
already-active targets. Replay must not repeat the setup of a callee either.

## 7. Capability: streams to clients

**watane plans.** H1: one producer, one consumer, memory only, batched item
calls, backpressure, `unknown-stream` after a restart (3).

**Golem does.** Two kinds (`develop/durable-streams.mdx`):
- **Method streams.** An agent method can take and return
  `stream<T>`. Items are recorded in the producer's oplog
  (`stream-registered`, `stream-items`, `stream-terminal`) and the consumer
  keeps a journal (`stream-session`). Readers resume by offset, never by
  connection: "Commit before `DurableLiveStreamBus::publish_committed`; the bus,
  readers, sockets and transports are resumable optimizations"
  (`durable_host/AGENTS.md`, "Durable streams"). The producer commits with
  `CommitLevel::Always` before it publishes
  (`worker/durable_stream_producer/mod.rs` lines 406-418).
- **HTTP clients** use the Durable Streams protocol (MIT): opaque offsets,
  catch-up then long-poll or SSE, `Stream-Up-To-Date`, idempotent producers
  `(id, epoch, seq)`, a newer epoch fences an older writer
  (`invoke/making-custom-apis/durable-streams.mdx`).
- A checkpoint advances only after the whole batch is consumed; a partly
  consumed batch does not promote it.
- Cancelling a session cancels the streams, not the computation: "Session
  deletion does not interrupt or terminate the agent invocation".
- Limits are fixed or set: 16 MiB per item, 16 live readers per stream, 1024
  streams per session, `live_stream_event_broadcast_capacity` 1 to 1024
  (`operate/durable_streams.mdx`).

**How it is tested.** `tests/tool_streaming/`, `tests/external_durable_stream.rs`,
`tests/concurrent_delivery_order.rs`. Rule: crash the producer after a
committed `stream-items` and the consumer after a journaled item; assert each
item once and one terminal (`tests/AGENTS.md`).

**Worth copying.** Commit, then publish. Advance the cursor only after the
whole batch. An explicit up-to-date marker (our `synced`). Takeover fencing:
a newer attach ends the older one. Every stream ends exactly once.

**Pitfalls.** The top flaky test is a stream test,
`mcp_stdout_is_durable_before_consumption_and_projects_only_streamable_content`,
5.4 % failures (#3839). Durable per-item streams are hard to get right and
cost a commit per record.

## 8. Capability: host-call durability versus commit-on-success

**watane plans.** A mod's writes commit when its call succeeds (TECHNICAL 4).
Every host effect is a request that applies after the whole chain commits (H5,
7). A failed call leaves nothing behind.

**Golem does.**
- Entries are appended in memory and committed in batches: by default every
  128 operations (`max_operations_before_commit`), at the end of an invocation,
  and at barriers.
- **The barrier is the external write.** For a `write-remote` call the oplog
  commits before the effect starts and again after the `end`:

```rust
// golem-worker-executor/src/durable_host/concurrent/call/terminal.rs:847
if function_type == DurableFunctionType::WriteRemote || forced_commit {
    public_state.worker().commit_oplog_and_update_state(CommitLevel::DurableOnly).await?;
```

  The crash matrix states why: "both shared start paths commit a `WriteRemote`
  Start ... before allowing the effect. A crash during that commit cannot lose
  a request that already left the executor"
  (`.agents/skills/understanding-durable-execution/reference/crash-matrix.md`).
- **The ambiguous window.** After the effect and before `end`, recovery
  re-sends. Exactly-once outside Golem needs an idempotent peer. Golem adds an
  `idempotency-key` header to outgoing HTTP itself
  (`concepts/durable-execution.mdx`); `generate-idempotency-key` gives a key that
  is committed before the guest sees it.
- **Denials are recorded.** A permission denial is written into the `start`,
  so replay makes the same decision without checking again (crash matrix,
  "After a denied Start").
- **Secrets** are opaque handles. A host call that takes `borrow<secret>`
  substitutes the plaintext at the wire; `reveal` is a separate import, and
  each reveal is recorded without the value (`wit/deps/golem-secrets/reveal.wit`).

**How it is tested.** The crash-window rules in `tests/AGENTS.md`: crash
before `start`, after the effect before `end`, after `end` before delivery,
after `AgentInvocationFinished`, during snapshot load, during an update.

**Worth copying.** H5 already is Golem's "commit `start` before the effect".
What we lack is the rule's other half: the commit before an effect must be
durable even if other commits become lazy (N4).

**Pitfalls.** Golem's single-server mode replicates SQLite with Litestream
about once a second; the docs warn that losing the disk loses the newest
oplog writes, "and agents then replay from an earlier point and may run side
effects again" (`deploy/server.mdx`). A lazy commit before an external effect
turns into a repeated effect.

## 9. Capability: per-call cost

**watane.** A call that writes nothing costs about 20 µs. A writing call costs
about 4 ms, almost all one `F_FULLFSYNC`, on one host-wide redb writer: about
240 writing calls per second for all mods (3, 18 V6).

**Golem.**
- **Local server storage:** one SQLite database per agent and namespace
  (`IndexedStorageConfig::KVStoreMultiSqlite`, `cli/golem/src/config.rs` lines
  313-321; `storage/indexed/multi_sqlite/mod.rs`). So agents do not share a
  writer.
- **Durability of a commit:** WAL mode (`golem-common/src/config.rs` line 508)
  and sqlx's default `synchronous = FULL`. Nothing sets `fullfsync`, so on
  macOS a commit is a plain `fsync`, which does not flush the drive's cache.
  It is cheaper than our commit and weaker against power loss. (Not measured
  here.)
- **Commits per model call:** two forced commits per `write-remote` call (before
  and after), plus one per committed stream record; other entries ride along
  in batches of up to 128.
- **Replay cost** grows with history until the next snapshot; a suspended agent
  is replayed on every wake. Snapshots are off by default
  (`SnapshotPolicy::Disabled`).
- **Fuel** is metered for billing, borrowed 10 000 units at a time; the epoch
  ticks every 10 ms (`runtime.rs`).

**Comparison.** Per model call Golem pays about two small fsyncs on a
per-agent file; watane pays one 4 ms full fsync on a shared writer per writing
call, and nothing per delta. Golem's per-agent files and its "durable only
before an external effect" rule are the two levers N4 asks about.

## 10. Other host features in brief

| Feature | Golem | watane |
|---|---|---|
| HTTP | WASI P3 HTTP; recorded; in-flight request resent with an idempotency key; a partly read body may resume with `Range` (crash matrix) | H3 HTTPS streams; restarts a whole turn |
| Secrets | opaque handles; host substitutes at the wire; `reveal` is an import and recorded | ChatGPT tokens stay in the app; the judge key sits in the checked provider's state (9) |
| Quotas | `reserve(token, amount)` then `commit(used)`; reject, pause or stop (`wit/deps/golem-quota/types.wit`) | `max-budget-usd` refused for OpenAI (11) |
| Update | automatic (replay against new code; stays on old version if calls differ) or snapshot-based (`update-mode`, `golem-host.wit`) | an update ends the mod's streams (3, S6); state keys carry over |
| Limits | memory per agent; fuel metered; no default wall-clock limit; idle agents suspended after 10 s | 10 s budget per call; fuel; one call per mod at a time |
| Retries | composable retry-policy trees in WIT (`golem-retry.wit`); a trap is permanent | lesson 6 of the summary: typed errors, no sleeping in calls |

## 11. How Golem tests the platform

- **An in-process executor over real storage.** Tests start the executor,
  store a precompiled test component (`test-components/`, 52 of them, several
  per SDK language) and drive it through a DSL (`golem-test-framework/src/dsl/mod.rs`):
  `simulated_crash`, `resume`, `complete_promise`, oplog reads and searches.
  "Tests here must never spawn external processes."
- **One storage contract, five backends.** `tests/indexed_storage.rs` runs the
  same tests over in-memory, Redis, SQLite, multi-SQLite and Postgres.
- **Rules that make tests fail under a wrong model** (`tests/AGENTS.md`;
  `reference/testing-patterns.md`, eleven patterns):
  - "Count effects, not echoes": assert a counter at the provider, not equal
    return values.
  - Assert identity (the key of each attempt) and the execution count
    separately.
  - Crash at named windows.
  - "Restart for real": drop the executor and start it again over the same
    storage; a cursor reset is not a restart. Then check state and log shape:
    one `Finished` per key, every `Start` has a terminal.
  - Completed replay performs no new effect: stop or count the dependency
    before the restart.
  - Asymmetric orderings: completion order differs from start order in at
    least one case.
  - "Replay errors are findings": never retry or loosen a test to make it
    green.
  - Prefer production behaviour over test-only hooks; no sleeps for ordering.
- **Fakes.** `golem-model-mock` (BSL) is a scripted, in-process
  OpenAI-compatible server: each step has expectations (model, tool names,
  last message role, authorization header) and a response; a mismatch is
  permanent; `verify()` checks every step was used.
- **Golden files** for serialised records (`tests/goldenfiles/`, 350 files).
- **No deterministic simulation.** No madsim, turmoil, loom or shuttle;
  proptest strategies for values. Flakiness is tracked in a generated issue
  (#3839) that scores each test over 30 days of CI.

## 12. Lessons for watane

### Adopt

1. **A commit before an external effect is always durable.** Golem forces the
   oplog to storage before and after every `write-remote` call, and batches the
   rest. watane's H5 already orders "commit, then act". If N4 makes commits
   lazy, the rule must be: a call chain that issues a host request commits
   durably before the host acts on it; calls that only write state may commit
   lazily. Otherwise a crash loses the record of an effect that happened.
2. **Keys from durable positions.** Golem derives RPC and tool keys from the
   caller's `start` index and the callee dedupes. watane: a job key from
   `(session, step, call_id)`; a client key on `start`, `say` and `answer`; the
   receiver returns the recorded result for a known key.
3. **A step record, written before its effects.** The kernel writes the step
   (turn n, call ids, job keys) in the same call that issues the jobs. After a
   restart a step with missing results is repaired: re-issue under the same key
   if the tool says `idempotent`, else record `aborted` (summary lesson 2).
   This is the Live/Replayed pattern of `golem-rust/src/durability.rs` without
   an oplog.
4. **First answer wins.** An ask completes once; a late answer, the expiry or a
   cancel learns that it lost. Ask ids carry the session's incarnation.
5. **Commit, then publish; cursor after the whole batch.** Already true for
   pushes (3). Add: the kernel's cursor advances only when the item call that
   holds the batch commits, and a newer `attach` by the owner ends the older
   stream with `replaced`.
6. **Host-held secrets for mods.** Golem substitutes a secret at the wire and
   records each reveal. The checked provider's judge key could be held by the
   host and added to requests to the judge host, so the mod never holds it.
7. **The test rules of section 11**, especially counting effects, named crash
   windows and real restarts.

### Avoid

1. **An oplog with transparent replay.** It needs a deterministic guest, a
   recording for every host call, completion markers under async, and replay
   on every wake. Golem's own rules for it run to hundreds of lines, its long
   histories break updates (#3773), and it removed guest durability knobs in
   v1.6.0. watane's reactors with explicit state avoid the whole class.
2. **Guest-chosen durability levels.** Golem had them and removed them. N4
   should stay a host policy, not a mod API.
3. **Durable per-delta streams.** Golem commits each stream record; it is its
   flakiest area. Keep deltas live only (3).
4. **Unbounded history.** Golem keeps stream history forever. Keep the owner
   quota and explicit `delete` (summary, section 10).

### Proposal changes, by section

- **3 Streams.** Say what a failed item call does: the same batch, with its
  sequence number, is delivered again up to 3 times, then the stream ends with
  `consumer-failed`; the consumer drops a batch whose sequence it has stored.
  A second `attach` by the owner ends the first with `replaced`.
- **6 Agent tools.** `describe` returns `idempotent: bool` (MCP's annotation, as
  in `golem-tool/common.wit`), next to `parallel`. `call` takes a job key; the
  host passes it to the tool mod.
- **7 Requests.** Each request carries a key derived from the chain and the
  request's ordinal, so the host drops a duplicate. A chain that issues a
  request commits durably, whatever N4 decides.
- **9 Providers.** Option: the judge key as a host-held secret for the judge
  host, added at the wire.
- **10 Kernel.** Step records before effects (Adopt 3); client keys on
  `start`, `say`, `answer`; kernel-owned asks (OpenAI broker) as state records,
  first answer wins, expiry recorded as an answer. A token budget per session
  as reserve-then-commit, since `max-budget-usd` is refused for OpenAI (11).
- **12 S7 Resume.** After a restart the kernel also repairs open steps (Adopt
  3) before it marks the session idle.
- **15 N4.** Golem's evidence: commit lazily except before external effects;
  per-owner storage removes the shared writer. Concrete option: item calls
  that issue no request commit with redb's non-durable commit; the next durable
  commit, or a 100 ms timer, makes them durable. A later step: one redb file
  per mod, which matches "each mod's writes commit when its own call succeeds";
  the ordering of H5 across files then needs a design of its own.

### What challenges a watane design choice

- **Reactors and commit-on-success versus an oplog.** Golem resumes in the
  middle of a step; watane resumes at the last committed step and restarts the
  turn. With 10 s calls and no writes per delta, that loses at most one model
  turn, which both systems re-send anyway. Keep reactors.
- **One kernel for all sessions.** Golem runs one agent per session and scales
  by more agents; a long loop runs "in its own agent ... so the agent that
  users talk to stays responsive". watane's kernel is one mod, and the host
  runs one call per mod at a time, so every session's item calls, `read`s and
  `say`s queue behind each other. Batches keep it cheap, but a slow call (a
  compaction, a large `read`) stalls every session. Measure in V8. If it
  bites: declared read-only exports that the host may run beside the mod's
  current call, rejecting any write, as Golem does for read-only methods.
- **Authority across calls.** Golem's agent RPC starts a new owner that acts
  with its own wallet plus an optional narrowed `scope-card`; watane's G7 makes
  consent follow the origin client. Golem confirms the narrowing-only rule and
  cascading revocation; it does not argue for changing G7.

## 13. Test cases watane should write

1. **Effect before durability.** Fail the host between a chain's commit and its
   `claude-code.start`: after restart no `claude` runs and the session is idle.
   Kill after spawn and before the first item: crash cleanup kills the process
   group.
2. **Failed item call.** The kernel's `items` call traps once: the same batch
   arrives again with the same sequence; the transcript holds each entry once
   (count entries, not echoes).
3. **Open step after restart.** Restart the host after the kernel committed
   the jobs of a step and before their results: a non-idempotent tool gets an
   `aborted` result; an idempotent one runs again under the same key; a counter
   in the tool mod shows one execution per key that completed.
4. **Client keys.** `say` twice with the same key gives one entry and the same
   index; `start` twice gives one session.
5. **Ask races.** Answer, expiry and cancel in each order (asymmetric): one
   outcome is recorded; the others get `already-resolved`.
6. **Attach.** `attach(after)` returns exactly the later entries, then
   `synced`. A second `attach` ends the first with `replaced`. A batch whose
   call failed does not move the cursor.
7. **Real restart.** Drop the host, open the same home: state, transcript
   shape (every tool call has one result) and status agree.
8. **Lazy commit (N4, if taken).** Crash inside the lazy window: state may lose
   the last item batches but never a record that preceded a host request.
9. **Golden files** for each stored kernel record and transcript entry
   version.
10. **Scripted Responses server** with per-step expectations (model, tool
    names, last role, account header) and a final `verify()`, like
    `golem-model-mock`, served by `dev`.
11. **Writer contention.** Two mods writing at full rate: neither starves
    (V6 extended).
12. **Fork.** A forked session shares no handle, ask id, job key or stream id
    with its source.

## 14. Reusable code

| What | Path @ `09a6cae` | Licence | How |
|---|---|---|---|
| Step memo shape (`Durability::run`, Live/Replayed, `forced_commit`) | `sdks/rust/golem-rust/src/durability.rs` | Apache-2.0 | read; port the shape into the kernel |
| Saga with compensation | `sdks/rust/golem-rust/src/transaction/` | Apache-2.0 | read |
| Quota reserve/commit, tool middleware, secret reveal, promise WIT | `sdks/rust/golem-rust/wit/deps/golem-quota/`, `golem-tool/`, `golem-secrets/`, `golem-1.x/` | Apache-2.0 (copy in the SDK) | read; borrow names and shapes |
| Tool annotations (`read-only`, `destructive`, `idempotent`) | `sdks/rust/golem-rust/wit/deps/golem-tool/common.wit` | Apache-2.0 | read |
| Durable host rules, crash matrix, testing patterns | `golem-worker-executor/src/durable_host/AGENTS.md`; `.agents/skills/understanding-durable-execution/reference/crash-matrix.md`, `testing-patterns.md`; `golem-worker-executor-tests/tests/AGENTS.md` | BSL 1.1 | read |
| Oplog commit points and storage | `golem-worker-executor/src/durable_host/concurrent/call/{start,terminal}.rs`, `services/oplog/mod.rs` (`CommitLevel`), `storage/indexed/multi_sqlite/` | BSL 1.1 | read |
| Scripted model server | `golem-model-mock/` | BSL 1.1 | read; write our own |
| Test DSL (`simulated_crash`, `complete_promise`) | `golem-test-framework/src/dsl/mod.rs` | BSL 1.1 | read |
| Durable Streams protocol and clients | durable-streams/durable-streams @`461b402` | MIT | read; a dependency only if pages need resumable HTTP streams |

Nothing here is a dependency. The SDK needs Golem's host and WASI P3; the
executor is BSL.

## 15. Gaps

- Nothing was run, so the per-commit cost of Golem on macOS is inferred from
  its settings, not measured.
- sqlx's default `synchronous = FULL` is from the sqlx 0.8.6 documentation
  (docs.rs), not from source.
- The exact PR that removed persistence levels was not found; the removal is
  dated by comparing `golem-host.wit` at `v1.5.10` and `v1.6.0`.
- The executor was read selectively (durable host, oplog, stream producer,
  config); the scheduler, sharding and filesystem snapshots were skipped.
