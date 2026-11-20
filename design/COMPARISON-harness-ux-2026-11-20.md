# The front end of agent harnesses: a comparison (2026-11-20)

A record, not a contract. It collects how current agent harnesses show an
agent session to a person, so that we can design the front end of the agent
layer. It maps each pattern to the agent layer's draft (`FUNCTIONAL.md`
sections 1-4, `TECHNICAL.md` 4.5, 9.3, 9.4, 9.8). Accepted lessons move into
the contracts through a design slice.

## 0. Sources and ground rules

### Code read

Shallow, sparse clones on 2026-11-20 in the session scratchpad. Code was read,
never run.

| Prefix | Repo | Commit | License | Root of cited paths |
|---|---|---|---|---|
| `Z:` | zed-industries/zed | `0a514d7` | GPL-3.0 (agent crates) | `crates/agent_ui/src/` |
| `TV:` | same | `0a514d7` | same | `crates/agent_ui/src/conversation_view/thread_view.rs` |
| `Zk:` | same | `0a514d7` | same | `assets/keymaps/default-macos.json` |
| `OC:` | sst/opencode | `0f0d168` | MIT | `packages/` |
| `G:` | block/goose | `ca949ba` | Apache-2.0 | `ui/desktop/src/` |
| `C:` | cline/cline | `52f5154` | Apache-2.0 | `apps/vscode/webview-ui/src/` |
| `X:` | openai/codex | `4360c5f` | Apache-2.0 | `codex-rs/tui/src/` |
| `V:` | BloopAI/vibe-kanban | `3f095f0` | Apache-2.0 | `packages/` |

A citation reads `prefix path:line`. **Zed is GPL: take ideas, copy nothing.**
The others may be ported with their notices. In opencode, `UI-en` is
`ui/src/i18n/en.ts` and `APP-en` is `app/src/runtime/i18n/en.ts`. In goose,
`en.json` is `i18n/messages/en.json`.

### Docs, changelogs and complaints read

Official docs and changelogs first, then forums and issue trackers for
pitfalls. Claude Code (CLI, desktop, web), claude.ai, Claude in Chrome, Codex
(app, CLI, cloud), ChatGPT Work, VS Code with Copilot, Copilot coding agent,
Cursor 3, Devin and Devin Desktop (formerly Windsurf), Warp, Amp, Kiro, Jules,
Roo Code, Conductor. URLs are given at each claim.

### Changes since 2025 that matter here

- ChatGPT agent mode is retired. OpenAI's help article points to ChatGPT
  Work. The help page returned 403 to us; the claim comes from a search
  snippet and https://composio.dev/content/chatgpt-agent-mode-explained-what-it-can-and-cannot-do (unverified).
- Windsurf's Cascade docs now live under "Devin Desktop":
  https://docs.devin.ai/desktop/cascade/cascade
