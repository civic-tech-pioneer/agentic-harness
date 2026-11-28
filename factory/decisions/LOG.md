# Deputy decisions before 2026-11-28

Moved from `factory/PLAN.md` on 2026-11-28. Each one past its veto window is to be written into the spec that owns its fact (deputy skill, section 0); tracked by #122.


- 2026-11-27, Deputy Scissors, S153 BLOCKED: a Codex build team declines the adversary's exploit-development step. The build team writes defensive boundary checks and guard tests instead, and the Claude review team runs the adversary agent on the PR in round 1, so the check moves, it is not dropped (foreman section 3; TECHNICAL 14's boundaries unchanged). Undo: have the build team run it again. Friction: the factory's adversary step assumes a builder that writes exploits; propose to the human that a Codex build hands it to the Claude side by rule.

- 2026-11-26 (Deputy Scissors, rung 2, Door C, for the human's veto) S138 F1.1: a console (page-owned) session's timer firing carries its owner's authority when it reads the kernel's own installed identity, its origin client (TECHNICAL 8); every other owner stays strict, so F6.1 stays closed. Codex and Claude agreed blind. Record: factory/decisions/D001-page-timer-authority.md. Undo: one comparison and two sentences in 9.5.

- 2026-11-26 (Deputy Scissors) S134 F1.5 accepted as a known limit: OpenAI `persist: false` is not built for any session (root and child alike); no VISION or FUNCTIONAL clause promises it; recorded in TECHNICAL 11. Undo: an upkeep slice that builds it. S134 F1.4 settled on the builder's reading (toolsets carry no adopted revision; host consent decides).

- 2026-11-26 (deputy-7) S127 cut at r2: F2.1 accepted (the denial and cancellation stay immediate and every §14 row HELD; only the reason text can be lost when the kernel-facing stream is full at a launch deadline or Claude ends before ready; two unusual events together). Clause: the stall rule (findings in code the last answer added). undo: an upkeep item, drain the outbox before close.

- 2026-11-25 (deputy-6, S127, **Door C, for the human's veto**) judge wire shape: Jev (the human's choice) speaks TypeSafe
  System One, and no operator may sit between (VISION), so the checked provider's judge call is `POST
  <origin>/v1/systemone` with `noul` questions in one request; it maps probabilities to `safe | unsafe | unsure` by
  thresholds on its own page (default 0.7/0.3, the docs' starting values) and writes the reasons itself. Origin: entered
  in the app only (TECHNICAL 5 wins over 10's "judge URL" setting). `[http]`: `api.typesafe.ai`; "a judge they chose"
  (VISION V6) holds by choosing or deriving a provider, so no VISION edit. Clauses: Human answers 2026-11-25 (2), (4);
  TECHNICAL 5, 10; dev TECHNICAL 17 item 17 (the provider fixes its shape and scripts `Raw`). Against: TECHNICAL 10's
  `{safe|unsafe|unsure, reasons}` judge answer, fixed in S127. Undo: a second request shape in the provider (an adapter
  per judge API), before any participant relies on it.

- 2026-11-25 (deputy-4, S40 r4) F4.3 accepted as a known limit: a message finished in the window between its transcript write and `response.completed` when the host restarts is shown but missing from the model's history; Alice repeats it. Clause: rulings narrow; same kind as the accepted no-auto-continue limit (S40-CUT). Undo: write the finished message to request history with the transcript, a later slice.

- 2026-11-24 (S40 r2) after a host restart every open job of an OpenAI session in `tools` is aborted, idempotent ones included; the kernel reopens no toolset and reissues no job (TECHNICAL 9.5 and the restart table say so; case 22's idempotent half not yet built). Clause: rulings narrow (stall by area: two blocking findings in code r1's answer added, one a breach). Undo: reissue under the session owner's origin once the handle is usable, a later slice.

- 2026-11-24 (S39 r1) accepted: A1.1 a provider release built other than by `publish-test-mods.sh` trusts `local/agent` (needs a hand-published provider and a local mod with the slug `agent`; the four-line pin test closes the script path); A1.2 the script checks the channel's publisher only after signing the provider (operator-only). Whether a release build must refuse the dev default is a TECHNICAL 2.2/10 question for later.

- 2026-11-24 (S38 r2) F2.1 replaces the r1 ruling on F1.2: TECHNICAL 8's two 'Kept for' cells cite watane host 19.7 and name Q38 as open, restating no key; the doubles use 19.7's `workspace:<folder>` as the main case. Clause: rulings narrow to the spec's literal words. Undo: one cell each.
- 2026-11-24 (S38 r2) accepted: R2.2 (a late `consent-pending` needs a forged wrapped line and a disagreeing host check: two unusual events); Q2.1 (Stop during a consent wait leaves the app's question open; watane 19.3 does not say what `interrupt` does before `initialize`) and Q2.2 (forbidden path refused at call or at apply) go to watane S103's review.
- (superseded by S38 r2) 2026-11-24 (S38 r1) F1.2: watane Q38 (consent key with `via`) is the human's and open; S38 keeps option (b) in TECHNICAL 8 and the doubles, marked pending, as the reversible option ((b) to (a) keeps every consent valid; host 18 Q38). Clause: the deliberation ladder's 'build the option that keeps the others open'. Undo: drop `via` from two rows and the doubles.
- 2026-11-24 (S38 r1) R1.1 accepted: a withdrawn consent reads as 'Connection to Claude Code ended'; host 19.7/19.2 end it with a plain `gone`, and AGT-J1 / AGT-F-29 ask only that the stream end. A distinct host ending would be a watane interface change (Door C), not planned. Undo: a `consent-withdrawn(what)` ending in watane and one console line.

- 2026-11-23 (deputy-2, S36 F3.1, the same finding a third time) Take back first, replacing the F2.1 rule below, which let the original and its replacement both run: Edit, Stop and send, and remove call `unqueue(n)` first (the kernel's atomic take, TECHNICAL 9.3); `unknown-input` leaves everything alone; a later refused `say` or `interrupt` puts the text in the composer (18.6 Send). Clauses: AGT-F-41 "take one out before a turn takes it", AGT-F-50 "take a queued input back to edit it"; none against. Cost: a refused re-send loses its queue place, and its text lives in the composer under that rule's reload limit. Not deliberated on the PR: the clause decides it (Kind rule, Door none). undo: rewrite 18.6 Queue and the console's Dock again. PR #50 comment 7114381778.
- (withdrawn by the line above) 2026-11-23 (S36 F2.1, r1's F1.5 back a second time) The console's queue keeps an item until its transfer is accepted: Edit, Undo and Stop and send never remove a queued item before the replacement say (or interrupt and say) succeeds, so a refusal changes nothing and needs no recovery state; a newer draft is left alone.
- 2026-11-23 (S35, the composer's kept draft, F1.4 to F4.3): no spec asks for a draft that survives a reload while its send is pending; each fix opened the next race. Rule: the composer keeps unsent typing per session across a reload; Send removes the text from composer and kept draft at once; a refusal while the sending composer is mounted puts it back (newer typing wins); a refusal after a reload or unmount is lost from the composer (known limit in TECHNICAL 18). PR #43 comment 7088686568.

- 2026-11-23 (watane S100, for S36): the host gives the resolved path (9.6's open point) as a top-level `"path"` on a file-tool `ask`/`check` item (absolute, symlinks resolved; `input` as `claude` sent it; absent when unresolvable or refused). S36 writes it into 4.5's item table and has the kernel offer a file key only from `path`. A request before `system/init` passed is denied by the host and pushes nothing.

- 2026-11-22 (S35): stage B cases 9 (asks), 10 (hooks) and 11 `compact` move to S36; case 12's depth-0 half (removing Task) moves to stage F (9.7, delegation). S35's continuation named them as not covered; none is a single-turn case S35 needs.

- 2026-11-22 (deputy): R-B2 settled without the human: `codex_env = "watane"` is in `factory/config.toml` since the factory sync (PR #45), and the `watane` Codex environment holds this repository. Removed from Waiting.

- 2026-11-22 S35's user-sim round-2 rejection (an attach mid-message shows Working, not Writing, until the
  entry arrives) is accepted as a polish limit (9.4): setting `synced.writing` costs a state write per delta-only
  call, which case 56 and P09 rule out. Its red test stays `#[ignore]` with that reason.
- 2026-11-22 R-B3 (S35): the provider compares its caller with `local/agent` while the kernel is unpublished;
  the publisher's name replaces it when the kernel is published (one 2.2 line).
- 2026-11-22 (the human's answer) Auto-merge is retired: the `codex-push` ruleset (27841001: only
  `slice/*` without bypass, for the Codex push App) blocks it, so the PO merges each green PR with
  `gh pr merge <n> --admin --rebase`. Foremen open their PR and do not arm it.
- 2026-11-21 Stage B is two end-to-end slices (S35 one turn, S36 steering, restart, budgets), each with
  a watane half (S99, S100); S35 builds the unchecked provider in `provider/` as a self-contained mod
  until R-B1 is answered.
- 2026-11-20 S2's user-sim rejection is accepted: the failing outcome is a gap of the proposal,
  now named as a precondition of stage B (TECHNICAL 4.6), not a defect of the slice.
- 2026-11-20 Adversary RISK 3 of S2: the checked provider's first judge call waits until the
  workspace consent is held (option b; one TECHNICAL 10 line to undo). Written in by S5.

- 2026-11-20 The performance review (68a27f3, another session, on the human's ask) becomes
  slices S5 (must-dos P01-P06, P13, P15), S6 (P07-P12), S7 (P14, console). Its record and the
  AGT-F-8 edit reach main with the PO's next push.

- 2026-11-20 The human is away overnight (standing orders from the session that started this
  PO): no questions to the human until morning; the PO decides and records here.
- 2026-11-20 The kernel's pure logic is a plain Rust crate in `kernel/`, tested natively, with no
  WIT until watane has the 2.1 additions; the WASM component wraps it later (keeps R1 and the
  host interfaces open).
- 2026-11-20 One gate script `factory/gate.sh` (S3) runs every part that exists: kernel, console,
  mod contract. Later slices add no gate lines.
- 2026-11-20 The console is built against a scripted fake kernel feed of ACP v1 plus
  `_meta.watane` (N6), so it needs no host.
- 2026-11-20 S1 runs locally: its outcomes need the Mac's logged-in `claude`.

- 2026-11-20 S3's spec debts (real kernel file names in TECHNICAL 1 and 14; tools, ask-expiry,
  overlay keys, frontmatter, Bash allow grammar) go into S6, which owns `design/` next; the code
  follow-ups (drop `yarn ls`) go to the next kernel slice after S8.
- 2026-11-20 S3's foreman refused any `*` (and `:*`) in an allow rule for a package runner:
  fail-closed, so `Bash(npm test:*)` is refused. Kept; the wider question is under Waiting.
- 2026-11-20 Next kernel slice is S8, the adapters of 9.8: pure functions, disjoint from S4
  (console) and S6 (design), needed by stage B.

- 2026-11-20 S4 is accepted as behaviour; its look goes back as S9 (the frames give layout and
  words, TECHNICAL 18.4). S7 waits for S9 (same files). S4's wire assumptions and spec notes
  become S10 after S6 (same files). Its decisions stand meanwhile: decision JSON
  `{"allow":true[,"for":"session"]}` / `{"deny":true,"reason"}`; acts with no 2.3 call shown off.

- 2026-11-20 S8 BLOCKED on 4.5 (ask/check items lack the call id; a `claude` stdout line typed
  like a host item would pass as an ask; `expires-at` on the kernel's clock). Decided: merge S8 as
  built (it reads `tool_use_id` when present, null otherwise, never guessed), and write the three
  rules into 4.5 in S10 (option 1). Also under Waiting: it is a wire rule of the watane host.

- 2026-11-21 S6 settled the Bash boundary as option (a) of Waiting (TECHNICAL 14 "Not protected":
  no refused-word list is complete; a missed word runs only inside the shell grant's sandbox, for
  a profile the participant made or adopted). Kept; the human may still choose (b).
- 2026-11-21 S6 open RISKs: kept keys and timers per owner are accepted limits named in 14 (same
  pattern as slots); finding running sessions at restart without a prefix scan goes to stage B's
  first kernel slice; the session allow on `Bash` is under Waiting.
- 2026-11-21 Next kernel slice after S11 is S12: S6's code to change (runner names, `yarn ls`,
  frontmatter `:`), then `expires-at` once S10 writes it.

- 2026-11-21 S11 BLOCKED on 9.4: "read with an after inside a group starts at its first part"
  loops forever on entries over a page (cases 34, 49). Decided (a), as built: only `attach` moves
  `after` to the group's start; `read` takes `after` as given (a client that lost the start reads
  with `after = i - n`). Undo: one call, one test, one sentence; (c), a `restart` flag on `read`,
  can come on top. The sentence goes to S13; the RISKs to S12 and S13.

