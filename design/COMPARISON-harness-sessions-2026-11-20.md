# Sessions in other agent harnesses: a comparison (2026-11-20)

A record, not a contract. It mines other harnesses for capability group 5,
sessions, of the agentic layer (`ARCHITECTURE-PROPOSAL.md` revision 3). It
cites the proposal by section number. Accepted lessons move into the
contracts through a design slice, as the proposal says.

## 0. Sources and ground rules

### The harnesses read

All were shallow clones on 2026-11-20 in the session scratchpad. Code was
read, never run.

| Repo | Commit | License | What was read |
|---|---|---|---|
| openai/codex (`codex-rs`) | `bb5b9c6` | Apache-2.0 | `rollout/`, `history/`, `thread-store/`, `state/`, `core/src/context_manager/` |
| sst/opencode (now `anomalyco/opencode`) | `b7f0b90` | MIT | `packages/core/src/{bus.ts,session/,snapshot.ts,database/}`, `packages/schema/src/{event.ts,session-event.ts}` |
| block/goose (now `aaif-goose/goose`) | `80ce943` | Apache-2.0 | `crates/goose/src/session/`, `crates/oi-kit-provider-types/src/conversation.rs` |
| badlogic/pi-mono | `e5d93ea` | MIT | `packages/coding-agent/src/core/session-manager.ts`, `packages/durable/`, `packages/ai/src/api/transform-messages.ts` |
| OpenHands | `3df9648` (front end only) | MIT | Front end in the clone; the backend is `OpenHands/software-agent-sdk` at `main`, read through `gh api`, **not pinned** |
| google-gemini/gemini-cli | `d4cb171` | Apache-2.0 | `packages/core/src/services/{chatRecordingService,gitService}.ts`, `utils/sessionUtils.ts`, `cli/src/utils/sessionCleanup.ts` |
| cline/cline | `20b0427` | Apache-2.0 | `sdk/packages/core/src/{session,hooks,hub}/`, `apps/vscode/src/sdk/legacy-*.ts` |
| zed-industries/zed | `10d1b2a` (sparse clone) | GPL-3.0-or-later | `crates/agent_ui/src/thread_metadata_store.rs`; `crates/agent/src/db.rs` from `main` via `gh api` |
| 0xPlaygrounds/rig | `0019e24` | MIT | `crates/rig-core/src/conversation/`, `crates/rig-memory/`, `crates/rig-agent/src/run/`, `crates/rig-cassette/` |
| golemcloud/golem-ai | `0923caf` | Apache-2.0 | `llm/llm/src/durability.rs`, `test/llm/` |
| antinomyhq/forge (now `tailcallhq/forgecode`) | `a22b492` | Apache-2.0 | `crates/forge_repo/`, `forge_domain/`, `forge_app/`, `forge_snaps/` |
| langchain-ai/langgraph | `58a6e1c` | MIT | `libs/checkpoint/`, `libs/checkpoint-sqlite/`, `libs/checkpoint-conformance/`, `libs/langgraph/langgraph/channels/delta.py` |
| anthropics/claude-agent-sdk-python | `69846cd` | MIT | `src/claude_agent_sdk/{types.py,_internal/}`: what the `claude` binary offers for sessions |

Citations read `repo@commit:path:line`. Paths are relative to the repo root.

Issues were found with `gh search issues`. GitHub rate-limited some searches,
so the issue lists are samples, not complete.

### Licenses

watane has no `LICENSE` file at its root. `log` and `net` declare
`MIT OR Apache-2.0`; the other crates declare none. `deny.toml` allows
permissive licenses only (MIT, Apache-2.0, BSD, MPL-2.0 and similar).

- MIT and Apache-2.0 code may be ported. Apache-2.0 asks us to keep its
  notice and mark changes.
- **Zed's agent crates are GPL-3.0. Read them for ideas; copy nothing.**

### The watane facts that shape every answer

- A mod's state is `get`, `put` and `delete` on keys (host TECHNICAL, state
  interface). **There is no prefix scan.** Lists need their own index keys.
- Limits per mod (host TECHNICAL, limits table): 4 MiB per value, 16 MiB
  written per call, **256 MiB for all of a mod's state**, 10 s and 64 MiB
  memory per call.
- One call commits all its writes atomically, or none. A writing call costs
  about 4 ms, on one host-wide writer (proposal 3, V6).
- The kernel runs in a fresh instance per call. It holds nothing in memory
  between calls.
- Claude keeps its own session files under `~/.claude/projects`. The kernel
  keeps a second, indexed transcript for clients (proposal 2.3, 10).

---

## 1. Transcript format and schema versioning

**watane needs:** an entry format that a newer kernel can read, and that an
older one does not silently misread.

**Codex.**
- *Interface.* One JSONL line per `RolloutItem`:
  `{timestamp, ordinal?, type, payload}`
  (`codex@bb5b9c6:codex-rs/rollout/src/recorder.rs:2100`). The enum has
  `SessionMeta`, `ResponseItem`, `Compacted`, `TurnContext`, `EventMsg` and
  more (`codex-rs/history/src/lib.rs:442`).
- *Versioning.* No version field per line. Enums end in
  `#[serde(other)] Other` (`codex-rs/protocol/src/models.rs:1426`), so an
  unknown item type reads as `Other`. `SessionMeta` has a `history_mode`
  (`Legacy` or `Paginated`). An unknown mode is refused
  (`rollout/src/recorder.rs:1188`).
- *Tests.* `protocol/src/models.rs:3916` reads a removed `ghost_snapshot`
  item as `Other`.
- *Pitfall.* A removed feature (`undo`, `features/src/lib.rs:1219`,
  `Stage::Removed`) left lines in old files. The reader strips them
  (`recorder.rs:1183`).

**OpenCode.**
- *Interface.* Each durable event type has its own version:
  `Event.durable({type: "session.forked", durable: {aggregate:
  "sessionID", version: 2}})`
  (`opencode@b7f0b90:packages/schema/src/session-event.ts:192`). The
  envelope is `{aggregateID, seq, version}`
  (`packages/schema/src/event.ts:34`).
- *Versioning.* The stored type includes the version (`versionedType`).
  Rows written before a payload rename are upgraded on read
  (`SessionLegacy.event`, `packages/core/src/bus.ts:345`).
- *Pitfall.* Migration `20260622170816_reset_v2_session_state` deletes every
  v2 session row and event: a format change that dropped data instead of
  migrating it.

