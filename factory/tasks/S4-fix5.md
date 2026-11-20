Folder: console
Effort: medium
Depends on: none
Slice: S4 — Alice opens the console on a scripted fake kernel and sees a session as TECHNICAL 18 describes, and after a reload the same transcript with nothing doubled.

## Goal

Close the "calls without pause" class once, structurally, instead of path by path: every
`attach` and `read` the console makes goes through one rate governor, so no kernel answer on any
path can make it call the kernel faster than a fixed rate. And fix a paging gap an honest kernel
can hit: a short page (the 2 MiB cap of TECHNICAL 9.4) must not skip entries.

## Acceptance

AC1. The 2 red tests of commit 5717d16 pass with their assertions unchanged; every other test
     keeps passing (timings in older tests may follow AC2, as listed under Changed tests).
AC2. One governor in `console/kit/kernel.ts` wraps every `attach` and `read` of `openSession`
     (and `fetchEntry`): a token bucket of 8 calls, refilled at 4 calls per second. A call
     without a token waits for one. Tests on fake timers: 1000 calls requested at once make at
     most 8 calls in the first instant and at most 8 + 4·t in t seconds; a normal open of the
     story and a 300-entry paged open still finish (the second within 2 s of fake time).
AC3. A live item after `synced` resets the try count but not the one-immediate-reattach
     allowance (`quick`); only a stream open 10 s after `synced` restores `quick`.
AC4. Short pages: when `read(lo, 256)` returns `k` entries, `lo+1 .. lo+k`, with `k < 256` and
     `lo + k < hi`, the console reads on from `lo + k` until the range `lo .. hi` is covered,
     before moving down to the next older range. A test with a fake whose `read` returns at most
     100 entries per call and 300 entries behind ends with all 300 rows, in order, none missing.
AC5. `npm run build` rebuilt; the committed `ui/` matches it.

## Spec

TECHNICAL 9.4 (read's 2 MiB cap, attach), 18.3; the S4-fix1 to S4-fix4 briefs. Do not edit
`design/`, `kernel/`, `factory/` or `AGENTS.md`.

## Interface

Unchanged.

## Files

`console/kit/kernel.ts`, `console/kit/*.test.ts`, `console/kit/adv_S4.test.tsx` (no assertion
changed), `ui/**` (built only).

## Changed tests

Tests in `console/kit/kernel.test.ts`, `console/kit/paging.test.ts` and
`console/kit/adv_S4.test.tsx` that count calls within a fake time window may need their timer
advances to let the governor refill (AC2); what each proves stays, and no upper bound on calls
is raised.

## Out of scope

A plausibility check on `behind`; index order checks; memory bounds against a hostile kernel
(the kernel is the console's own mod and the authority on its data: reported as an accepted
limit).
