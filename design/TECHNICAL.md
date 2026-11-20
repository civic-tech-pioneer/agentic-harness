# Agentic harness: technical specification

How the agent layer does what `FUNCTIONAL.md` says. It is the last of the
spec chain (`VISION.md`, `FUNCTIONAL.md`, then this); code follows it, and
where this file disagrees with the two above, this file is fixed.
`ARCHITECTURE-PROPOSAL.md` (revision 4) is the record of why; this file
states the rules. The human answered the open points of the proposal
(section 15, `N<n>`, and N11 of `FUNCTIONAL.md`) on 2026-11-20, except N5;
each answer is a rule in the section that owns its fact, cited as
`(N<n>, the human, 2026-11-20)`. N5 (17) stays open.

Sections 1 to 17 are the backend, which comes first. Section 18 is the
console. It uses only the kernel interface of 2.3, and a console slice that
needs more adds it to the backend sections first (17).

## 1. Parts and layout

| Part | Where | Owns |
|---|---|---|
| H1 Streams | host, `host/src/agent/streams.rs` | stream table, FIFO per consumer, numbered batches, backpressure, ends (3) |
| H2 Claude Code | host, `host/src/agent/claude/` (`launch.rs`, `run.rs`, `control.rs`, `init.rs`) | launch parsing, flags, the process, stdin and stdout, `initialize` and `init` checks, pending asks (4) |
| H3 HTTPS streams | host, `host/src/agent/https_stream.rs` | one request, SSE splitting, idle timeout, account binding (5) |
| H4 Agent tools | host, `host/src/agent/tools.rs`, with the MCP broker of host 10 | handles, MCP tokens, keyed jobs (6) |
| H5 Requests | host, the W35 request table | effects applied after a durable commit (7) |
| H7 Timers | host, `host/src/agent/timers.rs`, host state | timers per mod, kept across restarts (7) |
| H6 Consent, accounts | app | consent words and storage, Sign in with ChatGPT, the token file, the judge key (8, 5) |
| WIT | `host/wit/` in `watane:host@0.1.0` | the interfaces of section 2 |
| pkg | pkg 4.5, 5.1, 7.2 | import and export rows, two grant kinds (15) |
| dev | `dev/` | a fake `claude` and a scripted Responses server, which also serves a judge origin (the judge fake, 16), for previews and tests (12) |
| M2 Kernel | mod `mods:agent`, this repository (R1, the human, 2026-11-20) | profiles, offers, sessions, transcripts, broker, OpenAI loop, delegation (9) |
| M1 Providers | two mods `mods:claude-code`, from two publishers, each in its own repository (R1) | unchecked: start and hand off; checked: judge then forward (10) |
| C1 Console | the kernel's page, `ui/`, built from `console/` in this repository | starting sessions, the transcript, the status line, asks, the queue, the session list (18) |

**This repository** is the kernel `mods:agent` and its console (R1, the human,
2026-11-20). Each provider, the unchecked and the checked, lives in its own
repository, not here. Planned layout (the first build slice may rename these;
if it does, it updates sections 1 and 14 in the same commit):

- `mod.toml`, `wit/world.wit` (2.3, the imports of 2.1)
- `src/lib.rs` (exports; owner checks by `caller.identity()`, 9.3)
- `src/profiles.rs` (9.2 compilation, overlays, offers adoption) and
  `src/refuse.rs` (11)
- `src/sessions.rs` (9.3: records, keys, queue, quota) and `src/transcript.rs`
  (9.4: entries, attach, `synced`)
- `src/broker.rs` (9.6, session allows) and `src/delegation.rs` (9.7:
  delegates, limits, counters)
- `src/openai/` (9.5: loop, repair, errors) and `src/adapters/` (9.8)
- `console/` (source) and `ui/` (built from it), 18

A provider repository holds `mods:claude-code` with `src/lib.rs`; the checked
one also `src/judge.rs` (10: judge call, verdict parse, breaker).

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
  resume: func(s: stream-id) -> result<_, stream-error>;   // consumer only: after `defer` (3)
}
record item { seq: u64, body: list<u8> }
enum sink-outcome { taken, defer }
interface stream-sink { items: func(s: string, items: list<item>) -> sink-outcome; ended: func(s: string, outcome: outcome); }   // host-only export
interface stream-producer { ready: func(s: string); ended: func(s: string, outcome: outcome); }               // host-only export

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