**Pi (coding agent).**
- *Interface.* A header line `{type: "session", version, id, cwd,
  parentSession?}`; `CURRENT_SESSION_VERSION = 3`
  (`pi-mono@e5d93ea:packages/coding-agent/src/core/session-manager.ts:41`).
- *Versioning.* Chained, idempotent, in-memory migrations
  (`session-manager.ts:287-347`). The file is rewritten once if any ran.
- *Tests.* `test/session-manager/migration.test.ts`. Real v1-era sessions are
  kept as fixtures (`test/fixtures/large-session.jsonl`).

**rig.**
- *Interface.* `FORMAT = "rig-session/4"` in each log header
  (`rig@0019e24:crates/rig-memory/src/format.rs:33`).
- *Versioning.* **No migration.** A log in another format is refused and
  left byte for byte (`crates/rig-memory/src/dir/tests.rs:87-101`).
  `AgentRun` has `RUN_FORMAT = 7` and refuses others by name
  (`crates/rig-agent/src/run/mod.rs:340-354`).

**Zed (GPL, ideas only).**
- `DbThread::VERSION = "0.3.0"`; data is zstd-compressed JSON in SQLite
  (`zed@main:crates/agent/src/db.rs:192`).
- *Pitfall.* `from_json` treats **any** version it does not know as the old
  agent-1 format (`db.rs:197-210`). A newer file read by an older Zed is
  misparsed, not refused.

**Forge.** No version field. Old shapes are absorbed with
`#[serde(untagged)]` fallbacks
(`forge@a22b492:crates/forge_repo/src/conversation/conversation_record.rs:535-560`).
Issue tailcallhq/forgecode#3260: one row of the wrong shape broke the whole
`conversation list`.

**OpenHands.** No version per event. Events are pydantic with
`extra="forbid"`. Events written before branching have no `parent_id` and
fall back to "the previous event" with no rewrite
(`software-agent-sdk@main:openhands/sdk/conversation/event_store.py:136-154`).
The wire envelope is versioned by URL, not by a field
(`openhands/agent_server/session_protocol.py:1-8`).

**Gemini CLI.** No version field. The format is told by the file extension
(`.json` legacy, `.jsonl` new) and by line shape
(`gemini-cli@d4cb171:packages/core/src/services/chatRecordingService.ts:748-771`).

**Lessons for watane.**
- **Adopt** an envelope per entry: `{v, kind, ...}`, with a version per
  kind, as OpenCode does. A kind changes its version on its own.
- **Adopt** upgrade-on-read: the kernel reads old versions and upgrades them
  in memory (OpenCode, OpenHands). It writes only the current version.
- **Adopt** refusal of the unknown: a kind or version newer than the kernel
  knows becomes a placeholder entry (`unreadable`, raw bytes kept), never a
  guess (avoid Zed's fallback, Forge's untagged guesses).
- **Avoid** a reset migration (OpenCode v2). A kernel update may never drop
  transcripts.
- **Proposal change (10).** Name the entry envelope and its versions in the
  kernel's TECHNICAL when it is written.

---

## 2. Append-only event log or message table

**watane needs:** a store that fits `get`/`put`/`delete`, a 4 MiB value
limit, and one atomic commit per call.

| Harness | Model |
|---|---|
| Codex | Append-only JSONL per thread (the truth) plus an SQLite index for listing (`codex-rs/state/migrations/0001_threads.sql`), backfilled from the files (`0008_backfill_state.sql`) |
| OpenCode | SQLite event table `event(id, aggregate_id, seq, type, data)` plus `event_sequence(aggregate_id, seq)` (`packages/core/src/database/migration/20260323234822_events.ts`); projections build message tables (`session/projector.ts`) |
| Goose | SQLite message table `messages(id AUTOINCREMENT, message_id, session_id, role, content_json, ...)` (`goose@80ce943:crates/goose/src/session/session_manager.rs:1213`) |
| Pi | Append-only JSONL tree, one entry per line with `id` and `parentId` |
| Pi `durable` | Commit log with sequence numbers; SQLite, JSONL and IndexedDB back ends (`packages/durable/README.md`) |
| OpenHands | One file per event, `event-{idx:05d}-{event_id}.json`, plus `base_state.json` (`openhands/sdk/conversation/persistence_const.py:4-10`) |
| Gemini CLI | Append-only JSONL with upserts: the last line per message id wins; `$set`, `$patch`, `$rewindTo` records |
| Cline | Whole-file JSON snapshot, rewritten per turn (`sdk/packages/core/src/session/stores/session-manifest-store.ts:220-245`); plus an SQLite event log with a cursor for clients (`hub/server/hub-event-log.ts`) |
| Forge | One SQLite row per conversation; the whole transcript is one JSON blob, upserted (`crates/forge_repo/src/conversation/conversation_repo.rs:47-63`) |
| Zed | One SQLite row per thread; zstd JSON blob |
| rig | Append-only JSONL per agent, with a compare-and-append store (`crates/rig-memory/src/store.rs:76-104`) |
| LangGraph | A checkpoint per step (full state) plus pending writes; `DeltaChannel` (beta) stores deltas to escape full copies (`libs/langgraph/langgraph/channels/delta.py:25`) |

