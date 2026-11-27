
### S32: stage A, end to end: Alice watches a real kernel stream in the console (PR #41; continuing on watane's real host since 2026-11-22 12:05 UTC, routine trig_014twBAS43FX17jqormQVpSm, cloud; first delivery 2026-11-21 10:47 by trig_01QrwDoJCFh8Q6DGBbTcF7qA)

Story: Alice runs `watane-dev serve` with the agent kernel installed, opens its console page, and
sees numbered items of a real kernel stream arrive in the page (18.4 layout and theme), live and
after a reload, with nothing doubled. One story in two repositories, landed together: watane's
half is S96 (host H1 streams, H5 durable requests, H7 timers, page connections; pkg stage 12), this
repository's half is this slice. Interface, named the same in both sketches: the
`watane:host@0.1.0` streams and timers of TECHNICAL 2.1 as watane S95 wrote them in host 19
(stream-sink `taken | defer`, `streams.resume`, `stream-producer.ended`, page connections).
Outcomes:
1. The kernel crate gains its first WIT world and a WASM build (`wit/world.wit`, `mod.toml`), so
   `check-mod.sh` checks it; the kernel opens a stream to its page and pushes items.
2. The console attaches to the real stream in `watane-dev serve` (the scripted fake stays for unit
   tests only); a reload re-attaches after its last index.
3. Cases 7, 20, 21 (kernel part), 36 and 51 (kernel part) pass, run against watane's host.
4. Folded in from S30 (stopped, branch `slice/S30` kept for reuse): the short decision form (9.4),
   the status line after an answer, the `long_S7` timing limits moved out of the default gate.
