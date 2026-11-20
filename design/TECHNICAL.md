# The agent layer: technical specification (draft)

How the agent layer does what `FUNCTIONAL.md` says. Draft, not yet a
contract (see `VISION.md`). `ARCHITECTURE-PROPOSAL.md` (revision 4) is the
record of why; this file states the rules. Where an open point of the proposal
(section 15, `N<n>`) is undecided, this draft follows the proposal's current
choice and marks it.

## 1. Parts and layout

| Part | Where | Owns |
|---|---|---|
| H1 Streams | host, `host/src/agent/streams.rs` | stream table, FIFO per consumer, numbered batches, backpressure, ends (3) |
| H2 Claude Code | host, `host/src/agent/claude/` (`launch.rs`, `run.rs`, `control.rs`, `init.rs`) | launch parsing, flags, the process, stdin and stdout, `initialize` and `init` checks, pending asks (4) |
| H3 HTTPS streams | host, `host/src/agent/https_stream.rs` | one request, SSE splitting, idle timeout, account binding (5) |
| H4 Agent tools | host, `host/src/agent/tools.rs`, with the MCP broker of host 10 | handles, MCP tokens, keyed jobs (6) |
| H5 Requests | host, the W35 request table | effects applied after a durable commit (7) |
| H7 Timers | host, `host/src/agent/timers.rs`, host state | timers per mod, kept across restarts (7) |
| H6 Consent, accounts | app | consent words and storage, Sign in with ChatGPT, the token file (8, 5) |
| WIT | `host/wit/` in `watane:host@0.1.0` | the interfaces of section 2 |
| pkg | pkg 4.5, 5.1, 7.2 | import and export rows, two grant kinds (15) |
| dev | `dev/` | a fake `claude` and a scripted Responses server for previews and tests (12) |
| M2 Kernel | mod `mods:agent`, its own repository | profiles, offers, sessions, transcripts, broker, OpenAI loop, delegation (9) |
| M1 Providers | two mods `mods:claude-code`, from two publishers | unchecked: start and hand off; checked: judge then forward (10) |

A new source file starts with a `//!` line that cites its section here, as
every watane file does.

## 2. Interfaces

### 2.1 Host (`watane:host@0.1.0`, added)

```wit
interface streams {                                  // no grant
  type stream-id = string;
  open: func() -> result<stream-id, stream-error>;
  push: func(s: stream-id, item: list<u8>) -> result<_, stream-error>;
  close: func(s: stream-id, outcome: outcome);
  cancel: func(s: stream-id);
  hand-off: func(s: stream-id) -> result<_, stream-error>;
}
record item { seq: u64, body: list<u8> }
interface stream-sink { items: func(s: string, items: list<item>); ended: func(s: string, outcome: outcome); }   // host-only export
interface stream-producer { ready: func(s: string); }                                                         // host-only export

interface claude-types { /* launch, decision, session-error, start-error: shared types, no grant */ }
interface claude-code { start: func(launch: string) -> result<stream-id, start-error>; }        // grant: Claude Code (unchecked)
interface claude-code-shell { start: func(launch: string) -> result<stream-id, start-error>; }  // grant: Claude Code with shell and web
interface claude-session {                           // no grant; caller = the stream's consumer
  say: func(s: stream-id, content: string) -> result<_, session-error>;
  answer: func(s: stream-id, ask: string, d: decision) -> result<_, session-error>;
  interrupt: func(s: stream-id) -> result<_, session-error>;
  compact: func(s: stream-id) -> result<_, session-error>;
}
variant decision { allow(option<string>), deny(string), none }

interface https-stream {                             // [http] hosts; an account needs consent
  start: func(request: string) -> result<stream-id, start-error>;
}

interface agent-tools {                              // no grant; a toolset needs consent
  open: func(toolsets: list<string>) -> result<string, tools-error>;
  describe: func(handle: string) -> result<list<tool>, tools-error>;
  call: func(handle: string, key: string, tool: string, args: string) -> result<stream-id, tools-error>;
  close: func(handle: string);
}
record tool { name: string, description: string, input-schema: string, parallel: bool, idempotent: bool }

interface timers {                                   // no grant
  after: func(id: string, delay-ms: u64) -> result<_, timer-error>;
  cancel: func(id: string);
}
interface timer-sink { fired: func(id: string); }    // host-only export

// in the existing interface caller:
identity: func() -> string;   // <publisher>/<slug>, local/<slug>, page, or agent:<handle-name>
```

Every function with an outside effect is a request (7): `claude-code.start`,
`claude-code-shell.start`, `https-stream.start`, `agent-tools.open`,
`agent-tools.call`, `streams.open`, `timers.after`, `timers.cancel`.

### 2.2 Providers (`mods:claude-code@0.1.0`)

```wit
interface sessions {
  record started { stream: stream-id, direct: bool }
  start: func(launch: string, input: string) -> result<started, start-error>;
  say: func(s: stream-id, content: string) -> result<_, session-error>;
  answer: func(s: stream-id, ask: string, d: decision) -> result<_, session-error>;
  interrupt: func(s: stream-id) -> result<_, session-error>;
  compact: func(s: stream-id) -> result<_, session-error>;
}
```

