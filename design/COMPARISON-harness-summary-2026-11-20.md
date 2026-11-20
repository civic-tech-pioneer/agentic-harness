# Agent harnesses: what to take from them (summary, 2026-11-20)

A record, not a contract. It summarises nine comparisons of existing agent
harnesses against `ARCHITECTURE-PROPOSAL.md` (revision 3). The detail, the
citations (repo@commit) and the licences are in each file:

| File | Group |
|---|---|
| `COMPARISON-harness-loop-2026-11-20.md` | turn loop, tool dispatch, retries, usage limits, structured output |
| `COMPARISON-harness-context-2026-11-20.md` | system prompt, project instructions, memory, compaction, caching, reasoning |
| `COMPARISON-harness-delegation-2026-11-20.md` | skills, subagents, handoffs, recipes, worktrees, meta-harnesses, factory as mods |
| `COMPARISON-harness-permissions-2026-11-20.md` | rules, modes, approvals, hooks, judges, trusted folders, bypasses |
| `COMPARISON-harness-sessions-2026-11-20.md` | transcripts, resume, fork, undo, replay, attach, retention, migration |
| `COMPARISON-harness-protocol-2026-11-20.md` | client event protocol (ACP, Codex app-server), provider traits, errors |
| `COMPARISON-rust-harnesses-2026-11-20.md` | catalogue of about 35 Rust harnesses and meta-harnesses |
| `COMPARISON-golem-2026-11-20.md` | Golem platform: durable Wasmtime workers, oplog, agents, RPC, tests |
| `COMPARISON-restate-2026-11-20.md` | Restate: journal, virtual objects, durable promises, timers, AI agent patterns, tests |

Harnesses read: Codex (codex-rs), OpenCode, Goose, Pi, OpenHands, Gemini CLI,
Cline, Zed, Forge, rig, golem-ai, swiftide, AutoAgents, vibe-kanban, lash,
grok-build, mango-external-agents, the Claude Agent SDKs, ACP and its adapters.
Nothing was built or run except three `wasm32-wasip2` checks. Spot-checked
here: Codex `normalize.rs` inserts `"aborted"` outputs; lash has
`poll_effect`/`handle_response`; the Python Claude SDK refuses a CLI without
`hook_on_failure_supported`.

## Nine lessons that recur across groups

1. **Every limit lives in code, and every unknown fails closed.** Each limit
   that sat in a prompt or a per-agent list failed in some harness (subagent
   depth, `max_turns`, `tools:`). Claude hooks fail open on timeout or error;
   Claude's `--max-turns` is unreliable. So the kernel counts turns, tool
   errors and subagents itself; the host sets hooks to block on failure; an
   ask nobody can answer is a deny; an unknown entry kind is `unreadable`,
   never guessed. (delegation, permissions, loop, sessions)
2. **Every tool call gets exactly one result, in call order.** The most
   reported bug across all harnesses: a call without a result makes each
   later request fail. Before each request: pair by `call_id`, add an
   `aborted` result for a missing one, drop a result without a call. Denials,
   bad arguments, unknown tools and cancels all become results. Port Codex
   `core/src/context_manager/normalize.rs` (Apache-2.0) or rig
   `conversation/pairing.rs` (MIT). (loop, sessions, protocol)
3. **The loop is data.** A sans-IO state machine returns the next step
   (call the model, call tools, done) and is saved between calls: lash
   `lash-sansio` (MIT, alpha), rig `AgentRun` (MIT), Goose's derived turn
   counts. That is the shape of "one step per 10 s call". Retries do not
   sleep inside a call; delegation returns an id at once and the result
   later. Copy the design, not the crates (rig's saved format changed twice
   in a month). (loop, delegation, rust)
4. **The OpenAI loop keeps a private wire history beside the ACP
   transcript.** ACP has no field for encrypted reasoning, and with
   `store:false` it must be resent. Replay reasoning only when it carries
   `encrypted_content`, strip item ids, drop another model's reasoning on a
   model switch. Our spike replays raw items with ids, the shape that broke
   Pi. (context, loop, sessions)
5. **Attach needs ids and an end-of-replay marker.** Index every stored
   entry, never deltas; read and register in the same call; send `synced
   {through}` after the replay. ACP's v2 draft adds ids that later updates
   patch, turn state as an update, and a replay end. (sessions, protocol)
