Folder: console
Effort: medium
Depends on: S15-1
Slice: S15 — Alice's console works against the kernel JSON of S10 unchanged.

## Goal

The adversary's breach of S15 (TECHNICAL 14, "An ask shows the tool call that will run"): with two
open entries (two asks, or an ask and a check) on one `toolCallId`, the card answers one entry but
shows the other entry's call. After this task the card always shows the call of the very entry
its bar answers (or, for a check, the check it shows): its header, kind, question, diff and Input
come from that entry's own `toolCall` and `input`; the status line's subject too.

## Acceptance

AC1. The red tests in `console/kit/adv_S15.test.tsx`
     (`adv_S15_second_entry_on_one_call_relabels_the_first_asks_card`,
     `adv_S15_check_after_ask_relabels_the_asks_card`) pass, unchanged.
AC2. Nothing else changes: every other test passes unchanged, including
     `console/kit/s15_1.test.tsx` (ask before check, one at a time), `adv_S4.test.tsx`,
     `adv_S9.test.tsx`, `adv_S9_diff.test.tsx`, `frame_S9.test.tsx`, and `npm run test:slow`
     (`adv_S9_ask_scroll_hides_input`, `regress_S9_needs_you_focus_visible`,
     `regress_S15_layout`). When the shown entry is decided and another stays open on the call,
     the card switches to the open one's own call; a decided card keeps the call of the entry
     decided last when none stays open.
AC3. `npm run typecheck` passes; `ui/` rebuilt and committed.

## Spec

`design/TECHNICAL.md` 9.4 (the ask body), 9.8 (the ask's `toolCall` and `input` are the truth),
14 (row "An ask shows the tool call that will run"), 18.3 (rows `ask`).

## Interface

None new. A possible shape: keep each entry's own normalized call with the entry (in the reducer's
internal `x` or on the row's ask list) and draw the row's call from the picked entry.

## Files

`console/kit/reduce.ts`, `console/kit/model.ts` (internal fields only), `console/kit/parts/rows.tsx`,
`console/kit/status.tsx`, and `ui/` by `npm run build`.

## Changed tests

none (searched `console/kit/*.test.*`, `console/src/*.test.*`, `console/test/*`).

## Out of scope

Whether the kernel should refuse two entries on one call id (a spec question for a kernel slice).