- Cursor 3.0 added an Agents Window: https://cursor.com/changelog/3-0
- Amp removed public threads (https://ampcode.com/news/end-of-public-threads)
  and fork (https://ampcode.com/news/stick-a-fork-in-it).
- Cline moved checkpoint restore onto the user message ("Reset Code",
  "Reset Chat"); the old "Restore Files" / "Compare" controls are gone from
  the code (`C: components/chat/UserMessage.tsx:152,233`).

"Unverified" marks a claim from a third party only, or one we could not open.
Six code strings were spot-checked against the clones; all matched.

### The agent layer, as a front end sees it

- **Read:** `attach(session, after)` replays stored entries, sends
  `synced`, then live ACP v1 `SessionUpdate`s. Entries: `message`, `tool`,
  `ask`, `verdict`, `turn`, `compaction`, `unknown`.
- **Turn states:** `running`, `waiting {asks}`, `paused {until}`,
  `idle {completed | interrupted | stopped {limit} | failed {error}}`,
  `unknown`, `resume-lost`.
- **Write:** `start`, `say` (queues while busy, at most 32), `answer`
  (`allow | deny`, reason, changed input only for `AskUserQuestion` and
  `ExitPlanMode`), `interrupt(replacement?)`, `cancel` (session, asks and
  children), `rename`, `delete`, `list`, `check`.
- **Asks** come as `ask` items and end with `ask-resolved {id, by}`. An ask
  expires after 10 minutes and is denied.
- **The checked provider** writes `verdict` entries. It stops the turn after
  3 denials in a row or 10 of 50.
- **Consents** (folder, ChatGPT plan, toolset) are asked in the app, never in
  the client, and no call waits for them.

---

## 1. The conversation

### Patterns

**1.1 A thought block that is open while it streams and closes when done.**
- Zed's `thinking_display` setting has four modes; the default `auto` opens
  the block while a thought streams and closes it when the next chunk
  arrives, unless the user toggled it (`Z: entry_view_state.rs:151-188`).
  `preview` keeps it open but capped, auto-scrolled, with a fade at the top
  (`TV:9016-9035`).
- Cline forces the block open while it streams and caps it at 150px
  (`C: components/chat/ChatRow.tsx:908-916`, `ThinkingRow.tsx:95,106`).
- opencode shows "Thinking" with a shimmer, then "Thought" and the duration,
  for example "Thought 12s" (`OC: session-ui/src/timeline-next/tools/thought.tsx:8,55-60`).
- Codex puts the bold header of the reasoning summary into the status line
  in place of "Working" (`X: chatwidget/streaming.rs:57-60`).
- Goose has a setting `showThinking: always | collapsed | never`, default
  collapsed (`G: utils/settings.ts:37,102`).
- claude.ai shows a summary, never the raw chain of thought, in an
  expandable "Thinking" section
  (https://platform.claude.com/docs/en/build-with-claude/extended-thinking).

**1.2 A one-line row for cheap tools; a full card only where it matters.**
- Zed draws a full card only for a tool that needs confirmation, an edit or
  a terminal. Every other tool is one line with a chevron (`TV:9903`).
- Zed picks the icon from the ACP tool kind: Read and Search a magnifier,
  Edit a pencil (or the file-type icon), Delete, Move, Execute a terminal,
  Fetch a globe, Think, Other a hammer (`TV:11770-11781`).
- Codex uses the verb tense to show status: "Running"/"Ran",
  "Reading"/"Read", "Exploring"/"Explored" (`X: exec_cell/render.rs:301-375`).

**1.3 Consecutive cheap tool calls fold into one summary line.**
- Codex folds reads, lists and searches into one "Explored" cell with
  sub-lines "Read", "List", "Search" (`X: exec_cell/render.rs:422-443`).
- Cline: "Cline read 3 files, 2 folders, performed 1 search"
  (`C: components/chat/chat-view/components/messages/ToolGroupRenderer.tsx:359-397`).
- opencode: "Explored 3 files, 2 searches", "Ran 2 commands", "Worked {duration}"
  (`OC: session-ui/src/timeline-next/tools/activity-label.ts:11-14`).
- vibe-kanban folds only from 2 entries up, and merges repeated edits of one
  file (`V: web-core/src/shared/lib/aggregateEntries.ts:163-176`). It also
  folds the thoughts of all earlier turns into one "Thinking" group and keeps
  the current turn open (`:79-160`).

**1.4 A density setting, with failures always expanded.**
- opencode has presets "Everything", "Detailed", "Compact", "Quiet",
  "Messages only"; each sets, per category (shell, edit, thinking,
  subagents, notices, tools), whether rows are separate, grouped or hidden,
  and open or closed. Default: compact
  (`OC: session-ui/src/timeline/detail.ts:22-83`, `app/src/settings/model.tsx:311`).
- VS Code: `chat.persistentProgressVerbosity` is Compact, Verbose or
  Conversation; finished groups read "Completed N steps". Failed tool calls
  expand on their own (`chat.tools.autoExpandFailures`)
  (https://code.visualstudio.com/docs/copilot/chat/chat-agent-mode,
  https://code.visualstudio.com/updates/v1_109).
- Claude Code desktop cycles Normal / Thinking / Verbose with Ctrl+O
  (https://code.claude.com/docs/en/desktop).

**1.5 Output cut short, and the cut is said.**
- Codex keeps 5 lines of tool output and adds
  "… +N lines (ctrl + t to view transcript)" (`X: exec_cell/render.rs:353-355`,
  `ui_consts.rs:12-16`).
- Zed says what the model saw: "Output exceeded terminal max lines and was
  truncated, the model received the first {}." (`TV:9561-9578`).
- Cline shows output of at most 5 lines in full, else a 75px box that grows
  to 200px (`C: components/chat/CommandOutputRow.tsx:27,101-102`).
- Edits and terminals open on their own when the diff or terminal appears
  (Zed `expand_edit_card`, `expand_terminal_card`, default true, `TV:1606-1620`).
  Cline opens diffs by default, capped in height
  (`C: components/chat/DiffEditRow.tsx:75,182`).

**1.6 The plan sits above the composer, not in the scroll.**
- Zed: an activity bar above the input shows "Plan", "Current: …",
  "{n} left", with "Open Plan File" and "Clear Plan" (`TV:4897-5023`). Entry
  icons: in progress, complete, cancelled, pending (`TV:5089-5096`).
- opencode hides the `todowrite` tool from the timeline and shows a pill
  "{done} of {count} todos done" (`OC: session-ui/src/timeline-next/rules.ts:42-60`,
  APP-en:685-687).
- Claude Code: Ctrl+T toggles the checklist, at most 5 tasks shown
  (https://code.claude.com/docs/en/interactive-mode). Warp: a chip
  bottom-right, ● current, ✔ done, ○ not started, ■ cancelled
  (https://docs.warp.dev/_llms-txt/agents.txt).
- Codex prints the plan in the scroll: "• Updated Plan · 2/5 complete",
  done steps struck through (`X: history_cell/plans.rs:276-306`).

**1.7 A subagent is a card with status, a short preview, stop and open.**
- Zed: "Spawning Agent…", "Subagent Canceled", "Subagent Failed"; the
  preview holds the last 8 entries; buttons "Stop Subagent", "Make Subagent
  Full Screen"; header "· {model}" and "— {n} files changed"
  (`Z: conversation_view/subagent_card.rs:287-290`, `TV:12561-12846`).
- Claude Code CLI: running subagents in a tree below the prompt with
  "(+N)" descendants. Enter opens one, and you can type to it
  (https://code.claude.com/docs/en/sub-agents).
- Codex app: a panel above the composer with each subagent's status, a
  stop-all control and a way to open its thread
  (https://learn.chatgpt.com/docs/agent-configuration/subagents).
- Goose: "View subagent session". Inside the child: "Subagent progress isn't
  shown live here." with "Load latest progress"
  (`G: components/ToolCallWithResponse.tsx:733-747`, `components/SubagentSessionNotice.tsx`).

**1.8 A status lane that keeps its height.** opencode reserves the status
line's height when it is empty, so the text above does not jump
(`OC: session-ui/src/timeline-next/status-line.tsx:5-9`).

**1.9 @-mentions by category.** Zed: "Files & Directories", Symbols, Fetch,
Threads, Skills, Diagnostics, "Branch Diff"
(`Z: completion_provider.rs:270-276`).

### Best example

Zed for the card rules (row vs card, open while needed). opencode for the
density presets and the status lane.

### Pitfalls

- Users split on collapse defaults. Some cannot expand subagent prompts and
  results; others want all tool output closed
  (https://claudeissues.com/issue/66646-bug-subagent-dispatch-prompt-and-returned-result-hidden-in-main-thread-collapsed,
  https://claudeissues.com/issue/91827-persistent-setting-to-collapse-tool-call-output-by-default-reopen-of-40428).
  One persistent setting, and everything expandable, settles both.
- Goose shows a tool that got no result after streaming ended as a success,
  as a workaround (`G: components/ToolCallWithResponse.tsx:540-552`). It
  hides a lost result.
- The Codex app slows after many file edits
  (https://community.openai.com/t/codex-becomes-extremely-slow-and-causes-severe-ui-lag-when-working-with-many-files/1376629).
  Long transcripts need virtual scrolling.

### For watane

- `AgentThoughtChunk` drives a thought block. It is open while chunks
  arrive and closes on the next message or tool chunk. A finished thought
  shows its duration.
- `ToolCall` kind picks the icon and row-or-card. Read, search and fetch are
  rows, folded when 2 or more follow each other. Edit, execute and any tool
  with an open ask are cards.
- A `tool` entry's output is cut at 64 KiB and marked. The UI says
  "Output cut at 64 KiB" so that nobody thinks they see it all.
- A tool whose status never arrived shows "No result" after a restart, never
  success. AGT-F-27 records such calls as aborted; show "Aborted by restart".
- An `unknown` or `unreadable` entry shows as a closed row "Unrecognised
  event", with the raw JSON behind it. Never drop it.
- The plan needs a `Plan` update. The Claude adapter table (9.8) has no line
  for `TodoWrite`, and no line for Claude's thinking blocks (section 10).

---

## 2. Permission and approval

### Patterns

**2.1 The ask lives in its tool card, which is forced open, with the diff.**
- Zed puts the prompt at the bottom of the tool card and keeps the card open
  while the ask is pending (`Z: entry_view_state.rs:100-102`, `TV:11148-11230`).
  The diff renders inside the card before approval (`TV:9927-9944`).
- Codex shows the edit in the overlay: "Would you like to make the following
  edits?" (`X: bottom_pane/approval_overlay.rs:267-308`).
- Claude Code's screen reader mode reads a diff line by line before the
  approval (https://code.claude.com/docs/en/accessibility).

**2.2 A pointer to open asks that stays in view.**
- Zed shows "Awaiting Confirmation ({n})" with a "Scroll" button above the
  composer (`TV:4781,4806`), and "Subagents Awaiting Permission:" with
  "Scroll to Subagent" (`TV:4663`).
- opencode puts the ask in a dock above the composer, headed "Permission
  required". It shows every argument and lets them scroll rather than cut
  them, because "the user approves these arguments"
  (`OC: app/src/session/requests/session-permission-dock.tsx:45,86-91`).

**2.3 Labels: once, always, deny, and deny with a reason.**
- opencode: "Deny" (ghost), "Allow always" (neutral), "Allow once" (primary)
  (`OC: app/src/session/requests/session-permission-dock.tsx:52-64`).
- Goose: "Allow Once", "Always Allow", "Deny". The header is "Goose would like
  to call {toolName}. Allow?" (`G: components/ToolCallConfirmation.tsx:44-50`,
  `ToolApprovalButtons.tsx:124-143`).
- Codex: "Yes, proceed" … "No, and tell Codex what to do differently"
  (`X: bottom_pane/approval_overlay.rs:884-955`).
- vibe-kanban: deny opens a reason box, "Let the agent know why this request
  was denied...". The default reason is "User denied this tool use request."
  (`V: web-core/src/shared/components/NormalizedConversation/PendingApprovalEntry.tsx:32,149-152`).

**2.4 The scope of "always" is shown, and sometimes withheld.**
- Codex has the finest scopes: once, command prefix, this session, a host
  for this conversation or for ever (`X: bottom_pane/approval_overlay.rs:884-955`).
- VS Code: "Allow Once / In this Session / Always Allow / Skip"
  (https://code.visualstudio.com/docs/agents/run/approvals).
- opencode lists the resource patterns that "always" will cover
  (`session-permission-dock.tsx:95-104`). Zed: "Always for `{cmd}` commands"
  (`TV:11394`).
- Goose hides "Always Allow" when the ask carries a risk note
  (`G: components/ToolApprovalButtons.tsx:124-143`). Claude in Chrome always
  asks before deletions, permission changes and sensitive data, even under
  "always allow"
  (https://support.claude.com/en/articles/12902446-claude-in-chrome-permissions-guide).

**2.5 The decision stays in the transcript, with who made it.**
- Codex: "✔ You approved codex to run X this time", "✗ Request denied for
  codex to run…", "Review timed out before codex could apply…". When a judge
  decides, the line starts "Auto-reviewer" instead of "You"
  (`X: history_cell/approvals.rs:60-183,277`).
- Goose collapses the card to "{tool} - Allowed once", "Denied once",
  "Cancelled" (`G: components/ToolApprovalButtons.tsx:107-118`).

**2.6 A stale ask says so; a timed ask counts down.**
- Goose: a late click shows "This approval request is no longer active." with
  `role="alert"` (`G: components/ToolApprovalButtons.tsx:96-101,146`).
- vibe-kanban counts down from creation to `timeout_at` and disables the
  buttons at 0 (`PendingApprovalEntry.tsx:43-68,205-210`). Goose's form:
  "Waiting for your response ({timeRemaining} remaining)", then "This request
  has expired." (`G: en.json:1022,1034`).

**2.7 Judges, and what the human sees of them.**
- Anthropic reports that people accept 93% of manual prompts. In Claude
  Code's auto mode a denial goes back to the model, and a human is pulled in
  after 3 denials in a row or 20 in all
  (https://anthropic.com/engineering/claude-code-auto-mode).
- Codex keeps the last 10 judge denials and shows them in a popup with
  "Action" and "Rationale" columns
  (`X: auto_review_denials.rs:7`, `chatwidget/permission_popups.rs:187-215`).
- Cursor's docs say plainly "The classifier can make mistakes."
  (https://cursor.com/docs/agent/security/run-modes).

**2.8 Keys.** Zed: cmd-y allow once, cmd-alt-y allow always, cmd-alt-z
reject; only the first pending card shows key hints (`Zk:295-298`). Codex: y,
a (session), p (prefix), d, Esc (tell Codex) (`X: keymap.rs:2222-2236`).
vibe-kanban: Enter approves, Cmd+Enter denies, off while a dialog is open
(`V: web-core/src/shared/keyboard/registry.ts:467-480`).

**2.9 When the user is away.**
- Notify only when the session is not in view; Zed shows "Waiting for tool
  confirmation" (`Z: conversation_view.rs:2175-2179,3624`).
- Codex in a background thread: "Approval needed in {thread}"
  (`X: bottom_pane/pending_thread_approvals.rs:48-64`).
- Jules auto-approves a plan on a timer if you leave (https://jules.google/docs/review-plan).
  Claude Code cloud sessions waiting on an approval count as inactive and
  can expire (https://code.claude.com/docs/en/claude-code-on-the-web).

### Best example

opencode's dock for clarity (every argument, scope listed, primary on
"Allow once"). Codex for decisions written back into the transcript with the
actor, judge included.

### Pitfalls

- "Waiting for Approval" with no dialog drawn: the agent hangs and the user
  cannot see why
  (https://forum.cursor.com/t/no-approve-dialog-though-agent-wait-for-the-approval/157685).
- Allow rules that never match, because agents chain commands
  (https://forum.cursor.com/t/command-allowlist-never-sticks-because-a-later-command-must-be-a-complete-match-of-a-prior-one/173017).
- Prompts for read-only commands such as `git status`
  (https://github.com/orgs/community/discussions/167487); "Yes, proceed"
  fatigue (https://community.openai.com/t/vibe-coding-too-many-yes-proceed-confirmations/1372824).
- One notification for "waiting on you" and "finished"
  (https://claudeissues.com/issue/94373-feature-desktop-notify-differently-for-waiting-on-a-permission-prompt-and-task-f).
- Clicking through asks one session at a time; users want one triage view
  (https://claudeissues.com/issue/58247-batch-permission-triage-ui-for-multiple-pending-tool-approval-prompts-across-ses).

### For watane

- An `ask` item opens the tool card of the same call and keeps it open. A
  pointer "Needs you (n)" sits above the composer while the turn is
  `waiting`.
- Buttons: "Allow", "Deny", and "Deny and say why" (sends `reason`). There
  is no "Always": profiles change only in the kernel's page (AGT-F-1). Offer
  "Change the profile…" as a link to that page instead.
- `AskUserQuestion` renders as a question form; `ExitPlanMode` as a plan with
  "Approve" and "Request changes" (vibe-kanban's composer pattern,
  `V: ui/src/components/SessionChatBox.tsx:440-462`).
- `ask-resolved {by}` collapses the card to one line with the actor:
  "Allowed by you", "Denied by you", "Expired after 10 minutes, denied",
  "Cancelled". A second answer gets `unknown-ask`; show goose's "no longer
  active" line.
- A `verdict` entry sits on the tool row: "Judge ({host}): unsafe —
  {reason}". When the provider stops the turn after denials, the idle line
  names it and links to the list of verdicts.
- The 10-minute expiry needs a deadline in the ask item for a countdown
  (section 10).

---

## 3. Control during a run

### Patterns

**3.1 Send while busy: queue or steer, and the user can see which.**
- Zed queues by default. Each queued message has a "Steer" toggle:
  "Interrupt the agent at its next step to send this message. When off,
  queued messages wait for the agent to finish." (`TV:5784-5799`). The send
  button becomes a queue icon with "Queue and Send" and "Send Immediately"
  (`TV:6827-6863`).
- VS Code turns Send into a menu: "Add to Queue / Steer with Message / Stop
  and Send" (https://code.visualstudio.com/docs/copilot/chat/chat-agent-mode).
- Codex: Enter steers, Tab queues. The preview lists "Messages to be
  submitted after next tool call (press esc to interrupt and send
  immediately)", "Messages to be submitted at end of turn", "Queued follow-up
  inputs" (`X: bottom_pane/pending_input_preview.rs:99-176`, `keymap.rs:1958-1959`).
- Cursor: Enter queues, Cmd+Enter sends now (https://cursor.com/docs/agent/overview).
  Amp does the reverse: send steers, Cmd+Enter queues (https://ampcode.com/docs/threads).
- Claude Code: a queued message is grey until Claude reads it; Up takes it
  back into the input (https://code.claude.com/docs/en/interactive-mode).

**3.2 The queue is a list you can edit.** Zed: "{n} Queued Messages",
"Next in Queue", "Edit Queued Message", "Send Now", "Remove Message from
Queue", "Clear All" (`TV:4832-5923`). Goose adds drag to reorder and "{n}
messages queued" (`G: en.json:2093-2150`). vibe-kanban allows only one
queued message (`V: web-core/src/features/workspace-chat/model/hooks/useSessionQueueInteraction.ts:43-52`).

**3.3 Stop is one button and Esc; it freezes the queue.**
- Zed: after a user Stop, the queue waits for the next manual send. "Send
  Now" cancels the turn and swallows the resulting stop event so nothing is
  sent twice. Auto-send waits for a confirmed idle
  (`Z: conversation_view/message_queue.rs:12-26,157-206`).
- Goose treats "stop", "wait" or "hold on" typed while busy as a stop, and
  pauses the queue: "Queue paused by interruption. Use "Send Now" or add a
  new message to resume." (`G: utils/interruptionDetector.ts:15-42`, `en.json:2132`).

**3.4 Stop asked is not stop done.** Zed: "Request Stop (Activity
Unconfirmed)" (`TV:6815-6817`). Goose: "Waiting for cancellation to finish".
vibe-kanban: "Stopping" (`V: SessionChatBox.tsx:520-561`).

**3.5 The interrupt leaves a mark.** Codex: "Model interrupted to submit
steer instructions." and "■ Conversation interrupted"
(`X: chatwidget/input_restore.rs:464`, `history_cell/notices.rs:353`).

**3.6 Rewind says what it restores and what it cannot.**
- Claude Code: Esc Esc or `/rewind` offers "Restore code and conversation",
  "Restore conversation", "Restore code", "Summarize from here", "Never
  mind". It warns that Bash changes are not restored, and marks mid-turn
  messages "No code restore" (https://code.claude.com/docs/en/checkpointing).
- Kiro splits Revert (last turn's files), Restore (files and context) and
  Fork (context only), and says bash and MCP edits are not tracked
  (https://kiro.dev/docs/checkpoints/).
- Cline: "Reset Chat" ("Rewind conversation, keep current code edits") and
  "Reset Code"; when no checkpoint exists: "No workspace checkpoint was
  created for this message." (`C: components/chat/UserMessage.tsx:179-233`).
- Goose: "Edit in Place" or "Fork Session" (`G: en.json:4817`). Zed:
  "Editing will restart the thread from this point." (`TV:7692-7741`).
- opencode: a "Reverted" toast with "Undo" (`OC: app/src/session/revert.ts:157-166`).

**3.7 Side questions.** `/btw` in Claude Code and Devin, `/side` in Cursor:
ask without stopping or changing the main thread
(https://claude.com/blog/claude-code-desktop-redesign,
https://docs.devin.ai/work-with-devin/devin-session-tools).

### Best example

Zed's queue: per-message Steer toggle, frozen after Stop, auto-send only on
confirmed idle. Claude Code's rewind menu for honest limits.

### Pitfalls

- Restoring a Cursor checkpoint cannot be undone, and some restores reverted
  files nobody asked for
  (https://forum.cursor.com/t/restore-checkpoint-permanently-destroys-change-history/129652,
  https://forum.cursor.com/t/cursor-exe-reverts-working-tree-files-without-clicking-restore-checkpoint/173146).
- Codex's "shift + ← to answer" for queued inputs is found awkward
  (https://community.openai.com/t/i-find-this-new-queued-follow-up-inputs-feature-a-little-frustrating/1398627).
- Windsurf stops at 40 tool calls and charges a credit to continue
  (https://docs.devin.ai/desktop/cascade/cascade).

### For watane

- `say` while `running` or `waiting` queues. Show the queue under the
  scroll with "Next" and the count "n of 32". At 32, `queue-full` disables
  Send with that reason.
- `interrupt(replacement)` is "Stop and send". `interrupt()` is Stop (Esc).
  Show "Stopping…" until the `turn` entry `idle {interrupted}` arrives.
- `cancel` is not Stop: it ends the session, denies its asks and cancels its
  children (AGT-F-12). Put it in a menu as "End session", with a confirm that
  counts the children.
- The kernel has no "steer at the next tool call", and no way to read, edit,
  reorder or remove queued inputs (section 10).
- The launch document can fork and resume at a message (4.1 `session`), but
  the kernel has no `fork` or `rewind` call. File changes are not
  checkpointed. A future rewind must say "conversation only".

---

## 4. Status and trust

### Patterns

**4.1 A small, fixed set of states, each with a word, an icon shape and a
colour.**
- Codex overview: "Needs input" (red ●), "Working" (green ●), "Ready"
  (cyan ○), "Inactive" (dim ○), "Error" (!) (`X: app/agents_overview_view.rs:92-104`).
- Claude Code agent view groups "Pinned / Ready for review / Needs input /
  Working / Completed"; needs input is yellow, failed red, stopped grey
  (https://code.claude.com/docs/en/agent-view).
- Warp tabs: working, blocked, completed, errored, plus an unread badge
  (https://docs.warp.dev/_llms-txt/agents.txt).

**4.2 A status line with the elapsed time and the key that acts on it.**
Codex: "Working (0s • esc to interrupt)", time as "1m 05s"
(`X: status_indicator_widget.rs:47,79-91`). opencode: "Working", "Waiting for
approval", "Waiting for your answer", "Waiting for shell"
(`OC: session-ui/src/timeline-next/status.ts:7-26`).

**4.3 A usage-limit pause shows the reset time and resumes on its own.**
- Claude Code: "Usage limit reached · limit resets 3:45pm", then "Continuing
  automatically at 3:45pm · Esc to cancel". After 3 automatic continues it
  stops; after a long sleep it asks "Press enter to continue"
  (https://code.claude.com/docs/en/interactive-mode).
- opencode: "Model budget reached · this model is paused until {time}"
  (`OC: UI-en:90-103`).
- Codex warns at 50, 75, 90 and 95% of a limit: "Heads up, you have less than
  {n}% of your {limit} limit left." (`X: chatwidget/rate_limits.rs:19,68`).

**4.4 A context meter with thresholds and one action.**
- Zed: a ring with a percentage, warning colour at 85%. At 80% a callout
  "Thread reaching the token limit soon", with "Start New Thread" and a hint
  to `/compact` (`TV:6017-6021,13832-13852`, `zed@0a514d7:crates/acp_thread/src/acp_thread.rs:3696-3718`).
- Goose: "{used} / {limit}", orange from 75%, red from 90%; a click opens
  "Context window" with "Compact now"
  (`G: components/bottom_menu/ContextWindowIndicator.tsx:17-36`, `en.json:335-386`).
- Codex: "{percent}% context left" (`X: bottom_pane/footer.rs:933-945`).
  VS Code breaks the use down by category on hover
  (https://code.visualstudio.com/updates/v1_109).

**4.5 Compaction is a divider with before and after.**
- Cline: "Context compacted · 120k → 30k tokens · 40 → 5 messages", and
  "Compaction failed / cancelled / skipped" (`C: components/chat/CompactionRow.tsx:34-52`).
- Zed: "Compacting Context…", "Context Compacted" with the summary behind a
  chevron (`TV:5164-5168`). Claude Code: "Summarized conversation".

**4.6 Errors are named, with one action, and retries count.**
- Zed callouts: "Rate Limit Reached", "Provider Unavailable", "Connection
  Interrupted", "Authentication Failed", "Output Limit Reached", "Request
  Refused" (`TV:12954-13059`). Retry: "Retrying. Next attempt in {n} seconds
  (Attempt {a} of {max})." (`TV:3962-3997`).
- opencode: "Attempt {n} - retrying in {s}s" (`OC: UI-en:80-89`).

**4.7 An unknown state is said, not guessed.**
- Zed: "Activity unconfirmed — The session is attached, but the agent has
  not reported its activity. You can send manually; automatic sends wait for
  confirmed Idle." (`TV:5740-5746`).
- Zed keeps sent messages after a lost connection: "Waiting… The agent
  accepted this message but hasn't shown it yet.", "Connection lost", with
  "Restore" and "Discard" (`TV:1963-2096`).

**4.8 Cost and tokens, marked as estimates.** Cline shows ↑/↓ tokens and an
estimated cost per task (https://docs.cline.bot/core-workflows/task-management).
Goose marks numbers "(estimated)" or "(reported)" (`G: en.json:2153-2189`).

### Best example

Codex's status line and overview states; Claude Code's usage-limit lines;
Zed's "Activity unconfirmed".

### Pitfalls

- A sidebar dot stays "waiting for approval" after the session ended
  (https://claudeissues.com/issue/74119-bug-desktop-sidebar-dot-stuck-in-orange-brown-waiting-for-approval-color-on-sess).
- Cost is opaque after metered billing
  (https://visualstudiomagazine.com/articles/2026/08/07/copilot-credit-complaints-keep-coming-too-expensive-to-use.aspx).
- Long runs burn money unseen (Devin ACUs; secondary,
  https://shardstitch.com/guides/devin-stuck-burning-acus/).

### For watane

The turn states map one to one:

| Turn state | Word | Shape | Line |
|---|---|---|---|
| `running` | Working | filled dot, pulse | "Working · 1m 05s · Esc to stop" |
| `waiting {asks}` | Needs you | filled dot, hand | "Waiting for your answer (2)" |
| `paused {until}` | Paused | pause bars | "Paused until 15:45 (usage limit) · continues on its own · End session" |
| `idle {completed}` | Done | hollow dot | – |
| `idle {interrupted}` | Stopped | square | "Stopped by you" |
| `idle {stopped {limit}}` | Stopped | square | "Stopped: {limit} reached" (and for the turn limit: "the agent summed up") |
| `idle {failed {error}}` | Error | ! | the error's message, its code, one action |
| `unknown` | Not running | dashed dot | "watane restarted during this turn. Send a message to continue." |
| `resume-lost` | Not running | dashed dot | "Claude no longer has this session. Your next message starts a new one from a summary." |

- A `compaction` entry is a divider: "Context compacted · {before} tokens".
  It has no "after", and there is no "compacting" state (section 10).
- `counters {turns, tool-errors-in-a-row, tokens}` give a turn and token
  count against the profile's limits. They give no context window and no
  plan usage (section 10).
- Never show `running` after `cancel` or a restart (AGT-F-10). Derive the
  state from the last `turn` entry and live updates only, never from a
  timer in the client.

---

## 5. Session management

### Patterns

**5.1 Sort by what needs the person.** opencode ranks `input` > `running` >
`unread` > `failed` > `idle` (`OC: app/src/shell/layout/project-avatar-state.ts:71-90`).
Claude Code's agent view groups by state (4.1) and puts "2 awaiting
input" in the tab title (https://code.claude.com/docs/en/agent-view). Codex
app has an Activity view of unread, running and waiting chats, Cmd+Option+U
(https://learn.chatgpt.com/docs/notifications).

**5.2 Titles made for you, editable, live.** Zed: "New Agent Thread", then a
generated title; "Regenerate Thread Title" (`Z: agent_ui.rs:262`,
`agent_panel.rs:5807`). Goose updates titles live from ACP
`session_info_update` (`G: acp/chatNotifications.ts:16-25`).

**5.3 Search and filter.** Zed: "Search all threads…", groups Today /
Yesterday / This Week / Older (`Z: threads_archive_view.rs:96-100,174`).
Cline: "Fuzzy search history...", sort "Most Expensive", "Most Tokens"
(`C: components/history/HistoryView.tsx:30-36,407`). Claude Code: filters
`s:blocked`, `n:<text>`.

**5.4 Notifications: only when hidden, by kind, and quiet by default.**
- Zed notifies only when the thread is not visible; captions "Waiting for
  tool confirmation", "Finished running tools", "Agent stopped due to an
  error"; no notice when a queued message was auto-sent; sound off by
  default (`Z: conversation_view.rs:2175-2365,3606,3624`).
- opencode has toggles per kind (agent, permissions, questions, errors) for
  system notices and sounds (`OC: APP-en:591-597,1282-1300`).
- Warp keeps a mailbox with "All tabs / Unread / Errors", at most two toasts,
  and only the parent of an orchestrated run notifies
  (https://docs.warp.dev/_llms-txt/agents.txt).

**5.5 Keys to move between sessions.** Zed Ctrl+Tab; opencode ⌥↑/⌥↓, and
⇧⌥↑/⇧⌥↓ to the next unread (`OC: app/src/shell/titlebar/tab-strip.tsx:63-65`).

**5.6 Archive, not delete.** Zed "Archive Thread" and "Delete Thread"; VS
Code "Mark as Done"; Amp "Snooze Until I'm Needed"
(https://ampcode.com/docs/threads).

**5.7 A recap on return.** Claude Code shows a one-line recap after 3 minutes
away (https://code.claude.com/docs/en/interactive-mode).

### Best example

opencode's attention ranking with jump-to-next-unread; Zed's notification
rules.

### Pitfalls

- A session list that misses sessions
  (https://claudeissues.com/issue/33363-bug-claude-code-desktop-app-session-list-incomplete-missing-sessions-from-claude).
- A redesign that lost history (Codex cloud,
  https://community.openai.com/t/codex-cloud-web-original-code-review-workflow-lost-legacy-task-history-disappearing/1416056).
- Shared transcripts that leak files the agent read; Amp ended public
  threads for this (https://ampcode.com/news/end-of-public-threads).

### For watane

- `list` gives `{id, title, profile, backend, turn, updated}`. Sort by
  turn: waiting, running, paused, failed, unknown or resume-lost, then idle.
- There is no unread mark, no last-message preview, no parent id and no
  change feed. A client must poll `list` or attach to each session, and only
  one live stream per session exists (section 10).
- The kernel never deletes on its own (AGT-F-13). Show the quota: "52 of 64
  sessions". Offer delete with what it removes; there is no archive.
- Titles come only from `rename`. A client can make one from the first
  input; the kernel does not.
- Notifications belong to the host, not to the agent layer. A client page
  that is hidden cannot tell the person an ask waits (section 10).

---

## 6. Configuration

### Patterns

**6.1 One row of pickers under the composer, each with a key.** Claude Code
desktop: model Cmd+Shift+I, effort Cmd+Shift+E, mode Cmd+Shift+M
(https://code.claude.com/docs/en/desktop). Zed: profile Cmd+I, mode
Shift+Tab, model Cmd+Alt+/ (`Zk:285-297`).

**6.2 Modes named by consequence, with a sentence each.** Goose: Autonomous
("Full file modification capabilities…"), Manual ("All tools, extensions and
file modifications will require human approval"), Smart ("Intelligently
determine which actions need approval based on risk level")
(`G: components/settings/mode/ModeSelectionItem.tsx:30-60`). Codex: "Ask for
approval", "Approve for me", "Full access", with a warning on Full access;
modes an organisation forbids show disabled
(https://learn.chatgpt.com/docs/permission-modes.md).

**6.3 What is on, at a glance.** Cline: "Auto-approve: Read, Edit", or
"None" (`C: components/chat/auto-approve-menu/AutoApproveBar.tsx:44,109`).
Zed: "Tools Unsupported" when the model has no tools
(`Z: profile_selector.rs:185`); MCP servers in the options menu.

**6.4 Profiles as named tool sets.** Zed: Write ("Get help to write
anything."), Ask ("Chat about your codebase."), Minimal ("Chat about anything
with no tools.") (`Z: profile_selector.rs:392-394`). Roo modes remember their
last model (https://roocodeinc.github.io/Roo-Code/basic-usage/using-modes).

### Best example

Goose's mode descriptions; Zed's profile names.

### Pitfalls

Labels that change between releases ("Ask permissions" became "Manual" in
Claude Code; https://code.claude.com/docs/en/desktop). Modes hidden until
enabled in settings (Codex).

### For watane

- The client does not change the profile; the participant does, in the
  kernel's page. Show the chosen profile as a chip: name, model, mode, number
  of tools and skills. Its menu says "Edit in Agents".
- Before `start`, call `check(profile, overlay)` and show every refusal at
  once, by field (AGT-F-4, AGT-F-5), like Zed's "Tools Unsupported".
- Show the session's grants as the chip's detail: folder, toolsets, the
  provider (checked or not, with the judge's host).
- Consents are asked in the app. The client needs a state for "waiting for
  your consent in watane", or the start just fails (section 10).

---

## 7. Delegation and review

### Patterns

**7.1 A board or an inbox of agents.** vibe-kanban: "To Do", "In Progress",
"In Review", "Done", "Cancelled"; one worktree per attempt; a retry is a new
attempt. Conductor: one workspace per task, "Create PR" Cmd+Shift+P
(https://www.conductor.build/docs/concepts/workflow). Cline Kanban cards show
the agent's latest message or tool call
(https://docs.cline.bot/kanban/features).

**7.2 Child inline, child full screen, and a way back.** Zed's subagent card
(1.7). opencode's child view has a bar "Main session" with an Esc hint
(`OC: app/src/session/composer/subagent-bar.tsx:109-111`). Claude Code opens
a child with Enter and lets you type to it.

**7.3 A child's ask bubbles up.** Zed: "Subagents Awaiting Permission:" with
"Scroll to Subagent" (`TV:4663`). Codex: "Approval needed in {thread}"
(`X: bottom_pane/pending_thread_approvals.rs:48-64`).

**7.4 Stop all, archive all.** Codex app stop-all; Amp "Archive N Threads"
with the children.

**7.5 Review changes as a diff with scopes, per-hunk keep or reject, and
batched comments.**
- Zed: "Review: {title}", per hunk "Keep" and "Reject", "Keep All", "Reject
  All", and an "Undo" toast after a reject (`Z: agent_diff.rs:686-1249`,
  `ui/undo_reject_toast.rs:13-19`).
- Codex app scopes: Unstaged, Staged, Commit, Branch, Last turn
  (https://learn.chatgpt.com/docs/code-review.md?surface=app). opencode:
  "Session changes", "Last turn changes", "Diff too large to render" with
  "Render anyway" (`OC: UI-en:6-37`).
- Line comments gather and go back as one message: vibe-kanban ("{{count}}
  review comments will be included"), Conductor, Claude Code (Cmd+Enter
  sends all) (https://code.claude.com/docs/en/desktop).
- VS Code "Mark as Reviewed", cleared when the file changes again
  (https://code.visualstudio.com/docs/agents/run/review-code-edits.md).

### Best example

Zed's review buffer with undoable reject. vibe-kanban's board for many
parallel agents.

### Pitfalls

- Review, not agents, is the bottleneck: "4 agents finished in 25 minutes;
  review took 3 hours" (blog anecdote,
  https://pub.towardsai.net/i-ran-4-claude-code-agents-in-parallel-the-bottleneck-was-me-f596bc7f393e).
- No diffs to accept in the Codex IDE extension
  (https://community.openai.com/t/codex-vscode-extension-does-not-show-diffs-which-hinders-development/1396210).
- No parent and child tree in the Codex app
  (https://community.openai.com/t/hierarchical-multi-agent-workflow-visual-tree-based-agent-orchestration-in-codex-app/1401395).

### For watane

- A child session is a session with a `parent`. Draw it as a card in the
  parent's scroll and as an indented row in the list. The parent's stream
  does not say which child a tool call started, and `session-info` has no
  parent (section 10).
- `cancel` on a parent cancels the children. The confirm names them.
- Who answers a child's ask is not said. If the owner of the child is the
  same client, its asks should show in the parent's "Needs you" pointer.
- watane keeps no diffs. Edits in a workspace reach the client as ACP tool
  calls; if the adapter fills ACP `Diff` content, the client can list
  "Changes in this session". Keep and reject are the client mod's business.

---

## 8. Accessibility and keyboard

### Patterns

**8.1 A linear, labelled reading mode.** Claude Code's screen reader mode
labels lines `you:`, `claude:`, `thinking:`, `tool:`, `tool error:`,
`Permission Required:`; menus become numbered lists; mode changes are spoken
("[plan mode on]"); the bell rings on a reply end, a prompt, and a tool that
ran over 5 seconds; `prefersReducedMotion` stops spinners and shimmer
(https://code.claude.com/docs/en/accessibility).

**8.2 Announce state, not tokens.** Set `aria-busy` while a reply streams and
announce when it ends; make the approval prompt reachable and focused
(https://tianpan.co/blog/2026-06-29-streaming-tokens-meet-the-screen-reader,
https://jira.atlassian.com/browse/ROVO-978). Goose uses `role="status"` for
notices, `role="alert"` for a stale ask, and `aria-label="Tool status:
{status}"` on the dot (`G: components/ToolCallStatusIndicator.tsx:25-45`).

**8.3 Colour never alone.** Codex pairs colour with ● ○ ! and falls back to a
static "•" when animations are off (`X: exec_cell/render.rs:203-209`).

**8.4 Every action has a key, and the keys are findable.** Claude Code
desktop lists all with Cmd+/; agent view with `?`. Zed jumps between messages
with Shift+PageUp/PageDown (`Zk:299-314`); Claude Code with `{` and `}`.
vibe-kanban uses chords, such as `g s` for settings
(`V: web-core/src/shared/keyboard/registry.ts:78-293`). Codex lets every key
be remapped (`X: keymap.rs`).

**8.5 Accessible view of a reply.** VS Code Alt+F2 reads a response line by
line (https://code.visualstudio.com/docs/chat/chat-overview).

### Pitfalls

- Screen readers repeated progress messages over and over
  (https://www.github.com/microsoft/vscode/issues/211165).
- Cline's context bar lacks a role; a TODO says so
  (`C: components/chat/task-header/ContextWindow.tsx:169`).
- No product documents screen-reader support for its graphical agent panel.

### For watane

- One live region per session, polite, announcing turn-state changes only:
  "Needs your answer", "Done", "Stopped: {limit}". Asks are assertive.
- Focus moves to a new ask only when the person is not typing; otherwise
  the pointer gets focus with the next Tab.
- Each turn state has its own shape, not only colour (table in 4).

---

## 9. Ranking

### The 15 patterns most worth adopting

1. **One status line from the turn state.** A word, a shape, elapsed time and
   the key that acts, for each of the nine states, including honest
   "Not running" for `unknown` and `resume-lost` (Codex, opencode, Zed
   "Activity unconfirmed").
2. **The ask inside its tool card, forced open, with every argument and the
   diff,** plus a "Needs you (n)" pointer above the composer (Zed, opencode).
3. **Decisions stay in the transcript with the actor:** you, the judge,
   expired, cancelled (Codex, goose). Maps to `ask-resolved {by}` and
   `verdict`.
4. **Deny with a reason** that goes to the agent (vibe-kanban, Codex).
5. **A visible queue while busy:** queue by default, "Stop and send" as the
   explicit interrupt, "n of 32" (Zed, VS Code, Codex).
6. **Stop freezes the queue,** and "Stopping…" shows until idle is confirmed
   (Zed message queue).
7. **A paused-until line** with the reset time, automatic resume and a way
   out (Claude Code, opencode).
8. **Cheap tool calls fold into one line;** cards only for edits, shells and
   asks; failures always open (Zed, Codex, Cline, VS Code).
9. **The thought block opens while streaming and closes when done,** with its
   duration (Zed, opencode, Cline).
10. **The plan pinned above the composer,** not in the scroll (Zed, opencode,
    Claude Code).
11. **A session list ranked by need:** needs you, running, paused, failed,
    idle; a key to jump to the next one that needs you (opencode, Claude
    Code agent view).
12. **Notifications only when hidden, split into "needs you" and "done",**
    none for an auto-sent queued input, sound off by default (Zed, opencode).
13. **Judge verdicts on the tool row,** a list of recent denials with
    reasons, and the count toward the stop rule (Codex auto-review).
14. **A subagent card** with status, the last few entries, Stop and Open;
    child asks bubble up to the parent (Zed, Claude Code, Codex app).
15. **A density setting with one persistent choice,** everything expandable,
    nothing hidden for good (opencode presets, VS Code).

### Five anti-patterns

1. **A waiting state with no visible prompt.** The agent waits on an ask the
   UI never drew, or that never got focus (Cursor forum 157685, ROVO-978).
2. **Stale state:** a dot that says running or waiting after the session
   ended (claudeissues 74119). AGT-F-10 forbids it; the UI must not guess
   from timers.
3. **Rubber-stamp prompts:** asking for every read-only command, and
   "always" rules that never match, so people click yes 93% of the time
   (Anthropic auto mode, Cursor allowlist threads).
4. **A restore that does less than it says,** or cannot be undone (Cursor
   checkpoints; shell edits never tracked anywhere).
5. **Showing a guess as a fact:** a tool with no result drawn as success
   (goose workaround), cost without "estimated", or collapsed content that
   cannot be opened (Claude Code subagent issue 66646).

---

## 10. What a good UI needs that the agent layer does not yet give

Each is a gap in the draft, not a decision. The owning document decides.

1. **A live turn-state event.** Turn states are entries (9.4), and live
   updates are ACP `SessionUpdate`s, which have no turn state. The spec does
   not say how `running → waiting → paused` reaches an attached client live.
   A `_meta.watane` turn update is needed.
2. **The message in progress on attach.** Deltas are never entries, so a
   client that attaches mid-message gets the rest of it without its start.
   Attach needs a snapshot of the open message, thought and tool calls.
3. **The queue as data.** `say` returns an index, but there is no event
   when a queued input is taken, and no call to list, edit, reorder or
   remove queued inputs.
4. **Steer at the next tool call.** Only a hard `interrupt` exists. Claude
   Code and Codex both deliver a mid-turn message after the current tool
   call. Decide whether the kernel offers it.
5. **Ask details.** The ask item has `tool` and `input` only. A UI needs the
   tool call id it belongs to, a deadline (`expires-at`) for the 10-minute
   rule, and the diff for an edit.
6. **"Checking" while a judge runs.** A check can take until the judge's
   timeout. The client sees nothing until the verdict. A pending-check update
   would let it show "Checking with {judge host}…". It also needs the
   denial count toward AGT-F-33.
7. **Plans and Claude's thinking.** The 9.8 adapter table maps no Claude
   `TodoWrite` to an ACP `Plan`, and no Claude thinking block to
   `AgentThoughtChunk`. Tool kind, title, locations and `Diff` content are
   not said to be filled either.
8. **Context and usage.** Counters hold tokens used, not the context window,
   the percentage, or plan usage left. There is no warning before a limit,
   and no reset time for a Claude usage limit (only OpenAI's `paused`).
9. **Compaction in progress and its result.** There is no "compacting"
   state, and the entry has tokens before but not after.
10. **A status feed across sessions.** `list` is a poll, and one live stream
    per session replaces the last. A client that shows many sessions needs a
    cheap feed of `{session, turn, needs-you, updated}` changes, and an
    unread mark or last-entry index.
11. **Children in the parent's stream.** No event links a parent's tool call
    to the child session it started, `session-info` has no `parent`, and it
    is not said who answers a child's asks.
12. **Fork and rewind.** The launch document supports `fork` and resume
    `at`, but the kernel interface has neither. Edit-and-resend needs one.
13. **Pending consent.** Consents are asked in the app and no call waits.
    The client needs a state or error such as `consent-pending {kind}` to say
    "Answer the question in watane".
14. **Notifications.** Nothing lets a hidden client page, or the kernel, ask
    the host to notify the person that an ask waits or a turn ended.
15. **Titles.** Only `rename` exists. Decide whether the kernel suggests a
    title, or each client makes its own.