- 2026-11-21 S10's foreman decisions kept: keys ACP has keep its spelling (`toolCallId`), the
  kernel's own are kebab-case (`expires-at`); a decision is a later `ask` entry repeating the ask;
  an ask goes out live once, as its entry; the wrap list stays a blocklist (each new item type is
  added to 4.5), the envelope option still declined. S10's code lists become S14 (kernel) and S15
  (console); S12 was already out, so S14 follows it.

- 2026-11-21 S14's open points, decided fail-closed: an unknown `by` on `ask-resolved` is
  `unknown` and the ask stays open; a non-object `input` or a negative or fractional `expires-at`
  is `unknown`; a joined whole of kind `part` is `unreadable` (S19). Accepted limits, named in 14
  by S18: a first-key JSON reader would read a duplicate `i` differently (every reader here reads
  the last); a nested duplicate key in `input` is safe only while the host serializes the item.

- 2026-11-21 S19 consequence: with an unknown `by` read as `unknown`, every answered ask would
  add an `unknown` entry, since 4.5 names no `by` for a person's answer. Kept fail-closed; S18
  lists the `by` values, and S20 makes the answer ones give nothing.

- 2026-11-21 S15's open risks: (a) two entries on one `toolCallId` and (c) the ask's `toolCall`
  without `kind` or diff go to S20 (kernel and 9.8, fail-closed); (b) a part group over 256 parts
  read mid-group shows one unknown row that can only expire: accepted (fails safe; entries over
  64 MiB do not occur under the 256 KiB-per-part rule's caps).

- 2026-11-21 S22 RISK 1: open Claude asks and checks are capped at 64 per session; one past it
  is denied at once (`busy`), so every closing call writes at most 64 decisions (S20).

- 2026-11-21 S20: a `for: session` allow given before `access/` is revoked must not keep allowing
  calls: the slice that builds session-allow matching (the broker, 9.6) reads `access/` before
  answering from a session allow, failing closed. The host keeps checking `allow` + `input` (2.3).

- 2026-11-21 S25 RISKs: `delete` skips a failing restart step; at most one `turn` entry per turn
  past quota; the quota refusal stops naming the kernel's shared account (all to S26). Accepted:
  duplicate lines from a restart step that fails in a timer firing (the real host rolls a call
  back whole, 7; only `MemState` lacks it); no migration (nothing deployed).

- 2026-11-21 S26 RISKs: a provider `ended` during an active turn ends it `failed {provider-ended}`
  (S28); the past-quota flag is made unreadable without the accounts read (S28). Accepted: the
  transcript's last `turn` entry may trail the record past quota (attach and pushes carry the
  record's turn); decisions use the chunk cap's kept room (older); for a moment after `cancel` the
  host may run one more `claude` than the kernel counts (the host's own caps bound it).

- 2026-11-21 S28 RISKs: every call that ends a turn or a session's work (`cancel`, a turn ended
  `stopped {quota}`, `ended`) succeeds at the ceiling, charged into the reserve; past the ceiling a
  decision carries the ask's index, not its body (S29). Accepted: a deferred `ended` that the host's
  backstop ends `consumer-failed` (host H1, stage B); `ended` on an older-boot record (only if the
  host breaks R1).

- 2026-11-21 S29 RISK: the reserve is shared, the bound per call: about 45 owners crossing at once
  with full sets of open asks could pass the 32 MiB reserve, then the host's 256 MiB limit refuses
  the writes. Accepted as a named limit (9.3 states the bound). Also accepted: a 256-byte `reason`
  can escape to ~1.5 KiB; an adapter-made decided ask below quota is stored as given (no adapter
  makes one; build it from the record when one does).

- 2026-11-21 The human trusted the agentic-harness folder (via the parent session); S1 goes to a
  local foreman.

- 2026-11-21 S31 RISKs: no session allow for `Task`, `Skill`, `WebFetch`, `WebSearch`; a Bash
  reach pins `run_in_background` and `timeout`; the host sets `allowUnsandboxedCommands: false`
  explicitly (all S33; also under Waiting, as a security boundary). Accepted: `allows` changed shape
  with no migration (nothing deployed).

- 2026-11-21 The human: slices were cut by layer (kernel-only, console-only, spec-only), not as
  stories across components. Corrected: every slice is a story a person can see, crossing watane's
  host, the kernel mod and the console thinly; spec lines and risk fixes go into the slice that
  builds the feature. S30 (console-only) stopped and folded into S32; S34 (the human's scope-key
  rule, kernel and spec) finishes. Stage A is one story in two repositories (watane S96 with S32).