### 2.3 Kernel (`mods:agent@0.1.0`)

```wit
interface sessions {                                 // the owner only, by caller.identity()
  start: func(key: string, profile: string, overlay: string, input: string) -> result<string, error>;
  check: func(profile: string, overlay: string) -> list<refusal>;
  attach: func(session: string, after: u64, caps: string) -> result<stream-id, error>;
  read: func(session: string, after: u64, max: u32) -> result<list<string>, error>;
  say: func(session: string, key: string, input: string) -> result<u64, error>;
  answer: func(session: string, key: string, ask: string, decision: string) -> result<_, error>;
  interrupt: func(session: string, replacement: option<string>) -> result<_, error>;
  cancel: func(session: string);
  list: func(after: option<string>, max: u32) -> result<list<session-info>, error>;
  rename: func(session: string, title: string) -> result<_, error>;
  delete: func(session: string) -> result<_, error>;
}
record session-info { id: string, title: string, profile: string, backend: string, turn: string, updated: u64 }
record refusal { field: string, backend: string, model: string, reason: string }
variant error { not-owner, unknown-session, refused(list<refusal>), queue-full, quota, paused(u64), backend(string) }

interface offers {                                   // a collect (W34)
  skills: func() -> list<offered-skill>;
  profiles: func() -> list<offered-profile>;
}
```

`input`, `overlay`, `decision` and `caps` are JSON: ACP content blocks, an
overlay document (9.2), `{allow | deny, reason?, input?}`, and ACP client
capabilities.

## 3. Streams (H1)

| Rule | Value |
|---|---|
| Consumer of a host stream | the mod whose request opened it |
| Consumer of a mod stream | the caller of the call in which `open` ran; only during a call through a call link from a mod or its own page |
| Page consumer | one `/events` connection, named by the connection id that `watane.js` sends with `/call` |
| `push`, `close` | producer only; `cancel` consumer only; `hand-off` the consumer of a host stream, once, to the caller of the current call |
| Wrong caller, unknown or ended id | `not-yours`, `unknown-stream` |
| Id | 128 random bits, never reused |
| Sequence | each item numbered from 1 per stream |
| Batch | every waiting item, up to 256 items or 1 MiB, one `stream-sink.items` call |
| Failed batch | delivered once more with the same numbers; a second failure ends the stream `consumer-failed` |
| Commit | items pushed in a call are queued when that call commits; a failed call's pushes are dropped |
| Item call's caller | `host` for host streams, `mod:<producer>` for mod streams |
| Hops | an item call that pushes on passes its hop count; at 8 the item is dropped and logged |
| In flight | 256 items or 4 MiB per stream; `push` past it fails `full`; `ready` when below half; host streams stop reading their source |
| Page queue | lossless, per connection, apart from the host 9.3 bus |
| Ends | close, cancel, disable, removal or update of either side, lost grant or consent, closed page connection, `consumer-failed`, `idle-timeout`, app quit; each side gets `ended(outcome)` |
| Restart | streams are memory only; old ids are `unknown-stream` |
| Limits | 32 open streams per consumer; 4 MiB per item (`too-large`); 4 `claude` processes per origin client, 8 in all |

## 4. Claude Code (H2)

### 4.1 The launch document

JSON with `"version": 1`, parsed into a Rust type that refuses unknown
fields. Each value goes out as one `--flag=value` argument.

| Field | Grammar or rule | Goes to `claude` as |
|---|---|---|
| `workspace` | a workspace id the origin client holds (8) | cwd |
| `extra-workspaces` | workspace ids, the same rule | `--add-dir=` each |
| `model` | `[A-Za-z0-9.\-\[\]]{1,64}` | `--model=` |
| `fallback-models` | the same, at most 4 | `--fallback-model=` |
| `effort` | `low`, `medium`, `high`, `xhigh`, `max` | `--effort=` |
| `system-prompt` | `{replace \| append: text}`, at most 64 KiB | `--system-prompt=` / `--append-system-prompt=` |
| `tools` | names from the grant's allowlist (4.2) | `--tools=` |
| `allow`, `deny` | `Name` or `Name(arg)`; Name from the allowlist or `mcp__watane__<tool>`; no comma, space or leading `-` outside the parentheses; an allow `Bash(<prefix>…)` with an interpreter or runner prefix (`bash`, `sh`, `zsh`, `env`, `python*`, `node`, `npx`, `uv run`, `devbox run`, ...) is refused | `--allowedTools=`, `--disallowedTools=` each |
| `mode` | `manual`, `accept-edits`, `plan`, `dont-ask` | `--permission-mode=` |
| `ask` | bool | `--permission-prompts=host --permission-prompt-tool=stdio`, or `--permission-prompts=none` |
| `pre-tool-check` | `{timeout-s: 5..120}` | the host's PreToolUse callback in `initialize` (4.5) |
| `agent-handle` | a handle the origin client holds | `--mcp-config=/dev/fd/3` |
| `subagents` | each exactly `{name [a-z0-9-]{1,64}, description, prompt, tools ⊆ tools, model}`; any other key is refused by name; 64 KiB in all | `--agents=` |
| `subagent-depth` | 0..3, default 1; 0 also removes `Task` | env `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` |
| `skills` | `{name [a-z0-9-]{1,64}, files}`; paths under the skill folder, no `..`, 1 MiB in all; frontmatter keeps `name` and `description` only | the run folder's plugin, `--plugin-dir=` |
| `sandbox` | the typed record of 4.2; required with the shell grant | the host's `--settings=` |
| `json-schema` | at most 64 KiB, valid JSON Schema | `--json-schema=` |
| `max-turns` | 1..1000 | `--max-turns=` |
| `max-budget-usd` | positive | `--max-budget-usd=` |
| `session` | `{new: uuid}`, `{resume: uuid, at?: uuid, drops-turn?: uuid}`, `{fork: uuid, at?: uuid}` | `--session-id=`, `--resume=`, `--fork-session`, `--resume-session-at=`, `--resume-drops-turn=` |
| `persist` | bool | `--no-session-persistence` when false |
| `partial` | bool | `--include-partial-messages` |

