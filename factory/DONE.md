
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


### S153 (upkeep) the kernel's guards and tests, kernel/ only
(sketched 2026-11-27 by Deputy Scissors, the upkeep lane after S152; builder codex by rule 3 (last two: S151 claude, S152 codex); reviewer claude; in flight, cloud, Codex task https://chatgpt.com/local/6b094528-7d2c-83ed-880b-02018fb1104a, 08:55 UTC)
Why: the kernel-side Upkeep items, while S151 holds design/. Re-check every item on main first (S138 and S148 changed the timer and skill paths): drop what is fixed and say so in the PR, item by item.
Outcomes, each its Upkeep item's check: (1) (guard, S138 r2 P2.1) a Harness step pins `run_client`'s `me() == "page"` guard on arming `page-origin` and that every successful client call drains what is owed; (2) (guard, S40 F6.1, the human's (b)) a timer firing of an OpenAI session whose owner mod was disabled during a timed pause sends no model request: through the real adapter, an enabled owner's pause resumes by itself, a disabled owner's stops with no work (check first whether S138's firing-origin work already closed it); (3) (S142 r1 A1) an interrupt before `initialized` cancels the launch, decided from the kernel's current state; (4) test polish: S127 P4.1 (`s127_judge.rs:333` asserts the replacement launch carries the original follow-up), S38 P1.2 (`s38.rs` assistant line without `session_id`), S36 r1 P1.1 (`s36_firing.rs:49, :96`).
Touches: `kernel/` (src, tests, wasm/tests/harness). Not design/ (S151) or console/; no interface change; spec-side lines stay in Upkeep. A guard never weakens; `installed-agent` never weakened to pass.
Boundary: the session owner's authority and the console (TECHNICAL 14): outcomes (1) to (3) touch the kernel's boundary files, so the adversary runs. Protects: *guard* (1) to (3), *polish* (4). Size: an upkeep slice.

- **S154 (upkeep, guard first) kernel and console items that wait on no spec change** (sketched 2026-11-27 by Deputy Rock; in flight, see In flight). Re-check each item on main first and strike what is already done. Guards first: (1) S138 r2 P2.1, a Harness step that pins `run_client`'s `me() == "page"` guard on arming `page-origin`, and that every successful client call drains what is owed; (2) the human's (b) on S40 F6.1, a timer firing of a disabled owner's OpenAI session sends no model request (read the host-resolved origin through the production adapter; an enabled owner's pause still resumes); (3) S142 r1 A1/A2 and S153 P1.3 (a): an interrupt before `initialized` cancels the launch, the cancel path closes open asks as `ended` does, and Escape while Loading is covered. Then polish, cheapest first: S152 P1.1/P1.3 (console tests), S142 r2 P2.1 to P2.3 (a visible reason beside disabled controls; the user-sim's red test is in PR #64's r2 review), S127 P4.1, S36 P1.1 and P7.1. Outcomes: each item's check passes on the real programs, and each guard test fails when its guard is removed. Touches: `kernel/`, `console/`, `factory/` tests only; never `design/` or `AGENTS.md` (S151 holds them; the spec-side items wait for it). Boundary: authority and consent (TECHNICAL 14), so the adversary runs; a Codex build hands its exploit step to the Claude review side (S153's decision). Exempt from the size rule (upkeep).
  DONE 2026-11-27: PR #71 merged (rebase) by Deputy Rock; builder codex, reviewer claude, 1 round (0/3), adversary HELD, user-sim ACCEPTED; ~0.6 h in flight. Digest: stopping a Claude launch before it initializes denies its open asks and keeps accepted answers, also on Stop and send; both Loading Stop guards are pinned; most other items were already on main (each named by its test in the review).

