# agentic-harness: where things are, for agents

The agent layer of watane: a kernel mod that runs Claude Code and ChatGPT
sessions for other mods, its providers, and the console, the kernel's own
page. Its small host part lands in watane (`../../watane`), never here.

## Find the right document

| You need | Read |
|---|---|
| Why, what comes first, boundaries | `design/VISION.md` |
| What it does, journeys | `design/FUNCTIONAL.md` |
| How: interfaces, state, rules, tests, build stages | `design/TECHNICAL.md` |
| What the console looks like | the canvas row "Revision 2" of [watane agent frontend](https://claude.ai/artifact/8jz2aHuaPo2REtKQyMt89e), with `TECHNICAL.md` section 18 |
| What is built, what is next | `factory/PLAN.md`; one task's brief: `factory/tasks/<id>.md` |
| How the build process works | `.claude/factory/README.md` |
| watane's own rules (host, pkg, app, dev) | `spec/` and `<sub>/TECHNICAL.md` in the watane checkout |

## How to treat them

- **The chain decides.** `design/VISION.md`, then `design/FUNCTIONAL.md`,
  then `design/TECHNICAL.md`, then code. When two disagree, fix the lower one
  at once, and fix each file that cites it. The open points are N5
  (`factory/PLAN.md` "Waiting on the human", stage F) and any later item the
  PLAN lists under "Waiting on the human"; they are the human's, and a slice
  that depends on one waits for the answer. The answered ones (N1–N4,
  N6–N11, R1) are rules in the specs.
- **Backend first.** Sections 1–17 of `TECHNICAL.md` (and 3.1–3.10 of
  `FUNCTIONAL.md`) are the backend. When a choice or the time left pits it
  against the console, the backend wins. The console uses only the kernel
  interface every client has. If it needs more, add that to the backend
  sections first.
- **The console's scope is the canvas target**, not more. Anything past it is
  `FUNCTIONAL.md` section 5.
- **Contracts and records.** The three `design/` specs and this file are
  contracts. These are records: read them for why, never cite them as the
  rule, and do not rewrite their history:
  - `design/ARCHITECTURE-PROPOSAL.md` and `design/REVIEW-*.md`;
  - `design/COMPARISON-*.md`;
  - `design/FRONTEND.md` and `design/FRONTEND-2.md`;
  - `research_notes/` and `proto/`.
- **Ask the human** only to go against `VISION.md`, to weaken a security
  check (`TECHNICAL.md` section 14), or to settle an open point (`N<n>`).

## Tests

Each behaviour is proven at the lowest level that shows it
(`.claude/factory/README.md`, test pyramid).

| Level | Command |
|---|---|
| Gate (links, mod contract, kernel, console) | `factory/gate.sh` (runs `.claude/factory/check-mod.sh`, then each part below that exists) |
| Kernel crate | `cargo test` in `kernel/` (a workspace: the crate and `kernel/wasm`, the mod's wrapper, whose host-independent mappings it tests natively) |
| Kernel component | `cargo build --release --target wasm32-wasip2 -p agent-mod` in `kernel/`, copied to `kernel/wasm/logic.wasm` (prebuilt logic, watane dev 7.1; git-ignored); the gate does both. `watane-dev test --json` in `kernel/wasm/` needs `ui/` copied there and a watane-dev that links host 19 (not before watane S96) |
| Checked provider | `cargo test` in `provider-checked/logic/` (in the gate): the judged sequence of TECHNICAL 10 on a fake host, the scripted judge played as raw System One bodies; no test reaches a real judge |
| Console views | `npm test` in `console/` (in the gate) |
| Console page (slow) | `npm run test:slow` in `console/`: Chromium layout and open-at-the-end checks; run before a console slice delivers |
| Stage A on watane's real host | `npm run test:host` in `console/`: needs `watane-dev` and `watane` of watane rev c2a7d4d or later on `PATH` (S101: a machine with no IPv6 loopback is served on 127.0.0.1 alone); exit 2 means not run; by hand, not in the gate (CI installs v0.1.0). Its ask-expiry check waits out the host's 10-minute ask deadline (about 14 minutes in all) |
| Installed release journey (slow) | `npm run test:installed` in `console/` with `WATANE_BIN_DIR` set to the Watane build directory containing `watane-app` (feature `driver`), `watane-dev`, and `fake-claude` (S39). Needs a desktop session or Xvfb. |
| Stage B budgets (slow) | `npm run test:load` in `console/`: 8 fake sessions for 2 owners on `watane-dev serve`, item delay and Stop measured at the page (TECHNICAL 16 "Budget record"); needs `watane-dev`, `fake-claude`, `cargo` and `python3`; exit 2 means not run; by hand, on a quiet machine. The state bytes per entry: `cargo test -- --ignored s36_load` in `kernel/` |
| Stage C on a participant home (slow) | `npm run test:host:c` in `console/`: S38's outcomes 1 to 4 on a home whose app is a fake `Confirm` (`console/test/host/home`, `stage-c-home`, linked against `watane-host` of `WATANE_SRC`, else the watane checkout whose `target/` holds `watane-dev`), and outcomes 2 and 4 on `watane-dev serve`; needs `watane-dev` and `fake-claude` of watane S103 or later, `cargo`, `python3` and Chromium; exit 2 means not run; by hand, not in the gate |
| Stage C budgets (slow) | `npm run test:load:c` in `console/`: the stage B load on that home, each of the 8 sessions started in a folder new to it, so each launch asks and the fake app grants its consent; Stop's budget 2 s (TECHNICAL 17 row C, the kernel and host part on the fake); the writer's busy share only when `WATANE_SRC` has `console/test/host/home/commit-clock.patch` applied (else it prints "not measured"); same needs as `test:load` plus `WATANE_SRC`; exit 2 means not run; by hand, on a quiet machine |
| Stage D on a scripted preview (slow) | `npm run test:host:d` in `console/` with `watane-dev`, `watane` and `scripted-responses` from Watane S114 or later and Chromium. The test requires `serve --script` before it starts a session. Exit 2 means a prerequisite is missing. A host that rejects `serve --script` gives exit 1. |
| Stage D mixed load (slow) | `npm run test:load:d` in `console/` with the stage D preview tools, `fake-claude` and Python 3. Measures page latency, Stop, cadence and writer busy share for eight sessions of each backend across two owners. Use the instrumented Watane preview tools described in [the Stage D fixture guide](console/test/host/stage-d-tools/README.md); on stock, uninstrumented Watane it exits 1 ("missing telemetry"). Exit 2 means not run (a prerequisite the line names). By hand, on a quiet machine, not in the gate. |
| Stage D state I/O (slow) | `cargo test s128_stage_d_state_bytes_per_step -- --ignored --nocapture` in `kernel/`. Measures four-job Responses steps with the fake host at 10 and 1,000 steps. |
| Probes of the real programs | `proto/claude-probes/probes.py`, `proto/agent-spike/chatgpt-probes.py` (by hand, after a `claude` update) |

## Do not touch

- `.claude/skills/foreman`, `.claude/agents/`, `.claude/factory/`,
  `.github/workflows/factory-ci.yml`: the factory's copy, written by
  `watane-mods/factory/bin/sync.sh`.
- `ui/` by hand: the console build writes it from `console/`.

When the first slice adds the mod code, `watane-dev` writes its own guide to
a mod folder. Merge that guide into a section of this file; never let it
replace this file.
