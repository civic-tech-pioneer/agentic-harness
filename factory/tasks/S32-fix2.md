Folder: console
Effort: low
Depends on: none
Slice: S32 — stage A, end to end (adversary pass 1: RISKs 3 and 4).

## Goal

The served console never gets stuck on a deleted probe and never shows the fake kernel when
served. Today `console/src/served.ts` starts the probe with the fixed key `console-probe`; kept
keys outlive a delete (9.1), so after the owner deletes it every load gets the deleted id back and
fails `unknown-session`. And when `/watane.js` fails to load on a served page, `main.tsx` mounts
the scripted fake, against 18.2 ("never the fake when served").

## Acceptance

AC1. When `list` holds no probe session, `probe.start` is called with a fresh key
     (`console-probe-<time in ms>-<random>`, at most 64 bytes, TECHNICAL 9.1 key grammar), so a
     deleted probe's key is never reused; a test: list empty, start answers a new id, the page
     opens it; a second mount with list still empty sends a different key.
AC2. The page knows it is served by its origin, not by `window.watane` alone: served when the
     host is a `<local>.localhost` origin (host 8.1) or `/watane.js` was requested; when served
     and `window.watane` is missing, it shows the cause in `#root` ("watane.js did not load") and
     never mounts the fake. `npm run dev` (vite's `localhost:<port>`) and the slow tests keep the
     fake. Test it with the served helper (`console/src/served.ts`) and a unit test of the choice.
AC3. Nothing else changes: `npm test` and `npm run test:slow` pass with every assertion unchanged.
     Held cases: S9 `kit/adv_S9*.test.tsx`, `kit/regress_S9_*`, `test/adv_S9_*.slow.mjs`,
     `test/regress_S9_*.slow.mjs`; S15 `kit/adv_S15.test.tsx`, `kit/regress_S15_*`; S7
     `src/follow_S7_fix2.test.tsx`, `src/follow_S7_fix3.test.tsx`, `test/adv_S7_*.slow.mjs`; S23
     `kit/adv_S23_*`, `src/adv_S23_*`, `kit/regress_S23_status_subject_after_answer.test.tsx`,
     `kit/s23_fix5_open_invariant.test.ts`, `test/adv_S23_*.slow.mjs`, `test/regress_S23_*.slow.mjs`;
     S30 `kit/short_S30_1.test.ts`, `src/short_S30_1.test.tsx`, `src/s30_2_status.test.tsx`,
     `kit/s30_2_status.test.tsx`, `kit/adv_S30_status_names_the_open_ask.test.tsx`; S32
     `kit/watane.test.ts`, `src/served.test.tsx`, `test/served_S32.slow.mjs`.

## Spec

TECHNICAL 18.2 (the kernel served by watane), 9.1 (client keys), 9.9; watane host 8.1, 8.5.

## Interface

None.

## Files

`console/src/served.ts`, `console/src/main.tsx`, `console/src/served.test.tsx`. Do not build or
commit `ui/` (the foreman does).

## Changed tests

`src/served.test.tsx` `ac4_starts_a_probe_when_there_is_none`, if it asserts the key
`console-probe`: it asserts the key's prefix `console-probe-` after.

## Out of scope

The kernel (S32-fix1); reconnect (the 15 need).
