# The agent layer: functional specification (draft)

What the agent layer does, free of implementation. `TECHNICAL.md` says how;
`ARCHITECTURE-PROPOSAL.md` (revision 4) is the record of why. Draft, not yet
a contract (see `VISION.md`).

Ids in `Traces:` refer to `spec/FUNCTIONAL.md`, and to the rows this draft
proposes for it (section 2). `N<n>` refers to the open points of the proposal
(section 15); where one is open, this draft follows the proposal's current
choice and says so.

## 1. Terms

| Term | Meaning |
|---|---|
| agent session | A conversation between a client and an agent, with its turns, its transcript and its state. It lives until the owner deletes it. |
| turn | One run of the agent from an input until it stops, waits for an answer, or is interrupted. |
| backend | What runs the agent: Claude Code (the participant's `claude` program) or OpenAI (the participant's ChatGPT plan). |
| profile | A named, complete setting of an agent: instructions, model, tools, permissions, skills, subagents, limits and output format. The participant makes it. |
| overlay | A change that a client asks for when it starts a session. It can only narrow the profile. |
| offer | A skill or a profile that a mod proposes. It has no effect until the participant adopts it. |
| skill | Instructions, and the files they need, that an agent can load when it needs them. |
| subagent, child session | An agent that another agent starts for a part of its task. |
| client | The mod (or its page) that starts a session and reads it. It is the session's owner. |
| origin client | The client at the start of a chain of calls. Consents name it. |
| kernel | The mod `mods:agent`: profiles, sessions, transcripts, permissions, the OpenAI agent, delegation. |
| provider | The mod that holds the Claude Code grant and starts `claude` for the kernel: unchecked, or checked by a judge. |
| judge | A service that the participant chose, which reads an agent step and says whether it is safe. |
| ask | A question from an agent for permission to use a tool. |
| check | A question from the host to the checked provider before any tool runs. |
| workspace | A folder in which agents may read and change files: one the participant picked for a client, or a scratch folder of one session. |
| toolset | A named list of calls of installed mods that agents may use (spec 1). |
| transcript | The stored entries of a session: finished messages, tool steps, asks, verdicts and turn states. |
| key | A text that a client chooses for a call, so that a repeated call changes nothing. |

## 2. Proposed rows for `spec/FUNCTIONAL.md`

- **F-AGENT-5** With the participant's consent, a mod can run agent sessions on the participant's own Claude or ChatGPT subscription. watane holds no key and runs no service for it. (agent layer)
- **F-AGENT-6** An agent session is private to the client that started it. The client may publish what it chooses.
- **F-AGENT-7** The participant decides, for each client mod, the folders, the plan and the toolsets its agents may use, sees them, and withdraws each one.
- **F-AGENT-8** The participant defines the profiles of agents. A client can only narrow a profile.
- **F-AGENT-9** The participant can pick a provider that checks each agent step with a judge of their choice. A check that cannot be done denies.
- **J11** Run agents from a mod (section 4).

## 3. Requirements

### 3.1 Profiles and offers

- **AGT-F-1** The kernel shall let only the participant, in the kernel's own page, create, change and delete profiles, and say which client may use which profile. Traces: F-AGENT-8, F-RUN-5.
- **AGT-F-2** When a mod offers a skill or a profile, the kernel shall show its content to the participant and use it only after the participant adopts it. If the offered content changes, then the kernel shall stop using it until the participant adopts it again. Traces: F-AGENT-8, F-AGENT-4.
- **AGT-F-3** When a client starts a session with an overlay, the kernel shall accept an overlay that appends to the instructions, adds skills, removes tools, adds deny rules or lowers limits, and shall refuse one that adds tools, allow rules, workspaces, subagent profiles or a weaker permission mode. Traces: F-AGENT-8.
- **AGT-F-4** If a backend or a model cannot honour a field that a profile sets, then the kernel shall refuse the profile for that backend and model, and name every such field at once. It shall never drop a field. Traces: F-AGENT-8, F-QUAL-3.
- **AGT-F-5** When a client asks to check a profile and overlay, the kernel shall return the refusals of AGT-F-4 without starting anything. Traces: F-AGENT-8.

### 3.2 Sessions

