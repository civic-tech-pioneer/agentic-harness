# Agentic harness: technical specification

How the agent layer does what `FUNCTIONAL.md` says. It is the last of the
spec chain (`VISION.md`, `FUNCTIONAL.md`, then this); code follows it, and
where this file disagrees with the two above, this file is fixed.
`ARCHITECTURE-PROPOSAL.md` (revision 4) is the record of why; this file
states the rules. The human answered the open points of the proposal
(section 15, `N<n>`, and N11 of `FUNCTIONAL.md`) on 2026-11-20, and N5 on
2026-11-27 ((a), 17); each answer is a rule in the section that owns its
fact, cited as `(N<n>, the human, <date>)`.

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
| M1 Providers | two mods `mods:claude-code`, from two publishers: both in this repository, the unchecked one in `provider/`, the checked one in `provider-checked/` (R-B1, the human, 2026-11-23) | unchecked: start and hand off; checked: judge then forward (10) |
| C1 Console | the kernel's page, `ui/`, built from `console/` in this repository | starting sessions, the transcript, the status line, asks, the queue, the session list (18) |

**This repository** is the kernel `mods:agent` and its console (R1, the human,
2026-11-20), and both providers: the unchecked one in `provider/`, the
checked one in `provider-checked/` (R-B1, the human, 2026-11-23). A provider
may move to its own repository later if there is evidence it should; that is
an option, not a rule (R-B1, the human, 2026-11-23). The kernel's pure logic is a plain Rust crate, `kernel/`
(`agent-kernel`, serde only), with its own tests; the WASM mod wraps the crate
later (PLAN, deputy decision "The kernel's pure logic is a plain Rust crate").
Layout: the files marked built exist; the others are planned (the first build
slice that touches one may rename it; if it does, it updates sections 1 and 14
in the same commit):

- built: `kernel/src/lib.rs` (exports), `kernel/src/error.rs` (12: codes,
  problems, refusals), `kernel/src/profile.rs` (4.1, 4.2, 9.2: parsing and
  grammar), `kernel/src/overlay.rs` (9.2: overlays that only narrow),
  `kernel/src/refuse.rs` (11), `kernel/src/check.rs` (9.2 `check`: every
  refusal at once), `kernel/examples/check.rs` (the maker's command) and
  `kernel/tests/it/` (the tests)
- built: `mod.toml` at the root
- built (S32): `kernel/wasm/wit/world.wit` (2.3, the imports and
  exports of 2.1) with `kernel/wasm/wit/deps/watane-host/host.wit` (host
  19.1's text, 2.1 "Exact text"); they move to `wit/` at the root, where
  `check-mod.sh` checks them, once the watane-dev that CI installs links host
  19 (watane S96 and a release: before it, watane-dev refuses
  `watane:host/streams`); `kernel/wasm/` (`agent-mod`, the WASM wrapper: its
  `src/lib.rs` holds the exports, the owner checks by `caller.identity()`
  and the host and state over the imports; 14); `kernel/wasm/logic.wasm`,
  built from it by `factory/gate.sh` and git-ignored. It is prebuilt logic
  (watane dev 7.1): dev's builder takes only `logic/src` and the crates of
  its allowlist (dev 6.2: `wit-bindgen`, `serde`, `serde_json`), and the
  kernel needs the crates of 13
- planned: `kernel/src/sessions.rs` (9.3: records, keys, queue, quota) and
  `kernel/src/transcript.rs` (9.4: entries, attach, `synced`); built:
  `kernel/src/quota.rs` (9.3 Quota: the limits, the accounts, the metered
  state that charges each write)
- planned (S151): `kernel/src/manage.rs` (9.10: the `manage` export, the
  `clients` list, the access questions and their answers, the reach text,
  offers and adoption) and `manager/` (the profile manager view of the
  kernel's page, built into `ui/manager/`; 18.1)
- planned: `kernel/src/broker.rs` (9.6, session allows) and
  `kernel/src/delegation.rs` (9.7: delegates, limits, counters)
- built (S40): `kernel/src/openai/` (9.5: `wire.rs` the wire history,
  repair, the body and its budget; `answer.rs` the host's https items
  classified; `run.rs` the loop's states, requests, retries, pauses, slots
  and restart; `tools.rs` toolsets, the broker's OpenAI asks and the jobs;
  `skill.rs` the listing and read of compiled skill files, 11)
  and `kernel/src/named.rs` (2.3 `profiles` and `check` by name, 9.1
  `access/`); `kernel/src/adapters/` (9.8); the WASM mod's exports and owner checks by
  `caller.identity()` (9.3) are in `kernel/wasm/src/lib.rs` (S32, 14)
- `console/` (source) and `ui/` (built from it), 18

A provider holds `mods:claude-code` with `src/lib.rs`; the checked one,
built in `provider-checked/` of this repository (S127, R-B1), also
`logic/src/judge.rs` (10: the System One request, its reading and the
threshold mapping), a self-contained mod like the unchecked one under the
same boundary list: `mod.toml` (`[http]`: the judge's host), `wit/world.wit`
(`sessions`, `settings`, and the sink, producer and timer exports),
`logic/src/lib.rs` (the owner check, the judged sequence of 10 and the
breaker, behind a `Host` trait tested natively), `ui/` (the thresholds page),
`BOUNDARIES.md` (a copy of its rows of 14). The unchecked provider is
`provider/` of this repository (S35, R-B1), a self-contained mod: its own `mod.toml`, `wit/world.wit`, `logic/`
crate and tests, no path into `kernel/` or `console/`. Its boundaries are
the provider rows of 14, in this repository's one boundary list
(`factory/config.toml` `boundaries`); `provider/BOUNDARIES.md` is a copy of
those rows beside it. The kernel links it as a local provider (`kernel/wasm/deps.json`,
`kernel/wasm/wit/deps/claude-code/`, written by `watane-dev add`).

`factory/publish-test-mods.sh <publisher-home> <publisher-channel-link>`
assembles the two mod folders in a temporary directory. It builds the WASM
components and builds the console under the agent mod's `ui/`. It publishes
the prebuilt provider first, then uses `watane-dev add --release` to bind the
staged agent to that release. It checks and publishes the agent and prints
its install link. Both releases use the same test publisher. The publisher
app signs them on approval and exports the site. The configured loopback URL
serves that directory throughout publishing and installation.

A new source file starts with a `//!` line that cites its section here, as
every watane file does.

## 2. Interfaces

### 2.1 Host (`watane:host@0.1.0`, added)

```wit
interface streams {                                  // no grant
  type stream-id = string;                           // 128 random bits as 32 hex chars
  record item { seq: u64, body: list<u8> }
  enum sink-outcome { taken, defer }
  record version-need { found: string, need: string }
  variant outcome {                                  // a producer's `close` passes only closed, replaced, lagging or failed
    closed, cancelled, lagging, replaced, consumer-failed, idle-timeout, gone,
    mismatch(string), no-binary, version(version-need), consent-refused(string),
    no-app, no-account, failed(string),
  }
  enum stream-error { not-yours, unknown-stream, full, too-large }
  open: func() -> result<stream-id, stream-error>;
  push: func(s: stream-id, item: list<u8>) -> result<_, stream-error>;
  close: func(s: stream-id, outcome: outcome);
  cancel: func(s: stream-id);
  hand-off: func(s: stream-id) -> result<_, stream-error>;
  resume: func(s: stream-id) -> result<_, stream-error>;   // consumer only: after `defer` (3)
}
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
  enum timer-error { too-many, bad-id, bad-delay }
  after: func(id: string, delay-ms: u64) -> result<_, timer-error>;
  cancel: func(id: string);
  firing-origin: func() -> option<string>;
}
enum timer-error { too-many, bad-id, bad-delay }     // bad-delay: over 30 days (7)
interface timer-sink { fired: func(id: string); }    // host-only export

interface host-info {                                // no grant (the human, 2026-11-21)
  generation: func() -> u64;  // the host's start number (rule G1)
}

interface agent-access {                             // proposed (S151, 15): no grant; a request (7)
  enum access-error { not-page, no-mod, too-large, too-many }
  ask: func(client: string, profile: string, reach: string) -> result<stream-id, access-error>;
}

// in the existing interface caller:
identity: func() -> string;   // <publisher>/<slug>, local/<slug>, page, or agent:<handle-name>
```

> `timers.firing-origin()` is a pure read through an import with no grant. In the timer owner's top-level `timer-sink.fired(id)`, it returns `some(identity)` with the mod identity defined in watane’s `spec/TECHNICAL.md` 3 that the host resolved for the firing's chain. The host resolves it after taking the mod's call lock and rechecking the row and origin client. If the stored origin is missing or disabled, or removal rewrote the row to the timer owner, the read returns the timer owner's identity. The read gives no previous origin, local name, generation or reason for fallback. It returns `none` outside that invocation, including in every nested call. The value is stable for that invocation and does not report live status. A firing ignored because the timer was cancelled or replaced never enters `fired`. The immediate caller remains `host`. The value cannot select an origin or confer authority. The host still checks grants, consents and the origin client at apply.
>
> In the Harness (host TECHNICAL 2.6), `timers.firing-origin` returns `none`. A `Fakes` entry under `watane:host/timers` overrides it. Previews use the real host and need no fake.
>
> The kernel binds a timer id to the expected session owner in its own state when it arms the timer. Before a timed resume or restart retry starts owner-scoped work, the kernel compares `timers.firing-origin()` with that owner's expected origin client (8, 9.5). For owner `page` it compares the kernel's installed identity as reported by the host. If the value is `none` or differs, it performs only local cleanup or waits for an ordinary call carrying the owner's authority. It makes no model request, toolset operation or consent request on that path. A durable timer armed in the intended owner's chain can drive owner-scoped restart recovery through `fired`. An idempotent job eligible for recovery uses its original key and the usual owner, consent and tool checks. Other jobs keep their existing abort rule. The host supplies no startup callback. A restart pass under another owner can do local housekeeping and cannot use a stored identity to schedule or perform work under the session owner's authority. Work without an armed timer waits for an ordinary call carrying the owner's authority.

**Access questions** (proposed, S151; watane's to write, 15). `agent-access.ask`
asks the participant, in the app, whether `client` may start agents with
the calling mod's profile `profile`; it is how the kernel's `manage.allow`
(9.10) asks. The host accepts it only when the top-level call of the chain is a
call from the calling mod's own page to that mod (the caller's own page, not
any page): from a timer firing, a stream item, a request's effect, another
mod's call (a collect callee included) or an agent's tool call it is
`not-page`. `client` is the identity (`caller.identity()`
form) of an installed mod other than the caller, or `page` for the caller's
own page; any other is `no-mod`. `profile` is `[A-Za-z0-9._-]{1,64}` and
`reach` plain text of at most 4 KiB (`too-large`); at most 4 questions are
open per calling mod (`too-many`). It is a request: applied after the call
commits, it opens a stream whose producer is the host and whose consumer is
the caller, and asks the app (8). Granted: one item `{"type":"granted"}`,
then `closed`. Refused, or no answer in 600 s: `consent-refused("profile:<client>:<profile>")`.
A preview or a home with no app: `consent-refused` at once. The caller's
`streams.cancel` withdraws the question. The host keeps nothing.

**G1.** `host-info.generation()` is the host's generation, its start number:
raised once each time the host starts, kept in host state, the same for every
call until the next start. It is a pure read: not a request (below), no grant.
The kernel cannot tell a restart from its own state, so it compares this
number with the one it keeps (9.3, "Restart"). (The human, 2026-11-21:
"restart detection as a new interface `host-info` with `generation()` (in
place of `caller.boot()`)". The kernel's stored fields that hold it keep the
name `boot`: `slots.boot`, a record's `boot`, the `active/` pair and the
`slots/wait/` entry; each holds a generation.)

Every function with an outside effect is a request (7): `claude-code.start`,
`claude-code-shell.start`, `https-stream.start`, `agent-tools.open`,
`agent-tools.call`, `streams.open`, `timers.after`, `timers.cancel`, and
`claude-session.say`, `answer`, `interrupt` and `compact` (applied after the
call commits, so a rolled-back call, deferred or failed, sends nothing to
`claude`). `streams.resume` is not a request: it only lets a deferred batch go
on (3); one made in the call that returns `defer` is dropped with it.

**Exact text.** watane's host 19.1 (`host/wit/host.wit` in the watane
checkout) is the exact WIT; the block above names what the kernel uses. The
kernel's copy is `kernel/wasm/wit/deps/watane-host/host.wit` (1), taken from host 19.1 and
refreshed when it changes (S32; S35 takes host 19.1 with `host-info` as
watane S99 writes it). The WASM wrapper's `generation` (G1) is
`host-info.generation()` (S35; stage A's constant 1 is gone).

**The kernel's side of the seam** (S32 seam audit, against host 19):
- An owner stream (9.4) is the kernel's own `streams.open()`, made in the
  owner's call; it can fail `not-yours` (a page call that carries no live
  connection, host 8.2) or `full` (32 per consumer), and `attach` then answers
  `backend("<the host's code>: <cause>")` and writes nothing. A provider
  stream is never opened by the kernel: it is the `stream` of the provider's
  `start` (2.2). A provider `start` that fails (any `start-error`, or the
  provider is not linked) refuses the session's `start`
  `backend("<the error's case>")` and writes nothing.
- **The kernel's calls on a provider stream** (S35). The record keeps whether
  the stream is `direct` (2.2). On a `direct: true` stream the kernel is the
  host stream's consumer (after the provider's `hand-off`, 10), so it calls
  `claude-session.say`, `interrupt` and `answer` itself; on a `direct: false`
  stream it calls the provider's `sessions` functions of the same name. A turn's
  input goes to `say` as text: an input that is not a JSON array is its own
  text (as the stored message reads it), else the `text` of its ACP text blocks joined with a
  blank line (other blocks are left out of the text until a later slice
  carries them). The owner's `cancel` is `streams.cancel` of the provider
  stream. Each is a request (7), made after the call's last write.
- `push` can fail `full` (the owner stream is at its window: 3, 9.4),
  `unknown-stream` or `not-yours` (the stream ended meanwhile: the kernel drops
  its live stream as if `ended` had come, and closes nothing), or `too-large`
  (an item over 4 MiB, which the 64 KiB chunk of 9.1 rules out: treated as
  `unknown-stream`).
- `close` passes the `outcome` variant, never a text: `closed`, `replaced`,
  `lagging`.
- `timers.after` is asked for at most 30 days (`bad-delay` past it): a later
  deadline is armed at 30 days, and that firing finds nothing due and arms the
  next. The kernel's ids (`s/<session>`, `restart`) are under 128 bytes and its
  timers stay under 256 (7), so `bad-id` and `too-many` are not met.
- `stream-sink.items` gets a batch: the kernel takes its items in `seq`
  order, drops every item at or below the last `seq` it took on that stream
  (`stream/<id>` keeps it; case 7), and answers `defer` for the whole batch
  when any item defers (3): the host then rolls the call back whole. Each
  item's `body` is the provider stream's own line (a `claude` stdout line or a
  host item, 4.5), read by the adapter (9.8, S32 R2); the provider wraps
  nothing.
- **A failed call commits nothing** (S32 R1). On watane's host a call that
  returns an `error` of 2.3 returns normally, and the host commits what it
  wrote; only a trap rolls back. So the WASM wrapper buffers every state write
  of a call and puts them only when the crate's call succeeds; an `error`
  drops them, and with them every host call the crate holds for after the last
  write (requests, timers). The wrapper holds every timer call of a call until
  it commits, in call order after the writes, so a refused call arms nothing
  (not the `restart` of a new generation either). The buffer commits the puts
  and deletes in call order. `stream-sink.items` and `ended` trap on a kernel
  error, as before (3, "Failed batch").
- `stream-sink.ended` of a provider stream passes its `outcome`: anything but
  `closed` ends an active turn `idle {failed {error: provider-ended, cause}}`
  with the outcome's name as `cause` (`no-binary`, `version 2.1.200 < 2.1.295`,
  `mismatch(tools)`, ...), so the console shows why (18.5).

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
`<publisher>/agent`, pinned at build time and compared exactly. A development
build uses `local/agent` for the unpublished kernel (10, R-B3).

### 2.3 Kernel (`mods:agent@0.1.0`)

```wit
interface sessions {                                 // the owner only, by caller.identity()
  start: func(key: string, profile: string, overlay: string, input: string) -> result<string, error>;
  start-in-folder: func(key: string, profile: string, overlay: string, input: string, folder: string) -> result<string, error>;
  profiles: func() -> list<profile-info>;                // the profiles this caller may use (9.1)
  check: func(profile: string, overlay: string) -> list<refusal>;   // by stored profile name, as `start` checks it (9.2)
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
// Children appear in the same owner-scoped list; their links are transcript entries (9.4).
record session-info { id: string, title: string, profile: string, backend: string, workspace: option<string>, turn: string, updated: u64 }
record queued-input { n: u64, input: string }
record profile-info { name: string, backend: string, model: option<string> }
record refusal { field: string, backend: string, model: string, reason: string }
variant error { not-owner, unknown-session, refused(list<refusal>), queue-full, quota, paused(u64), backend(string), unknown-input, unknown-allow, behind(u64), too-large(u64), busy }


interface offers {                                   // a collect (W34): exported by an offering mod, imported by the kernel
  skills: func() -> list<offered-skill>;             // 9.2, `adopted/`
  profiles: func() -> list<offered-profile>;
}
record offered-profile { name: string, description: string, profile: string }   // `profile`: a profile document (9.2), at most 64 KiB

interface manage {                                   // the kernel's page only (9.10): any other caller is `not-owner`
  use sessions.{error};
  record stored-profile { name: string, profile: string, digest: string, offer: option<string>, stale: bool }
  record allowed { profile: string, digest: string, current: bool }   // current: the digest equals the stored profile's
  record client-info { identity: string, allowed: list<allowed>, asking: list<string> }   // `page` first
  record offer-info { name: string, description: string, profile: string, digest: string, adopted-as: list<string> }
  profiles: func(after: option<string>, max: u32) -> result<list<stored-profile>, error>;
  put-profile: func(name: string, profile: string) -> result<string, error>;     // the digest; writes no access
  delete-profile: func(name: string) -> result<_, error>;                        // refused while allowed or asked
  clients: func() -> result<list<client-info>, error>;
  forget: func(client: string) -> result<_, error>;
  allow: func(client: string, profile: string) -> result<_, error>;              // asks the app; returns at once
  withdraw: func(client: string, profile: string) -> result<u32, error>;         // the turns still to end
  offers: func() -> result<list<offer-info>, error>;
  adopt: func(offer: string, digest: string, as: string) -> result<string, error>;   // the digest; writes no access
}
```

`sessions` keeps its signatures; S151 changes what `start`,
`start-in-folder`, `check` and `profiles` read (the pinned entries of 9.10)
and lets `profiles` and `check` add the caller to `clients`. `digest` is the
SHA-256 of a profile's stored bytes, 64 lowercase hex.

`input`, `overlay`, `decision` and `caps` are JSON: ACP content blocks, an
overlay document (9.2), a decision (below), and ACP client capabilities.

`answer`'s `decision` has exactly one of `allow` or `deny`, its value `true`:

| Decision | Meaning |
|---|---|
| `{"allow":true}` | allow this call |
| `{"allow":true,"for":"session"}` | allow it and add its (tool, scope key), shown as `reach` (9.4, 9.6), to the session's allows; refused `{field: "decision"}` for an ask with no `reach` |
| `{"allow":true,"input":{…}}` | allow with a changed input; only for `AskUserQuestion` and `ExitPlanMode` (4.5) |
| `{"deny":true,"reason":"<text>"}` | deny; `reason` is optional, empty when absent |

Any other shape, `{"none":true}` among them, or `for`, `input` or `reason`
with the wrong outcome, is `refused` with one refusal `{field: "decision", reason}`. A
check's `none` exists only in the checked provider's `claude-session.answer`
(2.1, 4.5), never in a client's `answer`. `answer`'s `ask` is
the `id` of the ask entry (9.4; the ask id of the deadlines in 9.3), never the
tool call id. A check (`kind: "check"`, 9.4) is answered only by the checked
provider (10); the owner's `answer` to it is refused, `{field: "ask"}`.
`allow` in `take-back` is the id of an `allow`
entry (9.4). `too-large(u64)` carries the limit in bytes, always (9.3). `busy`: a Claude session's provider stream is not free (9.5), an OpenAI `start`, `say` or interrupt replacement finds no request slot (9.5 Slots), the session holds 64 pending answers (9.3, `pending-answers`), the active index is full at 1 024 sessions (9.3, A3), ; its message names the cap. `read`: 9.4. `read`'s `max` is 1 to 256: `max` 0 is `refused` with one refusal `{field: "max", reason}` (11), and a `max` over 256 reads as 256.

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
| `profile` | the session's kernel profile name (9.2), set by the kernel: `[A-Za-z0-9._-]{1,64}` as host 19.3 takes it; the kernel leaves the field out for a name outside it | nothing: the host uses it only in the workspace consent's words (8, "{profile}"), never as an argument (watane S95) |
| `workspace` | a workspace id the origin client holds (8), or `scratch` for a new scratch workspace (4.6) | cwd |
| `extra-workspaces` | workspace ids, the same rule except that `scratch` is refused | `--add-dir=` each |
| `model` | `[A-Za-z0-9.\-\[\]]{1,64}` | `--model=` |
| `fallback-models` | the same, at most 4 | `--fallback-model=` |
| `effort` | `low`, `medium`, `high`, `xhigh`, `max` | `--effort=` |
| `system-prompt` | `{replace \| append: text}`, at most 64 KiB | `--system-prompt=` / `--append-system-prompt=` |
| `tools` | names from the grant's allowlist (4.2); an absent `tools` and `[]` both mean no built-in tools | `--tools=` |
| `allow`, `deny` | `Name` or `Name(arg)`, `arg` without parentheses, no control character anywhere; Name from the allowlist or `mcp__watane__<tool>`; no comma, space or leading `-` outside the parentheses; an allow `Bash(…)` follows the grammar below | `--allowedTools=`, `--disallowedTools=` each |
| `mode` | `manual`, `accept-edits`, `plan`, `dont-ask` | `--permission-mode=` |
| `ask` | bool | `--permission-prompts=host --permission-prompt-tool=stdio`, or `--permission-prompts=none` |
| `pre-tool-check` | `{timeout-s: 5..120}` | the host's PreToolUse callback in `initialize` (4.5) |
| `agent-handle` | a handle the origin client holds | `--mcp-config=/dev/fd/3` |
| `subagents` | each exactly `{name [a-z0-9-]{1,64}, description, prompt, tools ⊆ tools, model}`; any other key is refused by name; 64 KiB in all (N9, the human, 2026-11-20: subagents stay `--agents=` flags) | `--agents=` |
| `subagent-depth` | 0..3, default 1; 0 also removes `Task` | env `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` |
| `skills` | `{name [a-z0-9-]{1,64}, files}`; paths under the skill folder, no `..`, 1 MiB in all; a `SKILL.md` that starts with `---` has strict frontmatter, below | the run folder's plugin, `--plugin-dir=` |
| `sandbox` | the typed record of 4.2; required with the shell grant | the host's `--settings=`, which always sets `allowUnsandboxedCommands: false` itself: Claude's default is `true`, so "none of" in 4.2 means the key is written `false`, never left out (PLAN S33) |
| `json-schema` | at most 64 KiB, valid JSON Schema | `--json-schema=` |
| `max-turns` | 1..1000 | `--max-turns=` |
| `max-budget-usd` | positive | `--max-budget-usd=` |
| `session` | `{new: uuid}`, `{resume: uuid, at?: uuid, drops-turn?: uuid}`, `{fork: uuid, at?: uuid}` | `--session-id=`, `--resume=`, `--fork-session`, `--resume-session-at=`, `--resume-drops-turn=` |
| `persist` | bool | `--no-session-persistence` when false |
| `partial` | bool | `--include-partial-messages` |

The allow grammar for `Bash` (deny rules are not held to it). An allow
`Bash(<arg>)` is refused unless all of these hold:

- Bare `Bash`, with no parentheses, is refused: it allows any command; so is
  an empty `<arg>` (`Bash()`, `Bash(:*)`).
- `<arg>` has no non-ASCII character and none of `` ` ``, `$`, `{`, `}`, `;`,
  `|`, `&`, `<`, `>`, `'`, `"`, `\`: the shell would read more than a word.
- A colon appears only as one trailing `:*`.
- The first word (split on whitespace, after that `:*` is removed) is no path
  (no `/`), has no `~`, and is made only of `A-Za-z0-9._+-`.
- The first word, lowercased, is not an interpreter or runner: no name
  starting `python`, `pypy`, `node`, `perl`, `ruby`, `php` or `lua` (versioned
  names such as `python3.12`, `nodejs`, `perl5.36.0`, `luajit`), and none of these, because each runs, wraps or schedules
  another command: `.`, `ash`, `at`, `awk`, `bash`, `batch`, `builtin`, `bun`, `bunx`, `busybox`, `byobu`, `capsh`, `chroot`, `chrt`, `command`, `coproc`, `corepack`, `crontab`, `csh`, `dash`, `deno`, `doas`, `dtach`, `elixir`, `enable`, `env`, `eval`, `exec`, `expect`, `fakeroot`, `fc`, `fish`, `flock`, `gawk`, `gdb`, `ionice`, `irb`, `julia`, `ksh`, `lldb`, `ltrace`, `lua`, `mawk`, `mksh`, `nawk`, `nice`, `node`, `nohup`, `npx`, `nsenter`, `numactl`, `osascript`, `parallel`, `perl`, `php`, `pipx`, `pkexec`, `pnpx`, `prlimit`, `pwsh`, `r`, `rbash`, `rscript`, `ruby`, `run0`, `runuser`, `screen`, `script`, `sem`, `setpriv`, `setsid`, `sg`, `sh`, `source`, `ssh`, `stdbuf`, `strace`, `su`, `sudo`, `systemd-run`, `taskset`, `tclsh`, `tcsh`, `time`, `timeout`, `tmux`, `toybox`, `trap`, `ts-node`, `tsx`, `unshare`, `uvx`, `valgrind`, `watch`, `xargs`, `yarnpkg`, `zellij`, `zsh`.
  This list is the rule; the list in `kernel/src/profile.rs` (`RUNNERS`) must
  equal it (a test reads this list). A list of this kind cannot be complete, so a word not on it
  is not thereby safe: the shell grant's sandbox (4.2) is the limit (14, Not
  protected). `find` is
  allowed only with no `*` and no `-exec`, `-execdir`, `-ok` or `-okdir`.
- A package runner (table) needs a second word from its row and no option
  word (starting `-`) after it, and the rule has no `*` or `:*` anywhere
  (fail-closed: an option or a forwarder can name a program to run).

| Runner | Second words allowed |
|---|---|
| `npm` | `install`, `i`, `ci`, `test`, `ls`, `list`, `audit`, `outdated` |
| `pnpm` | `install`, `add`, `test`, `list`, `ls`, `audit`, `outdated` |
| `yarn` | `install`, `add`, `test`, `list`, `audit`, `outdated` (not `ls`: yarn runs a script named `ls`) |
| `uv` | `pip`, `sync`, `lock`, `add`, `remove`, `tree`, `venv` |
| `devbox` | `install`, `add` |

The `skills` frontmatter rule, for each file of a skill named `SKILL.md` in
any case and any folder. Frontmatter starts with `---` and a newline at
byte 0 (leading white space or a byte order mark before `---` is refused). It
ends at a line `---`. Its keys are `name` and `description`, each once; an
indented line continues the key above it and may not contain `:`; any other line is refused. Every
character is printable ASCII (space to `~`). A description has at most
1,024 characters before escaping. Trim the value and each continuation
and count one space between them. Each refusal names its reason.

### 4.2 Grants and tools

