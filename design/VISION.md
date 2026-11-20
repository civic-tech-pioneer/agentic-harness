# The agent layer: vision (draft)

Draft, not yet a contract. It becomes one when a design slice takes up
`ARCHITECTURE-PROPOSAL.md` (revision 4) and the human settles its open points
(section 15 there). The layer lives in this mod, `agentic-harness`; its small
host part lands in watane, whose `spec/VISION.md`, `spec/FUNCTIONAL.md` and
`spec/TECHNICAL.md` then gain the rows that `TECHNICAL.md` section 15 lists.
Paths such as `spec/…`, `host/…` and `factory/…` in these files are watane's;
`design/`, `proto/` and `research_notes/` are this mod's.

## Why

Agents bring the cost of software close to zero (`spec/VISION.md`). Today
they stay outside watane: a maker's coding agent writes a mod, and the mod
itself cannot ask an agent for anything. The factory (step 6), the set of
mods that steers coding agents and changes itself, cannot run inside the host
until mods can start agents, talk to them, give them tools and stop them.

Agents are also the strongest authority a mod could ask for. An agent acts
with the participant's rights, reads text an attacker may have written, and
spends the participant's plan. So the layer must keep watane's promise that a
stranger's mod is safe to run (V5): every boundary stays with the host, every
reach is shown and can be withdrawn, and whatever the host cannot check fails
closed.

The agent layer lets a mod run agent sessions on the participant's own
subscriptions, Claude and ChatGPT, with no API key, no watane account and no
operator. Almost all of it is mods: a small host part owns the boundaries,
and the rest can be downloaded, replaced and built on like any other mod.

## Goals served (`spec/VISION.md`)

| Goal | How the agent layer serves it |
|---|---|
| V1 Own environment | The participant decides which agents a mod may run, in which folders, with which tools, and which profiles exist. A mod can make agents part of the person's own tool. |
| V2 Stigmergy | The kernel, the providers, profiles and skills are mods and offers that others can find, run and derive from. |
| V3 Cheap making | A mod gets agent sessions through one interface, for both vendors, without writing a client, a tool loop or a permission system. |
| V4 No center | Sessions run on the participant's own Claude and ChatGPT plans. No watane service, key or account sits between. |
| V5 Safe by default | The host owns the process, the credentials, the folders, the stdin of `claude` and the consent. A session is private to the client that started it. Limits live in code. A failing check denies. |
| V6 Chosen trust | The participant picks the provider: Claude Code unchecked, or a provider that asks a judge they chose before each step. |
| Step 6 The factory | Sessions, delegation, tools, limits and resume are what the factory needs to run as mods. |

## Boundaries: what the agent layer must never become

- **Never a model client in the host.** The host starts `claude`, sends HTTPS
  requests and keeps time. It never builds prompts, runs a tool loop or reads
  a model's answer for meaning. That is the kernel's work, in the sandbox.
- **Never a holder of the Claude login.** `claude` keeps its own login. The
  host passes the participant's token through and never reads it.
- **Never a credential in a mod.** Plan tokens and MCP tokens stay in the app
  and the host; a mod sees ids and handles.
- **Never a second way around consent.** Folders, accounts and toolsets are
  consented per origin client. A mod in the middle gains nothing.
- **Never an agent that approves.** No agent installs a mod, approves a grant
  or signs a statement: the confirmation floor holds (F-AGENT-4).
- **Never a modified or impersonated vendor client.** Claude runs as the
  unmodified binary; ChatGPT through watane's own registered client. No
  subscription token goes anywhere its vendor does not allow.
- **Never a judge as a boundary.** A checking provider lowers risk. The
  boundaries are the host's checks, the sandbox of `claude` and the consent.
- **Never waiting inside a call.** Every call returns within its budget. Long
  work is a stream, a job or a timer.
- **Never one vendor's protocol as the client's.** Clients see one event
  format for every backend.

## Success

- A client mod starts a session with one profile on Claude and on ChatGPT,
  gets the same event format from both, answers an ask, and resumes after a
  restart of watane.
- A hostile mod cannot widen a session past its profile, its consents and its
  grant: every probe of `ARCHITECTURE-PROPOSAL.md` section 18 holds on the
  real programs, and each breach found later becomes a regression test.
- With the checked provider, a tool call that the judge calls unsafe does not
  run, and a judge that is slow or down denies.
- The participant sees, in the app, every folder, account and toolset a mod's
  agents can reach, and withdraws each one.
- The factory's roles (product owner, foreman, validator) run as sessions of
  mods in the host (N5 decides the implementer).