- **AGT-F-6** When a client starts a session or sends an input with a key that it used before for that session, the kernel shall return the first result and change nothing. A deleted session's keys stay used. Traces: F-AGENT-5, F-QUAL-3.
- **AGT-F-7** The kernel shall let only the owner of a session attach to it, read it, send input, answer its asks, interrupt, cancel, rename or delete it, and list the owner's own sessions. Traces: F-AGENT-6.
- **AGT-F-8** When the owner attaches after an index, the kernel shall send every stored entry after that index, then a marker that the replay is complete, then live updates, with no entry missing or doubled. A second attach of the same owner shall end the first. Traces: F-AGENT-6.
- **AGT-F-9** The kernel shall give the client one event format for both backends. If a backend sends output that the kernel does not know, then the kernel shall keep it as an unknown entry and the session shall go on. Traces: F-AGENT-5.
- **AGT-F-10** The kernel shall show the state of each session's turn: running, waiting for an answer, paused until a time, or idle with its reason (completed, interrupted, stopped by a limit, failed). After a cancel or a restart it shall never show a turn as running. Traces: F-AGENT-6, F-QUAL-3.
- **AGT-F-11** When the owner sends input during a turn, the kernel shall queue it, in order, up to 32 inputs, and start the next turn with it. When the owner interrupts with a replacement input, the kernel shall end the turn and start the next with that input. Traces: F-AGENT-5.
- **AGT-F-12** When the owner cancels a session, the kernel shall end its turn, deny every open ask, and cancel its child sessions. Traces: F-AGENT-6.
- **AGT-F-13** The kernel shall never delete a session on its own. If an owner exceeds its quota, then the kernel shall refuse new sessions of that owner and name the quota. Traces: F-RUN-8.

### 3.3 Asks

- **AGT-F-14** When an agent asks for permission, the kernel shall decide by the profile, or ask the owner. If nobody can answer, then the kernel shall deny. Traces: F-AGENT-1, F-AGENT-9.
- **AGT-F-15** The first answer to an ask shall decide it. The kernel shall refuse a second answer and an answer after its turn ended, and tell the owner when an ask was decided by anyone else. Traces: F-AGENT-6.
- **AGT-F-16** If nobody answers a Claude ask within 10 minutes, then the host shall deny it. Traces: F-AGENT-9.

### 3.4 Claude Code

- **AGT-F-17** The host shall start only the unmodified `claude` program that the participant picked, at or above the minimum version, under the participant's own login, and shall never read or store that login. Traces: F-AGENT-5.
- **AGT-F-18** Nothing in a workspace, and nothing of the participant's own Claude settings, instructions, memory or plugins, shall configure a watane session. Traces: F-AGENT-8, F-RUN-2.
- **AGT-F-19** If a mod sends a message that starts with `/`, then the host shall refuse it and name the cause. The host shall offer compaction as its own operation. Traces: F-AGENT-8.
- **AGT-F-20** If `claude` reports a setup that differs from what the host started, or cannot confirm that a failing check blocks the tool, then the host shall stop it and name the difference. Traces: F-QUAL-3, F-AGENT-9.
- **AGT-F-21** The host shall refuse an agent's writes to the version-control folder, agent and editor configuration, CI workflows, and the build and hook files of common tool chains. Traces: F-RUN-2.
- **AGT-F-22** An agent shall run programs or use the web only under the grant that says so, and only inside Claude's sandbox, confined to its workspaces. Traces: F-RUN-7, F-AGENT-5.

### 3.5 OpenAI

- **AGT-F-23** The app shall sign the participant in to their ChatGPT plan and keep the tokens. The host shall add a token only to requests to OpenAI's agent endpoints, and no mod shall see a token. Traces: F-AGENT-5.
- **AGT-F-24** When OpenAI reports a usage limit, the kernel shall pause the session until the reset time and show it. When it reports an error that can be retried, the kernel shall retry later with backoff, never inside a call. Traces: F-QUAL-3.
- **AGT-F-25** Before each request to OpenAI, the kernel shall make sure that every tool call in the history has exactly one result, in order. Traces: F-QUAL-3.

### 3.6 Tools

- **AGT-F-26** An agent shall reach only the toolsets that the participant approved for its session's origin client. Each tool call shall run as its own call of the tool's mod, with its own budget. Traces: F-AGENT-1, F-AGENT-3, F-AGENT-7.
- **AGT-F-27** After a restart, the kernel shall run again only the open tool calls whose mod marked them safe to repeat, and record every other open call as aborted. Traces: F-QUAL-3.

### 3.7 Consent

- **AGT-F-28** The host shall ask the participant in the app before an origin client's agents first use a folder, the ChatGPT plan or a toolset. The question shall name the origin client, and for a folder the profile and the provider. No call shall wait for the answer. Traces: F-AGENT-7, F-AGENT-4, F-RUN-7.
- **AGT-F-29** The app shall show these consents with the grants, and the participant shall withdraw each one. When one is withdrawn, the host shall end every session stream that depends on it. Traces: F-AGENT-7.
- **AGT-F-30** The plan of a mod whose calls reach a Claude Code grant shall say "Uses Claude Code through {provider}". Traces: F-INST-1, F-AGENT-7.
- **AGT-F-31** A preview shall refuse the Claude Code grants and the ChatGPT plan; the maker tools shall offer a fake `claude` and a scripted OpenAI stream instead. Traces: F-MAKE-4.