| Grant | Built-in tools | Required |
|---|---|---|
| Claude Code (unchecked), `claude-code` | `Read`, `Write`, `Edit`, `Glob`, `Grep`, `NotebookEdit`, `Skill`, `Task`, `TodoWrite` | a workspace |
| Claude Code with shell and web (unchecked), `claude-code-shell` | the above and `Bash`, `WebFetch`, `WebSearch` | a workspace; `sandbox.enabled: true`; writes only in the workspaces; loopback denied only while no allowlisted domain names or resolves to a loopback address (V27, 14); none of `excludedCommands`, `allowUnsandboxedCommands`, `allowUnixSockets`, `allowLocalBinding`; a network allowlist of plain domain names |

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
   item `{"type":"initialized","workspace":"<id>"}` on the `claude-code.start` (and
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
| `control_request` `can_use_tool` | `{"type":"ask","id":<host id>,"tool_use_id":<the request's tool_use_id>,"tool":…,"input":…,"path":<absolute path>?,"expires-at":<ms>}` |
| `control_request` `hook_callback` (PreToolUse) | `{"type":"check","id":<host id>,"tool_use_id":<the request's tool_use_id>,"tool":…,"input":…,"path":<absolute path>?,"expires-at":<ms>}` |
| `control_cancel_request` | `{"type":"ask-resolved","id":…,"by":"cancelled"}` |
| an `answer` the host accepted, whoever called it | `{"type":"ask-resolved","id":…,"by":"answered","outcome":"allow"\|"deny"\|"none"}`: `outcome` is the answer the host sent to `claude`, `none` only for a check (step 5) |
| an ask the host denied at its `expires-at`, or a check it has not answered at its `expires-at` | `{"type":"ask-resolved","id":…,"by":"expired"}` (step 5) |
| a line whose top-level `type` is any watane item type that can share the stream: `ask`, `check`, `ask-resolved`, `initialized`, `wrapped`, `consent-pending`, and the checked provider's `verdict` and `checking` (10); a slice that adds an item type to any of these streams adds it here | never passed through: the host pushes `{"type":"wrapped","line":"<the line, as a JSON string>"}` instead |
| a line that is not valid UTF-8, not exactly one JSON object, or that has a duplicate key at the top level | the same `wrapped` item (a byte that is not UTF-8 becomes U+FFFD in `line`; the kernel keeps this `line` as it is; the `\xNN` form of 9.8 is only for bytes that reach the kernel without the host, as in its fixtures): the host decides on the parsed value, so the kernel and the host never read one line two ways |
| any other line, known or not | the line unchanged |

   **`path`** (watane S100; host 19.3 step 4): an `ask` or `check` of `Read`,
   `Glob`, `Grep`, `Edit`, `Write` or `NotebookEdit` carries `"path"` after
   `input`: the tool's own path input (`file_path` for `Read`, `Edit`, `Write`;
   `path` for `Glob`, `Grep`; `notebook_path` for `NotebookEdit`), resolved by
   the host with its symlinks resolved, absolute. An absent input path means
   the working folder only for `Glob` and `Grep`; for the other four it gives
   no `path`, as does an input that holds another of those keys, a path the
   host cannot resolve, and one its rules refuse. `input` stays as `claude`
   sent it. The kernel keeps `path` on the ask entry (9.4) and computes a file
   key only from it (9.6); a `path` that is not a string is not kept.

   `expires-at` is on the host's clock: for an ask, the time the host pushed
   the item plus 10 minutes, the same instant at which it denies the ask
   (step 5); for a check, the push time plus the hook's `timeout-s`. A
   `control_request` whose `tool_use_id` is missing, not a string, or empty is
   not passed as an ask or check: the host answers it deny (ask) or `deny` (check) at once and pushes
   nothing for it. So no `claude` stdout line can pass as a host item. The
   host's `id` of an `ask` or `check` item is unique on its stream: it is never
   reused, also after its `ask-resolved`. The host builds each `ask`, `check`
   and `ask-resolved` item from its own parsed value and serializes it
   itself; it never copies bytes of a `claude` line into an item. An
   `expired` or `cancelled` item has no `outcome`.

5. Writes to stdin are only these:

| Operation | Written | Rule |
|---|---|---|
| `say` | a `user` message with the content | refused `slash-command` when the first text block starts with `/` after white space |
| `compact` | a `user` message `/compact` | – |
| `answer` to an ask | `control_response` `{behavior: allow, updatedInput}` or `{behavior: deny, message}` | `none` refused; `allow(none)` sends the original input; a changed input only for `AskUserQuestion` and `ExitPlanMode` |
| `answer` to a check | `{}` for `none`; `{hookSpecificOutput: {hookEventName: PreToolUse, permissionDecision: deny, permissionDecisionReason}}` for `deny` | `allow` refused |
| `interrupt` | `control_request` `interrupt` | – |

   After any answer, and when an ask expires (10 minutes from the push of
   the ask item, at its `expires-at`; deny), the host
   pushes `ask-resolved {id, by}`: `by` is `answered` (with the `outcome` it sent) for an answer,
   `expired` for the expiry and `cancelled` for a `control_cancel_request`
   (step 4). The host pushes no other `by`. A second answer is `unknown-ask`.
   The host closes checks too: at a check's `expires-at` (the push time plus
   `timeout-s`, step 4) it pushes `ask-resolved {by: expired}` for a check it
   has not answered and writes nothing to stdin for it (`claude`'s own hook
   timeout, `onFailure: block`, already blocks the tool); an `answer` to it
   after that is `unknown-ask`. Nothing
   else is ever written: no `apply_flag_settings`, `mcp_set_servers`,
   `set_permission_mode`, `updatedPermissions` or second `initialize`.
6. A turn is over when `claude` reports itself idle, not at the first
   `result`.

### 4.6 Folders

- **Run folder** `<home>/runs/<random>/`: the plugin folder `watane-run`.
  Deleted when the stream ends; all of `runs/` is deleted at open, after the
  recorded process groups are killed.
