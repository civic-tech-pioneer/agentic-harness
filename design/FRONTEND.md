# The agent frontend: design brief (draft, 2026-11-20)

A record, not a contract. A client page for the agent layer (`FUNCTIONAL.md`,
`TECHNICAL.md`), built from two comparisons:
`COMPARISON-harness-ux-2026-11-20.md` (what good harness UIs do) and
`COMPARISON-agent-ui-libraries-2026-11-20.md` (what to build it with). The
mock-ups are on the design canvas "watane agent frontend".

## Principles

1. **One honest status line.** Driven by the turn state (TECHNICAL 9.3): a
   word, a shape, the elapsed time, and the key that acts on it. `unknown` and
   `resume-lost` say plainly that nothing is running.
2. **Asks live in their tool card.** The card stays open with every argument
   and the diff; "Needs you (n)" stays above the composer until it is
   answered. Allow once, allow for this session, deny with a reason that goes
   back to the agent.
3. **Every decision stays in the transcript** with who made it: you, the
   profile, the judge, expiry, or cancel (`ask-resolved {by}`, `verdict`).
4. **Quiet by default, loud when it matters.** Consecutive reads and searches
   fold into one line; edits, shell runs, asks and failures get full cards;
   a failed call always opens.
5. **Thinking is open while it streams, then folds** to "Thought for 4 s".
   The plan is pinned above the composer.
6. **You can always act.** Input while busy goes to a visible queue (n of 32)
   that can be edited and removed; "Stop and send" is the explicit interrupt;
   Stop shows "Stopping…" until idle is confirmed.
7. **Pauses explain themselves.** A usage limit shows its reset time and
   resumes on its own; a lost session offers to continue from a summary.
8. **The session list ranks by need:** Needs you, Running, then Recent.
   Notifications only when the session is not in view.
9. **The page shows its reach.** The header names the profile, the provider
   (unchecked or checked), the workspace and the plan in use; each opens the
   consent it rests on.

## Layout

- **Sidebar** (240 px, collapses on a phone): new session, sessions grouped
  by need, each with its client mod's monogram, title, state glyph and age.
- **Header:** title (renames in place), profile chip, provider chip,
  workspace chip, context meter, and a menu (fork here, rewind, rename,
  delete).
- **Transcript:** user turns as quiet panels; agent text full width; tool
  lines and cards; decision lines; a turn footer ("Completed in 42 s · 4
  tools · 18k tokens").
- **Composer dock:** "Needs you" pointer, pinned plan, queue, status line,
  then the input with model and mode shown read-only (they belong to the
  profile) and Send or Stop.

## Building it

React and assistant-ui, bundled into each mod's `ui/` (decided with the human,
2026-11-20; it replaces the framework-free kit the library comparison first
recommended). A bundled app fits host 8.3: same-origin scripts, no `eval`, no
WebAssembly, no iframes. The cost is a copy of React and the kit in every mod
(a few hundred KB), accepted for now.
- **State:** assistant-ui's ACP runtime, with the adapter extended for `ask`,
  `ask-resolved`, `verdict`, `turn`, `synced` and entry indexes.
- **Parts:** assistant-ui's components, styled in the manager's visual
  language (Inter, warm neutrals, `#3355D6`, 9 px radii, light and dark);
  AI Elements (Apache-2.0) as the reference for Tool, Confirmation, Reasoning,
  Queue, Prompt Input and Terminal.
- **Rendering:** Shiki with its JavaScript regex engine; ANSI with anser
  (escaped); a diff view without `eval`.
- **One package:** the kit ships as one versioned npm package (for example
  `@watane/agent-ui`), not code copied into each mod.
- **Pinned and hashed:** every dependency is pinned, and the build records a
  hash per file it emits.

### How the code reaches a page (open, decide later)

1. **Bundled in the mod** (the default, built first). The host serves `ui/` as
   it does today.
2. **Trusted sources in the manifest.** `mod.toml` names a source with a
   pinned version and a hash; the install plan shows it ("loads code from
   …"), the participant approves it like a grant, and the host adds it to that
   mod's page policy. The plan must show the costs: an outside party learns
   when the mod opens, and the page needs the network.
3. **Build definitions, built locally.** The package carries sources and a
   lockfile; the participant's machine builds `ui/`. Needs Node and npm there,
   and runs install scripts.

What keeps all three open: `ui/` stays the only contract between a mod and the
host; the kit is one named, versioned package; versions are pinned and files
hashed from the start.

## What the UI needs that the spec does not give yet

From the UX comparison; each would be a change to `TECHNICAL.md`:
1. A live `turn` update on the stream, not only a stored entry.
2. The start of a message that is still being written when a page attaches.
3. The queue as data: an event when a queued input is taken; list, edit,
   remove.
4. A soft steer: a message delivered after the current tool call.
5. Asks with their tool-call id, `expires-at`, and the diff for edits.
6. A "checking" state while the judge runs, and the breaker's count.
7. Adapter rows: Claude `TodoWrite` → ACP plan; Claude thinking → thought
   chunks.
8. A context-window meter, plan usage and Claude's reset time.
9. A "compacting" state and tokens after compaction.
10. A status feed across sessions: unread, parent in `session-info`, children
    linked to the parent's stream.
11. Kernel `fork` and `rewind` calls (the launch document already has both).
12. `consent-pending` while the app asks the participant.
13. A way for a page to ask the host for a notification.
14. A suggested title after the first turn.