### 4.2 Grants and tools

| Grant | Built-in tools | Required |
|---|---|---|
| Claude Code (unchecked), `claude-code` | `Read`, `Write`, `Edit`, `Glob`, `Grep`, `NotebookEdit`, `Skill`, `Task`, `TodoWrite` | a workspace |
| Claude Code with shell and web (unchecked), `claude-code-shell` | the above and `Bash`, `WebFetch`, `WebSearch` | a workspace; `sandbox.enabled: true`; writes only in the workspaces; loopback denied; none of `excludedCommands`, `allowUnsandboxedCommands`, `allowUnixSockets`, `allowLocalBinding`; a network allowlist of plain domain names |

Two grants follow N1 (open).

### 4.3 Protected paths

The host's `--settings=` denies `Edit`, `Write` and `NotebookEdit` on Claude
Code's own protected list and on `.git/**`, `.claude/**`, `.agents/**`,
`.codex/**`, `**/.aws/**`, `**/CLAUDE.md`, `**/AGENTS.md`, `.mcp.json`,
`.envrc`, `**/.github/workflows/**`, `build.rs`, `package.json`, `Makefile`
and `.husky/**`. No launch can remove them.

### 4.4 The run

- **Fixed arguments:** `-p --input-format=stream-json
  --output-format=stream-json --verbose --restricted --strict-mcp-config
  --system-prompt-snapshot=off` and the host's `--settings=`. Never
  `--safe-mode`, `--bare`, `--plugin-url`, `--worktree`, `--bg`, `--cloud`,
  `--remote-control`, `--chrome`, `--ide`, `--file`, `--betas`, `auto` or a
  bypass mode (N2: `auto` stays out).
- **Binary.** The one the participant picked in Settings, at or above the
  minimum version: the first that answers `initialize` with
  `hook_on_failure_supported` and has the protected-path symlink fixes
  (2.1.295 or later).
- **Process.** Its own process group; the pid is recorded in
  `<home>/runs/pids`.
- **Environment.** Only `HOME`, `USER`, `LOGNAME`, `LANG`, `LC_*`, `TMPDIR`,
  `SHELL`, a fixed `PATH`, `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`, and
  `CLAUDE_CODE_OAUTH_TOKEN` when the participant set it. The host never reads
  that token.
- **MCP.** `{"mcpServers":{"watane":{"type":"http","url":…,"headers":{"Authorization":"Bearer <token>"}}}}`
  written to a pipe that the child inherits as fd 3.

### 4.5 The control protocol

1. The host writes `initialize`. With `pre-tool-check` it holds
   `hooks.PreToolUse = [{matcher: "*", hookCallbackIds: [<id>], timeout:
   <timeout-s>, onFailure: "block"}]`.
2. With a callback, the answer must hold `hook_on_failure_supported: true`
   and not `hooks_applied: false`; otherwise the host kills the group and
   ends the stream `mismatch`.
