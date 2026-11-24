
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