6. **Closed vocabularies.** One error type `{kind, retryable,
   user_action}`; a usage limit is a typed outcome with `resets_at`, not a
   retry (parse it from the 429 body and from `response.failed`); refusals
   per model, not per backend, all returned in one list; clients learn when
   an ask is resolved by someone else. (loop, protocol)
7. **Claude Code has more side doors than our probes covered.** A leading
   `/` in a message runs a command or a skill, possibly a denied one. A
   user's plugin can approve a call our hook blocked. A hook `allow` skips
   the prompt, so a judge's "safe" must send no decision, not `allow`. A
   subagent's own `permissionMode` applies under a `default` parent.
   Subagents in argv hit ARG_MAX. Claude deletes old session files, so resume
   can be lost. (context, permissions, delegation, sessions)
8. **Keep reactors; make every repeat harmless.** Golem and Restate both
   replay code from a log; both found it hard (Golem #3773: a long history
   traps an agent on update; Golem v1.6.0 removed code-chosen durability).
   Keep the explicit state machine and commit-on-success, and add keys
   instead: a client key on `start`, `say`, `answer` (a nested call's writes
   commit even when its caller fails, host 6.6, so a retried `start` must not
   make a second session); a job key from (session, step, call id); the step
   recorded before its jobs run; on resume, only tools that `describe` marks
   `idempotent` run again, the rest get `aborted`. Commit durably before any
   external effect, even if N4 makes other commits lazy. (golem, restate)
9. **Waits belong to the host, not to a call.** A host timer
   (`timers.after`/`cancel`) that survives restarts serves retries with
   backoff, ask expiry and schedules; a mod ignores a firing whose id it no
   longer holds. When retries run out the session pauses, not fails. Asks
   bind to their turn: the first answer wins, an answer for an ended turn is
   refused, an interrupt withdraws pending asks. (restate, golem)

## For the human

- **ACP v2 (draft since 2026-07-20).** It fixes lesson 5 and turn state, but
  it is a draft behind `unstable_protocol_v2`. Option: pin one exact schema
  release. (protocol)
