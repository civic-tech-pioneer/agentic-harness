# Comparison: agent UI component libraries for a watane page (2026-11-20)

A record, not a contract. It compares front-end libraries for an agent page
(chat with streaming, tool calls, asks, reasoning, plans) that a mod serves
from its own origin. The rules it cites are owned by `host/TECHNICAL.md`
8.3 (content policy), `pkg/TECHNICAL.md` (package limits),
`dev/TECHNICAL.md` (template) and `design/TECHNICAL.md` 2.3,
9.4 and 9.8 (what a page receives).

Sources were read from shallow clones (2026-11-20) unless a row says
"docs" or "memory". No code from the clones was run. Sizes are
bundlephobia numbers for the whole package (minified, gzip), not
tree-shaken; they are a rough guide only.

## 1. What decides it

These facts of watane narrow the field before any library is looked at.

| Fact | Source | Effect on a library |
|---|---|---|
| `script-src 'self'`; no `'unsafe-eval'`, no `'wasm-unsafe-eval'` | host 8.3 | No `eval` or `new Function`. No WebAssembly compile, so Shiki needs its JavaScript regex engine. No inline script, so no inline `<script type="importmap">`. |
| `connect-src 'self'` | host 8.3 | No CDN at run time, no telemetry, no remote fonts or icons. Calls go only to `/call` and `/events`. |
| `font-src 'self' data:`, `img-src 'self' data: blob:` | host 8.3 | Fonts (KaTeX, Inter, Geist) must ship in `ui/`. Remote images in agent markdown will not load; show them as links. |
| `frame-src 'none'` | host 8.3 | No iframe sandboxes for generative UI or HTML previews. |
| `worker-src 'self'` | host 8.3 | A worker must be a file in `ui/`; a `blob:` worker is refused. |
| `style-src 'self' 'unsafe-inline'` | host 8.3 | Inline styles and injected `<style>` work. Compiled Tailwind works; the Tailwind CDN script does not. |
| A page is static files in `ui/`; scripts load only from the mod's own origin | host 8.3, dev template | Each mod ships its own copy of any kit. Pages cannot share one script. |
| The template is plain HTML and JS, no build step; the app's own UI is "plain HTML, CSS and JavaScript modules ... no framework and no build step" | dev template, `app/TECHNICAL.md` | A React or Svelte library needs a bundler that watane's tools do not run today. |
| A package is at most 64 MiB and 4096 files | `pkg/TECHNICAL.md` | Size is not a hard limit. Readability of the vendored code matters more, since makers fork mods. |
| The app is Tauri 2, so a page runs in the system webview (WebKit on macOS) | `app/TECHNICAL.md` | Test in WebKit. WebKit also blocks WebAssembly without `'wasm-unsafe-eval'` (to confirm in watane's webview). |
| A page gets ACP v1 `SessionUpdate`s, plus watane items: `ask` / `ask-resolved` under `_meta.watane`, `verdict`, `turn`, `compaction`, `unknown`, `synced`, entry index `_meta.watane.i` | agents 9.4, 9.8 | No library speaks this transport. Every option needs an adapter from `watane.on` messages to its model. Decisions go back through `sessions.answer` as `{allow | deny, reason?, input?}`. |

## 2. Comparison table

Columns: **R** reasoning, **T** tool call, **A** approval, **P** plan or
todo, **D** diff, **X** terminal output, **Q** prompt input with stop and
queue. "y" means a ready component; "-" means none; "c" means you write a
custom renderer on its hooks.

| Library | Framework | R | T | A | P | D | X | Q | Data model | ACP fit | Size (gz) | Styling | Licence | Maturity | CSP / bundling |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| assistant-ui | React 18/19 | y | y | y | y | y | c | y | Own thread model; External-store runtime; private ACP adapter | Best: ACP v1 adapter exists in repo | ~280 KB core + React ~60 KB | shadcn copy-in, Tailwind | MIT | 13.4k stars, daily pushes, 0.x | OK with a bundler. Avoid `safe-content-frame` (iframes). Needs Vite/esbuild. |
| Vercel AI Elements | React 19 (shadcn) | y | y | y | y | - | y | y | AI SDK `UIMessage` parts | Good: tool part states include `approval-requested` | depends on parts; Streamdown 162 KB | shadcn copy-in, Tailwind | Apache-2.0 | 2.5k stars, monthly | OK with a bundler. Mermaid, Rive, xyflow parts are optional. |
| CopilotKit (+ AG-UI) | React (core), Lit (few parts) | y | y | y | c | - | - | y | AG-UI events through an `AbstractAgent` | Medium: write an agent that turns ACP into AG-UI events | large (30 deps) | Tailwind, CSS vars | MIT | 38.8k stars; AG-UI 20.8k | Telemetry to `telemetry.copilotkit.ai`, Scarf, licence-key nags; iframe sandboxes (`@jetbrains/websandbox`). Poor fit. |
| prompt-kit | React (shadcn) | y | y | - | c (steps) | - | - | y | Plain props | Low effort for display; no asks | small per part | shadcn, Tailwind | MIT | 3.2k stars, 0 test files | OK with a bundler. Fewer agent parts. |
| LlamaIndex chat-ui | React | c | c | - | c | - | - | y | Vercel AI SDK `useChat` shape | Low fit | ~890 KB | Tailwind | MIT | 0.6k stars, last push 2025-12 | Heavy (CodeMirror, MDX editor, PDF viewer). No. |
| OpenAI ChatKit | Web component (closed core) | y | y | y | y | - | - | y | OpenAI ChatKit server protocol | None | n/a | themed by options | Apache-2.0 wrapper | 2.0k stars | The component itself loads from `cdn.platform.openai.com`; its source is not in the repo. Ruled out. |
| OpenAI Apps SDK UI | React + Tailwind 4 | - | - | - | - | - | - | - | General widgets | n/a | medium | Tailwind 4 | MIT | 1.1k stars | General UI kit, not agent parts. |
| deep-chat | Web component (no framework) | - | - | - | - | - | - | partial | OpenAI-like messages, own handler | Low: no tool or ask parts | 117 KB | Shadow DOM, style objects | MIT | 3.7k stars, 0 test files | `new Function` for string-typed function attributes (`typeConverters.ts`); avoid that path. |
| ACP-native web UIs (acp-ui, react-acp, @acp-components/react, mezame, crow-ui, Panda) | Vue, React | varies | y | y | varies | some | some | varies | ACP over WebSocket | High in name, but young | n/a | varies | acp-ui MIT; @acp-components no licence | acp-ui 0.5k stars; others one release or apps | Apps, not libraries, or too young. Reference only. |
| Zed agent panel | Rust, GPUI | y | y | y | y | y | y | y | `AgentThreadEntry`, `ToolCallStatus` incl. `WaitingForConfirmation` | Native ACP | n/a | GPUI | GPL-3.0-or-later | mature | Reference for behaviour only; do not port code. |
| Chainlit, Gradio | Python server + React / Svelte front | y | y (Steps) | y (Ask) | y (TaskList) | - | - | y | Server-driven | n/a | n/a | own | Apache-2.0 | mature | Not embeddable; Gradio-lite needs WASM. Reference only. |

Leaf libraries (for any choice):

| Need | Library | Framework | Size (gz) | Licence | CSP finding |
|---|---|---|---|---|---|
| Streaming markdown | Streamdown 2.8 | React | 162 KB | Apache-2.0 | No CDN in core. Code plugin uses Shiki's JS regex engine (no WASM). Image download does `fetch(src)`. |
| Repair half-written markdown | remend 1.5 (from Streamdown) | none, no deps | 9 KB | Apache-2.0 | Pure string function. Pairs with any parser. |
| Markdown parse, sanitize | marked 18, DOMPurify 3 | none | 15 KB, 11 KB | MIT, Apache-2.0/MPL-2.0 | No eval. Both ship single ESM files. |
| Code highlight | Shiki 4 | none | 65 KB + grammars | MIT | Use `createJavaScriptRegexEngine`; the default Oniguruma engine is WASM and is blocked. |
| Diff | @pierre/diffs 1.6 | vanilla core + React wrapper | 133 KB | Apache-2.0 | Checks for WASM (`canUseWASM.ts`) and falls back to `shiki-js`. Workers only through a `workerFactory` you give, so a file worker works. Shadow DOM. |
| Diff | diff2html 3.4 | none | 13 KB | MIT | Precompiled Hogan templates. Custom `rawTemplates` call `Hogan.compile`, which needs eval; do not use them. |
| Diff | react-diff-view 3.3 | React | 23 KB | MIT | Fine; takes unified diff text, so ACP `oldText/newText` needs a diff step first. |
| Terminal output | ansi-to-html 0.7 | none | 3 KB | MIT | `escapeXML` defaults to `false`: set it to `true`, or agent output becomes HTML. |
| Terminal output | anser 2.3 | none | 2 KB | MIT | Has `escapeForHtml`. |
| Live terminal | xterm.js 6 | none | 82 KB | MIT | No deps. Only needed for an interactive terminal; ACP terminal content is output. |
| Long transcripts | virtua 0.55 | React, Vue, Solid, Svelte | 10 KB | MIT | No vanilla API. |
| Long transcripts | @tanstack/virtual-core 3 | none | 10 KB | MIT | Vanilla. Try CSS `content-visibility: auto` first. |
| Components | Lit 3 / Preact 11 / Svelte 5 | — | 6 / 5 / 14 KB | BSD-3 / MIT / MIT | All CSP-safe. Lit and Preact run without a build step (Preact through `htm`; confirm `htm` uses no eval). Svelte needs its compiler. |

## 3. Per-library notes

**assistant-ui.** The widest agent catalogue: thread, composer with queue
and stop, reasoning, tool fallback, approval card, agent plan, diff viewer,
model selector, branches, attachments, context display, guardrail notice,
command palette. Primitives are Radix-based, so keyboard and ARIA come
from Radix. The repo holds `packages/acp` (MIT, about 2.5k lines): an ACP
v1 client, a thread reducer (`acpThreadState.ts`) and conversions
(`conversions.ts`). It maps `agent_message_chunk` to text,
`agent_thought_chunk` to reasoning, `tool_call` and `tool_call_update` to
tool parts with locations, diff and raw output, `plan` to plan state, and
permission requests to a tool-call `approval` with status
`requires-action`. It handles `session/load` replay (`load-start`,
`load-ready`). Limits: the package is `private`, version 0.0.0, "can be
removed without notice". It speaks JSON-RPC over one WebSocket, not
watane's `/call` plus `/events`; you would inject a fake socket or drive
the External-store runtime yourself. The reducer imports a few helpers
from `@assistant-ui/core`; the rest is plain TypeScript. Deps include
`assistant-cloud` and `safe-content-frame` (iframes, refused by
`frame-src 'none'`). 1612 test files (Vitest, jsdom). Releases are 0.x
and frequent, so expect API churn.

**Vercel AI Elements.** shadcn components you copy into your project:
Message, Conversation, Reasoning, Chain of Thought, Tool, Confirmation,
Plan, Queue, Task, Terminal (ANSI), Code Block (Shiki), Prompt Input
(status `submitted` / `streaming` / `error` turns submit into stop),
Model Selector, Sources, Inline Citation, Attachments, Context (token use),
Checkpoint. No diff viewer. The data model is AI SDK `UIMessage` parts.
Tool parts carry states `input-streaming`, `input-available`,
`approval-requested`, `approval-responded`, `output-available`,
`output-error`, `output-denied`; that is close to an ACP tool call plus a
watane ask. Imports `ai` for types only in most parts. Apache-2.0. 47 test
files (Vitest). Good markup and states to copy; the React and Tailwind
code itself would need a bundler.

**CopilotKit and AG-UI.** AG-UI is an event protocol (text, tool call
start/args/end/result, reasoning, state snapshot/delta, run, step,
subagent). An ACP-to-AG-UI agent is possible through `AbstractAgent`
(`selfManagedAgents`), but CopilotKit brings a runtime, GraphQL client,
telemetry (`telemetry.copilotkit.ai`), `@scarf/scarf`, licence-key
features and iframe renderers. Too much that the page policy refuses or
does not need.

**prompt-kit.** Small shadcn parts (message, markdown, reasoning,
thinking bar, tool, steps, prompt input, chat container with stick to
bottom). No approval part, no tests. Fine as a visual reference.

**LlamaIndex chat-ui.** Heavy and slow-moving. Skip.

**OpenAI ChatKit.** The React package is a thin wrapper that waits for the
`openai-chatkit` element, which loads from OpenAI's CDN. Not usable under
`connect-src 'self'` and `script-src 'self'`, and its core is not open.

**OpenAI Apps SDK UI.** A general design system (buttons, inputs, menus,
markdown, code block). No agent parts.

**deep-chat.** The only full chat web component with no framework. It
targets direct model APIs. It has no tool, ask, plan or reasoning parts,
uses `new Function` on one attribute path, and has no tests.

**ACP-native web UIs.** acp-ui (Vue, MIT) is an app. react-acp (JSR,
0.1.9) projects ACP into an assistant-ui runtime, like assistant-ui's own
package. `@acp-components/react` has one release, no repo and no licence.
mezame, crow-ui, Panda and Kiro UI are apps. All assume ACP over a
WebSocket to a local agent.

**Zed.** The behaviour to copy: tool calls show early (`Pending`), wait
for confirmation in place (`WaitingForConfirmation`), and carry content
blocks, diffs and terminals; the plan sits above the composer. Its entry
list (`AgentThreadEntry`) is close to watane's entries. GPL, so read only.

**Chainlit, Gradio.** Server-driven apps. Useful names: Steps for tool
calls, Ask actions for approvals, TaskList for plans, `metadata.status`
pending/done for tool messages.

## 4. ACP and watane items to components (top 2)

`ext` = assistant-ui External-store runtime (or the ported ACP reducer).
`AIE` = AI Elements with `UIMessage` parts. Message ids come from entry
indexes (`e<i>`), so replay and live updates merge without duplicates.

| Page receives | assistant-ui | AI Elements |
|---|---|---|
| `user_message_chunk` (replay) | user message, text part | `Message from="user"` |
| `agent_message_chunk` | append to text part; `MarkdownText` (react-markdown or Streamdown) | append to text part; `MessageResponse` (Streamdown) |
| `agent_thought_chunk` | reasoning part; `Reasoning` | reasoning part; `Reasoning` (opens while streaming, shows duration) |
| `tool_call` (pending) | tool-call part, `running`; `ToolFallback` or a UI per tool name | tool part `input-available`; `Tool` header by kind |
| `tool_call_update` in_progress / completed / failed | result or `isError` | `output-available` / `output-error` |
| tool content `diff` (path, oldText, newText) | registry `diff-viewer` or `code-diff` | none; add @pierre/diffs or diff2html |
| tool content `terminal` | custom part | `Terminal` (ANSI) |
| tool `locations` | kept as metadata; custom chip | custom chip |
| `plan` (entries, priority, status) | plan state from the ACP extras; registry `agent-plan` | `Queue` or `Task` items with done state |
| `ask` (`RequestPermission` shape in `_meta.watane`) | tool-call `approval`, status `requires-action`; registry `approval-card` | tool part `approval-requested`; `Confirmation` with Accept and Reject |
| answer | ACP option kind `allow_*` to `allow`, `reject_*` to `deny`; call `sessions.answer` | same; `Confirmation` has no edit-input field, add one for `input` |
| `ask-resolved` (decided elsewhere, timed out, cancelled) | `permission-resolved`, `resolution: "cancelled"` | `approval-responded` or `output-denied`; add a "decided by ..." line |
| `verdict` | registry `guardrail-notice` | none; custom notice |
| `turn` states | thread `isRunning`, message status | `PromptInput status`; submit becomes stop |
| `synced {through}` | like `load-ready`: end the replay spinner | none; own flag |
| `compaction` | divider; `context-display` | divider; `Context` |
| `unknown` / `unreadable` | collapsed raw JSON | collapsed `CodeBlock` |
| `usage_update` | `context-display` | `Context` |
| `available_commands_update` | `composer-trigger-popover` (slash) | none |
| `interrupt` with replacement, queued input | `message-queue` | `Queue` above `PromptInput` |

Adapter effort: assistant-ui, small for the ACP part (the reducer exists),
medium for transport and watane items. AI Elements, medium: write a
reducer from updates to `UIMessage` parts yourself.

## 5. CSP and bundling findings

1. No library on the list is a set of plain files you can drop into `ui/`.
   The React kits need a bundler; ChatKit needs a CDN.
2. Shiki must use `createJavaScriptRegexEngine`. Streamdown's code plugin
   already does. @pierre/diffs detects the block and falls back.
3. Iframe-based parts (assistant-ui `safe-content-frame`, CopilotKit
   sandboxes, MCP app renderers) are refused by `frame-src 'none'`.
4. Telemetry and cloud clients fail quietly but add noise and code:
   CopilotKit telemetry, `assistant-cloud` (only active if configured; to
   confirm).
5. Eval paths to avoid: deep-chat string function attributes, diff2html
   custom templates.
6. HTML escaping: ansi-to-html escapes nothing by default. Sanitize all
   agent markdown (DOMPurify, or Streamdown's `rehype-harden`).
7. Fonts: KaTeX and any UI font go in `ui/`. Prefer the system font stack.
8. An inline import map is refused. Use relative ES module imports or one
   bundled file.
9. Each mod vendors its own copy; there is no shared script origin.

## 6. Recommendation

**Build our own small kit, framework-free, and port from two sources.**

- Plain ES modules and custom elements (or plain DOM functions), styled
  with CSS custom properties, no Tailwind. This matches the template and
  the app UI: no framework, no build step. A maker's agent can read and
  fork it.
- Port the ACP projection from assistant-ui `packages/acp`
  (`acpThreadState.ts`, `conversions.ts`; MIT, keep the notice): the
  reducer from updates to a thread model. Extend it for `ask`,
  `ask-resolved`, `verdict`, `turn`, `synced` and entry indexes.
- Port the parts and their states from AI Elements (Apache-2.0, keep the
  notice): Tool, Confirmation, Reasoning, Queue/Task for plans, Prompt
  Input with stop, Terminal, Context. Copy behaviour and markup, not the
  React code.
- Leaves as vendored single ESM files: remend + marked + DOMPurify for
  streaming markdown; anser (or ansi-to-html with `escapeXML: true`) for
  output. Add later, as one prebuilt vendor file each, only when needed:
  Shiki (JS engine) and @pierre/diffs (vanilla core) or diff2html.
- Ship it through dev's template (an agent page template), so each mod
  gets a copy.

Why not adopt assistant-ui: it is the best fit on features and has an ACP
adapter, but it brings React, Radix, Tailwind and a bundler that watane's
tools do not run; the ACP package is private and may vanish; 0.x churn
would land in every mod that vendors a bundle. It stays the fallback if
watane decides that agent pages may need a build step.

Why not AI Elements as is: no diff part, no reducer, React and Tailwind.
It is the best catalogue to copy from.

Risks of building:

- Streaming markdown is the hard part (half-open fences, tables, links).
  remend covers most of it; test with recorded streams.
- Accessibility is ours to do: `role="log"` with polite live updates for
  the transcript, focus and `role="alert"` for a new ask, labelled
  buttons, keyboard for collapsibles. Radix would have given part of this.
- Long transcripts: start with `content-visibility: auto`; add
  @tanstack/virtual-core if it is not enough.
- Scope creep: start with message, reasoning, tool, ask, plan, prompt
  with stop. Leave branches, model picker and attachments for later.

Test approach for the kit: unit-test the reducer on recorded update
streams (the kernel adapters' fixtures, plus a replay then `synced` then
live case). Check rendering in the dev preview, which serves the real
content policy, in WebKit.

## 7. Addendum (2026-11-20, after review with the human)

The recommendation in 6 overstated the constraint. Host 8.3 forbids
scripts from other origins, `eval`, WebAssembly and iframes, not large apps: a
bundled React app in `ui/` loads fine, and "no build step" is a convention of
dev's template, not a limit. The cost of bundling is a copy of React and the
kit in every mod, which the human accepts for now.

Changed recommendation: **assistant-ui with React, bundled** (candidate 2
above), with its ACP adapter extended for watane's items. How the code reaches
a page stays open, with bundling as the default; `FRONTEND.md` ("How the code
reaches a page") lists the three options and what keeps them open.

## Sources

- Clones read: assistant-ui/assistant-ui, vercel/ai-elements,
  vercel/streamdown, CopilotKit/CopilotKit, ag-ui-protocol/ag-ui,
  ibelick/prompt-kit, run-llama/chat-ui, openai/chatkit-js,
  openai/apps-sdk-ui, OvidijusParsiunas/deep-chat,
  agentclientprotocol/typescript-sdk, pierrecomputer/pierre,
  rtfpessoa/diff2html, otakustay/react-diff-view, rburns/ansi-to-html,
  inokawa/virtua.
- Zed `crates/acp_thread/src/acp_thread.rs` and its `Cargo.toml`, read
  through the GitHub API.
- npm registry and bundlephobia for versions, deps and sizes; GitHub API
  for stars and last push.
- Web search for ACP web clients: agentclientprotocol.com client list,
  formulahendry/acp-ui, jsr.io/@hafbit/react-acp, mezame, crow-ui.