**Pitfalls.**
- Whole-blob stores grow O(n) per write and O(n²) over a session: Forge
  reached a 6.6 GB database (forgecode#3550); Cline froze on 5-10 MB task
  files (cline#9011); LangGraph added `DeltaChannel` and reports 85 % storage
  bloat (langgraph#7714).
- A torn whole-file write lost Cline's history, and an error path then
  deleted the task folders (cline#7101).
- Codex rollouts reached 3.75 GB with inline base64 images (codex#56423).

**Lessons for watane.**
- **Adopt** an append-only log of immutable entries, one key per entry:
  `s/<session>/e/<index>`. This matches the proposal (10: "an index per
  entry") and gives O(1) work per entry.
- **Adopt** a small head value per session, `s/<session>/head`:
  owner, next index, title, state, Claude uuid, parent. One call writes the
  new entries and the head together, so they never disagree (redb commits
  them in one transaction). This is rig's compare-and-append and pi
  durable's "one atomic commit per session", for free.
- **Avoid** a value that grows with the session. No key may hold "the whole
  transcript" or "all sessions of an owner" without a cap.
- **Avoid** storing binary blobs inline. Store a reference, or truncate (see
  3).

---

## 3. Persistence granularity

**watane needs:** a write rate the single host writer can carry (about 240
writing calls per second for all mods), and no lost user input.

**What they persist.**
- **Codex** filters before writing. Deltas, errors and progress events are
  "transient, non-durable" (`codex-rs/rollout/src/policy.rs:162`). Command
  and MCP output is cut to 64 KiB for persistence (`policy.rs:16-19`).
- **OpenCode** marks each event type `durable` or `ephemeral`. Text deltas
  are ephemeral: "Stream fragments are live-only; Text.Ended is the
  replayable full-value boundary" (`packages/schema/src/session-event.ts:419`).
- **OpenHands** sends deltas as frames that "may be dropped freely; the real
  text arrives with the durable message" (`session_protocol.py:10-21`).
- **Gemini CLI** re-appends a whole message on every status change of its
  tool calls. Many writes per turn, no fsync.
- **Pi durable** commits partial output to watchers every 100 ms but to
  storage at most once per second ("deferred commits"); a crash loses about
  one second (`packages/durable/README.md:621`).
- **Forge** saves before each model request and after the response with all
  tool results (`crates/forge_app/src/orch.rs:277,396`). A crash during tool
  runs loses the whole assistant turn, while the tools' effects stay.
- **Claude Agent SDK** passes `--session-mirror`, and `claude` then writes
  `transcript_mirror` frames with its own transcript entries on stdout
  (`_internal/transport/subprocess_cli.py:672-673`). The SDK batches them:
  per turn, or at 500 entries or 1 MiB
  (`_internal/transcript_mirror_batcher.py`). For watane this is a possible
  source of Claude's entry uuids; it is not in the launch table (4.1), and
  plain `stream-json` lines already carry a `uuid`.

**Pitfalls.**
- Input lost before the first reply: pi wrote the file only on the first
  assistant message (pi#10000). Codex lost a whole day's session after a
  power loss and an update (codex#19037).
- Gemini CLI's per-update upserts multiply lines per message. Its loader
  must merge them by id.

**Lessons for watane.**
- **Adopt** the proposal's rule as is (3, 10): one entry per finished
  message, tool step, ask or verdict; deltas only live. Codex, OpenCode and
  OpenHands all landed on the same rule.
- **Adopt** "the input is an entry before anything runs". `say` and `start`
  store the user input in the same call that asks for the upstream stream.
  If the call fails, neither exists.
- **Adopt** turn markers as entries: `turn-started`, `turn-ended
  {outcome}`. Codex (`TurnStarted`, `TurnComplete`, `TurnAborted`) and
  OpenCode (`session.execution.started|succeeded|failed|interrupted`) need
  them to tell a finished turn from a cut one after a restart.
- **Adopt** a size cap per stored tool output, like Codex's 64 KiB, well
  below the 4 MiB value limit. The live stream can still carry the full
  item.
- **Evidence for N4 (15).** No harness persists per delta. Pi's durable
  package, the only one that persists partial output, does so at most once
  per second. So "wait for evidence" holds: the current design needs no
  weaker durability.

---

## 4. Resume after a crash or restart

**watane needs:** after the app restarts, a session the owner can read at
once and continue with `say` (proposal S7).

### 4.1 Reloading

- **Codex.** Reads the JSONL. A line that does not parse is logged, counted
  and skipped (`codex-rs/rollout/src/recorder.rs:1175-1180`). A
  per-thread writer lock keeps two processes from appending
  (`rollout/src/writer_lock.rs`).
- **OpenHands.** Rebuilds its index from the event files. A missing file
  does not stop the scan: "so one lost file does not make every later event
  unknown" (`event_store.py:466-520`). An unreadable file becomes an
  `UnreadableEventFile` stand-in that keeps its id and place
  (`event_store.py:252-299`). Sequence numbers are never reused, even past a
  lost file (`event_store.py:103-115`).
- **Pi.** Skips malformed lines. An unterminated last line gets a newline,
  so the next append does not fuse with it (pi#8345). On a failed write the
  in-memory entry is rolled back, so it never becomes a parent (pi#11460,
  `session-manager.ts:1199-1214`).
- **Gemini CLI.** Skips lines that do not parse. A file it cannot read is
  kept as `<file>.unreadable-<ts>` before a rewrite
  (`chatRecordingService.ts:990-1058`).
- **Cline.** Marks a session `stale` when it is marked running and its pid
  is dead (`sdk/packages/core/src/session/services/persistence-service.ts:430-470`).
- **Claude Agent SDK.** `resume` can load a session from an external store
  into a temporary `CLAUDE_CONFIG_DIR` for the CLI. To do so it **copies the
  login credentials** into that folder
  (`_internal/session_resume.py:173-185`).

### 4.2 Orphaned tool calls

A tool call with no result breaks the next request on every provider. It is
the most reported session bug in this survey.

| Harness | Where it repairs | What it inserts |
|---|---|---|
| Codex | When building each prompt, not on disk (`codex-rs/core/src/context_manager/normalize.rs:21`, `history.rs:1234`) | Output `"aborted"`, with an id derived from the call's id through a fixed UUID namespace, "to preserve prompt-cache reuse" (`normalize.rs:18,156-162`); also removes outputs with no call (`normalize.rs:171`) |
| Pi | Request time (`packages/ai/src/api/transform-messages.ts:179-258`) and, since #9986, on disk at abort | `"No result provided"`, `isError: true`; drops aborted or failed assistant messages from the replay |
| rig | `pairing::normalize` keeps entry ids (`crates/rig-core/src/conversation/pairing.rs:260`) | `NO_RESULT_PROVIDED = "No result provided"` (`pairing.rs:184`); drops answers with no call |
| Gemini CLI | When converting a loaded session (`packages/core/src/utils/sessionUtils.ts:279-291`) | "Command execution was interrupted or cancelled before completion. It may have executed partially or completed; check system state before running again." |
| Cline | Request time (`sdk/packages/core/src/session/services/message-builder.ts:55,560-575`) | "Tool execution was interrupted before a result was produced.", `is_error: true` |
| OpenHands | View only (`openhands/sdk/context/view/properties/tool_call_matching.py:15-59`) | Drops unmatched calls and results from the model's view |
| Goose | Before each request: a pipeline of fixes (`crates/oi-kit-provider-types/src/conversation.rs:238-298`) | `fix_tool_calling`, `fix_empty_tool_results`, `fix_lead_trail`, and more, each reporting what it changed |
| Forge | Avoided by construction: the assistant message is stored only with all its results (`crates/forge_domain/src/context.rs:563-610`) | Nothing; the cost is a lost turn |

**Pitfalls (issues).**
- opencode#21326, #21489, #22808, #27594: an interrupted tool call left the
  session stuck for good.
- codex#93, #114, #1579: "No tool output found for function call" after an
  interrupt.
- pi#11227: recovery removed a cut tool call but kept its result. Every later
  request failed.
- OpenHands#7512: prompt trimming orphaned tool calls. OpenHands#18941:
  results matched to calls by position after a restart.
- gemini-cli#30016: cancel rolled back tool rounds that had already changed
  files.
- forgecode#3146, rig#1926: a resend without the provider's opaque reasoning
  field failed. With `store=false` the history must carry them.

### 4.3 Lessons for watane

- **The two backends differ.** For Claude, `claude` repairs its own history
  on `--resume`; the kernel must not. For the OpenAI loop, the kernel owns
  the history and must repair it.
- **Adopt** a pairing pass before every OpenAI request (proposal 10, OpenAI
  loop): each `function_call` gets an output; outputs with no call go;
  pairs are matched by `call_id`, never by position. Port rig's `pairing.rs`
  (MIT) or Codex's `normalize.rs` logic (Apache-2.0).
- **Adopt** persisted synthetic results. At restart (S7 step 1), for each
  call with no result the kernel appends an entry with Gemini's wording:
  "interrupted; may have run partly; check before running again". Pi moved
  to this after #9986. A persisted entry also shows the client what
  happened.
- **Adopt** deterministic ids for anything synthetic (Codex). The history is
  resent each turn, so a changing id breaks the prompt cache.
- **Adopt** keeping the opaque fields: with `store=false`, store each
  reasoning item's `encrypted_content` in the entry and resend it. Hide it
  from clients.
- **Adopt** an `interrupted` entry at restart for a turn that has
  `turn-started` and no `turn-ended`. Never roll back tool rounds that ran
  (gemini-cli#30016).
- **Avoid** the SDK's store-backed resume: it copies the Claude login, which
  G5 forbids.
- **Pitfall to plan for.** Claude deletes its own session files after
  `cleanupPeriodDays` (the SDK's README for session stores says so:
  `examples/session_stores/README.md:164`). Then `--resume=<uuid>` fails
  while the kernel still shows the transcript. **Proposal change (S7, 12):**
  when resume fails with "not found", the kernel marks the session
  `resume-lost`, tells the owner, and offers a new Claude session in the
  same workspace. It does not replay the transcript into a prompt
  silently.

---

## 5. Fork, branch and rewind

**watane needs:** possibly a fork at an entry later; at least ids that make
it possible without a migration.

**Interfaces.**
- **Codex.** `SessionMeta` carries `forked_from_id` and
  `forked_from_ordinal_exclusive` (`codex-rs/protocol/src/protocol.rs:3562-3569`),
  and `history_base`, "an exclusive prefix of another paginated rollout
  inherited by this thread". A fork references the parent's prefix instead
  of copying it. `ThreadStore::prepare_fork` and `revert_thread(before_turn_id)`
  (`codex-rs/thread-store/src/store.rs:399-421`). Rollback is an event:
  `ThreadRolledBack {num_turns}` (`protocol.rs:4161`).
- **OpenCode.** `session.forked {parentID, boundary}` (version 2). Revert is
  three events: `revert.staged`, `revert.cleared` (undo the revert),
  `revert.committed {to}` (`packages/core/src/session/revert.ts`). The
  boundary is a message id; the plan uses `seq > boundary.seq`.
- **Pi.** A tree: each entry has `parentId`; the leaf is the last line. A
  branch moves the leaf; old branches stay. `branch_summary` and `label`
  entries. A fork copies the root-to-leaf path into a new file with
  `parentSession`.
- **Pi durable.** A fork points at one entry; the child sees entries up to
  it. It gets a fresh provider session identity, for the prompt cache
  (`packages/durable/docs/spec.md:1802-1850`).
- **OpenHands.** `parent_id` per event and `leaf_event_id` as HEAD. `fork(from_event_id)`
  checks the branch point first, so a bad id leaves no orphan folder
  (`openhands/sdk/conversation/impl/local_conversation.py:965-1060`).
- **rig.** Fork only at rest: "no tool call is pending and the model owes no
  answer" (`crates/rig-memory/src/fork.rs:17-22`).
- **Claude (the binary).** `--fork-session` with `--resume`;
  `--resume-session-at=<message uuid>` loads only up to that message;
  `--resume-drops-turn=<uuid>` refuses the resume if the cut range holds
  anything else, such as a queued message
  (`claude-agent-sdk-python@69846cd:src/claude_agent_sdk/types.py:2710-2740`,
  `_internal/transport/subprocess_cli.py:657-661`). The SDK's offline
  `fork_session` remaps every uuid and does not copy file-history snapshots
  (`_internal/session_mutations.py:240-254`).
- **Goose, Forge.** Copy the whole conversation under a new id. Goose
  truncates by timestamp or message id
  (`crates/goose/src/session/session_manager.rs:622-640`).
- **LangGraph.** Each checkpoint has `parent_checkpoint_id`; a run from an
  old checkpoint is a fork ("time travel"); `copy_thread`.

**Pitfalls.**
- cline#12388: checkpoints keyed by a derived "run count" drifted. Key a
  fork point to an immutable id.
- opencode#63282: a committed revert left the session pointing at a deleted
  boundary and blocked every send.
- gemini-cli#29530: after `/rewind` the history ended with a model turn;
  the provider refused it. Check the ending rule after every cut.
- pi#11451: settings restored from the wrong branch. Per-branch settings
  must live on the branch.

**Lessons for watane.**
- **Adopt** a monotonic index that is never reused and entries that never
  change. Then a fork point is just `(session, index)`.
- **Adopt** a `claude-uuid` field on entries that come from Claude.
  `stream-json` user and assistant lines carry a `uuid`. Without it the
  kernel cannot map its index to `--resume-session-at`.
- **Adopt** "fork only at rest" (rig) for v1: an index after a
  `turn-ended`.
- **Adopt**, for the OpenAI loop, a fork by reference: the child's head
  names `parent` and `through`; reads below `through` go to the parent
  (Codex `history_base`). It costs no quota. Deleting a parent then needs a
  copy or a refusal.
- **Proposal change (4.1).** The `session` field gains
  `{resume: uuid, at?: claude-uuid, drops-turn?: claude-uuid}`, mapped to
  `--resume-session-at=` and `--resume-drops-turn=`, both `=` form. Probe
  both under `--restricted` first (a V16 in 18).
- **Proposal change (10).** Add `fork(session, at) -> session` to
  `mods:agent/sessions` when the need is shown; no rewind in place. A
  "rewind" is a fork plus a client that switches to it, as OpenCode's
  staged revert can be undone.

---

## 6. File-change checkpoints and undo

**watane needs:** probably nothing in v1. The kernel has no file system;
any undo would be a host effect (proposal G1, 4.4).

**What they do.**
- **Gemini CLI.** A shadow git repo per project: `GIT_DIR=~/.gemini/history/<id>/.git`,
  `GIT_WORK_TREE=<project>`, its own config and copied ignore rules
  (`packages/core/src/services/gitService.ts:136-195`). A commit before each
  file-changing tool. `/restore` runs `git restore --source <hash> .` and
  `git clean -f -d` (`gitService.ts:415-419`). It handles a commit lost to
  git gc.
- **Cline.** `git stash create` in the user's own repo, a private index
  file, refs under `refs/cline/checkpoints/<session>/<run>`
  (`sdk/packages/core/src/hooks/checkpoint-hooks.ts:165-185,624-640`).
  Restore refuses when HEAD moved, using `git update-ref` as a
  compare-and-swap (`sdk/packages/core/src/session/checkpoint-restore.ts:376-478`).
  No checkpoint outside git.
- **OpenCode.** Content-addressed snapshots per assistant step; restore
  takes an explicit map of paths: "paths outside the map are untouched"
  (`packages/core/src/snapshot.ts:62-66`). Revert restores only files that
  later steps touched (`session/revert.ts:88-112`).
- **Forge.** Per-file copies before each edit; undo restores the newest
  copy (`crates/forge_snaps/src/service.rs:22-80`). Not tied to messages.
- **Codex.** Had "ghost commits" for undo; the feature is now removed
  (`codex-rs/features/src/lib.rs:1219`).
- **Claude.** Has its own file checkpoints: an environment variable
  `CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING` and a `rewind_files` control
  request (`_internal/transport/subprocess_cli.py:819-820`, `_internal/query.py:869-879`).

**Pitfalls.**
- cline#14367, #15023: restore with `clean -fd` deleted ignored and
  untracked folders for good.
- cline#13550: restore moved the branch pointer and dropped user commits.
- cline#13131: checkpoints cost 90 s per turn in a large, cloud-synced
  repo.
- opencode#64030: revert restored nothing; the snapshot repo had no commit.
  opencode#63884: edits by background subagents were missed.
- gemini-cli#15786: checkpoints hung off the confirmation path, so
  auto-accepted edits had none.

**Lessons for watane.**
- **Keep it out of v1.** Codex removed its undo. Each design above lost
  user data at least once.
- **If added later**, it is a host effect with a consent, and it follows
  OpenCode: restore only the paths the session wrote; never `clean`; never
  touch the user's git refs or HEAD; bound the snapshot cost.
- **Claude's own checkpoints** would need an environment variable, which the
  allowlisted environment (4.1) forbids, and a control request, which the
  host owns (4.3). Both would be new host rules. Not now.

---

## 7. Replay for tests

**watane needs:** kernel tests that run without a subscription, and restart
tests that are deterministic.

**What they have.**
- **Codex.** `wiremock` serves SSE fixtures (`core_test_support::responses::sse`)
  for resume, fork and compaction suites (`codex-rs/core/tests/suite/{resume,fork_thread,compact_resume_fork}.rs`).
  `InMemoryThreadStore` stands in for the files.
- **OpenCode.** `@opencode/http-recorder` records HTTP and WebSocket traffic
  once, then replays JSON cassettes (`packages/http-recorder/README.md`).
  `session-runner-recorded.test.ts`, `v1-migration.test.ts` on an
  in-memory SQLite.
- **Gemini CLI.** `--record-responses <file>` and `--fake-responses <file>`:
  one JSONL line per model call; "No more mock responses" when the file runs
  out (`packages/core/src/core/fakeContentGenerator.ts:21-110`). 62
  `*.responses` fixtures in `integration-tests/`; `resume_repro.test.ts`.
- **OpenHands.** `TestLLM` plays a scripted list of messages and
  exceptions; a mock OpenAI-compatible server serves the scripts to
  end-to-end tests (`tests/e2e/mock-llm/scripts/mock-llm-server.py`).
- **Goose.** An `OpenAiFixture` server in ACP tests
  (`crates/goose/tests/acp_fork_session_test.rs`); recorded MCP replays
  (`crates/goose/tests/mcp_replays/`).
- **Forge.** Mock completions and tool results from `VecDeque`s, with insta
  snapshots (`crates/forge_app/src/orch_spec/orch_runner.rs:28-41`).
- **rig.** `rig-cassette`: recorded HTTP cassettes, golden replay, and a
  test where "a run interrupted after tool call N resumes in a fresh process
  from the effect log and produces the same output"
  (`crates/rig-cassette/tests/durable_execution.rs`).
- **golem-ai.** `Restart::crash_once` writes a marker inside a durable
  function and then waits for an outside harness to kill the worker;
  `test6` crashes mid-stream, `test12`/`test13` check that a tool is not
  re-run on replay (`test/llm/components-rust/test-llm/src/lib.rs:10-45,716-812`).
- **Storage conformance suites.** rig `check_store`
  (`crates/rig-memory/src/conformance.rs:55`), LangGraph
  `libs/checkpoint-conformance`, pi `registerStorageConformance`.

**Lessons for watane.**
- **Adopt** recorded fixtures per backend: Claude `stream-json` lines for
  the fake `claude` (review 11 already plans one), and Responses SSE
  cassettes for H3. Fail loudly when a fixture runs out (Gemini CLI).
- **Adopt** a crash test that kills the host between two commits, then
  restarts it and checks S7 (golem-ai, rig). It is cheap in watane: a
  failing call is a crash that commits nothing.
- **Adopt** old transcripts as fixtures from the first release on (pi). A
  kernel update test loads them.

---

## 8. Attach and reattach with cursors

**watane needs:** `attach(session, after)` that replays and then goes live
with no gap and no duplicate (proposal 3, 10, S1, S7).

**Interfaces.**
- **OpenCode.** `log({aggregateID, after?, follow?})`: "replays then
  transitions to live. Both modes emit one `Synced` marker at the captured
  replay watermark" (`packages/core/src/bus.ts:160-172`). The live part
  re-reads the log after each wake, so it cannot skip
  (`bus.ts:829-872`). It subscribes before the historical read: "events
  committed during replay either appear in the read or arrive through a
  post-marker wake". `after` accepts only values that came from the log
  (`packages/schema/src/event.ts:17-20`).
- **OpenHands.** `/sockets/session/{id}?after_seq=N`. A `sync{from_seq,
  through_seq}` frame first; `durable{seq, event}` only after the write;
  `transient{event}` with no seq; `item_started{item_id, anchor_seq}`,
  where `item_id` is the id the finished event will have
  (`session_protocol.py:10-120`). A slow client is disconnected, not waited
  for. The client's cursor is "the highest seq with nothing missing below
  it", not the highest seen, and steps over a hole after 1024 newer seqs
  (`OpenHands@3df9648:src/utils/session-seq-cursor.ts:1-63`).
- **Cline.** "Replay-then-live: subscribe first and buffer live events while
  durable pages stream out, then flush the buffer past the last replayed
  sequence; no gap, no duplicates" (`sdk/packages/core/src/hub/server/browser-websocket.ts:392-435`).
  It also dedupes by event id, for events sent before they got a sequence.
- **Pi durable.** The opposite choice: a late client starts from a window
  of the newest 50 entries; "nothing is replayed".
- **LangGraph.** `list(config, before=, limit=)` pages checkpoints
  backwards.

**Pitfalls.**
- OpenHands replaced `after_timestamp` with `after_seq`; timestamps were
  local and naive.
- OpenHands: a status snapshot sent after the replay overrode the newer
  state the replay had built (`src/contexts/conversation-websocket-context.tsx:330-345`).
- OpenHands#16727, #12684: one missing file made the whole conversation fail
  to open.

**Lessons for watane.**
- **Adopt** an explicit end-of-replay item: `synced {through}`. A client
  then knows which items are history and which are live. OpenCode and
  OpenHands both have it.
- **Adopt** an index on every durable item on the stream, and none on a
  delta. A delta carries the index its entry will get (OpenHands
  `anchor_seq`, `item_id`). A client advances its cursor only on durable
  items.
- **Adopt** "read and register in one call". The kernel runs one call at a
  time per call chain; the attach call must read the entries up to
  `through` and register the stream as live in the same commit, so no entry
  can fall between them.
- **Adopt** a paged replay. A replay of thousands of entries cannot go in
  one call (256 items or 4 MiB per stream, 10 s). The kernel pushes a page,
  then the next one on `ready` (the proposal already says "paced by
  `ready`").
- **Adopt**, on attach during a message, a live `item-started {index}` so
  the client shows that text is coming (OpenHands). The proposal (3) says
  deltas of a message being written are not replayed; that stays.
- **Proposal change (10, S1, S7).** The meaning of `after` is unclear. S1
  calls `attach(session, 0)` to get everything; S7 calls
  `attach(session, last index)` and "gets the transcript". Define: indexes
  start at 1; `after` is exclusive; 0 means from the start; the reply
  carries `through`.
- **Proposal change (10).** `read(session, after, max)` returns
  `list<string>`. Return each entry with its index, and `through`, so a
  polling agent (13) can tell "no more" from "max reached".

---

## 9. Session listing and titles

**watane needs:** an owner lists its own sessions, newest first, with a
title, without reading transcripts. The kernel interface (10) has no `list`
yet.

**What they do.**
- **Codex.** An append-only index of session names, rewritten under a lock
  (`codex-rs/rollout/src/session_index.rs:1-2`), and an SQLite `threads`
  table. Pitfalls: listing loaded every session file (codex#22411); listing
  "scales with thread count" and timed out on mobile (codex#45246); an
  incomplete index picked the wrong "last" session (codex#61003).
- **OpenCode.** `session.renamed` is an event; titles are generated
  (`packages/core/src/session/title.ts`).
- **Goose.** `sessions.name` plus `user_set_name`; an LLM name is applied
  only while the user has not set one
  (`crates/goose/src/session/session_manager.rs:640-660`, `session_naming.rs`).
- **Gemini CLI.** An LLM summary, one sentence, at most 80 characters, from
  the last 20 messages, stored as `$set.summary`; fallback: the first user
  message.
- **Pi.** A `session_info` entry holds the name; fallback: the first
  message. Listing streams each file with bounded concurrency.
- **Cline.** Reads only `metadata.title` from the manifest, skipping
  validation on the hot path.
- **Forge.** Title by an LLM in a hook at start; a partial index orders by
  `updated_at` (`crates/forge_app/src/hooks/title_generation.rs:14-41`).
- **OpenHands.** Paginated search with `page_id`; an explicit title
  suppresses the LLM title.

**Pitfalls.**
- One broken row broke the whole list: forgecode#3260, OpenHands#17497.

**Lessons for watane.**
- **Adopt** a per-owner index in pages: `o/<owner>/sessions/<page>`, each
  page a bounded list of `{session, updated, title}`. No prefix scan exists,
  so the index is the only way to list.
- **Adopt** a title in the session head: the first input cut to 80
  characters, until the owner sets one (`user-set`, as Goose). An LLM title
  costs the participant's plan; leave it to clients.
- **Adopt** per-entry decoding: an unreadable session shows as such in the
  list; it never fails the list.
- **Proposal change (10).** Add `list(after, max)` and `rename(session,
  title)` to `mods:agent/sessions`, owner-checked like the rest.

---

## 10. Retention and deletion

**watane needs:** a way to stay under 256 MiB for all transcripts of all
clients, without deleting what the participant wants.

**What they do.**
- **Gemini CLI.** `general.sessionRetention {enabled, maxAge: "30d",
  maxCount, minRetention: "1d"}`, run at start
  (`packages/cli/src/config/settingsSchema.ts:393-435`). It also deletes
  files it cannot parse (`packages/cli/src/utils/sessionCleanup.ts:302-353`).
- **Cline.** No retention for transcripts. Its client event log keeps 7 days,
  200 000 rows or 64 MiB (`sdk/packages/core/src/hub/server/hub-event-log.ts:21-23`).
- **Zed.** Deleting a thread deletes its subagent threads, found through
  `parent_id` (`crates/agent/src/db.rs:743-770`).
- **LangGraph.** `delete_thread`, `delete_for_runs`, `prune(strategy=
  "keep_latest")`; the docs warn that a naive prune breaks `DeltaChannel`
  history silently.
- **Claude.** Sweeps its own files by `cleanupPeriodDays`.
- **Goose, Pi, Forge.** Delete by hand only. Goose has `archived_at`.

**Pitfalls.**
- gemini-cli#27268: 30-day retention switched on by default, without notice.
  Sessions vanished.
- gemini-cli#26094: cleanup ran for the active project only; others grew to
  536 MB.
- opencode#16101: asks for one lifecycle for storage, archive and child
  sessions.

**Lessons for watane.**
- **Adopt** a quota per owner inside the kernel. One client may not fill the
  kernel's 256 MiB and stop every other client. When an owner is over, its
  `start` and `say` fail with a clear error; nothing is deleted.
- **Adopt** explicit deletion only: `delete(session)`, owner-checked, which
  removes the entries, the head, the index row and its child sessions (Zed).
  A large session spans many calls (16 MiB written per call); the head is
  marked `deleting` first, so a restart finishes the job (rig `destroy` is
  idempotent for the same reason).
- **Avoid** silent automatic deletion, and never delete what fails to parse.
- **Note.** Deleting a kernel session does not delete Claude's file under
  `~/.claude/projects`. The host could remove it as an effect; that is a
  host rule to decide, not a kernel one.
- **Proposal change (10, 2.3 "k3").** Add `delete(session)` and the owner
  quota.

---

## 11. Migrating stored sessions across versions

**watane needs:** a kernel update (D5: the kernel is a downloadable mod)
that reads every transcript an older kernel wrote.

**What they do.**
- **Goose.** `CURRENT_SCHEMA_VERSION = 18`; migrations one by one in one
  `BEGIN IMMEDIATE` transaction at startup
  (`crates/goose/src/session/session_manager.rs:32,1408-1440`). A first-run
  race between two processes needed `IF NOT EXISTS` and `INSERT OR IGNORE`
  (`session_manager.rs:1140-1151`). Legacy JSONL files are imported once
  (`session/legacy.rs`). It imports Claude Code, Codex and Pi sessions too
  (`session/import_formats/mod.rs`).
- **OpenCode.** 64 dated migration files, run at startup; v1 JSON files
  migrated into SQLite with tests (`packages/core/test/v1-migration.test.ts`).
- **Forge.** Diesel migrations embedded in the binary.
- **Codex.** SQLite migrations for the index; legacy rollouts are
  rewritten by a background job that renames a new file over the old one
  under a maintenance lock (`codex-rs/rollout/src/maintenance.rs:1-8`). The
  state DB backfill loaded whole rollouts into memory and was OOM-killed
  (codex#56423).
- **Cline.** A one-off reader of the pre-SDK files; every read is
  non-throwing. It had to replay the old truncation, or migrated sessions
  exceeded the context by millions of tokens (cline#12996).
- **Pi, OpenHands.** Upgrade on read; pi rewrites the file once.

**Pitfalls.**
- codex#57321: a resumed legacy thread was missing from the history after a
  model migration.
- goose#13045 and Zed#63864: `SQLITE_BUSY` during writes aborted turns or
  dropped grants. A migration that holds the writer long hurts everyone.

**Lessons for watane.**
- **Avoid** a startup migration of all transcripts. In watane it cannot
  fit: one call has 10 s, 64 MiB and 16 MiB of writes, and it would hold
  the host-wide writer.
- **Adopt** upgrade-on-read (1) as the only migration. If a stored shape
  must change, the kernel rewrites one session's entries lazily, page by
  page, in calls of their own, behind a version in the head.
- **Adopt** a kernel test that loads every fixture transcript of every
  earlier kernel version (7).

---

## 12. Who does it best

"Best" is for watane's needs, not in general. A dash means the harness does
not do it.

| Capability | Codex | OpenCode | Goose | Pi | OpenHands | Gemini CLI | Cline | Zed | rig | golem-ai | Forge | LangGraph |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Format, versioning | good (`serde(other)`) | **best** (version per kind) | fair (DB version) | good (chained) | good (no rewrite) | weak | fair | weak (misreads new) | good (refuses) | – | weak | fair (`v`) |
| Log vs table | good | **best** (event log + projections) | fair (table) | good | good | fair | weak (blob) | weak (blob) | **best** (CAS append) | – | weak (blob) | weak (full copies) |
| Granularity | **best** (persist policy) | **best** (durable/ephemeral) | fair | good | good | weak | fair | fair | good | per chunk | weak | per step |
| Resume, orphans | **best** (stable synthetic ids) | weak (#21326) | good (fix pipeline) | good | good (holes, stand-ins) | good (wording) | good | – | **best** (pairing, idempotent steps) | fair | avoids by losing a turn | fair |
| Fork, rewind | good (prefix by reference) | good (staged revert) | basic (copy) | good (tree) | good (tree + HEAD) | basic (rewind) | fair | – | good (at rest) | – | basic | good (parent ids) |
| File undo | removed | **best** (restore listed paths) | – | – | – | fair (shadow git) | fair (lost data) | – | – | – | basic | – |
| Test replay | good (wiremock SSE) | good (recorder) | good | good (fixtures) | good (TestLLM) | **best** (record/fake flags) | good (VCR) | – | **best** (cassette, crash resume) | good (crash once) | fair | good (conformance) |
| Attach, cursor | – | **best** (`log` + `Synced`) | – | – (window) | **best** (`sync`, cursor rule) | – | good (buffer + dedupe) | – | – | – | – | fair (`before`) |
| Listing, titles | fair (scaling bugs) | good | good (`user_set_name`) | good | good (paged) | good | good | good | – | – | fair | – |
| Retention, delete | – | fair | fair | basic | – | weak (silent) | good (log only) | good (cascade) | good (`destroy`) | – | basic | good (`prune` warnings) |
| Migration | fair (OOM) | good (tested) | good (simple) | good | good | fair | good (tested) | weak | refuses | – | fair | – |

## 13. Test cases watane should write

Kernel (`mods:agent`), with the fake `claude` and recorded SSE:

1. `start` stores the input as entry 1 in the same call; a failed `start`
   leaves no session and no entry.
2. `say` stores the input before the upstream stream is asked for; a
   failing call leaves neither.
3. Deltas write nothing; the finished message is one entry; a 300-delta
   turn costs one writing call per finished entry (V6 budget).
4. `attach(s, 0)` replays all entries in index order, then `synced
   {through}`, then live items; no index is missing or repeated.
5. An entry committed while a replay is being paged appears exactly once.
6. `attach` with a replay larger than one stream window pages on `ready`.
7. `attach` during a message gives `item-started {index}`, then the entry
   with that index.
8. `read(s, after, max)` returns indexed entries and `through`; `max` reached
   and "no more" are told apart.
9. Restart mid-turn (the host killed between two commits): the session is
   idle; an `interrupted` entry follows the last entry; `say` resumes with
   `{resume: uuid}`.
10. Restart with an OpenAI function call stored and no result: a synthetic,
    persisted output with a deterministic id; the next request validates
    (every call has one output, matched by `call_id`).
11. Two restarts give the same synthetic ids (prompt cache).
12. A result with no call is dropped from the request, kept in the
    transcript.
13. `encrypted_content` of reasoning items is resent and never pushed to the
    owner's stream.
14. Claude `--resume` reports the session missing (`cleanupPeriodDays`):
    the session becomes `resume-lost`, nothing is replayed into a prompt.
15. An entry of an unknown kind or newer version reads as `unreadable`,
    keeps its bytes, and does not break `read`, `attach` or `list`.
16. Fixture transcripts from each earlier kernel version load and upgrade on
    read; nothing is rewritten unless asked.
17. `list` is paged, newest first, owner-only; one broken head shows as
    broken, the rest list.
18. Another owner gets `not-yours` from `attach`, `read`, `list`, `rename`,
    `delete`, `fork`.
19. Over the owner's quota, `start` and `say` fail; nothing is deleted.
20. `delete` of a large session spans calls; a restart midway finishes it;
    child sessions go too.
21. A stored tool output above the cap (64 KiB) is cut with a marker; the
    live item was whole.
22. (When `fork` exists) a fork at a non-rest index is refused; a fork at a
    rest index reads the parent's prefix and its own entries in order.

## 14. Reusable code

| What | Path | License | wasm32-wasip2 |
|---|---|---|---|
| Tool-call pairing: `validate`, `repair`, `normalize`, `pending_calls` | `rig@0019e24:crates/rig-core/src/conversation/pairing.rs` | MIT | Likely: `rig-core` with `default-features = false`; CI builds `wasm32-unknown-unknown` only; wasip2 not verified |
| Compare-and-append store contract, torn-tail repair, conformance suite | `rig@0019e24:crates/rig-memory/src/{store.rs,format.rs,conformance.rs}` | MIT | Yes for the logic ("pure data, builds for wasm32"); the store maps onto redb keys |
| Fork at rest (`turn_cut`) | `rig@0019e24:crates/rig-memory/src/fork.rs:17-71` | MIT | Yes (logic) |
| Synthetic outputs with stable ids; orphan output removal | `codex@bb5b9c6:codex-rs/core/src/context_manager/normalize.rs:15-243` | Apache-2.0 | Logic only: port it; it uses Codex's `ResponseItem` types |
| Persist policy (what is durable, output caps) | `codex@bb5b9c6:codex-rs/rollout/src/policy.rs` | Apache-2.0 | Logic only |
| Request fix pipeline (`fix_conversation` and its steps) | `goose@80ce943:crates/oi-kit-provider-types/src/conversation.rs:238-700` | Apache-2.0 | Logic only; port the steps that fit the Responses API |
| Mid-stream crash continuation (`retry_prompt`, replay/live states) | `golem-ai@0923caf:llm/llm/src/durability.rs:26-76,289-418` | Apache-2.0 | Not as is (golem-rust, WASI HTTP 0.3); the logic ports. Not needed while watane restarts whole turns |
| Replay-then-live with a watermark | `opencode@b7f0b90:packages/core/src/bus.ts:829-872` | MIT | TypeScript: design only |
| Session socket protocol, client cursor rule | `software-agent-sdk@main:openhands/agent_server/session_protocol.py`; `OpenHands@3df9648:src/utils/session-seq-cursor.ts` | MIT | Design only; the cursor rule is a few lines for a client page |
| Atomic file write, replay buffer with dedupe | `cline@20b0427:sdk/packages/core/src/session/stores/atomic-file.ts:23-53`; `hub/server/browser-websocket.ts:392-435` | Apache-2.0 | Design only |
| Durable session spec (invariants, forks, restart limits) | `pi-mono@e5d93ea:packages/durable/docs/spec.md` | MIT | Design only |
| Checkpointer conformance tests | `langgraph@58a6e1c:libs/checkpoint-conformance/` | MIT | Python: test design only |
| Thread DB, cascade delete | `zed@main:crates/agent/src/db.rs` | GPL-3.0 | **Do not copy** |

## 15. Proposal changes in one place

| Section | Change |
|---|---|
| 3 | Durable items on a stream carry their index; deltas carry the index they will become; `attach` ends its replay with `synced {through}` |
| 4.1 | `session` gains `at` and `drops-turn` (`--resume-session-at=`, `--resume-drops-turn=`); probe them under `--restricted` (a new V16 in 18) |
| 10 | Entry envelope `{v, kind}` with a version per kind; upgrade on read; unknown becomes `unreadable` |
| 10 | Entries record `claude-uuid` when they come from Claude |
| 10 | Turn markers as entries; the input is stored before the upstream starts; stored tool output capped (64 KiB) |
| 10 | The OpenAI loop runs a pairing pass before each request; synthetic outputs are persisted with deterministic ids; reasoning `encrypted_content` is kept |
| 10 | New operations: `list(after, max)`, `rename`, `delete`; later `fork(session, at)`; `read` returns indexed entries and `through` |
| 10, 2.3 | A quota per owner inside the kernel's 256 MiB; no automatic deletion |
| 12 S1, S7 | Define `after`: indexes from 1, exclusive, 0 is from the start |
| 12 S7 | On restart, an `interrupted` entry and synthetic results; a `resume-lost` state when Claude's file is gone |
| 15 N4 | No harness persists per delta; the evidence supports waiting |

## 16. Not verified

- OpenHands' backend was read at `main` of `software-agent-sdk`, not a
  pinned commit. The older V0 replay code was not in the clone.
- rig's crates were not built for `wasm32-wasip2`.
- `--resume-session-at`, `--resume-drops-turn` and `--session-mirror` were
  read from the Python SDK, not probed on `claude` 2.1.326 under
  `--restricted`.
- `cleanupPeriodDays` was read from the SDK's README for session stores, not
  from Claude's settings documentation.
- Issue searches were rate-limited; the issue lists are samples.
