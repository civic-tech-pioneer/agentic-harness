# Calling agents from watane: what to reuse (2026-11-20)

A record, not a contract. Question: can a mod call agents, with as much as
possible inside the Wasmtime sandbox, on the participant's **subscription**
(not token billing), for at least Anthropic and OpenAI?

## Answer in one table

| | OpenAI | Anthropic |
|---|---|---|
| Subscription route | "Sign in with ChatGPT", plan usage for open-source and local apps: OAuth (PKCE, loopback `127.0.0.1`, dynamic client registration), bearer token on `POST api.openai.com/v1/responses` | Only the **unmodified `claude` binary**, signed in by the participant (`/login` or `claude setup-token`) |
| Not allowed | paid or hosted apps without OpenAI's partner form | Claude.ai login in our app; holding or passing on its tokens; the Messages API with a subscription token; the Agent SDK (so also `claude-agent-acp`) unless Anthropic approves |
| Where the agent loop runs | **in the mod** (sandboxed) | in the `claude` process (native; its own sandbox and permission mode) |
| Host shim | the OAuth flow, the token store, refresh, and adding `Authorization` in the `wasi:http` `send_request` hook for `api.openai.com` (the mod never sees the token) | start an allowlisted binary in a confined folder after consent; give each stdout line to the mod as a call; stdin from the mod |
| Tools for the agent | function tools in the Responses request; the mod maps them to WIT calls of other mods | the agent reaches watane's existing MCP toolset (`/mcp/<toolset>`, with a token) |

Sources: code.claude.com/docs/en/legal-and-compliance ("signing in to the
unmodified Claude Code binary with their own Claude subscription, including
where a platform hosts Claude Code"), code.claude.com/docs/en/agent-sdk/overview,
code.claude.com/docs/en/authentication (`setup-token`: one year, model requests
only); developers.openai.com/siwc/token-sharing-open-source and its pages
`sign-in`, `models-and-inference`, `preview-limitations`, `token-reference`,
`profiles-and-sessions`; developers.openai.com/cookbook/articles/sign-in-with-chatgpt.

OpenAI plan-usage limits (preview): `stream: true` and `store: false` required;
no `previous_response_id` (resend the history); no `system` role (use
`instructions`); no `temperature`, `top_p`, `max_output_tokens` and more; no
hosted tools. Plus and Pro (docs); Plus shares a five-hour limit across all
apps; the participant caps each app in ChatGPT settings. Access token 1 h,
refresh token 30 days, rotating. UI asks: "Continue with ChatGPT", "Using
ChatGPT plan", a "Manage usage" link.

## No library does it; the parts that do work

Built for `wasm32-wasip2` and inspected with `wasm-tools component wit`
(probes: `scratchpad/probes`, rustc 1.100, 2026-11-20):

| Crate | Builds | Note |
|---|---|---|
| rig-core 0.46 | no | its own `Send` bug on wasip2; a WASI transport PR (#2823) closed unmerged 2026-10-19 |
| genai 0.7.5, async-openai 0.43 client, llm 1.3.8, anthropic-ai-sdk, misanthropic | no | reqwest 0.13 / tokio `net` / tokio `full` |
| reqwest 0.12 | builds, panics at run | treats every wasm32 as a browser |
| golem-ai-llm-* 0.5.5 | builds | transport is wasi:http **0.3** (P3); panics under wasmtime 50 `-S http`; WIT shape worth reading |
| rmcp 3.11 client | builds | tokio executor traps when it waits ("condvar wait not supported") |
| **wstd 0.6.8** | yes | `wasi:http/outgoing-handler@0.2`, no sockets, `Send` future |
| **eventsource-stream / sse-stream** | yes | SSE parsing |
| **async-openai types only** | yes | Chat Completions types; the Responses types must be checked |
| **agent-client-protocol 5.4 (schema)** | yes | no tokio; usable as the one event model for both vendors |
| **claude-codes 2.1.330, `types` only** | yes, parses an `init` line at run time | serde models of Claude Code's stream-json, follows its versions |

A 51-line wstd + serde component that builds an Anthropic request imports only
base WASI and `wasi:http@0.2.12`.

## The spike (`proto/agent-spike/`, 2026-11-20): both paths work

One component (`guest/`, 1.9 MB, wasm32-wasip2) assembled from wstd,
sse-stream, claude-codes (`types`) and agent-client-protocol-schema; it imports
base WASI and `wasi:http@0.2.12` only. One native host (`host/`, Wasmtime
48.0.9 sync linker, like watane). Every agent event comes out as an ACP
`SessionUpdate`, whichever vendor made it.

- **Claude**: the host starts the unmodified `claude -p --output-format
  stream-json --verbose` in a temp folder with `ANTHROPIC_API_KEY` removed
  (auth: the participant's `CLAUDE_CODE_OAUTH_TOKEN`, a setup-token). Each of
  the 13 lines went to a **fresh instance**: 0.2 ms each, so the reactor model
  costs nothing here. Read tool call, its result and the answer mapped to ACP.
  12.2 s for the run.
- **OpenAI**: `login` did the Sign in with ChatGPT flow (dynamic registration,
  loopback 1455, PKCE); plan usage granted; `models` listed ten models
  (gpt-6.2-sol first). `openai gpt-6.2-sol "..."` ran three model turns with
  two tool calls (`add`, `now`, answered by the host as stand-ins for other
  mods) in **one call of 10.8 s**, streaming text deltas as it went. The guest
  never saw the token: the `send_request` hook adds `Authorization` for
  `api.openai.com` only, and `Authorization` is a forbidden header for the guest.
  `include: ["reasoning.encrypted_content"]` with `store: false` was accepted.

Learned:
- watane's `host/src` does not link `wasi:http` yet, although host 6.8
  describes it; the spike's hook is new code, not a copy.
- One tool loop in one call passes the 10 s budget with three turns. Agent
  calls need their own budget, or one call per model turn.
- No ID-token check yet.

**MCP for Claude.** The host serves the same stand-in tools over MCP
(streamable HTTP, JSON answers, loopback, a fresh bearer token per run: the
shape of watane's `/mcp/<toolset>`), and starts `claude` with
`--strict-mcp-config --mcp-config <json with the token> --allowedTools
mcp__watane__add mcp__watane__now`. Claude found the tools (through its own
ToolSearch), called both in parallel, and answered; 7.5 s, all 15 lines mapped
to ACP. Both vendors now reach one set of host tools.

**Hostile guest** (`agent-spike-host hostile`, 12 checks, all pass). A `probe`
export lets guest code send any request; the hook records each decision before
any future runs, and the test judges on that record, with a fake token:
- denied by the hook, nothing sent: another host, a look-alike
  (`api.openai.com.evil.example`), plain `http`, loopback;
- refused before the hook (forbidden header): the guest's own
  `Authorization`, a `Host` swap, `Proxy-Authorization`;
- sent with the host's token: `api.openai.com` in any case (401 for the fake);
- the MCP endpoint: 401 without or with a wrong token, 200 with the run's.

wstd races the request body against the response, so a denial can reach the
guest as "connection reset" instead of `HttpRequestDenied`. Harmless (nothing
left the host), but a mod cannot rely on the error text.

## Open points for the human

- A grant that starts a native agent process is outside the Wasmtime boundary
  (spec/TECHNICAL 6): a new grant kind and a weaker sandbox for that mod.
- The 10 s call budget (host 6) does not fit a model turn. Streaming as events
  into short calls, or a separate budget for agent calls.
- Where the OAuth tokens live (the app's part of the home, the keychain) and who
  may use them (a grant per account, like `[http]` hosts).