- **G4 wording.** Subagents should reach `claude` through `initialize` (or
  `agents/*.md` in the run's plugin folder), not argv; G4 says flags only.
  (delegation P2)
- **N5, the factory's implementer needs a shell.** The in-WASM OpenAI loop
  cannot give one. Options: a `codex app-server` backend in the host, or run
  the implementer on the Claude grant with shell and web. (delegation)
- **watane has no LICENSE file at the root.** Only `deny.toml` (permissive
  dependencies) and two crates (`net`, `log`: MIT OR Apache-2.0) state one.
  The "reusable" verdicts assume a permissive licence. Golem's executor and
  Restate's server are BSL 1.1 (read only); `golem-rust` (Apache-2.0) and
  Restate's SDKs and `sdk-shared-core` (MIT) can be ported. (golem, restate)
- **Project instructions opt-in.** A profile field that makes the host read
  AGENTS.md (workspace root down, 32 KiB total) for both backends; off by
  default as `--restricted` is today. (context)

## Probes (run 2026-11-20)

Each comparison numbered its own new probes from V16; renumbered here.
Scripts: `proto/claude-probes/probes.py` (claude 2.1.326, `sonnet`) and
`proto/agent-spike/chatgpt-probes.py` (`gpt-6-luna`). Full results and their
effect on the design: proposal section 18.

| # | Question | Result |
|---|---|---|
| V16 | Does `/<denied-skill>` run under `--restricted` with a deny rule? | **Yes**, also with `Skill` left out of `--tools`. The host must refuse or escape a leading `/` |
| V17 | `hook_on_failure_supported`; does `onFailure: block` block? | Yes, on an error and on a timeout; unset fails open |
| V18 | User plugins under `--restricted`; plugin hook vs our deny; `--safe-mode` | User plugins do not load; a plugin `allow` does not beat our `deny`; `--safe-mode` drops our skills, so it stays out |
| V19 | User memory under `--restricted`? | Off (user `CLAUDE.md` and auto memory); on without it |
| V20 | `--resume-session-at=`, `--resume-drops-turn=` | Work with `--fork-session`; a wrong uuid is refused |
| V21 | Subagent `permissionMode` under `manual`; depth env var | **`acceptEdits` escalates**, `bypassPermissions` does not; depth 1 stops nesting. Concurrency not probed |
| V22 | `--permission-prompts=host` alone | No asks: keep `--permission-prompt-tool=stdio` |
| V23 | ChatGPT plan tokens: caching and compaction | `prompt_cache_key` and `context_management` compaction work (items round-trip); `compaction_trigger` and `/responses/compact` refused |
| V24 | `updatedInput` vs deny rules | Deny rules are checked again |
| V25 | `dontAsk` with a hook `allow` vs empty | `allow` approves what the mode denies; empty does not |

## Proposal changes, by section

Merged from the nine files; each file has the reasons.

- **3 Streams:** an idle timeout on HTTPS streams (Codex's 300 s was too
  long). Index durable items only; `synced {through}` after replay. Number
  every item; when a `stream-sink.items` call fails, the same numbered batch
  is redelivered and the consumer drops duplicates, then the stream ends
  `consumer-failed`; the cursor moves only after the whole batch commits; a
  second `attach` ends the first with `replaced` (Restate #5375 had this
  race).
- **4.1 Launch document:** close the `subagents` key list and refuse risky
  keys by name; subagents via `initialize`; host sets subagent depth and
  concurrency env vars; refuse `Bash(…)` allow rules whose prefix is an
  interpreter or runner (`bash`, `python`, `npx`, ...); `max-turns` and
  `max-tool-errors`; `session.at` and `drops-turn`; raise the minimum
  `claude` version (symlink fixes, hook `onFailure`).
- **4.2 Protected paths:** Claude's own protected list is the floor, plus
  `.agents`, `.codex`, `.aws`; case-insensitive on macOS.
- **4.3 Control protocol:** refuse or escape a leading `/` in `say`; hook
  `onFailure: block` with a timeout, and kill the process unless `initialize`
  confirms it; init check refuses plugins outside the run folder and a short
  built-in list; an allow with no input sends the original input; changed
  input only for `AskUserQuestion`/`ExitPlanMode`; `control_cancel_request`
  drops the pending ask; a turn ends at idle, not the first `result`.
- **4.4 Folders:** a worktree workspace made by the host (vibe-kanban's
  manager, Apache-2.0).
- **5 HTTPS / 7 Requests:** retries never sleep in a call. The loop file
  proposed a `not-before` on `https-stream.start`; Restate's comparison
  proposes the more general host timer of lesson 9, with Restate's backoff
  shape (`sdk-shared-core` `src/retries.rs`, MIT). Take the timer.
- **6 Agent tools:** `describe` returns `parallel: bool` and `idempotent:
  bool`, both default false; jobs carry a key from (session, step, call id).
- **7 Requests:** a call chain that issues a host request commits durably
  first, whatever N4 decides.
- **8 Consent / 10 Kernel permissions:** order deny, ask, allow; strictest
  match wins; an invalid pattern disables its tool; an unknown tool asks;
  nobody to ask means deny.
- **9 Providers:** the judge's "safe" sends no decision, "unsafe" sends deny;
  model the judge on Codex's Guardian (structured verdict, deadline, at most
  three attempts, unreadable means deny, stop after 3 denials in a row or 10
  of 50); `serverRequest/resolved`-style event when an ask is answered by the
  judge, the expiry or a cancel.
