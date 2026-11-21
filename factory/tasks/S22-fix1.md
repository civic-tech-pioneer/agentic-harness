Folder: design
Effort: low
Depends on: S22-1
Slice: S22 — one decision per ask in the record.

## Goal

The adversary's RISKs 1–5 on S22 (documents-only read, HELD otherwise) are closed in the spec text.
The foreman made these one-clause fixes itself under foreman section 4 (each in this slice's own
text, cheap to undo) and briefs them here afterwards.

## Acceptance

AC1 (RISK 1). 9.3 R4's `idle`, `unknown`, `resume-lost` row closes each Claude ask or check still
open or pending (restart / its pending answer), so a restart in any turn state leaves no Claude ask
without a decision.
AC2 (RISK 2). 9.3's `pending-answers` row has a cap of 64 and an enforced "past it": the owner's
`answer` is refused `busy` (2.3 names this `busy`), and a session allow or "no owner stream" deny
is not sent; the circular "denied `timers`" is gone.
AC3 (RISK 3). 9.8 rule 4 says the pending answer at `cancel`, R4 or `ended` is what the kernel
answered, not known to have reached `claude`; 14 "Not protected" names it, the record only.
AC4 (RISK 4). 14 "Not protected" names an open check recorded `deny, by: kernel` at `cancel`, R4 or
`ended` although the provider's `none` may have let the tool run.
AC5 (RISK 5). The `cancel` bullet and the R4 Claude row say "ask or check".
AC6. 9.4 "Code to change (S20)" lists AC1 and AC2's kernel changes. The HELD list of the
adversary's first pass still holds. `factory/gate.sh` passes.

## Files

`design/TECHNICAL.md`.

## Changed tests

None: documents only.