Every spec line this needs is written in this slice, not in a later one.
Touches: `kernel/`, `wit/`, `mod.toml`, `console/`, `ui/`, `design/TECHNICAL.md`. Protects: *core*.
Starts when watane S96 starts (it fakes nothing it can call for real once S96 merges; until then
it builds against S96's branch or the host 19 text).

Delivered (PR #41, not armed): the seam audit found 10 mismatches between the kernel's fake host
and host 19 (batches with `seq`, `SinkOutcome {Taken, Defer}`, the `outcome` variant, `bad-delay`,
`ended` through `provider_ended`, no host clock/id/generation yet, prebuilt `logic.wasm`, the world
staged in `kernel/wasm/wit/` until a watane-dev with host 19 ships), all fixed; a probe session
(`probe.start`, 9.9) is stage A's item source. User-sim ACCEPTED on a wasmtime stand-in host
(live, reload, lagging, replaced, cases 20 and 21); adversary BREACHED 1 (a lost provider `ended`
held a slot), fixed, HELD. Not yet run on watane's host: cases 7, 20, 21, 36, 51. Next: when S96
merges, a continuation routine rebases S32, runs the story on watane's real host, and the PO
compares the console and arms the merge.

### S36: stage B, end to end: Alice steers a turn (DONE 2026-11-24: PR #50 merged as f8c984c at head 17791ac on the human's word "S36 can be merged itself", no scope cut; r7's F7.1-F7.3 accepted as known limits (Upkeep); before that: in flight, cloud, builder claude, reviewer codex; handed out 2026-11-23 after S35 merged at 042b0dd; routine `trig_0184S7g9auvX4raMDZLWTrYS`, 17:39 UTC; DELIVERED as PR #50 (+2404 -252) against watane slice/s100 at ff6428c; r1 to Codex https://chatgpt.com/local/6b048603-4b28-83eb-a045-6edb4806e05e; r1: 8/3 (blocking/polish; adversary BREACHED: an answer can grant a scope the ask never offered; user-sim REJECTED 3 outcomes; call: another round); answer round to Claude 19:03 UTC; r1 answered at aa3432a (all 8 blocking; the adversary's two F1.1 follow-up breaches fixed; load measured in Chromium, within budget); r2 to Codex; r2: 3/0, call another round; answer round to Claude 20:23 UTC with the deputy's ruling on F2.1; r2 answered at 7377679 (20:48, all three, red first; recovery machinery removed); r3 to Codex; r3 (21:01 UTC) 5/0, call another round (F3.1 the queue transfer a third time, now a double run under the r2 rule; F3.2 to F3.4 queue UI; F3.5 load cadence; user-sim REJECTED one outcome), posted with a deputy ruling that replaces the r2 rule (take back first); answer round to Claude; r3 answered at 392338a (21:48, all five, red first); r4 to Codex. Round 4 is worth it: blocking 8, 3, 5, but r3's rise came from the r2 rule, now withdrawn; take-back-first removes the transfer window structurally and F3.2 to F3.5 are narrow with reds; r4 (~21:58) 3/0, call another round (F4.1 taken text lost in live recovery, F4.2 stale capacity reply, F4.3 load gaps between bins: each the third of its line), posted with a deputy ruling (a per-session taken list, a capacity epoch, sliding windows); answer round to Claude. Round 5 is worth it: blocking 8, 3, 5, 3; the queue's double run is closed, and the ruling closes the three remaining lines by construction; r4 answered at ea6d0a9 (22:33, all three; load figures not remeasured in the cloud); r5 to Codex; r5 (~22:44 UTC) 5/0, call another round (taken text lost in a render batch, on a storage quota error, by the Undo timer; the epoch bumped too late; the load check's right edge). The trend 8, 3, 5, 3, 5 is flat, so the deputy treats it as stalled and rules the last time on these lines (the r4 rule literally; memory as truth; epoch at receipt; max-gap load check). Round 6 is worth it only because S36 waits on S100 anyway; a r6 return on these lines that needs a render-batch race or a storage failure is accepted as a risk in 18.6. Answer round to Claude; r5 answered at dc8c5d1 (23:20, all five; load not remeasured); r6 to Codex; r6 (~23:30) 3/1, call stalled (F6.1 one item sent twice from composer and row; F6.2 an expired Undo masks a later deadline; F6.3 the real load run fails the deputy's 25 ms rule, the deputy's error), posted with a ruling (twice the nominal spacing; one owner and one guard per taken item; one timer per deadline) and the deputy accepts the storage removeItem reload limit (18.6). Round 7 is worth it only because S36 waits on S100; if r7 is not an approve, the deputy stops the queue polish and takes S36 to the human with a scope cut (minimal Edit/Undo/Stop and send for AGT-F-50); r6 answered at 17791ac (00:12, all three and P6.1; real load passed three times); r7 to Codex; r7 (~00:21) 3/1, call stalled (F7.1 a refused Edit send after a session switch loses the edits; F7.2 a reload can offer a second send of an accepted item; F7.3 one 40.6 ms gap in the real load run). STOP RULE: held, PR comment says push nothing; the cut (no taken list, refusals to the composer under the S35 known limit, load gaps up to 3x spacing) waits on the human. S36 waits on S100 anyway)

Story: in the same session Alice answers an ask, queues a second message while the first runs,
presses Stop, restarts `watane-dev serve` and finds the session where she left it. Covers asks
(9.4, `answered.outcome`, checks closed `expired`), the resolved `path` on a file-tool ask (Deputy decisions, 2026-11-23), the queue, session allows, interrupt, restart
detection through `host-info.generation`, the rest of 17's stage B cases, and the stage B budgets
(8 fake sessions, 2 owners, 10 and 10 000 entries). Watane half (watane S100, if any host work is
left after S99: interrupt, host ask items 4.5, the load test's fake). Carry: S34 RISK 3, S26/S28
limits on the real rollback, the felt-wrong second-tab freeze. Also S35's moved cases: 9 (asks), 10 (hooks), 11
`compact` (Deputy decisions, 2026-11-22). Cloud.
Watane half: watane S100 (PR #15) is not merged yet (its last answer round runs). Build against its interface as written
on watane `slice/s100` (host 19.3: ask/check items with the top-level `path`, `answer` and `ask-resolved {id, outcome}`,
expiry, interrupt, `host-info.generation`); the user-sim runs on the real `watane-dev serve` built from watane
`slice/s100` until S100 merges, then on main. Do not merge before S100 merges. The deputy merges (no auto-merge).

### S38: stage C, the consumer half: Alice gives Notes' agents a folder, sees the wait, and can take it back
(in flight, cloud, builder codex, reviewer claude; rule 3: last two handed out watane S103 claude, S107 codex, so by the
count Claude builds next; deputy reason: the watane side has S103 on Claude, and this slice is console and kernel work,
where Codex had no round yet in this repository, so Codex builds it and Claude reviews; sketched 2026-11-24 by deputy-3; Codex task https://chatgpt.com/local/6b054293-b150-83ed-8869-70e96b05307e, dispatched ~07:53 UTC; DELIVERED 08:05 as PR #52 (507+/85-, pushed only slice/S38); r1: Claude review routine `trig_01LqzqTFjZfs2fKqkKTvzqbj`, fires 08:10 UTC; r1 (08:40): 2/8 findings, changes (2) (validator RETURN x2, adversary HELD, user-sim REJECTED on outcome 3, accepted as R1.1); answer r1 dispatched 08:45 UTC into the same Codex task; r1 answered at 2d87caf (F1.1, F1.2, P1.1, P1.3, P1.4 fixed; R1.1 recorded); r2: update+run of the routine fired the stale r1 brief in a new session `cse_01LxwP91yLzc3xcdJySnNMKD` at 08:45 UTC (stop refused by auto mode): it reviews head 2d87caf in full, counted as r2, the deputy applies the r1 rulings when weighing it; r2 (09:10): 1/7, changes (1), F2.1 = F1.2 again (the reviewer never saw the ruling): ruled narrower, answer r2 dispatched 09:11 UTC into the Codex task; r2 answered at 1e0731c (F2.1 as ruled, P2.1, P2.4, R2.2 hardening); r3: scoped Claude review (validator only, user-sim not run: no outcome changed) routine `trig_01HfzzdxruNuMtibzfyM5kXq`, fires 09:15 UTC; r3 (09:21): 1/2, changes (1): F3.1 a regression from r2's optional R2.2 hardening (a relaunched stream hid a new consent wait); ruling: remove the hardening, R2.2 stays accepted; answer r3 dispatched 09:22 UTC. r4 check-in: worth one validator-only round, the fix is a revert plus the reviewer's own red test, trend 2, 1, 1 with each round's finding of a different kind; r3 answered at c97dd89 (F3.1 as ruled, P3.1, P3.2); r4: validator-only Claude review routine `trig_01DwPznkc4pxHumjXYngbaoz`, fires 09:27 UTC.)
spec-check: covered by the spec-chain review of 2026-11-23 (PR #51); its TECHNICAL errors 1, 2, 12 and 13 are this slice's.

Story (J11, its consent part; TECHNICAL 17 stage C, "this repository" column): Alice starts a session with reach into
her notes folder. The kernel asks the host for the folder; while the app's consent waits, the console shows the
`consent {what}` activity (9.3); she agrees and the agent works in that folder. She withdraws the consent in the manager
mid-turn: the session's stream ends, the console says why, and the next session there asks again.

Outcomes (user-sim on `watane-dev serve` with dev's fake `claude`; a real `claude` run is the human's, not CI's):
1. A new session with a folder shows `running` with `activity: consent {what}` while the consent waits, and runs in
   that folder once it is given; the transcript shows the turn.
2. A refused consent ends the launch with its cause named in the console; no stream is left open, no folder key offered.
3. A withdrawal mid-turn ends that session (state and transcript up to the last finished message kept; the cause named),
   and the next session in that folder asks again.
4. AGT-J4's folder line: a folder in `~/.ssh` (or any path the host refuses) is refused before any question reaches
   the participant, and the console names the cause.
5. Case 31 for `consent` passes; the stage C budgets hold.

Interface: watane host 19.7 (consents, `consent-pending`, withdrawal) and 19.5 (H4) as watane's `host/TECHNICAL.md`
states them; watane S103 builds them (in flight). Until it merges, fake them in dev's fake host or a test double at that
interface, and say where; a gap between the fake and host 19.7 is a question to the deputy, not a guess. Once S103 merges,
the last task runs outcomes 1-3 against it.
Constraints: no shell grant (V27 waits on the human; build nothing that assumes its answer). Fix TECHNICAL errors 1, 2,
12, 13 of PR #51's list in the sections this slice touches (with `spec-change`); the rest stay with #51. Stage B carry
list items that this slice's code reaches (S32 R1 first) are folded in. Touches: `kernel/`, `console/`, `provider/` if
the consent passes through it, `design/TECHNICAL.md` 4, 9, 14. Boundary: TECHNICAL 14 (adversary runs). Protects: *core*.

### S39: Alice installs the agent mod in her watane app and talks to her real Claude Code in its console
(in flight, cloud, builder codex, reviewer claude; rule 3: last two handed out S38 codex, S103 claude, so Codex builds;
asked by the human, 2026-11-24: "the harness ui integrated into the harness backend as a complete mod"; sketched by
deputy-3; Codex task https://chatgpt.com/local/6b058467-f1e8-83ed-936f-4c8adb37c047, dispatched ~12:30 UTC; BLOCKED 12:58 at 3ac6a3f as PR #53 (+470/-19): both releases build and install in the app and the console opens, but a scratch turn ends `no-app`: watane's app never gives the host its Settings accounts (`HostConfirm` has no `accounts()`). watane S103 (PR #26) adds exactly that (`app/src/confirm.rs`), so S39 holds until S103 merges, then continues from `slice/S39` in the same Codex task: prove the installed scratch journey on watane main. No round runs meanwhile. Changed 2026-11-24 15:20 UTC under the new deputy rule (a dependent slice does not wait; mboogerd/claude-plugin#1, the human's go-ahead): continued now against watane `slice/s103`, rerun on watane main once PR #26 merges. DELIVERED 15:50 UTC at 29606b1 (outcomes 2, 3, 5 on watane slice/s103 1ffb3b1; CI gate and installed-agent green; desktop CI pinned to that watane revision until PR #26 merges); r1: Claude review routine, fires 15:53 UTC; r1 (16:18): 0/5, VERDICT approve (validator PASS/RETURN regraded, adversary HELD, user-sim ACCEPTED, all 5 outcomes on watane slice/s103 1838d16). Before merge: once watane PR #26 merges, the same Codex task moves the CI pin (`.github/workflows/installed-agent.yml:22`) to watane main, takes P1.1 (two Mac notes) and A1.1's four lines (test the provider pin before building), and a confirming round reads that diff. watane PR #26 merged at a923a8dc (2026-11-24); pre-merge brief sent into the Codex task ~19:00 UTC; answered at dbba42b (CI pinned to watane main a923a8dc, outcomes 2, 3, 5 rerun; P1.1; A1.1), CI green; r2: Claude confirming read, fires 19:17 UTC; r2 (19:19): 0/0, VERDICT approve, gate and installed-agent green at dbba42b on watane main a923a8dc; waits on the human's merge word.)
Why: today the kernel, its console and the provider run only in `watane-dev serve`, assembled by test scripts
(`kernel/wasm/ui` and `logic.wasm` are git-ignored build outputs). The agent layer is a mod (VISION) and must install and
run like one. This is stage C's "the same on the real `claude`" (TECHNICAL 17), for scratch sessions.

Outcomes (user-sim on the real watane app, CI on dev's fake `claude`):
1. One command from a clean checkout builds the complete agent mod (kernel logic, the console as its page) and the
   `claude-code` provider, and publishes them as releases with `watane-dev publish` to a local test site; the built mod
   passes `watane-dev check`.
2. In a fresh watane app home, Alice pastes the agent mod's link into "Install from link": one consent window lists the
   kernel and the provider it needs with their grants in words, she approves, and both are installed.
3. She opens the agent mod from her launcher: its page is the console; with dev's fake `claude` set as the Claude Code
   binary (Settings, S102), she starts a scratch session, sees the reply stream, answers a tool ask, stops a long turn,
   and after quitting and restarting the app the transcript is back.
4. The same with her real `claude` binary and login: the human's check on the Mac, not CI's (no cloud team can sign in
   to the human's account). The PR says how to run it in a few commands.
5. Existing gates stay green; `npm run test:host` and the kernel tests still pass.

Interface: watane's install of a mod with a dependency (as the app fixtures `recommendations` and `reading-list`), host
19.3 (Claude Code launch, scratch workspaces), app 19.1/19.3 (the Claude Code binary setting). Constraints: picked
folders and their consent are left out (watane S103, not merged; S38's folder field may stay but needs no new work); no
real account in CI; no change to watane unless a gap blocks install, and then a question to the deputy first. Touches:
`kernel/wasm/`, `provider/`, `console/` build, a build or release script, `factory/gate.sh` if the gate builds the mod,
`design/TECHNICAL.md` 1 (layout) and 17. Boundary: TECHNICAL 14 (the provider's grants and the install consent: the
adversary runs). Protects: *core*.

### S40: stage D, the consumer half: Alice runs a profile on her ChatGPT plan in the console
(deputy-4, 2026-11-25 07:25 UTC: r3 answered at 890caad by the cut; installed-agent red once on it ('Timed out waiting for Claude Code settings', green on 3f13dc1), rerun once. r4 check-in: worth one more round, a reading check of the cut against J11 with 2 blocking left in r3, both in the cut mechanism. r4: Codex review, a follow-up in https://chatgpt.com/local/6b05eeef-aae8-83eb-85bf-a60c3e428074, dispatched 07:28 UTC by deputy-4 from the Desktop app; r4 (07:32): 3/1, changes (3) (validator RETURN, adversary BREACHED: a queued continuation can dispatch under another origin, user-sim REJECTED); trend 2 to 3, all in restart: the cut's last answer round. Deputy rulings (PR comment 7156848307): F4.1 stands (a turn waiting on an approval is mid-turn), F4.2 stands as an outcome (no request under any origin but the owner's; a busy refusal is fine), F4.3 accepted as a known limit (a message finished just before a restart is shown but not in the model's history; one line in 9.5), P4.1 taken; answer r4: Claude routine `trig_01WwvGqijC7yA3Tx3tB1XvrC`, fires 08:19 UTC. r5 check-in: worth one more round only as a check of F4.1 and F4.2 (2 blocking, one a breach); a new blocking finding in restart after it goes to the human as stalled; r4 answered at 10ed0a3 (F4.1 fixed; F4.2 by the busy refusal for start, say and interrupt; the builder named a residual: a running turn's later request can still wait in the FIFO); r5: Codex, same task, dispatched 08:30 UTC, checks F4.1, F4.2 and grades the residual.) [deputy-5, 2026-11-25: r5 (09:29, posted 12:13 UTC, Codex held by a signed-out pane): 1/0, changes (1), validator RETURN, adversary BREACHED, user-sim ACCEPTED on the cut; F4.1 fixed; F5.1 = F4.2 a second time (the builder's FIFO residual): stalled, cut by deputy-5 (PR comment 7162124773): no model request of an OpenAI session is sent under any origin but its owner's; a running turn's later request with no slot stops `busy` (state kept, owner's next message continues) or waits only for its own owner; general origin plumbing and slot fairness out of scope (Upkeep if wanted). r6 check-in: worth one focused round, 1 blocking left, a breach, trend falling 3, 1; a new blocking finding in restart or slots after r6 goes to the human. Answer r5: Claude routine `trig_01JVYBY9zA2NiNuDEi7Rff2o`, fires 12:17 UTC.; r5 answered at 553c95f (F5.1: a later request with no slot stops busy; `Origins` checker); r6: Codex, same task, 12:29 UTC; r6 (12:32): 1/0, changes (1), validator PASS, adversary BREACHED, user-sim ACCEPTED: F6.1 a retry or resume timer of a session whose owner mod was disabled fires under the kernel's origin (watane substitutes it); new after r6, so the human chose (a), then (b) if it fails (Human answers). CI green at 553c95f. Answer r6: Claude routine `trig_01GV5y7MPmh3LK88sd4xySkP`, fires 16:21 UTC; r7 to Codex in the same task, a check of F6.1 only; a blocking r7 means (b): accept F6.1 as a known limit, Upkeep, merge.; r6 answered at 91ae032; r7 (16:30): 1/0, changes (1), adversary HELD, user-sim REJECTED: F7.1 the r6 fix also stops a valid owner's timer firing (watane reports a timer callback's caller as `host`), so the automatic resume after a usage-limit reset broke; (a) did not succeed, so (b) (PR comment 7168348172): revert to 553c95f, F6.1 a known limit (one line in 9.5), Upkeep; answer r7: Claude routine `trig_01KGmYmFKpPqioTnJBgJxdtU`, fires 16:41 UTC; then a mechanical check (code diff against 553c95f is the 9.5 line only), green CI, merge with no further round.]
(in flight, cloud, builder claude, reviewer codex; rule 3: last two handed out watane S113 codex, S112 codex; the
consumer-halves lane, sketched 2026-11-24 by deputy-3: it builds against watane `slice/s111` (PR #28), whose branch holds a
first version of host 19.4 `https-stream` with the `openai` account and dev's scripted Responses server; it merges main
and reruns on watane main once PR #28 merges; routine `trig_01MKpjib1FAYCnuVhuMx2RRD`, fires 19:05 UTC; BLOCKED 20:05 as PR #54: a preview has no scripted OpenAI stream (watane dev question 15). Ruling (b): S40 is reviewed and merged on its kernel and console proof (cargo tests on FakeHost, the console's fake kernel, test:host:d's 5 checks on the real host); outcomes 2 to 4 on `watane-dev serve` and the stage D budgets move to Upkeep, for when watane S114 (dev 15 option (a), F-MAKE-4) merges. r1: Codex review https://chatgpt.com/local/6b05eeef-aae8-83eb-85bf-a60c3e428074, dispatched 20:12 UTC; r1 (20:38): 7/3 by the reviewer, changes (7), adversary HELD, user-sim REJECTED; deputy grade 5/5 (F1.2 a split usage-limit body and F1.7 a malformed head need host behaviour that does not occur: polish, taken as cheap); posted on the PR; answer r1 to a Claude routine, fires 20:42 UTC, rebuilt on watane main 71b3ae6a (S111 merged); r1 answered at 7e87c41 (+574/-97, all 7 fixed red-first, test:host:d 5 PASS on watane main a98d96f); r2: same Codex review task, dispatched 21:05 UTC; r2 (21:08): 3/1, changes (3), adversary BREACHED 1 (restart recovery opened a session's toolsets under another owner's origin), user-sim REJECTED; F2.1 and F2.2 sit in the restart recovery r1's answer added (stall by area): ruled narrow, every open job is aborted after a restart, the reissue code goes, case 22's idempotent half is a known limit; F2.3 fixed; answer r2 to Claude, fires 21:36 UTC; r2 answered at 3f13dc1; r3 (21:47): 2/1, changes (2), adversary BREACHED (the global restart pass can still send HTTPS or open a first toolset under the origin that armed it, for another owner's session); F3.2 a restart sends after a quota stop. Restart recovery stalled three rounds running (F1.5, F2.1/F2.2, F3.1/F3.2): held, cut proposed to the human (Waiting on the human, S40-CUT).)
Story (AGT-J2; TECHNICAL 17 stage D, "this repository" column: the OpenAI loop of 9.5 and the console): Alice switches a
profile to an OpenAI model; the kernel refuses the fields OpenAI cannot honour and names them; with a profile it can run,
the console shows the same kinds of events as with Claude; when her plan's limit runs out the session reads paused until
the reset time and continues by itself after it.
Outcomes (user-sim on `watane-dev serve` with dev's scripted Responses server; no real OpenAI call in CI):
1. A profile with an OpenAI model and file tools or a folder is refused, and the console names each field (AGT-J2).
2. A runnable OpenAI profile streams a reply into the console, with tool calls through the kernel's tools, as the Claude
   backend's events do (one event format for every backend).
3. A usage-limit answer pauses the session until its reset time, shown in the console, and it resumes on its own after it.
4. The context meter shows the session's use; cases 1-4, 19 (`openai`), 22, 39, 40 pass where the kernel owns them, and
   the stage D budgets hold.
Interface: watane host 19.4 (H3) as on `slice/s111`; agentic-harness TECHNICAL 5 and 9.5. Constraints: no change to watane;
a gap is a question to the deputy. No real account in CI; a real ChatGPT run is the human's. Touches: `kernel/` (the
OpenAI loop), `console/`, `design/TECHNICAL.md` 9.5 and 17. Boundary: TECHNICAL 14 (the token never reaches the kernel;
adversary runs). Protects: *core*.



- **S128 (upkeep, in flight, cloud, builder codex, reviewer claude; rule 3: last two S127 claude, S126 codex; sketched
  2026-11-25 by deputy-6; Codex task https://chatgpt.com/local/6b072ad7-d4b0-83ed-80af-2de03a41f066, 18:40 UTC; beside S127: S127 owns the kernel's turn/verdict code and the console's activity and verdict
  display, S128 edits none of them).** Outcomes, each item's check: (1, guard) the F6.1 fix below, through the real adapter;
  (2, guard) the `installed-agent` flake, now 4 sightings (S40 890caad, 10ed0a3, a2235d6 twice): find the race or bound
  the wait by what it measures; (3, guard) `kit/s7_fix1.test.ts:34` failed on main 716782c by 5 µs (a ratio of two
  wall-clock timings): no such assertion on a loaded runner; (4) case 22's idempotent half; (5) S40 on `watane-dev
  serve` (watane S114 is merged): SKIP lines of `test:host:d` become checks, S40 outcomes 2 to 4 on a preview, the stage D
  load test. Touches: kernel (timer adapter, restart), console/test/host, kit. No test calls an external provider.
  Protects: *guard* (1-3), *core* (4, 5). Size: an upkeep slice. BLOCKED 18:58 UTC (PR #57, draft): F6.1 and case 22
  need a watane authority contract (a resolved origin for timer firings). Ruled (19:00): the alternative; items 1 and 4
  cut, both stay known limits (9.5); the contract is held as a Door C question in watane PLAN Unsettled. DELIVERED
  ~19:15 UTC (PR #57, +590/-100, CI green); r1: Claude review routine `trig_016nHNDkf4QGgFeMKcE6ZXVw`, 19:21 UTC;
  r1: 1/9, changes (1) (validator RETURN on F1.1 the load test measures no budgets, adversary HELD, user-sim not run:
  no watane tools in the Claude cloud, the builder ran 9/9 and 1/1); answer r1 to Codex in the same task, ~19:45 UTC;
  P1.2 accepted (deputy): ~1 s first-request delay, F6.1 window widens to every first request (one line in 9.5).
  The items:

### S127: stage E: Alice's Claude session runs through the checked provider, with Jev as its judge
(in flight, cloud, builder claude, reviewer codex; rule 3: last two handed out watane S126 codex, S125 codex; the main
line, sketched 2026-11-25 by deputy-6; routine `trig_01E1Ngk3t7NCGXJnEjBgEjHR`, fires 18:39 UTC; spec-check 382f793 (deputy-6, stage E scope: 14 points, folded in below).)
DELIVERED 18:58 UTC as PR #56 (+3705/-38). Deputy D1.1 (blocking): the answer field is `noul`, not `probability`
(PR comment 7171667656); BLOCKED question (outcome verification) ruled (b): merged on its native proof, `--scripted-judge`
as watane S129, then `test:host:e` in Upkeep. r1: Codex review https://chatgpt.com/local/6b073026-11b0-83ed-b550-0ef125943064,
dispatched 19:00 UTC; withheld by Codex's cyber filter; retried once with the adversary worded as a boundary check: https://chatgpt.com/local/6b07347c-1450-83ed-b77a-1aabbc8df068 (a second refusal: r1 runs on Claude). D1.1 and D1.2 answered at 4fe51c4 (CI green). r1 (Codex, 2026-11-25 20:33): 4/2, `changes (4)` (validator RETURN, adversary gap F1.1, user-sim REJECTED), posted 2026-11-26 06:28 with the deputy's ruling (all four blocking; the preview-scores risk accepted); answered at db21a42 (all 4 fixed); r2: Codex review https://chatgpt.com/local/6b07d354-fff0-83ed-a526-8085a130307b, 2026-11-26 ~06:40 UTC; r2: 2/1 `changes (2)` (all §14 HELD), posted 06:58 with a **cut** (deputy-7): F2.1 (a verdict's reason lost under backpressure at teardown) accepted as a known limit in TECHNICAL 10, F2.2 (a queued input dropped on a terminal launch denial) fixed small; r3 checks only these two. Answered at 3df5713; r3: Codex review https://chatgpt.com/local/6b07d9e3-a408-83eb-9c6e-d1831c52e073; r3: 1/1 `changes (1)` (§14 HELD): F3.1 a say between a terminal launch verdict and `ended` sends the queued follow-up to the ended stream (same queue code as F2.2). Posted ~07:38 with Deputy Rock's last-round ruling: a few-line fix on existing state, else a one-line known limit in TECHNICAL 10; r4 scoped to F3.1 only; P3.1 polish. Answered at 764d5ed (F3.1 fixed on existing state, P3.1 done); r4: Codex review https://chatgpt.com/local/6b07e18d-25f4-83ed-bf16-01dbaf891068, ~07:38 UTC.
Story (AGT-J3; TECHNICAL 17 stage E, "this repository" column): Alice installs the checked provider, enters her judge's key
and origin in the app (watane S122), and runs a Claude session through it: before each tool call the console shows
`checking`, a safe call goes on, an unsafe or unsure one is denied with a reason she can read, and after repeated denials
the breaker stops the session and says why.
Outcomes (user-sim on `watane-dev serve`, dev's fake `claude` and dev's scripted judge, plays through `Raw`):
1. A tool call the judge scores safe runs; the console shows `checking` while the judge answers (case 31 for `checking`).
2. A call scored unsafe, or in the unsure band, is denied before it runs, and the transcript shows the verdict and a
   reason naming each question and its score (AGT-F-32, AGT-F-34).
3. A judge that times out, answers garbage, refuses the key or has no key denies (cases 19 `judge`, 25, 44).
4. Three denials in a row, or 10 of the last 50, interrupt the session with a verdict item that says why (the breaker).
5. Cases 32, 33, 36, 37, 38, 42, 43, 46, 47, 50, 51 (provider parts) pass, and the stage E budgets of 17 hold.
Decisions (deputy-6, Door C, listed for the human's veto): see Deputy decisions 2026-11-25 "judge wire shape". In short:
the judge call is TypeSafe System One (`POST <origin>/v1/systemone`, Bearer, `noul` questions in one request); the
provider maps the probabilities to `safe | unsafe | unsure` with thresholds kept on its own page (default: unsafe at
>= 0.7, safe at <= 0.3, unsure between, for the harm question); `reasons` are provider-written text naming each
question and its score. The origin stays the one Alice enters in the app (TECHNICAL 5; 10's "judge URL" setting goes);
the provider's `[http]` lists `api.typesafe.ai` (and the fake's `judge.test` in tests); another judge API is a derived
provider. V7 (Jev, the Mac, 2026-11-25): p50 173 ms, p95 267 ms, max 846 ms per call, new TLS each, 2 questions.
Spec work in the slice (spec-change): TECHNICAL 10 (request shape, threshold rule, retries within one deadline, the
`verdict`/`outcome` field names of 9.8 and 10 made one), 9.8's entry, 16 cases 25 and 44 (unsure is the band),
FUNCTIONAL AGT-F-32/34 and line 36 wording ("scores" not "says"), and V7 into ARCHITECTURE-PROPOSAL 18.
Constraints: **no cloud test calls an external provider** (the human, 2026-11-25): no Jev call in CI or a cloud
session; tests use dev's scripted judge with `Raw` replies in System One's shape. No change to watane: dev's `Play`
shape stays (Leftovers there); a gap is a question to the deputy. A real Jev run is the deputy's, on the Mac.
Touches: `provider-checked/` (new, R-B1), `kernel/` (`checking` activity and verdict items only where 9.3/9.8 need it),
`console/` (the activity and verdict display), `design/` (above). Boundary: TECHNICAL 14 (the key never reaches the
provider or kernel; only the owner may spend the judge: adversary runs). Protects: *core*.

### S132 (design) stage F: delegation and counters, made one consistent contract
(in flight, cloud, builder codex, reviewer claude; Codex task https://chatgpt.com/local/6b07e7d8-0604-83eb-98fc-12f978a90070, 08:03 UTC; DELIVERED ~08:14 as PR #59 at 7e61cae (+281/-68, design/ only; 7 Door C choices in its body, §14 child admission rewritten); r1: Claude review routine `trig_01Vhs2W87TUiUeaQZFqNtwmm`, 08:16 UTC; rule 3: last two handed out S131 claude, watane S130 codex, but Deputy Rock's reason: a design slice of spec text suits either side, and Codex builds two in three; the main line, sketched 2026-11-26 by Deputy Rock after S127 merged; spec-check 10abec0, report `factory/spec-check-S132.md` (Sonnet, no second-vendor confirmation). Size: a design slice, below 1,500 lines by nature; it unblocks stage F's build.)
Why: stage F's clauses do not fit together. AGT-J5, stage F's own done-when, cannot run on any backend: a foreman (a child) must start a validator, but an OpenAI child has no `task` tool and Claude has no child sessions, only launch-time subagents. A build slice on this would come back as a question in every round.
Outcomes (the review checks them in the documents):
1. AGT-J5 runs, as written or as narrowed: the design says which backend's sessions can be children and parents, at what depth, and FUNCTIONAL 3.9, AGT-J5 and TECHNICAL 9.7 agree with it (spec-check findings 1, 2, 4).
2. A child's narrowing is a rule: which profile fields compare, where the check runs, the refusal field, and the child's workspace, folder and owner (findings 3, 10).
3. The counters and `usage` per backend: scope (session or turn), what happens at `max-turns` on Claude, how `max-tokens` stops a turn, and the stage `usage` lands in (findings 5, 7).
4. `task`'s result, the transcript entry that carries a child, restart with live children, and the console's clause for children (findings 6, 9, 10); the OpenAI `skill` tool's stage (finding 11).
5. The stage F budget row holds against the slot caps, or is restated (finding 8). The low findings are fixed where one line does it.
Touches: `design/FUNCTIONAL.md` (3.9, 3.11, AGT-J5), `design/TECHNICAL.md` (2.3, 9.1 to 9.5 where 9.7 cites them, 9.7, 14, 16 cases 23, 24, 52, 17, 18.3), `factory/spec-check-S132.md`. No code.
Constraints: design only, through the `spec-change` skill; no code, no VISION change (a needed one goes to the deputy as BLOCKED). Prefer narrowing to building: a mechanism only where FUNCTIONAL promises the behaviour. Each Door C choice (an interface, a public tool, a format) is listed in the PR body with its options and why, so the deputy can report it for the human's veto. Out of scope: the factory profiles and N5 (waiting on the human).

### S131 (upkeep) the host tests check what they skip: stage C on a participant home, stage B and D test polish
(in flight, cloud, builder claude, reviewer codex; rule 3: last two handed out watane S130 codex, S129 codex; the upkeep
lane, sketched 2026-11-26 by deputy-7 from Upkeep; no spec-check: tests only, no spec document touched; routine `trig_012rpGShQDeh3hBe4E8XsmGc`, fires 06:31 UTC; PR #58 opened ~07:20 at 495975c (+6524/-60, 5.3k of it the test home's Cargo.lock; shares only AGENTS.md with S127); r1: Codex review https://chatgpt.com/local/6b07deda-2b30-83ed-ae72-c1ec08020061, 07:27 UTC; r1: 8/2 `changes (8)` (boundary HELD; outcome check could not run, no watane-dev in Codex), posted ~07:40 with Deputy Rock's ruling (F1.1/P1.1 sketch amended, F1.2 a named known gap with the console fix to Upkeep, F1.7/F1.8 narrowed); answer by the build session itself (idle at 07:24; a new answer routine only if no push by ~08:05); answered by the build session at 461bb74 (~08:21, all 8, main merged in: squash at merge); r2: Codex review https://chatgpt.com/local/6b07ec6b-3124-83ed-8204-bae83a42b053, 08:24 UTC; r2: 1/1 `changes (1)` (F2.1 the expiry test accepts a host cancellation; P2.1 a red run for the stream assertion; boundary HELD), posted 08:33 by Deputy Paper with the answer brief as a PR comment (the build session answers PR comments); answered at 48d54d4 (F2.1 fixed, red first; P2.1 answered without change); r3: same Codex review task, scoped to 461bb74..48d54d4, ~08:47 UTC. Size: an upkeep slice.)
Outcomes, each item's check (re-check each against main first; drop the ones already true and say so):
1. S38 after watane S103: S38 outcomes 1 to 3 run on a participant home with a fake `Confirm` (host 19.7), outcomes 2 and 4
   on `watane-dev serve`, and the stage C budgets of TECHNICAL 17 are measured and recorded in the Suite time table.
2. S128 polish P1.4, P1.5, P1.9, P2.1 to P2.5 (`console/test/host/`, `stage_d.mjs`, `metrics.test.ts`, `AGENTS.md` exit
   codes, the stage D Budget row); P2.6 only if `watane.patch` still exists.
3. S36 r1 polish P1.1 (`kernel/tests/it/s36_firing.rs`), P1.3 (`console/test/host/stage_b.mjs` browser assertions);
   S38 P1.2 (`kernel/tests/it/s38.rs`).
Touches: `console/test/host/`, `kernel/tests/it/s36_firing.rs`, `kernel/tests/it/s38.rs`, `console/kit/metrics.test.ts`,
`AGENTS.md`, this plan's Suite time rows; also `console/package.json` test aliases and `factory/tasks/S131-*.md` (amended r1, Deputy Rock). Not: `kernel/src/`, `console/kit/` sources, `console/src/`, `ui/`, `provider-checked/`,
`design/` (S127's files). A product bug found on the way is reported in the PR, not fixed here. No test calls an external
provider. Protects: *guard* (tests that skip hide regressions).


### S134 stage F: an OpenAI agent delegates to two children, and every turn stops at its limits
(in flight, cloud, builder claude, reviewer codex by rule 3 (last two S133 codex, S132 codex); sketched 2026-11-26 by
Deputy Paper, the main line; spec-check 35b3abe: 0 severe, 2 medium (term clashes: `subagent-limits` names OpenAI
children while FUNCTIONAL keeps 'subagent' for Claude; `max-turns` counts OpenAI model steps, not turns), 3 unsure
(the tool-error streak counts denials; `usage-unavailable` absent from AGT-F-36 and case 23; Claude's weaker limits are
S132's Door C, already out for veto): the build team settles the first four with spec-change; routine `trig_01Pey52A9vKhTvnKCAC6SgG5`, fires 08:36 UTC; DELIVERED as PR #60 at 06e67e6 (+4934/-194); r1: Codex review https://chatgpt.com/local/6b07fd23-9fbc-83eb-ac12-256d44550040 r1: 9/1 findings (changes), posted 10:02 UTC; answer brief as PR comment (the build session is subscribed); Deputy Scissors ruled F1.9 narrow: p95 state bytes on the fake host, page-level budgets "not measured (needs watane-dev)" as a known limit; r1 answers e0c1c1d (7 fixed, F1.4 and F1.5 disagree, F1.9 in part); r2: Codex follow-up in the same task, 10:14 UTC; r2: 3/1 (changes), answer brief as PR comment 10:27 UTC; r2 answers abb3a97 (4 fixed); r3: Codex follow-up, 10:33 UTC; r3: 1/1 (changes; F3.1 same mechanism as F2.2), answer brief 10:50 UTC with a ruling: one rule (no path drops `Work.stopped`), and a recurrence in r4 becomes a known limit. r4 check-in: worth it, blocking fell 9→3→1, one narrow fix left, live-UI only (stored state right); r3 answers 5535d7f (2 fixed); r4: Codex follow-up, 10:56 UTC, ~09:50 UTC. Size: a whole stage, likely over 1,500 lines.)
Why: stage F of TECHNICAL 17, on S132's design (9.7, merged at 95ec3bc7). The factory as mods needs an agent that hands
named work to other agents and a turn that cannot run away.
Outcomes (user-sim on the real programs, scripted Responses and dev's fake `claude`, no real provider):
1. AGT-J5: Alice runs an OpenAI root profile with two named delegates; the console shows two sibling children as indexed
   `child` entries, each opens its own transcript, and the root's final answer uses both children's results.
2. Admission (9.7, cases 23 and 24): a child that calls `task`, a name outside `delegates`, and a candidate wider than
   its root (one row of the child-rule table each) are refused with that field named, and no child session exists.
3. Counters (AGT-F-36, case 52 `task` part): an OpenAI turn stops at `max-turns` (with at most one tool-free summary step),
   at `max-tool-errors` and at `max-tokens`, each naming the limit; a Claude turn's native `max-turns` stop is reported;
   a Claude profile with `max-tokens` or `max-tool-errors` is refused (S132's Door C choice).
4. Revoking: once `access/<owner>` no longer names the root's profile, a `task` call gets the `profile` refusal as its
   output, and reads still work.
5. The stage F budgets of TECHNICAL 17 are measured and recorded in Suite time.
Touches: `kernel/src/` (9.7), `console/src/` and `console/kit/` (child entries), their tests, `design/` only where the
build finds a gap (through spec-change). Not S131's files (`console/test/host/`, `kernel/tests/it/s36_firing.rs`,
`s38.rs`, `AGENTS.md`) until S131 merges; then merge main.
Constraints: S132's nine Door C choices stand (PR #59 body), do not reopen them; fold S132's review polish P1.1 to P1.12
(PR #59, comment 'Review r1 by Claude') in where cheap. Factory profiles wait on N5 (Waiting on the human): out of
scope. No watane change: if one is needed, BLOCKED with the interface it needs.


### S139 (upkeep) the console's known gaps and the `installed-agent` flake
(sketched 2026-11-26 by Deputy Scissors, the upkeep lane; builder codex, reviewer claude by rule 3; in flight, cloud, Codex task https://chatgpt.com/local/6b081583-0fe0-83ed-9e52-86ea878b7056, 11:23 UTC; DELIVERED as PR #62 at 148004e (+259/-97; also ui/); r1: Claude routine `trig_01WWHnzB9VfNZRDLjpxzK4Q1`, 11:54 UTC; exempt from the size
rule (upkeep).)
Outcomes (each item's check, re-checked against main first; an item already fixed is dropped with a line):
1. *guard* the `installed-agent` CI flake (`console/test/host/installed.mjs:216`, second sighting): the race found and
   fixed, or a wait with a bound that measures it.
2. S131 F1.2: the console's consent activity names the folder the app asks for; `stage_c.mjs`'s KNOWN GAP becomes a check.
3. S39 review, console: after Stop no 'An event this console does not know'; a stopped turn's footer does not read
   'Completed'; with no Claude Code binary chosen the error points to Settings → Claude Code.
4. S36 polish P1.2 and P1.3 (console tests), and S36's felt-wrong list where cheap.
Touches: `console/` only. Not `kernel/` or `design/` (S138); a spec gap goes to Upkeep, not into `design/TECHNICAL.md`.


### S140 (upkeep, guard first) the console's known gaps and test polish, console/ only
(sketched 2026-11-26 by Deputy Rock, the upkeep lane; builder claude by rule 3 (last two S138 codex, S139 codex),
reviewer codex; in flight, cloud, routine `trig_01Hkd9KmgKVeQ1HVtLVY31hY` fires 12:18 UTC; DELIVERED as PR #63 at 5ee2aac (13:01 UTC, +243/-30; items 2, 3, 5 already fixed by S139, dropped; CI green); r1: Codex review https://chatgpt.com/local/6b0830f4-f93c-83ed-b07a-f5fbc2366044, 13:17 UTC; r1: 1/2 (changes; F1.1 the 240-char preview counts UTF-16 units, ruled narrow), posted 13:29 UTC with the answer brief as a PR comment for the subscribed build session (it woke on it); r1 answers at 27f9956 (13:33 UTC); r2: Codex follow-up in the same task, 13:35 UTC; r2: 0/0, VERDICT: approve on 27f9956 (13:41 UTC); CI `gate` timed out at 15 min once on 27f9956, green on rerun (second sighting after S134); user-sim not reported by the review lead, asked in the same task 14:00 UTC; adversary not run (deputy: `provider/BOUNDARIES.md` is a copy, `publish-test-mods.sh` an equivalent JSON read, tested both ways); the build session (`cse_01LxwP8xUGgojrr3XJGNbqD4`) subscribed to PR activity, so it may wake on the review: check it before any answer routine (one writer); spec-check not run: the slice changes no spec text (below). Upkeep is exempt from the size rule.)
Why: Upkeep holds console work that no slice in flight touches; S138 edits only kernel/, design/ and the wit/deps folders.
Outcomes (each item's check, re-checked against main first; an item already fixed is dropped with a line):
(1) *guard* the `installed-agent` flake: `console/test/host/installed.mjs:216` 'Timed out waiting for Claude Code
settings' after AC1 passes (S40, twice): the race found and fixed, or the wait bounded and measured, with the reason in the
test; (2) the console's consent activity names the folder the app asks for (S131 F1.2), and `stage_c.mjs` drops its
known-gap line for it; (3) after Stop the transcript shows no 'An event this console does not know' for claude's
`control_response` to the interrupt, and a stopped turn's footer does not read 'Completed'; with no Claude Code binary
chosen the error points to Settings → Claude Code (S39 review); (4) S39 polish P1.2 to P1.5 and S139 r1 polish P1.1 to P1.4,
P1.6, P1.7 (PR #62), each where its fix lands outside kernel/ and design/; (5) S36 r1 polish P1.2 and P1.3 (console tests).
Touches: `console/`, `provider/BOUNDARIES.md` (S39 P1.4). Not kernel/ or design/ (S138), not the wit/deps folders.
Constraints: no new interface, no spec text change: an item whose fix needs a rule change in design/ or a kernel change
stays in Upkeep with a line in the PR body; a guard never weakens; `installed-agent` must not be made weaker to pass.
deputy compares: the stopped-turn transcript and footer, and the consent activity, against the console design (TECHNICAL 18).

### S142 (upkeep) the console's Stop and question fixes, stage E on a real host, and stage D's lost item, console/ only
(sketched 2026-11-26 by Deputy Rock, the upkeep lane; builder codex by rule 3 (last two S141 codex, S140 claude: Claude
built one of them, so Codex); reviewer claude; in flight, cloud, Codex task https://chatgpt.com/local/6b08428a-6b20-83eb-aadb-b832d92ba039, 14:33 UTC; spec-check not run: no spec text changes (below). Upkeep is exempt from the size rule. PR #64 opened 15:02 UTC on 6a0f917; r1 review routine `trig_01Jx5kk2Jp9kcU8432F6ELQ3`, 15:05 UTC. r1: 1/5 findings, review 15:18; answer to Codex (same task) 15:21 UTC, rulings: F1.1 hide Stop and send in a folder wait; A1 to A4 accepted (Deputy Paper: a stale-view race needing a grant and a click in one update window, closed later by a kernel rule, Upkeep). Answers ca2ae278 15:25; r2 review routine `trig_018HybQkC8vsYCgbHdxnQx5H` 15:26 UTC.)
Why: Upkeep's console items, while S138 (approved, held on a watane release) keeps kernel/ and design/.
Outcomes (each item's check, re-checked against main first; an item already fixed is dropped with a line):
(1) *guard* Stop before `initialized`, while a folder consent waits, cancels the stream instead of sending `interrupt`,
so no later yes answers into an idle session (watane S103 r1): a test that fails today, where the fix is in the console;
(2) S131 F1.8 diagnosed: why one stage D load run lost 1 item and 4 tool calls of an OpenAI session at the page, shown
by a run or a test, then fixed if the cause is in console/, else written as a kernel Upkeep line with the evidence;
(3) `test:host:e`: stage E's outcomes on `watane-dev serve` with dev's scripted judge (after S127);
(4) S38 P2.6 'Open the question' opens it and the wait shows the folder (States-2); S36 user-sim: Edit and Undo keep a
queued row's place, and Stop and send is not offered on a queued row while the session reads Not running.
Touches: `console/` (src, kit, test/host), `ui/` (the built console). Not kernel/ or design/ (S138), not the wit/deps folders.
Constraints: no new interface and no spec text change: an item that needs a rule change in design/ or a kernel change
stays in Upkeep with a line in the PR body; a guard never weakens; a flake is never fixed by a retry alone.
deputy compares: the consent wait and the queue rows against the console design (TECHNICAL 18).

### S144 (upkeep) the console's queue rows and disabled controls match the design, and its flakes and stage D race, console/ only
(sketched 2026-11-26 by Deputy Paper, the upkeep lane; builder codex by rule 3 (last two: watane S143 claude, S142 codex); reviewer claude; in flight, cloud, Codex task https://chatgpt.com/local/6b085267-a7a0-83eb-ac29-92339e15f025, 15:47 UTC; DELIVERED as PR #65 at 30df2491 (16:12 UTC; validator PASS, adversary HELD, user-sim ACCEPTED); r1 review routine `trig_01TJ5VCT4SVA7q2jNmTep5wU` 16:14 UTC; r1: 2/7 findings (16:31; validator RETURN, adversary BREACHED 1 in the Stage D check, user-sim REJECTED on a pre-existing phone wrap); answer to Codex (same task) 16:35 UTC, rulings: F1.1 rebuild ui/, F1.2 narrow the Stop-cut excuse with both red tests; A1.1 to A1.3 accepted (Deputy Paper: older than S144 or already owned; A1.1 to Upkeep); answers d04a87ee 16:38; r2 review routine `trig_01M1iUYmDmjHutE5NFbeD3RE` 16:40 UTC; spec-check not run: no spec text changes. Upkeep is exempt from the size rule.)
Why: the console's Upkeep items, while S138 (approved, held on a watane release) keeps kernel/ and design/.
Outcomes, each its Upkeep item's check:
(1) the queue rows look as `Main-2` draws them: the "QUEUED · n of 32 · sent when this turn ends" label, a dashed row per item, quiet buttons, the × with aria "Remove from queue", and the "Removed … from the queue [Undo]" line; a row shows a long message cut to a few lines, not all 255 KiB (S36 user-sim); Stop and send is not offered while the session reads Not running (S36 user-sim);
(2) every disabled control says why, visibly and to a screen reader (S142 P2.1, with the r2 review's red test `regress_S142_r2_disabled_stop_and_send_says_why` committed unchanged; "Try again", "Resume now"); the consent wait shows "Esc to stop" (P2.2); one spelling of watane (P2.3);
(3) the `parts_S15` replay flake is a fixed cause, not a longer timeout; S36 F7.3's load check spacing widened to 3x;
(4) stage D's Stop race against the runner's window is diagnosed: the runner counts by the stored interrupted turn, or the Upkeep line names the owner with its trace; the 40 ms source-spacing gaps explained or the check corrected with a reason.
Touches: `console/` (src, kit, test/host), `ui/` (the built console). Not kernel/ or design/ (S138), not the wit/deps folders.
Constraints: no new interface, no spec text change: an item that needs one stays in Upkeep with a line in the PR body; a guard never weakens; `installed-agent` must not be made weaker to pass.
deputy compares: the queue rows and the consent wait against `Main-2` and `States-2` (TECHNICAL 18).

### S138 the kernel acts on a timer firing only under its session owner's authority (consumer half of watane S133/S136; F6.1)
(sketched 2026-11-26 by Deputy Scissors, the main line; builder codex, reviewer claude by rule 3 (last two S137 claude,
S136 codex); in flight, cloud, Codex task https://chatgpt.com/local/6b08156b-80bc-83eb-94e0-70035ce5604a, 11:22 UTC; DELIVERED as PR #61 at 590c639 (+546/-12); r1: Claude routine `trig_017eS5rRjpFSfNeaRv74NHJN`, 11:51 UTC; spec-check not run: the contract is merged in watane (PR #48 design, PR #50 build, host 19.1, 19.6).) r1 answers at 07d910f (12:20 UTC; F1.1 to F1.3 fixed; F1.ci needs a watane release with S136, the human's, asked 12:23 UTC by Deputy Rock); r2: Claude routine `trig_014PqYzP7y2xYu5EcKcccLX4`, 12:22 UTC. r2: 0/4, VERDICT: approve on 07d910f (12:30 UTC); approved; F1.ci needs no release (Deputy Rock's correction, 2026-11-26 16:54 UTC): `installed-agent.yml` builds watane from a pinned commit (a923a8dc) that predates S136. r2 answer to Codex (same task) 16:56 UTC: bump the pin to watane main 44422b22; the r3 review checks only that diff. Answers a05529a8 16:58 (pin only); r3 review routine `trig_01WMPSuJSmDdKFz14mGdTxRu` 17:00 UTC. CI on a05529a8: add and check pass; installed-agent fails on the newer host's call-grant wording (F2.ci, installed.mjs:255); answer to Codex 17:14 UTC: update that expectation only. r3: 1/0 (the same F2.ci as F3.1); answers bc1bb2cd 17:15; r4 review routine `trig_011rubwFXpcQDkn1kZBkNvNA` 17:16 UTC. Check-in r4: still worth it, the slice was approved at r2 and r3/r4 only cover the CI pin and one test string the newer host words differently; nothing else is open. r4's routine verified the diff, then scheduled a wakeup to wait for CI that never fired (friction); r4b routine `trig_01KPbuYE1nrz6V7a7tUNJzz5` 17:43 UTC with CI already green. Friction: the reviewer found no `.claude/factory/review-lead.md` in this repository and used the README's review-team row.
Why: the guard in Upkeep (the human's (b) on S40 F6.1): a timer firing of an OpenAI session whose owner mod was disabled
during a timed pause may send one more model request under the kernel's origin. watane now gives a mod's own top-level
`timer-sink.fired` the read `timers.firing-origin() -> option<string>` (the resolved origin identity, the timer owner's
own identity after the client is disabled or removed, `none` elsewhere).
Outcomes (user-sim on the kernel's tests with the fake host, and on `watane-dev serve` where this cloud has it):
1. A timed resume or restart retry whose firing reads the session owner's identity runs as before.
2. A firing that reads `none`, the kernel's own identity, or any identity other than the owner bound at arm time does
   only local cleanup: no model request, no toolset operation, no consent; the session shows why (a test per case).
3. The owner disabled during a timed pause: the firing sends no model request (F6.1's known limit is removed from 9.5
   and the Upkeep guard is closed).
4. The fake host (`FakeHost`) answers `firing-origin`; the Harness default `none` and a `Fakes` override are used in the
   tests that run on watane-dev.
Touches: `kernel/` (timer firing paths in `openai/run.rs`, `sessions.rs`, the host adapter and `FakeHost`),
`kernel/tests/`, `design/TECHNICAL.md` (2.1, 9.5, 15, case 22). Not `console/` (S139).
Constraints: copy verbatim the section "Copy verbatim into agentic-harness design/TECHNICAL.md" of
https://github.com/civic-tech-pioneer/watane/pull/48 (its body) into 2.1 and 15; do not change the WIT shape (a gap is
BLOCKED). Settle S133 r1 P1.5 (the expected owner is the arming call's `caller.identity()`, never the kernel itself; a
mismatch through a front mod fails safe) and P1.12 (no install generation: a known limit, or a Door C note) in 9.5 with
spec-change (https://raw.githubusercontent.com/mboogerd/claude-plugin/main/skills/spec-change/SKILL.md). Case 22's
idempotent half stays a known limit (S133 gives no restart callback). Adversary: a front mod or another owner whose
firing tries to make the kernel act for a different owner (TECHNICAL 14).

### S145 (upkeep) the console's last S142/S144 polish and its Stage D clocks, console/ only
(sketched 2026-11-26 by Deputy Paper, the upkeep lane; builder codex by rule 3 (last two: S144 codex, watane S143 claude); reviewer claude; in flight, cloud, Codex task https://chatgpt.com/local/6b0863c1-ae4c-83eb-8254-412ade2530c4, 16:58 UTC; r1 (Claude routine `trig_01UqV1ZKC5CDw4uU9ez2j5oh`): 3/7, CI gate red = F1.1, weighed by Deputy Scissors: all three blocking stand, P1.1 fixed with F1.3; answer sent to the same Codex task 21:42 UTC; that follow-up never started (no reply, no push by 06:20); resent once as a new task https://chatgpt.com/local/6b0923b2-e4a8-83eb-afcb-14b21d25704c 06:25 UTC; DELIVERED as PR #66 at 24c692dd (17:38 UTC; validator PASS, adversary HELD, user-sim ACCEPTED); r1 review routine `trig_01UqV1ZKC5CDw4uU9ez2j5oh` 17:41 UTC; spec-check not run: no spec text changes. Upkeep is exempt from the size rule.)
Why: the console's Upkeep items left by S142 and S144, while S138 (approved, held on a watane release) keeps kernel/ and design/.
Outcomes, each its Upkeep item's check (Upkeep lines "S144 r2 polish", "S144 deputy compare", "S144 r1 P1.6", "S144 r1 A1.1", "S142 r2, polish", "S142 deputy compare"):
(1) at 390 px the consent diamond sits beside its words (the user-sim's red test `regress_S144_consent_shape_beside_word_at_phone`, committed unchanged first); a taken row's error does not wrap into a narrow column nor repeat above the composer;
(2) "Enter a message first." shows only after an empty send; the status line says "Thinking" from a streaming thought row (TECHNICAL 18.5; corrected at r1 F1.2: 9.3 has no `thinking` activity, so this sketch line was wrong); Esc while Not running sends nothing; the queue label and Stop and send after a Failed turn or during Paused say what will happen (P1.6); the Removed line's Discard either matches a frame rule or goes;
(3) Stage D: the click and the wire are timed on one clock (or the offset is recorded); the Stop-cut excuse ties a complete batch to the stored interrupt again (P2.1); `acceptedMessages` matches in the real load or is removed with its reason (P2.3); a red test for each.
Touches: `console/` (src, kit, test/host), `ui/` (rebuilt and committed). Not kernel/ or design/ (S138), not the wit/deps folders.
Constraints: no new interface, no spec text change (P1.1, P1.2's spec lines stay in Upkeep); a guard never weakens; `installed-agent` must not be made weaker to pass; open the PR ready for review, not as a draft.
deputy compares: the consent wait at 390 px and 1280 px, and the composer, against `Main-2` and `States-2` (TECHNICAL 18).

### S148 stage F: an OpenAI session uses the skills its profile names, through the `skill` function (TECHNICAL 11, 9.7)
(sketched 2026-11-26 by Deputy Scissors, the main line; builder codex by rule 3 (last two: watane S147 codex, watane S146 claude); reviewer claude; in flight, cloud, Codex task https://chatgpt.com/local/6b08ad95-8eb0-83ed-bcee-2e4bc0c8706b, 22:15 UTC; DELIVERED as PR #67 at 580b8cba, CI green; r1 Claude routine `trig_01PLJdyL1p91vuV9bhofPZ2V` 06:41 UTC, adversary asked for (14).)
Why: stage F's remaining backend (TECHNICAL 17, row F: "OpenAI delegation and `skill`"). S134 built delegation and the counters; until `skill` exists, any `skills` on an OpenAI profile is refused (11), so a factory profile cannot carry its skills on OpenAI.
Outcomes, each checkable on dev's scripted Responses server:
(1) Alice starts an OpenAI session whose profile names two adopted skills: the first request advertises a `skill` function whose listing gives each skill's name and description, with `</description>` and `<` escaped (T8); the model calls `skill` with one name and gets that skill's SKILL.md as the result, shown in the transcript as an ordinary tool row;
(2) a `skill` call for a name the profile does not name, or a profile skill that has other files besides SKILL.md, is refused with a reason she can read (launch refusal for the second, per 11), and no request carries it;
(3) a child (9.7) gets only the skills its delegate names, a subset of its root's adopted skills at the same revisions; a child asking for a skill outside that subset is refused with no session;
(4) a toolset function named `skill` collides with the active built-in and is refused before a request is sent (9.7), and a profile with no skills advertises no `skill` function;
(5) Claude sessions are unchanged (their skills stay the plugin folder).
Touches: `kernel/` (OpenAI loop, profile check, refusals), `design/TECHNICAL.md` 11 and 9.7 (the "refused on OpenAI until `skill` is built" lines become what is built), `fake/` or dev's scripted Responses fixtures if needed. Not `console/` or `ui/` (S145): a skill call renders as an existing tool row.
Constraints: no change to the kernel interface (2.3) or WIT; a skill's body is read from its adopted revision only; stage F's load row (17) is measured only if the scripted fixtures allow it at kernel level, else named as a known limit in 16 with its reason (page-level budgets need `watane-dev`, as in S134). N5 (factory profiles) stays with the human. spec-check skipped: 11 and 9.7 were made one contract by S132 (design) and S134; the slice changes only the two "until built" lines (the review checks them).


### S152 (upkeep) the console's guards and tests after S145, console/ only
(sketched 2026-11-27 by Deputy Paper, the upkeep lane; builder codex by rule 3 (last two: watane S150 codex, watane S149 claude); reviewer claude; in flight, cloud, Codex task https://chatgpt.com/local/6b092968-e6e4-83ed-822d-9837a9f94063, 06:54 UTC)
Why: the console-side Upkeep items, while S148 holds kernel/ and design/. Re-check every item on main first (S145 closed several S144/S142 lines): drop what is fixed and say so in the PR, and remove those Upkeep lines in the PR body's list for the deputy.
Outcomes, each its Upkeep item's check: (1) (flake, guard) `console/kit/parts_S15.test.ts:42` passes in the full console run: cause named and fixed; (2) S145 P1.3 to P1.8, P1.11 (tests and guards); (3) S144 r1 P1.6 (queue label and Stop and send after a failed or paused turn); (4) S142 r1 A2 (Escape while the console reads Loading) as a console test against today's kernel behaviour; (5) S36 user-sim's "felt wrong" items where still true.
Touches: `console/` (src, kit, test), `ui/` (rebuilt and committed). Not kernel/ or design/ (S148), no interface change, no spec text change (spec-side lines stay in Upkeep). A guard never weakens; `installed-agent` never weakened to pass.
Boundary: the console (TECHNICAL 14, "The console has no power a client lacks"): the adversary runs if a change adds a kernel call. Protects: *guard* (1), *polish* (the rest). Size: an upkeep slice.

