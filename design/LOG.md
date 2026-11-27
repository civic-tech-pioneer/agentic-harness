# Spec log

A record (`AGENTS.md`, "Contracts and records"): why the contracts changed, newest first. The
project has not run `spec-adopt` yet, so its config has only the older `spec = [...]` list and no
`spec-lint`; entries cite sections, not ids.

## 2026-11-27: S151, factory profiles (stage F design)

The sketch (Deputy Paper): settle who writes a client mod's role profiles and its
`access/<identity>`, with whose consent; absent entries stay denied; a requesting mod never
provisions itself. N5 answered (a) by the human (PLAN, Human answers, 2026-11-27).

- FUNCTIONAL: term "profile access"; AGT-F-1 and AGT-F-2 extended; AGT-F-56 (the participant
  allows one version of one profile for one client, confirmed in the app, words named) and
  AGT-F-57 (shown and withdrawn in the kernel's page; withdrawal ends running turns); J11
  names the app's question; new journey AGT-J7 (the factory gets its profiles).
- TECHNICAL: new 9.10 (one writer, `manage`, callable only by `page`; `clients` listed by a
  client's own call; pinned entries `{profile, digest}`; the question through a proposed host
  request `agent-access.ask`; the reach text; withdrawal in batches of 16; profiles and their
  index; offers by collect with no source, adoption and stale). 2.1, 2.3, 8, 9.1, 9.3, 9.7
  "Revoking", 12, 14 (new row, the console row and "Not protected"), 15 (three watane rows,
  proposed, not accepted), 16 cases 78–85, 17 (stage F row, slices F-P1 to F-P3), 18.1.
- The adversary on the design (BREACHED 5, all spec gaps) fixed in the same slice: every read
  of a session's profile compares its digest, so no session runs an unallowed version; the
  reach names who answers asks, the programs' network, the changed fields, shows everything
  or asks nothing (4 KiB), escapes non-ASCII; `withdraw` ends children's owner-given turns;
  the host's page-chain rule tightened; the loopback route and the pkg import named.
- N5 recorded as answered in VISION, FUNCTIONAL, TECHNICAL and AGENTS.md.
- Open for the human: the consent-boundary changes and the proposed watane rows (the PR body
  lists them, with the Door C choices for the deputy).
- Review r1 (Codex, 9 blocking): the page identity holds no power over profiles. `manage`
  works only on a lease the participant opens in the app (`agent-access.ask(manage)`), bound
  to the page connection (`caller.connection()`, proposed), and each create, edit, restore,
  adoption and deletion is a `store` or `remove` question applied on the host's answer; no
  VISION change (the deputy's ruling). Also: the host ends an open `allow` question when the
  client's installation changes, and identity reuse after the answer is a stated limit;
  withdrawal completes on the kernel's own timer; per-version field digests for the changed
  fields; capacity counts open questions and questions end at a restart; three kinds of
  work after a change (new input, launches and reads, compiled work) and byte-identical
  restore revives on purpose; offers: changed, gone, unchanged, and AGT-J7 reordered; manager
  responses metadata only, 256 KiB; 17 separates AGT-J7 on the fakes from JF1-2's real start
  and its waits (shell provider path and grant, V27).
- The adversary on the r1 design (BREACHED 3): a loopback client could hold the one `manage`
  question slot (now the latest caller wins, with a pairing code, and `manage.close`); a reused
  `local/<slug>` identity against AGT-F-56 (AGT-F-56 now says a client is its watane identity,
  the limit in 14); a later `allow` cancelled a withdrawal (now a session-id cutoff). Its risks
  closed in the text: connection ids made and bound by the host, R1 and `ended` deletions
  named in "One writer", delegates and skills, duplicate keys, offer order and crowding, the
  same-origin console in "Not protected".

## 2026-11-26: page timer origin under D001

The human: "a page-owned session's timer firing carries its owner's authority when it reads the kernel's own installed identity (its origin client, TECHNICAL 8); every other owner stays strict, and a non-page owner's fallback to the kernel identity is still refused."

- AGT-T-138 and TECHNICAL 2.1 use the page's host-reported origin client (must, said). An authenticated page call obtains that identity through a reserved timer, an implementation detail inferred from the existing host contract.
- AGT-F-24's retry promise applies to the page's existing consent principal.
- Decision record: `factory/decisions/D001-page-timer-authority.md` on main.

## 2026-11-26: enforce session owner authority on timer firings

The human: "the kernel acts on a timer firing only under its session owner's authority (consumer half of watane S133/S136; F6.1)."

- AGT-T-138 added (must, said). TECHNICAL 9.5 compares Watane's resolved firing origin against the session's bound owner and defines local cleanup on mismatch. This closes the F6.1 known limit.
- TECHNICAL 2.1 and 15 copy the Watane S133 consumer contract. 9.3, 14 and case 22 reference that contract or its enforcement.
- The bound identity is the session's immediate caller identity (S133 P1.5). A page's owner identity remains `page`. Watane resolves page-origin firings to the installed mod identity, so the owner check takes the local cleanup path.
- Spec-check compared the owner check with AGT-F-26 and AGT-F-28. AGT-F-24's unconditional retry wording is in tension with the owner check. Its wording needs the product owner's decision about page authority.
- Deferred: install-generation binding (S133 P1.12) and recovery of an idempotent job under its original key (case 22).

## 2026-11-26: S134 r3, answers (build team, Claude)

No contract changed. The deputy's ruling on F3.1 recorded: if round 4 finds the live-update
mechanism of `Work.stopped` again, it becomes a known limit (the stored state is right and a
reattach shows it), not another round.

## 2026-11-26: S134 r2, answers (build team, Claude)

- TECHNICAL 16 "Budget record": the page-level stage F budgets are "not measured (needs
  watane-dev)", a known limit, by the deputy's ruling on r1 F1.9 (was "stage F is not done until
  they are measured"); the 10% history tolerance is also met by the p95 series.
- Deputy's rulings recorded: F1.4 on the build team's reading (toolsets by name), F1.5 OpenAI
  `persist: false` a known limit for root and child alike (11).

## 2026-11-26: S134 r1, answers to the Codex review (build team, Claude)

Who: the S134 build team, answering review r1 (PR #60). No `must`, vision statement or security
check weakened; each change narrows or states a gap.
- TECHNICAL 9.7, the collision rule: only an *active* built-in collides, on every request
  (`task` in a root that names delegates; `skill` once built). Was "`task` or `skill`" (F1.7).
- TECHNICAL 9.7, the `toolsets` row: names only; a toolset has no adopted revision in the
  kernel, the host's checks decide what the child's own handle reaches (F1.4, a clarification).
- TECHNICAL 11 `persist`, and the 9.7 `persist` row: OpenAI `persist: false` is not built for
  any OpenAI session, a known limit (F1.5, found by the review; Upkeep).
- TECHNICAL 16 case 24 and the Budget record: a root's limit stop interrupts its live child;
  p95 per step; stage F not done until the page-level budgets are measured (F1.1, F1.9).

## 2026-11-26: S134, stage F terms and S132's polish (build team, Claude)

Who: the S134 build team, settling the deputy's spec-check findings on the sketch (PLAN, S134:
"the build team settles the first four with spec-change") and S132's review polish P1.1 to P1.12
(PR #59, "Review r1 by Claude"). No vision sentence, `must`, or security check changed; no
statement weakened. S132's nine Door C choices stand.

Statements:
- FUNCTIONAL 1, new terms **live child** (decided, said by the spec-check: `subagent-limits`
  names OpenAI children while FUNCTIONAL keeps "subagent" for Claude) and **model step**
  (decided, same source: `max-turns` counts OpenAI model steps, not turns). The field names
  stay (`subagent-limits`, `max-turns`: profile formats of 4.1); the terms say what they count.
  TECHNICAL 9.2 and 9.7 cite them.
- FUNCTIONAL AGT-F-36: "consecutive failed tool calls" becomes "consecutive failed or denied
  tool calls" (inferred from TECHNICAL 9.7, which already counted denials; the spec-check's
  unsure item 1), and a new sentence: a token cap with missing counts fails the turn and names
  it (inferred from TECHNICAL 9.7 `usage-unavailable`; unsure item 2). "native model-turn
  limit" becomes "native model-step limit" (the rename above).
- TECHNICAL 9.7 Admission: the order of the five checks and that the comparison step returns
  every refusal at once (P1.11); rules compare as written, globs as globs, no `describe`
  (P1.4); a `persist` row: `false` when the root's is (P1.3, a narrowing, fails closed).
- TECHNICAL 9.7: the live cap counts live children; a child the owner continues after its call
  resolved is the owner's work, intended (P1.2).
- TECHNICAL 9.7 terminal outputs: codes for a child request stopped `busy` (`busy`) and an owner
  interrupt of the child's first turn (`interrupted`); `aborted` only when the parent's call
  itself is aborted (P1.5).
- TECHNICAL 9.7: parent `interrupt` interrupts live children; parent `cancel` or `delete`
  cancels them; a child is never deleted with its parent (P1.1).
- TECHNICAL 9.7 Limits: a final text answer that crosses `max-tokens` ends `stopped`; the root's
  cap excludes its children's tokens, with the bound (P1.9); what counts as success and failure
  in the tool-error streak.
- TECHNICAL 9.7: Claude's `max-tokens`, `max-tool-errors` "refused" (11), not "rejects in stage
  F" (P1.11); `subtype: "error_max_turns"` named; a known limit for the unprobed
  `model_usage_since_last_result` (P1.6, fails safe).
- TECHNICAL 9.3 R4: the paused-child row holds for every child, intended (P1.8).
- TECHNICAL 17: how the stage F fixture keeps each child to one request slot (P1.7).
- TECHNICAL 16 cases 23 and 24: `usage-unavailable`, a denial in the streak, a child interrupt,
  cancel and delete, and a `task`/`skill` toolset collision (P1.10).
- Wording: a blank line before "The function and its lifetime", a straight apostrophe in 14
  (P1.11).

Not changed: P1.12 (`factory/spec-check-S132.md` is the deputy's record).
