
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
