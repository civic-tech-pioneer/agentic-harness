Folder: .
Effort: medium
Depends on: S2-fix1
Slice: S2 — fix the user-sim's rejections: a stage A or B foreman reading the specs cold hits an undefined term (V8) and a stage B "Done when" that needs stage C and E.

## Goal

A foreman starting stage A or B from the three contracts alone finds every probe id it must meet
defined in a contract, and a stage B "Done when" it can reach with stage B's parts. A few small
inconsistencies the user-sim found are fixed in passing. Documents only; the acceptance checks
are readings and greps you run and quote.

## Acceptance

AC1 (F1). `design/TECHNICAL.md` section 16 gains a short table "Probes named here" defining each
     V id the contracts cite (at least V7, V8, V26, V27): what it checks, which stage or slice
     runs it, and where its result is recorded (the record `ARCHITECTURE-PROPOSAL.md` section 18
     for V7, V26, V27; for V8 a new section in this file or `factory/PLAN.md`, your choice, named
     once). V8 is defined as the stage A prototype through three layers (host stream, a mod
     consumer, a page consumer) measuring order, delay **and** the cost of one fsync per commit
     (7, N4); its record holds the numbers, and whether to change 7 is the human's. Sections 7
     and 17 cite that table. One sentence says these V ids are probe ids, not watane's `V<n>`
     goals of `spec/VISION.md`. `grep -n -E '\bV(7|8|26|27)\b' design/*.md` (contracts only)
     shows each defined in 16.
AC2 (F2). Section 17 stage B "Done when" is reachable with stage B's parts: J11 on the fake
     **up to the consent** (the session runs in a scratch workspace of 4.6, which needs no
     consent; the plan line, the folder consent and its withdrawal are stage C), AGT-J6's Claude
     part, and cases 5, 6, 8–18, 27–30, and 31 for the turn states stage B has (not `checking`,
     not `consent`). Stage C's "Done when" adds case 31 for `consent`; stage E adds it for
     `checking`. One sentence in 17 (or 4.6) says a stage B launch uses a scratch workspace.
AC3. In passing:
     - `design/VISION.md` line about paths: `factory/…` paths are this repository's when they
       are `factory/config.toml`, `factory/PLAN.md` or `factory/tasks/`; drop `factory/…` from
       the list of watane paths (or say "watane's `factory/` other than those").
     - Section 14 row "The console has no power a client lacks" names `ui/` as well as
       `console/`.
     - The "Not protected" line of 14 adds "whether the shell grant's sandbox blocks loopback
       (probe V27, slice S1)".
     - `design/FUNCTIONAL.md` section 2 title says the rows are accepted (N3) for
       `spec/FUNCTIONAL.md`, applied by a watane slice (TECHNICAL 15), not "Proposed".
     - `consent-pending {what}` (in the 15 checklist) is named in 8 (what the requester gets
       while a consent waits, which the kernel shows as the activity `consent`, 9.3) and in 12.
AC4. `git diff --stat` touches only `design/TECHNICAL.md`, `design/VISION.md`,
     `design/FUNCTIONAL.md` (and `factory/PLAN.md` only if you put the V8 record there: then add
     only an empty "## V8 record" heading). No other rule changes meaning.

## Spec

`design/TECHNICAL.md` 4.6, 7, 8, 12, 14, 16, 17; `design/FUNCTIONAL.md` 2, J11;
`design/VISION.md` opening; `design/ARCHITECTURE-PROPOSAL.md` section 18 (record: what V7, V8,
V26, V27 are; read for why, do not edit); `factory/PLAN.md` S1 (V26, V27).

## Interface

none (documents only).

## Files

- `design/TECHNICAL.md`, `design/VISION.md`, `design/FUNCTIONAL.md`
- `factory/PLAN.md` (only the empty heading, only if AC1 puts the V8 record there)

## Changed tests

none (documents only).

## Out of scope

`paused` with no end time (stage D); the watane checkout path in `AGENTS.md`; PLAN's stale lines
(the product owner's).
