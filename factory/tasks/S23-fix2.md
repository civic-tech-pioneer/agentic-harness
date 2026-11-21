Folder: console
Effort: low
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary BREACHED 3 and 4).

## Goal

A fence label that names an inherited object key (`constructor`, `__proto__`, `toString`, ...) is
not a language, and no failure to post a block to the Worker can hold the highlight queue past
one budget: the block stays plain and the next block starts at once.

## Acceptance

AC1. `console/kit/adv_S23_highlight.test.ts` `adv_S23_fence_label_constructor_is_not_a_language`
     passes, unchanged: `known()` reads only own keys of its table (e.g. `Object.hasOwn`, or a
     `Map`, or a null-prototype object).
AC2. Same file, `adv_S23_fence_label_constructor_stalls_the_queue_past_the_budget` passes, unchanged.
AC3. `postMessage` (load or tokens) that throws fails that job at once: the block resolves `null`
     (plain, cached as plain), the Worker is ended, no timer is left armed, `busy` is cleared, and
     the next queued block starts; no uncaught error. A unit test with a fake Worker whose
     `postMessage` throws for one block shows the next block highlighted well inside 250 ms.
AC4. Held, unchanged: `kit/highlight_S23_2.test.ts`, `kit/code_S7_3.test.tsx`,
     `test/highlight_backtrack.slow.mjs`, `test/build.test.ts`.

## Spec

TECHNICAL 18.2, 18.6 Long sessions (the per-block budget). Brief S23-2.

## Interface

None with siblings.

## Files

`console/kit/parts/highlight.ts`, tests in `console/kit/`. Do not build `ui/`.

## Changed tests

None.

## Out of scope

A cap on the queue length and retrying a block that ran over budget on a busy CPU (adversary RISKs
c and d: accepted limits, noted for the PO).