- **S156 (upkeep) console words and the queue against the design** (sketched 2026-11-27 by Deputy Rock; in flight, see In flight). `deputy compares` against `Main-2` and `States-2`. Re-check each item on main first and strike what is done. Items: S154 r1 P1.2 (the Stop footer counts asks as tools; "stopped", not raw "cancel"; an answer accepted on a cancelled launch must not read as if the edit ran); the S144 deputy compare (muted queue buttons, "Enter a message first." only after an empty send, no Discard beside Undo, "Thinking" for a `thinking` activity); S144 r2 P2.4 (a taken row's error wraps to ~6 lines at 390 px and repeats above the composer); the S142 deputy compare (queue rows as in `Main-2`: the QUEUED label, a dashed row per item, quiet buttons, the × with aria "Remove from queue"); the S36 felt-wrong lines (a queued row shows a whole 255 KiB message; Stop and send offered while Not running); S38 P2.2 words. Outcomes: each item visible as the frame shows it at 390 px and at desktop width, checked in Chromium; no kernel call changes. Touches: `console/` only; never `design/`, `AGENTS.md` or `kernel/` (a wording the spec owns goes in the PR body for the design lane). No boundary moves, so no adversary. Exempt from the size rule (upkeep).
  DONE 2026-11-27: PR #72 merged (rebase) by Deputy Rock; builder claude, reviewer codex, 2 rounds (2/1, 0/1); user-sim ACCEPTED, no boundary; deputy compare against Main-2: queue rows match, "Ask cancelled · <call> — the turn was stopped" just as good as the frame's words; ~0.75 h in flight. Digest: after Stop, Alice reads what ran (asks cancelled, an allowed call with no result yet reads No result, a tool count without asks), and the queue fits a phone as Main-2 draws it.

- **S157 (upkeep) kernel and console guards and polish from Upkeep.** In flight (cloud, builder codex by rule 3: the last two handed out were S156 claude and watane S155 codex; reviewer claude; Codex task https://chatgpt.com/local/6b096f2f-9f30-83ed-b4dc-5952ebae6079 11:53 UTC; DELIVERED as PR #73 at 92197141 ~12:30 UTC (+74/-1: most items found already on main, so Upkeep was stale; user-sim ACCEPTED by the builder); r1 Claude review routine `trig_01Rsqyj1ifU2WyPFdbPFwivf` 12:35 UTC, checks each already-on-main claim). Sketched 2026-11-27 by Deputy Paper.
  Re-check each item on main first and drop what is already fixed; say so in the PR. Guards first:
  1. (guard, S138 r2 P2.1) a Harness step in `kernel/wasm/tests/harness/run.sh` that pins `run_client`'s `me() == "page"` guard on arming `page-origin`, and that every successful client call drains what is owed.
  2. (guard, the human's (b) on S40 F6.1) a timer firing of an OpenAI session whose owner mod was disabled during a timed pause sends no model request: stop only foreign firings, read the host-resolved origin through the production adapter, test through the real adapter (an enabled owner's pause resumes by itself, a disabled owner's stops with no work). If watane's adapter does not expose the resolved origin of a timer firing today, say so in the PR with the line that shows it and leave this item; do not change watane.
  3. tests: S152 P1.1 (pin each Escape-while-Loading guard alone), P1.3; S156 P2.1 (the attach-format replay); S127 P4.1; S36 P1.1 and P7.1; S38 P1.2.
  4. console: S142 r2 P2.1 to P2.3 (the disabled Stop and send says why, with the user-sim's `regress_S142_r2_disabled_stop_and_send_says_why`; "Esc to stop" on the consent wait; "watane" spelling).
  5. AGENTS.md: S153 P1.2 (`harness/run.sh` needs a watane checkout and writes into its `target`).
  Each outcome is its item's check, run by the real tests. Touches: `kernel/`, `console/`, `AGENTS.md`; never `design/` (S151 holds it; spec-side items wait for the design lane). No boundary moves, so no adversary, unless item 2 changes what the kernel refuses: then the Claude review runs it. Protects: *guard* (1, 2), *polish* (3 to 5). Size: an upkeep slice.
  DONE 2026-11-27: PR #73 merged (rebase) by Deputy Paper after r1 approve (0/2) and green CI on 92197141.

- **S159 (upkeep) stage D and E checks on `watane-dev serve`, and the Stop budget on main.** In flight (cloud, Codex task https://chatgpt.com/local/6b097c13-3d20-83ed-ae82-b1ec9afa606a ~12:50 UTC; DELIVERED as PR #74 at e1fc1f05 ~13:15 UTC (+280/-25; Stop measure changed, not the budget; user-sim ACCEPTED by the builder); r1 Claude review routine `trig_01H4iNEniGe3u5toYCDT9rB9` 13:20 UTC, checks the new Stop measure). Sketched 2026-11-27 by Deputy Paper; builder codex by rule 3 (the last two handed out were watane S158 claude and S157 codex), reviewer claude.
  Re-check each item on main first and drop what is already done; say so in the PR. Guards first:
  1. (guard) the long-session Stop p95 budget fails on main (158 ms against 100, seen by Deputy Rock 2026-11-27): find why and fix the cause, or show that the budget measures something else and fix the measure. Widening the budget is a spec change: name it in the PR, do not make it.
  2. (diagnose, from S131 F1.8 and S142 r1 P1.2) the Stop race against the stage D runner's window: write the trace that tells a cut from a loss (a receipt log at the source or kernel, S144 r2 P2.3), reproduce with the controlled delay, and fix it if the cause is in the kernel or the runner.
  3. (S142 r1 P1.3) the stage D Claude source-spacing gaps (up to 57.2 ms against 40 ms): measure on the instrumented run and say whether the source, the runner or the check is off.
  4. `test:host:e`: stage E's outcomes on `watane-dev serve` with dev's scripted judge; S40 on `watane-dev serve`: turn the SKIP lines of `test:host:d` into checks and run S40 outcomes 2 to 4 with a scripted OpenAI stream.
  5. S144 r2 P2.1 and P2.2 (`metrics.mjs`: the Stop-cut excuse tied to the stored interrupt, one clock for click and wire).
  Each outcome is its item's check, run on the real programs. Touches: `kernel/`, `console/test/`, `package.json` scripts; never `design/` (S151 holds it), never watane. A budget or spec number that should change is a finding for the deputy, not an edit. No boundary moves: no adversary. Protects: *guard* (1), *core* (2 to 4), *polish* (5). Size: an upkeep slice.
  DONE 2026-11-27: PR #74 merged (rebase) by Deputy Paper after r1 approve (0/4) and green CI on e1fc1f05.
- **S160 (upkeep) the kernel's launch and Stop edges, and the console after Stop.** In flight (cloud, builder codex, reviewer claude; Codex task https://chatgpt.com/local/6b0987e8-8a20-83ed-a541-59fcc2bad032 ~14:00 UTC; DELIVERED as PR #75 at 07d1220c ~14:15 UTC (+411/-66; Stage D wrapper gap 76.6 ms against 60, some slow timing checks over budget, cancelled-Allow wording back to Upkeep); r1 Claude review routine `trig_01FRNkT3s11L9vDvFasEhdFJ` 14:17 UTC, adversary on outcome 1; r1 (~14:55 UTC): 2/4, changes (2) (validator PASS, adversary HELD with 3 risks, user-sim REJECTED 1): B1 a half message stays after any provider end not `gone`, B2 four mutants of the launch rule survive; CI green on 07d1220c; rulings (Deputy Scissors): B1 blocking (TECHNICAL 2.1 treats every cause alike), B2 land the pin, P1 accepted risk (an old key with consent across a live upgrade is interrupted, not cancelled: one line in TECHNICAL 14 Not protected once S151 leaves design/), P2 to Upkeep; answer r1 to the same Codex task ~14:58 UTC, with screenshots for the deputy compare; r1 answered at 53a94a85 (B1, B2 fixed; P1, P4 in the PR body; P3 deferred with P2); deputy compare done on the screenshots: only the partial message and half thought go, nothing else moves; r2 Claude review routine `trig_017jTt2Wn8v7EATJMrgV34Rz` 14:39 UTC). Sketched 2026-11-27 by Deputy Scissors; builder codex by rule 3 (the last two handed out were watane S158 claude and S159 codex). The code half of `Upkeep`; first strike each item already on main (the list was stale before: S157), and name it in the PR.
  1. (S153 r1 P1.3 (a) to (c), S154 r1 A1.1) a cancelled launch closes its open asks as `ended` does; the native `provider_item` path advances `last` and clears `launching`; an old-format key with consent is interrupted, not cancelled; the `launching` fallback `key.consent.is_some()` applies only to keys without `launching`.
  2. (S154 r1 P1.2, S144 r2 P2.5, S142 r1 A2) after Stop the console counts tools, not asks; the decision line reads "stopped", not raw "cancel"; an answer on a cancelled launch does not read "Allowed"; Esc while Not running sends nothing; Esc while Loading is covered against the kernel's launch rule.
  3. (S138 r2 P2.3, S38 stage B carry) owed work drains in one layer (`Kernel::read` or `run_client`); a half-streamed message does not stay shown after `gone` until a reload.
  4. (S159 r1 P1.1, P1.2, P1.4; S36 P7.1) a `test:load:d:cut` script for `--controlled-cut`; the Claude spacing report says which part is off; exact just-over-limit fixtures in `console/test/host/load_check.test.ts`.
  5. (S144 deputy compare, S142 deputy compare, S36 felt wrong; polish, cheap ones only) against `Main-2`/`States-2`: "Enter a message first." only after an empty send; no Discard beside Undo on a Removed line; a queued row offers no Stop and send while Not running; queue rows clip a long message.
  Each outcome is its item's check, run on the real programs. Touches: `kernel/`, `console/`, `package.json` scripts; never `design/` or `AGENTS.md` (S151 holds both: S157 r1 P1.1 waits), never watane. An item that needs a spec change goes back to `Upkeep` as a finding, not an edit. A kernel fix to consent state is near the consent boundary (TECHNICAL 14): the foreman decides on the adversary and says why. Protects: *guard* (1), *core* (2, 3), *polish* (4, 5). Size: an upkeep slice. `deputy compares` (5).
  DONE 2026-11-27: PR #75 merged (rebase) at 60858cee by Deputy Scissors after r2 approve (0/1) and green CI on 53a94a85; builder codex, reviewer claude, 2 rounds (2/4, 0/1); ~1 h in flight.

- **S151 (design, stage F) factory profiles** (in flight, see In flight): who writes a client's role profiles and `access/<identity>`, with whose consent** (sketched 2026-11-27 by Deputy Paper; hand out when S148 merges, since both edit `design/TECHNICAL.md`). watane step 6's JF1 cannot start a session until this exists (watane `spec/design/factory/TECHNICAL.md` 5, "Factory profiles at F"; watane evidence civic-tech-pioneer/watane#56). Settle in the design files: the writer of `profile/<name>` and `access/<identity>` for a client mod, Alice's consent for a specific profile for a specific mod identity (words she reads), its scope and withdrawal, and how a profile offer is adopted; absent entries stay denied, and a requesting mod never provisions itself. Outcomes: a reviewer can build it from the documents; each Door C choice is listed for the deputy, a consent-boundary change for the human; the build rows for it sized for slices. Touches: `design/` only. Boundary: consent (TECHNICAL 14), so the review runs the adversary on the design. Size: a design slice (it settles a Door C row before code depends on it).
S151 (design, stage F, factory profiles; builder claude by rule 3 (S152 and watane S150 were Codex), reviewer codex; routine `trig_0164LvFYR5dyVp5mpv4rc8Tm` 08:31 UTC; DELIVERED as PR #69 at 6d7f621c 08:54 UTC (+507/-33, design/ and AGENTS.md; adversary BREACHED 5, all fixed in the design); r1 Codex review https://chatgpt.com/local/6b094570-3600-83eb-a43b-8f2aace5404d 08:57 UTC; r1: 9/2 (manager shares the console's authority, identity reuse, withdrawal completion, change disclosure, capacity, version rule, offers, read budget, JF1-2 dependencies); answer r1 routine `trig_011PaH44sPkbWaFjh95e4pXJ` 09:17 UTC, rulings: F1.1 keep VISION (a trusted authorization for manage, else BLOCKED), F1.2 to F1.8 narrow first, H3 to H5 revised for the human; answers 1d15a2dc 09:42 UTC (all 11 fixed, adversary rerun BREACHED 3, fixed); r2 Codex review https://chatgpt.com/local/6b094fda-f128-83ed-a436-75138dccd040 09:43 UTC; r2: 7/1, 09:47 UTC (F1.1, F1.3 to F1.8 back, narrower); answer r2 routine `trig_01Uz9u6nG35utT9pXXavwEXb` 09:53 UTC, rulings (Deputy Rock): each narrowed to the smallest design fix, F1.6 to AGT-F-56's literal words (no restore revival); a finding back in r3 is cut; answers c7a35b62 ~10:08 UTC (all 7 + P2.1 fixed; F1.6 added a per-name rev in the index, a warning for r3); r3 Codex review https://chatgpt.com/local/6b09560d-5694-83eb-8cdf-5f0995a55056 10:10 UTC; r3: 4/0 ~10:18 UTC (new findings, all in 9.10's manager mechanism; rev kept); **cut at r3 by Deputy Rock**: F3.1 accepted limit (a supersede at the host stream cap is refused too-many, fails closed), F3.2 to F3.4 one sentence each, no new state; answer r3 routine `trig_015SW9bjc2eoviZMZFN72pre` 10:19 UTC; answers 444b9b0a ~10:30 UTC (as cut); r4 worth it (Deputy Rock): trend 9, 7, 4 and r4 checks only the four cut sentences, no new scope; r4 Codex review https://chatgpt.com/local/6b095b87-9008-83eb-89ba-91fae4f4305b ~10:32 UTC; r4: 0/0 approve ~10:30 UTC, CI green on 444b9b0a; **approved, held for the human**: merge waits on the human for S151-H, Waiting on the human)..; S138, S142, S144, S145, S148, S152, S153, S154, S156, S160 merged.
  DONE 2026-11-28: PR #69 merged (rebase) at 40fa22fa by Deputy Paper on the human's S151-H answer (accept H1 to H5; H2 import limited to `mods:agent`, written by the next design-lane slice); builder claude, reviewer codex, 4 rounds (9/2, 7/1, 4/0 cut at r3, 0/0); held ~21 h for the human.

## Moved from PLAN.md on 2026-11-28 (PLAN.md holds the present only)

### S35: stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude` (DONE 2026-11-23: PR #43 merged with --admin at 042b0dd on the human's word after the extra round, r6 changes 2 answered (F6.1 done on main, F6.2 fixed); was in flight, builder claude, reviewer codex; continuation DELIVERED 21:31 UTC at 8fc6db6 on watane c2a7d4d (routine `trig_01NpPnJxwNjH279WAFwnziDk`), screenshots in `factory/runs/S35/screens/`; r1: Codex review dispatched 21:40 UTC, https://chatgpt.com/local/6b035f75-9cbc-83eb-953f-4ea568e56065; r1: 8 Codex findings (validator RETURN, adversary BREACHED, user-sim REJECTED) plus deputy design findings D1.1, D1.2, answer dispatched 22:04 UTC (session `cse_01LxwP923RKBfVNTWsTUagAQ`); r1 answered at 4522428 (all 10 fixed); r2 (22:37): Codex `changes (6)`, validator RETURN, adversary HELD, user-sim ACCEPTED; answer r2 dispatched 23:10 UTC (session `cse_01LxwP8yPPmPVipCPszcmKqW`; a 23:00 fire carried the stale r1 brief and pushed nothing); r2 answered at ea98cb9 (all 6 fixed); r3: Codex review dispatched 23:29 UTC; r3 (23:58): Codex `changes (3)` (validator RETURN, adversary HELD, user-sim ACCEPTED); answer r3 dispatched 00:01 UTC (session `cse_01LxwP8zfZtC4o6shbfqRvp7`); r3 answered at d0dfddd (all 3 fixed); r4: Codex review dispatched 00:20 UTC; r4 (00:26): Codex `changes (3)` (validator RETURN, adversary HELD, user-sim REJECTED), all on the composer's kept draft; deputy ruling (same finding 4 rounds running) and answer r4 dispatched 00:51 UTC (session `cse_01LxwP92VSjjigxaiXQaSyiJ`); r4 answered at 8177ca5 (by the ruling); r5 (last): Codex review dispatched 01:12 UTC; r5 (01:18): `changes (1)` (validator RETURN, adversary HELD, user-sim ACCEPTED): **round cap reached, held for the human** (Waiting: S35-CAP); then the human's R-B1 before merge; first routine `trig_01XowNDWUGosFjLtAtbGFBwc`)

Story: Alice runs `watane-dev serve` with the agent kernel and the unchecked provider installed,
opens the console, starts a session in a scratch workspace (4.6, no consent steps), types a
message and watches the answer stream in from dev's fake `claude`: text deltas, folded thinking,
finished entries, the status line Working then Idle. She reloads and sees the same transcript,
nothing doubled. One story in two repositories, handed out together: watane S99 is the host half
(H2 `claude-code.start` / `claude-session.say` with the unchecked path and
`allowUnsandboxedCommands: false`, `host-info.generation` as S97 writes it, dev's fake `claude`,
pkg). Interface, named the same in both sketches: the `watane:host@0.1.0` H2 and `host-info`
additions of TECHNICAL 2.1 as watane S97/S99 write them in host 19.
Outcomes:
1. S32 R1 first: a kernel call that fails commits none of its writes on watane's host (a test that
   fails today).
2. The unchecked provider (10) starts a fake `claude` session and hands its stream to the kernel;
   the kernel runs the launch and one `say` (9.3), items through the adapters (9.8, R2) into the
   transcript.
3. The console shows the turn live and after a reload; the status line follows the turn (fixes
   the S32 "felt wrong" Idle line).
4. The stage B cases of 17 that a single turn covers (5, 6, 8 to 12, 58 reattach part) pass on
   watane's host.
Carry list items folded in: R1, R2, S34 RISK 1 (the first `workspace`), the felt-wrong list.
Touches: `kernel/`, `provider/` (see Waiting: its own repository), `wit/`, `mod.toml`, `console/`,
`ui/`, `design/TECHNICAL.md` (2.1, 4.1, 4.6, the 15 row 4.6 names). Fakes nothing once S99
merges; until then it builds against S99's branch. Protects: *core*. `PO arms`. Cloud.

### S37: spike: what can enforce a profile's local-network setting for the shell grant (V27) (DONE 2026-11-23: report read, PR #48 closed unmerged, answer under V27; was in flight, cloud, builder codex, no review round (spike: the deputy reads it, closes the PR unmerged, and puts the answer under V27); Codex task https://chatgpt.com/local/6b03eab5-4b88-83eb-8f84-d9cc7bb8606e, 2026-11-23 ~07:15 UTC)

Question: which mechanisms can enforce a profile setting "local network access: denied (default) / allowed" for
the shell grant, on macOS and Linux? Local network means loopback (127.0.0.0/8, ::1), private ranges (10/8,
172.16/12, 192.168/16, fc00::/7), link-local (169.254/16, fe80::/10) and the metadata address 169.254.169.254.
Candidates: (a) the kernel refuses allowlist entries at profile check (`localhost`, `*.localhost`, IP literals in
those ranges); (b) Claude Code's sandbox settings (what its network proxy and `allowedDomains` can and cannot
refuse, per platform: Seatbelt on macOS, bubblewrap on Linux); (c) a watane host egress proxy in front of the
sandbox that checks the resolved address at connect time; (d) anything else found (OS firewall rules per
process, a resolver that refuses such answers). For each: what it stops and what it doesn't (DNS rebinding,
names that resolve to private addresses, IPv6 and IPv4-mapped forms, the metadata address, a command that
bypasses the proxy), its cost, and where it lives (kernel, host, Claude config). Probe on the Mac and in a Linux
container where a claim can be tested; mark each claim probed or from docs.
Outcome: `design/SPIKE-local-network.md`, a write-up with a recommendation and the probe scripts; no product
code. The human decides V27 from it. Touches: `design/SPIKE-local-network.md`, `probes/` only. Builder codex,
reviewer claude. Cloud.


## Done

- **S159** (upkeep) Stop in a long session is measured from the click to the visible "Stopping…" with the interrupt sent, under the unchanged 100 ms (main's 158 ms was the test's own lookups); stage D can tell a Stop cut from a lost batch by sink receipts; Claude spacing is stamped at both ends. PR #74 rebase-merged by Deputy Paper as 0eedee04; builder codex, reviewer claude, 1 round (0/4); user-sim ACCEPTED, no boundary; ~1 h in flight.
- **S157** (upkeep) a stopped session's replay is pinned (Done card, one tool in the footer) and AGENTS.md says where the Harness checkout and build go; the other sketched guards and tests were already on main, each checked by the review and struck from Upkeep. PR #73 rebase-merged by Deputy Paper as 7f154bff; builder codex, reviewer claude, 1 round (0/2); user-sim ACCEPTED, no boundary; ~1 h in flight.
- **S153** (upkeep) the kernel's guards: the page-origin arming guard and the drain of owed work are pinned on the Harness, an interrupt before `initialized` cancels the launch (decided from the current stream key), and S127 P4.1 asserts the follow-up; S40 F6.1, S38 P1.2 and S36 P1.1 were already closed on main. PR #70 rebase-merged 2026-11-27 09:31 UTC at 79e560bd by Deputy Scissors, no bypass; builder codex (BLOCKED once: declined the adversary's exploit step, see Deputy decisions), reviewer claude, 1 round (0/3); adversary HELD (run by the Claude review); ~40 min in flight.
- **S152** (upkeep) the console's guards after S145: three Stop races and Escape while Loading pinned, empty Send says why, the Removed live region no longer repeats Undo, the `parts_S15` flake fixed (on main as 140fc3a, S144 r1 F1.2). PR #68 rebase-merged 2026-11-27 08:53 UTC at 857842fd by Deputy Scissors, no bypass; builder codex, reviewer claude, 1 round (0/3) after one CI answer (F0.ci: its own test raced CI, fixed with a wait); adversary not run (no kernel call, reviewer checked); ~1.6 h in flight.
- **S148** (stage F) an OpenAI session uses the skills its profile names through the `skill` function; a skill description over 1024 characters is refused at check; a near-limit child admission's state cost is a stated limit (TECHNICAL 16). PR #67 rebase-merged 2026-11-27 08:28 UTC at 546b5553 by Deputy Scissors, no bypass; builder codex, reviewer claude, 3 rounds (1/8, 2/4, 0/1); cut at r2 by Deputy Paper (F2.1 bound, F2.2 accepted); user-sim ACCEPTED, adversary HELD; ~2 h in flight.
- **S145** (upkeep) at 390 px the consent diamond sits beside its words, the composer Escape no longer interrupts during IME composition or on key repeat, "Thinking" comes only from a streaming thought row, and Stage D times click and wire on one clock. PR #66 rebase-merged 2026-11-27 ~07:00 UTC at eed55ab7 by Deputy Paper, no bypass; builder codex, reviewer claude, 2 rounds (3/12, 0/0); ~14 h in flight, most of it the Mac asleep. User-sim ACCEPTED; adversary BREACHED 3 (F1.3, P1.1 fixed; P1.2 accepted to Upkeep, already on main). Deputy compared the consent wait at 390/1280 px with States-2: the remaining differences (wording, Esc hint, disabled Open the question) are on main from S142/S144 and sit in Upkeep as P1.9/P1.10. Accepted risks A1.1 (a stale-view Esc may interrupt the next turn: a turn-scoped interrupt is a 2.3 backend decision), A1.2 (stage_d `to = min(clicks)`).
- **S138** the kernel acts on a timer firing only under its session owner's authority (consumer half of watane S133/S136; F6.1): a timer firing reads its resolved origin with `timers.firing-origin()` and stops a foreign firing; a page session resumes at its reset (D001 O2). PR #61 squash-merged by Deputy Paper 2026-11-26 21:40 UTC (624eba83, no bypass); builder codex, reviewer claude, 4 rounds (3/?, 0/4, 1/0, 0/0); in flight since 11:22 UTC (~10 h, of which ~5 h held on a release it did not need: `installed-agent` pins watane by commit; the pin moved a923a8dc -> 44422b22 and one consent string followed the newer host). r1 adversary BREACHED 1 (F1.2, liveness), outcome check REJECTED 1 (F1.1), both fixed in r1 answers and confirmed in r2. Friction: a review routine that waits with ScheduleWakeup never wakes (r4 re-run as r4b).
- **S144** (upkeep) the queue rows look as `Main-2` draws them (label, dashed rows, quiet buttons, × "Remove from queue", Removed/Undo), a long message shows three lines, Stop and send is not offered while nothing runs, every disabled control says why (visibly and to a screen reader), the consent wait shows "Esc to stop", `parts_S15` is a fixed cause, load fixtures sit just over their limits, and Stage D's Stop-cut excuse is bounded by the click and by stored admission. PR #65 rebase-merged by Deputy Paper 2026-11-26 (c217b8bd, no bypass; Codex had opened it as a draft); builder codex, reviewer claude, 2 rounds (2/7, 0/5); ~1.2 h in flight. Adversary BREACHED 1 in r1 (the Stage D check), HELD in r2; user-sim ACCEPTED (r2); deputy compared the queue rows and the consent wait (polish in Upkeep). A1.1 to A1.3 accepted. Friction: CI gate's third 15-min timeout (a factory proposal); `review-lead.md` missing (fifth sighting); a Codex PR opened as a draft blocks the merge until marked ready.
- **S142** (upkeep) Stop and Escape during a folder-consent wait cancel the launch instead of interrupting, and a queued row's Stop and send is disabled there, so a later yes never feeds Claude after a Stop; the wait shows its folder; "Open the question" is disabled with a reason until a host/page interface exists; stage E runs on a real host by hand (`npm run test:host:e`, AGENTS.md); stage D's lost item is traced to a Stop race against the runner's window (Upkeep). PR #64 rebase-merged by Deputy Paper 2026-11-26 (deac873d, no bypass); builder codex, reviewer claude, 2 rounds (1/5, 0/4); ~1.1 h in flight. Adversary HELD (r1), user-sim ACCEPTED (r2), deputy compared the consent wait and queue rows (differences in Upkeep). A1 to A4 accepted (Deputy Paper). Friction: `review-lead.md` still missing (fourth sighting).
- **S140** (upkeep) the console's known gaps and test polish: a red `installed-agent` or stage C run says what it checked and why (the whole grant list, the old flake's routing race, NOT RUN with exit 2 for a missing binary); the queue preview keeps 240 code points; a folder with a newline or tab reads as itself in the consent wait; the release script uses `node -e`; `provider/BOUNDARIES.md` names its commit (PR #63, rebase-merged 2026-11-26 14:29 UTC by Deputy Rock, no bypass; builder claude, reviewer codex, 2 rounds 1/2, 0/0; ~2.2 h in flight). Items 2, 3, 5 were already done by S139. User-sim ACCEPTED; adversary not run (a copy and an equivalent JSON read). Friction: CI `gate` timed out at 15 min once (second sighting); the review lead did not run user-sim until asked; `review-lead.md` missing. Sketch in factory/DONE.md.
- **S131** (upkeep) the host tests check what they skip: stage C runs on a participant home with a fake Confirm and its budgets are measured; the expiry check requires deny by host for `expired` at or after the deadline; S128/S36/S38 test polish done; the console's folder name in consent is a named KNOWN GAP (Upkeep). PR #58, squash-merged 2026-11-26 ~08:58 UTC at 05cda43 by Deputy Paper, no bypass; builder claude, reviewer codex, 3 rounds (8/2, 1/1, 0/0); ruled by Deputy Rock at r1 (F1.1/P1.1 sketch amended, F1.2 a known gap, F1.7/F1.8 narrowed); boundary HELD; Codex could not run the host commands (no watane-dev), judged from the builder's red-first runs; ~2.5 h in flight. Sketch in factory/DONE.md.
- **S132** (design) stage F, delegation and counters, made one consistent contract: AGT-J5 runs on an OpenAI root with two sibling children (depth 1); Claude keeps its launch-time subagents; `task` returns one terminal result, a child shows as an indexed `child` entry; counters per turn; §14 child admission spelled out (PR #59, rebase-merged 2026-11-26 ~08:30 UTC at 95ec3bc7 by Deputy Rock, no bypass; builder codex, reviewer claude, 1 round 0/12; ~25 min in flight). **Door C, reported to the human for veto:** 9 choices in PR #59's body; two narrow a FUNCTIONAL promise: AGT-F-36 counters per turn instead of per session, and Claude refuses `max-tokens` and `max-tool-errors` (only its native `max-turns`); both fail closed. Polish P1.1 to P1.12 ride with S134's brief. Sketch in factory/DONE.md.
- **S127** (stage E) Alice's Claude session runs through the checked provider with Jev as its judge: the console shows `checking` before each tool call, a safe call goes on, an unsafe or unsure one is denied with a reason she can read, and repeated denials trip the breaker, which says why (PR #56, rebase-merged 2026-11-26 07:57 UTC at 8e92d1b4 by Deputy Rock, no bypass; builder claude, reviewer codex, 4 rounds 4/2, 2/1, 1/1, 0/1; ~13 h in flight). Cut at r2 (deputy-7): F2.1 a known limit in TECHNICAL 10 (a verdict's reason can be lost under backpressure at teardown); r3's F3.1 fixed on existing state (TECHNICAL 9.4). Boundary HELD, outcomes ACCEPTED on fakes. Codex's cyber filter refused the adversary twice in r1 (worded as a boundary check since). Sketch in factory/DONE.md.
- **S128** (upkeep) the `installed-agent` flake is a fixed race (the test now waits for the install to finish routing), `kit/s7_fix1` counts row reads instead of timing them, S40 runs on `watane-dev serve`, and `test:load:d` measures the stage D budgets at the page with a Budget record row (PR #57, rebase-merged 2026-11-25 at a19a90f3 by deputy-6, no bypass; builder codex, reviewer claude, 2 rounds (1/9, 0/6); ~1.3 h in flight). Cut: F6.1 and case 22 stay known limits (watane timer-origin contract held, watane PLAN Unsettled). P1.2 accepted (~1 s first-request delay). Polish in Upkeep. Sketch in factory/DONE.md.
- **S40** Alice runs a profile on her ChatGPT plan in the console: the kernel refuses what OpenAI cannot honour and names each field, an OpenAI session streams the same events as Claude, and a usage limit pauses it until the reset and resumes by itself (PR #54, squash-merged 2026-11-25 ~18:45 UTC at b494cc34 by deputy-6, no bypass; builder claude, reviewer codex, 7 rounds; r7 reverted to the reviewed 553c95f plus one 9.5 line, checked mechanically; ~24 h in flight). F6.1 a known limit (the human's (b); Upkeep guard). CI: `installed-agent` red twice on the doc-only head with the known 'Timed out waiting for Claude Code settings' flake (not required; green on 553c95f). Sketch in factory/DONE.md.
- **S39** the agent mod installs and runs in the watane app (PR #53, merged 2026-11-24 ~20:00 UTC without the admin bypass at 893f14a3, the first merge under the changed ruleset; builder codex, reviewer claude, 2 rounds 0/5, 0/0; ~7.5 h in flight, of which ~3 h waiting for watane S103 under the new 'dependent slices do not wait' rule). One command builds and publishes the agent mod and the Claude Code provider; one consent installs both from a link in the app; the launcher opens the console, which streams, asks, stops and keeps its transcript across a restart (CI `installed-agent` on watane main). Outcome 4, the human's real Claude login on a Mac: commands in the PR body. Risks A1.1, A1.2 accepted; polish under Upkeep.

- **S38** stage C consumer half (PR #52, merged 2026-11-24 ~09:45 UTC with `--admin` on the human's word, main 499c254; builder codex, reviewer claude, 4 rounds 2/8, 1/7, 1/2, 0/0; ~1.9 h in flight). Alice starts an agent session in a folder she picks; the console shows the consent wait, names a refusal and a forbidden folder, and a withdrawal ends the session with its transcript kept (shown as 'Connection to Claude Code ended', R1.1 accepted); tested against a double of watane host 19.7 until S103 merges. Decisions: Q38 left to the human (the spec cites 19.7), R1.1, R2.2 accepted. Friction: update+run of a routine fired a stale brief.

- **S32** stage A end to end (PR #41, merged 2026-11-22 ~14:10 UTC by the PO with `--admin`; 2026-11-21 08:59-10:47,
  then 2026-11-22 12:06-13:42 on watane's real host, and a 15-min fix after the PO's comparison). Alice runs
  `watane-dev serve` (watane 4f105a9) with the agent kernel and watches a real kernel stream's numbered items in
  the console, live, after a reload, a host restart and `kill -9`, nothing doubled; a refused send keeps her text
  and says why (18.6 "Send"); Deny… opens its reason form in view (18.6 "Asks"). `npm run test:host` (by hand):
  7 pass, 2 honest skips (case 7 and 36's defer half need a provider stream: stage B). User-sim ACCEPTED (real-host
  run 2); adversary HELD (one earlier BREACH fixed). World stays staged in `kernel/wasm/wit/` until a watane release
  with host 19 (CI installs v0.1.0). Gate 55 s; kernel 459 tests 3.1 s; console 496 tests; `test:slow` 28 s.
  Friction: no IPv6 loopback in the cloud (patched host for the run); SessionStart installs watane-dev v0.1.0 (12 min
  builds by hand); the rebase dropped a DELIVERED commit. Felt wrong, to S35/S36: probe composer invites a message,
  raw error wording in the alert, reload drops typed text, blank flash after reload, new port per `serve` restart,
  `serve --stop` deletes preview data, "(18.8)" shown to the person.
- **S2** the agent layer's contract (PR #3, merged 2026-11-20 22:02 UTC, ~20 min in flight). A
  foreman now reads VISION, FUNCTIONAL and TECHNICAL as contracts: N1-N4, N6-N11 and R1 are
  rules; N5 open until stage F; TECHNICAL 14 names files per boundary; 15 is a watane checklist;
  17 names the repository per stage; AGENTS.md exists. Adversary BREACHED 2 (any mod could drive
  the checked provider; boundary files missing), fixed, HELD. User-sim REJECTED twice on a gap
  older than the slice (how a launch gets a scratch workspace id); named in TECHNICAL 4.6 as a
  precondition of stage B's first slice, accepted by the PO. Implementer runs 1/1/2/1. No suite.

- **S5** the performance review's must-dos in the contract (PR #5, merged 2026-11-20 22:37 UTC,
  ~1.5 h). A foreman of stage A, B or D now finds a bound for every size and wait: 256 KiB entries
  and inputs (larger entries as lossless `part`s), `queue/<id>`, a 512 KiB session record,
  `too-large(u64)`; `stream-sink.items` returns `taken | defer` with a lag clock; slots 8/8/16 with
  `busy`; a trim mark; `read` pages backwards (`before`); budgets per stage in 17; cases 34-53.
  TECHNICAL 10: the checked provider judges only after `initialized`, which waits on the workspace
  consent. User-sim ACCEPTED on its 4th run; adversary BREACHED 3, fixed, HELD twice. Implementer
  runs 1/1/1. No suite (gate: links, under 1 s).
- **S3** the kernel's core (PR #6, merged 2026-11-20 22:49 UTC, ~1 h). A client mod's maker runs
  `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile> <overlay> <backend>
  <model>` and sees every refusal at once. Crate `agent-kernel` (serde only): errors (12),
  profiles and grammar (4.1, 4.2, 9.2), narrowing overlays, the refuse table (11). `factory/gate.sh`
  is the gate. User-sim ACCEPTED; adversary BREACHED 6 (Bash allow rules, skill frontmatter), all
  fixed with tests in `kernel/tests/it/adversary.rs`, HELD on pass 7. Implementer runs: 3 tasks,
  6 fix tasks. Suite: 57 tests, CI gate job 17 s.

- **S4** the console on a scripted session (PR #8, merged 2026-11-20 23:26 UTC, ~1.7 h). Alice
  runs `npm run dev` in `console/` and sees a session on a scripted fake kernel: her message,
  folded thinking, "Explored · n reads", an edit card with its diff, an open ask with its bar,
  "Needs you (1)", the status line counting down, the plan, the queue; a reload doubles nothing.
  assistant-ui's External-store runtime over the console's own reducer (`console/kit/`).
  User-sim ACCEPTED on run 2; adversary HELD after 5 fix rounds (one governor of 8 burst, 4/s
  over attach and read). Merged before the PO's side-by-side check: the PO's comparison with
  `Main-2` found the gaps that became S9. Implementer runs 2/1/1 + fixes 1/1/1/1/2/1. Suite:
  `npm test` 167 tests 6.2 s, gate 24 s, `npm run test:slow` 7.5 s.

- **S8** the kernel's adapters (PR #10, opened and armed by the PO, merged 2026-11-20 23:42 UTC;
  19 min in flight). `cargo test` in `kernel/` turns each row of 9.8 into its ACP v1 update and entry from
  golden fixtures; unknown lines become `unknown`, never an error. Real libraries: claude-codes
  =2.1.330, agent-client-protocol-schema =1.20.0. Adversary HELD with a RISK on 4.5 (PO decision
  below); user-sim ACCEPTED. Implementer runs 1/1. Suite: 66 tests 0.23 s; gate 13 s.

- **S6** the review's should-dos and S3's spec debts (PR #11, merged 2026-11-20 23:45 UTC, 21 min in flight). A foreman of
  stage B or D finds P07-P12 as rules with cases 54-59: active history from the newest compaction
  (P07), 64 KiB transcript chunks (P08), `stream/<id>` and `live/<session>` keys (P09), one timer
  per session, 240 kernel-wide and 120 per owner (P10), host byte budgets 512/128/16 MiB (P11),
  kept keys charged to their owner in a 224 MiB account (P12); real kernel files in 1 and 14;
  4.1 names every refused first word and runtime prefix; `ask-expiry` in seconds. User-sim
  ACCEPTED on run 2; adversary BREACHED 2 (the refused-word list), HELD on pass 3. Implementer
  runs 1/1/1. Gate 14 s, no tests changed. Code to change, for the next kernel slice: drop
  `yarn ls`, add the missing runner names and prefixes, the frontmatter `:` rule.

- **S11** the transcript store (PR #12, opened and armed by the PO after its BLOCKED, merged 2026-11-21 00:11 UTC; ~45 min).
  A kernel maker sees transcripts of 10 and 10 000 entries stored in 64 KiB chunks over an
  in-memory host-7 state with counted gets; entries over 256 KiB as UTF-8-safe `part`s joined byte
  for byte; `read` both ways under 256 entries and 2 MiB; attach's replay decision (128 / 2 MiB,
  else `behind`); `unreadable` keeps its `i`; `Bash(yarn ls)` refused. User-sim ACCEPTED (run 1);
  adversary BREACHED 1 (`read(u64::MAX)` overflow), fixed, HELD.

- **S10** the console's wire and S8's 4.5 rules in the contract (PR #13, armed by the PO, merged 2026-11-21 00:14 UTC; 24 min).
  A stage B kernel maker builds every stream message and entry body the console reads as JSON:
  `answer` takes four decision shapes (others refused), the turn JSON per state, 9.4 "The wire";
  4.5: a `control_request` without a string `tool_use_id` is denied, and a stdout line typed like
  any watane item (or not one UTF-8 JSON object) goes out as `{"type":"wrapped"}`; 9.8 "Give
  nothing"; cases 60-65; AGENTS.md tests made true. User-sim ACCEPTED on run 2; adversary BREACHED
  4 then 1, fixed, HELD on pass 3. Implementer runs 1 each of 5 tasks. Gate 20 s.

- **S13** S11's spec debts (PR #14, merged 2026-11-21 00:25 UTC, 5 min in flight). 9.4: only `attach` moves `after` to a
  group's first part, `read` takes it as given (case 34); 2.3: `max` 1 to 256, 0 refused, over 256
  read as 256; 7: "A call commits whole"; 9.4: `unreadable` keeps the `i` of its place, and "Who
  makes entries". User-sim ACCEPTED (run 1); no adversary (no code, 14 unchanged). Gate 16 s.

- **S12** the kernel catches up with the spec (PR #15, merged 2026-11-21 00:36 UTC, 19 min). `RUNNERS` equals 4.1's 98 words
  (a test reads 4.1) plus runtime prefixes; frontmatter `:` rule; `unreadable` at its own place;
  `append` takes only whole-entry kinds; `read` refuses `max` 0 with field `max`; the Claude
  adapter builds the 9.4 ask from the item (`expires-at` from the item, no `options`), writes
  `ask-resolved` expiry as a decision entry, `thought` on a message, `_meta.watane.at` on every
  update. User-sim ACCEPTED (run 1); adversary HELD (pass 1) on profile, transcript and ask
  boundaries, with RISKs folded into S14 and S16. Implementer runs 1/1/1. Kernel 102 tests; gate
  26 s.

- **S9** the console matches the frames (PR #16, compared and armed by the PO, merged 2026-11-21 00:45 UTC; ~1.2 h). Alice's
  console now reads like canvas Revision 2: the edit ask as one card (pencil, path, "Waiting for
  you", Input, a tinted folded diff, "Allow this edit?" with Deny… / Allow edits in `<reach>` this
  session / Allow once), the status line with subject and "denied in m:ss if unanswered", the
  Needs you pill, the frame's composer with Stop and the model line, header chips with Context %,
  the grouped "Agents" sidebar, chevrons on folds; the console flake fixed (`ready()` waits for
  every row kind, 20/20). PO check at 1440 and 760 px, light and dark: matches; the ask's full
  Input above the diff departs from the frame by TECHNICAL 14 (the section holds over a frame).
  User-sim ACCEPTED on run 3; adversary BREACHED 4 (the ask card could hide what runs), fixed,
  HELD. Implementer runs 1/1/2 + 8 fixes of 1. Suite: `npm test` 230 tests 7.8 s (+26%),
  `test:slow` 23 s, gate 45 s.

- **S16** S12's spec debts (PR #17, merged 2026-11-21 00:47 UTC, ~7 min). The done "Code to change" lists are gone; what S14
  still owes is named once, in 9.4 "Code to change (S14)", with case 66; 4.5: a host ask or check
  id is unique on its stream, never reused; 9.8: an item without `input` or with an open id is
  `unknown`; 9.4: `raw` as a JSON string, every line takes the `i` of its place, a repeated
  thinking block counts once. User-sim ACCEPTED (run 1); no adversary (no code, 14 unchanged).
  Gate 15 s.

- **S14** the kernel finishes the wire (PR #18, merged 2026-11-21 01:03 UTC, 22 min). Strict line reading (non-UTF-8 is
  `unknown`, bytes as `\xNN`); a duplicate top-level key is `unknown`; every line `read` or replay
  gives takes the `i` of its place, joined groups too; an ask or check without `input`, with a
  reused open id, or with a non-string id is `unknown`; a repeated thinking block is not pushed
  again. User-sim ACCEPTED on run 2; adversary BREACHED 3 then 1, fixed, HELD on passes 3 and 4
  (ask and privacy boundaries). Implementer runs 1/1/2/1/1/1. Kernel 111 tests; gate 18 s.

- **S17** the restart pass in the contract (PR #20, merged 2026-11-21 01:08 UTC, 15 min). A stage B kernel foreman finds:
  a host boot number `caller.boot()` (2.1, a watane need in 15); `active/<n>` (4 pages, at most
  1 024 sessions, else `busy`) of running, waiting and paused sessions; a lazy restart pass of at
  most `A + 1` calls, one session each, no key listing; one outcome per state (Claude running or
  waiting: asks denied `restart`, turn `unknown`; paused unchanged; OpenAI re-queued or kept);
  `say` on `unknown` resumes; cases 67-71. User-sim ACCEPTED on run 2; no adversary (no code, 14
  unchanged). Gate 17 s.

- **S19** the kernel fails closed on S14's open points (PR #21, merged 2026-11-21 01:21 UTC, 12 min). An `ask-resolved` with a
  `by` other than `expired` or `cancelled` is `unknown` and the ask stays open; an ask or check
  whose `input` is not an object or whose `expires-at` is not a u64 integer is `unknown`; a joined
  whole of kind `part` is `unreadable` (dropped when its last part has no whole `i`). User-sim
  ACCEPTED (run 1); adversary HELD twice (one RISK fixed in S19-fix1). Implementer runs 1/1/1.
  Kernel 116 tests in 1.63 s.

- **S18** spec text for S14 and S19 (PR #22, merged 2026-11-21 01:30 UTC, 16 min). 4.5: `ask-resolved` `by` is `answered`
  (with `outcome`), `expired` or `cancelled`; the host serializes every item itself. 9.8: strict
  item rules 1-6. 14 "Not protected": a first-key JSON reader, a nested duplicate key in `input`.
  AGT-F-10: "never show a turn as running when nothing drives it". Case 72. User-sim ACCEPTED
  (run 1); adversary BREACHED 2 (`answered` closing a provider-denied ask with no decision;
  `allow` on a check), fixed, HELD on pass 3. Gate 18 s.

- **S15** the console reads the wire S10 wrote (PR #23, compared and armed by the PO, merged 2026-11-21 01:35 UTC; 44 min).
  Alice's console reads S10's kernel JSON unchanged: `expires-at`, `answer` by the ask's `id`,
  `part` groups joined across pages, replay and live, checks with no buttons, the ask card from
  the ask entry only (each entry keeps its own call); S9's "felt wrong" list fixed. PO check at
  1440 and 760 px: look unchanged from S9, Allow once at the end of the bar. User-sim ACCEPTED
  (run 1); adversary BREACHED 1 (two entries on one call id), fixed, HELD over 4 000 random
  sequences; S9's held cases kept. Implementer runs 1 each of 5. Suite: `npm test` 280 tests
  10.0 s (+19%), `test:slow` 30 s, gate 21 s.

- **S22** one decision per ask in the record (PR #24, merged 2026-11-21 01:51 UTC, 14 min). A Claude ask's decision is written
  once, when it closes: the kernel's own answer is a pending answer until `answered`, and an
  outcome that differs (an unsafe verdict after an owner's allow) gives one `deny, by: checked`;
  the host closes a check with `expired` too (watane, 15); `ended` closes every open or pending
  ask; `pending-answers` capped at 64; a forging checked provider named in 14 (record only);
  case 73. User-sim ACCEPTED (run 1); adversary HELD twice (RISKs 1-5 fixed in S22-fix1).

- **S21** sessions, turns, the active index and the restart pass (PR #25, merged 2026-11-21 02:12 UTC, 46 min). A kernel maker
  drives sessions through 9.3 on `MemState` with a `FakeHost` (`caller.boot()`, timers, provider
  requests as recorded calls): start, say, queue, unqueue, answer, rename, delete, list; the
  active index; the restart pass, each call bounded; cases 67, 68, 69 (Claude), 71. `start`
  checks `access/<identity>`, failing closed. User-sim ACCEPTED on run 2 (Continue with a full
  queue); adversary BREACHED 2 (queue bytes counted before escaping; `start` ignored `access/`),
  fixed, HELD twice. Implementer runs 1 each of 7. Kernel 179 tests in 3.18 s; gate 22 s.

- **S24** S21's spec debts (PR #26, armed by the PO, merged 2026-11-21 02:29 UTC; 12 min). 9.3 gains `open`; 9.1 the formats of
  `active/`, `queue/`, `access/`, `key/` and `owner/`; `access/` is reread beyond `start` (say,
  interrupt, R5, answer allow, a turn's end), failing closed, with an unlisted name refused the
  same whether or not the profile exists; an OpenAI profile needs `model`; a child session needs
  no `access/` entry; revoking access does not stop a running turn (14); case 74. User-sim
  ACCEPTED (run 1); adversary HELD twice, 7 RISKs settled in the text.

- **S20** asks as the kernel records and shows them (PR #27, merged 2026-11-21 03:12 UTC, 37 min). One decision entry per
  Claude ask, written at close; pending answers in the record; `answered`, `expired`, `cancelled`
  decided against the record; `ended`, `cancel` and R4 close every open or pending ask, at most 64
  per session (a 65th denied `busy`); ask cards carry an ACP `kind` and, for edits, a diff from the
  input (none with `replace_all`); a second ask on a `toolCallId` with another tool or input is
  `unknown`; `interrupt` and `cancel`; `access/` reread by every call that gives a session more
  work. User-sim ACCEPTED (run 1); adversary BREACHED 3 then 1 (a reused id from a second stream
  or a reload; `MultiEdit` with `replace_all`), fixed, HELD. Implementer runs 2/1/1/1/1. Kernel 227
  tests in 3.9 s; gate 18 s.

- **S25** quotas (PR #28, merged 2026-11-21 04:04 UTC, ~48 min). Owners are held to 9.3 "Quota": 64 sessions and 64 MiB per
  owner (settable), the 224 MiB account and 32 MiB reserve, more owner pages past 256, `quota`
  refusals naming their cap, a running turn ends `stopped {limit: quota}`, kept keys stay charged;
  a key costs its name plus its value, metered per call; past quota a new ask is denied and the
  turn interrupted. User-sim ACCEPTED on run 2 (refusal text named the wrong cap); adversary HELD
  twice. Implementer runs 1/1/1. Kernel 262 tests in 3.67 s; gate 21 s.

- **S7** the console stays fast on a long session (PR #29, compared and armed by the PO, merged 2026-11-21 04:41 UTC; ~3 h).
  Alice opens a running 5 000-entry session and scrolls, types and stops without lag: about 35
  rows mounted, one render per frame, keystroke p50 ~30 ms, Stop 27 ms, code highlighted (Shiki,
  lazy chunks) once complete and visible; the reducer's cost per message is flat; `test:slow`
  builds once; the transcript does not follow the end while focus is in an open ask's card, with a
  "New activity below" pill. PO check: look unchanged from S15 at 1440 px; the long session in dark
  renders right. User-sim ACCEPTED on run 2 (page scrolled; `npm test` over budget); adversary
  BREACHED 2 (the stream carried a focused ask card out of view), fixed, HELD; earlier held cases
  kept. Implementer runs 1 each, fix5 2. Suite: long fixture in `npm test`; budget 14.5 s.

- **S27** the console's rules after S7 in the contract (PR #30, merged 2026-11-21 04:53 UTC, ~8 min). 18.2: the reducer's view
  is what the transcript draws, the runtime gets the newest 100 rows; 18.6 Place: no follow while
  focus is in an open ask's card, Needs you ends it, the pill resumes it, a pointer press on the
  scrollbar or the card's text does not count as leaving (S23 builds it); 18.6 "Long sessions":
  viewport ± 800 px, at most 100 rows plus focused and open-ask rows, one render per frame, code
  highlighted once complete and visible with a per-block budget (S23 sets it). User-sim ACCEPTED
  (run 1); no adversary (no code, 14 unchanged).

- **S26** the kernel tracks its provider streams (PR #31, merged 2026-11-21 05:30 UTC, 1.3 h). A kernel maker drives a Claude
  session through a fake provider stream: `stream/<id>` and the record's `streams`, 8 provider
  slots (4 per owner, `busy`, case 52), the 10-minute idle release through `s/<id>`, resume with
  `resume: true` after a release, `ended`, `cancel` or restart (not for `persist: false`),
  `attach` with `live/<session>`, deferral rolled back whole and applied once, `lagging` and
  `replaced` closes resuming the provider, a delta reads only two keys (P09); S25's three items.
  User-sim ACCEPTED on run 2 (a deferred provider stayed deferred after a kernel close);
  adversary BREACHED 1 then 1 (past-quota `turn` entries), fixed, HELD. Implementer runs 1 each
  of 7.

- **S28** no session is left running with nothing driving it (PR #32, merged 2026-11-21 06:07 UTC, ~35 min). A provider
  `ended` mid-turn closes the open asks and ends the turn `failed {provider-ended}`; the next `say`
  resumes the Claude session; the console shows "Try again". `ended` never fails on quota (charged
  past the ceiling, as the restart firing). The past-quota flag is a `Past` value only
  `Past::load` makes. Case 76. User-sim ACCEPTED (run 1); adversary BREACHED 1 (the new `turn`
  entry made `ended` fail at the ceiling, leaving the session running), fixed, HELD. Implementer
  runs 1/1/1. Kernel 342 tests in 3.8 s; gate 28 s.

- **S29** ending a turn never fails at the ceiling (PR #33, merged 2026-11-21 06:51 UTC, 0.6 h). At and past the ceiling,
  `cancel`, `interrupt` without a replacement, provider items while past and `ended` succeed and
  free what they hold, charged into the reserve; new work is refused `quota`. Past quota a decision
  is the short form `{id, kind, ask-i, decision}` (counted at each write), so a closing call with 64
  open asks of 300 KiB writes 9.4 KB; case 77. User-sim ACCEPTED (run 1); adversary HELD, BREACHED 1
  (a pending id's decided ask never closed, so repeats stored without limit), fixed, HELD.
  Implementer runs 1 each of 4. Kernel 369 tests in 3.9 s; gate 29 s.

- **S31** the broker (PR #34, merged 2026-11-21 07:31 UTC, 0.6 h). Session allows answer later
  matching Claude asks (`by: session-allow`), with `access/` reread before each; `take-back`; no
  owner stream or an `agent:` owner denies at once (`no-owner`); the OpenAI rule matcher as a pure
  function; a `Bash` allow is the exact command (option a of Waiting). User-sim ACCEPTED (run 1);
  adversary BREACHED 1 (a Bash allow covered `dangerouslyDisableSandbox`), fixed, HELD.
  Implementer runs 1 each of 4. Kernel 400 tests in 3.2 s; gate 31 s.

- **S23** the console after S7 (PR #35, compared and armed by the PO, merged 2026-11-21 08:23 UTC; ~3.3 h). An answered ask
  shows "Answered, waiting" (or "Checking with your judge") until its decision, with no second
  click; a scrollbar press or the card's text no longer carries the ask out of view; code
  highlighting cannot freeze the page; two open asks behave; status line, sidebar and pills agree;
  each view is tied to its subscription, so an answer never reaches another session. User-sim
  ACCEPTED on run 3 (72-run matrix); adversary breached on 8 of 10 passes (answers reaching a moved
  or switched ask), each fixed with a red test and a randomized open-ask invariant, HELD on passes
  7 and 10. PO check at 1440 px: look unchanged; "Answered, waiting" shown.

- **S33** session allows fail closed (PR #36, merged 2026-11-21 08:24 UTC, 0.3 h). Bare-name allows
  only for `TodoWrite` and `mcp__watane__<tool>`; a Bash reach pins `timeout` and
  `run_in_background`; a Glob reach needs a relative pattern; 4.1: the host writes
  `allowUnsandboxedCommands: false`; the gate runs `cargo fmt --check`. User-sim ACCEPTED; adversary
  BREACHED 1 (an absolute Glob pattern escaped a workspace allow), fixed, HELD. Kernel 411 tests.

- **S1** the open probes of section 18 (PR #37, local foreman, merged 2026-11-21 08:31 UTC,
  1 h 39 min). A maker runs `python3 -I proto/claude-probes/probes.py subagent-slash loopback` on
  the Mac: V26 held (a leading `/` in a subagent prompt runs no skill; a `Skill` deny stops it);
  V27 BREACH (an allowlisted `localhost` reaches loopback through the sandbox's proxy; LAN and
  link-local behind any allowlisted name too), under Waiting. User-sim ACCEPTED; adversary
  BREACHED 1 (recorded as the finding), HELD on the fix. Gate 247 s on the Mac.

- **S34** scope keys for session allows, and `host-info.generation()` (PR #39, merged 2026-11-21
  09:25 UTC). The human's rule built: a session allow holds one (tool, scope key) computed afresh
  from the input that will run (Bash: the exact command and its pins; file tools: the workspace;
  WebFetch: `domain:<host>`; Skill: its name; every other tool and any unknown input key: no key,
  always asks); writes to dot-files, non-ASCII paths and named build files get no key; restart
  detection is `host-info.generation()`; `timer-error` gains `bad-delay` (30-day cap); 4.1 gains
  `profile`; 15 gains the H2 rows. User-sim ACCEPTED; adversary HELD, BREACHED 2 (a Unicode
  `Makefile`; an uncharged timer firing at the ceiling), fixed, HELD.
- **S134** stage F: an OpenAI root delegates to two named sibling children (AGT-J5), each a `child` link in the console with its own transcript; a child that would widen its root (calls `task`, a name outside `delegates`, a wider rule) is refused with no session; stopping, cancelling or deleting the root stops its children, live and stored; every OpenAI turn stops at `max-turns`, `max-tool-errors` or `max-tokens` and names it; Claude reports its native `max-turns` and refuses the other two. PR #60, squash-merged 2026-11-26 11:16 UTC at f8c0fe0d by Deputy Scissors, no bypass; builder claude, reviewer codex, 4 rounds (9/1, 3/1, 1/1, 0/0); ~2.7 h in flight. Adversary BREACHED in r1 (F1.1 to F1.8), every breach fixed by r4; outcome check on scripted Responses, `watane-dev` absent (page-level budgets a known limit). Accepted: F1.5 (`persist: false` not built for OpenAI). F1.4 on the builder's reading. F3.1 ruled (no path drops `Work.stopped`). CI: one gate timeout at 15 min on 5535d7f, green on rerun (Friction). Sketch in factory/DONE.md.
- **S139** (upkeep) the console names the folder a consent waits for (stage C checks it), a stopped turn reads stopped (no unknown-event row, no "Completed" footer), a missing Claude Code binary points to Settings → Claude Code, S36 polish done, and the `installed-agent` wait is bounded and measured (both red runs predated S128's guard). PR #62, rebase-merged 2026-11-26 ~12:20 UTC at 41353b37 by Deputy Scissors, no bypass; builder codex, reviewer claude, 1 round (0/7); ~1 h in flight. Polish P1.1 to P1.7 in Upkeep. Sketch in factory/DONE.md.


## Friction archive (moved 2026-11-28)



- 2026-11-24 (S38 r2) `RemoteTrigger` update then run fired the routine's old brief in a new session (second time after S35 r2): for a later round, create a new routine instead of update+run. (deputy-3)

- 2026-11-20 po skill 0: "the order of work is the vision's steps"; this vision has none, the
  order is in TECHNICAL 17. The skill should allow "or the order of work the spec chain names".
- 2026-11-20 config `boundaries` points at TECHNICAL 14, which names sections, not files; the
  foreman's adversary rule matches diff files against it, so it can never match. adopt.sh (or
  the skill) should say the section must name files. S2 fixes it here.
- 2026-11-20 gate `check-mod.sh` passes with nothing checked while there is no `wit/world.wit`:
  design and probe slices merge on a vacuous CI. Fine for now; a design-only fallback check
  (links, `//!`-style headers) would give such slices a real gate.
- 2026-11-20 po skill 2/README "Slices that span projects" assume both projects run this factory;
  nothing says what to do when the provider (watane) runs its own older factory. Recorded as a
  cross-project dependency instead.
- 2026-11-20 PLAN template has no place for the human's answers (only `Deputy decisions` and
  `Waiting on the human`); added `Human answers`.
- 2026-11-20 po skill 4: "use AskUserQuestion only on a turn the human's own message started";
  a PO started by another session on the human's behalf cannot tell. Worked here; worth a line.

- 2026-11-20 a second session committed a design change (3926a0d: AGENTS.md, console, N11) to
  this checkout 14 s after the PO's plan commit, overlapping S2's sketch (AGENTS.md, drafts). The
  skill assumes the PO alone writes `main` besides merges; nothing tells another session that a
  slice owns those files. A `Touches` lock visible outside PLAN.md (or a PR for every design
  change) would have caught it.

- 2026-11-20 PO error: `git commit -a` in a checkout another session also edits swept that
  session's uncommitted `design/TECHNICAL.md` edits into plan commit 5e46864, on main. Nothing
  lost; S2 will rebase over it. The po skill should say: commit `factory/PLAN.md` by path only.

- 2026-11-20 po skill 3 (local): `claude --bg` in a fresh worktree stops on the workspace trust
  prompt; the skill does not say so, and an unattended PO cannot start any local foreman. The
  skill (or adopt.sh) should have the human trust the project once, or create local worktrees
  where trust is inherited.

- 2026-11-20 S2 foreman: user-sim and adversary are built around red code tests; on a documents
  slice they report checks instead (worked, outside their format). The skill's `gh` steps do not
  apply in the cloud (no `gh`; it used the GitHub MCP tools). It reported "about 2 hours in
  flight" for 20 minutes: a cloud foreman cannot tell elapsed time either.

- 2026-11-20 watch-cloud.sh missed S3 and S5 (the PO stayed idle for 1.5 h until the parent
  woke it). The foremen pushed `DELIVERED <id>` as an empty commit after arming auto-merge; the
  rebase merge drops empty commits and `delete_branch_on_merge` deletes the head, so CI (17-40 s)
  merged and erased the report before the script's 120 s poll saw it. The watchers were running
  and never exited. Fix: watch-cloud.sh should also return when `gh pr list --head slice/<id>
  --state merged|closed` finds the PR (or the foreman pushes DELIVERED before it opens the PR, and
  the report lives in the PR body). The PO now watches with an inline loop that checks both.
- 2026-11-20 S5 foreman: a design slice's user-sim fixes are new spec text, so it rejected three
  times on readings the previous fix introduced; the adversary can only propose cases as text.
- 2026-11-20 S3 foreman: it could not fix the spec-path gap (TECHNICAL 1 and 14 name `src/` at the
  root) because `design/` was locked by S5; it put it in the report. Worked as intended.

- 2026-11-20 po skill 5 vs auto-merge: a cloud foreman arms auto-merge as it delivers, so a UI
  slice merges (S4: 38 s of CI) before the PO can compare it with the design; `--disable-auto`
  came too late. A UI slice should open its PR with auto-merge off, and the PO arms it after
  the comparison (or the foreman skill names UI slices as "PO arms").
- 2026-11-20 watch-cloud.sh miss: fixed in the factory (watane-mods aaf89b6, synced by PR #7).

- 2026-11-21 Flake: S4's `ac1_app_shows_sidebar_header_rows_dock_on_the_played_story` races the
  played story (PR #9, rerun once). Weighed: *guard*, hits any PR; folded into S9 (owns
  `console/`, in flight, merge armed by the PO) rather than a separate slice on the same files.
  A cloud foreman cannot be told mid-run, so a late outcome is checked at acceptance.

- 2026-11-21 S9: every console task rebuilds the committed `ui/`, so parallel tasks always conflict
  there on rebase; and fix tasks for the user-sim reopened adversary breaches twice (a brief that
  changes the ask card should list the adversary's held cases as acceptance).

- 2026-11-21 S21: a validator committed a 550 MB build copy (`kernel/target-base`); the foreman
  force-pushed it away. The validator's instructions (factory) or `.gitignore` should stop it.
  Parallel kernel tasks all edit `tests/it/main.rs` and `error.rs`: trivial rebase conflicts.
  User-sim and adversary red tests should pass clippy themselves.

- 2026-11-21 S7: the `npm test` budget moved three times within the slice (8.6, 11, 12.5, 14.5 s);
  a budget should be fixed at the slice's end, not per task. Parallel console tasks conflict in the
  committed `ui/` every time (S9, S15, S7): a factory rule to build `ui/` once at integration.

- 2026-11-21 S26: briefs that paraphrase a spec rule missed two of them (user-sim and adversary
  caught both); a brief should quote the rule. Implementers still skip the red phase on some
  tasks, and `cargo mutants` is not in the cloud image.

- 2026-11-21 S28: a brief that changes what a call writes should quote the ceiling rule (9.3
  "Past"); the adversary found the gap. A brief's AC that says "existing tests show it" was false.

- 2026-11-21 S29: a load-sensitive timing test (`long_S7`) in the default gate fails parallel
  tasks' checks for code they did not touch (S30 moves it). No `cargo mutants` or `/usr/bin/time`
  in the cloud image.

- 2026-11-21 Driving a second project (watane, the bridge): its PLAN has its own format (Status
  markers, steps, Foremen) and a long history; the PO must read its README, PLAN and VISION first.
  Its checkout holds another session's uncommitted blog files, so `git pull --rebase --autostash`
  is unsafe there; `git merge --ff-only origin/main` works when the PO has no local commits. Its
  branches are lowercase (`slice/s95`) while ids are `S95`: the watcher must match both. A slice
  of one project that waits on another's merge has no watcher that spans repositories.
- 2026-11-21 po skill 3 Local: the hand-out's `cd` into the worktree moved the PO's Bash cwd there
  for the rest of the session; run `claude --bg` in a subshell.

- 2026-11-21 PO error, recurring (the human caught it): from S3 on, the PO cut slices by folder
  (kernel on an in-memory fake host, console on a scripted fake kernel, spec debts on their own),
  because the watane host parts did not exist and folders kept slices apart. Each layer then made
  more risks and spec debts, which became more layer slices. The po skill's "crosses every part
  it needs, thinly" was read as "within this repository"; a story that needs another project
  should be one story with a half per project, started together. Worth a line in the po skill:
  a slice whose story names only a test run ("a kernel maker runs `cargo test`") is a layer slice.

- 2026-11-21 S1 (local): `test:slow` not run on the Mac (no global playwright); `probes.py` exits 0
  even when an expectation fails. Local foreman worked once the folder was trusted.

- 2026-11-21 PO: S32 (stage A, end to end) shares `kernel/` and TECHNICAL with S34, still in flight;
  handed out anyway to start the end-to-end path the human asked for. S34 should merge first and
  S32 rebases on it. A cloud foreman cannot be told mid-run.

- [~] **S163 (upkeep) kernel and console from Upkeep, code side only.** In flight (cloud, builder codex, reviewer claude; Codex task https://chatgpt.com/local/6b09a2fc-21c8-83eb-bfdb-a318b5bd405b 15:35 UTC). Delivered as PR #76 at ca422b0 (CI green); Deputy Paper took over 2026-11-28; r1: Claude review routine `trig_01XzhNgzHD2kQaNDuz16g3Wn` 07:24 UTC. Sketched 2026-11-27 by Deputy Rock; builder codex by rule 3 (the last two handed out: watane S162 claude, watane S161 codex). The spec-side items wait for S151 to leave `design/`. First strike each item already on main and name it in the PR.
  1. kernel: (S160 P3) a native `Consent(Some)` after Workspace is ignored once `launching` is `Some(false)`; (S153 P1.3 a) a cancelled launch closes its open asks as `ended` does; (S154 A1.1) the `launching` fallback `key.consent.is_some()` applies only to keys without `launching`; (S138 P2.3) owed work drains in one layer (`Kernel::read` or `run_client`).
  2. console words: (S160) a failed turn's footer does not read "Completed in 0 s" beside "Failed", the status shows words, not the raw cause, and the session leaves RUNNING; (S154 P1.2) "Stopped after ... tools" counts tools, not asks, the decision line reads "stopped", an answer on a cancelled launch does not read "Allowed"; (S144 r2 P2.4, P2.5) a taken row's error does not wrap to ~6 lines at 390 px nor repeat above the composer, and Esc while Not running sends nothing.
  3. console, against `Main-2`/`States-2` (S142 and S144 deputy compares, S36 felt wrong): queue rows as the frame draws them (label "QUEUED · n of 32 · sent when this turn ends", a dashed row per item, quiet buttons, × with aria "Remove from queue"); "Enter a message first." only after an empty send; no Discard beside Undo on a Removed line; "Thinking" for a `thinking` activity; a queued row shows a bounded preview of a long message.
  4. tests: (S159 P1.4) a `test:load:d:cut` script for `--controlled-cut`; (S36 P7.1) exact just-over-limit fixtures in `console/test/host/load_check.test.ts`.
  Outcomes: each item's check, and a user-sim on the console for 2 and 3. Touches: `kernel/`, `console/`; not `design/` or `AGENTS.md` (S151 holds them), no host interface change. `deputy compares` (3). Boundary: consent (TECHNICAL 14) for item 1: the adversary runs on it. Protects: *core* (1), *polish* (2 to 4). Size: an upkeep slice.
  DONE 2026-11-28: PR #76 merged (rebase) at 551b4f46 by Deputy Paper after r3 approve (0/0) and green CI on a510f45. A late native or line consent after `initialized` no longer reopens the folder wait, and a failed turn's footer and status read in words. Builder codex, reviewer claude, 3 rounds (2/3, 1/2 stalled by rule and narrowed to one line, 0/0); ~17 h in flight (overnight pause). Accepted A1.1 (legacy key), polish P2.1 as #124.

### S165: the design lane, spec-side upkeep (upkeep lane, design/ only)

Sketched 2026-11-28 by Deputy Scissors. In flight (cloud, Codex task https://chatgpt.com/local/6b0a975c-c378-83ed-a1f9-cf3be32ed06e on GPT-6 Sol, 08:56 UTC; builder codex by rule 3: last two handed out were watane S164 codex, watane-factory JF1-1 claude); reviewer claude. DELIVERED as PR #129 at 321f2466 (+88/-192, CI green); r1 Claude review routine `trig_01PgGb2NiiyapP7NqFcxVvwa` 09:27 UTC. r1: 6/6 (sentence-level spec fixes), weighed by Deputy Scissors: all six stand; F1.6 ruled `Part of #n` for the code parts; A1.1, A1.2 accepted (precedent S39 r1, S154 A1.3); answer sent to the same Codex task 09:50 UTC; answers b644b28f 09:51, CI green; r2 Claude review routine `trig_01WzXeVEtPxdC5S3Bn82zBgm` 10:07 UTC. r2: 2/9; F2.1 (OpenAI Name(arg) rule dropped, AGT-F-4) spec-side here, code filed as #133 (guard); F2.2 `Part of #122`, the S153 process entry stays for a factory proposal (deputy); A2.1 accepted (S154 A1.3); answer sent 10:27 UTC; answers 789d7553 10:29, CI green; r3 Claude review routine `trig_01RC57NDBfezraTFDu59iAFM` 10:42 UTC.

Story: a reader of `design/` finds what is built and what the human and the deputies decided written where it belongs, with no drift between TECHNICAL, FUNCTIONAL and the code.

Issues, in this order: #97 (the human's S151-H H2 answer: pkg limits the `agent-access` import), #122 (the deputy decisions in `factory/decisions/LOG.md` before 2026-11-28 into the specs that own them), #125, then the `spec`-labelled ones: #98, #99, #101, #102, #103, #105, #106, #107, #108, #109, #113, #118, #119.

Outcomes (each issue's check): every listed issue is either fixed by the PR (`Fixes #n` in its body) or closed with the commit on `main` that already fixed it; `factory/gate.sh` passes; no clause the PR adds contradicts another level of the chain.

Touches: `design/` (VISION untouched), `factory/decisions/`. Constraints: no code, WIT or kernel interface change; a change to VISION or a weaker boundary in section 14 is not this slice's: stop and say `BLOCKED` with the clause. Where an issue asks for a decision the specs leave open, pick the reading of the spec's literal words and name it in the PR body. Upkeep is exempt from the size rule. spec-check not run: the slice is itself the drift fix, and the review checks the chain.

Merged 2026-11-28 ~10:55 UTC by Deputy Scissors (PR #129, rebase). Digest: a reader of `design/` finds the human's S151-H answer, the earlier deputy decisions and the drift fixes of 13 spec issues written where they belong. Builder codex (GPT-6 Sol), reviewer claude; rounds r1 6/6, r2 2/9, r3 0/5. Left open as `Part of`: #97, #102, #106, #107, #122 (S153 process entry, a factory proposal). Code side: #133 (guard: OpenAI `Name(arg)` and path-argument rules refused at the profile check, widened by A3.1). Polish: #136. Accepted: A1.1 (S39 r1), A1.2 and A2.1 (S154 A1.3), A3.1 (= #133). Suite: spec only, gate ~1 min. In flight ~2 h.