interface https-stream {                             // [http] hosts; `openai` needs consent
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
`agent-tools.call`, `streams.open`, `timers.after`, `timers.cancel`, and
`claude-session.say`, `answer`, `interrupt` and `compact` (applied after the
call commits, so a rolled-back call, deferred or failed, sends nothing to
`claude`). `streams.resume` is not a request: it only lets a deferred batch go
on (3); one made in the call that returns `defer` is dropped with it.

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

Every function of `sessions` refuses with `not-owner` a caller whose
`caller.identity()` is not the identity of the kernel `mods:agent`. The checked
provider makes no judge call for a call it refuses (10). `not-owner` and
`not-direct` (10) are cases of the shared `start-error` and `session-error` of
`claude-types` (2.1). The kernel's identity is one fixed published value
`<publisher>/agent`, a constant in each provider's `src/lib.rs`, compared
exactly; no `local/…` identity and no other publisher's `agent` matches.

### 2.3 Kernel (`mods:agent@0.1.0`)

```wit
interface sessions {                                 // the owner only, by caller.identity()
  start: func(key: string, profile: string, overlay: string, input: string) -> result<string, error>;
  check: func(profile: string, overlay: string) -> list<refusal>;
  attach: func(session: string, after: u64, caps: string) -> result<stream-id, error>;
  read: func(session: string, after: u64, before: option<u64>, max: u32) -> result<page, error>;
  say: func(session: string, key: string, input: string) -> result<u64, error>;
  answer: func(session: string, key: string, ask: string, decision: string) -> result<_, error>;
  interrupt: func(session: string, replacement: option<string>) -> result<_, error>;
  cancel: func(session: string);
  list: func(after: option<string>, max: u32) -> result<list<session-info>, error>;
  rename: func(session: string, title: string) -> result<_, error>;
  delete: func(session: string) -> result<_, error>;
  queued: func(session: string) -> result<list<queued-input>, error>;
  unqueue: func(session: string, n: u64) -> result<_, error>;          // only before a turn takes it
  take-back: func(session: string, allow: string) -> result<_, error>; // a session allow (9.6)
}
record page { entries: list<string>, last: u64 }
record session-info { id: string, title: string, profile: string, backend: string, turn: string, updated: u64 }
record queued-input { n: u64, input: string }
record refusal { field: string, backend: string, model: string, reason: string }
variant error { not-owner, unknown-session, refused(list<refusal>), queue-full, quota, paused(u64), backend(string), unknown-input, unknown-allow, behind(u64), too-large(u64), busy }

interface offers {                                   // a collect (W34)
  skills: func() -> list<offered-skill>;
  profiles: func() -> list<offered-profile>;
}
```

`input`, `overlay`, `decision` and `caps` are JSON: ACP content blocks, an
overlay document (9.2), `{allow | deny, for?: "session", reason?, input?}`,
and ACP client capabilities. `allow` in `take-back` is the id of an `allow`
entry (9.4). `too-large(u64)` carries the limit in bytes, always (9.3). `busy`: a Claude session's provider stream is not free (9.5); its message names the cap. `read`: 9.4.

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
| Batch | every waiting item, up to 256 items or 1 MiB but always at least one item, one `stream-sink.items` call |
| Failed batch | delivered once more with the same numbers; a second failure ends the stream `consumer-failed` |
| Deferred batch | an `items` call that returns `defer` is rolled back like a failed call (its pushes and writes are dropped) but is not a failure: it does not count towards `consumer-failed`. The host keeps that batch and every later item, delivers nothing more on that stream, and a host stream stops reading its source, until the consumer calls `resume` |
| Resume | consumer only; the host then delivers the kept batch again, with the same numbers. `resume` of a stream that is not deferred does nothing |
| Commit | items pushed in a call are queued when that call commits; a failed call's pushes are dropped |
| Item call's caller | `host` for host streams, `mod:<producer>` for mod streams |
| Hops | an item call that pushes on passes its hop count; at 8 the item is dropped and logged |
| In flight | 256 items or 4 MiB per stream; `push` past it fails `full`; `ready` when below half; host streams stop reading their source |
| Page queue | lossless, per connection, apart from the host 9.3 bus |
| Ends | close, cancel, `lagging`, `replaced` (9.4), disable, removal or update of either side, lost grant or consent, closed page connection, `consumer-failed`, `idle-timeout`, app quit; each side gets `ended(outcome)` |
| Forwarding | a forwarding mod defers when a `push` to its own downstream stream fails `full`, and resumes in its `stream-producer.ready` for that downstream stream. The checked provider (10) and the kernel (for its owner's live stream, 9.4) both do so. When that downstream stream ends (any outcome: closed, `replaced`, `lagging`, page gone), `stream-producer.ended` runs and the mod resumes its deferred upstream: it resumes every deferred upstream of that stream. The kernel acts on the `ended` of a stream only while it is still the session's registered live stream (a `replaced` one is not), then has no live stream and stores entries without pushing them; the checked provider cancels its `claude` stream |
| Deferral ends | a stream's lag clock starts when a `push` to it fails `full` and stops and resets when the host calls `ready` for it (below half, above); after 10 s on one run of the clock the stream ends `lagging` and both sides get `ended`. So a consumer that stops taking items, or takes a few and stops (a page that stays connected but stops reading), cannot hold its producer, nor the producer's deferred upstream, for longer. A stream whose consumer has deferred it has no lag clock (a running one stops when the consumer defers, and a new one starts only at a failed push after `resume`; its producer's pushes fail `full` because the consumer chose to wait), only this backstop: a stream deferred for 5 minutes without `resume` ends `consumer-failed` (host backstop). Neither needs a timer of the mod: a call that defers is rolled back (7) |
| Restart | streams are memory only; old ids are `unknown-stream` |
| Byte budgets | queued items (in flight and kept by a deferral) count against the host's budget of 512 MiB in all, 128 MiB per origin client (the consumer's 32 streams × 4 MiB), and 16 MiB per page connection (page queues are lossless). A `push` that would pass the origin client's or the host's budget fails `full`, as the in-flight limit does (its producer defers or ends, 3, 9.4). A page connection past its 16 MiB ends; its streams end `lagging`, and the page reattaches after its last index (9.4) and recovers by replay |
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
| `subagents` | each exactly `{name [a-z0-9-]{1,64}, description, prompt, tools ⊆ tools, model}`; any other key is refused by name; 64 KiB in all (N9, the human, 2026-11-20: subagents stay `--agents=` flags) | `--agents=` |
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

There are two grants (N1, the human, 2026-11-20): `claude-code`, file tools in
confined folders, and `claude-code-shell`, shell and web inside Claude's
sandbox.

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
  bypass mode (N2, the human, 2026-11-20: `auto` stays out).
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
   ends the stream `mismatch`. Once this step has passed, the host pushes the
   item `{"type":"initialized"}` on the `claude-code.start` (and
   `claude-code-shell.start`) stream. The host starts the process only after
   the workspace consent is held (8), so `initialized` never comes before it;
   the checked provider relies on that (10).
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
  by a session; a mod sees its id. It asks no consent (8): no participant folder is in it.
- **Not yet specified:** which call creates a scratch workspace, how a picked
  folder becomes a workspace id, and how the kernel holds one across a resume.
  Stage B's first slice (17) adds this to 2.1, 4.1, 4.6 and a 15 row before
  any build slice of that stage starts.
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
- **Judge account** (N10, the human, 2026-11-20). The host keeps the judge's
  key as an account `judge`, as it keeps the OpenAI tokens. The participant
  enters the key and the judge's origin (`https://host[:port]`) in the app,
  on the page the manager shows for that checked provider, never in the
  provider's own page. The app stores it in
  `<home>/app/accounts/judge/<provider mod id>.json` (0600, written
  atomically).
  - **Identity.** `<provider mod id>` is the provider's `caller.identity()`
    (`<publisher>/<slug>`), compared exactly. The unchecked provider, another
    publisher's mod or a preview gets `no-account`.
  - **Value.** `account` is exactly `openai` or `judge`; any other value is
    refused `destination`.
  - **Binding.** The key is looked up by the caller's identity, so a caller
    with no key of its own gets `no-account` (Identity). A request with
    `account: "judge"` is refused before a byte leaves (`destination`) unless
    the caller is the provider the key was entered for, the method is `POST`, and the URL's origin is exactly the
    account's origin and is in the provider's `[http]`. The origin is compared
    after one parse, used for both the check and the connection: scheme
    `https`, no userinfo, host lower-cased without a trailing dot, the port
    with 443 as default; the same parse decides `[http]`. The host adds
    `Authorization: Bearer <key>`; no redirects. Without an entered key:
    `no-account`.
  - **Lifetime.** The manager page of the provider shows that a key is kept
    (never its value) and removes it. Removing it ends the provider's live
    judge streams. Uninstalling the provider deletes its file; a later mod
    with the same identity starts with no key.
  - **Entry.** The app never prefills the judge origin from the provider's
    state or plan, and shows the origin next to the key before saving.
  - **Scope.** The account is bound to the judge's origin (the human's
    answer, N10), not to one path.
  - Entering the key is the participant's consent for that provider; there is
    no per-origin-client consent for this account (8).

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
  calls (N4, the human, 2026-11-20: every commit keeps the host's full
  durability, one fsync per write; the proposal's per-batch option is not
  built). V8 (stage A, 16 "Probes named here") measures the cost and 16 "V8
  record" keeps it; whether to change this is the human's then.
- **Timers.** Per mod: ids of at most 128 bytes, at most 256 live, delays up
  to 30 days, stored in host state (`timers/<mod>/<id>` → due time). A due
  timer fires as a top-level call of the mod's `timer-sink.fired(id)`; one
  due during downtime fires soon after start. `after` with a live id
  replaces it.
- **Kernel timers.** The kernel holds at most one host timer per session,
  `s/<session id>`, due at the session's earliest deadline (9.3, 9.5, 9.6).
  Live session timers are at most 240 kernel-wide, 16 of the host's 256 being
  kept for the kernel's own use; what happens at the cap is in 9.3.
- **Rolled back.** `claude-session.say`, `answer`, `interrupt` and `compact`
  are requests too: a call that is rolled back (deferred or failed, 3) sends
  nothing to `claude`, and its `timers.after` and `timers.cancel` are dropped
  with it and made again on redelivery.

## 8. Consent (H6)

| Consent | Asked when | Kept for | Words |
|---|---|---|---|
| Workspace | a launch names a folder new for the origin client | (origin client, folder) | "Let {origin client} have agents work in {folder} ({profile}, through {provider})?" |
| Account | an `https-stream` with `account: "openai"` new for the origin client (the `judge` account asks none: entering its key is the consent, 5) | (origin client, account) | "Let {origin client} use your ChatGPT plan?" |
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
- **While a consent waits,** a request's requester gets `consent-pending
  {what}` once (`what` as in `consent-refused {what}`, 12); the kernel shows
  it as the activity `consent {what}` (9.3).
- **Previews** refuse both grants and accounts.

## 9. The kernel (M2)

### 9.1 State

The kernel's state (host 7: key-value, 4 MiB per value, no prefix scan):

| Key | Value |
|---|---|
| `profile/<name>` | a profile (9.2) |
| `access/<identity>` | the profile names this client may use |
| `adopted/<offer id>` | `{content hash, content}` |
| `key/<owner>/<key>` | the result of the first call with that key that changed something; kept after a delete. A call that fails and changes nothing (any error of 2.3: `busy`, `queue-full`, `quota`, `too-large`, `refused`, `paused`, ...) is not kept, so a retry with the same key can succeed |
| `session/<id>` | the session record (9.3) |
| `owner/<owner>/<n>` | page n of the owner's session ids, 256 per page |
| `queue/<id>` | the session's queued inputs (9.3), apart from the record |
| `entries/<id>/<n>` | chunk n of the transcript: at most 256 entries and at most 64 KiB, or exactly one entry when that entry is over 64 KiB (an entry is at most 256 KiB, 9.4); an append that would take the last chunk past 64 KiB starts a new chunk, so an append rewrites at most 64 KiB plus the entry and earlier chunks are never rewritten; the session record's `chunks` lists the first index of each, so a read starts at the right chunk without reading the ones before |
| `wire/<id>/<n>` | chunk n of the OpenAI wire history; the loop state's `wire-chunks` (in `loop/<id>`) lists the first wire index of each, so a request finds the chunk holding `active` (9.5) and reads none before it |
| `loop/<id>` | the OpenAI loop state (9.5), with `active` and `wire-chunks` |
| `quota/<owner>` | the owner's bytes and session count; the bytes count every key the owner causes (9.3) |
| `quota` | the kernel-wide account: bytes and sessions of all owners (9.3) |
| `stream/<id>` | `{session, kind}` (`kind`: `provider`, `request`, `job`, `owner`), written in the call that opens or receives the stream, deleted on its `ended` (9.4) |
| `live/<session>` | the id of the session's live owner stream: written by `attach`, deleted when it ends (9.4) |
| `slots` | `{provider, request, job: counts; timers; head, tail}`: the stream slots in use, the live session timers (9.3) and the ends of the waiting FIFO (9.5) |
| `slots/wait/<n>` | a session id waiting for a slot (9.5) |

### 9.2 Profiles and overlays

A profile holds the launch fields of 4.1 that are not chosen per session
(`workspace`, `extra-workspaces`, `agent-handle` and `session` are), plus
`backend` (`claude` or `openai`), `toolsets`, `delegates` (profile names),
`max-tool-errors`, `max-tokens`, `ask-expiry` and `subagent-limits`.

An overlay holds `system-prompt-append`, `skills+`, `tools-`, `deny+`, and
lower values of the limits. Anything else is refused (`refused`).

No profile field reads workspace instructions (`AGENTS.md`, `CLAUDE.md`)
(N7, the human, 2026-11-20); an opt-in profile field is later
(`FUNCTIONAL.md` section 5).

Compilation: profile, then adopted offers it names, then the overlay; then the
refuse table (11) for the backend and model. Every refusal is returned at
once; `check` runs only this step.

### 9.3 Sessions

Session record: `id`, `owner`, `key`, `profile`, `backend`, `title`,
`handle`, `workspace`, `claude-uuid`, `parent?`, `children`, `turn`,
`allows`
(session allows, 9.6), `counters {turns, tool-errors-in-a-row, tokens}`,
`chunks` (the first index of each entry chunk, 9.1), `streams` (the ids of the session's open streams, at most 10: 1 provider or 1 request, 8 jobs, 1 owner; 9.4, 9.5), `deadlines` (each `{what, at}`, at most 64; below), `next-index`, `created`, `updated`. The queue is not in the record: it lives
in `queue/<id>` (9.1), at most 32 inputs, each numbered from 1 per session.

**Sizes.** Every input (`start`, `say`, an `interrupt` replacement) is at
most 256 KiB serialized, else `too-large(262144)`. The queue holds at most 32
inputs and 2 MiB in all; past either, `queue-full`. So the `queue` update and
`queued` carry at most 2 MiB of inputs, under the 4 MiB item (3). The record is at most 512 KiB; its variable fields:

| Field | Cap | Past it |
|---|---|---|
| `key` (a client key, also in `key/<owner>/<key>`) | 256 bytes | `too-large(256)` |
| `title` | 256 bytes | `rename` refuses `too-large(256)` |
| `children` | live children only, at most 20 (9.7); a finished child leaves the list, its id stays in the parent's transcript | – |
| `allows` | at most 64 session allows and 64 KiB in all, each at most 1 KiB | an answer `for: session` is refused `too-large(65536)`; the ask stays open |
| `chunks` | 16 384 chunk starts, 8 bytes each (u64), so 128 KiB of the 512 KiB record | the session is at its quota: the turn ends `stopped {limit: quota}` and new input is refused `quota` |
| `streams` | 10 ids | – (the slots of 9.5 allow no more) |
| `deadlines` | 64 | – (an open ask past the 61st is denied `timers`, below) |

| Turn state | Meaning |
|---|---|
| `running {activity?}` | a turn is under way; `activity` while it lasts: `checking {on}` (the checked provider asks its judge, 10), `compacting`, `consent {what}` (a request waits for the participant in the app, 8). The activity is live only: pushed as a `turn` update, never stored in the record or as an entry, so it costs no write |
| `waiting {asks}` | an ask is open |
| `paused {until}` | a usage limit, or retries ran out; the `resume` deadline resumes it (below) |
| `idle {stop}` | `completed`, `interrupted`, `stopped {limit}` (names the limit), `failed {error}` (`failed {timers}` when a retry finds no timer slot, below) |
| `unknown` | written after a restart for a session that was running |
| `resume-lost` | `claude` refused a resume; the next `say` starts a new Claude session seeded with a summary |

- `start` and `say` and `answer` check `key/<owner>/<key>` first.
- `say` while `running` or `waiting` queues; `queue-full` past 32 inputs or 2 MiB. `queued`
  lists the queue; `unqueue` removes one input not yet taken, and refuses
  (`unknown-input`) one that is gone. Each change pushes `{"type":"queue",
  "inputs":[…]}` under `_meta.watane`.
- `interrupt` ends the turn (`idle {interrupted}`), then starts the
  replacement if given. `cancel` ends the session's stream and handle, denies
  open asks, cancels children. `delete` cancels, removes the entries,
  the queue (`queue/<id>`) and wire history, keeps the keys.
- **Deadlines and the session timer.** The kernel holds one host timer per
  session, `s/<session id>` (7), due at the session's earliest deadline. A
  deadline is `{what, at}`, `what` one of `ask <ask id>` (9.6, `ask-expiry`),
  `retry` and `resume` (9.5), `idle` (9.5, the 10-minute release); at most 64
  (one per open ask plus the three others: a 62nd open ask with a deadline is
  denied `timers`). When the timer fires, the kernel handles every due
  deadline and sets the timer for the next one, or cancels it; a session
  whose deadlines are all gone frees its timer. Live session timers are at
  most 240 kernel-wide (the host allows 256 per mod, 7; 16 are kept for the
  kernel's own use); the count is in `slots` (9.1) beside the stream counts
  and is set to the live count at restart (timers are kept across restarts, 7).
  A session that needs a timer at the cap (and at the host's `too-many`, if it
  ever comes) gets, by deadline: an ask, denied at once, reason `timers`; a
  retry, the turn ends `idle {failed {timers}}`; a usage limit, `paused` with
  no time (the next `say` resumes it, as after retries ran out); an idle
  release, the provider stream released at once. Nothing waits for a timer
  slot. The checked provider's judge timers (10) are the provider's own 256,
  for at most 8 sessions' streams: unchanged.
- **Quota.** An owner's bytes (`quota/<owner>`) count every key the owner
  causes: session records, `queue/`, entries, wire chunks, loop state,
  `stream/`, `live/` and its kept `key/<owner>/<key>` results (kept after a
  delete, so they stay charged). The kernel also keeps one account for all
  owners in `quota` (9.1): bytes and sessions. The kernel's state is at most
  256 MiB (host 7); 32 MiB of it is reserved for control records (session
  records, turn states, `slots`, `quota`, `stream/`, `live/` and the entries
  that record a stop), so owners together hold at most 224 MiB of the rest.
  Past an owner's 64 sessions or 64 MiB (kernel settings by default), or past
  the kernel's 224 MiB: new sessions and inputs are refused `quota`, and a
  running turn ends `stopped {limit: quota}`, written from the reserve. Only a
  `delete` frees bytes (kept keys stay). The 64 MiB per owner stays a kernel
  setting; with the 224 MiB account it can be set higher without one owner
  taking the reserve.

### 9.4 Entries, transcripts, attach

An entry is JSON:

```json
{"v": 1, "i": 42, "kind": "message", "at": 1763600000000, "uuid": "<claude message uuid>", "body": {}}
```

| Kind | Body |
|---|---|
| `message` | a finished ACP message (user or agent, text and content blocks) |
| `tool` | an ACP tool call with its final status and output (output cut to 64 KiB, marked) |
| `ask` | the ask: its tool call id, the full tool input, `expires-at`; later its decision, the reason given, and who decided |
| `allow` | a session allow given or taken back: its reach (9.6) and who |
| `verdict` | a checked provider's verdict and reasons |
| `turn` | a turn state of 9.3 |
| `compaction` | that a compaction happened, and how many tokens before |
| `unknown` | a vendor line or event the adapters do not know, raw |
| `part` | one slice of an entry over 256 KiB, as below |

- Indexes start at 1, increase by one, never change. Deltas are never
  entries.
- **An entry is at most 256 KiB serialized.** A larger one is stored, read
  and pushed as consecutive `part` entries:
  `{"v":1,"i":<k>,"kind":"part","at":…,"body":{"n":<1..m>,"m":<m>,"text":"<slice>"}}`.
  `text` is a slice of the whole entry's serialization, cut at UTF-8
  character boundaries; the whole entry carries the index of its last part.
  A reader joins the `text` of parts 1..m in order and parses the result as
  one entry. A page (`read` either way, a replay) may end inside a group, and
  the client joins the parts across pages. `attach` and `read` with an
  `after` that falls inside a group start at the group's first part. Live, a finished entry over 256 KiB goes
  out as its parts, as replay pushes entries, never as one update. So every
  chunk (64 KiB, or one entry), `read` page (2 MiB) and replay (2 MiB) holds at least one
  whole entry or part, and paging always makes progress. No content is cut by
  this rule (tool output stays cut at 64 KiB, as before).
- Reading an entry of an older `v` upgrades it in memory; an unknown `kind`
  or newer `v` reads as `{"kind": "unreadable", "raw": …}`. Nothing is
  migrated in bulk.
- `attach(session, after, caps)`: in one call, read the entries with
  `i > after`, open the owner's stream and register it as the session's live
  stream; push those entries, then `{"type":"synced","through":<i>}`; then
  live updates. A previous live stream of the session ends `replaced`.
- **Replay fits the stream window.** `attach` replays at most 128 entries and
  2 MiB, half the in-flight limit of 3. When more entries lie after `after`,
  it opens nothing and returns `behind {through}`, `through` being the session's high-water index (`last` of `read`); the client pages with
  `read` first, then attaches again. `read` returns at most `max` (at most
  256) entries and at most 2 MiB, so every replay call stays within its
  budget, whatever the length of the transcript.
- **`read`** returns a `page {entries, last}`. Without `before`: the oldest
  entries with `i > after`. With `before = b`: the newest entries with
  `after < i < b`. Both give them in ascending order, at most `max` (at most
  256) and 2 MiB. `last` is the session's highest stored index when the call
  ran (the high-water mark), so a client paging backwards knows where the
  live end is.
- **A full owner stream.** The kernel pushes to the owner's live stream in
  item calls and defers when a push fails `full` (3); it resumes in
  `stream-producer.ready`. Outside an item call (a `say`, an `answer`, a
  timer) it cannot defer: if a push to the owner's live stream fails `full`,
  it closes that stream with outcome `lagging`, and the client attaches again
  after its last index. A deferral of a session's upstream lasts at most 10 s:
  the host ends an owner stream whose lag clock (from the failed push to
  `ready`) reaches 10 s `lagging` (3, "Deferral ends"); the deferred upstream
  has no lag clock, so it is the owner stream that ends, never the session, so a page that stays connected but stops reading cannot
  freeze a session. When the owner's stream ends mid-deferral (3,
  `stream-producer.ended`), the kernel resumes every deferred upstream of the
  session and stores entries without pushing them.
- **A stream finds its session without the record.** Keys `stream/<id>` and
  `live/<session>` (9.1) map a stream to its session; the record's `streams`
  lists the open ones (9.3). An item call reads `stream/<id>` first. A call
  that only forwards a delta (no finished entry) reads `stream/<id>` and
  `live/<session>` only: no session record, no transcript chunk, no wire
  chunk, and it writes nothing. The restart pass that writes `unknown` (9.3)
  deletes the `stream/` keys of the session's `streams` and `live/<session>`
  (streams are memory only, 3).
- **Chunk reads.** A `read` page (2 MiB) reads at most 33 chunks of 64 KiB
  plus one, so it stays bounded. Appending an entry rewrites only the last
  chunk (9.1).
- **No partial replay.** A message still being written is not replayed: its
  text so far lives only in the deltas already sent, and keeping it would cost
  a state write per delta (V6). A client that attaches mid-message drops that
  message's later deltas and shows it whole when its entry arrives.
- Live updates are ACP `SessionUpdate`s (v1), each with `_meta.watane.i` when
  it finishes an entry. The wire is ACP v1; the kernel adds its own entry ids
  (`_meta.watane.i`), turn state (`turn`) and replay end (`synced`); ACP v2 is
  not used (N6, the human, 2026-11-20).
- Besides the entries, these go out live under `_meta.watane` and are not
  stored: `{"type":"turn"}` with every change of the turn state (9.3),
  `{"type":"queue"}` (9.3), and `{"type":"usage","context-used",
  "context-window","compact-at"?}` after each step (9.7).

### 9.5 The OpenAI loop

`loop/<id>` holds one state:

| State | On | Next |
|---|---|---|
| `ready` | an input, or all tool results in | repair the wire history, `https-stream.start`; `requesting {stream, attempt}` |
| `requesting` | `response.completed` | store output items; record the step `{n, calls, job keys}`; broker each call; `tools` or `waiting` or (no calls) `idle {completed}` |
| `requesting` | a retryable error | set the `retry` deadline to the backoff (9.3); `retrying {attempt}` |
| `requesting` | a usage limit | `paused {until: resets_at}`; set the `resume` deadline (9.3) |
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
- **Body budget.** ChatGPT sign-in refuses bodies over about 1 MiB, and the
  host refuses them (5), while compaction happens on the server by token
  count, after a request is accepted. So the kernel sets `compact_threshold`
  to at most 200 000 tokens (about 800 KB of text), and measures each body
  before sending. Over 896 KiB, it sets a trim mark at the first item of the
  last 4 steps and trims every item before the mark (`encrypted_content`
  reasoning left out, oldest first, then tool outputs cut to 8 KiB, marked)
  until the body is at most 768 KiB. The mark, and how far trimming went
  (the last item trimmed and the step reached: reasoning or tool outputs), are
  kept in the loop state; the mark
  does not move until a body is over 896 KiB again; every later request sends
  the items before it trimmed the same way, so it extends the trimmed request.
  Still over 1 MiB after trimming everything before the mark: the turn ends
  `failed {too-large}` and the next `say` continues as for `resume-lost`,
  from a summary. Trimming starts a new prefix (cache), once. `instructions`
  stay byte-stable.
- **Active history.** A compaction item returned by the server replaces every
  wire item before it. The loop state keeps `active`: the wire index of the
  newest compaction item, or of the summary that a `resume-lost` or `failed
  {too-large}` restart starts from; 0 before any. Repair and the request body
  read and send only the items from `active` on; the trim mark of the body
  budget lies at or after `active`. `loop/<id>` keeps `wire-chunks` (the first
  wire index of each `wire/<id>/<n>` chunk, as the record's `chunks` does for
  entries, 9.1), so a request reads no chunk before the one holding `active`.
  Chunks wholly before `active` are deleted in the step that moves it: they
  hold nothing a request sends, and the transcript entries keep what the
  participant sees. So a step's bytes read do not grow with the session's
  whole history.
- **Compaction items** returned by the server are stored in the wire history
  and sent back as input; a `compaction` entry marks them. A compaction and a
  summary restart each start a new prefix, once; `instructions` stay
  byte-stable. The kernel never
  sends a `compaction_trigger` item or calls `/responses/compact`, which the
  plan refuses.
- **Errors:** `{kind, retryable, user-action}`. Retryable: 5xx, 429 without
  `usage_limit_reached`, connection errors, `idle-timeout`. Backoff: 1 s
  initial, factor 2, at most 60 s, at most 6 attempts, then `paused` with no
  time and the error shown. `usage_limit_reached` is read from the 429 body
  and from `response.failed`.
- **Slots.** The kernel consumes at most 32 streams (3). It splits them: 8
  provider streams (Claude sessions; the host's limit of 8 `claude`), 8 model
  requests (`https-stream.start` of this loop), 16 tool jobs; one session
  holds at most 8 job slots, and one owner at most half of each kind (4
  provider, 4 request, 8 job slots), on top of the per-session cap. A `start` or a `say` that needs a Claude session's provider stream when its owner holds 4, or all 8 are held, is refused `busy` (2.3, 12); it never waits. A Claude session idle for 10 minutes (the `idle` deadline, 9.3) releases its provider stream (the kernel cancels it; the next `say` resumes the Claude session with `session {resume}`, 4.1; with `persist: false` there is nothing to resume, and that `say` continues as for `resume-lost`), so slots do not stay held by idle sessions. A `task` child (9.7) refused `busy` gets the call output `{err: {code: busy}}`. A child session counts against its root's owner. Owners that are different mods acting together can still take every slot: an accepted limit (availability, not a boundary of 14). State: `slots` and `slots/wait/<n>` (9.1). The
  loop state keeps a step's calls not yet started (`pending`); they start in
  waves as job slots free. A request that finds no request slot waits in
  `ready` in the FIFO. Every `ended` of a kernel-consumed stream frees its
  slot and starts the oldest waiting work whose session and owner are under
  their caps in the same call; work at a cap stays in the FIFO and never
  blocks another owner's. A FIFO entry whose session no longer waits
  (interrupt, cancel, delete, restart) is skipped and removed when reached. The restart pass
  that writes `unknown` (9.3) sets the counts to zero (streams are memory
  only, 3).
- **Parallel:** the jobs of a step start together, up to the free job slots,
  only when every tool is `parallel`; else one after another, in call order.
- **After a restart:** a `requesting` state starts again; in `tools`, jobs of
  `idempotent` tools start again under their keys, the others get `aborted`.

### 9.6 The broker

- Rules: deny, then ask, then allow; the strictest match wins. A rule that
  does not parse disables its tool. A tool that no rule names asks.
- Matching for OpenAI: tool names and `Name(arg)` with `*` and `?` globs;
  path rules are refused at compilation.
- An ask goes to the owner when it has an attached stream; otherwise, and for
  an `agent:` owner, it is denied. Kernel asks expire after `ask-expiry`
  (an `ask` deadline of the session's timer, 9.3; at the timer cap the ask is denied `timers`); Claude asks after 10 minutes (the host).
- For Claude, rules went in as flags; the broker sees only asks.
- **Session allows.** An answer `{allow, for: session}` adds a rule to the
  session's `allows`: the tool's name, and for a file tool the workspace it
  is in (`Edit(<workspace>/**)`). The rule names that reach for the page. The
  kernel keeps it; it never sends it to `claude` as a rule
  (`updatedPermissions`), so it can be taken back. Later asks that match an
  allow are answered `allow` by the kernel, with `by: session-allow`.
  `take-back` removes the allow, writes an `allow` entry, and later matching
  calls ask again. Allows end with the session.

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
- **Usage.** After each step the kernel pushes `usage` (9.4):
  `context-used` from the step's input tokens, `context-window` from the
  model, and `compact-at` for OpenAI (the request's `compact_threshold`). For
  Claude, `compact-at` is absent: Claude decides when it compacts.

### 9.8 Adapters

| From | To |
|---|---|
| `assistant` text, `stream_event` deltas | `AgentMessageChunk`; a finished message is a `message` entry |
| `assistant` `thinking` blocks and deltas | `AgentThoughtChunk` |
| `assistant` `tool_use` of `TodoWrite` | `Plan`, with its entries and their status; also a `tool` entry |
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
- **Checked:** it forwards every item unchanged (an `ask` item reaches the
  kernel at once; only its answer waits for the verdict, step 6), answers every `check`
  (`none` when safe, `deny` otherwise), judges every `ask` and input, and
  pushes each verdict as an item
  `{"type":"verdict","on":<on>,"verdict":…,"reasons":[…]}` (`on`: step 3). No
  call of it waits for the judge, so no call nears its budget. Its sequence:
  1. `start` checks the caller (2.2), checks the launch's form, opens its own
     stream to the kernel, calls `claude-code.start` with `pre-tool-check` and
     **no input**, stores `pending/<own stream> {launch, inputs}` in its state
     (`inputs`: the launch input, in order), and returns `{stream: <its own>,
     direct: false}` in the same call. It forwards `consent-pending` and every
     other item.
  2. Every input (the launch input, every `say`, an `interrupt` replacement)
     waits for the host's `initialized` item (4.5; an item call) before its
     judge call. Inputs that arrive before it are kept in `pending/<own
     stream>` in order (at most the kernel's queue, 9.3) and judged after it,
     in order. The provider acts only on the first `initialized` of a stream.
     Once the `claude` stream has ended (consent refused or withdrawn, or any
     end), no input is judged: the provider closes its own stream and drops
     them. Nothing of the session goes to the judge before `initialized`: the
     host starts the process only once the workspace consent is held (8), so
     `initialized` cannot come before it (outcome 5; deputy decision of
     2026-11-20 on S2's RISK 3). On `initialized` it starts the judge call for
     the launch.
  3. Every judge call spans calls: `https-stream.start` (a request, 7), a
     timer `judge/<own stream>/<on>` at the deadline (`timeout-s` minus 2 s),
     the verdict read from the judge stream's items. `on` is the host's ask or
     check id, or `launch`, or `input/<n>` (the provider's own count of the
     inputs on that stream, from 1, the launch input first; `say` in 2.2 carries
     no kernel input number);
     the provider's judge state is keyed by (own stream, `on`). The first
     readable verdict is final, and so is the timer: it firing first is
     `unsafe`, and a later verdict is ignored. A new attempt starts only after
     an error or an ending with no readable verdict, from the `ended` of the
     one before, only while the deadline is ahead; at most 3 attempts. A
     refused `timers.after` (`bad-id`, `too-many`) is an `unsafe` verdict for
     that `on` at once.
  4. Safe on the launch and its input: `claude-session.say(input)`. Unsafe: a
     verdict item, then it cancels the `claude` stream and closes its own; the
     kernel ends the turn `idle {failed {judge}}` with the verdict's reasons.
     No input reaches `claude` before the launch's safe verdict, also when
     the launch input itself was dropped (step 8).
  5. `say` from the kernel is judged the same way (steps 2 to 4), one input
     at a time in order; it returns at once, and the input reaches `claude`
     only after its safe verdict. An unsafe verdict on a later input denies
     that input only: a verdict item, the turn ends `idle {failed {judge}}`,
     the `claude` stream stays; only the launch's unsafe verdict cancels it
     (step 4).
  6. A `check` is judged the same way and answered by `claude-session.answer`:
     `none` when safe, `deny` after an unsafe or unsure verdict, an unreadable
     one, an error or the deadline. An `ask` is answered only with `deny` (unsafe,
     unsure, unreadable, error, deadline); a safe verdict lets the ask go on to the kernel
     unanswered, and the kernel's broker and the owner decide (9.6). So "safe"
     never widens the mode. An owner's `allow` to an ask whose verdict is
     pending is forwarded only after a safe verdict; after an unsafe one the
     ask is already denied and the answer is `unknown-ask`. Claude's own hook
     timeout (`timeout-s`, `onFailure: block`) still stops the tool if the
     provider never answers.
  7. When its own stream to the kernel is full, it defers the item call and
     resumes on `ready` (3). Its `resume` is made in a later call, never in the
     call that returns `defer`.
  8. `interrupt` drops every input of the session still waiting for its
     verdict or for `initialized`; a later verdict for one sends nothing.
- **Callers:** every function of `sessions` first checks that
  `caller.identity()` is the kernel `mods:agent`'s and otherwise returns
  `not-owner` before it judges, opens a stream or makes any call; so no other
  mod can make the provider spend the participant's judge key (2.2).
- **Judge call:** `https-stream.start` with `account: "judge"` (5) to the
  judge's host in its `[http]`, with the tool, its input and a short transcript excerpt; deadline = the
  launch's `timeout-s` minus 2 s; at most 3 attempts inside it (as in step 3); verdict
  `{safe | unsafe | unsure, reasons}`; anything else, a timeout or an error
  is unsafe; so is a refusal of the account (`destination`, `no-account`),
  and a missing or wrong key therefore denies.
- **Breaker:** 3 denials in a row or 10 of the last 50: `interrupt`, and a
  verdict item that says why. Every verdict item carries `denials-in-a-row`.
- **Checking:** before each judge call the provider pushes
  `{"type":"checking","on":<on>}`; the kernel shows it as the
  activity `checking` (9.3) until the verdict.
- **Settings** (judge URL, thresholds) only from its own page, in its state.
  The provider holds no key: the host keeps it as the account `judge`, entered
  in the app (5; N10, the human, 2026-11-20).

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
| `consumer-failed`, `idle-timeout`, `replaced`, `lagging` | stream outcomes | 3, 5, 9.4 |
| `bad-launch {field, reason}` | claude-code | a value outside its grammar (4.1) |
| `no-binary`, `version {found, need}` | claude-code | 4.4 |
| `mismatch {field}` | claude-code outcome | 4.5 |
| `slash-command`, `unknown-ask`, `not-allowed-here` | claude-session | 4.5 |
| `consent-refused {what}` | any request outcome | 8 |
| `consent-pending {what}` | any request, once, while a consent waits | 8 |
| `destination`, `forbidden-header`, `too-large`, `no-app`, `no-account` | https-stream | 5 (`no-account`: no judge key entered) |
| `not-granted`, `unknown-handle`, `unknown-tool` | agent-tools | 6 |
| `too-many`, `bad-id` | timers | 7 |
| `not-owner`, `unknown-session`, `refused`, `queue-full`, `quota`, `paused`, `backend`, `busy`, `too-large {limit}` | kernel | 9, 9.3, 9.5 |
| `not-owner`, `not-direct` | providers | 2.2, 10 (`not-owner`: the caller is not the kernel) |
| `unknown-input`, `unknown-allow`, `behind {through}` (`through`: the high-water index) | kernel | 9.3, 9.6, 9.4 |
| `too-large` | OpenAI loop outcome | 9.5, body budget |
| `failed {timers}`, ask denial `timers` | kernel turn outcome, ask reason | 9.3 (no timer slot at the cap of 240) |

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
refused. These ports rest on watane taking MIT OR Apache-2.0 for all of it
(N8, the human, 2026-11-20; a change to watane, 15).

## 14. Security properties

Each row names the files that hold it: "watane" paths are in the watane
checkout, the others in this repository (layout in 1). A slice whose diff
touches a file of this repository named here gets the adversary step; a slice
that adds or moves such a file updates sections 1 and 14 in the same commit.
Each provider repository keeps a copy of the provider rows of this section
(with this repository's commit it copied, refreshed when they change) and its
`factory/config.toml` `boundaries` points at that copy, so its slices that
touch a provider file named here get the adversary step; a provider repository is built only with that in place (17,
stages B and E).

| Property | Holds by | Files |
|---|---|---|
| A mod cannot widen `claude` past its launch | 4.1 grammar and one argument per value; `--restricted`; fixed flags; closed subagent keys; the host owns stdin (4.5); `initialize` and `init` checks | watane `host/src/agent/claude/launch.rs`, `init.rs`, `control.rs`, `run.rs` |
| A mod cannot run a denied skill by name | `slash-command` (4.5) | watane `host/src/agent/claude/control.rs` |
| A failing check cannot let a tool run | `onFailure: block` confirmed in `initialize` (4.5) | watane `host/src/agent/claude/init.rs`, `control.rs`; checked provider's `mod.toml`, `src/judge.rs` |
| "Safe" cannot widen the mode | checks answer `none` or `deny`, never `allow` (4.5); the checked provider answers an ask only `deny`, a safe ask goes to the owner (10) | watane `host/src/agent/claude/control.rs`; checked provider's `src/judge.rs` |
| Writes cannot plant code that runs later | protected paths (4.3) | watane `host/src/agent/claude/run.rs` (the host's `--settings=`) |
| No mod sees a credential | tokens and the judge's key in the app; MCP tokens on fd 3; handle names, not handles (5, 6) | watane `app/` account files, `host/src/agent/https_stream.rs`, `host/src/agent/tools.rs`; `mod.toml` |
| A plan token goes only to OpenAI's agent endpoints | account binding, forbidden headers, no redirects (5) | watane `host/src/agent/https_stream.rs` |
| A judge key goes only to its judge | the account binding of `judge` (identity, value, origin parse and lifetime as 5): the provider it was entered for, `POST`, the account's origin in the provider's `[http]`, no redirects (5) | watane `host/src/agent/https_stream.rs`, `app/` judge account; checked provider's `mod.toml`, `src/judge.rs` |
| A mod in the middle gains no authority | consent per origin client (8); the providers' `sessions` refuse any caller but the kernel (2.2, 10) | watane `host/` consent per origin client, `app/` consent storage; `mod.toml`; the providers' `mod.toml` and `src/lib.rs` |
| A session is private | one consumer; owner checks by identity (3, 9.3) | watane `host/src/agent/streams.rs`; `src/lib.rs`, `src/sessions.rs`, `src/transcript.rs` |
| A runaway agent stops | counters and delegation limits in the kernel (9.7) | `src/delegation.rs`, `src/openai/` |
| A repeated call changes nothing | client keys, job keys, numbered items (9.3, 6, 3) | `src/sessions.rs`; watane `host/src/agent/tools.rs`, `streams.rs` |
| An ask shows the tool call that will run | the ask carries the call's own id and input as the adapter read them (4.5, 9.6, 9.8) | `src/adapters/`, `src/broker.rs` |
| A session allow can be taken back | the kernel holds it, `claude` never gets it as a rule (9.6) | `src/broker.rs` |
| The console has no power a client lacks | it calls only 2.3, as the kernel's page identity (18.1) | `wit/world.wit`, `mod.toml`, `src/lib.rs`, `console/`, `ui/` |
| A profile or overlay cannot widen what the participant set | compilation and the refuse table (9.2, 11; AGT-F-3, AGT-F-4) | `src/profiles.rs`, `src/refuse.rs` |

Not protected: the judge account reaches any `POST` path of the origin the
participant entered; `claude` itself within its limits; a prompt injection within
what the profile allows; plan usage; another program of the same user
(spec 6); whether a subagent prompt that starts with `/` runs a skill (not
probed; probe V26, slice S1); whether the shell grant's sandbox blocks
loopback (probe V27, slice S1; table in 16).

## 15. Provides and needs

**Provides:** `watane:host` interfaces of 2.1; the kernel's `mods:agent`
interfaces of 2.3; the providers' `mods:claude-code` of 2.2.

**Needs** (N3, the human, 2026-11-20: all of these changes to watane's
contracts are accepted, F-AGENT-2 and W6 included). The checklist is applied
by watane slices, never from this repository. Each row: the watane target,
the change, and the section here it comes from.

- [ ] `spec/VISION.md`: where the agent layer sits in the order of work (before the factory, step 6) (1).
- [ ] `spec/FUNCTIONAL.md` F-AGENT-2 and W6: a token also comes from a toolset consent (6, 8).
- [ ] `spec/FUNCTIONAL.md` F-AGENT-5 to F-AGENT-9 and J11 (`FUNCTIONAL.md` section 2) (3, 4, 9).
- [ ] `spec/TECHNICAL.md` W2: consents stored with grants (8).
- [ ] `spec/TECHNICAL.md` W5: previews refuse the grants and accounts (8).
- [ ] `spec/TECHNICAL.md` new W row: the agent layer's boundary (14).
- [ ] `host/TECHNICAL.md` H1 to H5 and H7 (3, 4, 5, 6, 7).
- [ ] `host/TECHNICAL.md` 9.3: page connection ids and their own queues (3).
- [ ] `host/TECHNICAL.md` host state: timers per mod, kept across restarts (7).
- [ ] `host/TECHNICAL.md` `caller.identity()` (9.3, 14).
- [ ] `host/TECHNICAL.md` `claude-types`: `start-error` and `session-error` also carry the providers' `not-owner` and `not-direct` (2.2, 10, 12).
- [ ] `host/TECHNICAL.md` H1: `stream-sink.items` returns `taken` or `defer`; `streams.resume`; a deferred batch is not a failure; `stream-producer.ended`; a lag clock from a failed `full` push to `ready` that reaches 10 s ends the stream `lagging`, except a stream its consumer has deferred (2.1, 3; P03).
- [ ] `host/TECHNICAL.md` H2: the `initialized` item after `initialize` passes, never before the workspace consent is held (4.5, 10; P04); `claude-session.say`, `answer`, `interrupt` and `compact` are requests, applied after the call commits, so a deferred or failed call sends nothing to `claude` (2.1, 7).
- [ ] `host/TECHNICAL.md` H1: byte budgets for queued items (512 MiB host, 128 MiB per origin client, 16 MiB per page connection; `full`; a page connection past its budget ends `lagging`) (3; P11).
- [ ] `host/TECHNICAL.md` tool annotations `@parallel`, `@idempotent` (6).
- [ ] `host/TECHNICAL.md` a request that waits on a consent tells its requester `consent-pending {what}` once (7, 8).
- [ ] `host/TECHNICAL.md` account kind `judge` and its binding (provider identity compared exactly, `account` exactly `openai` or `judge`, `POST`, origin after one parse with `[http]`, no redirects, `no-account`, ending live judge streams when the key is removed) (5, 10, 12; N10).
- [ ] `pkg/TECHNICAL.md` 4.5: rows for the imports and exports of 2.1 (2.1).
- [ ] `pkg/TECHNICAL.md` 4.5: the changed `stream-sink` export, the `stream-producer.ended` export and the `streams.resume` import (2.1; P03).
- [ ] `pkg/TECHNICAL.md` 5.1 and 7.2: two grant kinds (4.2, 8).
- [ ] `app/TECHNICAL.md` new section: Sign in with ChatGPT and the account file (5).
- [ ] `app/TECHNICAL.md` new section: the consent words and their storage (8).
- [ ] `app/TECHNICAL.md` new section: the `claude` binary setting (4.4).
- [ ] `app/TECHNICAL.md` new section: entering, showing (never the value) and removing the judge key and origin on the checked provider's manager page, with the origin shown next to the key and never prefilled, stored in `<home>/app/accounts/judge/<provider mod id>.json` and deleted when the provider is uninstalled (5, 10; N10).
- [ ] `dev/TECHNICAL.md` new section: the fake `claude` and the scripted Responses server, which also serves a judge origin: it counts every request it receives and plays a judge that answers safe, unsafe or unsure, is down, slow or unreadable, or answers a redirect (12, 16, 17).
- [ ] watane licence files: MIT OR Apache-2.0 for all of watane (13; N8).

## 16. Test plan

Integration tests are modules of one binary per subsystem
(`<sub>/tests/it/<file>.rs`). The kernel and providers are tested with
`watane-dev test` in this repository; the console as 18.7 says.

| Kind | What |
|---|---|
| Probes | `proto/claude-probes/probes.py` and `proto/agent-spike/chatgpt-probes.py` against the real programs, rerun after each `claude` update and before raising the minimum version |
| Golden fixtures | recorded stream-json per `claude` version; recorded Responses streams; each mapped to entries and updates, compared to a stored result |
| Fakes | dev's fake `claude` (scripted lines, asks, checks, a hook that errors or times out, a crash) and scripted Responses server (checks every request body, answers errors and usage limits); the same server also serves a judge origin: it counts every request it receives and plays a judge that answers safe, unsafe or unsure, is down, slow or unreadable, or answers a redirect (the "scripted judge server" of cases 25, 32, 33) |
| Hostile | one test per boundary of 14, as the attacker it names (the adversary step of a slice) |
| Load | a slow test per stage (17): the stage's fakes at the load of its budget row, p95 measured against the budgets of 17, recorded in "Budget record" |
| Crash windows | kill the host at named points (after commit, before effect; mid-batch; mid-step) and restart for real; count effects, not return values |

**Probes named here.** These V ids are probe ids, not watane's `V<n>` goals
of `spec/VISION.md`.

| Id | What it checks | Run by | Result recorded in |
|---|---|---|---|
| V6 | the cost of one item through watane's call path: with no write, and with one state write (one full fsync) | done (`proto/stream-cost-bench`) | `ARCHITECTURE-PROPOSAL.md` section 18; it is why 9.4 writes once per finished entry, never per delta |
| V7 | latency of one judge check (`pre-tool-check`) on a real judge account | stage E, before the checked provider is built | `ARCHITECTURE-PROPOSAL.md` section 18 |
| V8 | the stage A prototype through three layers (host stream, a mod consumer, a page consumer): order, delay and the cost of one fsync per commit (7, N4) | stage A | "V8 record", at the end of this section (numbers; whether to change 7 is the human's) |
| V26 | whether a subagent prompt that starts with `/` runs a skill (14) | slice S1 | `ARCHITECTURE-PROPOSAL.md` section 18 |
| V27 | whether the shell grant's sandbox blocks loopback (14) | slice S1 | `ARCHITECTURE-PROPOSAL.md` section 18 |

**Cases** (the research's recurring ones first):
1. An orphaned tool call is repaired before the next request; a result
   without a call is dropped.
2. Each request of an ordinary step extends the previous one without moving
   its prefix (cache). Trimming by the body budget, a compaction and a summary
   restart each start a new prefix, once; the request after it extends that
   one. `instructions` are byte-stable through all of them (three tests: one
   per kind of new prefix).
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
19. The account binding of `openai` and of `judge`: every other host, path,
    method, header and a redirect is refused before a byte leaves.
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
25. A judge that is down, slow, unreadable or without an entered key
    denies; 3 denials in a row interrupt the turn.
26. An overlay that widens anything is `refused`, naming each field.
27. `unqueue` of an input a turn already took is `unknown-input`; the turn
    is not changed.
28. A session allow answers the next matching ask; after `take-back` the
    same call asks again; `claude` never received the rule.
29. An ask entry carries its tool call id, input and `expires-at`; the host
    denies it at that time.
30. An attach mid-message shows that message once, whole, from its entry;
    no delta of it is shown. An attach with more than 128 entries or 2 MiB
    behind is `behind`; paging with `read`, then attaching, misses and
    doubles nothing. Streaming a turn writes state once per finished entry,
    never per delta (count the commits).
31. Every turn-state change reaches an attached stream as a `turn` update,
    in order, also `checking` and `consent` (stage B has neither: C adds
    `consent`, E adds `checking`, 17).
32. A mod that is not the kernel calls a provider's `sessions.start` or `say`:
    `not-owner`; no request with `account: "judge"` leaves (counted at the
    scripted judge server), and no verdict reaches the caller; the same for a
    mod `local/agent` and a mod with slug `agent` from another publisher.
33. The judge binding: another caller, `GET`, another origin, an origin not in
    `[http]`, userinfo, `:443` or upper case forms, an `account` other than
    `openai` or `judge`, and a redirect are each refused before a byte leaves;
    a removed key gives `no-account`.
34. (9.4) An entry of 3 MiB is stored as `part` entries of at most 256 KiB;
    `read` forwards and backwards, and attach, each make progress and give it
    back byte for byte; an `after` inside the group starts at its first part.
35. (9.3) An input over 256 KiB is `too-large`; the 33rd queued input, or one
    that takes the queue past 2 MiB, is `queue-full`; with every record field
    at its cap the record stays under 512 KiB; the 65th session allow is
    refused and its ask stays open.
36. (3) A consumer that answers `defer` gets the same batch, same numbers,
    after `resume` and nothing before; two defers never end the stream
    `consumer-failed`; the deferred call's pushes and writes are gone; a
    deferred host stream stops reading its source. The kernel, outside an item
    call, closes a full owner stream `lagging`, and a reattach misses and
    doubles nothing.
37. (10, outcome 5) With the workspace consent pending, the scripted judge
    server counts no request; refused, it never counts one; once held, the
    first request comes after `initialized`. `start` returns before any
    verdict.
38. (10) A judge slower than the deadline: the timer denies; the late verdict
    changes nothing; no input reaches `claude` (fake) before its safe verdict,
    and an unsafe verdict on a later input (not the launch, step 4) ends the
    turn `idle {failed {judge}}` while the `claude` stream stays; the next input with a safe verdict reaches `claude`.
39. (9.5) A response with 33 parallel calls: at most 16 jobs, and at most 8 of
    one session, are open at once; every call gets exactly one output; a model
    request of another session still starts; after a restart the slot counts
    start from zero.
40. (9.5, case 2) The three tests of case 2: trimming by the body budget, a
    compaction and a summary restart each start one new prefix; the trimming
    test says: the next 3 requests extend the trimmed one.
41. (9.4) `read` with `before` gives the newest page first; paging back to
    index 0, then attaching after the newest index shown, shows every entry
    once; `last` is the high-water index.
42. (10, 14 "Safe" cannot widen the mode) A checked session in `manual` mode;
    the fake `claude` raises an ask and the scripted judge answers safe. The
    provider sends `claude-session.answer` only for `deny`; the safe ask
    reaches the owner unanswered, and the tool runs only after the owner's
    `allow`. Unsafe, or the deadline, gives `deny`.
43. (10, outcome 5) With the workspace consent pending, (a) `interrupt` with a
    replacement and (b) `interrupt` then `say` each leave the scripted judge
    server's count at zero until `initialized`; with the consent refused or
    withdrawn, a later `say` never reaches it; every input a `say` gives
    before `initialized` is judged after it, in order.
44. (10) The scripted judge answers `unsafe` (and, in a second test,
    `unsure`), then ends its stream normally: it counts exactly one request,
    and the check is answered `deny`. A new attempt starts only after an error
    or an ending with no readable verdict; the first readable verdict, or the
    timer, is final.
45. (2.1, 7, 3) An idle line with one queued input arrives while the owner
    stream is full; after `resume`, the fake `claude` received that input
    exactly once; a `timers.cancel` in a deferred call is dropped with it and
    made again on redelivery.
46. (10) Two checked sessions with the same `on`: each keeps its own deadline
    timer `judge/<own stream>/<on>` and its own verdict.
47. (10) `interrupt` drops every input still waiting for its verdict or for
    `initialized`, and a later verdict for one sends nothing; a second
    `initialized` on a stream changes nothing.
48. (9.5, AGT-F-55) One owner that asks for more than half of each slot kind
    gets 4 provider, 4 request and 8 job slots at most, and another owner's
    work still starts; a consumer that defers cannot spin: its `resume` in the
    deferring call is dropped.
49. (9.4) Paging backwards with `read` through a 3 MiB entry, with pages that
    end inside its part group, joins it whole, byte for byte.
50. (10) `start` with the consent pending, `interrupt`, `say X`; after
    `initialized` the judge answers input/2 safe at once and the launch late
    (and, in a second test, unsafe): X reaches the fake `claude` only after the
    launch's safe verdict, and never after an unsafe one. A refused
    `timers.after` for a judge call denies that `on` at once.
51. (3, 9.4, 10) The owner stream ends mid-deferral: upstream resumes and
    nothing is lost from the transcript. A page that stops reading: `lagging`
    after 10 s, ended by the host, and a reattach misses and doubles nothing;
    a page that takes a few items and then stops: the same, and the Claude
    session and its deferred upstream stay alive;
    the `ended {replaced}` of an old owner stream leaves the new one live. A consumer that
    never resumes: `consumer-failed` after 5 minutes. A checked provider whose
    downstream ends cancels its `claude` stream.
52. (9.5, 2.3) A `start` or `say` when the owner holds 4 provider streams, or
    all 8 are held, is `busy` and does not wait; a Claude session idle for 10
    minutes releases its stream (the `idle` deadline, 9.3, set when a turn ends
    idle, cancelled by `say`), and the next `say` resumes it with `session
    {resume}`; with `persist: false` that `say` continues as for
    `resume-lost`. After `busy` (or `queue-full`), a retry with the same key
    succeeds once a slot is free (9.1). A `task` child refused `busy` gets the
    output `{err: {code: busy}}` (stage F).
53. (9.3) A client key over 256 bytes and a `rename` over 256 bytes are
    `too-large(256)`; the 65th session allow is `too-large(65536)` and its ask
    stays open; a session at 16 384 chunks ends its turn `stopped {limit:
    quota}` and new input is `quota`.
54. (9.5, P07) After a compaction, the next request's `input` starts at the
    compaction item; after a summary restart, at the summary; the wire chunks
    before `active` are gone; a session of 1 000 steps reads, per step, bytes
    within 10% of one of 10 steps (state bytes counted).
55. (9.1, 9.4, P08) Appending a 4 KiB entry to a transcript of 10 and of
    10 000 entries rewrites at most 64 KiB + the entry; an entry of 100 KiB
    has a chunk of its own; earlier chunks are never written again; `read` and
    attach give back every entry once across chunk ends.
56. (9.1, 9.4, P09) An item call carrying only a delta reads `stream/<id>` and
    `live/<session>` and nothing else, and writes nothing (count the reads and
    commits); after a restart, no `stream/` key or `live/` key of a stream
    from before it is left.
57. (9.3, 9.5, 9.6, 7, P10) One session with an open ask, a retry and an idle
    release holds one host timer; it fires each in time order. With 240
    session timers live, a new ask is denied `timers`, a retry ends `idle
    {failed {timers}}`, a usage limit pauses with no time, an idle release
    happens at once; 5 owners × 64 paused sessions never get `too-many`.
58. (3, P11) Producers that together pass an origin client's 128 MiB, or the
    host's 512 MiB, get `full` on `push`; a page connection that stops
    reading past 16 MiB ends, its streams end `lagging`, and a reattach misses
    and doubles nothing.
59. (9.3, P12) Kept keys are charged to their owner and stay charged after
    `delete`; owners together stop at 224 MiB with `quota`; a turn running
    when the account fills ends `stopped {limit: quota}` and that entry is
    written; the reserve is never used by entries of a turn.

**V8 record.** Empty until stage A fills it: the numbers of V8 (order and
delay through the three layers, the cost of one fsync per commit), the date
and the machine. Whether to change 7 is the human's.

**Budget record.** Empty until a stage's load test (kind "Load") runs: for
each stage A–F, the p95 numbers of the budget table in 17, the date and the
machine. A stage is done only within its budgets.

## 17. Build plan

Each stage is one or more vertical slices; each works before the next
starts. The backend part of a slice is done first and carries the slice's
weight. The console part shows that backend in the console, so the human can
watch it work. A console part never waits on a backend part of a later
stage: it shows only what is built.

**One-call bound.** One kernel mod serves every session through one call
slot, so each kernel call does bounded work: one batch (3) or one step (9.5),
with state I/O within its stage's bytes budget, whatever the transcript's
length. A long session cannot hold the slot.

| Stage | Backend | Console shows | Repository | Done when |
|---|---|---|---|---|
| A | H1 streams, H5 durable requests, H7 timers; the V8 three-layer prototype (16, "Probes named here") | the page skeleton (18.4 layout, theme) attached to a kernel stream, numbered items arriving | watane: host, pkg (H1, H5, H7); this repository: the kernel stream, the console skeleton | cases 7, 20, 21 pass; cases 36 and 51 for the host part (3); case 58 (host part); the budgets of stage A; V8 measured and recorded in 16, "V8 record" |
| B | H2 with the unchecked provider and a kernel that runs Claude sessions (9.3, 9.4, 9.8, the queue and session allows); dev's fake `claude` | a session on the fake: transcript, status line, asks, queue, Stop, reload and reattach | watane: host H2, dev's fake `claude`, pkg; the unchecked provider's own repository (built only with its `factory/config.toml` `boundaries` pointing at 14); this repository: kernel, console | J11 on the fake without its consent steps (a stage B launch uses a scratch workspace of 4.6, which asks none; the plan line, the folder consent and its withdrawal are stage C) and AGT-J6 (Claude part); cases 5, 6, 8–18, 27–30, and 31 for the turn states stage B has (not `checking`, not `consent`); new cases 34, 35, 36 (kernel part), 41, 45, 49, 51 (kernel part), 52, 53, 55, 56, 57 (idle and ask parts), 58 (the kernel's reattach part), 59; the budgets of stage B |
| C | H6 consents and the app's words; the `claude` binary setting; the plan line | the same on the real `claude`; the `consent` activity; new session with reach | watane: host H6, app; this repository: the console's consent activity and plan line | J11 on the real `claude`, with the consent, plan line and withdrawal; AGT-J4; case 31 for `consent`; the budgets of stage C |
| D | H3 and accounts; the OpenAI loop (9.5); dev's scripted Responses server | backend choice with refusals; paused until a time; the context meter | watane: host H3, app accounts, dev's Responses server; this repository: the OpenAI loop, the console | AGT-J2, AGT-J6; cases 1–4, 19 (`openai`), 22; new cases 39, 40, 48, 54, 57 (retry and usage-limit parts); the budgets of stage D |
| E | The checked provider (needs a judge account, V7: 16, "Probes named here") | checking, verdicts, the breaker | watane: the host's account kind `judge`, the app's judge page, dev's judge fake (16); the checked provider's own repository (built only with its `factory/config.toml` `boundaries` pointing at 14); this repository: the `checking` activity | AGT-J3; cases 19 (`judge`), 25, 32, 33; case 31 for `checking`; new cases 36 (provider), 51 (provider part), 37, 38, 42–44, 46, 47, 50; the budgets of stage E |
| F | Delegation and counters (9.7); factory profiles | children in the transcript; stopped at a limit | this repository: kernel, console; N5 (open, decided at stage F: (a) the Claude grant with shell and web, nothing new, or (b) a `codex app-server` backend in the host, a third native process kind; recommended (a)) | AGT-J5; cases 23, 24, 52 (`task` part); the budgets of stage F; N5 decided |

Budgets (the "Load" kind of 16). All p95; measured by a slow load test on the
stage's fakes, recorded with the date and the machine in 16, "Budget record";
a stage is done only within them. Writer busy share: the share of wall time the host's state writer (host 7, one per host) spends in commits, fsync included, over the load run. B and D run with 2 owners (two client mods), so the per-owner caps hold; stage A's 8 host streams come from a test source in the host's own tests (no H2–H4).

| Stage | Load | Item delay, source to page | Stop, to idle at the page | Writer busy share | State bytes read and written |
|---|---|---|---|---|---|
| A | 8 host streams × 100 items/s of 1 KiB, through a mod consumer to a page | ≤ 50 ms | `cancel` to `ended`: ≤ 250 ms | ≤ 10% | none per item |
| B | 8 fake `claude` sessions, each 50 deltas/s and 5 finished entries/s of 4 KiB; transcripts at 10 and at 10 000 entries | ≤ 100 ms | `interrupt` to `idle {interrupted}`: ≤ 1 s | ≤ 50% | per finished entry ≤ 2 MiB, within 10% between 10 and 10 000 entries |
| C | as B on the real `claude` | ≤ 100 ms (kernel and host part) | ≤ 2 s | ≤ 50% | as B |
| D | B, plus 8 OpenAI sessions on the scripted Responses server, each one step/s with 4 tool calls; wire histories at 10 and at 1 000 steps | ≤ 100 ms | ≤ 1 s | ≤ 60% | per step ≤ 4 MiB, within 10% between 10 and 1 000 steps |
| E | D, with the Claude sessions through the checked provider and the scripted judge answering in 200 ms | ≤ 100 ms for items no verdict holds; a check adds ≤ judge time + 100 ms | ≤ 1 s | ≤ 60% | as D |
| F | D, each OpenAI session with 2 live children | as D | as D, the children stopped too | ≤ 60% | as D |

The should-dos P07 and P08 (`research_notes/performance-review-2026-11-20.md`)
are now rules: the active history (9.5) and the 64 KiB chunk (9.1). The bytes
budgets of stages D and B measure them (cases 54 and 55).

## 18. The console (C1)

The kernel's own page. Its first target is the canvas row "Revision 2" of
["watane agent frontend"](https://claude.ai/artifact/8jz2aHuaPo2REtKQyMt89e)
(`VISION.md`). The records `FRONTEND.md` and `FRONTEND-2.md` say why it looks
so; this section states the rules. Where a frame and this section disagree,
this section holds, and the frame is the reference for anything it does not
say (layout, spacing, words).

### 18.1 A client like any other

- The page calls its own mod, which serves it the kernel interface of 2.3;
  `caller.identity()` is the kernel's `page`, and that identity owns the
  console's sessions. The page has no other way in: no export, state key or
  stream that a client mod lacks.
- It lists and opens only its own sessions (`list`, AGT-F-7). It never shows
  the sessions of other clients; F-AGENT-6 stands (N11, the human,
  2026-11-20).
- The profiles it offers are those `access/page` names (9.1).

### 18.2 Stack

- React and assistant-ui, bundled (decided with the human, 2026-11-20). The
  source is in `console/`; the build writes `ui/`, which the host serves as
  any mod's page (host 8.3): same-origin script files, no inline script, no
  `eval`, no WebAssembly, no frames, no outside site.
- State: assistant-ui's ACP runtime. Its adapter is extended for the
  `_meta.watane` items of 9.4 (`ask`, `allow`, `verdict`, `turn`, `queue`,
  `usage`, `synced`, entry indexes).
- Rendering: Shiki with its JavaScript regex engine for code, anser
  (escaped) for program output, and a diff view that needs no `eval`.
- Every dependency is pinned in a lockfile, and the build records a hash of
  each file it writes. `ui/` is committed and is the only thing the host
  reads.
- The console's parts stay in their own folder, `console/kit/`, so a second
  mod can take them as one package (`@watane/agent-ui`) when it needs them.
  It is not published before then.

### 18.3 From the kernel to the screen

On open and on every reconnect: `attach(session, after: <last index shown>,
caps)`; on `behind`, page with `read` backwards (`before`, newest page first, so the end of the
transcript shows at once), then attach again. The items before `synced`
replace nothing that is already shown, and an index already shown is
dropped. Then live updates follow. A message that was being written at
attach shows "Writing…" until its entry arrives.

| Kernel gives | Console shows |
|---|---|
| `message` (user) | a quiet panel, right |
| `AgentThoughtChunk` | "Thinking…", open while it streams; then folded to "Thought for n s" |
| `ToolCall` of reads and searches in a row | one folded line, "Explored · n reads, m searches" |
| `ToolCall` of an edit, a program run, or any failed call | a card: name, argument, diff or output, status; a failed one open |
| `ask` (unanswered) | the card of its tool call with the ask bar; "Needs you (n)" above the input |
| `ask` (decided), `allow`, `verdict` | a decision line: who, what, the reason |
| `Plan` | the plan above the input, "n of m done" |
| `turn` | the status line (18.5); at a turn's end, a footer line of time, tools and tokens |
| `queue` | the queue above the input |
| `usage` | the context meter in the header |
| `unknown`, `unreadable` | a muted line, "An event this console does not know", raw on demand |

### 18.4 Layout and look

The frames give layout, spacing and words; these values are fixed:
- Sidebar 260 px (hidden under 760 px); header with title, profile, provider,
  workspace and any session allow; transcript column at most 780 px; the
  dock: Needs you, plan, queue, status line, input.
- The manager's look: Inter, JetBrains Mono for code; light and dark token
  sets of the frames (`--bg #F6F6F4`, `--panel #FFFFFF`, `--ink #18181B`,
  `--muted #5D5E66`, `--line #E3E3DF`, `--accent #3355D6`, and their dark
  values).
- Buttons 36 px, chips 32 px, rows at least 44 px. Radii: 9 px for buttons,
  12 to 14 px for cards, 18 px for the input. An outer radius is the inner
  one plus the padding.

### 18.5 The status line

| Turn | Word | Shape | Time | Acts |
|---|---|---|---|---|
| `running` | the activity: Thinking, Reading `<file>`, Writing | dot | elapsed | Esc stops |
| `running {checking}` | Checking with your judge | square | elapsed | |
| `running {compacting}` | Making room | dot | elapsed | |
| `running {consent}` | Waiting for your answer in watane | diamond | | open the question |
| `waiting` | Waiting for you | diamond | time left to `expires-at` | Needs you |
| `paused {until}` | Paused until hh:mm | two bars | | Resume now |
| `idle {completed}` | Idle | ring | | |
| `idle {stopped}` | Stopped at the `<limit>` | ring | | |
| `idle {failed}` | Failed, with the cause | triangle | | Try again |
| `unknown` | Not running: watane restarted | dashed ring | | Continue |
| `resume-lost` | Claude no longer has this session | dashed ring | | Continue from a summary |

Only the word and its subject are in the live region; the clock is not read
out. Stop shows "Stopping…" until a `turn` update says idle.

### 18.6 Behaviour

- **Place.** The transcript follows new output only while the reader is
  within 40 px of the end; otherwise a "New activity below" pill appears.
  Session rows do not move while the pointer is over the list.
- **Asks.** Allow once (primary), Allow `<reach>` this session, Deny… with a
  reason field, Cancel, and Esc in that field closing it. "Needs you" scrolls
  to the first open ask and puts focus on Allow once. A session allow shows
  in the header until the session ends or `take-back`.
- **Queue.** Up to 3 shown, then "+ n more". Edit takes an input back into
  the composer (`unqueue`); remove has Undo for 6 s; Stop and send interrupts
  with it. A full queue turns Send off and the placeholder says why.
- **Plan.** It folds to one line while an ask waits or inputs are queued,
  unless the participant opened it.
- **Keys.** Enter sends, Shift+Enter is a new line, Esc in the composer
  stops the turn.
- **Motion.** Only under `prefers-reduced-motion: no-preference`; hover
  styles only under `hover: hover`; a pressed button scales to 0.96 over
  150 ms.
- **Accessibility.** Every state has its own shape; a streaming message is
  `aria-busy` until done; every control is reachable by keyboard and named.

### 18.7 Tests

| Level | What | How |
|---|---|---|
| View | each row of 18.3 and 18.5 from recorded event streams: the fake `claude`'s and the scripted Responses server's output, stored as fixtures | a component test run, under the 2-minute budget |
| Page | AGT-J6 against the kernel in `watane-dev serve` with the fake `claude` | a browser test, marked slow |
| Seen | each slice's console part, by the user-sim against the frames | the acceptance step |

### 18.8 Not in the first target

The items of `FUNCTIONAL.md` section 5 under "In the console". Each needs its
backend first, through this file's backend sections.