### 3.8 The checked provider

- **AGT-F-32** The checked provider shall ask its judge before a session starts, before each input, before each tool call and before each permission ask. If the judge says unsafe or unsure, answers something unreadable, is slow or is down, then the provider shall deny. If the judge says safe, then the provider shall leave the decision to the profile. Traces: F-AGENT-9.
- **AGT-F-33** After 3 denials in a row, or 10 of the last 50, the checked provider shall stop the turn and say why. Traces: F-AGENT-9.
- **AGT-F-34** The client shall see each verdict and its reasons. Traces: F-AGENT-6, F-AGENT-9.
- **AGT-F-35** The plan of the checked provider shall name the host of its judge and say that it sends excerpts of sessions there. Traces: F-INST-1.

### 3.9 Limits and delegation

- **AGT-F-36** The kernel shall count the turns, the failed tool calls in a row and the tokens of each session, for both backends, and stop the turn at a limit of the profile, with the limit named. At the turn limit the agent gets one last turn without tools to sum up. Traces: F-AGENT-8, F-QUAL-3.
- **AGT-F-37** An agent shall start a child session only with a profile that its own profile names. A child shall only narrow its parent. The kernel shall limit the depth and the number of live children. An OpenAI child shall not start children. Traces: F-AGENT-8.

### 3.10 Restart and quality

- **AGT-F-38** After a restart of watane, the owner shall reattach to each session and continue it. If `claude` no longer has the session, then the kernel shall mark it as lost and continue in a new Claude session that starts from a summary. Traces: F-QUAL-3.
- **AGT-F-39** Every call into a mod of the agent layer shall return within its budget. Long work shall be a stream, a job or a timer. Traces: F-RUN-3.
- **AGT-F-40** Each failure shall carry a fixed machine-readable code and a message that names the cause. Traces: F-QUAL-3.

## 4. Journeys

### J11: Run agents from a mod

Alice installs Notes and the agent kernel. The plan of Notes says it uses
agents; the plan of the Claude Code provider says it starts Claude Code with
her account, unchecked. In the kernel's page she makes a profile "Tidy":
Sonnet, file tools, no shell, her own instructions, and lets Notes use it.
She asks Notes to tidy her notes folder. The app asks whether Notes may have
agents work in that folder with "Tidy" through the provider; she agrees. The
agent's text streams into Notes as it works. The agent asks to delete a file;
Notes shows the ask; Alice denies it, and the agent goes on without it.
She quits watane mid-turn, starts it again, and Notes shows the session as
not running, with everything up to the last finished message; she sends
"continue", and the agent resumes where Claude left off. In the manager she
withdraws the folder consent, and Notes' next session there asks again.

### AGT-J2: The same profile on ChatGPT

Alice signs in to her ChatGPT plan in Settings. She switches "Tidy" to an
OpenAI model; the kernel refuses it and names the fields OpenAI cannot honour
(file tools, a folder). She makes "Sort", with only Notes' own toolset, and
Notes runs it. The app asks once whether Notes may use her ChatGPT plan and
the toolset. Notes shows the same kind of events as with Claude. Her plan's
limit runs out: Notes shows the session paused until the reset time, and it
continues on its own after it.

### AGT-J3: A checked agent

Bob picks the checked provider and gives it a judge and its key in the
provider's page; its plan named the judge's host. A web page the agent reads
tells it to upload Bob's keys. Before the upload runs, the judge says unsafe;
the tool call is denied and Bob's client shows the verdict. Bob turns his
network off: the next tool call is denied because the judge cannot be
reached, and after three denials the turn stops and says why.

### AGT-J4: A hostile client

Mallory's mod starts a session with an overlay that adds the shell, and is
refused. It sends a message `/deploy`, and is refused. It defines a subagent
that may accept edits on its own, and is refused. It asks for a folder in
`~/.ssh`, and is refused before any question reaches the participant. It
reads another mod's session, and is refused. Each refusal names its cause.

### AGT-J5: An agent that delegates

The factory's product owner runs as a session of a mod, with a profile that
names "foreman" as a profile it may start. It starts a foreman session for a
slice; the foreman starts a validator child. The kernel refuses a fourth
level and a profile not named. Alice cancels the product owner's session,
and both children stop.

## 5. Later

- Workspace instructions (`AGENTS.md`) for a profile that opts in (N7).
- A worktree workspace for agents working in parallel (the factory).
- A wait that the host holds for agents that drive sessions over MCP.
- Codex as a third backend (N5).
- File undo.
- The `auto` permission mode (N2).