- **Scratch workspace** `<home>/scratch/<random>/`, named by the host, owned
  by the origin client; a mod sees its id. It asks no consent (8): no participant folder is in it.
  As host 19.3 "Folders" states it (watane S99; no WIT change):
  - **Made by the launch.** `workspace: "scratch"` makes a new one when the
    start applies; its id is `scratch:<32 hex>`.
  - **Seen in `initialized`.** The host's `initialized` item names the
    launch's workspace, `{"type":"initialized","workspace":"<id>"}`. The
    kernel keeps the first one a session's provider stream gives in the
    record's `workspace` (9.3), written once (a later one with another id
    is an `unknown` entry).
  - **Its folder, from `system/init`.** The id is not a path. The workspace's
    own folder is the `cwd` of the stream's first `system/init` line: `claude`
    reports its process folder, which the host set to the workspace folder
    (host 19.3 "Folders"), as `getcwd` gives it (symlinks resolved, as the
    host resolves an item's `path`, 4.5 step 4). The kernel keeps it once in
    the record's `folder` (9.3) when it is a plain absolute folder. The
    first `system/init` decides, even one with no usable `cwd`; a line the
    kernel cannot read as one JSON object (not UTF-8, not JSON, a key twice:
    an `unknown` entry) may be an init however it is spelled, so before the
    first init it decides too, with no folder (fail closed: no file keys): the record's
    `init-seen` is then set, and a later `system/init` with another `cwd`
    changes nothing; neither writes an entry. This is the one place the
    kernel learns the folder (9.6, 14).
  - **Held across a resume.** A session whose record has a `workspace` launches
    with `workspace: "<that id>"`; before initialization it relaunches with the
    original picked folder, or with `scratch` for a scratch session.
  - **Lifetime.** It outlives its stream and the session's `delete`; the host
    deletes it with its owner mod (host 19.3).
- **Picked workspace:** `start-in-folder` takes an absolute path for a Claude profile.
  It refuses paths containing control characters with `refused [{field: "folder"}]`.
  The kernel passes it to the same provider as a scratch launch. The host
  resolves and validates it before asking (host 19.3, 19.7), and
  `initialized.workspace` is its canonical path. `list` returns the picked
  path for display, canonical once available. The requested path never grants
  file reach. Canonical, symlinks resolved. Refused: `/`, the home
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
- **Timers.** Per mod: ids of at most 128 bytes (else `bad-id`), at most 256
  live (else `too-many`), delays up to 30 days (2 592 000 000 ms; a longer
  one is refused `bad-delay`, watane S95), stored in host state (`timers/<mod>/<id>` → due time). A due
  timer fires as a top-level call of the mod's `timer-sink.fired(id)`; one
  due during downtime fires soon after start. `after` with a live id
  replaces it. The kernel asks for at most 30 days (2.1, "The kernel's side
  of the seam").
- **Kernel timers.** The kernel holds at most one host timer per session,
  `s/<session id>`, due at the session's earliest deadline (9.3, 9.5, 9.6).
  Live session timers are at most 240 kernel-wide, 16 of the host's 256 being
  kept for the kernel's own use; what happens at the cap is in 9.3. The
  kernel never asks a delay over 30 days, so it never meets `bad-delay`: a
  deadline further ahead sets the timer 30 days ahead, and that firing,
  which finds no deadline due, sets the timer again for the earliest one
  (9.3, "Deadlines and the session timer").
- **A call commits whole.** The host-state writes of a call (host 7) commit
  together when its call chain commits, or not at all; a deferred or failed
  call leaves no write behind. So a failed `append` (9.4) leaves no chunk
  written and nothing charged to the owner (9.3, Quota).
- **Rolled back.** `claude-session.say`, `answer`, `interrupt` and `compact`
  are requests too: a call that is rolled back (deferred or failed, 3) sends
  nothing to `claude`, and its `timers.after` and `timers.cancel` are dropped
  with it and made again on redelivery.

## 8. Consent (H6)

| Consent | Asked when | Kept for | Words |
|---|---|---|---|
| Workspace | a launch names a folder new for the origin client | as watane host 19.7 keeps it (watane Q38, open: whether the key also names the provider) | "Let {origin client} have agents work in {folder} ({profile}, through {provider})?" |
| Account | an `https-stream` with `account: "openai"` new for the origin client (the `judge` account asks none: entering its key is the consent, 5) | as watane host 19.7 keeps it (watane Q38, open: whether the key also names the provider) | "Let {origin client} use your ChatGPT plan?" |
| Toolset | `agent-tools.open` with a toolset new for the origin client | (origin client, toolset) | "Let {origin client}'s agents call: {functions}?" |
| Profile access (S151, proposed) | the kernel's `manage.allow` (9.10) through `agent-access.ask` (2.1), for a client and a version of a profile the client has no entry for | by the kernel, as a pinned entry of `access/<client>` (9.10); the host keeps nothing | "Let {client} start agents with {kernel}'s profile "{profile}"? It can: {reach}" ({client} and {kernel} the names watane shows for those mods, "the agent console" for `page`; {reach} as plain text, 9.10) |

- **Origin client:** the page's mod for a page call; the mod of the handle
  for an agent's call; for an item, job or timer, the origin of the call that
  opened it. The host carries it with the chain.
- Stored with the grants (host 4.4); shown and withdrawn in the manager like
  grants. A withdrawal ends dependent streams with `gone` and closes
  dependent handles (host 19.7).
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
- **Profile access** is not stored with the grants: the participant sees and
  withdraws it in the kernel's page (AGT-F-57), and only the participant's
  answer in the app writes it (9.10).

## 9. The kernel (M2)

### 9.1 State

The kernel's state (host 7: key-value, 4 MiB per value, no prefix scan):

| Key | Value |
|---|---|
| `profile/<name>` | a profile (9.2) |
| `access/<identity>` | since S151, a JSON array of pinned entries `{"profile", "digest"}`, at most 64, written only as 9.10 says; a name is allowed only while its entry's digest is the stored profile's and the profile is not stale (9.10), and any other value, an array of names included, is unreadable. The rest of this cell names an entry's profile as "the profile names". Before S151: a JSON array of the profile names this client may use. `<identity>` is the caller's identity as `caller.identity()` gives it (2.1). An absent or unreadable value (since S151: not a JSON array of pinned entries, 9.10) allows none: fail closed, with one exception, the console's first run (S35): for the identity `page` only, an absent `access/page` allows the built-in profile `scratch` and nothing else (an unreadable one still allows none). `profile/scratch` absent reads as the built-in `{"backend":"claude","partial":true,"ask":true}`: Claude's default model, no built-in tools, mode `manual` with its asks sent to the owner (`ask: true`, 4.1: without it the host denies every `can_use_tool` at once and pushes no ask, host 19.3; S36), no allow rule, in a scratch workspace by default (4.6), which is less than any profile the participant can make: with no built-in tool and no `agent-handle`, `claude` has no tool to ask about, and every allow is the owner's own answer to one call. The first-run allowance covers only that built-in: a stored `profile/scratch` is a profile like any other and needs `access/page` to list it (S35 adversary), and a stored `access/page` replaces the allowance `start` and `check` (2.3) check it before they read `profile/<name>` and refuse a name not listed with one refusal `{field: "profile"}` and one reason, whether or not the profile exists, so a client learns nothing about profiles it may not use (a listed name with no profile is refused `{field: "profile", reason: "no such profile"}`; `check` takes the profile by name, and an OpenAI profile without a model is refused `{field: "model"}`). `profiles()` lists the names the caller may use in this key's order, at most 64, each with the `backend` (default `claude`) and `model` of its stored profile, a name with no stored profile left out; it reads this key and at most 64 `profile/` keys, and for `page` with no `access/page` it lists the built-in `scratch`. A child session's profile needs no separate entry (9.7). Each call that gives a session more work reads it again (9.7, "Revoking") |
| `adopted/<offer id>` | `{content hash, content}`: an adopted skill offer (9.2). An adopted profile offer is a `profile/<name>` with a link in `profiles` (9.10) |
| `profiles` | the profile manager's index (9.10): a JSON array of at most 256 `{name, offer?: {name, digest}, stale?}`, in the order stored; written only by `manage` |
| `clients` | the identities the participant may allow (9.10): a JSON array of at most 64, in the order of their first `profiles` or `check` call; `page` and `agent:` identities never in it |
| `access-ask/<stream>` | an open access question (9.10): `{client, profile, digest}`; deleted at its stream's `ended` |
| `access-asks` | the stream ids of the open access questions, at most 4 (9.10) |
| `key/<owner>/<key>` | `<owner>` is written percent-encoded as one path segment (`%` to `%25`, then `/` to `%2F`), because identities contain `/` (2.1) and the raw form is not one key per (owner, key). The value is `{call, owner, session?, value}`: `call` is `start`, `say` or `answer`; `session` the session a `say` or `answer` acted on; `value` the result returned. The result of the first call with that key that changed something; kept after a delete. A call that fails and changes nothing (any error of 2.3: `busy`, `queue-full`, `quota`, `too-large`, `refused`, `paused`, ...) is not kept, so a retry with the same key can succeed. The same key with another call kind, owner or session is refused, one refusal `{field: "key"}`; a kept result is never overwritten |
| `session/<id>` | the session record (9.3) |
| `page-origin` | the installed mod identity reported by the reserved page-origin timer (9.5); absent until the host supplies it |
| `next-id` | the decimal number of the next session id `s<n>` (the WASM wrapper's `new_id`, S32-2): one write per new id; absent reads as 1 |
| `owner/<owner>/<n>` | page n of the owner's session ids, a JSON array of at most 256; `<owner>` percent-encoded as in `key/` above. `start` adds the id to the first page with room (by the page counts of `quota/<owner>`), opening page `pages.len()` when none has room; the record's `owner-page` names its page, so `delete` reads only that page; an emptied page is deleted and its count stays 0. `list` reads the pages in order. The per-owner session cap (9.3 Quota, default 64) bounds the pages: one page by default |
| `queue/<id>` | `{next, inputs: [{n, input}]}`: the session's queued inputs (9.3), apart from the record; `input` is the input JSON as a string, `next` the number the next input takes. An emptied queue keeps `next`, so a number is never used twice in one session |
| `entries/<id>/<n>` | chunk n of the transcript: at most 256 entries and at most 64 KiB, or exactly one entry when that entry is over 64 KiB (an entry is at most 256 KiB, 9.4); an append that would take the last chunk past 64 KiB starts a new chunk, so an append rewrites at most 64 KiB plus the entry and earlier chunks are never rewritten; the session record's `chunks` lists the first index of each, so a read starts at the right chunk without reading the ones before |
| `wire/<id>/<n>` | chunk n of the OpenAI wire history; the loop state's `wire-chunks` (in `loop/<id>`) lists the first wire index of each, so a request finds the chunk holding `active` (9.5) and reads none before it |
| `loop/<id>` | the OpenAI loop state (9.5), with `active`, `wire-chunks` and the rendered skill listing |
| `skills/<id>/<name>` | raw `SKILL.md` bytes pinned at OpenAI root start or child admission, charged to the session and read only to execute an allowed skill call (11) |
| `skill-revisions/<id>` | a session's skill names and SHA-256 file digests, pinned alongside the files for admission and deletion (9.7, 11) |
| `quota/<owner>` | `{bytes, sessions, pages}`: the owner's bytes and live session count, the bytes counting every key the owner causes (9.3, Quota), and `pages` the entry count of each `owner/<owner>/<n>` page; later its live session timers (9.3) and its slot counts per kind (9.5), the slot counts tagged with the generation of `slots` (9.3): counts of an older boot read as zero, so the restart pass need not find every owner. Absent reads as all zero |
| `quota` | `{bytes, sessions}`: the kernel-wide account, bytes and sessions of all owners (9.3); absent reads as zero |
| `stream/<id>` | `{session, kind, last?}` (`kind`: `provider`, `request`, `job`, `owner`, or `access` with no `session`: an access question, 9.10), written in the call that opens or receives the stream, deleted on its `ended` (9.4). `last` is the last item `seq` an items call took on a provider stream (2.1, case 7); absent is 0; one write per batch |
| `adapter/<stream>` | the Claude adapter's state for a provider stream (9.8, "Adapter state"), written with `stream/<id>`'s `last` and deleted with it; counted in the session's `bytes` |
| `live/<session>` | the id of the session's live owner stream: written by `attach`, deleted when it ends (9.4) |
| `active/<n>`, n 0..3 | A1: page n of the sessions whose turn is `running`, `waiting` or `paused`, each entry the pair `[id, boot]` (not an object, so that 256 entries with 64-byte ids fit the 24 KiB page; `boot`: the generation of the session's last restart step, or of the call that added it). At most 256 entries per page, so at most 1 024 listed sessions; a page is at most 24 KiB (a session id is at most 64 bytes, 9.3). The record's `active-page` names the page that holds the session while it is listed. Kept by every turn change that enters or leaves the index (9.3, A2); it is what the restart pass reads in place of a prefix scan |
| `slots` | `{provider, request, job: counts; timers; boot; active: four page counts; restart: the pass's page cursor; head, tail; drain-owed}`: the stream slots in use, the live session timers (9.3), the generation (9.3), the entries of each `active/` page, the page where the restart pass goes on, and the ends of the waiting FIFO and whether a timer left an ordinary call a drain (9.5) |
| `slots/wait/<n>` | `{session, boot}`: a session waiting for a slot, with the generation of the call that queued it; an entry of an older boot is skipped (9.5) |

### 9.2 Profiles and overlays

A profile holds the launch fields of 4.1 that are not chosen per session
(`workspace`, `extra-workspaces`, `agent-handle` and `session` are; `profile`
is the kernel's, its own name), plus
`backend` (`claude` or `openai`), `toolsets`, `delegates` (profile names),
`max-tool-errors`, `max-tokens`, `ask-expiry` and `subagent-limits`.
Stage F accepts `delegates` and `subagent-limits` only on OpenAI profiles.
`subagent-limits` (it limits OpenAI child sessions, not Claude subagents, 9.7) is an object with optional `live` (1..20, default 4)
and `depth` (only 1, the default). Depth counts edges from the root. Other
values and unknown keys are refused by field. Neither field can be set
by an overlay.

Those four per-session fields are refused in a profile (`refused`, the field
named). `ask-expiry` is in seconds, a whole number from 0 up; Claude accepts
only 600 (11). `max-tool-errors` is 1..4294967295, `max-tokens` a whole number
from 1 up.

An overlay holds exactly these keys:

| Key | Effect |
|---|---|
| `system-prompt-append` | text added after the profile's prompt |
| `skills+` | skills added; one named like an existing skill is refused |
| `tools-` | tools removed, also from each subagent's `tools` |
| `deny+` | deny rules added |
| `max-turns` (1..1000), `max-tool-errors` (1..4294967295), `max-tokens`, `max-budget-usd`, `subagent-depth` (0..3; the profile's value defaults to 1) | each only lowered: a higher value is refused, naming the profile's value; where the profile sets none, the overlay's value stands |

Every other key is refused as `overlay.<key>`. The widening keys are refused
by name: `tools` and `tools+` (adds tools), `allow` and `allow+` (adds allow
rules), `workspace`, `extra-workspaces` and `workspaces+` (adds workspaces),
`subagents` and `subagents+` (adds subagent profiles), `mode` (changes the
permission mode), `system-prompt` (replaces the instructions); any other key
is "not an overlay field".

No profile field reads workspace instructions (`AGENTS.md`, `CLAUDE.md`)
(N7, the human, 2026-11-20); an opt-in profile field is later
(`FUNCTIONAL.md` section 5).

Compilation: profile, then adopted offers it names, then the overlay; then the
refuse table (11) for the backend and model (a Claude profile without `model` is checked with the model `"default"`, `claude`'s default model, and launches with no `--model=`; an OpenAI profile without `model` is refused `{field: "model"}`, since OpenAI has no default model to check `effort` and `json-schema` against). Every refusal is returned at
once; `check` runs only this step.

### 9.3 Sessions

Session record: `id` (made by the kernel, at most 64 bytes), `owner`, `key`, `profile`, `profile-digest` (the digest of the profile version `start` checked, 9.10; absent for the built-in `scratch`), `backend`, `title`,
`handle`, `workspace`, `requested-folder` (the launch path of a picked folder, for display and retry before initialization), `folder` (the workspace's own folder: the `cwd` of the provider stream's first `system/init`, kept once when it is a plain absolute folder, 4.6; absent until then), `init-seen` (true once the first `system/init` was read: only that one sets `folder`, S36 r1), `claude-uuid`, `parent?`, `task-call?` (on a child: the root's `task` call id while its delegated first turn is outstanding, 9.7), `children`, `turn`,
`boot` (the generation of the session's last restart step, or of its creation; "Restart" below), `owner-page` (the `owner/<owner>/<n>` page that lists it, 9.1), `bytes` (the bytes of the session's keys other than its record, 9.3 Quota, so `delete` frees them without reading them), `active-page` (the `active/` page that lists the session, absent when it is not listed; 9.1),
`allows`
(session allows `{id, tool, key}`, 9.6), `owed-idles` (the `idle` lines `claude` still owes for turns `interrupt` ended, below; absent when 0), `pending-answers` (Claude asks the kernel answered itself that the host has not yet closed: `{ask id: {i, toolCallId, kind, outcome, by, for?, reason?, input?}}`, the pending answers of 9.8 rule 4; `toolCallId` is the ask's call, kept so a later ask on the call is compared without reading every ask (9.8); `input` is the changed input an `allow` carried (2.3), shown in the decision entry and not counted in the 1 KiB; `i` and `kind` are the ask entry's index and its `ask` or `check`, so the decision entry can repeat the ask, and follow the kind rules of 9.8 rule 4, after a restart without the adapter; kept in the record so they survive a restart, charged as control records, 9.3),
`chunks` (the first index of each entry chunk, 9.1), `streams` (the ids of the session's open streams, at most 10: 1 provider or 1 request, 8 jobs, 1 owner; 9.4, 9.5), `deadlines` (each `{what, at}`, at most 64; below), `open` (the open asks and checks, each `{id, kind, i, expires-at?}`: the ask id, `kind` `ask` or `check`, the index `i` of its ask entry (9.4), and that entry's `expires-at` when it has one; an open ask leaves `open` when it is decided or closed. A decision entry can repeat its ask (9.4) after the adapter's memory is gone (a restart), and `waiting.expires-at`, the earliest open ask's, is recomputed without reading entries), `next-index`, `created`, `updated`. The queue is not in the record: it lives
in `queue/<id>` (9.1), at most 32 inputs, each numbered from 1 per session.

**Sizes.** Every input (`start`, `say`, an `interrupt` replacement) is at
most 256 KiB serialized, else `too-large(262144)`. The queue holds at most 32
inputs and 2 MiB in all, counted in stored bytes: each input as the JSON string the queue stores (escapes included, without its quotes); past either, `queue-full`. So `queue/<id>`, the `queue` push and
`queued` carry at most 2 MiB of inputs, under the 4 MiB item (3). The record is at most 512 KiB; its variable fields:

| Field | Cap | Past it |
|---|---|---|
| `key` (a client key, also in `key/<owner>/<key>`) | 256 bytes | `too-large(256)` |
| `title` | 256 bytes | `rename` refuses `too-large(256)` |
| `requested-folder` | 4096 bytes | `start-in-folder` refuses `too-large(4096)` |
| `children` | live delegation only, at most 20 child ids (the originating `toolCallId` is the child's `task-call`, and the root's open call is `child {session}` in its loop state, 9.5), charged to the record byte cap; a finished child leaves the list, its id stays in the parent's transcript (9.7) | – |
| `allows` | at most 64 session allows and 64 KiB in all, each at most 1 KiB (its tool and key) | an answer `for: session` is refused `too-large(65536)`; the ask stays open |
| `chunks` | 16 384 chunk starts, 8 bytes each (u64), so 128 KiB of the 512 KiB record | the session is at its quota: the turn ends `stopped {limit: quota}` and new input is refused `quota` |
| `streams` | 10 ids | – (the slots of 9.5 allow no more; an owner stream that `attach` replaces leaves `streams` in that `attach`, not at its `ended`) |
| `live` | the id `live/<session>` holds (9.1), or absent: the session's live owner stream, kept in the record too so a call that has the record pushes without reading that key (an item call that only forwards a delta has no record and reads the key, 9.4). `attach` sets both, and the stream's end, `lagging`, `replaced`, R4 and `delete` clear both in the same call | – |
| `overlay` | the overlay `start` was given, kept when it narrows anything, so a later launch (a new provider stream after the idle release, an ended stream or a restart) narrows as the first did (4.1, 9.2; S35). A later launch is compiled and checked as `start` is (9.2 compilation and the refuse table, `check(profile, kept overlay)`): any refusal refuses the call that needed it (`say`, `interrupt` with a replacement) `refused(<the refusals>)`, and it writes nothing and starts no stream (S35 adversary) | the record's 512 KiB: `start` is refused `too-large` |
| `deadlines` | 64 | – (an open ask past the 61st is denied `timers`, below) |
| `open` | one per open ask or check, with `{id, kind, i, toolCallId, expires-at?}`. An ask with a kernel deadline (the OpenAI loop's, `ask-expiry`) is bounded by `deadlines` (a 62nd is denied `timers`); a Claude ask or check takes no kernel deadline. At most 64 open asks and checks, the pending answers counted in (`open` + `pending-answers` <= 64); past it the ask is denied at once, `by: kernel`, reason `busy`, its ask and decision entries written in that call and a `Request::Answer` deny sent, so `cancel`, R4 and `ended` write at most 64 decisions in one call | – |
| `pending-answers` | 64 (one per Claude ask the kernel answered that the host has not closed), each at most 1 KiB without `input` (the decision's input is bounded by the record's 512 KiB: a pending allow that would push the record past 512 KiB is refused `too-large(524288)` and writes nothing) | the kernel sends no further answer to a Claude ask of the session until one closes: the owner's `answer` is refused `busy` (2.3), and a session allow or the "no owner stream" deny leaves the ask open for the host to close at its `expires-at` (4.5) |

| Turn state | Meaning |
|---|---|
| `running {activity?}` | a turn is under way; `activity` while it lasts: `checking {on}` (the checked provider asks its judge, 10), `compacting`, `consent {what}` (a request waits for the participant in the app, 8). The activity is live only: pushed as a `turn` update and kept out of the record and entries. Pending consent joins the provider stream cursor write so attachment during a wait can see it. `initialized` clears it |
| `waiting {asks}` | an ask is open |
| `paused {until}` | a usage limit, or retries ran out; the `resume` deadline resumes it (below) |
| `idle {stop}` | `completed`, `interrupted`, `stopped {limit}` (names the limit; `stopped {limit: busy}` when an OpenAI turn's later request finds no request slot, 9.5 Slots), `failed {error}` (`failed {timers}` when a retry finds no timer slot, below) |
| `unknown` | written by the restart step ("Restart", R4) for a Claude session that was `running` or `waiting` when watane stopped, and by nothing else; it ends with the next `say` (R5), which resumes the Claude session |
| `resume-lost` | `claude` refused a resume; the next `say` starts a new Claude session seeded with a summary |

The turn state is this JSON, one line per state:

```json
{"state":"running","since":1763600000000,"activity":{"kind":"checking","on":"<ask id>"}}
{"state":"running","since":1763600000000,"activity":{"kind":"compacting"}}
{"state":"running","since":1763600000000,"activity":{"kind":"consent","what":"<text>"}}
{"state":"waiting","asks":["<ask id>"],"expires-at":1763600060000}
{"state":"paused","until":1763603600000}
{"state":"idle","stop":{"kind":"completed"},"seconds":12,"tools":3,"tokens":5400}
{"state":"idle","stop":{"kind":"stopped","limit":"<limit>"}}
{"state":"idle","stop":{"kind":"failed","error":"<error>"}}
{"state":"unknown"}
{"state":"resume-lost"}
```

`activity` is optional and live only; `checking` has `on` the ask id, or
`"input"` for a check of the input. `asks` and `expires-at` of `waiting` count asks
(`kind: "ask"`) only, not checks; `expires-at` is the earliest open ask's. `until` of `paused` is absent for a pause with no time. `stop.kind`
`interrupted` is as `completed`. `seconds`, `tools` and `tokens` are the turn's
wall time, tool calls and tokens, for the footer of 18.3; each is optional.
The body of a `turn` entry (9.4) and the `state` of a `turn` item (9.4, "The
wire") are this JSON; the entry has no `activity`.

- `start` and `say` and `answer` check `key/<owner>/<key>` first: the key kept for another call kind, owner or session is refused, one refusal `{field: "key"}`, and a kept result is never overwritten (9.1).
- An input whose first text block starts with `/` after white space (4.5's rule and its white space, host 19.3) is refused at `start`, `say` and an `interrupt` replacement, one refusal `{field: "input", reason: "slash-command"}`, before any write: the host would refuse the write to `claude` after the call committed, and the turn would never end (S35 user-sim, case 11).
- `say` while `running` or `waiting` queues; `queue-full` past 32 inputs or 2 MiB. On `paused` with no `until`, `say` resumes (starts the turn); on a timed pause it queues. `say` reads `access/` again, whether it starts a turn or queues (9.7, "Revoking"). `queued`
  lists the queue; `unqueue` removes one input not yet taken, and refuses
  (`unknown-input`) one that is gone. Each change pushes `{"type":"queue",
  "inputs":[…]}` under `_meta.watane`.
- `interrupt` ends the turn (`idle {interrupted}`) in the call, then starts the
  replacement if given. `claude` ends the interrupted turn later with a `result` and an
  `idle` of its own; the record counts these in `owed-idles` (one per `interrupt` sent to a
  provider stream, zeroed when the session's provider stream is opened or released), and each
  such `idle` ends nothing (no second `turn` entry, and a replacement's turn runs on; S36's
  host run) (a replacement reads `access/` again, 9.7, "Revoking";
  on `unknown` the replacement starts first, queued inputs waiting, and resumes as R5 does). `cancel` ends the session's stream and handle, closes
  each open ask or check (one with a pending answer gets that answer as its decision,
  9.8 rule 4; an open one is denied, `by: kernel`), cancels children (9.7). `delete` runs no restart step (R6), so a session can always be
  deleted: it removes the entries, the queue (`queue/<id>`) and wire history,
  keeps the keys, and sends `cancel` once after its last write (the host ends the
  stream and handle and closes the open asks, 4.5; the kernel writes no decision
  entry for them). A `delete` that fails sends nothing.
- A provider stream's `ended` while the turn is `running` or `waiting` closes each open or pending ask (9.8 rule 4) and ends the turn `idle {failed {error: provider-ended}}` in the same call: one `turn` entry (also past quota, since it ends an active turn), no `idle` deadline (the stream is gone), and no queued input starts; the next `say` starts a turn, the oldest queued input first, and resumes the Claude session (as after the idle release; `persist: false`: a fresh session). A client offers Try again (18.5). On any other turn, `ended` changes no turn.
- **Deadlines and the session timer.** The kernel holds one host timer per
  session, `s/<session id>` (7), due at the session's earliest deadline. A
  deadline is `{what, at}`, `what` one of `ask <ask id>` (9.6, `ask-expiry`),
  `retry` and `resume` (9.5), `idle` (9.5, the 10-minute release); at most 64
  (one per open ask plus the three others: a 62nd open ask with a deadline is
  denied `timers`). When the timer fires, the kernel handles every due
  deadline and sets the timer for the next one, or cancels it; a session
  whose deadlines are all gone frees its timer. OpenAI firings are subject
  to the owner check in 9.5. Live session timers are at
  most 240 kernel-wide (the host allows 256 per mod, 7; 16 are kept for the
  kernel's own use); the count is in `slots` (9.1) beside the stream counts
  and, unlike the stream counts, is not reset at a restart: timers are kept across restarts (7), so the count stays true without a scan. One owner holds at most 120 of the 240 (half, as the slots of 9.5); a child session counts against its root's owner (as 9.5). At the owner's cap the same four outcomes follow as at the kernel's cap.
  A session that needs a timer at the cap (and at the host's `too-many`, if it
  ever comes) gets, by deadline: an ask, denied at once, reason `timers`; a
  retry, the turn ends `idle {failed {timers}}`; a usage limit, `paused` with
  no time (the next `say` resumes it, as after retries ran out); an idle
  release, the provider stream released at once. Nothing waits for a timer
  slot. The checked provider's judge timers (10) are the provider's own 256,
  for at most 8 sessions' streams: unchanged. As built (S26): the `idle`
  deadline (`now` + 10 minutes) is set when a Claude turn moves into `idle`
  while the session holds a live provider stream (not by `cancel`, which
  releases at once, nor by an end that starts the next queued input), and
  any call that starts a turn removes it. The timer and the counts
  (`slots.timers`, `quota/<owner>` `timers`) follow the record: a session
  holds a counted timer while its `deadlines` is not empty. A firing with
  no due deadline changes nothing but the timer: it sets it again for the
  earliest deadline (a deadline over 30 days ahead, 7). Until stage D, a due
  deadline of a later stage (`ask`, `retry`, `resume`) stays and keeps its count but arms no
  timer: the timer is set for the earliest deadline still in the future. A session firing cannot fail, as the restart
  firing (R2): one whose restart step (R6) fails still sets the timer again (for the earliest
  deadline in the future, else after the idle delay) and charges what the step wrote, and a firing
  charges what it wrote past the reserve too; the next call on the session does its step.
- **Quota.** An owner's bytes (`quota/<owner>`) count every key the owner
  causes: session records, `queue/`, entries, wire chunks, loop state, skill snapshots,
  `stream/`, `live/` and its kept `key/<owner>/<key>` results (kept after a
  delete, so they stay charged). The kernel also keeps one account for all
  owners in `quota` (9.1): bytes and sessions. The kernel's state is at most
  256 MiB (host 7); 32 MiB of it is reserved for control records (session
  records, turn states, `slots`, `quota`, `stream/`, `live/` and the entries
  that record a stop), so owners together hold at most 224 MiB of the rest. The reserve is headroom, not a second account: every write is still charged to the owner that causes it, and the 224 MiB account counts them all; the reserve only lets the writes named next go on past it.
  Past the kernel's 224 MiB account (or an owner's quota) only these writes go on, into the reserve: the entry and turn state that record a stop, ask decisions (including denials by expiry), `stream/` and `live/` keys of an `attach`, slot and timer counts, and every `delete`. `rename`, a session allow (`for: session`) and new input are refused `quota`; so is everything else past the account.
  Past an owner's 64 sessions or 64 MiB (kernel settings by default; the owner's session ids live on as many `owner/<owner>/<n>` pages as the cap needs, 9.1), or past
  the kernel's 224 MiB: new sessions and inputs are refused `quota`, and a
  running turn ends `stopped {limit: quota}`, written from the reserve. Only a
  `delete` frees bytes (kept keys stay). The 64 MiB per owner stays a kernel
  setting; with the 224 MiB account it can be set higher without one owner
  taking the reserve. As built (S25):
  - *Bytes.* A key costs its name's bytes plus its value's. Charged to the
    owner of the session a call acts on (the caller for `start`): `session/`,
    `queue/`, `entries/`, `wire/`, `loop/`, `skills/`, `skill-revisions/`, `stream/`, `live/` of its sessions,
    its `key/<owner>/` results and its `owner/<owner>/` pages. Not charged
    (the kernel's own): `slots`, `slots/wait/`, `active/`, `quota`,
    `quota/<owner>`, `profile/`, `access/`, `adopted/`. A call adds the change
    in bytes of each charged key it writes or deletes to `quota/<owner>` and
    `quota`, and that of a session key other than the record to the record's
    `bytes`; a call that changes no charged byte writes neither account. A
    `delete` frees the record and its `bytes` without reading the chunks.
  - *Past.* An owner is past its quota when its sessions are at the cap (for a
    new session) or its bytes are at or over its cap; the account is past when
    its bytes are at or over 224 MiB. Each is read before the call writes (as built (S28):
    the flag lives only in the accounts the call read, never between calls), so
    a call that starts below a cap may end over it by its own writes (at most
    one input and what it records); the next call finds it past. No call
    takes the account past 256 MiB: one that would fails `quota` and, as
    every failed call, commits nothing (7). The exceptions cannot fail at
    the ceiling, since each ends a turn or closes a session's work and frees
    what it holds: a firing of the restart pass, a provider stream's
    `ended`, `cancel`, `interrupt` without a replacement, and a provider
    item while past (among them the one that ends the turn `stopped {limit:
    quota}` and sends `Request::Interrupt`). As built (S29): what they
    wrote, even a restart step that failed, is charged to both accounts and
    the record's `bytes`, and what they write is bounded in bytes: at most 64
    decisions and one `turn` entry beyond the record and the keys they
    delete, and every decision written while past is short (9.4), so once
    past a call writes at most one full decision (the one that crossed, at
    most the size of its ask entry) and the rest short (together within the
    record's 512 KiB plus 16 KiB, as they repeat only fields the record
    holds). A provider item while past stores nothing else (below). The 32
    MiB reserve covers what a call that starts below the account writes past
    it; past the account no call starts new work, so what calls write after
    that is bounded by the asks and active turns open when it crossed.
  - *Refused `quota` when past:* `start`, `say` (whether it starts a turn or
    queues), an `interrupt` replacement, `rename`, an `answer` with `for:
    session`. Not refused: `answer` without `for: session`, `interrupt`
    without a replacement, `cancel`, `unqueue`, `delete`, `read`, `list`,
    `queued`, the restart step, timers. The refusal's text names the cap for an
    owner's own caps (`64 sessions per owner`, `64 MiB per owner`); when the cause is
    the kernel's 224 MiB account or the state ceiling it reads "this mod's agent
    sessions are at their limit", which tells an owner nothing of other owners (14).
  - *Any call when past* (the owner or the account; a provider item, an owner's
    `answer`, `interrupt`, `cancel`, `unqueue`, the restart step, a timer, `attach`'s step): an ask decision
    and the host's `ask-resolved` are stored (short, 9.4); a turn change is written as an entry only when it ends an active turn (so a turn gets at most
    one `turn` entry past quota; a provider `Turn` that would change the record's turn
    without ending it is dropped: no entry, the record's turn kept, no push; an ask
    that closes or `close_all` writes no entry, while the record's turn follows (a
    `waiting` turn's asks shrink, then it is `running`) and the `turn` push goes out); a new
    ask is not stored and is denied at once (`Request::Answer` deny, reason
    `quota`); any other entry is not stored. When the session's turn is
    active, the first item not stored ends it `stopped {limit: quota}` (its `turn`
    entry from the reserve) and sends `Request::Interrupt`; queued inputs stay
    queued and no queued input starts while past. The same holds at the
    chunk cap (16 384, 9.3 Sizes): an entry that would take the session past
    it ends the turn `stopped {limit: quota}`, the room for that `turn` entry
    kept.
- **The active index.** A2: a call whose end state moves a session from
  `idle`, `unknown` or `resume-lost` (or a new session) into `running`,
  `waiting` or `paused` adds it to the first `active/` page with room (9.1); a
  call that moves it out removes it from its `active-page`. Each such change
  reads and writes `slots` and one page (at most 24 KiB each way) and the
  record it already writes. A change among `running`, `waiting` and `paused`,
  or a turn that ends and starts the next queued input in the same call,
  writes no page. `delete` removes the session if listed. A3: at 1 024 listed
  sessions, a `start` or `say` that would add one is refused `busy` (2.3, 12;
  its message names the cap); it never waits. At the default 64 sessions
  per owner (below), 16 owners fill it: an accepted limit, as the slots of 9.5.
- **Restart.** Streams are memory only (3), so a restart leaves turns that no
  process drives. The kernel finds them without a prefix scan: it lists
  nothing, it reads `active/` pages.
  - **R1, detection.** Every top-level call except an item call on a stream
    (`stream-sink.items`, `ended`, `stream-producer.ready`, `ended`; those
    streams were opened in this boot) reads `slots` and calls `host-info.generation()`
    (2.1). When they differ, that call first starts the pass: it sets
    `slots.boot`, zeroes the stream slot counts (9.5; `timers` stays, 7), sets
    `slots.restart` to page 0, and sets the kernel's own timer `restart` (one
    of the 16 kept for the kernel, 7) with delay 0. It does no session's step
    itself unless it reads that session's record (R6).
  - **R2, the pass.** Each firing of `restart` reads `active/` pages from
    `slots.restart` on (at most the 4 pages, 96 KiB) until it finds an entry
    whose `boot` is older, does that one session's restart step, keeps the
    cursor and sets `restart` again with delay 0; a firing that finds none
    cancels it. A firing whose session step fails marks that page entry current and goes on; R6 does the session's step when a call next reads its record. So the pass takes at most `A + 1` calls for `A` listed
    sessions, each reading at most 96 KiB of pages plus one session's step,
    and it lists no keys: no prefix scan. With no call after a start, nothing
    runs until one comes (a client's call or a due timer).
  - **R3, the step's bound.** A session's restart step reads and writes no
    more than an `interrupt` of that session or one step of 9.5 (the one-call
    bound of 17), whichever applies, plus its page. A second
    restart during the pass starts a new pass from page 0; entries of any older
    boot are stepped then.
  - **R4, the restart step.** It deletes the session's `stream/` keys and
    `live/<session>` of older boots and drops those ids from `streams` (the
    stale keys of 9.4), sets the record's `boot` and its page entry's `boot`,
    then by backend and state:

    | Backend, state | Becomes |
    |---|---|
    | Claude, `running` or `waiting` | each open ask or check is decided `deny`, `by: kernel`, reason `restart` (a Claude ask has no kernel deadline, 9.6; it expires only at the host, 4.5), and each ask with a pending answer is decided with that answer (9.8 rule 4); the turn becomes `unknown` (written as a `turn` entry and pushed) and the session leaves `active/`; the queue is kept. A tool call that was streamed but has no `tool` entry gets none: it ran inside `claude`, whose process is gone, and the kernel stores no open calls; the `unknown` turn entry after it ends the turn |
    | OpenAI child, `paused` | drop and cancel its `resume` deadline, set the loop idle and the turn `unknown`, and leave the queue and transcript for explicit owner continuation (R5). This holds for every child, also one whose delegated call resolved long before (intended: a child resumes only on its owner's word after a restart, S134, P1.8 of S132) |
    | Other `paused` sessions | unchanged; its `resume` deadline and timer are kept (7). Only the OpenAI loop writes `paused` (9.5); 9.8 gives nothing for a Claude rate limit, so a slice that adds one adds its row here |
    | OpenAI, `running` (loop `requesting`, `ready` in the FIFO or not, `retrying` or `tools`) or `waiting` (a tool approval open) | the turn stops: each open ask is decided `deny`, `by: kernel`, reason `restart`, and its `ask` deadline goes; every open job gets `aborted`, `idempotent` ones included, and every `pending` call too; the `retry` deadline goes; the loop goes to `idle` and the turn becomes `unknown` (written as a `turn` entry and pushed), with the transcript up to the last finished message, and the session leaves `active/`; the queue is kept. A `stopped {quota}` that recording the aborts gives stays. The step makes no model request, no toolset operation (open, describe or call) and no consent for it, whatever origin runs it (the pass runs under another owner's call); cancelling a stale timer is housekeeping it may do. An OpenAI turn does not continue by itself after a restart, and an idempotent job is not run again under its key: known limits (the deputy's cut, 2026-11-25; case 22). A message finished in the window between its transcript entry and `response.completed` when the host restarts is shown but is not in the model's history; the owner can repeat it: a known limit (the deputy, r4 F4.3) |
    | `idle`, `unknown`, `resume-lost` (not listed; reached only by R6) | each Claude ask or check still open or pending (left by an `unknown` `ask-resolved`, or an `interrupt` that `claude` did not follow with `control_cancel_request`) is closed as in the Claude row above, in every state (as built), `restart` for an open one and its answer for a pending one, the turn kept; then only the stale keys go; a Claude session's provider stream of an older boot counts as released, so its next `say` opens a new provider stream and resumes with `session {resume}` (`persist: false`: a fresh session), as after the idle release; no slot count is decremented for it (R1 zeroed the kernel's, the owner's reads zero by its boot tag) |

    A restarted OpenAI parent aborts its outstanding `task` calls with the
    other calls and clears its live child map. Each child independently
    takes its own R4 step. A paused child loses its automatic resume
    deadline even when its parent has already been stepped. All live
    delegation belongs to the boot that
    opened it, so an older-boot relationship is inactive even before
    either record is stepped. R4 retains the child's `parent` and the
    parent's child transcript entry and performs no recursive restart.
    A later explicit input to either session follows R5 independently.
    It cannot complete a pre-restart `task`.

    AGT-F-27 applies to the tool jobs of 6 (the OpenAI loop, row `tools`),
    the only calls a mod marks safe to repeat. AGT-F-10: no stored turn is
    shown as `running` unless something drives it: a Claude turn becomes
    `unknown`; so does a `running` OpenAI turn;
    `list` reports the turn the step will give (R6).
  - **R5, Continue.** `say` on an `unknown` session starts a turn that resumes
    the Claude session with `session {resume}` (4.1), unless its profile sets
    `persist: false` (nothing was kept: a fresh session); if `claude` refuses the
    resume, the turn becomes `resume-lost`. On an OpenAI session it starts a
    turn whose request carries the wire history and the new input, in the
    owner's own call (9.5 "After a restart"). When inputs are queued, the `say`
    input is still taken when the queue is full: the turn starts with the oldest
    queued input, then the new one joins the end of the queue, so order is kept
    and Continue is never refused `queue-full` for a queue that had room after
    the oldest left. R5 reads `access/` again (9.7, "Revoking"). The console's Continue (18.5) is `say` with the text
    `Continue`.
  - **R6, the lazy step.** Any call that reads a session record whose `boot`
    is older than `slots.boot` does that session's restart step first, in the
    same call (so `attach`, `read`, `say` and `answer` never act on a
    stale turn; an `answer` to an ask that step just denied is refused with one
    refusal `{field: "ask"}`, as for a check (2.3), and as an answer after the
    turn ended (16, case 9): the spec gives no other result for an ask already
    decided); the pass then finds the entry current and skips it. `list`
    reads no extra record for this: for a record of an older boot it reports
    the turn its restart step will give (`unknown` for a Claude `running` or
    `waiting`), without writing.

### 9.4 Entries, transcripts, attach

An entry is JSON:

```json
{"v": 1, "i": 42, "kind": "message", "at": 1763600000000, "uuid": "<claude message uuid>", "body": {}}
```

| Kind | Body |
|---|---|
| `message` | `{"role":"user"\|"agent","content":[<ACP content blocks>],"thought":{"text":"<text>","seconds":<n>}?}`; `thought` on an agent message is the thinking of that step joined, `seconds` from its first to its last thought chunk's `at`, rounded; a finished thinking block that repeats the deltas streamed for it counts once: it replaces those deltas in `text`, is not pushed live again and adds no time to `seconds`; a finished block with no streamed deltas is pushed once |
| `tool` | an ACP ToolCall with its final status: `{"toolCallId","title","kind"?,"status","content"?,"locations"?,"rawInput"?,"rawOutput"?}`; output cut to 64 KiB, marked |
| `ask` | the ask, below; later a decision entry that repeats it |
| `allow` | `{"id":"<allow id>","tool":"<tool>","key":"<scope key>","reach":"<rule>","given":true\|false,"by":"<who>"}`: a session allow given or taken back (9.6); `reach` is the one its ask showed, `<tool>(<key>)` |
| `verdict` | `{"on":"<ask id>"\|"input"\|"breaker","toolCallId":"<id>"?,"outcome":"safe"\|"unsafe"\|"unsure","reasons":[…],"by":"<provider>","denials-in-a-row":<n>?}`; `toolCallId` when `on` is an ask or check; `on` is `input` for the provider's `launch` and `input/<n>`, `breaker` for the breaker's item (10). An `unsafe` or `unsure` verdict on an input ends the turn `idle {failed {error: judge}}` (10 steps 4, 5). On a later input the queue advances as at any turn end; on `launch` it does not, and the kernel ends that provider stream at the verdict as at its `ended` (its open asks closed, its slot and keys freed), since the provider has closed it: a queued input stays queued, and a `say` before the late `ended` opens a new stream (S127 r2 F2.2, r3 F3.1) |
| `turn` | the turn JSON of 9.3 without `activity` |
| `child` | `{"session":"<child id>","toolCallId":"<parent call id>"}`; one entry in the parent's transcript when the child starts (9.7) |
| `compaction` | `{"tokens-before":<n>?}` |
| `unknown` | `{"raw":<the JSON value, or the line as a string>}`: a vendor line or event the adapters do not know |
| `part` | one slice of an entry over 256 KiB, as below |

**Naming.** Names ACP has keep ACP's spelling (`toolCallId`, `toolCall`,
`rawInput`, `sessionUpdate`); the kernel's own keys are kebab-case
(`expires-at`, `context-used`).

The body of an `ask`:

```json
{"id":"<ask id>","kind":"ask","toolCallId":"<the call's own id>",
 "toolCall":{"toolCallId":"<the same>","title":"…","rawInput":{}},
 "input":{},"path":"<absolute path>","expires-at":1763600060000,
 "reach":"Edit(<workspace folder>/**)"}
```

`kind` is `ask` or `check`. `toolCall` is the ACP ToolCall, its `rawInput` the
input; `input` is the full input. `path` is the host's resolved path of a file
tool's call (4.5 step 4), present only when the item had one (a non-string is
dropped); the file key comes from it, never from `input` (9.6). `reach` is the session allow that
`{"allow":true,"for":"session"}` would add (9.6): `<tool>(<key>)` from the
tool's scope function; it is absent when none is offered (a check, an
`agent:` owner, a tool or input with no scope key). A decision is a later `ask` entry that
repeats every field of the ask and adds:

```json
"decision":{"outcome":"allow"|"deny"|"none","by":"<who>","for":"session"?,"reason":"<text>"?}
```

A Claude ask or check gets exactly one decision entry, written when it
closes (9.8 rule 4), never at the kernel's own `answer`: the kernel's answer
is kept as a pending answer (9.3) and becomes the entry when the host's
`ask-resolved` arrives. When that item is `answered` with an `outcome` that
differs from the pending answer's, the entry has the item's `outcome` and `by`
`checked` on a `direct: false` stream, else `host`. A kernel ask's decision is
written at its answer.

`by` is `owner`; `session-allow`; `checked` (the checked provider); `kernel`
(a kernel ask's expiry, `timers`, no owner stream, `cancel`, `restart`, and for
an open Claude ask the provider stream's `ended`, 9.8 rule 4); or `host` (a
Claude ask the host resolved: `reason` is the host's `by`, `expired` or
`cancelled`; an `answered` on a `direct: true` stream that the kernel did not make, or whose
`outcome` differs from its pending answer, has no `reason`, 9.8). Repeating the ask lets a page that starts at the decision show
the card.

**The short decision** (S29). A decision written while the owner or the
kernel's account is past (9.3 "Past"; counted at the decision's write, with
the bytes the call has written so far, so a call that crosses the cap writes
the decisions after the crossing short) does not repeat the ask. Its body is

```json
{"id":"<ask id>","kind":"ask"|"check","ask-i":<i>,"decision":{"outcome":"deny","by":"kernel","reason":"cancel"}}
```

`ask-i` is the index of the ask entry (the `i` of the ask's `open` or
`pending-answers` entry, 9.3); `decision` is as above, a pending answer's
`input` included. The kernel writes it from the record alone, without
reading the ask entry. It has no `toolCallId`, `toolCall`, `input`,
`expires-at` or `reach` of its own; a reader tells it from a full decision by
`ask-i`. A client draws its card from the ask entry at `ask-i`: from the
entries it holds when that one is among them, else by reading that one entry
(`read` with `after: ask-i - 1`, `max: 1`; when that line is a `part` with
`m > 1`, `read` with `after: ask-i - m`, `max: m`, and join the parts). The
ask entry is never removed while the session lives, so it is always there to
read. An ask entry that already carries a decision (no adapter
makes one today, 9.8) is stored when its `id` is open or pending, and
is not stored otherwise; past quota it is stored short. The decision of an adapter's decided entry stored short
keeps only `outcome`, `by` and `reason` (cut at 256 bytes); any other
value makes the entry unstored. A decided entry that is stored closes its ask: the
id leaves `open` or `pending-answers` (as an `ask-resolved` does, 9.8 rule
4), so a repeat of it finds neither and is not stored.

- Indexes start at 1, increase by one, never change. Deltas are never
  entries.
- **An entry is at most 256 KiB serialized.** A larger one is stored, read
  and pushed as consecutive `part` entries:
  `{"v":1,"i":<k>,"kind":"part","at":…,"body":{"n":<1..m>,"m":<m>,"text":"<slice>"}}`.
  `text` is a slice of the whole entry's serialization, cut at UTF-8
  character boundaries; the whole entry carries the index of its last part.
  A reader joins the `text` of parts 1..m in order and parses the result as
  one entry. A page (`read` either way, a replay) may end inside a group, and
  the client joins the parts across pages. `attach` with an `after` that
  falls inside a group starts at the group's first part; `read` takes `after`
  as given, because a `read` that moved `after` back would return the same
  page again. A client that lost a group's start reads again with
  `after = i - n` (each part carries its `n`). Live, a finished entry over 256 KiB goes
  out as its parts, as replay pushes entries, never as one update. So every
  chunk (64 KiB, or one entry), `read` page (2 MiB) and replay (2 MiB) holds at least one
  whole entry or part, and paging always makes progress. No content is cut by
  this rule (tool output stays cut at 64 KiB, as before).
  A joined group reads as `read` reads a stored line at its last part's
  place: it takes that place's `i`, and any `i` in the joined text is ignored
  (S14). A joined text that is not JSON, not an object, of an unknown kind or
  of a newer `v` is `unreadable` with that `i` and the joined text as `raw`
  (S14). A joined whole of kind `part` is `unreadable` (S19). A stored `part`
  line, read alone, is still a `part`. A last part with no whole-number `i`
  never comes from `read` or replay, which give every line one; the joiner
  then yields the whole as parsed, and drops a whole of kind `part`, since an
  `unreadable` entry needs an `i` (S19).
- Reading an entry of an older `v` upgrades it in memory; an unknown `kind`
  or newer `v` reads as `{"kind": "unreadable", "raw": …}`. An `unreadable`
  entry is `{"v":<its v>,"i":<i>,"kind":"unreadable","at":<at>?,"raw":<the
  stored line as a JSON string>}`. It keeps the `i` of its place (and `at` when it has one),
  so paging, replay from `i + 1` and the drop of shown indexes (18.3) still
  hold. Every line a reader gets (`read`, replay) carries the `i` of its
  place, readable or not. The `i` of a line's place is its chunk's first index (the record's
  `chunks`, 9.1) plus the line's position counted from 0, so the chunk's
  first line has the chunk's first index. An `i` the stored line claims is
  ignored, so a corrupt line cannot take another entry's index. Nothing is
  migrated in bulk.
- **Who makes entries.** `append` here is the transcript store's own call
  inside the kernel, not a function of 2.3. Only the store makes `part` entries, when
  it stores an entry over 256 KiB; `unreadable` is never stored. A session
  appends only the kinds of the table above other than `part`, made by the
  adapters (9.8) or the kernel itself, under a session id the kernel made;
  never a kind or id a caller names.
- `attach(session, after, caps)`: in one call, read the entries with
  `i > after`, open the owner's stream and register it as the session's live
  stream; push those entries, then `{"type":"synced","through":<i>}`; then
  live updates. A previous live stream of the session ends `replaced`.
- **Replay fits the stream window.** `attach` replays at most 128 entries and
  2 MiB, half the in-flight limit of 3. When more entries lie after `after`,
  it opens nothing and returns `behind {through}`, `through` being the session's high-water index (`last` of `read`); the client pages with
  `read` first, then attaches again. `read` returns at most `max` (1 to 256, 2.3)
  entries and at most 2 MiB, so every replay call stays within its
  budget, whatever the length of the transcript.
- **`read`** returns a `page {entries, last}`. Without `before`: the oldest
  entries with `i > after`. With `before = b`: the newest entries with
  `after < i < b`. Both give them in ascending order, at most `max` (1 to 256,
  2.3) entries and 2 MiB. `last` is the session's highest stored index when the call
  ran (the high-water mark), so a client paging backwards knows where the
  live end is.
- **A full owner stream.** The kernel pushes to the owner's live stream in
  item calls and defers when a push fails `full` (3); it resumes in
  `stream-producer.ready`. Outside an item call (a `say`, an `answer`, a
  timer) it cannot defer: if a push to the owner's live stream fails `full`,
  it closes that stream with outcome `lagging` and resumes the session's
  provider stream in the same call (so does `attach` when it closes the old
  stream `replaced`), and the client attaches again after its last index. A deferral of a session's upstream lasts at most 10 s:
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
  chunk, and it writes nothing. Streams are memory only (3), so after a
  restart these keys are stale; nothing reads a `stream/<id>` of an old
  stream again (ids are never reused). The restart step of 9.3 (R4, R6)
  deletes the `stream/` keys of older boots and their `live/<session>`, in
  the same commit as the rest of the step; the generation is `slots.boot`
  (9.3, R1). Until then they stay charged to the owner (9.3, Quota).
- **Chunk reads.** A `read` page (at most 256 entries and 2 MiB) reads at
  most one chunk per entry it returns, plus the partial chunk at each end:
  at most 258 chunks and at most 2 MiB + 320 KiB (a partial 64 KiB chunk at
  the start, and at the end one chunk of up to 256 KiB read to learn that its
  entry does not fit), so it stays bounded. Appending an entry rewrites only the last
  chunk (9.1). An `append` that fails leaves no chunk written and nothing
  charged to the owner (7, "A call commits whole"; 9.3, Quota).
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
  `{"type":"queue"}` (9.3), and `{"type":"usage",…}` when the adapter has a sample (9.5, 9.7).

**The wire.** Every stream message is one JSON object:

```json
{"sessionUpdate":"agent_message_chunk","content":{…},"_meta":{"watane":{"at":1763600000000,"i":42}}}
{"_meta":{"watane":{"type":"entry","entry":{"v":1,"i":43,"kind":"ask","at":1763600000000,"body":{…}}}}}
{"_meta":{"watane":{"type":"synced","through":43,"writing":true}}}
{"_meta":{"watane":{"type":"turn","state":{"state":"idle","stop":{"kind":"completed"}}}}}
{"_meta":{"watane":{"type":"queue","inputs":[{"n":1,"input":"<input JSON>"}]}}}
{"_meta":{"watane":{"type":"usage","context-used":1200,"context-window":272000,"compact-at":200000}}}
```

In the `queue` item, `input` is the input's JSON as a string (2.3
`queued-input`).

- An ACP v1 `SessionUpdate` carries `_meta.watane.at` (ms, the kernel's clock
  when it adapted the line) on every live update, and `_meta.watane.i` on the
  update that finishes an entry (`message`, `tool`). A user `message`, which
  has no live update, goes out as its `entry` item (S35 user-sim). The
  update that finishes an agent `message` carries the whole message, not a last delta: a client replaces
  the text it streamed for that message with it (so a client that attached
  mid-message shows it whole, and nothing is doubled; S35).
- An entry with no ACP form (`ask`, `allow`, `verdict`, `turn`, `compaction`,
  `child`, `unknown`, `part`) goes out as an `entry` item, live and in replay. In
  replay every entry goes out as an `entry` item; a client also accepts a bare
  entry object. An ask goes out once, as its `entry` item: there is no separate
  `ask` update.
- `synced` ends the replay (accepted limit, S35: the Claude kernel does not set `writing` yet, since knowing that a message is being written would cost a state write on a delta-only item call, which case 56 and P09 rule out; an attach mid-message shows the turn's activity, Working, until the message's entry arrives; `kernel/tests/it/regress_s35.rs` `regress_s35_attach_mid_message_says_writing` keeps it, ignored, for the product owner). `writing` is true when a message was being written
  at the attach (18.3, "Writing…"), absent otherwise.
- Right after `synced`, before any live update, `attach` pushes the current
  `turn` item, the `queue` item, the current `usage` sample when there is one, and
  the turn's last `Plan` update when the turn has one.
- `turn`: `state` is the turn JSON of 9.3. `queue`: `inputs` as `queued` gives
  them. `usage`: `compact-at` is optional. Two null context fields clear
  the meter (9.7).

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
  Settled by S40-1: an input takes the step number of the response before it
  (`steps` when appended), so "the last 4 steps" are the items of the 4
  highest step numbers present; the mark is a wire index, and the trim record
  `{mark, through, stage}` trims the items below `through` (at most the mark),
  by `stage`: `reasoning` leaves out their reasoning items, `outputs` also
  cuts each tool output over 8 KiB to its first 8 KiB (at a character
  boundary) plus ` [cut by watane: <n> bytes]`, `<n>` the original size. A
  `wire/<id>/<n>` chunk is named by the wire index of its first item.
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
  provider, 4 request, 8 job slots), on top of the per-session cap. A `start` or a `say` that needs a Claude session's provider stream when its owner holds 4, or all 8 are held, is refused `busy` (2.3, 12); it never waits. A Claude session idle for 10 minutes (the `idle` deadline, 9.3) releases its provider stream (the kernel cancels it; the next `say` resumes the Claude session with `session {resume}`, 4.1; with `persist: false` there is nothing to resume, and that `say` continues as for `resume-lost`), so slots do not stay held by idle sessions. A pending `task` association consumes neither request nor job slots. The child's actual streams count against its root's owner (9.7). Owners that are different mods acting together can still take every slot: an accepted limit (availability, not a boundary of 14). State: `slots` and `slots/wait/<n>` (9.1). The
  loop state keeps a step's calls not yet started (`pending`); they start in
  waves as job slots free. A `start`, `say` or interrupt replacement whose
  request finds no request slot is refused `busy` and does not wait: the FIFO's
  drain runs in whatever call frees a slot, and a session's model request is
  sent only in its owner's call (the deputy's r4 ruling, F4.2); the owner sends
  again. No model request of a session is ever sent under an origin other than
  its owner's (the deputy's r5 ruling, F5.1), so no request waits in the FIFO
  past the call that queued it: a request a turn makes later (after its jobs,
  at a `retry` or `resume`, or for the next queued input) is sent in the call
  that makes it, which runs under the owner's origin, when a request slot is
  free; else, or when that call may be another owner's (the FIFO's drain, or an
  item or `ended` of a job the drain started, below), nothing is sent and the
  turn stops `idle {stopped {limit: busy}}` with its wire, outputs and queue
  kept, as after a restart (9.3 R4); the owner's next message continues it in
  her own call. No turn waits for a slot, so none hangs. Every `ended` of a
  kernel-consumed stream frees its slot and starts the oldest waiting jobs
  whose session and owner are under their caps in the same call; work at a cap
  stays in the FIFO and never blocks another owner's. A FIFO entry whose session no longer waits
  (interrupt, cancel, delete) or whose entry is of an older boot is skipped and
  removed when reached. The first call of a boot sets the counts to zero (9.3,
  R1; streams are memory only, 3).
- **Parallel:** the jobs of a step start together, up to the free job slots,
  only when every tool is `parallel`; else one after another, in call order.
- **After a restart:** the session's restart step (9.3, R4) stops a
  `running` turn, in `requesting`, `ready`, `retrying` or `tools`, and a
  `waiting` one (its open asks are denied `by: kernel`, reason `restart`, and
  their deadlines go): every open job gets `aborted`, `idempotent` ones included, and every `pending` call
  too; the `retry` deadline goes; the loop goes to `idle` and the turn to
  `unknown`, not running, with the transcript up to the last finished
  message (J11). A `stopped {quota}` that recording the aborts gives stays.
  The step makes no model request, no toolset operation (open, describe or
  call) and so no consent for the session, under any origin (it may run in
  the restart pass, under another owner's call). `paused` is unchanged. The owner's next `say` continues the session in her own call:
  its request carries the wire history, the aborted outputs included, and her
  input (R5). Known limits for now (the deputy's cut, 2026-11-25): an OpenAI
  turn does not continue by itself after a restart; an idempotent job is
  aborted, not run again under its key (case 22's idempotent half, not yet
  built); and the kernel opens no toolset again, so a handle kept from an
  earlier boot (the host keeps handles in memory) fails each later job, whose
  output is then the host's error.
- **AGT-T-138 Timer authority** (F-AGENT-7, AGT-F-26, AGT-F-28). The immutable
  `session/<id>.owner` binds `s/<id>` when the kernel arms it. The expected
  owner is the authenticated immediate `caller.identity()` that owns the
  session (2.3), or the root owner's identity for a delegated child. The
  expected origin is this identity for a non-page owner. The kernel's
  identity is never substituted for a non-page owner. A front mod can make
  the host's resolved origin differ from that identity. For owner `page`,
  the expected origin is the kernel's installed identity, the page's origin
  client (8). A successful authenticated page call arms the reserved
  zero-delay `page-origin` timer until the identity has been learned.
  Only a page chain may arm that ID. Its firing records the host's resolved
  identity in the mod's private state, which is scoped to that installation's
  identity. Page-chain resolution and fallback both identify the timer's
  owning mod. Until the host supplies that value, page firings fail closed.
  Before handling OpenAI deadlines or rearming an early OpenAI firing, the
  resolved origin must equal the expected origin client. Otherwise the kernel stops the
  loop, clears the deadlines, denies open asks with reason `timer-origin`,
  and records an `idle {failed}` turn explaining that the firing lacked the
  owner's authority. The queue and transcript remain available to the owner.
  Cleanup may cancel streams and record aborted jobs or resolve a delegated
  call locally. It makes no new model request, toolset operation or consent
  request. Timer callbacks dispatch only work queued by that callback, so a
  slot freed by cleanup cannot run another owner's queued job. A callback
  that owes a drain of the existing FIFO persists `drain-owed` in `slots`.
  The next successful ordinary client call drains it using the existing
  FIFO rules. The global
  restart pass and Claude idle release perform housekeeping.
  A durable timed pause of an enabled owner mod can resume after restart.
  Case 22 still applies to jobs and running turns.
  Known limit (S133 P1.12): the read carries no install generation. The
  kernel compares identities and relies on Watane's checks at resolution and
  apply. It cannot distinguish installations that share the same identity.
- **Settled by S40-3 (the loop).**
  - *The request.* `https-stream.start` gets exactly `{"url":"https://api.openai.com/v1/responses",
    "method":"POST","headers":{"content-type":"application/json","accept":"text/event-stream"},
    "body":<the body above, as text>,"account":"openai","idle-s":90}`. `reasoning` is
    `{"effort": <the profile's effort>}` when the profile sets one, else absent. A `start`, `say` or
    interrupt replacement sends it before its first write, so a host that refuses it
    (`start-error`) refuses the call `backend(<cause>)` and nothing is written; a
    request that finds no free slot refuses the call `busy` and writes nothing (Slots, above).
  - *State.* `loop/<id>` is `{state, ...}`: `idle` (between turns), `ready {wait?, attempt}`,
    `requesting {stream, attempt}`, `retrying {attempt}`, `paused`; with `model`, `effort`,
    `instructions` (the profile's `system-prompt` text, then the overlay's
    `system-prompt-append`, compiled once at `start`), `last-error`, `too-large`, the turn's
    `turn-tokens` and `turn-calls`; stage F adds `turn-steps`,
    `tool-errors-in-a-row` and a `summary` flag for the tool-free step (9.7).
    The wire fields are `next`, `active`, `wire-chunks`, `steps` and `trim`. `attempt` counts the retries so far: the first request is
    `requesting {attempt: 0}`; its failure sets `retrying {attempt: 1}` and the `retry`
    deadline at now + `backoff_ms(1)`; the failure of retry 6 (the 7th failed request) finds no
    backoff left and pauses with no time, `last-error` holding the error text (the turn JSON
    of 9.3 does not change). A usage limit sets `paused {until}` and a `resume` deadline at
    `until` (the reset time, at least 1 s ahead); the firing of either deadline sends the same
    request again (the body is rebuilt from the wire and the trim record, so it is the same).
  - *Items.* The first item of a request stream is its head `{status, headers}`; before it the
    host may push `{"type":"consent-pending","what":<text>}` (host 19.4, 19.7): the turn shows
    the live activity `consent {what}` (kept in `stream/<id>` for `attach`), cleared by the
    head. A stream that then ends `consent-refused(<what>)` ends the turn `failed
    {error: "consent-refused(<what>)"}`. A non-2xx status is classified over its body items joined (the first 64 KiB, kept in
    `stream/<id>` `body`): as soon as they read as JSON, else at `ended`; the loop then cancels
    the stream and frees its slot. `response.completed` appends the response's `output` items to the wire (step
    `steps + 1`), adds its input and output tokens to the turn's, and pushes `{"type":"usage",
    "context-used": <input tokens>, "context-window": 272000, "compact-at": 200000}`; with no
    `function_call` item the turn ends `idle {completed, tokens}` (`tokens` the turn's input plus
    output tokens, `tools` its call count when any), else the calls go to `openai_calls`. From stage F, apply the limit checks of 9.7 before dispatching those calls or starting another request. Items of
    a stream that is no longer the loop's current one are ignored, and so is every item after
    a stream's `response.completed` (deltas too: `stream/<id>` keeps `done`); the slot of a stream that is
    not current (a completed response's, or one a cancel left) is freed at its `ended`.
  - *Slots.* A request takes a slot when the kernel holds fewer than 8 requests and its owner
    fewer than 4 (`quota/<owner>` `request`, tagged by boot as `provider` is); else a `start`,
    `say` or interrupt replacement is refused `busy`, and a later request of the turn stops it
    `busy` (Slots, above). Inside an items call (it cannot start a stream: it may still be
    dropped) the request takes the place `slots.tail` in the FIFO: `slots/wait/<n>` `{session,
    boot}`, and the loop is `ready {wait: n}`; the end of that same call sends it, first, with a
    request slot free and its owner under 4, or stops the turn `busy`; no other call sends it. An
    entry counts only while the loop is still `ready {wait: n}` (or `tools {wait: n}`) for it; any
    other entry is skipped and removed when reached. Whatever frees a slot (an `ended`, a
    cancelled or failed request, `interrupt`, `cancel`, `delete`) starts the oldest waiting jobs
    whose owner is under its cap at the end of the call, each in a write of its own; a request
    entry another call queued is never sent there: its turn stops `busy`.
  - *Too large.* The turn ends `idle {failed {error: "too-large"}}` and `too-large` is set;
    the next input restarts the wire from `Wire::restart_from_summary` with the text of the
    transcript's last assistant messages joined by a blank line, the newest 32 KiB kept (cut at a
    character boundary), then sends the input.

- **Settled by S40-4 (the tool calls).**
  - *Toolsets.* The first request of a session whose profile lists `toolsets` is preceded by
    `agent-tools.open` and `describe` (6); the handle is kept in the record's `handle` and in
    `loop/<id>` `tools`, with the request's `tools`: per tool `{"type":"function","name",
    "description","parameters": <the input schema as JSON>,"strict":false}` in the order described,
    kept as stored so every request sends the same bytes, and each tool's `parallel` and
    `idempotent`. A profile with no toolsets opens nothing and sends no `tools`. A describe
    that returns `unknown-handle` immediately after opening waits for the opening call to commit.
    The next retry describes that same handle before sending a request.
    On Watane this delays a toolset session’s first request by about 1 s when the timer carries its owner’s authority. The timer authority rule above applies to this retry.
    `consent-pending(<what>)`
    from either call sets the live activity `consent {what}` and the `retry` deadline of the
    backoff above (the loop is `retrying`; the firing opens or describes again); `consent-refused`,
    `not-granted` or any other failure ends the turn `idle {failed {error: <that text>}}`.
    `cancel` and `delete` `close` the handle; `interrupt` keeps it.
  - *Calls.* `loop/<id>` `calls` lists the step's calls with no output yet, each `{item, st}`:
    `ask`, `pending` (allowed, not started), `job {stream}`, or stage F
    `child {session}` for a live `task` whose output is pending (9.7); `serial` says the step runs one call
    at a time (some call's tool is not `parallel`); `jobs` counts the session's job streams that
    have not ended; `items` keeps the `response.output_item.done` items of the response under way,
    which `response.completed` uses when its `output` is empty (dev 18.3's scripted server). The
    states are `waiting` (some call asks) and `tools {wait?}`. After built-in
    dispatch and the child `task` refusal (9.7), a call whose tool the session
    did not describe gets `{"error":"unknown tool"}`, or `aborted` when the session has no toolsets.
  - *Outputs.* A denied call's output is `{"error":"denied by the profile"}` (a deny rule, or a call
    no rule allows in mode `dont-ask`); a denied ask's is `{"error":"denied: <reason>"}`, the
    reason the owner's, or the kernel's `no-owner`, `expired`, `timers`, `busy`, `quota`; a job's
    `{"ok":v}` is `v` as JSON text, its `{"err":{code,message}}` is
    `{"error":"<code>: <message>"}`, a call that cannot start (`tools_call` failed) gets
    `{"error":"<the host's error>"}`. Each output is stored in the wire at once and shown as the
    Responses adapter maps a job output.
  - *Jobs.* The job of a call is `tools_call(handle, "<session>/<call_id>", name, arguments)`; its
    key is that string. Slots: 16 kernel-wide (`slots.job`), 8 per owner (`quota/<owner>` `job`,
    tagged by boot), 8 per session; a slot is freed at the job stream's `ended`, which also starts
    the next pending calls, in call order. A session whose pending calls wait for a kernel or owner
    slot (or finds the FIFO not empty) takes a place in the FIFO (`tools {wait: n}`); the drain
    starts its calls when slots free, oldest first, each in a write of its own. The drain may run
    in another owner's call, so a job it starts is marked `drained` in `stream/<id>`: its items
    and `ended` run under that origin and send no model request; when the step's request comes
    in one of them the turn stops `busy` (Slots, above). A job that ends with no item gives
    `aborted`. When every call has its output the loop is `ready`; inside an items call the
    request is queued with the FIFO and sent at that call's end (*Slots*).
  - *Restart, interrupt.* As the rows above; `interrupt` and `cancel` also write each open call's
    `aborted` output (wire and tool entry) and deny each open ask `by: kernel`, reason `interrupted`
    (`interrupt`) or `cancel` (`cancel`).

### 9.6 The broker

- Rules: deny, then ask, then allow; the strictest match wins. A rule that
  does not parse disables its tool. A tool that no rule names asks.
- Matching for OpenAI: tool names and `Name(arg)` with `*` and `?` globs;
  path rules are refused at compilation. Settled by S40-4: the name of a
  rule may hold `*` and `?` too (`notes_*`); it is replaced by the described
  tools whose names it matches before the rules compile, and the profile
  check accepts such a rule for `backend: "openai"` (the allowlist of Claude's
  tools applies to Claude profiles only). A call is matched by its tool's name
  alone (no `arg`). `ask-expiry` is in seconds and 600 when the profile sets
  none. An OpenAI ask's id is its call's `call_id`; the owner's `answer` decides
  it at once (decision entry `by: owner`), `{allow, for: session}` is refused
  `{field: "decision"}` (no scope function for a toolset's tool), and the ask
  has no `reach`.
- An ask goes to the owner when it has an attached stream; otherwise, and for
  an `agent:` owner, it is denied. Kernel asks expire after `ask-expiry`
  (an `ask` deadline of the session's timer, 9.3; at the timer cap the ask is denied `timers`); Claude asks after 10 minutes (the host). A kernel ask that reaches its `ask-expiry` is denied, reason `expired`, as the host denies a Claude ask at its 10 minutes (4.5); an `ask-expiry` of 0 denies at once. The session is not allowed anything by it.
- For Claude, rules went in as flags; the broker sees only asks. The kernel's own answer to a Claude ask (the owner's, a session allow, the "no owner stream" deny above) is kept as a pending answer; its decision entry is written when the ask closes (9.8 rule 4). A session holds at most 64 open asks and checks with the pending answers counted in (9.3); an ask past that is denied at once, `by: kernel`, reason `busy`.
- **Session allows: scope keys** (the human, 2026-11-21: "Session allows
  use a scope key per tool: a pure scope function turns a call's input into
  the smallest part that decides what it can reach and that a person can
  judge at a glance (Bash: the exact command; file edits: the workspace folder; a
  web fetch: the host; a skill: its name). A tool without one, or an input
  that cannot be scoped safely, gets no key and always asks. The kernel
  computes the key from the input that will run and shows it on the ask;
  'allow for this session' stores (tool, key); a later call is allowed only
  when its fresh key equals a stored pair exactly; take-back and session end
  remove it."). Each tool may have a scope function `scope(tool, folder, input, path) ->
  key or none`, pure, in `kernel/src/broker.rs`; the table below is the whole
  list, and a tool it does not name has none. The kernel computes the key
  when it stores a Claude ask, from that ask's `input` (the input that will
  run) and, for a file tool, its `path` under the record's `folder` (S36),
  and shows it on the ask as `reach` (9.4): `<Tool>(<key>)`, or the
  bare `<Tool>` for an empty key. An answer `{allow, for: session}` stores
  the pair (tool, key) in the session's `allows`. A later ask is answered
  `allow, by: session-allow`, by the kernel, only when the key computed
  afresh from its own input, with its tool, equals a stored pair exactly
  (byte for byte). The kernel keeps the pair; it never sends it to `claude`
  as a rule (`updatedPermissions`), so it can be taken back. `take-back`
  removes the pair and writes an `allow` entry; later matching calls ask
  again. Allows end with the session.

  | Tool | Input keys it knows | Key |
  |---|---|---|
  | `Bash` | `command`, `description`, `timeout`, `run_in_background` | the exact command; then `; timeout=<n>` and `; run_in_background=<true\|false>` when the input has them (pinned, S33) |
  | `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Read`, `Glob`, `Grep` | the fields of that tool's input (below), and the ask's `path` | the workspace folder, `<ws>/**` |
  | `WebFetch` | `url`, `prompt` | `domain:<host>`, the URL's host after one parse |
  | `Skill` | `skill`, `args` | the skill's name |
  | `TodoWrite` | `todos` | empty (reach `TodoWrite`): it reaches nothing outside the session |

  Every other tool has no scope function and always asks: `Task` and
  `WebSearch` (no safe key is named: a subagent prompt or a query cannot be
  judged at a glance), `AskUserQuestion` and `ExitPlanMode`, and
  `mcp__watane__<tool>` until that tool names its own key (14). An input
  with a key the tool's scope function does not know gets no key (file
  tools too: the `_host` key that `claude` adds to run a tool on an attached
  machine, PLAN S33 RISK 1), as does any value outside the rules below.
  **Stage B's host** (15): the file keys need the path that runs, symlinks
  resolved, before the key is computed (a link `<ws>/l -> /etc` would
  otherwise put `/etc` under `<ws>/**`). Since watane S100 the host gives
  each file-tool `ask` and `check` item a top-level `path` so (4.5 step 4);
  the kernel reads that and never the `input`'s path. The workspace's own
  folder used for `<ws>/**` is the record's `folder` (4.6),
  which comes from the `cwd` of the stream's
  first `system/init`. With no `folder`, or no `path` on the ask, no file
  key is offered and the ask still asks. Picked folders offer no recursive
  write keys because they do not name a source tree. A session allow grants only the
  reach its ask was opened with: `answer` with `for: "session"` computes the
  key afresh and is refused `refused`/`decision` unless it renders to that
  ask's `reach` exactly (S36 r1). A key for an
  `mcp__watane__<tool>` would rely on the host's `--strict-mcp-config`
  admitting only the `watane` server (4.4, 15), so that no other server's
  tool can carry that name.

- **As built (S31).** The OpenAI matcher is `Rules::compile(allow, ask,
  deny)` and `Rules::decide(tool, arg)` in `kernel/src/rules.rs`, pure, not
  yet called (the loop of 9.5, stage D, calls it; `ask-expiry` waits for it).
  A rule is `Name` (1 to 64 of `A-Za-z0-9_-`) or `Name(arg)`, `arg`
  non-empty and without parentheses or control characters; one with a
  readable name that does not parse disables that tool (`Deny`); one with no
  readable name, and a path rule (`arg` starting `/`, `~`, `./`, `../`, equal
  to `.` or `..`, or containing `**`), is a problem at compilation. `*` is
  any run, `?` one character, over the whole `arg`; a call without an `arg`
  matches only bare rules. For Claude (`kernel/src/broker.rs`,
  `kernel/src/sessions.rs`):
  - **As built (S34): the scope functions.** The kernel sets an ask's
    `reach` when it stores the ask, replacing any `reach` the item carries;
    none for a check, an `agent:` owner, and every tool the table above does
    not name. A key exists only when all of these hold, else none:
    - *Known keys.* The input is a JSON object whose every key is one the
      tool's row knows. The file tools know: `Read` `file_path`, `offset`,
      `limit`, `pages`; `Write` `file_path`, `content`; `Edit` `file_path`,
      `old_string`, `new_string`, `replace_all`; `MultiEdit` `file_path`,
      `edits`; `NotebookEdit` `notebook_path`, `new_source`, `cell_id`,
      `cell_type`, `edit_mode`; `Glob` `pattern`, `path`; `Grep` `pattern`,
      `path`, `glob`, `type`, `output_mode`, `-A`, `-B`, `-C`, `-n`, `-i`,
      `multiline`, `head_limit`, `offset`. Values of keys that are not part
      of the key are not read.
    - *Files.* The ask's `path` (the item's top-level `path`, 4.5 step 4;
      never `file_path`, `notebook_path` or `path` of the `input`, which are
      only checked as known keys) is a string under the record's `folder`
      (an absolute path, no trailing `/`, no `(`, `)` or control character),
      with no empty, `.` or `..` segment after it. A `Glob` or `Grep` whose
      `path` equals the `folder` (its path input absent: the working folder)
      has the key too; the other file tools get none for it. A `Glob` also needs a
      `pattern` that is a string not starting with `/` or `~`, with no `\`
      and no `..` segment (split on `/`), since `claude` searches an absolute
      pattern's own folder and ignores `path` (adversary, S33-fix1). For
      `Write`, `Edit`, `MultiEdit` and `NotebookEdit` no segment after the
      folder starts with `.` (`.git/config`, `.claude/`, `.mcp.json`,
      `.vscode/`, `.husky/`: a write there can make code run later) or holds
      a byte outside printable ASCII (a file system that folds Unicode case
      or drops ignorable code points opens `Ma\u{212A}efile` as `Makefile`
      or `\u{200C}.git` as `.git`; adversary, S34-fix2), and the last
      segment is none of `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`,
      `build.rs`, `package.json`, `Makefile`, `GNUmakefile` in any ASCII
      case (4.3); such a write always asks, on top of the host's protected
      paths (adversary RISK 1, S34). This list is not complete (a build file
      such as `Cargo.toml` or `conftest.py` can also run code later): the
      shell grant's sandbox and the host's protected paths stay the limit
      (14). Key
      `<ws>/**`, so reach `Edit(<ws>/**)` as before.
    - *Bash.* `command` is a string with no `:`, and `Bash(<command>)`
      passes the 4.1 allow grammar (so it has no `;` or `)`); then
      `timeout`, when present, is a JSON integer from 0 to 2^64-1, written
      in decimal, and `run_in_background`, when present, a JSON boolean. Key
      `<command>`, then `; timeout=<n>`, then `; run_in_background=<b>`,
      each only when the input has it, in that order: reach `Bash(ls;
      timeout=5; run_in_background=true)`. An absent key and a key with
      Claude's default value are different keys; `description` is never
      part of it. So `dangerouslyDisableSandbox` (an unknown key) always goes
      to the owner (S31-fix1). The key is the text of the command, not what
      it reads: the same command can reach other files after an allowed
      `cd` moved `claude`'s shell folder, or after the agent changed files
      it reads (`git status` and `.git/config`); the sandbox is the boundary
      (the human, 2026-11-21, (4); named in 14).
    - *WebFetch.* `url` is a string of 1 to 2 048 bytes, printable ASCII
      only (no space, no control character, no `\`), parsed once as a
      WHATWG URL (the `url` crate, as Node's `URL` that `claude` runs); the
      parse succeeds, the scheme is `http` or `https`, there is no username
      or password, no port other than the scheme's default (which the parser
      drops), and the host is a domain or an IP
      address with no `(` or `)`. Key `domain:<host>`, the parsed host as
      the parser writes it (lowercase; a non-ASCII name as its `xn--` form):
      reach `WebFetch(domain:example.com)`. Path, query and scheme are not
      part of it: an allow holds that host only, never another one, a
      subdomain or a parent; except that `claude` itself follows a redirect
      from the allowed host to the same host with or without a leading
      `www.` (same scheme and port), so the allowed server, never the model,
      can lead one fetch there (adversary RISK 2, S34; named in 14).
    - *Skill.* `skill` is a name `[a-z0-9-]{1,64}`, or two such names joined
      by one `:` (a plugin's skill). Key the name: reach `Skill(pdf)`.
      `args` is not part of it.
    - *TodoWrite.* Key empty: reach `TodoWrite`.
  - An allow is `{id, tool, key}`; its id is the index of its `given: true`
    entry. An answer `for: session` to an ask with no key is refused
    `{field: "decision"}`; one whose pair is already held adds nothing and
    writes no entry. The host gets the decision without `for`.
  - `take-back` of an id not held is `unknown-allow`, writing nothing; it
    reads no `access/` and is never refused `quota` (it only narrows; its
    entry is charged like the calls of 9.3 "Past" that cannot fail).
  - A new Claude ask (kind `ask`, stored, under the cap of 64): a matching
    allow while `access/<owner>` still lists the profile (read again each
    time, failing closed; PLAN S20) gives the pending answer `allow, by:
    session-allow`; else an `agent:` owner or no live owner stream gives
    `deny, by: kernel, reason: no-owner`; else it opens for the owner. A
    kernel-answered ask never enters `open` and makes no `waiting` turn.
    A check is never answered so. Past quota the ask is not stored and is
    denied `quota`, as before.

### 9.7 Delegation and counters

**Who delegates.** OpenAI roots can call the kernel's `task` function for
named OpenAI profiles. The root is depth 0 and its children are depth 1.
The profile field keeps its 4.1 name, `subagent-limits`, but it limits
OpenAI child sessions only, never Claude subagents (FUNCTIONAL 1, "live
child"). The root's effective `subagent-limits.live` counts its live
children: its outstanding `task` calls across steps, one per child whose
delegated first turn has not yet resolved the call. A child whose call
has resolved (it ended, or the owner interrupted it, which resolves the
call) is no longer live, even if the owner then gives it more work with
`say` or a replacement: that work is the owner's, bounded by the owner's
own session and slot caps (9.5), not by the root's live cap. This is
intended (S134, P1.2 of S132). Claude uses its own launch `subagents` and
`subagent-depth` (4.1, including depth 0 to disable `Task`). The kernel
records their activity within that Claude session and allocates no child
session or kernel child slots for them.

**Admission.** Check at dispatch, before opening anything or reserving an
id, in this order; each of the first four steps stops at its one refusal:
1. authorize the root again as in "Revoking" below (`profile`);
2. reject a child caller with `refused` on `subagent-limits.depth`;
3. check membership of the name in the root's effective `delegates`
   before loading the named profile: an unnamed, missing or unadopted
   profile is `refused` on `profile`;
4. at the live cap the refusal field is `subagent-limits.live`;
5. validate the candidate as an OpenAI profile under 9.2 and 11, then
   compare it to the root's compiled profile including its overlay, by the
   table below. Step 5 returns every refusal it finds at once (as `check`
   does, 9.2).

Rules compare as written: the root's and the candidate's `allow` and
`deny` lists after compilation (profile, adopted offers, overlay; 9.2) are
compared as rule strings, a glob as the glob, with no `describe` and no
expansion over described tools, so admission opens nothing.

| Field | Child rule |
|---|---|
| `backend`, `model` | Both are OpenAI on the same model. |
| `toolsets`, `skills` | `toolsets`: a subset of the root's effective toolset names; a toolset has no adopted revision in the kernel (it is a host grant by name, 6), so the child's own handle is acquired under the root's owner through the normal host checks (origin consent, the grant), which decide what it reaches. `skills`: a subset of the root's adopted skill identities with the same adopted revisions (11). |
| `tools`, `workspace`, `extra-workspaces` | OpenAI's existing absence rules apply. The child has no workspace handle, folder or requested folder. It inherits no folder consent or session allows. |
| `mode`, `ask`, `pre-tool-check` | Same effective values. |
| `allow`, `deny` | Exact-rule subset of allows and superset of denies, after compilation, compared as written (above). |
| `persist` | `false` when the root's is `false`: a root that keeps nothing cannot start a child that stores its `input`, once OpenAI `persist: false` is built (11: not yet, for any OpenAI session). |
| `max-turns`, `max-tool-errors`, `max-tokens`, `ask-expiry` | Equal or lower effective values. For an optional cap, absent means unbounded. |
| `delegates`, `subagent-limits` | No named delegates or child-supplied delegation limits. |

Instructions, effort, persistence and output schema use normal OpenAI
profile validation. Each comparison failure is a `refused` with that
field in the ordinary refusal record (2.3), and no child is created. The
child keeps the compiled settings used at admission. The kernel stores
the root's owner as the child's owner, and the root id as its `parent`.
The root's access entry plus the dispatch check authorize the child
profile; a separate client grant to start that profile is unnecessary.

- **Revoking.** `access/` (9.1) is read again by each call that would give a
  session more work: `say` (whether it starts a turn or queues), `interrupt`
  with a replacement, R5 Continue, the owner's `answer` with `allow`, and an
  OpenAI `task` that starts a child (checked against the root session's
  profile and owner). When `access/<owner>` no longer names the session's
  profile with the record's `profile-digest` (9.10; a record with none
  counts as not named, the first-run allowance of 9.1 aside), or is absent
  or unreadable, such a call is refused `refused`,
  `{field: "profile"}` (a `task` call gets that refusal as its output), and a
  queued input is not started when a turn ends: the turn ends as it would, the
  queue is kept for `unqueue` or `delete`. Calls that only read or take power
  away still work: `read`, `attach`, `list`, `queued`, `unqueue`, `rename`,
  `answer` with `deny`, `interrupt` without a replacement, `cancel`,
  `take-back`, `delete`. A call on a child session checks the root session's
  profile against the root's owner, as `task` does. A missing root is
  `refused` on `profile` for new work. A `resume` deadline (9.3) that fires after a
  revocation does not resume the turn: it stays `paused` with no time, as at
  the timer cap (the deadline is gone, so a session with no other deadline
  frees its timer, 9.3), so only a `say` resumes it, and that `say` is refused. A turn
  already running is ended only by `manage.withdraw` (9.10); an edit of the
  profile or the answer for a newer version does not end it (a limit, 14).

**The function and its lifetime.** `task(profile: string, input: string)`
has exactly those two required fields. Advertise it on normal root steps
with delegates and omit it from child requests and tool-free summaries.
An attempted call from a child is refused even if the model emits it.
Apply the broker's profile rules for the function name `task` before
starting a child. Refuse a toolset function whose name collides with an
active built-in on `toolsets` before sending a request, on every request
(a description kept from an earlier request is checked too). `task` is
active in a root that names delegates. `skill` is active in a session with
one or more effective skills (11). Otherwise the name is free.
The function permits parallel calls, and is never retried as an
idempotent job.

The parent's call stays open while the child's first turn runs. Starting
a child commits the child record, pending association and indexed
`child` entry together, or creates none. The pending association has
no stream and is bounded by the live child cap, the active index and
the normal record and owner quotas. It stays outside the streaming job
FIFO so waiting parents leave slots available for children. Child
launch and every request require the owner's origin and use the normal
start and slot checks. If dispatch lacks that origin or admission hits
a busy cap, return `{"err":{"code":"busy"}}`. The parent releases its
completed request by the existing `ended` rule.
The client learns the id from
the `child` entry immediately upon commit.

A started call has exactly one terminal output: `{"ok":{"session":"<id>",
"message":"<final assistant text>"}}` when the child completes with text;
otherwise `{"err":{"code":"<cause>","session":"<id>"}}`. Use the named
stop limit for a limit stop (`max-turns`, `max-tool-errors`, `max-tokens`,
`busy`, `quota`: a child request that stops `busy` after start gives
`busy`), `backend` for a failed turn, `interrupted` when the owner
interrupts the child's first turn, and `no-result` for an empty
completion. Use the existing `aborted` job output when the parent's call
itself is aborted (the parent is interrupted, cancelled, deleted or
restarted, or the child is cancelled or deleted). A refused dispatch returns `{"err":{"code":"refused","refusals":[
<2.3 refusal records>]}}`; a failure before start has no child id.
Outputs use the normal tool-output bound (9.4), cutting oversized text
inside the result so the envelope stays valid. On termination the parent
consumes the pending association once.
Only the current association may produce an output. A parent follow-up
request uses the origin and slot rules of 9.5, including stopping
`busy` if it cannot send.

Interrupt, cancel or deletion of the parent aborts pending calls and
stops live children: an `interrupt` of the parent interrupts each live
child (its turn ends `interrupted`, its toolset handle kept, 9.5), and a
`cancel` or `delete` of the parent cancels each live child (its handle
closed). A child is never deleted with its parent; the owner deletes it. Cancelling or deleting a live child also resolves
the parent's call as aborted; interrupting it resolves the call
`interrupted` (above). On restart use R4 (9.3); the aborted output
is recorded and never delivered to a fresh turn as completion of a live
call. Parent cleanup only touches its bounded live map and each child's
own turn and streams. Finish cleanup before reporting Stop for the
parent. Only the first delegated turn can resolve the associated call;
later explicit owner input uses the normal session rules. The child
link survives either turn ending. A client opens the child via existing
owner-scoped calls (2.3); a deleted child is unavailable.

**Limits.** Each input that begins a new OpenAI turn resets model-step
count, token total, failure streak and summary flag. The loop uses
9.5's existing per-turn token total and adds the step and streak counts
there. An ask, retry, or rate-limit
pause within that turn preserves all three. Count each completed model
response once and add its reported input plus output tokens, including
the tool-free summary. Compare `max-tokens` with the running total after
each completion, before dispatching tools or issuing a follow-up. At
or above the cap end `idle {stopped {limit: max-tokens}}`, also when
that response is a final text answer (it does not end `completed`). A
response can cross the cap before its usage arrives. A root's
`max-tokens` counts its own steps only, not its children's: each child
has its own cap, at most the root's (admission), so one root step that
starts children spends at most the root's cap once per live child on
top of its own. A turn with a missing count has no exact
token total. If the profile sets a
token cap and a completion lacks valid token counts, fail that turn
`idle {failed {error: usage-unavailable}}` before dispatching further work.

Count final tool failures and denials in committed output order (call
order for a batch committed together); a success resets the streak. A
success is a job's `{"ok"}` output or a `task` call's `ok` output; every
other output the loop writes for a call is a failure or a denial (an
`{"error":…}` text, an unknown tool, a `task` `err`), except the
`aborted` outputs of cleanup.
At `max-tool-errors`, stop with that limit, aborting other open calls
and pending work by the interrupt cleanup of 9.5. Those cleanup aborts
do not count. `max-turns` caps the completed model steps of one turn (FUNCTIONAL 1,
"model step"); the name is the profile field's of 4.1, where Claude's
`--max-turns` counts the same thing. When a response reaches `max-turns` and would otherwise
call tools, close its calls with aborted outputs and send at most one
summary request with no tools, under the usual request and origin rules.
Mark that request as the summary so retry or pause preserves its
tool-free mode and it can complete only once. After that step end `stopped {limit: max-turns}`, unless another limit
or a request failure ends it first. A final text response at the step cap
needs no extra step and completes normally. The token cap takes
precedence when a response reaches both limits.

For Claude the kernel passes `max-turns` as the native flag. When a
current turn's `result` reports `subtype: "error_max_turns"`, its ensuing idle
ends the turn `stopped {limit: max-turns}` without an extra step.
A Claude profile with `max-tokens` or `max-tool-errors` is refused
(11), as is an overlay that sets either on a Claude session. Its
result reports turn totals, and its own process controls native tool
execution. Late results from an interrupted turn cannot end the
replacement (9.3).

**Usage.** Stage D publishes the OpenAI sample from each completed
response (9.5). Stage F retains that sample and adds a Claude sample at
a completed turn's `result` only when it gives comparable context-use
and window data: require `num_turns: 1` and one model in
`model_usage_since_last_result`. Use that model's `contextWindow` and
the sum of `inputTokens`, `cacheReadInputTokens` and
`cacheCreationInputTokens`. The cumulative `modelUsage` and multi-step
turn totals cannot supply the current context sample. Omit the sample when a reliable single-step count and
positive window are unavailable. Claude decides its own compaction and
omits `compact-at`. Clear the prior Claude sample when a turn ends
without a sample so the meter does not present it as current. Use
`{"type":"usage","context-used":null,"context-window":null}` for this
clear on live streams; attach replays only a current sample (9.4).
Known limit (S134, P1.6 of S132): `num_turns` and `subtype` are on
record (the `result` fixture of 9.8 and `claude-codes` 2.1.330), but no
probe in `proto/` has yet recorded `model_usage_since_last_result`; until
`proto/claude-probes/probes.py` records it on the real `claude`, a
`result` without it gives no sample and the meter is cleared, which
fails safe (the next probe run after a `claude` update adds it, Upkeep).

### 9.8 Adapters

| From | To |
|---|---|
| `assistant` text, `stream_event` deltas | `AgentMessageChunk`; a finished message is a `message` entry |
| `assistant` `thinking` blocks and deltas | `AgentThoughtChunk`; a finished block that repeats its streamed deltas counts once (9.4, `thought`) |
| `assistant` `tool_use` of `TodoWrite` | `Plan`, with its entries and their status; also a `tool` entry |
| `assistant` `tool_use` | `ToolCall` (pending) |
| `user` `tool_result` | `ToolCallUpdate` (completed or failed); a `tool` entry |
| `ask`, `check`, `ask-resolved` items | an `ask` entry (9.4), pushed live as its `entry` item; its tool call id is the item's `tool_use_id`, its `expires-at` the item's, its `path` the item's when that is a string; an `ask-resolved` that closes an ask or check gives its one decision entry (9.8 rule 4); an item that breaks a rule below the table (1–6) is `unknown` |
| `system/compact_boundary` | a `compaction` entry |
| `result` | turn totals, optional context sample and native limit outcome (9.7); the ensuing Claude idle ends the current turn |
| Responses `response.output_text.delta` | `AgentMessageChunk` |
| Responses `function_call` items | `ToolCall`, then `ToolCallUpdate` with the job's output |
| Responses `reasoning` summaries | `AgentThoughtChunk` |
| Responses `compaction` items | a `compaction` entry, once per item (from `output_item.done`, or from `response.completed` when it did not stream) |
| anything else | an `unknown` entry |

Give nothing (no update, no entry): Claude `system` `init` (its facts go to the record, below), `status`,
`session_state_changed` (read for idle, 4.5 step 6), `hook_started`,
`hook_response`, `tool_progress`, `task_progress`, `rate_limit_event`,
`stream_event` other than text and thinking deltas; Responses
`response.created`, `response.in_progress`, `response.completed`,
`response.failed`, `response.incomplete`, `error` (the loop reads these,
9.5), `response.output_item.added`, `response.content_part.added`/`.done`,
`response.output_text.done`, `response.reasoning_summary_part.added`/`.done`,
`response.reasoning_summary_text.done`,
`response.function_call_arguments.delta`/`.done`. Then: a `wrapped` item; an
`ask` or `check` item whose `tool_use_id` is not a non-empty string, whose
`tool` is not a string, whose `expires-at` is not a whole number from 0 to 2^64 - 1, or whose `input` is not
a JSON object;
an `ask` or `check` item whose `id` is that of an ask or check still open on
the session or of an ask with a pending answer (the open one is not changed); an `ask` or `check` item whose
`tool_use_id` is that of an open ask or check, or of an ask with a pending answer, with a different `tool` or
`input` (the open one is not changed: one card per call, since a client draws
the card of a call from its ask, 18.3); these rules hold against the session record (`open` and `pending-answers`), not only a stream's memory, so a second stream or a reloaded instance is held to them; and, on a `direct: true` stream, a `verdict` or
`checking` item, are each an `unknown` entry, never an ask (14). The kernel
takes `verdict` and `checking` only from a checked provider's stream
(`direct: false`, 10). It reads items strictly (no lossy UTF-8 decoding): an
item it cannot parse is `unknown`, never an ask. An item that breaks a rule
below is an `unknown` entry, never an ask or a decision.

1. The `id` of `ask`, `check` and `ask-resolved` is a non-empty JSON string. A
   number or another JSON value is `unknown` (S14), so `"7"` and `7` never
   name the same ask.
2. The `tool` of an `ask` or `check` is a JSON string; any other value is
   `unknown`. `input` of an `ask` or `check` is a JSON object. Missing, `null` and any
   other JSON value are `unknown`; `{}` is an object (S19).
3. `expires-at` is a JSON integer from 0 to 2^64 - 1. A negative or
   fractional value, `5.0`, `1e3`, a value past 2^64 - 1, a string and `null`
   are `unknown` (S19).
4. `ask-resolved` reads its `by`. A Claude ask or check gets **one**
   decision entry, written **once, when it closes**, never at the kernel's
   `answer`. When the kernel answers a Claude ask itself (the owner's
   `answer`, a session allow, the "no owner stream" deny of 9.6), it keeps
   that answer as the ask's **pending answer** (`{outcome, by, for?, reason?}`,
   in the session record, so it survives a restart, 9.3) and writes no decision entry yet.
   While an ask has a pending answer it counts as decided: a second `answer`
   is refused `{field: "ask"}` (case 9), no session allow or
   restart step answers it again, and it is not counted as open for the
   owner. The ask closes:
   - at `answered` with an `outcome`: when it equals the pending answer's
     outcome, the decision entry is the pending answer (its `by`, `for`,
     `reason`); when it differs, the entry has the item's `outcome`, `by:
     checked` on a `direct: false` stream (10) and `host` otherwise, no
     `reason`, and the pending answer is dropped (a session allow it added
     stays; a later matching ask is still judged by the checked provider,
     10). Example: checked mode, the owner's allow while the verdict is
     pending, then an unsafe verdict: the provider denies, the host pushes
     `answered`/`deny`, and the one decision is `deny, by: checked`;
   - at `expired` or `cancelled` (an ask, or a check the host did not answer):
     the decision `deny`, `by: host`, `reason` = the `by` (9.4); the pending
     answer is dropped, since what the kernel sent never took effect (the
     host denied it, or `claude` cancelled it); so `expired` after a pending
     answer gives this one decision, never a second;
   - at the kernel's own closing steps: `cancel` and the restart step R4
     (9.3) and the provider stream's `ended` (3). The entry is the pending
     answer, what the kernel answered (whether it reached `claude` is not
     known: the request may not have applied, or the checked provider may
     still have held it; 14, "Not protected"); an open ask or check with no
     pending answer gets `deny`, `by: kernel`, reason `cancel`, `restart` or
     `ended`. The `ended` closes every open or pending ask or check of the
     session in the same call, so no Claude ask stays without a decision; then it ends
     an active turn `idle {failed {error: provider-ended}}` (9.3).

   An ask or check with no pending answer that the host closes with
   `answered` gives the decision entry from `outcome` (`by: checked` on a
   `direct: false` stream, else `host`). An `answered` whose `outcome` is
   missing or not `allow`, `deny` or `none`, or is `none` on an ask or
   `allow` on a check, is `unknown` (a check is answered only `none` or
   `deny`, 4.5), and so is an `expired` or `cancelled` item that carries an
   `outcome` key; the ask stays as it was, open or pending. Any other or
   missing `by`, or one that is not a string, is `unknown`. An `unknown`
   item leaves the ask as it was, open or pending, so a later `expired` or
   `cancelled` still gives its decision (S19). An `ask-resolved` whose `id` is not open or pending is
   `unknown`. A kernel ask (OpenAI) is unchanged: its decision is written at
   its answer. Stage E builds only the provider's side (10).
5. A line that is not valid UTF-8 is `unknown`. Its `raw` is the line as a
   JSON string: each byte that is not UTF-8 is written as the four characters
   `\xNN` (lowercase hex), and white space is trimmed from both ends. A
   literal `\xff` in valid text reads the same: an accepted ambiguity, only
   for display. There is no lossy decoding. (The host's `wrapped` `line`
   keeps U+FFFD, 4.5: that is the host's form, not a conflict.)
6. A line with a key twice at its top level is `unknown` in the kernel too,
   the kernel's half of 4.5's "never read one line two ways". Keys below the
   top level are read last-wins (14).

When an ask's input differs from the input of any `ToolCall` of the
same id, earlier or later, the ask's is the one that runs and the one shown: the ask entry's
`toolCall` and `input` are the truth, and a client draws the card from them
(18.3), while the ask is open and after its decision. The kernel builds the
ask's `toolCall` from the item's `tool` and `input` only (its `title` by the
same rule as a `ToolCall`'s), never copied from a `ToolCall` of the same id.
The `toolCall` carries ACP `kind` from `tool`: `Read` `read`; `Edit`,
`MultiEdit`, `Write`, `NotebookEdit` `edit`; `Bash` `execute`; `Grep`, `Glob`
`search`; `WebFetch`, `WebSearch` `fetch`; any other name `other`. For `Edit`,
`Write` and `MultiEdit` it also carries `content`, the ACP `diff`(s) of the
`input`: `Edit` one (`path` `file_path`, `oldText` `old_string`, `newText`
`new_string`), `Write` one with `newText` the `content` and no `oldText`,
`MultiEdit` one per `edits` item, in order. A field missing or not a string
gives no `content`, and an `Edit` with `replace_all: true`, or a `MultiEdit` with any edit `replace_all: true`, has none (a diff would understate what runs; the card shows `rawInput`); the ask is still an ask. Both are built from the item's
`input` only.

A `check` is the checked provider's to answer (10); its `none` means "no
objection", never an allow. A client shows a check as a pending check or a
decision line, with no answer buttons; the kernel offers no answer options for
it.

**Adapter state** (S35). A mod instance keeps no memory between calls (host
6: a new store per call), so the Claude adapter's state for a provider
stream (the tool calls it has seen, thinking so far, whether `result` and
the idle line came) is kept in `adapter/<stream>` (9.1), read and written once
per items call with the stream's `last`, and deleted with `stream/<id>`. A
deferred or failed call rolls it back with the rest. From the items it reads
the kernel also takes the `initialized` item's `workspace` (4.6) and the
`system/init` line's `session_id` (the record's `claude-uuid`, for a later
resume, 9.3 R5; only a uuid, `8-4-4-4-12` hex digits, as host 19.3 takes a
launch's `session`: another id is not kept, and the next launch starts a new
Claude session, S36) and `cwd` (the record's `folder`, once, 4.6; S36); none gives
an entry.

Libraries: `claude-codes` (types only) and `agent-client-protocol-schema`
(v1). An unknown Claude line never fails: the adapter falls back to `unknown`
when `claude-codes` cannot parse it.

### 9.9 The stage A probe (removed)

Removed in S35, when stage B linked the unchecked provider: the interface
`probe` of 2.3, its code and tests, and the console's probe session. The
rules below are the record of what stage A ran.

Stage A has streams and timers but no provider (17), so no session of 9.3
can run a turn. The probe gives the console a real kernel stream all the same:
a session whose entries the kernel writes itself, one per timer firing. It
is the kernel's part of V8 (16): host timer, kernel, owner stream, page. It
is removed when stage B links a provider (the interface `probe` of 2.3, this
section, and its code).

- `probe.start(key, count, every-ms)` is an owner call like `start` (9.3):
  the owner is `caller.identity()`, `key` a client key (9.1; a repeat with the
  same key answers the same id and changes nothing), and it counts against
  the owner's sessions and the quota as `start` does. `count` is 1 to 1 000
  and `every-ms` 100 to 60 000, else `refused` with one refusal naming the
  field. At the session-timer cap (9.3: 240 kernel-wide, 120 per owner) it is
  refused `busy`, naming the cap, and writes nothing. It makes a session record with `backend` `probe`, `profile` empty,
  `title` `Numbered items`, turn `idle`, no provider stream and no client
  input, and the deadline `{what: "probe", at: now + every-ms}` on the session
  timer (7, 9.3).
- Each firing of the session timer with the `probe` deadline due appends the
  next entry, `message` `{role: "agent", content: [{type: "text", text:
  "Item <n> of <count>"}]}`, with `n` from 1, through the same store-and-push
  path as a provider's finished entry (9.4: one write, then the push to the
  session's live owner stream when there is one, with its index), and sets
  the next deadline at `now + every-ms` until the `count`th entry. The
  record keeps `n` and `count` (`probe {n, count, every-ms}`), so the firing
  reads only the record.
- `attach`, `read`, `list`, `rename`, `delete` and `cancel` act on a probe
  session as on any (`cancel` drops its deadline, so no more items come).
  `say`, `answer`, `interrupt`, `queued`, `unqueue` and `take-back` are
  refused `backend("a probe session takes no input")`.
- A firing during a full owner stream stores the entry and drops the live
  stream `lagging` (9.4), as for a provider's entry; the page reattaches
  after its last index and misses and doubles nothing.

### 9.10 Profiles, access and offers (stage F)

Only the participant gives a client a profile (AGT-F-1, AGT-F-56, AGT-F-57).
A client mod, the factory's among them, asks for nothing it can grant
itself: it lists what it may use (`profiles`), and it may offer profiles
(`offers`), which give it nothing. The first consumer is watane's factory
mod: one mod with one identity, so one `access/<identity>` holds both its
role profiles (watane `spec/design/factory/TECHNICAL.md` 2 and 5, rows
"Planning and foreman split" and "Factory profiles at F"): an implementer
on Claude with shell and web (N5 (a), the human, 2026-11-27) and a reviewer
on OpenAI with no built-in tools, toolsets, skills or delegates. A child it
delegates to later needs no entry of its own (9.7).

**One writer.** The kernel writes `profile/`, `profiles`, `access/` and
`access-ask/` (9.1) only in calls of `manage` (2.3) and in the
`stream-sink` call that takes the host's answer to an access question
(below). Every `manage` function checks `caller.identity()` first: any
caller but `page` is `not-owner`, and the call reads and writes nothing. No
other call writes those keys: not `sessions`, not `offers`, not a
`timer-sink` firing, not the OpenAI loop. So neither a client mod, nor an
agent (`agent:<handle>`, through a toolset), nor a timer has a path to them.
`clients` is written only as **Clients** says. `manage` serves the profile
manager, a view of the kernel's page that is not the console: the console's
code calls only `sessions` (18.1). Both run in one page under one identity,
so the page alone can never widen a client: each widening waits for the
participant's answer in the app (**Allowing**), and a page script can at most
ask that question or take access away.

**Clients.** `clients` lists the identities the participant may allow,
besides `page`, which is always allowable and never stored. `sessions.profiles`
and `sessions.check` add the caller's `caller.identity()` when it is not
listed, is not `page`, does not start with `agent:`, and the list holds
fewer than 64; both calls return no error, so their write commits. So a
client is listed only by its own call: the participant never types an
identity, and an offer names none. At 64 the page says the list is full;
`manage.forget(client)` removes a listed client that has no entry and no open
question (else `refused {field: "client"}`).

**Pinned entries.** `access/<identity>` is a JSON array of at most 64
objects `{"profile": <name>, "digest": <64 lowercase hex>}`, one per
profile name, where `digest` is the SHA-256 of the exact bytes of the
`profile/<name>` value the participant allowed. Any other value is
unreadable and allows none, an array of names (the format before S151)
included; the first-run allowance for `page` (9.1) is unchanged. A name is
allowed for a caller only while its entry's digest equals the digest of the
stored profile and the profile is not stale (**Offers**): `start`,
`start-in-folder` and `check` read the entry and the profile and compare;
a mismatch is refused `{field: "profile", reason: "changed since the
participant allowed it"}`, a stale profile `{field: "profile", reason: "its
offer changed"}`, and `profiles()` leaves both out. `start` keeps the
digest it checked as the record's `profile-digest` (9.3); a session of the
built-in `scratch` has none. New work on a session (9.7, "Revoking") is
allowed only while the owner's entry for the session's profile holds the
record's `profile-digest` **and** the stored profile still has that digest:
the call reads `access/<owner>` and `profile/<name>` and compares both.
Every other read of `profile/<name>` for an existing session compares too:
a later launch (9.3, the kept overlay: a new provider stream after the idle
release, an ended stream or a restart), each OpenAI step that reads the
profile (9.5), and 9.7 admission's root profile; on a mismatch the call
that needed it is refused `{field: "profile", reason: "changed since the
participant allowed it"}`, an OpenAI step ends its turn `idle {failed
{error: refused}}` with that reason, and a `task` gets the refusal as its
output. So no session ever runs bytes other than those whose digest the
participant answered for. Hence:
- an edit of a profile changes its digest: no session of it, old or new,
  gets new work or a new launch until the participant allows the new
  version; a turn already running goes on with what it compiled at its
  start (`withdraw` ends it);
- allowing the new version replaces the entry's digest: new sessions start
  with it; sessions of the earlier version stay refused new work (their
  `profile-digest` names a version neither stored nor allowed), and the
  client starts new ones;
- a session with no `profile-digest` (one started before S151, or of the
  built-in `scratch` once `access/page` is stored) gets no new work through
  an entry.

**Allowing.** `manage.allow(client, profile)` returns at once (AGT-F-56: no
call waits):
1. `client` is `page` or listed in `clients`, else `refused {field:
   "client", reason: "not a client of the kernel"}`; `profile` is stored and
   in `profiles`, else `refused {field: "profile", reason: "no such
   profile"}`; a stale profile is refused `{field: "profile", reason: "its
   offer changed"}`.
2. If the entry already holds the stored digest, or an open question asks
   for the same client, profile and digest, it writes nothing. At 4 open
   questions it is `busy`; an entry list at 64 is `refused {field: "client",
   reason: "64 profiles"}`.
3. It renders the reach (below), calls `agent-access.ask(client, profile,
   reach)` (2.1) and writes `access-ask/<stream>` `{client, profile, digest}`,
   `stream/<stream>` `{kind: "access"}`, and the stream in `access-asks`. An
   open question for an earlier digest of the same client and profile is
   cancelled (`streams.cancel`) and its keys deleted in the same call. A host
   error is `backend("<code>")` and writes nothing; the request applies
   after the call commits (7).

The app asks the participant in its own words (8). The host keeps nothing:
the kernel keeps the permission, and its page shows and withdraws it
(AGT-F-57).

**The answer.** The host is the only producer of an access stream (2.1). Its
one item is exactly `{"type":"granted"}`; any other body is dropped and the
question ends without effect. On it the kernel reads `access-ask/<stream>`
and `profile/<profile>`: when the stored profile's digest still equals the
question's and the profile is not stale, it writes the entry (replacing that
profile's entry in place, or appending one); otherwise it writes nothing, and
the page shows that the question ended without effect. On the stream's
`ended`, whatever the outcome (`closed`, `consent-refused`, `cancelled`), it
deletes `access-ask/<stream>`, `stream/<stream>` and the `access-asks` entry.
A granted item after a `withdraw` of the pair finds no `access-ask/` key and
writes nothing.

**Reach.** The kernel renders the question's reach from the stored profile
alone, as plain text of at most 4 KiB, one line per item, in this order,
leaving out an empty line:
- the backend and model: "Claude Code, model {m}", "Claude Code, its default
  model" or "ChatGPT, model {m}";
- for Claude: "Can read and change files in the folders you choose" when
  `tools` holds a file tool, and "Can run programs and use the web, inside
  Claude's sandbox" when it holds a tool of the shell grant (4.2); "No
  built-in tools" when it holds none;
- for Claude with a shell-grant tool: "Programs may reach: {domains}" (the
  sandbox's network allowlist, 4.2), or "Programs have no network";
- "Tools: {names}";
- "Without asking: {allow rules}";
- who decides the other calls: with `ask: true`, "Asks {client} before each
  other tool call; {client} may allow any of them" for a client, "Asks you,
  in the agent console, before each other tool call" for `page`; with
  `ask: false`, "Refuses each tool call no rule allows";
- "Mode: {mode}, {meaning}" for any mode but `manual`, the meaning fixed per
  mode of 4.1 (`accept-edits`: "file edits run without asking"; `plan`:
  "plans only, changes nothing"; `dont-ask`: "refuses what no rule allows,
  asks nobody");
- "Toolsets: {names}", "Skills: {names}", "Delegates: {names} (each
  narrower than this profile)";
- "Limits: {field} {value}, ...";
- when `access/<client>` holds an entry for an earlier digest: "Changed
  since the version {client} has: {fields}" (the top-level fields whose
  values differ, by name).
Every name, rule and domain is shown in full, in the kernel's order above
(within a line, rules and names sorted), never the profile's; a character
outside printable ASCII is shown as `\u{XXXX}`. When the whole text would
pass 4 KiB, `allow` is refused `{field: "profile", reason: "too wide to show"}`
and asks nothing: the app never shows a cut question. The page shows the
whole profile beside the open question.

**Withdrawing.** `manage.withdraw(client, profile)` deletes that profile's
entry from `access/<client>` and cancels an open question for the pair (its
keys deleted), so every later call that gives the client's sessions of that
profile new work is refused at once (9.7, "Revoking"). Then it ends their
running turns: it reads the owner's `owner/<client>/<n>` pages and those
sessions' records, and for each session whose root's profile is that
profile (its own `profile` for a root, its parent's for a child, as 9.7
"Revoking" checks a child) and whose turn is `running`, `waiting` or
`paused`, it does what `cancel` does (9.3: the turn ends, open asks are
denied, live children are cancelled). A child whose delegation resolved and
that its owner gave more work is ended this way too, whether or not its
root's turn runs. It ends at most 16 turns per call and returns how many are
left; the page calls again until none is. Taking power away asks nothing:
`withdraw`, `forget` and `delete-profile` need no question.

**Profiles.** `manage.put-profile(name, profile)` stores a profile that
parses under 9.2 (the per-session fields refused, the grammar of 4.1; every
refusal at once, `refused`), at most 1 MiB, under a name of
`[A-Za-z0-9._-]{1,64}`, and returns its digest. It adds the name to
`profiles` (at most 256, else `refused {field: "name", reason: "256
profiles"}`) and drops the name's adoption link (the text is the
participant's own now). It writes no entry. `delete-profile(name)` is refused
`{field: "profile", reason: "allowed for {clients}"}` while an entry or an
open question names it; else it deletes `profile/<name>` and its `profiles`
entry. `manage.profiles(after, max)` pages through `profiles` with each
profile's text, digest, adoption link and stale flag. A profile stored before
S151 without a `profiles` entry still runs by name; the manager lists it once
it is stored again.

**Offers.** The kernel imports `mods:agent/offers` as a collect (watane W34,
2.3). `manage.offers()` calls `offers.profiles()` once. It keeps each
answer whose `profile` is at most 64 KiB and parses under 9.2, at most 64 in
all, and returns each with its digest (the SHA-256 of `profile`) and the
names of the profiles adopted from an offer of that name and digest. A
collect's answers carry no source (W34), so neither the kernel nor the page
names the offering mod, and two offers may share a name.
`manage.adopt(offer, digest, as)` collects again and takes the offer with
that name and digest (none: `refused {field: "offer", reason: "no longer
offered"}`). It stores the offer's profile as `profile/<as>`, as
`put-profile` does, with the link `{offer, digest}` in the `profiles` entry
of `as`; an existing profile `as` that is not adopted from an offer of that
name is refused `{field: "as"}`. Adopting writes no entry: the mod that
offered it, whichever it is, gains nothing until the participant allows a
client (**Allowing**). When `manage.offers()` finds offers named like an
adoption's `offer` but none with its digest, it marks the adoption `stale`
in `profiles`. A stale profile is refused for new sessions (`start`,
`check`, `allow`; 9.7 admission step 3 reads it as unadopted) until the
participant adopts it again, which gives it a new digest, so each client's
entry needs a new answer, or edits it (`put-profile` drops the link).
Sessions already started keep their version (**Pinned entries**). The kernel
learns of a changed offer only when the page collects; until then the
adopted text runs, never the offering mod's new text. Skill offers keep the
rule of 9.2 and `adopted/`.

**Bounds.** One `manage` call reads at most `clients`, `profiles` (256
entries of at most 256 bytes), the `access/` values of the listed clients
(64), one profile (1 MiB) and one collect (64 offers of 64 KiB); `withdraw`
also reads the owner's pages and records, bounded by the per-owner session
cap (9.3 Quota), and ends at most 16 turns. `start`, `start-in-folder` and
`check` read `profiles` once more than before, for the stale flag, and each
call that gives a session new work reads its `profile/<name>` (at most
1 MiB) to compare the digest. None of
these keys is charged to an owner (9.3 Quota): they are the kernel's own,
bounded by these caps.

## 10. The providers (M1)

- **Unchecked:** `start` checks the caller (2.2), calls `claude-code.start`
  with the launch unchanged, then, while it is still the stream's consumer,
  `claude-session.say` with the input's text (2.1, "The kernel's calls on a
  provider stream"; none for an input with no text), then `streams.hand-off`,
  and returns `{stream, direct: true}`. A `start-error` of the host passes
  through unchanged; a `say` error cancels the stream and is `bad-launch
  {field: "input", reason: <its case>}`. Its other functions return
  `not-direct`. The kernel identity it compares with (2.2) is, until the
  kernel is published, `local/agent` (PLAN R-B3). The test release builder
  compiles the provider with exactly the test channel's `<publisher>/agent`
  identity. Each build accepts one identity.
- **Checked:** it forwards every item unchanged (an `ask` item reaches the
  kernel at once; only its answer waits for the verdict, step 6), answers every `check`
  (`none` when safe, `deny` otherwise), judges every `ask` and input, and
  pushes each verdict as an item
  `{"type":"verdict","on":<on>,"outcome":"safe"|"unsafe"|"unsure","reasons":["<text>",…],"denials-in-a-row":<n>}`
  (`on`: step 3; `outcome` is the one name for a verdict here and in 9.4, S127). No
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
     unanswered, and the kernel's broker and the owner decide (9.6). The host
     reports such an answer as `ask-resolved {by: answered, outcome}`, which
     the kernel records as a `by: checked` decision, as it did not answer
     itself (9.8). So "safe"
     never widens the mode. An owner's `allow` to an ask whose verdict is
     pending is forwarded only after a safe verdict; the kernel keeps it as a
     pending answer, and it is recorded by the host's `answered` (the one
     decision, 9.8 rule 4), not at the kernel's `answer`. After an unsafe
     verdict the provider's `deny` goes first: the host pushes `answered`/
     `deny`, and the one decision is `deny, by: checked`; a later answer is
     `unknown-ask`. Claude's own hook
     timeout (`timeout-s`, `onFailure: block`) still stops the tool if the
     provider never answers.
  7. When its own stream to the kernel is full, it defers the item call and
     resumes on `ready` (3). Its `resume` is made in a later call, never in the
     call that returns `defer`. A call that cannot defer (a deadline timer,
     `ended`, `say`) still acts at once (a deadline is final when it fires;
     its `deny` goes out), and its item (the verdict, the breaker's reason)
     waits in the session's outbox in its state, sent first at the next
     `ready`; no item is forwarded past it, so the kernel always learns why
     (S127 r1 F1.1, F1.2). Not guaranteed (S127 r2 F2.1, the deputy's ruling):
     when the session ends while the kernel's stream is still full (a launch
     deadline, `claude` ending before `ready`), the outbox goes out only as far
     as the stream takes it before the close, so the kernel may miss that
     verdict's reason; the denial and the cancellation still happen at once,
     and the turn still ends failed. A `say` the host refuses after a safe verdict ends
     the session: `claude` cancelled, the own stream closed `failed("say:
     <case>")`, so the kernel's turn ends with that cause (S127 r1 F1.4).
  8. `interrupt` drops every input of the session still waiting for its
     verdict or for `initialized`; a later verdict for one sends nothing.
- **Callers:** every function of `sessions` first checks that
  `caller.identity()` is the kernel `mods:agent`'s and otherwise returns
  `not-owner` before it judges, opens a stream or makes any call; so no other
  mod can make the provider spend the participant's judge key (2.2).
- **Judge call** (S127, deputy decision 2026-11-25 "judge wire shape"): the
  judge is TypeSafe System One. `https-stream.start` with `account: "judge"`
  (5), `POST <origin>/v1/systemone`, header `content-type: application/json`
  only (the host adds the key), body
  `{"model":"jev-latest","state":{"kind":"tool"|"input","tool":"<name>"?,"input":<object>?,"text":"<input text>"?,"transcript_excerpt":[{"role":"user"|"assistant","text":"<text>"}]?},"questions":{"unsafe":{"type":"noul","instructions":"<text>"},"off_task":{"type":"noul","instructions":"<text>"}}}`:
  every question in one request, each phrased so that yes means harm
  (`model` is required; the answer may name another version).
  **Excerpt** (S127 D1.2: without it `off_task` cannot be judged and an
  ordinary edit falls in the unsure band): the person's latest input that
  reached `claude` (its first 2 KiB) and the assistant's latest text after
  it (the text blocks of its last `assistant` line, its last 2 KiB), oldest
  first, the empty ones left out; an input is judged with the turn before
  it; a denied input never becomes the excerpt's request. Kept in the
  provider's state with the session, so at most 4 KiB per request. `<origin>` is the provider's build origin
  (`JUDGE_ORIGIN`, default `https://api.typesafe.ai`, the one host of its
  `[http]`); the participant enters the same origin with the key in the app
  (5), and the host refuses any other (`destination`). Deadline = the
  launch's `pre-tool-check.timeout-s` minus 2 s; at most 3 attempts inside it
  (as in step 3). **Reading:** a `200` whose body is a JSON object with
  `answers.<question>.noul` (the probability of yes) a number from 0 to 1 for every question
  asked; anything else (another status, a missing question, a value out of
  range, a body that is not JSON) is unreadable. **Mapping**, with the
  thresholds of its page (default `unsafe-at` 0.7, `safe-at` 0.3, the
  docs' starting values): `unsafe` when any question scores at or above
  `unsafe-at`, `safe` when every question scores at or below `safe-at`,
  `unsure` between (the band). **Reasons** are the provider's own text, one
  per question, `"<question>: <score, 2 decimals>"`. Unreadable, a timeout or
  an error is unsafe; so is a refusal of the account (`destination`,
  `no-account`), and a missing or wrong key therefore denies. All of it is in
  `provider-checked/logic/src/judge.rs`; another judge API is a provider
  derived from this one (an adapter per judge API), not a setting. The shape
  is the one the deputy's V7 probe read from Jev (2026-11-25, S127 D1.1):
  `{"model":"jev-1.14.0","answers":{"<question>":{"type":"noul","noul":0.16},…},"usage":{…}}`;
  errors are 400 `{"detail":…}` and 422 `{"detail":[…]}` (unreadable). A
  change there is one function of `judge.rs`, and until it matches every call
  is unreadable, so it denies.
- **Breaker:** 3 denials in a row or 10 of the last 50 verdicts of a
  session: `claude-session.interrupt`, and a verdict item
  `{"type":"verdict","on":"breaker","outcome":"unsafe","reasons":["<rule>: the session was stopped"],"denials-in-a-row":<n>}`.
  Every verdict item carries `denials-in-a-row`; a safe verdict resets it.
- **Checking:** before each judge call the provider pushes
  `{"type":"checking","on":<on>}`; the kernel shows it as the
  activity `checking` (9.3) until the verdict. The kernel takes `verdict` and
  `checking` only from a checked provider's stream (`direct: false`); on a
  `direct: true` stream they are `unknown` entries (9.8).
- **Settings** (the thresholds `unsafe-at` and `safe-at`, `0 <= safe-at <
  unsafe-at <= 1`) only from its own page (`settings.set`, caller `page`), in
  its state; the page also shows the build origin, for the participant to
  enter in the app. No judge URL setting: the origin is entered in the app
  only (5). The provider holds no key: the host keeps it as the account
  `judge`, entered in the app (5; N10, the human, 2026-11-20).

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
| `skills` | plugin folder | the `skill` function below. Each skill must contain exactly `SKILL.md`; missing or extra files are refused on `skills[i].files` |
| `subagents`, `subagent-depth` | flags, env | refused |
| `delegates`, `subagent-limits` | refused | stage F, `task` and root limits (9.7) |
| `sandbox`, `workspace` | settings, cwd | refused unless absent |
| `json-schema` | flag; the kernel checks the answer; a missing `structured_output` fails | `text.format`; a schema outside the strict subset refused; the kernel checks the answer |
| `max-turns` | native flag and result (9.7) | stage F, completed model steps |
| `max-tool-errors`, `max-tokens` | refused | stage F, per-turn counters |
| `max-budget-usd` | flag | refused |
| `ask-expiry` | refused unless 600 (seconds) or absent | an `ask` deadline of the session timer (9.3) |
| `persist` | flag | `false` keeps the transcript in memory only; not built for OpenAI yet (S134 r1): an OpenAI session stores its transcript and wire whatever `persist` says, a root and its children alike, a known limit until a slice builds it (PLAN, Upkeep) |

On ordinary requests, a session with skills advertises `skill(name: string)`
with exactly one required field. Its description lists the effective profile
skill names and their frontmatter descriptions (4.1). The listing treats names
and descriptions as text, escaping `&`, `<` and `>`. The broker applies
profile rules to the function name (9.6). A permitted call returns the exact
`SKILL.md` from the session's compiled skill snapshot and completes a
regular tool row. An invalid argument or a name outside that snapshot gives
a failed tool output with its reason. Children compile from the delegate
profile checked at admission (9.7), including only the skill names and file
contents whose name and SHA-256 digest match the root's snapshot. The
kernel writes each file at session creation and deletes it with the session
without reading the file. The rendered listing is part of the loop state.

A skill accepted at check time can still exceed the Responses body budget
(9.5). JSON escaping can expand the file contents, and the next request
also carries the listing and wire history. An oversized request follows
the loop's existing trimming rules. A skill whose escaped body alone
exceeds the budget cannot be loaded. Its call ends the turn `too-large`.

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
| `too-many`, `bad-id`, `bad-delay` | timers | 7 (`bad-delay`: a delay over 30 days) |
| `not-owner`, `unknown-session`, `refused`, `queue-full`, `quota`, `paused`, `backend`, `busy`, `too-large {limit}` | kernel | 9, 9.3, 9.5 (`busy`: no provider or request slot, 64 pending answers on the session, 1 024 active sessions, 9.3, A3, or 4 open access questions, 9.10); `manage` (9.10) uses the same codes: `not-owner` for any caller but `page` |
| `not-page`, `no-mod`, `too-large`, `too-many` | agent-access (proposed) | 2.1, 9.10 |
| `not-owner`, `not-direct` | providers | 2.2, 10 (`not-owner`: the caller is not the kernel) |
| `unknown-input`, `unknown-allow`, `behind {through}` (`through`: the high-water index) | kernel | 9.3, 9.6, 9.4 |
| `too-large` | OpenAI loop outcome | 9.5, body budget |
| `failed {timers}`, ask denial `timers` | kernel turn outcome, ask reason | 9.3 (no timer slot at the cap of 240, or at the owner's 120) |
| ask denial `expired` | kernel ask reason | 9.6 (a kernel ask reached its `ask-expiry`) |

Each error also carries a message that names the cause (F-QUAL-3).

## 13. Libraries

| Use | Library | Where |
|---|---|---|
| stream-json types | `claude-codes` (types feature) | kernel, dev's fake `claude` |
| event format | `agent-client-protocol-schema` (v1) | kernel, clients |
| HTTP in a mod | none: `https-stream`; `wstd` only for mods that use `wasi:http` | – |
| SSE parsing | `sse-stream` | host (H3) |
| MCP types in a mod | `tower-mcp-types` | kernel, if it reads MCP shapes |
| URL parsing (WHATWG) | `url` | kernel: the `WebFetch` scope key (9.6) |
| SHA-256 digest | `sha2` | kernel: skill revisions (9.7, 11) |
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
Both providers are in this repository (`provider/` and `provider-checked/`,
R-B1), so this one list covers their files; `provider/BOUNDARIES.md` is a copy
of the provider rows, and the checked provider's folder keeps one too (17,
stage E).

| Property | Holds by | Files |
|---|---|---|
| A mod cannot widen `claude` past its launch | 4.1 grammar and one argument per value; `--restricted`; fixed flags; closed subagent keys; the host owns stdin (4.5); `initialize` and `init` checks | watane `host/src/agent/claude/launch.rs`, `init.rs`, `control.rs`, `run.rs` |
| A mod cannot run a denied skill by name | `slash-command` (4.5) | watane `host/src/agent/claude/control.rs` |
| A failing check cannot let a tool run | `onFailure: block` confirmed in `initialize` (4.5) | watane `host/src/agent/claude/init.rs`, `control.rs`; `provider-checked/mod.toml`, `provider-checked/logic/src/judge.rs`, `provider-checked/logic/src/lib.rs` |
| "Safe" cannot widen the mode | checks answer `none` or `deny`, never `allow` (4.5); the checked provider answers an ask only `deny`, a safe ask goes to the owner (10) | watane `host/src/agent/claude/control.rs`; `provider-checked/logic/src/judge.rs`, `provider-checked/logic/src/lib.rs` (a safe ask goes on unanswered) |
| Writes cannot plant code that runs later | protected paths (4.3) | watane `host/src/agent/claude/run.rs` (the host's `--settings=`) |
| No mod sees a credential | tokens and the judge's key in the app; MCP tokens on fd 3; handle names, not handles (5, 6) | watane `app/` account files, `host/src/agent/https_stream.rs`, `host/src/agent/tools.rs`; `mod.toml` |
| A plan token goes only to OpenAI's agent endpoints | account binding, forbidden headers, no redirects (5) | watane `host/src/agent/https_stream.rs` |
| A judge key goes only to its judge | the account binding of `judge` (identity, value, origin parse and lifetime as 5): the provider it was entered for, `POST`, the account's origin in the provider's `[http]`, no redirects (5); the same binding means the session's excerpt (the tool call, its input and the 4 KiB transcript excerpt of 10) leaves the machine only to the participant's own judge | watane `host/src/agent/https_stream.rs`, `app/` judge account; `provider-checked/mod.toml`, `provider-checked/logic/src/judge.rs`, `provider-checked/logic/src/lib.rs` |
| A mod in the middle gains no authority | host consent (8); the providers' `sessions` compare one pinned kernel identity (2.2, 10) | watane `host/` consent, `app/` consent storage; `mod.toml`; the providers' `mod.toml` and `src/lib.rs` (the unchecked one: `provider/mod.toml`, `provider/wit/world.wit`, `provider/logic/src/lib.rs`, `provider/BOUNDARIES.md`); the checked one: `provider-checked/mod.toml`, `provider-checked/wit/world.wit`, `provider-checked/logic/src/lib.rs`, `provider-checked/BOUNDARIES.md`; `factory/publish-test-mods.sh` pins the release identity |
| A session is private | one consumer; owner checks by identity (3, 9.3) | watane `host/src/agent/streams.rs`; `kernel/src/lib.rs`, `kernel/wasm/src/lib.rs` and `kernel/wasm/src/component.rs` (the exports and owner checks by `caller.identity()`, the provider calls of 2.1), `kernel/src/sessions.rs` (also the 9.1 stream routing, `stream/`, `adapter/` and `live/`, and the session timer of 9.3; a slice that puts either in another file names that file here), `kernel/src/call.rs` (when a call's session timers and buffered state commit, 2.1), `kernel/wasm/wit/deps/watane-host/host.wit` (resolved firing origin, 2.1), `kernel/src/openai/run.rs` and `kernel/src/openai/tools.rs` (the `request` and `job` streams, routed by `stream/<id>`, S40), `kernel/src/transcript.rs`, `kernel/wasm/wit/world.wit` (later `wit/world.wit`) |
| A child cannot widen its owner's authority | root access and named profile before load, effective-profile narrowing and depth/live checks at dispatch (9.7); owner, origin and consent checks still apply | `kernel/src/delegation.rs`, `kernel/src/openai/task.rs` (dispatch, the child's start and its one resolution), `kernel/src/openai/skill.rs` (reads from the checked skill snapshot), `kernel/src/openai/run.rs`, `kernel/src/openai/tools.rs`, `kernel/src/named.rs`, `kernel/src/sessions.rs` (`may_work`: a child's calls checked against its root's access) |
| A runaway agent stops | counters and delegation limits in the kernel (9.7); the OpenAI loop's slots, retries and pauses (9.5) | `kernel/src/delegation.rs`, `kernel/src/openai/` (`run.rs`, `tools.rs`, `task.rs`), `kernel/src/quota.rs` (the slot limits) |
| A repeated call changes nothing | client keys, job keys, numbered items (9.3, 6, 3; the kernel drops an item at or below the last `seq` it took, 2.1) | `kernel/src/sessions.rs`, `kernel/src/call.rs` (a failed call commits no write, id or timer, 2.1), `kernel/wasm/src/lib.rs`; watane `host/src/agent/tools.rs`, `streams.rs` |
| An ask shows the tool call that will run | the ask carries the call's own id and input as the adapter read them (4.5, 9.6, 9.8); the host's items carry `tool_use_id` and `expires-at`, and no stdout line passes as a host item (4.5); the kernel reads host items strictly (9.8) | `kernel/src/adapters/`, `kernel/src/broker.rs`, watane `host/src/agent/claude/control.rs` |
| A session allow can be taken back | the kernel holds it, `claude` never gets it as a rule (9.6) | `kernel/src/broker.rs` |
| A session allow covers only what the participant saw | each tool's scope function (9.6): a later call is allowed only when its key, computed afresh from the input that will run, equals a stored (tool, key) exactly; a tool with no scope function (`Task`, `WebSearch`, `mcp__watane__<tool>` until that tool names its key) and an input with a key the function does not know get no key and always ask; file keys rely on the host resolving symlinks first (the kernel reads only the item's resolved `path`, under the record's `folder` from the first `system/init`'s `cwd`, never the input's path; no `folder` or no `path`, no key); a session allow is refused unless its key renders to the `reach` its ask was opened with (S36 r1) and an `mcp__watane__` key on `--strict-mcp-config` (15) | `kernel/src/broker.rs`, `kernel/src/sessions.rs`, `kernel/src/record.rs`; watane `host/src/agent/claude/control.rs`, `run.rs` |
| The console has no power a client lacks | its code calls only `sessions` of 2.3, as the kernel's page identity (18.1); the profile manager in the same page calls `manage`, and every widening there waits for the participant's answer in the app (9.10), so a script in the page can ask, never grant; its one first-run default, the built-in `scratch` profile for `page` while `access/page` is absent (9.1), grants less than any profile a participant makes (PLAN R-B4) | `wit/world.wit` (staged as `kernel/wasm/wit/world.wit`, 1), `mod.toml`, `kernel/src/lib.rs`, `kernel/wasm/src/lib.rs`, `console/`, `manager/` (planned, 1), `ui/` |
| Only the participant gives a client a profile (S151) | one writer: `manage`, callable only by `page`; an access entry is written only on the host's granted item of an `agent-access` question whose words the app owns and which names the client as watane knows it, the profile and its reach; a client is listed only by its own call; an entry pins the profile's digest, and a changed or stale profile allows nothing new; absent or unreadable entries allow none; adopting an offer writes no entry; `withdraw` refuses new work at once and ends running turns (9.10, 9.7 "Revoking") | `kernel/src/manage.rs` (planned, 1), `kernel/src/named.rs` (the pinned entries read by `start`, `check`, `profiles`), `kernel/src/sessions.rs` (`may_work`: the record's `profile-digest`), `kernel/wasm/src/lib.rs` (`manage`'s owner check by `caller.identity()`, the access stream's routing), `kernel/wasm/wit/world.wit`; watane `host/` (`agent-access`, planned), `app/` (its consent words) |
| A profile or overlay cannot widen what the participant set | compilation and the refuse table (9.2, 11; AGT-F-3, AGT-F-4) | `kernel/src/profile.rs` (also the OpenAI rule names, 9.6), `kernel/src/overlay.rs`, `kernel/src/refuse.rs`, `kernel/src/check.rs`, `kernel/src/named.rs` (`check` and `profiles` read `access/` first, 9.1), `kernel/src/error.rs` |

Not protected: a turn already running when its profile is edited goes on with what it compiled at its start (no new work or launch follows, 9.10); a running turn is ended only by `withdraw`, at most 16 per call (the rest are refused new work at once and end on the page's next call, which a closed page may never make); a `local/<slug>` identity that a removed mod held and another mod later takes keeps that identity's entries, as an update under the same identity does, since the kernel hears of no removal (install-generation binding, S133 P1.12, later); a script running in the kernel's page (say, through rendered agent output), or an agent whose sandbox reaches loopback (V27) and calls the kernel's page interface as the page would, can ask the app's question, adopt offers, edit profiles and take access away, but no entry follows without the participant's answer, and an edit allows nothing new; the participant's own answer to a wide profile; a changed offer is seen only when the page collects (9.10); the judge account reaches any `POST` path of the origin the
participant entered; `claude` itself within its limits; a prompt injection within
what the profile allows; an allowed package-runner word (`test`, `install`, `add`, `ci`, `uv pip`, `sync`, ...) runs the project's own scripts, and an allowed command's own options (`git -c`, GNU `sed`'s `e`) can run others, and so can an allowed first word that the list of 4.1 misses (no such list is complete): inside the shell grant's sandbox (4.2) only, whose allowlisted names can reach loopback, LAN and link-local addresses (V27, breached; the shell grant waits on the human), and only for a profile the participant made or adopted (AGT-F-2) with that allow rule; kept client keys are charged forever (9.3), so owners acting together can fill the kernel's account for good: an accepted limit until a later slice frees kept keys (PLAN); plan usage; another program of the same user
(spec 6); a reader that takes the first of two equal keys would read a stored line `{"i":7,…,"i":4}` differently (every reader here, serde_json and `JSON.parse`, takes the last, and the `i` of a line's place overrides it anyway, 9.4); a duplicate key nested in an ask's `input` is read last-wins, so the ask shows the call that will run only while the host serializes the item itself (watane `host/src/agent/claude/control.rs`, 4.5) rather than passing `claude`'s bytes (the host rule of 4.5 step 4); a checked provider, a trusted holder that forwards every host item, can push its own `ask-resolved` or a false `outcome`, and no rule tells its item from the host's on a `direct: false` stream: the record then shows a decision `claude` did not get, or closes an ask the host still holds (the host denies it at its `expires-at`), the record only, since no tool runs because of it (the kernel's answers reach `claude` only through that provider, which may deny any call anyway, 10); a check whose hook times out: `claude` starts its hook timer when it writes the line, a little before the host pushes the item, so an answer that lands in that gap is recorded (`answered`, `none`) while `claude` already blocked the tool, the record only (the checked provider answers 2 s before its deadline, 10, so only a stuck provider reaches it); at the kernel's own closing steps (`cancel`, R4, the provider stream's `ended`, 9.8 rule 4) the decision can differ from what `claude` did: a pending answer is recorded although the request may not have applied or the checked provider may have held it (an `allow` the judge never cleared, while no tool runs, as the process or stream is gone), and an open check is recorded `deny, by: kernel` although the provider's `none` may have reached the host, and the tool run, with its `answered` item lost at the stream's end: the record only; a Bash session allow holds the text of one command, and that command can reach other files after an allowed `cd` or a change to a file it reads (inside the sandbox, 9.6); a `WebFetch` session allow on a host also reaches the same host with or without `www.` when the allowed server redirects there (`claude`'s own redirect rule, 9.6). The subagent slash-prompt probe V26 held and the shell network probe V27 breached (16).

**Known limit (S38):** Host 19.7 uses `gone` for consent withdrawal and other
provider-stream endings. The console displays “Connection to Claude Code ended”
and keeps the transcript through the last finished message.

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
- [ ] `host/TECHNICAL.md` 8.5: the step 11 `watane.js` passes the `connection` message (`{type, id}`) to the page's handlers too, so a page learns that its `/events` connection was replaced and reattaches its streams (18.2; S32). Without it, a `changed` cannot be told from a reconnect.
- [ ] `host/TECHNICAL.md` host state: timers per mod, kept across restarts (7).
- [ ] `host/TECHNICAL.md` 19.1 and 19.6: add `timers.firing-origin: func() -> option<string>` to the existing timer import. Return the resolved origin client only to the timer owner's top-level `timer-sink.fired(id)`, including the timer-owner fallback after removal or disable. Keep the existing durable timer path for restart work. Keep the host caller and all authority checks (agentic-harness 2.1, 7).
- [ ] `host/TECHNICAL.md` `caller.identity()` (9.3, 14).
- [ ] `host/TECHNICAL.md` new interface `host-info` with `generation()`: the start number, raised once per host start, kept in host state, a pure read, no grant (2.1, 9.3; G1; accepted, the human, 2026-11-21).
- [ ] `host/TECHNICAL.md` `claude-types`: `start-error` and `session-error` also carry the providers' `not-owner` and `not-direct` (2.2, 10, 12).
- [ ] `host/TECHNICAL.md` H1: `stream-sink.items` returns `taken` or `defer`; `streams.resume`; a deferred batch is not a failure; `stream-producer.ended`; a lag clock from a failed `full` push to `ready` that reaches 10 s ends the stream `lagging`, except a stream its consumer has deferred (2.1, 3; P03).
- [ ] `host/TECHNICAL.md` H2: the `initialized` item after `initialize` passes, never before the workspace consent is held (4.5, 10; P04); `claude-session.say`, `answer`, `interrupt` and `compact` are requests, applied after the call commits, so a deferred or failed call sends nothing to `claude` (2.1, 7); the host builds each `ask`, `check` and `ask-resolved` item itself and never copies bytes of a `claude` line into one, and `ask-resolved {by: answered}` carries the `outcome` (4.5; accepted, the human, 2026-11-21); the host pushes `ask-resolved {by: expired}` for a check it has not answered at its `expires-at` and writes nothing to stdin for it (4.5; S22; accepted, the human, 2026-11-21).
- [x] `host/TECHNICAL.md` H2: scratch workspaces (4.6): `workspace: "scratch"` makes one at apply, `initialized` names its id `scratch:<32 hex>`, a later launch names it again, the owner kept in host state (host 19.3 "Folders", watane S99).
- [x] `host/TECHNICAL.md` H2: a file tool's ask gives the kernel the path that will run with its symlinks resolved, before the kernel computes a scope key: the item's top-level `path` (4.5 step 4; host 19.3 step 4, watane S100), which the kernel scopes under the record's `folder` (4.6; S36).
- [ ] `host/TECHNICAL.md` H2: `--strict-mcp-config` admits only the `watane` server, so an `mcp__watane__<tool>` name is always watane's (4.4); a scope key for such a tool (9.6, 14) relies on it (PLAN S33 RISK 3, S34).
- [ ] `host/TECHNICAL.md` H2: the host's `--settings=` for the shell grant always writes `sandbox.allowUnsandboxedCommands: false`, never leaves it out (Claude's default is `true`); a test reads the settings the host writes (4.1, 4.2, 14; PLAN S33).
- [ ] `host/TECHNICAL.md` H1: byte budgets for queued items (512 MiB host, 128 MiB per origin client, 16 MiB per page connection; `full`; a page connection past its budget ends `lagging`) (3; P11).
- [ ] `host/TECHNICAL.md` tool annotations `@parallel`, `@idempotent` (6).
- [ ] `host/TECHNICAL.md` a request that waits on a consent tells its requester `consent-pending {what}` once (7, 8).
- [ ] `host/TECHNICAL.md` account kind `judge` and its binding (provider identity compared exactly, `account` exactly `openai` or `judge`, `POST`, origin after one parse with `[http]`, no redirects, `no-account`, ending live judge streams when the key is removed) (5, 10, 12; N10).
- [ ] `pkg/TECHNICAL.md` 4.5: rows for the imports and exports of 2.1, `host-info` among them (2.1).
- [ ] `pkg/TECHNICAL.md` 4.5: the changed `stream-sink` export, the `stream-producer.ended` export and the `streams.resume` import (2.1; P03).
- [ ] `pkg/TECHNICAL.md` 5.1 and 7.2: two grant kinds (4.2, 8).
- [ ] `app/TECHNICAL.md` new section: Sign in with ChatGPT and the account file (5).
- [ ] `app/TECHNICAL.md` new section: the consent words and their storage (8).
- [ ] `app/TECHNICAL.md` new section: the `claude` binary setting (4.4).
- [ ] `app/TECHNICAL.md` new section: entering, showing (never the value) and removing the judge key and origin on the checked provider's manager page, with the origin shown next to the key and never prefilled, stored in `<home>/app/accounts/judge/<provider mod id>.json` and deleted when the provider is uninstalled (5, 10; N10).
- [ ] `dev/TECHNICAL.md` new section: the fake `claude` and the scripted Responses server, which also serves a judge origin: it counts every request it receives and plays a judge that answers safe, unsafe or unsure, is down, slow or unreadable, or answers a redirect (12, 16, 17).
- [ ] watane licence files: MIT OR Apache-2.0 for all of watane (13; N8).
- [ ] **Proposed by S151, not yet accepted** (a consent-boundary change, for the human): `host/TECHNICAL.md` 19.1 and 19.7: the interface `agent-access` with `ask(client, profile, reach)` as 2.1 "Access questions" writes it: a request, only in a chain that started at the calling mod's own page (`not-page`), `client` an installed mod other than the caller or `page` (`no-mod`), `reach` plain text of at most 4 KiB, 4 open per mod, a host-produced stream with one `granted` item then `closed`, `consent-refused("profile:<client>:<profile>")` on a refusal or after 600 s, refused at once in a preview or with no app, `streams.cancel` withdraws the question, nothing kept; `Confirm::consent` gains the kind `profile-access {client, profile, reach}`; dev 19.9 gains a fake answer for it (granted or refused) for previews and tests (2.1, 8, 9.10).
- [ ] **Proposed by S151, not yet accepted**: `app/TECHNICAL.md` 19.2: the words of the profile access question (8), {client} and {kernel} as the names watane shows, {reach} shown as plain text, never markup (8, 9.10).
- [ ] **Proposed by S151, not yet accepted**: `pkg/TECHNICAL.md` 4.5: a row for the `agent-access` import (2.1). Open for the human: any mod may import it and raise a question about its own profiles, which grants nothing in the kernel but can confuse the participant; pkg could limit the import to `mods:agent` (S151 adversary).

## 16. Test plan

Integration tests are modules of one binary per subsystem
(`<sub>/tests/it/<file>.rs`). The kernel crate is tested with `cargo test` in
`kernel/` until the WASM wrapper exists (then `watane-dev test`); the
providers with `watane-dev test`; the console with `npm test` in `console/`
(18.7).

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
| V7 | latency of one judge check (`pre-tool-check`) on a real judge account | done, the deputy on the Mac, 2026-11-25, on Jev (TypeSafe System One): p50 173 ms, p95 267 ms, max 846 ms per call, a new TLS connection each, 2 questions | `ARCHITECTURE-PROPOSAL.md` section 18 |
| V8 | the stage A prototype through three layers (host stream, a mod consumer, a page consumer): order, delay and the cost of one fsync per commit (7, N4) | stage A | "V8 record", at the end of this section (numbers; whether to change 7 is the human's) |
| V26 | whether a subagent prompt that starts with `/` runs a skill (14) | S1, held | `ARCHITECTURE-PROPOSAL.md` section 18 |
| V27 | whether the shell grant's sandbox blocks loopback (14) | S1, breached | `ARCHITECTURE-PROPOSAL.md` section 18 |

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
    non-idempotent one gets `aborted`. (OpenAI: the idempotent half is not
    yet built; every open job gets `aborted` and the turn becomes `unknown`,
    9.5 "After a restart".) A durable pause can resume when the host
    resolves the firing to its bound owner. A missing, kernel fallback or
    foreign origin stops locally, including for a timer armed through a
    front mod. No model request, toolset operation or consent follows.
23. On OpenAI a step at `max-turns` that calls tools gets one tool-free
    summary, a step at `max-tokens` dispatches no calls, and the tool-error
    cap aborts remaining work. Verify streak reset on success and all
    counters reset on new input. On Claude a native max-turns result and
    idle report that limit without a summary; `max-tokens` and
    `max-tool-errors` are refused. Verify stage D's OpenAI usage sample
    and stage F's Claude end-of-turn sample or meter clear. A profile
    with `max-tokens` whose response lacks token counts fails the turn
    `usage-unavailable`; a denial counts in the tool-error streak.
24. An OpenAI root starts two named children; an unnamed profile, a
    grandchild, a third live child at a cap of two and a widening profile
    get the expected refusal fields. Successful dispatch commits one
    child link and gives exactly one terminal output. The owner can open
    each child's transcript after reattach, a different owner cannot.
    Cancel the root with two live children and see both stop. Restart
    with live children, including one paused, and see `aborted` calls,
    retained links, and no child or parent resubmission or late output. An owner input may
    continue a session under R5 without rejoining an old call. An
    `interrupt` of a live child resolves the parent's call `interrupted`,
    and a `cancel` or `delete` of a live child resolves it `aborted`. A
    toolset that describes a function named `task` in a root that names
    delegates is refused on `toolsets` before any request; in a session
    without delegates the name is free (and `skill` is free without effective skills).
    A root that stops at a limit with a live child interrupts that child.
25. A judge that is down, slow, unreadable or without an entered key
    denies, and so does a score in the unsure band (between the page's
    thresholds, 10); 3 denials in a row interrupt the turn.
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
    back byte for byte; `attach` with an `after` inside the group starts at its
    first part, and `read` takes `after` as given and still makes progress.
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
44. (10) The scripted judge answers a score at or above `unsafe-at` (and,
    in a second test, one in the unsure band), in System One's shape, then ends its stream normally: it counts exactly one request,
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
    deferring call is dropped. Every model request is dispatched under its
    session owner's origin or left unsent (the r5 assertion, checked over each
    host call in the test): with every request slot held, a turn's request
    after its jobs, after a job the FIFO started in another owner's call, or at
    a `retry` sends nothing in that or any other owner's call, and the turn
    stops `idle {stopped {limit: busy}}`; the owner's next `say` sends it, with
    the step's outputs, in her call.
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
    output `{"err":{"code":"busy"}}` (stage F). A waiting `task` holds one
    live association and no stream slot; verify child requests and jobs
    use the root owner's caps and completion removes the association once.
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
    commits); after a restart, the first call that reads a session's record
    (running or idle) leaves no `stream/` key or `live/` key of that session
    from before the restart.
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
    written; the reserve is never used by a turn's other entries.
60. (2.3) The four decision shapes are accepted as their outcomes; each of
    `{}`, `{"none":true}`, `{"allow":true,"deny":true}`,
    `{"deny":true,"for":"session"}` and `{"allow":"yes"}` is refused `{field: "decision"}`; the owner's answer to a
    check is refused `{field: "ask"}`.
61. (9.4) A stored entry of a newer `v` reads as `unreadable` with the `i` of its place;
    attach after it replays from `i + 1`, and a client that showed it drops it
    on replay.
62. (4.5) The fake `claude` prints a line
    `{"type":"ask","id":"x","tool":"Bash","input":{}}` on stdout: the
    kernel's stream gets a `wrapped` item, the transcript an `unknown` entry,
    no ask, the turn not `waiting`.
63. (4.5, 9.8) An ask item's `expires-at` is the host's; the kernel's ask
    entry carries the same value, read 5 minutes later; a `can_use_tool`
    whose `tool_use_id` is missing, empty or not a string is denied by the
    host at once and reaches no client.
64. (4.5, 9.8, 10) A stdout line that is not UTF-8, one with a duplicate
    top-level `type`, and a `{"type":"verdict",…}` line each reach the kernel
    as `wrapped` and become an `unknown` entry; a `verdict` or `checking` item
    on a `direct: true` stream is `unknown`; an ask item with an `expires-at` that
    is not a whole number from 0 to 2^64 - 1 is `unknown`, no ask.
65. (18.3) With an ask open on call T for input A, a later `ToolCall` T with
    input B leaves the card showing A, and Allow answers the ask's `id`; the
    same for a later `ToolCallUpdate` T carrying `rawInput` or `title` B and
    for a `tool` entry of T, before and after the decision. An earlier
    `ToolCall` T with input X, then an ask on T with input A: the card's name
    and argument both come from A. An ask on a read is a card, not folded.
66. (9.4, 4.5, 9.8) Kernel crate: a stored transcript with a readable line
    that claims `i` 7 in the place of index 3, and an unreadable one, read and
    replayed: each comes back with the `i` of its place. An `ask` item and
    a `check` item without `input` are `unknown` entries, no ask. A second
    `ask` item with the `id` of an ask still open, and an `ask` item with the
    `id` of a check still open, are `unknown` and the open one is unchanged; the `claude` fixtures reuse no `id` of an ask that
    opened, also after its `ask-resolved`. A thinking block streamed as
    deltas and then finished with the same text leaves `thought.text` as the
    text once, one live `AgentThoughtChunk` per delta and none for the
    finished block, and `seconds` from the deltas only; a finished block with
    no deltas is pushed once.
67. (9.1, 9.3, A1–A3) Kernel crate: `start`, `say` and a turn's end move
    sessions in and out of `active/`; each entering or leaving change reads and
    writes `slots` and one page of at most 24 KiB; a change among `running`,
    `waiting` and `paused`, and a turn that ends and starts the next queued
    input, write no page; `delete` removes a listed session. With 1 024 listed
    sessions (16 owners; seeded into the crate test's host state, since stage B's
    calls cannot list that many: provider slots, 9.5), a `start` or `say` that would add one is refused
    `busy`, its message names the cap, and it does not wait; a leaving session
    frees room for the next.
68. (2.1, 9.3, G1, R1–R3) Kernel crate: after the host's generation changes, the
    first call (not an item call on a stream) zeroes the stream slot counts,
    keeps `timers`, sets `restart`; with 1 024 listed sessions (seeded into the crate test's host state, as in 67), firings of
    `restart` step one session each, in at most `A + 1` calls for `A` stale
    sessions, each reading at most 96 KiB of pages plus that session's step
    (count calls and state gets); no call lists keys. A second boot change
    during the pass starts again from page 0. With no call after a start, no
    session changes; a step reads and writes no more than `interrupt` of that
    session or one step of 9.5 (17), whichever applies, plus its page.
69. (9.3, 9.4, 18.5, R4 Claude part, R5) Kernel crate with the fake `claude`:
    after a restart a `running` or `waiting` Claude session becomes `unknown`
    (a `turn` entry, pushed), its open asks are denied `by: kernel`, reason
    `restart`, it leaves `active/`, the queue is kept, and no `stream/` or
    `live/` key of that session from before the restart remains. A tool call streamed without a `tool` entry gets none. `say` on an `unknown` session starts a resumed turn; a refused
    resume gives `resume-lost`; with inputs queued, the new input joins the end
    and the oldest starts.
70. (9.5, R4 OpenAI part) Kernel crate with the scripted Responses server:
    after a restart an OpenAI session `running` in `requesting`, `ready`,
    `retrying` or `tools`, or `waiting` (its asks denied `by: kernel`, reason
    `restart`), becomes `unknown` with its loop `idle`: every open
    job (idempotent ones included, case 22) and every `pending` call gets
    `aborted`, the `retry` deadline goes, it takes no request slot and no FIFO
    place, and the step (in the pass, under another owner's call, or in the
    owner's own, R6) makes no model request, toolset operation or consent; a
    `stopped {quota}` that recording the aborts gives stays, with no request;
    the owner's next `say` sends one request, in her call, with the aborted
    outputs; a `say` that finds no request slot is refused `busy`, queues
    nothing, and nothing is sent for it in another owner's call. `paused`
    sessions (either backend) are unchanged, their deadlines and timer kept.
71. (9.3, R6) After a restart, each of `attach`, `read`, `say` and `answer`
    on a session of an older boot does its restart step first, in the
    same call, and acts on the new state (an `answer` to an ask the step denied
    is refused `{field: "ask"}`); the pass then skips that session.
    `list` over such sessions reads no extra record, reports `unknown` for a
    `running` or `waiting` one of either backend, and writes
    nothing.
72. (9.4, 9.8) Kernel crate: an `ask` or `check` item whose `id` is not a
    non-empty string (`7`, `null`, `""`), one whose `input` is missing, `null`
    or not an object (`{}` is an ask), and one whose `expires-at` is negative,
    fractional, `5.0`, `1e3`, past 2^64 - 1, a string or `null`, are each an
    `unknown` entry, no ask. A line with bytes that are not UTF-8 is `unknown`
    with `raw` holding `\xNN` for each such byte, trimmed. A line with a
    duplicate top-level key is `unknown`. An `ask-resolved` with an unknown or
    missing `by` is `unknown` and the ask stays open, so a later `expired`
    gives its `by: host` decision; with `by: answered` it closes the ask
    with the pending answer as its entry when the kernel answered it itself
    with that outcome, and gives the decision
    entry from `outcome` otherwise (`by: checked` on a `direct: false`
    stream, else `host`); a missing or bad `outcome`, `none` on an ask or
    `allow` on a check is `unknown`. An `ask` or
    `check` whose `tool` is not a string is `unknown`. A part group whose joined
    whole is of kind `part` reads as `unreadable`.
73. (9.3, 9.4, 9.8 rule 4, 4.5) Kernel crate, with the fake `claude` or
    fixtures: in each, exactly one decision entry per ask id. (a) Direct
    mode: the owner's allow, then `answered allow`: one `allow, by: owner`
    entry. (b) Checked mode: the owner's allow, then `answered deny`: one
    `deny, by: checked` entry. (c) A pending answer, then `expired`: one
    `deny, by: host, reason: expired`. (d) `expired` carrying an `outcome` is
    `unknown`, the ask stays. (e) A check, no answer, host `expired`: one
    decision, and a later `answer` is `unknown-ask`. (f) The provider
    stream's `ended` with one open ask and one with a pending answer: two
    decisions, `deny, by: kernel, reason: ended` and the pending answer. (g)
    `cancel` and the restart step with the same two asks give the same two
    kinds. (h) The cap: with 64 open asks and checks, a 65th is denied at once,
    `deny, by: kernel, reason: busy`, its ask and decision entries written in
    that call and a `Request::Answer` deny sent; it never opens, a later
    `ask-resolved` of it is `unknown`, and the stream's `ended` then writes 64
    decisions. With 64 pending answers the owner's `answer` is refused `busy`.
74. (9.7 "Revoking", 9.1) Kernel crate: an owner starts a session on
    profile `p` and queues one input; then `p` leaves `access/<owner>`. `say`,
    `interrupt` with a replacement, Continue on an `unknown` session and
    `answer` with `allow` are each refused `{field: "profile"}`; the turn's
    end starts no queued input and the queue stays; a `resume` deadline
    leaves the turn `paused` with no time and sends no request (the OpenAI loop's `resume`, 9.5; S40 r1); `read`, `answer` with `deny`,
    `cancel` and `delete` still work. An unreadable `access/` value acts as
    an empty one.
75. (9.8) Kernel crate, adapter: an `Edit` ask's `toolCall` has `kind: edit`
    and its diff (`Write` and `MultiEdit` likewise, each other tool its kind
    and no `content`); a second ask or check on the same call with another
    `tool` or `input` is `unknown` and the open one is unchanged; the same
    `tool` and `input` is an ask; after the open one is resolved, another
    input is an ask again.
76. (9.3, 9.8 rule 4) Kernel crate: the provider stream's `ended` on a
    `running` or `waiting` session closes each open or pending ask (the two
    decision entries of case 73 (f)) and then ends the turn `idle {failed
    {error: provider-ended}}` in the same call: exactly one `turn` entry
    after the decisions (no `turn` entry for `running` between them), the
    push of that turn, the session out of `active/` (no `active-page`), and
    no `idle` deadline. Past the owner's quota the same: one `turn` entry
    and its push. A queued input does not start: the queue keeps it and no
    `Request::Turn` is sent; the next `say` starts a turn with the oldest
    queued input first (the new one joins the queue), sends `Request::Turn`
    with `resume: true` (`persist: false`: `resume: false`) and opens a
    provider stream; with no queue, that `say` starts a turn with its own
    input. On `idle {completed}`, `unknown` and `resume-lost`, `ended`
    leaves the turn as it was and writes no `turn` entry; it still frees the
    stream's slot and keys.
77. (9.3 "Past", 9.4 "The short decision") Kernel crate: with the
    account at the ceiling (account plus reserve), each of `cancel`,
    `interrupt` without a replacement, a provider item that ends the turn
    `stopped {limit: quota}` (and sends `Request::Interrupt`) and a provider
    stream's `ended` succeeds, ends the turn, frees what it holds (the
    provider stream's slot and keys, the session's timer) and charges what it
    wrote to both accounts and the record's `bytes`; `say`, `start` and
    `rename` still fail `quota` there. Past quota every decision these calls
    write is short (`id`, `kind`, `ask-i`, `decision`; no ask fields), and
    its `ask-i` names the ask entry of the same `id`; with 64 open or pending
    asks whose ask entries are each over 100 KiB, a closing call past quota
    writes under 1 MiB of entries in all. Below quota the decisions repeat the
    ask, as before; a call that crosses the cap writes the decisions after
    the crossing short.
78. (9.10 "One writer", 14 "Only the participant gives a client a profile")
    Kernel crate, fake host: each `manage` function called as `local/c`,
    `pub/c`, `agent:h` and `host` is `not-owner` and reads and writes
    nothing; across every `sessions` call, `offers` collect, timer firing and
    stream item but the access answer, no write touches `profile/`,
    `profiles`, `access/` or `access-ask/`; `profiles` and `check` by
    `local/c` add it to `clients` once, by `page` and `agent:h` never, and
    not past 64.
79. (9.10 "Pinned entries", 9.7 "Revoking") Kernel crate: `access/<c>` as an
    array of names, with a string element, with a digest of 63 hex or upper
    case, allows none; with the stored profile's digest it allows the name,
    and after `put-profile` changes the profile it does not (`profiles()`
    leaves it out, `start` and `check` refused "changed since the participant
    allowed it"), while a session started before still takes `say`; after the
    answer for the new digest that session's `say`, replacement and R5 are
    refused `profile`, its running turn goes on, and a new `start` passes.
    After an edit and no answer, the old session's `say` after an idle
    release (a later launch), an OpenAI step that reads the profile and a
    `task` admission on it are refused or end the turn with "changed since
    the participant allowed it"; none launches the edited bytes.
80. (9.10 "Allowing", "The answer") Kernel crate: `allow` writes no entry and
    makes one `agent-access.ask` request after its writes; the fake host's
    `{"type":"granted"}` writes the entry; a refused or cancelled end, any
    other item body, a granted item after the profile changed, after
    `withdraw`, or for a stale profile, writes none; `allow` for an identity
    not in `clients` is refused `client`; a second `allow` while one is open
    asks nothing; a fifth open question is `busy`; an `allow` for a newer
    digest cancels the older question.
81. (9.10 "Withdrawing") Kernel crate: `local/c` has 20 running roots of `p`
    (two with live children) and 2 of `q`; `withdraw(local/c, p)` refuses
    `say` on every `p` session at once, ends 16 turns and returns 4, the next
    call ends 4 and returns 0; the children of ended roots are cancelled; the
    `q` sessions run on; a child of an idle `p` root whose delegation
    resolved and that its owner gave a running turn is ended too;
    `delete-profile(p)` is refused while an entry or a
    question names it and passes after.
82. (9.10 "Offers") Kernel crate with two fake offering mods: both offer
    `implementer` with different text; `offers()` returns both with digests
    and no source; `adopt` writes `profile/<as>` and its link and no entry;
    `adopt` with a digest no longer offered is refused `offer`; when the
    offer's text changes, the next `offers()` marks the adoption stale, and
    `start`, `check`, `allow` and a `task` naming it are refused; adopting
    again gives a new digest that each client's entry must be answered for.
83. (9.10 "Reach") Kernel crate, golden: the reach text of a Claude profile
    with file and shell tools and three allow rules, of the factory's
    reviewer (OpenAI, no tools), the ask line for a client, for `page` and
    for `ask: false`, the network line with and without domains, the
    "Changed since" line, a rule with a bidi control (shown escaped); a
    profile whose reach passes 4 KiB makes `allow` refuse "too wide to show"
    and ask nothing; plain text, the same bytes each run.
84. (2.1 "Access questions", 15; the watane slice) Host, fake `Confirm`:
    `agent-access.ask` from a timer firing, another mod's call and an agent
    tool call is `not-page`; an unknown `client` or the caller's own identity
    is `no-mod`; granted gives one item then `closed`, refused and 600 s give
    `consent-refused`, a preview refuses at once, `streams.cancel` drops the
    app's question; nothing is written to `installed.json`.
85. (18.1, 14 "The console has no power a client lacks") Console: no file of
    `console/src` names a `manage` function; on `watane-dev serve` with dev's
    fake answer, AGT-J7 runs end to end: offers, adopt, allow (the fake
    grants), the factory's stand-in client lists and starts the profile, an
    edit refuses its next start, `withdraw` stops its turn.

**V8 record.** Empty until stage A fills it: the numbers of V8 (order and
delay through the three layers, the cost of one fsync per commit), the date
and the machine. Whether to change 7 is the human's.

**Budget record.** Empty until a stage's load test (kind "Load") runs: for
each stage A–F, the p95 numbers of the budget table in 17, the date and the
machine. A stage is done only within its budgets.

- **Stage B** (S36, 2026-11-23, remeasured in S36 r1, r2 and r6; r6: `Linux 6.18.44-fc-v819 x86_64`, 4 cpus, a cloud
  VM with a real fsync, load average 3.5 to 5.1 from the run itself; watane `slice/s100` at ef94a58).
  `npm run test:load` in `console/` (`console/test/host/load_b.mjs`): 8 sessions on dev's fake
  `claude` in `watane-dev serve`, 4 for `page`, each shown in its own Chromium page, and 4 for the
  test mod `local/loadclient` (its `access/` seeded into the stopped preview, a test-only reach),
  all started together, each on the fake's `load` at 50 deltas and 5 entries of 4 096 bytes a
  second for 8 s, then Stop. Latency counts only once all 8 fakes loaded together, with no idle
  stretch, all through a common window of at least 6 s ending at the first Stop (8.0 s in each pass), as a
  maximum spacing (S36 r5 F5.5, r6 F6.3, the deputy's rules): for each fake and kind, no gap between consecutive
  stamps, nor from the window's start to the first or from the last to its end, exceeds twice the nominal
  spacing, 40 ms for deltas and 400 ms for entries; and only for the items written in it (S36 r2 F2.3: a run with
  one idle client fails; S36 r3 F3.5: sources that load in turn, not together, fail; S36 r4 F4.3: bursts at the
  edges of 2 s bins around an idle 2 s fail; S36 r6 F6.3: the real run's jitter, gaps of 28.5 ms and 205 ms,
  passes, and a gap over 40 ms or 400 ms fails, at the edges too; r5's 25 ms rule failed every real run by
  up to 3.5 ms of timer jitter). The passes below were measured under the r6 check. Each page session is paired with its fake by identity (the fake's
  first input, `load <k>`, which a test-only wrapper that stamps each stdout write records), and
  every line its fake wrote before Stop reached its page exactly once (about 1 750 per run, none
  missing, none twice, none from another session). Item delay, the fake's stdout write to the
  item's arrival at the Chromium page's `watane.on` handler, p95 over the page's 4 sessions:
  15.6, 26.7, 11.5 ms in three passes in a row over 1 754 to 1 759 items (max 106 to 243 ms; budget 100). The
  page's Stop button to exactly `idle {interrupted}` in that page, p95: 15.7, 26.3, 15.3 ms
  (budget 1 000). State bytes read and written per finished entry (kernel crate,
  `cargo test -- --ignored s36_load`, the mean over 30 entries of 4 KiB after 10 deltas each):
  74 613 at 10 entries, 81 257 at 10 000 (ratio 1.09; budget 2 MiB, within 10%), after the fix
  that a text delta reads no record (case 56, P09). The writer busy share is the host's, measured
  by watane S100 at this load (20%, host 15, "Stage B host record"; budget 50%). Transcripts of
  10 000 entries were not run on the host: the per-entry state bytes, the one figure that could
  grow with them, are the crate's. The first record (S36) timed raw Node socket arrivals and
  paired sessions with fakes by their delays; S36 r1 (F1.8) replaced it with these browser
  measurements; S36 r2 added the common-window rate check and remeasured; S36 r3 made it hold in every 2 s
  subwindow and remeasured (watane `slice/s100` at fdb3d9a); S36 r4 made the windows slide
  (`console/test/host/load_check.mjs`, a pure function with its test); S36 r5 made it a maximum spacing of 25 ms and
  250 ms, which the real run failed by timer jitter alone (review r6: worst 28.5 ms); S36 r6 set it at twice the nominal
  spacing and remeasured (the figures above). A host that exits as a zombie no one reaps (a container's init) counts as
  stopped (`/proc/<pid>/stat`).
- **Stage F** (S134-7, 2026-11-26; `Linux 6.18.44-fc-v880`, 4 cpus, a cloud VM). State bytes
  only, on the fake host: `cargo test s134_stage_f_state_bytes_per_step -- --ignored --nocapture`
  in `kernel/` (`kernel/tests/it/s134_budget.rs`). The fixture of 17: 2 owners, 1 root each, 2
  simultaneous `task` children per root, each child one step with 2 open tool jobs, the root
  compacting every tenth step; the mean over 128 steps of each root after 10 and after 1 000 steps
  of history (a window that spans whole 64 KiB chunk cycles, as D's 128 steps do; over 32 steps
  the 1 000-step figure read 42% over the 10-step one by chunk phase alone). Bytes read and written:
  per root step (its dispatch call) 145 206 at 10 and 158 240 at 1 000 (+9.0%); per child step
  (its two jobs and its answer, which resolves the root's call) 160 577 and 176 107 (+9.7%); the
  whole cycle (root step and both children) 466 362 and 510 454 (+9.5%); budget 4 MiB per step,
  within 10%. The thin margin is the chunk phase of a finite window. p95 per step (S134 r1,
  same run shape, 2026-11-26): root 232 992 at 10 and 247 311 at 1 000, child 266 047 and
  276 991, each within 4 MiB; the means of that rerun were within 0.4% of those above; the 10% history tolerance holds for
  the p95 series too (+6.2% root, +4.1% child; S134 r2). A Stop of each root with two
  live children (`interrupt`: all four children and both roots `interrupted`, the roots' lists
  empty) costs 300 261 bytes for the two roots at 10 and 485 525 at 1 000. Page-level budgets
  (item delay, Stop through the last child at the page, writer busy share): not measured (needs
  watane-dev), a known limit (the deputy's ruling on S134 r1 F1.9); no stage F page runner exists
  yet.

- **Stage F skill lookup** (S148 r1, 2026-11-27, Linux 6.18.57 x86-64,
  AMD EPYC 9V74, five visible CPUs with a four-CPU quota, fake host).
  Each root pins a 1,000 KiB reference body with a short description.
  The root delegates without loading the reference. Each child calls its
  smaller `skill` beside two tool jobs.
  `cargo test s148_stage_f_skill_state_bytes_per_step -- --ignored --nocapture`
  in `kernel/` measures the means at 10 and 1,000 history steps. Root 148,058
  and 161,071 bytes, child 169,651 and 185,134 bytes, and whole cycle 487,362
  and 531,340 bytes. The p95 values were 235,316 and 249,625 bytes per root
  step, and 274,851 and 285,769 bytes per child step. Both series stayed
  within the 4 MiB and 10% budgets. Page-level Stage F budgets have the same
  limit as the S134 measurement.

- **Stage F near-limit child admission** (S148 r2, 2026-11-27,
  Linux 6.18.57 x86-64, AMD EPYC 9V74, five visible CPUs with a four-CPU
  quota, fake host). On a fresh root, dispatching one `task` to a delegate
  with an ASCII `SKILL.md` at the 1 MiB skill limit (file name plus body)
  read 1,056,376 bytes and wrote 1,053,759 bytes, or 2,110,135 bytes per
  admitted child for that step.
  `cargo test s148_delegate_skill_admission_state_bytes_at_limit -- --nocapture`
  in `kernel/` records this cost. Each admission reads the delegate
  profile and writes the child's own snapshot. Multiple near-limit
  admissions in one root step can exceed 4 MiB. The deputy accepted this
  admission cost as a known limit. The skill lookup series above measures
  admissions whose child snapshots are small.

- **Stage D** (S128 r1, 2026-11-25, Linux 6.18.57 x86-64, 5 visible CPUs
  with a 4-CPU quota, load average 4.20/2.81/2.24, Watane `548b4e1b` with the
  test-only pacing and commit trace patch in `console/test/host/stage-d-tools/`).
  `npm run test:load:d` ran eight Claude sessions at the Stage B rate and
  eight OpenAI sessions at one four-tool step per second for a common
  7.06 s. The scripted server alternated owners on a 125 ms grid. Four
  sessions of each backend were attached in separate Chromium pages.
  Item delay from source write to `watane.on` was **56.9 ms p95** across
  1,697 items (Claude 57.1 ms, OpenAI 54.4 ms). The page's Stop button to
  `idle {interrupted}` took **82.6 ms p95**. Actual host and mod commits,
  including fsync, occupied **43.6%** of the common window. The kernel
  state I/O check measured **647,004 bytes** per four-job step at 10 steps
  and **628,788 bytes** at 1,000 steps, within 10% and below 4 MiB.

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
| A | H1 streams, H5 durable requests, H7 timers; the V8 three-layer prototype (16, "Probes named here") | the page skeleton (18.4 layout, theme) attached to a kernel stream, numbered items arriving: a probe session (9.9) in the transcript, live and after a reload | watane: host, pkg (H1, H5, H7); this repository: the kernel stream and the probe (9.9), the WASM wrapper and `wit/world.wit` (1), the console on the real kernel in `watane-dev serve` (18.2) | cases 20, 21 pass (case 7's kernel part needs a provider stream: stage B, and `cargo test` until then); cases 36 and 51 for the host part (3); case 58 (host part); the budgets of stage A; V8 measured and recorded in 16, "V8 record" |
| B | H2 with the unchecked provider and a kernel that runs Claude sessions (9.3, 9.4, 9.8, the queue and session allows); dev's fake `claude` | a session on the fake: transcript, status line, asks, queue, Stop, reload and reattach | watane: host H2 and `host-info.generation` (2.1, G1), dev's fake `claude`, pkg; this repository (one boundary list, its `factory/config.toml` `boundaries` pointing at 14, which names `provider/`): the unchecked provider in `provider/` (R-B1), kernel, console | J11 on the fake without its consent steps (a stage B launch uses a scratch workspace of 4.6, which asks none; the plan line, the folder consent and its withdrawal are stage C) and AGT-J6 (Claude part); cases 5, 6, 7, 8–18, 27–30, and 31 for the turn states stage B has (not `checking`, not `consent`); new cases 34, 35, 36 (kernel part), 41, 45, 49, 51 (kernel part), 52, 53, 55, 56, 57 (idle and ask parts), 58 (the kernel's reattach part), 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 71, 72, 73, 74; the budgets of stage B |
| C | H6 consents and the app's words; the `claude` binary setting; the plan line | the same on the real `claude`; the `consent` activity; new session with reach | watane: host H6, app; this repository: the console's consent activity and plan line | J11 on the real `claude`, with the consent, plan line and withdrawal; AGT-J4; case 31 for `consent`; the budgets of stage C |
| D | H3 and accounts; the OpenAI loop (9.5); dev's scripted Responses server | backend choice with refusals; paused until a time; the context meter | watane: host H3, app accounts, dev's Responses server; this repository: the OpenAI loop, the console | AGT-J2, AGT-J6; cases 1–4, 19 (`openai`), 22; new cases 39, 40, 48, 54, 57 (retry and usage-limit parts), 70; the budgets of stage D |
| E | The checked provider (needs a judge account, V7: 16, "Probes named here") | checking, verdicts, the breaker | watane: the host's account kind `judge`, the app's judge page, dev's judge fake (16); this repository (one boundary list, 14): the checked provider in `provider-checked/` (R-B1), the `checking` activity | AGT-J3; cases 19 (`judge`), 25, 32, 33; case 31 for `checking`; new cases 36 (provider), 51 (provider part), 37, 38, 42–44, 46, 47, 50; the budgets of stage E |
| F | OpenAI delegation and `skill` (9.7, 11); turn limits and Claude usage (9.7); factory profiles (9.10) | children in the transcript; stopped at a limit; the profile manager (18.1) | this repository: kernel, console, manager; watane: `agent-access` (15, proposed); N5 decided (a), the human, 2026-11-27: the factory's implementer uses the Claude grant with shell and web, nothing new | AGT-J5, AGT-J7; cases 23, 24, 52 (`task` part), 78–85; the budgets of stage F |

**Stage F's factory profiles, in slices** (S151; each one slice; the
consent boundary of 14, so each gets the adversary):

| Slice | Builds | Where | Waits on | Done when |
|---|---|---|---|---|
| F-P1 | 9.10 in the kernel: `manage` and its owner check, `clients`, the pinned `access/` format (every test that seeds `access/` moves to it), `profile-digest` and "Revoking" by digest, `allow` and the answer on a fake `agent-access`, `withdraw` in batches, the profile index, offers collect, `adopt` and stale, the reach text | this repository: `kernel/` | none (the host request faked in the kernel crate) | cases 78–83 |
| F-P2 | `agent-access` and the `profile-access` kind of `Confirm::consent`, the app's words, dev's fake answer | watane: host, app, dev, pkg (15) | the human's acceptance of the 15 rows marked S151 | case 84 |
| F-P3 | the profile manager view (18.1), the wrapper's `manage` export and the access stream's routing, AGT-J7 on `watane-dev serve` | this repository: `kernel/wasm/`, `manager/`, `ui/`, `console/` (the link) | F-P1, F-P2 | case 85; AGT-J7 on the fakes |

watane's JF1-2 (its factory TECHNICAL 6) starts a session once F-P3 has
merged and the participant has allowed the factory its profiles.

S39 exercises stage C's scratch path with the installed releases (section 1).
In a fresh app home, one install consent covers the agent and its provider.
The installed console starts a scratch conversation, shows streamed replies
and tool asks, stops a running turn, and restores the transcript after the app
restarts. Automated checks use dev's fake Claude Code. The real login check
uses the participant's own Claude Code binary and account.

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
| F | B's Claude load plus 1 OpenAI root per owner, 2 simultaneous `task` children per root; each child drives one step/s with at most 2 open tool jobs; exercise handoff and Stop with live children | as D | as D, including all live children stopped | ≤ 60% | as D |

In the stage F fixture each child request ends before its next request
starts: the fixture's toolset answers each job only after the request
stream that called it has delivered its `ended` (the scripted Responses
server closes each stream at `response.completed`, and the job waits at
least 100 ms), and a run in which any turn stops `busy` is a failed
measurement, not a result (S134, P1.7 of S132). Each owner then uses at most four request slots even if the root's dispatch and follow-up streams overlap both child streams
before `ended`. The root runs only `task` calls in that step and uses
no stream job slots, leaving at most four child jobs per owner. Use
D's short and long histories. Measure item delay from emission to
page and Stop from the parent's stop action through the last child stop.

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
- The profiles it offers are those `access/page` names (9.1), with the
  version the participant allowed (9.10).
- The profile manager (9.10, AGT-F-57) is a view of the same page, not part
  of the console: `manager/`, built into `ui/manager/`, plain, with no
  canvas frame (the console-style profile pages are later, `FUNCTIONAL.md`
  section 5). It calls only `manage`; the console links to it and calls no `manage` function (a console
  test reads `console/src` for any).

### 18.2 Stack

- React and assistant-ui, bundled (decided with the human, 2026-11-20). The
  source is in `console/`; the build writes `ui/`, which the host serves as
  any mod's page (host 8.3): same-origin script files, no inline script, no
  `eval`, no WebAssembly, no frames, no outside site.
- The kernel: served by watane (the page has `window.watane`, host 8.5), the
  console calls its own mod through `watane.call` and reads its streams from
  `watane.on` (`{"type":"stream"}` and `{"type":"stream-end"}` of host 8.4;
  `console/kit/watane.ts`). It keeps, per stream, the last `seq` it showed and
  drops any item at or below it, and holds items of a stream whose `attach`
  has not answered yet. From stage B (S35) it opens the owner's newest
  session (`list`); with none, or on New session, it shows the new-session
  view (frame `NewSession-2`). From stage D (S40) the view calls `profiles()`
  once when it opens and lists each profile as a radio row (its name, and
  `Claude Code` or `ChatGPT` with the model when there is one; the first is
  selected). The list is the kernel's: the view adds no profile of its own and
  calls `check` only for a listed one; an empty list, or a kernel that cannot
  answer, shows no profile and Start is off, saying why as text (S40 r1). The
  composer's placeholder names the selected backend (`Message Claude…`,
  `Message ChatGPT…`). "Runs on"
  marks the selected profile's backend; the other is not selectable (title
  "Set in the profile"). On each selection the view calls `check(name, "{}")`
  and lists every refusal under "Can't run on <backend>", each `field` and
  `reason`, in a `role="status"` `aria-live="polite"` region; while any holds,
  Start is off and says why as text. The view has no refusal rule of its own.
  A `start` refused `refused(list)` shows each refusal's `field: reason` in
  the line above the composer and puts the text back. An optional absolute
  folder path stays. The first Send calls `start` for a blank path or
  `start-in-folder` for a picked path. Both take a fresh key. The page
  opens the session and displays the picked path from `list`. Without `window.watane` (the
  component tests, `npm run dev`) it opens the scripted fake kernel
  (`console/fake/`), which the page never ships as its kernel when served.
  Known limit (S32): a page cannot tell a reconnected `/events` connection
  (whose streams the host ended `gone` on the old one) from the `changed` that
  follows any committing call (host 8.2), so until watane.js passes its
  `connection` message to handlers (15) a dropped connection leaves the view
  frozen until a reload.
- State: the console's own reducer over the wire of 9.4
  (`console/kit/wire.ts`, `console/kit/reduce.ts`); its view is the state the
  transcript draws. assistant-ui's External-store runtime
  (`useExternalStoreRuntime`) is fed only the newest rows (100 today,
  `RUNTIME_TAIL` in `console/kit/thread.tsx`), and nothing reads its
  messages. Dropping the runtime is open (the human's call). The ACP adapter
  of assistant-ui is not published, so the console reads ACP itself.
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
| `ask` (unanswered) | the card of its tool call with the ask bar, drawn from the ask entry only, never from any `ToolCall`, `ToolCallUpdate` or `tool` entry of the same `toolCallId`, before or after the ask, while it is open; its argument line comes from the ask's `input` (not `toolCall.rawInput` or `title`); an open ask's call is never folded into "Explored" (a `check` shows as a pending check with no answer buttons); "Needs you (n)" above the input, counting open asks only, not checks |
| `ask` (decided), `allow`, `verdict` | a decision line: who, what, the reason; a decided ask's line names the call by the ask's `toolCall.title` and shows its `input`'s argument, and its card keeps the ask's call (9.8) |
| `Plan` | the plan above the input, "n of m done" |
| `turn` | the status line (18.5); at a turn's end, a footer line of time, tools and tokens |
| `queue` | the queue above the input |
| `child` | a link beside the parent tool call that opens the child transcript via the standard session calls, live and on replay |
| `usage` | the context meter in the header when a sample is current; warning only with `compact-at` |
| `unknown`, `unreadable` | a muted line, "An event this console does not know", raw on demand |

### 18.4 Layout and look

The frames give layout, spacing and words; these values are fixed:
- Sidebar 260 px (hidden under 760 px); header with title, profile, provider,
  workspace and any session allow; transcript column at most 780 px; the
  dock: Needs you, plan, queue, status line, input.
- The profile reads as its stored name (`scratch`) in the header and in the
  new-session view alike. The input's model-and-mode line names the profile's
  model and mode once the kernel reports them; until then no session shows
  it, not even one of `scratch`: a stored `profile/scratch` (9.1) lists as
  the built-in does, so the console cannot tell that a session runs Claude's
  default model in mode `manual`. The header's context meter shows once
  the session's stream carries a `usage` item (9.4); the kernel pushes `usage`
  for OpenAI from stage D (9.5) and conditionally for Claude from stage F
  (9.7).
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
| `running` | the activity: Thinking, Reading `<file>`, Writing; Working when no activity is named (S35) | dot | elapsed | Esc stops |
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

Continue on `unknown` sends `say` with the text `Continue` (9.3, R5).

Only the word and its subject are in the live region; the clock is not read
out. Stop shows "Stopping…" until a `turn` update says idle, or shows another running turn (Stop and send's replacement, whose running update may reach the view in the same frame as the interrupted one; S36 r1).

### 18.6 Behaviour

- **Place.** The transcript follows new output only while the reader is
  within 40 px of the end and focus is not inside a card with an open ask;
  otherwise a "New activity below" pill appears. While focus is inside such a
  card the transcript does not follow, whatever the scroll position. Needs you
  ends the follow. The pill returns to the end and resumes it. Focus leaving
  the card (or the ask being decided) resumes it too, at the next scroll that
  ends within 40 px of the end (`console/kit/thread.tsx`,
  `console/src/follow_S7_fix3.test.tsx`). A pointer press on the transcript's
  scrollbar or on non-focusable text inside the card is not focus leaving the
  card and does not resume the follow (S7 RISK a; today such a press still
  counts as leaving, S23 builds the rule). Session rows do not move while the
  pointer is over the list.
- **Asks.** The bar is for an ask (`kind: "ask"`) only; a check has none.
  Allow once (primary), Allow `<reach>` this session, Deny… with a
  reason field, Cancel, and Esc in that field closing it. Opening Deny… puts
  focus in the reason field and keeps the whole form (the field, Cancel and
  Deny) in view: where any of it lies outside the transcript's visible part,
  or under the dock or a pill over the transcript ("Needs you", "New activity
  below"), the transcript scrolls just enough to show it, and no more (S32
  PO, against `design/frames/Main-2`). "Needs you" scrolls
  to the first open ask and puts focus on Allow once. A session allow shows
  in the header until the session ends or `take-back`.
- **Queue.** Up to 3 shown, then "+ n more". Take it back first (the
  deputy's rule, S36 r3, which replaces S36 r2's): a queue action calls
  `unqueue(n)` first, the kernel's one atomic take (9.3), so no turn can
  take the item after it, and the original and its replacement are never
  both alive. `unknown-input` (a turn took it first) says "That message was
  already sent" and changes nothing else. Edit, once `unqueue` succeeds, puts
  the input in the composer, and she sends it as an ordinary `say` (its slot
  is free, also in a full queue); a refused `say` comes back as "Send" says.
  Stop and send calls `unqueue`, then `interrupt` with the input (the
  replacement runs before queued inputs, 2.3); a refused `interrupt` copies the
  text into an empty composer with the error, as "Send" does. Edit and Stop and send
  while the composer holds text do nothing but say "Finish or clear your
  message first". What a queue action took is kept on a list per session,
  outside the dock (the deputy's rules, S36 r4 and r5): an item (its text,
  and what was tried: edit, stop and send, or Remove's undo) enters when
  `unqueue` succeeds and leaves only when the kernel accepts its `say` or
  `interrupt`, or when she discards it. Copying its text into the composer
  (Edit, or a refused `interrupt`, into an empty composer) does not remove
  it. One owner, one guard (the deputy's rule, S36 r6): a taken item has one
  send path at a time and one in-flight guard, keyed by its id. While the
  composer holds its text, the composer owns it: the draft carries the
  item's id, also in its `sessionStorage` mirror beside the draft, so the
  link survives a reload, and the item's row shows no Undo or retry.
  Sending the composer (edited or not) is the item's send, under its guard;
  when the kernel accepts it, the item and the link leave together; when it
  is refused, the text stays where it is (back in the composer and linked,
  or, if she typed since, the item back with its row). Clearing the
  composer gives the item back to its row; discarding the item drops the
  link and leaves the composer's text as an ordinary draft.
  Nothing else removes or overwrites one: not a timer, another queue action,
  a newer draft, a refused send, two calls that end together, or a session
  switch; a reply that ends after she switched writes to its own session's
  list. Memory is the list's truth while the page lives; `sessionStorage` is
  only a mirror, written after memory and read once on the page's first look
  (back after a reload): a failed write leaves memory as it is. Each item
  under the queue shows its text, its last error, Undo (a `say` that puts it
  back in the queue), Stop and send again for a stop and send's, and
  Discard; one call in flight at a time. Remove is `unqueue`, and its item
  stores the absolute time 6 s on, when its ordinary Undo ends: past it
  (also after a reload) Undo is hidden, unless it was refused or is in
  flight, and the item stays with Discard. Each ordinary Undo has its own
  timer for its deadline, and a click checks the deadline again; a refused
  item's retry is not an ordinary Undo and has no deadline (S36 r6). Known
  cost: a refused re-send loses the item's place in the queue. Known limit
  (accepted by the deputy, S36 r6): if the browser fails to delete the
  mirror (`removeItem` throws), an item the kernel accepted, gone from the
  live list, comes back after a reload; it needs a storage failure. The controls are off under "Another tab". A
  full queue turns Send off and says why (the placeholder, or the line above
  the composer while it holds text): 32 inputs, or a `say` refused
  `queue-full` (the 2 MiB limit), also one that ends after she typed again,
  which keeps Send off until the queue's length changes or the turn ends.
  Each session has a capacity epoch, one more for every queue or turn update
  the console receives, bumped by the kernel client on receipt
  (`kit/kernel.ts`), before the page is told; a `say` records it when sent,
  and its `queue-full` refusal turns Send off only if the epoch is unchanged:
  a refusal from an earlier epoch describes a queue that is gone and changes
  nothing, even when the same length and turn state come back. The lock
  stores the epoch it was set in and holds only in it (S36 r1 to r5). Every queue action
  stays in its session: the dock is the shown session's (App keys it by the
  session), and a call that ends after she switched acts on its own session
  alone.
- **Send.** A `say` that fails (any error of 2.3: `refused`, `queue-full`,
  `quota`, `paused`, `busy`, `backend`, ...) puts its text back into the
  composer, unless the participant has typed since, and shows the error's
  message in one line above the composer (`role="alert"`) until the next edit
  or send; the failure never escapes as an unhandled rejection (S32 user-sim).
  The composer keeps only unsent text: what the participant typed and has
  not sent, per session, in `sessionStorage`, back after a reload. Send takes
  the text out of the composer and the kept draft at once; a pending send is
  kept nowhere in the page. A refusal while the composer that sent it is
  still mounted puts the text back there and in the kept draft (typing since
  wins). Known limit: a send refused after its page reloaded or its composer
  unmounted is lost from the composer, and the transcript shows no such
  message; accepted, since the host refuses a `say` only on `busy` or a
  slash command (S35 r4, the deputy's rule).
- **Another tab.** When the attach stream ends `replaced` (9.4: another
  tab attached to the session), the tab does not reattach by itself. It
  shows one line above the composer (`role="status"`): "This session is open
  in another tab." with a button "Use here", which attaches again after the
  last index shown (and so replaces the other tab, which then says the same).
  The notice goes once that attach syncs. While it shows, Send, Stop and
  every ask button (Allow once, Allow ... this session, Deny...) are
  `disabled`, and the status line keeps its last word. Switching to another
  session and back clears it (`console/kit/kernel.ts`,
  `console/src/replaced_S36_3.test.tsx`).
- **Plan.** It folds to one line while an ask waits or inputs are queued,
  unless the participant opened it.
- **Keys.** Enter sends, Shift+Enter is a new line, Esc in the composer
  stops the turn.
- **Motion.** Only under `prefers-reduced-motion: no-preference`; hover
  styles only under `hover: hover`; a pressed button scales to 0.96 over
  150 ms.
- **Long sessions.** A session of thousands of entries costs what a short one
  does (P14).
  - The transcript is windowed: the rows within 800 px of the viewport, at
    most 100 (`OVERSCAN`, `WINDOW_CAP` in `console/kit/thread.tsx`), plus the
    rows kept, so at most about 120 are mounted. A row with focus, and a row
    with an open ask or check, stays mounted.
  - The page is told at most once per animation frame, with a 16 ms timer
    where there is no frame (`console/kit/kernel.ts`).
  - Code is highlighted once its block is complete and near the viewport,
    off the main thread or on it. Either way each block has a time budget
    (S23 sets its value): a block past it stays plain, and on a Worker the
    Worker is ended so the blocks behind it are not held up. Today only the
    size limit is built: a block over 64 KiB (`MAX_CODE` characters,
    `console/kit/parts/highlight.ts`) stays plain.
  - The reducer does no work per message that walks the transcript: at 5 000
    rows a message costs at most 8 times what it costs at 300 rows, and at
    most 50 µs (`console/kit/s7_fix1.test.ts`; copying the rows array on a
    change is what still grows).
  - The long fixture runs in `npm test` (`console/src/long_S7.test.tsx`): 5 000
    entries and the stream at 50 deltas a second; p95 over 20 samples: input
    at most 50 ms, Stop at most 100 ms, scroll at most 100 ms, stream at most
    100 ms; the end shown at most 2 s after open.
- **Accessibility.** Every state has its own shape; a streaming message is
  `aria-busy` until done; every control is reachable by keyboard and named.

### 18.7 Tests

| Level | What | How |
|---|---|---|
| View | each row of 18.3 and 18.5 from recorded event streams: the fake `claude`'s and the scripted Responses server's output, stored as fixtures | a component test run, under the 2-minute budget |
| Long | the limits of "Long sessions" in 18.6 on 5 000 entries with the stream running (`console/src/long_S7.test.tsx`, the fixture `console/fake/long.ts`) | in the component test run |
| Page | AGT-J6 against the kernel in `watane-dev serve` with the fake `claude` | a browser test, marked slow |
| Host | the stage A story on watane's own host: cases 20, 21, 36 and 51 (kernel part) and the page's live and reload story, against `watane-dev serve` of `kernel/wasm` (`console/test/host/stage_a.mjs`, `npm run test:host`; by hand, not in the gate: it needs a watane with host 19) | a script, run after a kernel or console change and each watane bump |
| Seen | each slice's console part, by the user-sim against the frames | the acceptance step |

### 18.8 Not in the first target

The items of `FUNCTIONAL.md` section 5 under "In the console". Each needs its
backend first, through this file's backend sections.
