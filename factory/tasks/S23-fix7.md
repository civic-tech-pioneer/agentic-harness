Folder: console
Effort: low
Depends on: none
Slice: S23 — the console after S7: answered asks, its risks and rough edges (adversary pass 6, BREACHED 1).

## Goal

The view App shows and answers against belongs to the current subscription, not just the current
session id: each run of the `openSession` effect (a new `info.id` or a new `kernel`) starts from an
empty view, so a view of a closed subscription never shows again, even when the reader switches
A → B → A.

## Acceptance

AC1. `console/src/adv_S23_fix6_aba.test.tsx`
     `adv_S23_fix6_aba_stale_card_of_closed_subscription_answers_reused_id` passes, unchanged.
AC2. The held view is tagged with its subscription (a token per effect run of `openSession`, whose
     deps are `kernel` and `info.id`); App shows `emptyView` unless the tag is the current
     subscription's, and `answerDrawn` sends only then. A test: same `info.id`, a new `kernel`
     prop whose attach never answers: the old card is gone and a click sends nothing to either
     kernel.
AC3. Held, unchanged: every S23 test in `console/src/` and `console/kit/`, `src/shell.test.tsx`,
     `src/App.test.tsx`, and every held case of S23-1's AC11.

## Spec

TECHNICAL 18.1, 18.3, 14 "An ask shows the tool call that will run". Brief S23-fix6.

## Interface

None.

## Files

`console/src/App.tsx`, tests in `console/src/`. Do not build `ui/`.

## Changed tests

None expected.

## Out of scope

A reopened card answered twice after a switch (the kernel refuses it, `{field: "ask"}`).