3. The `system/init` line must match the launch in tools, MCP servers,
   agents, permission mode (`manual` reads `default`) and plugins (exactly
   `watane-run` and Claude Code's `cc-plugin-*`). Skills are not compared.
   A difference: kill, `mismatch {field}`.
4. Then each stdout line becomes an item:

| Line | Item |
|---|---|
| `control_request` `can_use_tool` | `{"type":"ask","id":<host id>,"tool":…,"input":…}` |
| `control_request` `hook_callback` (PreToolUse) | `{"type":"check","id":<host id>,"tool":…,"input":…}` |
| `control_cancel_request` | `{"type":"ask-resolved","id":…,"by":"cancelled"}` |
| any other line, known or not | the line unchanged |

5. Writes to stdin are only these:

| Operation | Written | Rule |
|---|---|---|
| `say` | a `user` message with the content | refused `slash-command` when the first text block starts with `/` after white space |
| `compact` | a `user` message `/compact` | – |
| `answer` to an ask | `control_response` `{behavior: allow, updatedInput}` or `{behavior: deny, message}` | `none` refused; `allow(none)` sends the original input; a changed input only for `AskUserQuestion` and `ExitPlanMode` |
| `answer` to a check | `{}` for `none`; `{hookSpecificOutput: {hookEventName: PreToolUse, permissionDecision: deny, permissionDecisionReason}}` for `deny` | `allow` refused |
| `interrupt` | `control_request` `interrupt` | – |

   After any answer, and when an ask expires (10 minutes, deny), the host
   pushes `ask-resolved {id, by}`. A second answer is `unknown-ask`. Nothing
   else is ever written: no `apply_flag_settings`, `mcp_set_servers`,
   `set_permission_mode`, `updatedPermissions` or second `initialize`.
6. A turn is over when `claude` reports itself idle, not at the first
   `result`.

### 4.6 Folders

- **Run folder** `<home>/runs/<random>/`: the plugin folder `watane-run`.
  Deleted when the stream ends; all of `runs/` is deleted at open, after the
  recorded process groups are killed.
- **Scratch workspace** `<home>/scratch/<random>/`, named by the host, owned
  by a session; a mod sees its id.
- **Picked workspace:** canonical, symlinks resolved. Refused: `/`, the home
  folder, the watane home and its ancestors, `~/.claude`, `~/.codex`,
  `~/.ssh`, `~/.aws`, `~/Library/Keychains`, the run and scratch trees;
  compared without case on a case-insensitive file system.

## 5. HTTPS streams and accounts (H3, H6)

- **Request** (JSON): `url`, `method`, `headers`, `body`, `account?`,
  `idle-s?` (default 90, at most 300).
- **Items:** first `{status, headers}`; then one item per server-sent event,
  or per 64 KiB chunk of a body that is not SSE.
- **Refused before a byte leaves:** a URL that is not `https` or not in
  `[http]`; the headers `Authorization`, `Cookie`, `Host`, `Proxy-*`,
  `OpenAI-Organization`, `OpenAI-Project`, `ChatGPT-Account-Id`; a body over
  1 MiB; with `account: "openai"`, anything but `POST
  https://api.openai.com/v1/responses` or `GET https://api.openai.com/v1/models`.
- **No redirects. No retries.** The kernel decides retries (9.5).
- **Idle:** no bytes for `idle-s` ends the stream `idle-timeout`.
- **Accounts.** The app runs Sign in with ChatGPT with watane's own
  registered client: loopback redirect, PKCE, `state` and `nonce`, the ID
  token checked against OpenAI's JWKS, the scopes for plan usage. Tokens in
  `<home>/app/accounts/openai.json` (0600, written atomically, refreshes
  serialised). The host asks the app for a fresh access token per request
  through an in-process trait; without an app, `no-app`.

## 6. Agent tools (H4)

- **Handle:** 128 random bits, owned by (origin client, opening mod); lives
  until `close`, the end of either mod, or a withdrawn consent.
- **MCP token:** one per handle, 32 bytes, memory only (host 10.2), on
  `claude`'s fd 3. A tool's mod sees the caller `agent:<handle-name>`, a
  second random value.
- **Job:** `call` returns a stream at once; the tool runs later as its own
  top-level call with its own budget; its result is the stream's one item:
  `{ok: <json>} | {err: {code, message}}`.
- **Key:** a live job with the same (handle, key) returns the same stream.
  Keys are memory only.
- **`parallel`, `idempotent`:** from the tool mod's annotation (WIT doc
  comment `@parallel`, `@idempotent`, or `mod.toml` `[tools]`), default
  false. They decide order and retries, never permissions.
- **Manager functions** are refused in a handle's toolsets by default
  (F-AGENT-4).

## 7. Requests, durability, timers (H5, H7)

- **Requests.** Each function of 2.1 with an outside effect returns at once
  and is applied only after the whole call chain commits. Outcomes (consent
  refused, no binary, version too old, a refused value) arrive as the
  stream's first item or as `ended`.
- **Durable first.** A chain that issues a request commits with full
  durability before the host applies it, whatever the durability of other
  calls (N4).
- **Timers.** Per mod: ids of at most 128 bytes, at most 256 live, delays up
  to 30 days, stored in host state (`timers/<mod>/<id>` → due time). A due
  timer fires as a top-level call of the mod's `timer-sink.fired(id)`; one
  due during downtime fires soon after start. `after` with a live id
  replaces it.

## 8. Consent (H6)

| Consent | Asked when | Kept for | Words (draft) |
|---|---|---|---|
| Workspace | a launch names a folder new for the origin client | (origin client, folder) | "Let {origin client} have agents work in {folder} ({profile}, through {provider})?" |
| Account | an `https-stream` with `account` new for the origin client | (origin client, account) | "Let {origin client} use your ChatGPT plan?" |
| Toolset | `agent-tools.open` with a toolset new for the origin client | (origin client, toolset) | "Let {origin client}'s agents call: {functions}?" |

- **Origin client:** the page's mod for a page call; the mod of the handle
  for an agent's call; for an item, job or timer, the origin of the call that
  opened it. The host carries it with the chain.
- Stored with the grants (host 4.4); shown and withdrawn in the manager like
  grants. A withdrawal ends dependent streams and closes dependent handles.
