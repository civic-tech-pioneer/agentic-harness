# Plan

Kept by the product owner (`/po`). The factory's process: `.claude/factory/README.md`.

## Order of work

The vision (`design/VISION.md`) has no numbered steps; the order is `design/TECHNICAL.md`
section 17. Since S2 the three design files are the contract; N5 stays open until stage F.

| Stage | Builds | Where | State |
|---|---|---|---|
| 0 | Design slice: drafts become the contract (S2); open probes of proposal section 18 (S1) | this repo | sketched |
| A | H1 streams, H5 durable requests, H7 timers; V8 prototype | watane host | cross-project, waits (below) |
| B | H2 + unchecked provider + kernel Claude sessions; dev's fake `claude` | watane host, dev; kernel; provider | cross-project, waits |
| C | H6 consents, app words, `claude` binary setting, plan line | watane app, pkg | cross-project, waits |
| D | H3 accounts, OpenAI loop (kernel 9.5), scripted Responses server | watane host, dev; kernel | cross-project, waits |
| E | Checked provider (needs a judge account, V7) | `provider-checked/` (R-B1); watane half done (S122) | S127 in flight; V7 on Jev: p50 173 ms, p95 267 ms |
| F | Delegation and counters (kernel 9.7), factory profiles | kernel | N5 = (a) for now |

**Cross-project dependency.** Stages A to D put code in watane (`../../watane`: host, dev, pkg,
app), which runs its own older factory and is not on this generic one. Each such stage becomes
one watane slice (provider, first) plus one slice here or in a provider repo (consumer). The
interface between them is `design/TECHNICAL.md` section 2.1 (`watane:host@0.1.0` additions) and
the watane contract rows of section 15. None of them is this product owner's to hand out until
watane is on the generic factory, or the human hands stage A to watane's own product owner. Stage
A's backend is pure watane; its console part (TECHNICAL 17) is this repository's.

## In flight

 S138, S142, S144, S145, S148, S151, S152, S153, S154, S156, S160, S163 merged.

## Sketched
### Stage B carry list (folded into stage B's end-to-end slices, never a slice of its own)

- S34 RISK 1: write keys for build and config files beyond the list (`Cargo.toml`, `conftest.py`,
  `pyproject.toml`, `justfile`, `Dockerfile`, ...): decide with the first slice that sets a
  `workspace` (rec: no write key outside a named source tree; else a longer list).
- S34 RISK 3: `session_firing` returns before re-arming and charging when a restart step fails, and
  a due firing with no step fails `quota` at the ceiling (`restart_firing` handles both).
- S26 accepted limits and S28's: verify with the real host's rollback (7) once wired.
- V27 (Waiting): the allowlist rule, before the shell grant (stage C).
- S32 R1 (first, before any provider item reaches the kernel): in WASM a kernel error returns
  normally and the host commits writes made before it, while the crate assumes a failed call
  commits nothing: buffer writes in the crate until success (rec), or trap on error.
- S32 R2: provider item bodies go through the adapters (9.8), not straight into `Out`.
- S32 functional: an early wall-clock firing can stop a probe for good (the probe goes in stage B).
- S32 user-sim "felt wrong": the first tab freezes with no word when a second takes over; the status
  line reads Idle while probe items arrive; `probe.start` with two fields out of range names one.


### Upkeep

Upkeep is GitHub issues labelled `upkeep` (with `guard`, `core` or `polish`, and `spec` for spec-side items), filed from this list on 2026-11-28 as #97 to #121 (deputy skill, section 2).

## Deputy decisions