- **10 Kernel:** private wire history for OpenAI; compaction as an appended
  entry (instructions resent, never summarised, never split a call from its
  result; Pi's algorithm via rig-memory, MIT); entries versioned per kind,
  upgraded on read; `list`, `rename`, `delete`, an owner quota, no automatic
  deletion; stored tool output capped at 64 KiB; `say` takes ACP content
  blocks and returns the entry index; `attach` takes client capabilities;
  named `error` cases; profiles name the profiles they may start
  (`delegates`), narrow only, and cancelling a parent cancels its children;
  an adopted offer is bound to its content hash; an OpenAI child gets no
  `task` function. Client keys on `start`, `say`, `answer`; a child
  session's key from (parent session, call id); deleted keys stay
  tombstoned. A turn ends `completed`, `interrupted`, `stopped` or
  `failed`; add `steer` and `interrupt(reason, replacement?)`; `say` while
  busy queues in order (at most 32); interrupt, cancel and kill are three
  levels; `max-turns` stops with a tool-free summary. Transcripts stored in
  chunks (one value is capped at 4 MiB). A token budget per session,
  reserved then committed. The one kernel serialises every session behind
  one call slot: measure it in V8 before building.
- **11 Refuse table:** per model, all refusals in one list, a dry-run check;
  `json-schema` checked by the kernel, a missing Claude `structured_output` is
  a failure; `project-instructions` field (see "For the human").
- **12 Interactions:** S1 and S7 disagree on `attach`'s cursor; `after` is
  exclusive, indexes start at 1. S5: before resending history drop failed
  turns and repair orphaned calls. S6/S7: after cancel or restart emit `idle`
  or `unknown`, never `running`; S7 gets a `resume-lost` state; cancel answers
  every pending ask with deny.
- **13 Agents as clients:** `agent:sessions.wait` so an agent waits for news
  without polling; the host holds the wait and it returns only which topics
  changed.
- **9 Providers (judge):** the host could hold the judge's API key and add it
  at the wire, as it does for OpenAI. Ask expiry per backend: 10 minutes for
  Claude asks (they hold a process), a profile setting for kernel asks.
- **15 N4:** can wait; Codex, OpenCode and OpenHands all persist per finished
  message.
- **8 / dev:** `dev` serves a scripted Responses stream, as it serves a fake
  `claude`.
- **Leave out of v1:** file undo (Codex removed its undo; Cline's restore
  deleted user data).

## What to reuse

| What | Where | Licence | How | wasip2 |
|---|---|---|---|---|
| Tool-call/result repair | codex `core/src/context_manager/normalize.rs`; rig `conversation/pairing.rs` | Apache-2.0; MIT | port | pure Rust |
| Sans-IO turn machine | lash `lash-sansio` `turn_machine.rs`; rig `AgentRun` | MIT | read, spike lash | lash builds (one type checked) |
| Compaction engine | grok-build `xai-grok-compaction`; rig-memory (Pi's algorithm) | Apache-2.0; MIT | port | not checked |
| Claude permission rules + bash splitting | grok-build `xai-grok-permission-rules` | Apache-2.0 | port | not checked |
| Judge breaker and verdict parser | codex Guardian (~220 lines) | Apache-2.0 | port | pure Rust |
| Error and usage-limit parsing; char-safe truncation | codex | Apache-2.0 | port / copy | pure Rust |
| Claude CLI driving: flag detection, env allowlist, per-version fixtures | mango-external-agents | MIT | read, port tests | host side |
| Worktree manager; PreToolUse "ask" pattern | vibe-kanban executors | Apache-2.0 | read / port | host side |
| JSON repair for tool arguments | forge `forge_json_repair` | check file | port | pure Rust |
| MCP types in a mod | `tower-mcp-types` 0.27.0 (replaces rmcp) | MIT/Apache | dependency | builds |
| Codex app-server types (for N5) | `codex-codes` 0.169.8 | check crate | dependency | builds |
| Shared session state (optional) | `ahp-types` 1.1.0 | MIT | dependency | builds |
| Step memo (record a step's result once, reuse on retry) | `golem-rust` `src/durability.rs` | Apache-2.0 | port | not checked |
| Retry policy with backoff | Restate `sdk-shared-core` `src/retries.rs` | MIT | port | not checked |

Read only: Zed's agent crates (GPL-3.0). Do not copy: jcode and
`rig-claude-cloak` (call Anthropic with a subscription OAuth token, against
G5); goose and lash use the Codex CLI's own client id for ChatGPT.

## Test cases

Each file ends with its own list (about 120 in all). The ones that recur:
an orphaned tool call is repaired before the next request; a request extends
the previous one without moving the prefix (cache); a scripted Responses
stream that ends without `response.completed` is an error; a usage limit from
either path yields one typed item with `resets_at`; an unknown stream-json
line passes through as unknown, not an error; attach after a cursor returns
exactly the later entries then `synced`; a hook timeout blocks the tool; a
denied skill cannot run via `/`; a subagent cannot exceed depth, concurrency
or the parent's permissions; recorded `claude` fixtures per CLI version.
From Golem and Restate: count effects, not return values; crash at named
windows and restart for real; a retried `start` with the same key makes one
session; a failed batch is redelivered once with the same numbers; two
answers to one ask, only the first counts; an answer after the turn ended is
refused.

## Gaps

GitHub rate-limited issue searches, so issue lists are samples. OpenHands was
partly read at main, not a pinned commit. Zed was read sparsely. The new
Claude flags and env vars were probed (V16-V25), except the subagent concurrency env var.