- **Grant words:** "Can start Claude Code with your Claude account, to read
  and change files in folders you choose. Not checked by watane." and "Can
  start Claude Code with your Claude account and let it run programs and use
  the web, inside Claude's sandbox. Not checked by watane." The plan of every
  mod whose call links reach either grant adds "Uses Claude Code through
  {provider}."
- **Previews** refuse both grants and accounts.

## 9. The kernel (M2)

### 9.1 State

The kernel's state (host 7: key-value, 4 MiB per value, no prefix scan):

| Key | Value |
|---|---|
| `profile/<name>` | a profile (9.2) |
| `access/<identity>` | the profile names this client may use |
| `adopted/<offer id>` | `{content hash, content}` |
| `key/<owner>/<key>` | the result of the first call with that key; kept after a delete |
| `session/<id>` | the session record (9.3) |
| `owner/<owner>/<n>` | page n of the owner's session ids, 256 per page |
| `entries/<id>/<n>` | chunk n of the transcript: at most 256 entries or 1 MiB |
| `wire/<id>/<n>` | chunk n of the OpenAI wire history |
| `loop/<id>` | the OpenAI loop state (9.5) |
| `quota/<owner>` | the owner's bytes and session count |

### 9.2 Profiles and overlays

A profile holds the launch fields of 4.1 that are not chosen per session
(`workspace`, `extra-workspaces`, `agent-handle` and `session` are), plus
`backend` (`claude` or `openai`), `toolsets`, `delegates` (profile names),
`max-tool-errors`, `max-tokens`, `ask-expiry` and `subagent-limits`.

An overlay holds `system-prompt-append`, `skills+`, `tools-`, `deny+`, and
lower values of the limits. Anything else is refused (`refused`).

Compilation: profile, then adopted offers it names, then the overlay; then the
refuse table (11) for the backend and model. Every refusal is returned at
once; `check` runs only this step.

### 9.3 Sessions

Session record: `id`, `owner`, `key`, `profile`, `backend`, `title`,
`handle`, `workspace`, `claude-uuid`, `parent?`, `children`, `turn`,
`queue` (at most 32 inputs), `counters {turns, tool-errors-in-a-row,
tokens}`, `next-index`, `created`, `updated`.

| Turn state | Meaning |
|---|---|
| `running` | a turn is under way |
| `waiting {asks}` | an ask is open |
| `paused {until}` | a usage limit, or retries ran out; a timer resumes it |
| `idle {stop}` | `completed`, `interrupted`, `stopped {limit}`, `failed {error}` |
| `unknown` | written after a restart for a session that was running |
| `resume-lost` | `claude` refused a resume; the next `say` starts a new Claude session seeded with a summary |

- `start` and `say` and `answer` check `key/<owner>/<key>` first.
- `say` while `running` or `waiting` queues; `queue-full` past 32.
- `interrupt` ends the turn (`idle {interrupted}`), then starts the
  replacement if given. `cancel` ends the session's stream and handle, denies
  open asks, cancels children. `delete` cancels, removes the entries and
  wire history, keeps the keys.
- Quota per owner: 64 sessions and 64 MiB by default (kernel settings).

### 9.4 Entries, transcripts, attach

An entry is JSON:

```json
{"v": 1, "i": 42, "kind": "message", "at": 1763600000000, "uuid": "<claude message uuid>", "body": {}}
```

| Kind | Body |
|---|---|
| `message` | a finished ACP message (user or agent, text and content blocks) |
| `tool` | an ACP tool call with its final status and output (output cut to 64 KiB, marked) |
| `ask` | the ask, and later its decision and who decided |
| `verdict` | a checked provider's verdict and reasons |
| `turn` | a turn state of 9.3 |
| `compaction` | that a compaction happened, and how many tokens before |
| `unknown` | a vendor line or event the adapters do not know, raw |

- Indexes start at 1, increase by one, never change. Deltas are never
  entries.
- Reading an entry of an older `v` upgrades it in memory; an unknown `kind`
  or newer `v` reads as `{"kind": "unreadable", "raw": …}`. Nothing is
  migrated in bulk.
- `attach(session, after, caps)`: in one call, read the entries with
  `i > after`, open the owner's stream and register it as the session's live
  stream; push those entries, then `{"type":"synced","through":<i>}`; then
  live updates. A previous live stream of the session ends `replaced`.
- Live updates are ACP `SessionUpdate`s (v1), each with `_meta.watane.i` when
  it finishes an entry. N6 (ACP v2) is open.

### 9.5 The OpenAI loop

`loop/<id>` holds one state:

| State | On | Next |
|---|---|---|
| `ready` | an input, or all tool results in | repair the wire history, `https-stream.start`; `requesting {stream, attempt}` |
| `requesting` | `response.completed` | store output items; record the step `{n, calls, job keys}`; broker each call; `tools` or `waiting` or (no calls) `idle {completed}` |
| `requesting` | a retryable error | `timers.after(retry/<id>, backoff)`; `retrying {attempt}` |
| `requesting` | a usage limit | `paused {until: resets_at}`; `timers.after(resume/<id>, …)` |
| `requesting` | an error that is not retryable, or the stream ends without `response.completed` | `idle {failed}` |
| `waiting` | every ask decided | start the allowed jobs; denied calls get a denial output; `tools` |
| `tools` | each job item | store the output; when all are in, `ready` |
| any | `interrupt` or `cancel` | cancel streams and jobs; open calls get `aborted`; `idle {interrupted}` |