(none in the veto window; the earlier ones wait in `factory/decisions/LOG.md` to be written into the specs that own them, #122)

## Human answers (2026-11-20, through the PO)

- 2026-11-28 (to Deputy Paper) **S151-H: accept H1 to H5 as written; on H2, pkg limits the import to `mods:agent`.** PR #69 merged at 40fa22fa. The H2 limit goes into the design with the next design-lane slice (Upkeep).

- **2026-11-27, N5: (a), decided and closed** (the human, to Deputy Paper: "It is still a. Please record it as decided and not requiring any more of my time. If my mind changes on it, I'll get back on it"). The factory's implementer uses the Claude grant with shell and web; no `codex app-server` backend. Do not ask again; the human reopens it if they change their mind. Stage F's factory profiles build on (a).

- **2026-11-25, the human (to deputy-5): no test run in a cloud (CI or a cloud team) calls an external provider** (OpenAI, Jev or another judge, GitHub's API, any real service). Unit and integration tests use fakes: dev's scripted Responses server and scripted judge, fake `claude`, fake-watane. A check against a real provider is a deputy probe on the Mac (V7 on Jev). Checked 2026-11-25: no workflow in watane or agentic-harness passes a provider secret to a test.

- **2026-11-25, the human (to deputy-5):** (1) **S40-F6.1: (a)**, one more answer round with the narrow rule; if it does not succeed, (b): accept it as a known limit and fix it in Upkeep. (2) **V7: the judge is Jev.** Its API key is in the environment variable `TYPESAFE_API_KEY` on the Mac (never read it into a context or a log); spending limits are set at Jev, so the factory adds none. (3) **N5: (a) for now**, the Claude grant with shell and web; (b) may follow later. (4) **The Jev key stays on the Mac only**: no GitHub or cloud secret. V7 and every check against the real judge run on the Mac, by the deputy, as a read-only probe with the key taken from the environment (never printed); cloud teams build and test against dev's scripted judge.

- 2026-11-24 watane host 18 **Q38: (b)**, the human: a consent is kept per (origin client, provider, folder or account). TECHNICAL 8 cites watane host 19.7, which S103 writes to match; no change here.

S2 writes each into the document that owns it.
- **N1** Two Claude Code grants: file tools in confined folders, and shell-and-web inside Claude's sandbox.
- **N2** `auto` permission mode stays out.
- **N3** All listed changes to watane's contracts are accepted, F-AGENT-2/W6 included.
- **N4** Keep full fsync per write; decide after V8 (stage A) measures it.
- **N6** Stay on ACP v1, with the kernel's own entry ids, turn state and replay end.
- **N7** Workspace instructions off; a later opt-in profile field.
- **N8** watane gets MIT OR Apache-2.0 for all of it. A watane change: cross-project, not ours.
- **N9** Subagents stay `--agents=` flags, capped at 64 KiB.
- **N10** The host keeps the judge's key as an account bound to the judge's origin.
- **N11** The console shows only its own sessions; F-AGENT-6 stays as it is (asked after commit 3926a0d raised it).
- **R1** This repository is the kernel `mods:agent`; each provider has its own repository.

- **2026-11-21, the human:** (1) the three host-wire additions are accepted, with restart detection
  as a new interface `host-info` with `generation()` (in place of `caller.boot()`); `outcome` on
  `answered` and checks closed by the host with `expired` as written. (2) Session allows use a scope
  key per tool: a pure scope function turns a call's input into the smallest part that decides
  what it can reach and that a person can judge at a glance (Bash: the exact command; file edits:
  the workspace; a web fetch: the host; a skill: its name). A tool without one, or an input that
  cannot be scoped safely, gets no key and always asks. The kernel computes the key from the input
  that will run and shows it on the ask; "allow for this session" stores (tool, key); a later call
  is allowed only when its fresh key equals a stored pair exactly; take-back and session end remove
  it. (3) Bash: the exact command. (4) The sandbox is the boundary for Bash allow rules.

- **2026-11-23, the human, on S35-CAP** (PR #43 at the round cap): "Yes, one more round". Option (a): one more
  answer round for F5.1 by the builder (Claude), then a sixth review by Codex. The merge needed R-B1 too (answered below).

- **2026-11-23, the human, on R-B1:** (b). "b is fine with me ... I favor the pragmatic approach here and keep it
  together until there's real evidence that they ought to be apart." The provider stays as `provider/` in this
  repository; TECHNICAL 17 (stage B) gets one boundary list and drops "its own repository". S35's r5 answer
  round makes that change, so review r6 covers it. S35 merges on an approving r6 and green CI.

- **2026-11-23, the human, on the S35 composer ruling:** "I agree with the reviewers that it ought to work that
  way. But at the same time, if it is a lot of hassle to get it to work, it's not the kind of feature that needs
  much of our attention, especially because it's fairly rare." The ruling stands for S35; an Upkeep line (polish,
  low priority) keeps the recoverable sent text.
- **2026-11-23, the human, on watane S100's top-level resolved `path`** (Deputy decisions, for S36): "Sounds good.
  Approved." No longer a pending veto.

- **2026-11-23, the human, on the R-B1 vision edit:** "That should not have lived in vision, because it's way too
  specific, And only belongs to the technical documentation, as an option. So good to remove it there." VISION drops
  the repository rule entirely; TECHNICAL keeps a provider's own repository as an option for later.
- **2026-11-23, the human, on R-B4** (the built-in `scratch` profile): "Sounds good to me." Option (a), as S35 built it.
- **2026-11-23, the human, on V27:** "In principle, what I would like is that the profile at least has the option to
  block or allow access to localhost via whatever mechanism that may be. I could imagine that it's complex though, so
  I'm willing to take a best effort and a report on exactly what can be done and what are the limitations. I'll take
  the decision from there." Read as: a profile setting for local network access (loopback, private and link-local
  ranges), denied by default. Spike S37 reports the mechanisms; V27 is decided after it.

- **2026-11-23, the human, on N5:** "I'm leaning towards B as Well. But let's revisit this only once we have actually
  started at E stage E, that is." A leaning, not a decision: (b), a `codex app-server` backend in the host, keeps a
  second vendor for cross-checking. Revisit when stage E starts.

- **2026-11-23, the human, on S35-MERGE:** "Merge": merge PR #43 once CI on 8a9e512 is green, with no seventh review;
  then hand out S36. The user confirmed it in the deputy's session ("Yes, merge S35"); merged at 042b0dd.

- **2026-11-23, the human, with the monitor, on reviews** (watane-mods PR #5): the review lead triages blocking versus
  polish (polish never holds a merge; it goes under Upkeep); the lead may propose to accept a blocking finding (security
  only if cheap to undo and outside the vision's promises, else the human); no fixed round cap, each review ends with the
  trend and the lead's call, and from round 4 the deputy writes why one more round is worth it; the deputy rules when a
  finding comes back twice. "find a balanced perspective on building out many features of the product versus building
  few features perfect".

## Waiting on the human



- **S40-CUT (2026-11-24): restart recovery of OpenAI sessions has stalled three rounds** (each fix opened a new path where the global restart pass acts under the wrong origin). Proposed cut, by J11's own words for Claude ("Notes shows the session as not running, with everything up to the last finished message; she sends 'continue'"): after a host restart, an OpenAI session that was mid-turn shows not running with its transcript up to the last finished message; its open and pending calls are aborted; the restart pass sends nothing, opens nothing and asks nothing for it; the owner's next message continues it under her own origin. Keeps: everything else in S40 (profiles, refusals, streaming, tools, pause until reset, the meter). Becomes a known limit: an OpenAI turn does not continue by itself after a restart (TECHNICAL 9.5's restart row and case 22's reissue, Upkeep). Undo: an owner-scoped recovery in a later slice. Recommended: take the cut. DECIDED 2026-11-25 by the deputy under the new cut-early rule (mboogerd/claude-plugin#3, asked by the human): cut taken; answer r3 with the cut to Claude, routine `trig_014rQUrVhYxDfSZTELMCedUR`, fires 06:37 UTC. The human may veto; the undo is an owner-scoped recovery slice.

- **S35-MERGE: PR #43 after the extra round (r6: `changes (2)`, adversary HELD, user-sim ACCEPTED, F5.1 closed, CI gate
  green).** F6.1 (PLAN.md on the branch still shows R-B1 open) is done on main; the slice does not touch PLAN.md. F6.2 is
  a one-line citation in `provider/BOUNDARIES.md` (cite 3629c5c, not 866d325); the builder fixes it now (routine, 08:45
  UTC). Options: (a) the deputy checks F6.2 with Codex's own command (`git show <sha>:design/TECHNICAL.md | grep -Fx` each
  copied row, exit 0) and green CI, then merges with no seventh review (rec: a citation, checked mechanically); (b) one
  more Codex review round first. Undo: (a) a revert of the merge.

- **V27: the S37 spike report is in; your decision** (the human wants a profile setting for local network access, denied
  by default). Report: `design/SPIKE-local-network.md` at 5f7392a on `slice/S37` (PR #48, closed unmerged). Its findings:
  (A) the kernel refusing allowlist entries (`localhost`, `*.localhost`, local IP literals incl. IPv4-mapped) is cheap
  and probed, but misses DNS names that resolve local, rebinding, and direct dials; (B) Claude's sandbox: on macOS
  `["localhost"]` reached loopback through the sandbox proxy (S1, probed); Anthropic's sandbox-runtime has a
  resolved-address guard at dial time (loopback, link-local, metadata by default; private and ULA ranges need extra
  config) but whether the pinned Claude build exposes it is unverified; (C) a watane host egress broker that resolves
  once and dials only a checked address covers rebinding, mixed answers, IPv6 and the metadata address (probed in a
  Linux fixture), at the cost of a host component that must own both proxy paths; (D) OS egress rules (Linux cgroup or
  nftables, macOS Network Extension) or a filtering resolver: costlier, admin-level, or partial. Recommendation: keep
  the domain allowlist, add (A) as a first line, and call the setting enforced only with a dial-time address check (B if
  the Claude build exposes it, else C); if neither is available, fail closed for a profile that promises local denial,
  or label it best effort with the gaps named. Decide in stage C, with the probes in CI on Linux and macOS.
  Was: V27 (decide after the S37 spike report; the human wants a profile setting for local network access, denied by default):
  the shell grant's network allowlist reaches loopback through Claude's sandbox proxy**
  (S1, PR #37, security boundary). With `allowedDomains: ["localhost"]`, a command reaches a
  loopback server through the sandbox's own proxy (direct connections stay blocked); the proxy also
  reaches LAN and `169.254.169.254` addresses behind any allowlisted name. TECHNICAL 4.2's
  "loopback denied" holds only if no allowlisted name names or resolves to such an address.
  Options: (a) the kernel refuses allowlist entries that are `localhost`, `*.localhost`, IP
  literals, or private, loopback or link-local ranges (cheap, at profile check; does not stop
  DNS names that resolve there or rebinding); (b) the host puts its own egress proxy in front of
  the sandbox that checks the resolved address at connect time and refuses loopback, private and
  link-local (robust; a watane host part, stage C with the shell grant); (c) name it in 14 "Not
  protected" (the participant chose the allowlist). Rec: (a) now plus (b) before the shell grant
  ships. Undo: (a) one refuse-table row; (b) a host component.

## Friction

- (2026-11-26, Deputy Scissors, S134) CI `gate` was cancelled at its 15-min limit on 5535d7f (runs before took ~7 min); green on rerun. Watch for a second sighting.
- (2026-11-26, Deputy Paper, S144) third sighting: CI `gate` cancelled at its 15-min limit on 30df2491 (S134, S140 r1, S144 r1); rerun once. A factory proposal to the human (README "Keep it lean"): raise the gate job's `timeout-minutes` to 25 or split the slow browser suite into its own job, after measuring where the 15 min go.
- (2026-11-26, Deputy Paper, S138 r4) a Claude review routine that waits for CI with ScheduleWakeup never wakes (the run ends idle): review briefs must say "do not wait or schedule; if CI is running, post with CI pending and the deputy weighs CI".

(older lines archived in `factory/DONE.md`, "Friction archive")

## Suite time

- S131 (2026-11-26, `Linux 6.18.44-fc-v880 x86_64`, 4 cpus, a cloud VM; load average 2.6 to 4.3 from the runs themselves): console
  `npm test` 635 tests 38 s, `test:slow` 21 s; kernel `cargo test` 621 + 6 + 1 in 5 s (warm); gate 79 s. By hand: `test:host`
  25 pass, 7 skip, 212 s; `test:host:c` 6 pass, 36 s (warm); `test:host:d` 9 pass.
  - **Stage C budgets** (TECHNICAL 17 row C, the kernel and host part, on dev's fake `claude`; the real `claude` run is the
    human's): `npm run test:load:c`, the stage B load (8 sessions, 4 `page` in Chromium and 4 `local/loadclient`, each 50 deltas
    and 5 entries of 4 KiB a second) on a participant home of watane c52d2aa (S129) whose app is the fake `Confirm` of
    `console/test/host/home`, each session started with `start-in-folder` in a folder new to it (8 consents asked and granted per
    run). r1 (F1.7), the home built with `console/test/host/home/commit-clock.patch`, load average 1.4 to 2.7: item delay p95
    5.0, 4.9, 6.0 ms (budget 100; max 122, 114, 149 ms); Stop to `idle {interrupted}` at the page p95 16.8, 12.0, 11.0 ms (budget
    2 000); writer busy share over the common window 19.9, 20.8, 21.4% (about 3 130 commits; budget 50). State bytes per entry:
    as B (the crate's `s36_load`). The first three passes (before r1, stock home: writer not measured): item p95 6.8, 4.5, 5.0 ms,
    Stop p95 11.9, 13.9, 13.8 ms. Stage B on the same machine and host, same day: item p95 12.0 ms, Stop 19.5 ms.
  - **Stage D budgets, remeasured** (S128 P2.5): `npm run test:load:d` on watane 548b4e1 with `stage-d-tools/watane.patch` (which
    builds `watane-host` and `watane-dev` at opt-level 2 in the dev profile; each Claude delta carries a marker ~37 bytes longer
    than the stage B load's). Five runs: item p95 17.5, 17.2, 18.3, 23.2, 24.1 ms (budget 100); Stop p95 44.9, 44.0, 38.4, 36.3,
    46.8 ms (budget 1 000); writer busy 28.9, 27.7, 29.0, 30.5, 30.8% (budget 60); cadence 8/8 OpenAI sessions at one step a second
    (runs 4 and 5 print it; 1 to 3 had no cadence fault). Runs 1 and 3 pass; runs 2, 4 and 5 fail only the source check: one fake
    `claude` wrote no delta for 40.9 to 46.4 ms, over the 2x spacing of 40 ms (S36 F7.3, the second sighting: the deputy's
    "widen to 3x if it fails again"; the rule is also TECHNICAL 16's, which this slice may not touch).
    r1 (F1.8, the deputy's ruling: the rule kept at 2x, P2.1's second-step cadence window in force for all eight runs), three
    more runs on a quieter start (load 1.6 to 2.1): item p95 19.6, 18.9, 20.0 ms; Stop p95 58.9, 34.6, 40.0 ms; writer busy 29.0,
    28.7, 29.3%; cadence 8/8 each. Runs 2 and 3 pass; run 1 fails on completeness, not spacing: one OpenAI session (`s9`) had one
    source item and 4 of its 32 tool calls missing at the page within the window (undiagnosed: possibly its last step cut by the
    Stop at the window's end; reported in PR #58). **The stage D Budget row stays open**: 4 of 8 runs pass, every budget held in
    all 8.

- S4: console `npm test` 167 tests 6.2 s; `test:slow` 7.5 s; gate with kernel 24 s (2026-11-20).

- S3: kernel `cargo test` 57 tests in 0.05 s; CI gate job 17 s (2026-11-20, macos-15).
