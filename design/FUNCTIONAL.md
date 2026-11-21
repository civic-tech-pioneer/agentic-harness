# Agentic harness: functional specification

What the agent layer does, free of implementation. It sits between
`VISION.md` (why), which outranks it, and `TECHNICAL.md` (how), which it
outranks. `ARCHITECTURE-PROPOSAL.md` (revision 4) is the record of why.

Sections 3.1 to 3.10 are the backend, which comes first (`VISION.md`, what
comes first). Section 3.11 is the console, built alongside; each of its
requirements names the backend requirements it rests on, and none asks for
more than they give.

Ids in `Traces:` refer to `spec/FUNCTIONAL.md`, to the rows this file
proposes for it (section 2), and to this file's own `AGT-F` ids. `N<n>`
refers to the open points of the proposal (section 15), which the human
answered on 2026-11-20 (N11 of section 5 too) except N5 (decided at stage
F); a build slice that depends on N5 waits. An `AGT-F` id
never changes; a new requirement takes the next free number, in the section
where it belongs.

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
| console | The kernel's own page: a chat in which the participant runs and watches sessions. It is a client of the kernel like any other. |
| session allow | An answer to an ask that allows that kind of call for the rest of the session. |
| queue | The inputs an owner sent during a turn, waiting for it to end. |

## 2. Rows accepted (N3) for `spec/FUNCTIONAL.md`, applied by a watane slice (`TECHNICAL.md` 15)

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
- **AGT-F-8** When the owner attaches after an index, the kernel shall send every stored entry after that index, then a marker that the replay is complete, then live updates, with no entry missing or doubled. If more entries lie after that index than one replay may carry, then the kernel shall say so, and the owner shall read them in pages first, from either end. The work of each call shall have a bound that does not grow with the transcript. A second attach of the same owner shall end the first. Traces: F-AGENT-6, AGT-F-39.
- **AGT-F-9** The kernel shall give the client one event format for both backends. If a backend sends output that the kernel does not know, then the kernel shall keep it as an unknown entry and the session shall go on. Traces: F-AGENT-5.
- **AGT-F-10** The kernel shall show the state of each session's turn as it changes: running (and, while it lasts, checking with the judge, making room in the context, or waiting for the participant's consent in the app), waiting for an answer, paused until a time, or idle with its reason (completed, interrupted, stopped by a limit, failed). After a cancel or a restart it shall never show a turn as running when nothing drives it. Traces: F-AGENT-6, F-QUAL-3.
- **AGT-F-11** When the owner sends input during a turn, the kernel shall queue it, in order, up to 32 inputs and 2 MiB, and start the next turn with it. When the owner interrupts with a replacement input, the kernel shall end the turn and start the next with that input. The kernel shall refuse an input larger than its limit and name the limit. Traces: F-AGENT-5.
- **AGT-F-41** The kernel shall let the owner see the queued inputs of a session and take one out before a turn takes it. Traces: AGT-F-11, F-AGENT-6.
- **AGT-F-54** The kernel shall keep and return every entry whole, whatever its size, and every read and replay shall make progress. Traces: AGT-F-8, AGT-F-9.
- **AGT-F-12** When the owner cancels a session, the kernel shall end its turn, deny every open ask, and cancel its child sessions. Traces: F-AGENT-6.
- **AGT-F-13** The kernel shall never delete a session on its own. If an owner exceeds its quota, then the kernel shall refuse new sessions of that owner and name the quota. If the Claude Code sessions the kernel may run at once are taken, then the kernel shall refuse to start or resume one, name the cap, and never wait. Traces: F-RUN-8, AGT-F-39.

### 3.3 Asks