- **Request body:** `model`, `instructions` (byte-stable per session),
  `input` (the repaired wire history; changing facts in a first user item),
  `tools` (from `describe`, plus `skill`, plus `task` for a session that may
  delegate), `reasoning`, `include: ["reasoning.encrypted_content"]`,
  `store: false`, `stream: true`, `prompt_cache_key: <session id>`,
  `context_management: [{type: compaction, compact_threshold}]`.
- **Repair** before each request: pair calls and outputs by `call_id`; add
  `aborted` for a missing output, drop an output without a call, keep call
  order; drop failed turns; replay reasoning only with `encrypted_content`,
  without ids, and only to the model that made it.
- **Compaction items** returned by the server are stored in the wire history
  and sent back as input; a `compaction` entry marks them. The kernel never
  sends a `compaction_trigger` item or calls `/responses/compact`, which the
  plan refuses.
- **Errors:** `{kind, retryable, user-action}`. Retryable: 5xx, 429 without
  `usage_limit_reached`, connection errors, `idle-timeout`. Backoff: 1 s
  initial, factor 2, at most 60 s, at most 6 attempts, then `paused` with no
  time and the error shown. `usage_limit_reached` is read from the 429 body
  and from `response.failed`.
- **Parallel:** the jobs of a step start together only when every tool is
  `parallel`; else one after another, in call order.
- **After a restart:** a `requesting` state starts again; in `tools`, jobs of
  `idempotent` tools start again under their keys, the others get `aborted`.

### 9.6 The broker

- Rules: deny, then ask, then allow; the strictest match wins. A rule that
  does not parse disables its tool. A tool that no rule names asks.
- Matching for OpenAI: tool names and `Name(arg)` with `*` and `?` globs;
  path rules are refused at compilation.
- An ask goes to the owner when it has an attached stream; otherwise, and for
  an `agent:` owner, it is denied. Kernel asks expire after `ask-expiry`
  (a timer); Claude asks after 10 minutes (the host).
- For Claude, rules went in as flags; the broker sees only asks.

### 9.7 Delegation and counters

- A child session needs its profile in the parent's `delegates`, narrows the
  parent, and counts against `subagent-limits {depth 1..3 (default 1), live
  1..20 (default 4)}`, taken from the session tree in state.
- OpenAI: `task(profile, input)` starts a child and returns its id at once;
  the child's final message arrives later as the call's output. A child's
  request has no `task` tool, and a `task` call from a child is refused.
- Claude: subagents from the launch, depth from `subagent-depth`.
- Counters are kept for both backends from the transcript and the usage of
  each turn: `max-turns`, `max-tool-errors` in a row, `max-tokens`. A limit
  ends the turn `stopped {limit}`; at `max-turns` the last turn runs without
  tools.
- Cancelling a session cancels its children, depth first.

### 9.8 Adapters

| From | To |
|---|---|
| `assistant` text, `stream_event` deltas | `AgentMessageChunk`; a finished message is a `message` entry |
| `assistant` `tool_use` | `ToolCall` (pending) |
| `user` `tool_result` | `ToolCallUpdate` (completed or failed); a `tool` entry |
| `ask`, `check`, `ask-resolved` items | `RequestPermission`-shaped updates under `_meta.watane`, and `ask` entries |
| `system/compact_boundary` | a `compaction` entry |
| `result` | usage into the counters; `idle {completed}` only when `claude` is idle |
| Responses `response.output_text.delta` | `AgentMessageChunk` |
| Responses `function_call` items | `ToolCall`, then `ToolCallUpdate` with the job's output |
| Responses `reasoning` summaries | `AgentThoughtChunk` |
| anything else | an `unknown` entry |

Libraries: `claude-codes` (types only) and `agent-client-protocol-schema`
(v1). An unknown Claude line never fails: the adapter falls back to `unknown`
when `claude-codes` cannot parse it.

## 10. The providers (M1)

- **Unchecked:** `start` calls `claude-code.start`, then `streams.hand-off`,
  and returns `{stream, direct: true}`. Its other functions return
  `not-direct`.
- **Checked:** `start` judges the launch and input, sets `pre-tool-check`,
  calls `claude-code.start`, keeps the stream, opens its own stream to the
  kernel, and returns `{stream: <its own>, direct: false}`. It forwards every
  item unchanged, answers every `check` (`none` when safe, `deny` otherwise),
  judges every `ask` and input, and pushes each verdict as an item
  `{"type":"verdict","on":<ask or check id>,"verdict":…,"reasons":[…]}`.
- **Judge call:** `https-stream.start` to the judge's host in its `[http]`,
  with the tool, its input and a short transcript excerpt; deadline = the
  launch's `timeout-s` minus 2 s; at most 3 attempts inside it; verdict
  `{safe | unsafe | unsure, reasons}`; anything else, a timeout or an error
  is unsafe.
- **Breaker:** 3 denials in a row or 10 of the last 50: `interrupt`, and a
  verdict item that says why.
- **Settings** (judge host, key, thresholds) only from its own page, in its
  state. N10 (the host keeps the key) is open.

## 11. Refuse table

"Refused" is a `refusal {field, backend, model, reason}`; nothing is dropped.

| Field | Claude | OpenAI |
|---|---|---|
| `system-prompt` | flags | `instructions` |
| `model` | `--model=` | a slug in `/v1/models` |
| `fallback-models` | flags | refused unless empty |
| `effort` | `--effort=` | `reasoning.effort` if the model lists that level; else refused |
| `tools` | `--tools=` | refused unless empty |
| `allow`, `deny` | flags | the broker; a path rule refused |
| `mode` | flag | `manual`, `dont-ask`; others refused |
| `ask`, `pre-tool-check` | stdio, hook | the broker |
| `skills` | plugin folder | a `skill` function; a skill with other files refused |
| `subagents`, `subagent-depth` | flags, env | `delegates` and `task`; a subagent with `tools` refused |
| `sandbox`, `workspace` | settings, cwd | refused unless absent |
| `json-schema` | flag; the kernel checks the answer; a missing `structured_output` fails | `text.format`; a schema outside the strict subset refused; the kernel checks the answer |
| `max-turns`, `max-tool-errors`, `max-tokens` | flag and counters | counters |
| `max-budget-usd` | flag | refused |
| `ask-expiry` | refused unless 10 minutes or absent | a timer |
| `persist` | flag | `false` keeps the transcript in memory only |

## 12. Errors

| Code | Where | Meaning |
|---|---|---|
| `not-yours`, `unknown-stream`, `full`, `too-large` | streams | 3 |
| `consumer-failed`, `idle-timeout`, `replaced` | stream outcomes | 3, 5, 9.4 |
| `bad-launch {field, reason}` | claude-code | a value outside its grammar (4.1) |
| `no-binary`, `version {found, need}` | claude-code | 4.4 |
| `mismatch {field}` | claude-code outcome | 4.5 |
| `slash-command`, `unknown-ask`, `not-allowed-here` | claude-session | 4.5 |
| `consent-refused {what}` | any request outcome | 8 |
| `destination`, `forbidden-header`, `too-large`, `no-app` | https-stream | 5 |
| `not-granted`, `unknown-handle`, `unknown-tool` | agent-tools | 6 |
| `too-many`, `bad-id` | timers | 7 |
| `not-owner`, `unknown-session`, `refused`, `queue-full`, `quota`, `paused`, `backend` | kernel | 9 |

Each error also carries a message that names the cause (F-QUAL-3).

## 13. Libraries

| Use | Library | Where |
|---|---|---|
| stream-json types | `claude-codes` (types feature) | kernel, dev's fake `claude` |
| event format | `agent-client-protocol-schema` (v1) | kernel, clients |
| HTTP in a mod | none: `https-stream`; `wstd` only for mods that use `wasi:http` | – |
| SSE parsing | `sse-stream` | host (H3) |
| MCP types in a mod | `tower-mcp-types` | kernel, if it reads MCP shapes |
| JSON Schema check | to choose at build time, must build for wasm32-wasip2 | kernel |