- **AGT-F-14** When an agent asks for permission, the kernel shall decide by the profile, or ask the owner. If nobody can answer, then the kernel shall deny. Traces: F-AGENT-1, F-AGENT-9.
- **AGT-F-15** The first answer to an ask shall decide it. The kernel shall refuse a second answer and an answer after its turn ended, and tell the owner when an ask was decided by anyone else. Traces: F-AGENT-6.
- **AGT-F-16** If nobody answers a Claude ask within 10 minutes, then the host shall deny it. Traces: F-AGENT-9.
- **AGT-F-42** The kernel shall tell the owner, with each open ask, the tool call it is about, its full input, and when it will be denied unanswered. Traces: AGT-F-14, AGT-F-16.
- **AGT-F-43** When the owner answers an ask with a session allow, the kernel shall name its reach (the tool and its scope: the folder of a file tool, the exact command, the web host or the skill's name), keep it only for that session, allow later only calls with that same tool and scope, and let the owner take it back; after that, the same kind of call asks again. A tool or call whose scope cannot be named safely gets no session allow and always asks. Traces: AGT-F-14, F-AGENT-8.

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
- **AGT-F-55** When an agent asks for more tool calls than the kernel may run at once, the kernel shall run them in turns, each exactly once, and keep room for other sessions' requests to the model. Traces: AGT-F-26, AGT-F-39.

### 3.7 Consent

- **AGT-F-28** The host shall ask the participant in the app before an origin client's agents first use a folder, the ChatGPT plan or a toolset. The question shall name the origin client, and for a folder the profile and the provider. No call shall wait for the answer. Traces: F-AGENT-7, F-AGENT-4, F-RUN-7.
- **AGT-F-29** The app shall show these consents with the grants, and the participant shall withdraw each one. When one is withdrawn, the host shall end every session stream that depends on it. Traces: F-AGENT-7.
- **AGT-F-30** The plan of a mod whose calls reach a Claude Code grant shall say "Uses Claude Code through {provider}". Traces: F-INST-1, F-AGENT-7.
- **AGT-F-31** A preview shall refuse the Claude Code grants and the ChatGPT plan; the maker tools shall offer a fake `claude` and a scripted OpenAI stream instead. Traces: F-MAKE-4.

### 3.8 The checked provider

- **AGT-F-32** The checked provider shall ask its judge before a session's first input reaches Claude Code, before each input, before each tool call and before each permission ask. If the judge says unsafe or unsure, answers something unreadable, is slow or is down, then the provider shall deny. If the judge says safe, then the provider shall leave the decision to the profile. The provider shall send nothing of a session to its judge before the participant has consented to the session's workspace, and shall pass nothing to Claude Code before its verdict. Traces: F-AGENT-9.
- **AGT-F-33** After 3 denials in a row, or 10 of the last 50, the checked provider shall stop the turn and say why. Traces: F-AGENT-9.
- **AGT-F-34** The client shall see each verdict and its reasons. Traces: F-AGENT-6, F-AGENT-9.
- **AGT-F-35** The plan of the checked provider shall name the host of its judge and say that it sends excerpts of sessions there. Traces: F-INST-1.
- **AGT-F-53** The app shall keep the judge's key, which the participant enters in the app for one checked provider and one judge origin. The host shall add it only to that provider's requests to that origin, and no mod shall see it. The participant shall see that a key is kept and remove it. Traces: F-AGENT-9, AGT-F-23.

### 3.9 Limits and delegation

- **AGT-F-36** The kernel shall count the turns, the failed tool calls in a row and the tokens of each session, for both backends, and stop the turn at a limit of the profile, with the limit named. At the turn limit the agent gets one last turn without tools to sum up. Traces: F-AGENT-8, F-QUAL-3.
- **AGT-F-37** An agent shall start a child session only with a profile that its own profile names. A child shall only narrow its parent. The kernel shall limit the depth and the number of live children. An OpenAI child shall not start children. Traces: F-AGENT-8.
- **AGT-F-44** The kernel shall report, after each step, how much of the model's context a session uses, and, where the backend says so, at what point it makes room by summarising. Traces: AGT-F-36, F-QUAL-3.

### 3.10 Restart and quality

- **AGT-F-38** After a restart of watane, the owner shall reattach to each session and continue it. If `claude` no longer has the session, then the kernel shall mark it as lost and continue in a new Claude session that starts from a summary. Traces: F-QUAL-3.
- **AGT-F-39** Every call into a mod of the agent layer shall return within its budget. Long work shall be a stream, a job or a timer. Traces: F-RUN-3.
- **AGT-F-40** Each failure shall carry a fixed machine-readable code and a message that names the cause. Traces: F-QUAL-3.

### 3.11 The console

The console's first target is the canvas row named in `VISION.md`. Each
requirement here is met by what the backend requirements it traces give, and
by nothing else.

- **AGT-F-45** The kernel's page shall hold the console. The console shall reach the kernel only through the calls every client has, and shall show and change only the sessions it started. Traces: AGT-F-7, F-AGENT-6. It shall never show the sessions of other clients (N11, decided).
- **AGT-F-46** When the participant starts a session in the console, the console shall offer the profiles the console may use and both backends, and let them narrow the profile for that session. Before anything starts, it shall show every refusal of AGT-F-4 for that backend and keep Start off while one holds. It shall show what the session can reach: what is already allowed and what will be asked. Traces: AGT-F-3, AGT-F-4, AGT-F-5, AGT-F-28.
- **AGT-F-47** The console shall show a session's transcript from its stored entries and live updates: inputs, thinking, tool calls, asks, verdicts, decisions with who made them, and the end of each turn with its time, tools and tokens. Reads and searches in a row shall fold into one line. Edits, program runs, asks and failures shall show in full, and a failed call shall show open. After a reload or a restart of watane, the same transcript shall show again, nothing missing or doubled. Traces: AGT-F-8, AGT-F-9, AGT-F-34, AGT-F-38.
- **AGT-F-48** The console shall show one status line for the current turn: a word, a shape for each state that does not rely on colour, the time running or, for an ask, the time left, and the key that acts on it. It shall show no state, time or plan that the kernel did not report. Traces: AGT-F-10, AGT-F-42.
- **AGT-F-49** When an ask waits, the console shall show it inside its tool call, with every argument and, for an edit, the change. It shall keep a "Needs you" pointer until the ask is answered, and offer: allow once; allow for the session, with its reach named; and deny with a reason the agent receives. A session allow shall stay shown until the session ends or the participant takes it back. Traces: AGT-F-14, AGT-F-15, AGT-F-42, AGT-F-43.
- **AGT-F-50** The console shall accept input during a turn and show the queue: the participant shall take a queued input back to edit it, remove it with an undo, and interrupt the turn to send one now. When the queue is full, sending shall be off and the console shall say why. Stop shall show "Stopping…" until the kernel reports the turn ended. Traces: AGT-F-11, AGT-F-41.
- **AGT-F-51** The console shall list its sessions grouped by need (needs you, running, then recent) and shall not move a row while the pointer is over the list. It shall show how full the context is and warn before the kernel makes room. Traces: AGT-F-10, AGT-F-44.
- **AGT-F-52** The console shall work with the keyboard alone and with a screen reader, shall honour the participant's setting for reduced motion, and shall never carry a state by colour alone. It shall keep the reader's place: it follows new output only while the reader is at the end. Traces: F-QUAL-3.

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

Bob picks the checked provider and gives it a judge in the
provider's page and enters the judge's key in the app, which keeps it; its
plan named the judge's host. A web page the agent reads
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

### AGT-J6: Watching a session in the console

Alice opens the agent kernel's page. She starts a session with "Tidy" on
ChatGPT; the console lists the fields ChatGPT cannot honour and keeps Start
off, so she picks Claude Code instead. She asks it to merge two book lists.
The thinking streams, then folds; three reads fold into one line; a plan
appears above the input. The edit to `reading-list.md` asks, with its diff
and the time left. While it waits she types a second message, which goes to
the queue. She allows the edit for this session; the header shows that
allow, and she takes it back. She scrolls up to reread; new output does not
pull her down, and a pointer offers to jump back. She reloads the page: the
same transcript shows, and the turn's state is the kernel's.

## 5. Later

- Workspace instructions (`AGENTS.md`) as a later opt-in profile field (N7,
  decided; `TECHNICAL.md` section 9.2).
- A worktree workspace for agents working in parallel (the factory).
- A wait that the host holds for agents that drive sessions over MCP.
- Codex as a third backend (N5, open until stage F).
- File undo.
- In the console, past its first target: a board of many sessions and their
  children; one review of all edits of a turn; fork and rewind; a message
  delivered after the current tool call; notifications; suggested titles;
  the profile and offer pages in the console's style; a chat part other mods
  can embed.
- **N11 (decided, the human, 2026-11-20):** the console shows only its own
  sessions; F-AGENT-6 stays as it is. Showing other clients' sessions, read
  only, is not planned.

Not planned: the `auto` permission mode (N2, decided: it lets a classifier
watane does not see approve tool calls).