Ported, not depended on (each with its licence noted in the file): the
history repair of Codex `core/src/context_manager/normalize.rs`
(Apache-2.0) and rig `conversation/pairing.rs` (MIT); Codex Guardian's verdict
parser and breaker (Apache-2.0); Restate `sdk-shared-core` `retries.rs`
(MIT); Pi's compaction through rig-memory (MIT), if server compaction is ever
refused. N8 (watane's licence) is open.

## 14. Security properties

| Property | Holds by |
|---|---|
| A mod cannot widen `claude` past its launch | 4.1 grammar and one argument per value; `--restricted`; fixed flags; closed subagent keys; the host owns stdin (4.5); `initialize` and `init` checks |
| A mod cannot run a denied skill by name | `slash-command` (4.5) |
| A failing check cannot let a tool run | `onFailure: block` confirmed in `initialize` (4.5) |
| "Safe" cannot widen the mode | checks answer `none` or `deny`, never `allow` (4.5) |
| Writes cannot plant code that runs later | protected paths (4.3) |
| No mod sees a credential | tokens in the app; MCP tokens on fd 3; handle names, not handles (5, 6) |
| A plan token goes only to OpenAI's agent endpoints | account binding, forbidden headers, no redirects (5) |
| A mod in the middle gains no authority | consent per origin client (8) |
| A session is private | one consumer; owner checks by identity (3, 9.3) |
| A runaway agent stops | counters and delegation limits in the kernel (9.7) |
| A repeated call changes nothing | client keys, job keys, numbered items (9.3, 6, 3) |

Not protected: `claude` itself within its limits; a prompt injection within
what the profile allows; plan usage; another program of the same user
(spec 6); whether a subagent prompt that starts with `/` runs a skill (not
probed).

## 15. Provides and needs

**Provides:** `watane:host` interfaces of 2.1; the kernel's `mods:agent`
interfaces of 2.3; the providers' `mods:claude-code` of 2.2.

**Needs** (proposal N3):
- `spec/VISION.md`: where the agent layer sits in the order of work (before
  the factory, step 6).
- `spec/FUNCTIONAL.md`: F-AGENT-5 to F-AGENT-9 and J11 (`FUNCTIONAL.md`
  section 2). F-AGENT-2 and W6: a token also comes from a toolset consent.
- `spec/TECHNICAL.md`: W2 (consents stored with grants), W5 (previews refuse
  the grants and accounts), and a new W row for the agent layer's boundary.
- `host/TECHNICAL.md`: H1 to H5 and H7; 9.3 (page connection ids and their
  own queues); host state for timers; `caller.identity()`; tool annotations.
- `pkg/TECHNICAL.md`: 4.5 rows for the imports and exports of 2.1; 5.1 and
  7.2 two grant kinds.
- `app/TECHNICAL.md`: Sign in with ChatGPT and the account file; the consent
  words and their storage; the `claude` binary setting.
- `dev/TECHNICAL.md`: the fake `claude` and the scripted Responses server.

## 16. Test plan

Integration tests are modules of one binary per subsystem
(`<sub>/tests/it/<file>.rs`). The kernel and providers are tested with
`watane-dev test` in their own repositories.

| Kind | What |
|---|---|
| Probes | `proto/claude-probes/probes.py` and `proto/agent-spike/chatgpt-probes.py` against the real programs, rerun after each `claude` update and before raising the minimum version |
| Golden fixtures | recorded stream-json per `claude` version; recorded Responses streams; each mapped to entries and updates, compared to a stored result |
| Fakes | dev's fake `claude` (scripted lines, asks, checks, a hook that errors or times out, a crash) and scripted Responses server (checks every request body, answers errors and usage limits) |
| Hostile | one test per boundary of 14, as the attacker it names (the adversary step of a slice) |
| Crash windows | kill the host at named points (after commit, before effect; mid-batch; mid-step) and restart for real; count effects, not return values |

**Cases** (the research's recurring ones first):
1. An orphaned tool call is repaired before the next request; a result
   without a call is dropped.
2. Each request extends the previous one without moving its prefix (cache).
3. A Responses stream that ends without `response.completed` is an error.
4. A usage limit from the 429 body and from `response.failed` gives one
   `paused {until}`.
5. An unknown stream-json line becomes an `unknown` entry; the session goes on.
6. `attach(after)` returns exactly the later entries, then `synced`, then
   live; a second attach ends the first with `replaced`.
7. A failed batch is redelivered once with the same numbers; the consumer
   drops the duplicates.
8. A retried `start` with the same key makes one session; after `delete` the
   key still returns the old result.
9. Two answers to one ask: only the first counts; an answer after the turn
   ended is refused.
10. A hook callback that errors and one that times out both stop the tool.
11. `say("/x")` is refused; `compact` works.
12. A subagent with `permissionMode` is refused; depth 0 removes `Task`.
13. An `init` with an extra plugin, tool or MCP server is a `mismatch`.
14. An `initialize` answer without `hook_on_failure_supported` is a
    `mismatch` when `pre-tool-check` is set.
15. A check answered `allow` is refused; `none` leaves a `dont-ask` mode
    denying.
16. `updatedInput` on a non-question tool is refused.
17. Each launch field: a value outside its grammar is `bad-launch`; a
    flag-like value stays one argument.
18. Each protected path is denied, also through a symlink and in another case.
19. The account binding: every other host, path, method, header and a
    redirect is refused before a byte leaves.
20. A timer due during downtime fires after start; a firing for a cancelled
    id is ignored.
21. Kill after the commit of a request and before its effect: the effect
    happens once after restart.
22. After a restart, an idempotent job runs again under its key; a
    non-idempotent one gets `aborted`.
23. The turn limit leaves one last turn without tools; the tool-error and
    token limits stop the turn.
24. A child beyond `delegates`, depth or live count is refused; cancelling
    the parent cancels the children.
25. A judge that is down, slow or unreadable denies; 3 denials in a row
    interrupt the turn.
26. An overlay that widens anything is `refused`, naming each field.

## 17. Build plan

Each stage works before the next starts.

| Stage | Builds | Done when |
|---|---|---|
| A | H1 streams, H5 durable requests, H7 timers; the V8 three-layer prototype | cases 7, 20, 21 pass; V8 measured and recorded |
| B | H2 with the unchecked provider and a kernel that runs Claude sessions (9.3, 9.4, 9.8); dev's fake `claude` | J11 on the fake; cases 5, 6, 8–18 |
| C | H6 consents and the app's words; the `claude` binary setting; the plan line | J11 on the real `claude`; AGT-J4 |
| D | H3 and accounts; the OpenAI loop (9.5); dev's scripted Responses server | AGT-J2; cases 1–4, 19, 22 |
| E | The checked provider (needs a judge account, V7) | AGT-J3; case 25 |
| F | Delegation and counters (9.7); factory profiles | AGT-J5; cases 23, 24; N5 decided |
