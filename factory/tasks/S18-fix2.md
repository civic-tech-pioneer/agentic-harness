Folder: design
Effort: low
Depends on: S18-fix1
Slice: S18 — a foreman or maker who reads the spec finds every rule S14 built and S19 builds for odd host items, the accepted limits in 14, and AGT-F-10 as S17 left it.

## Goal

Close the adversary's breach B2 on S18-fix1 and the two wording RISKs it raised. The foreman made
these fixes as a decision of section 4: each fix is one clause, in the spec that owns the fact.

## Acceptance

AC1. An `answered` item with `outcome: allow` on a check is `unknown`. 9.4 "Code to change",
     9.8 rule 4 and case 72 say so.
AC2. 9.8 rule 4 says an ask or check the kernel answered itself stays open until the host's
     `ask-resolved` arrives, and counts as decided until then:
     - a second `answer` is refused `{field: "ask"}` (case 9);
     - no session allow, `ask-expiry` or restart step answers it again;
     - it is not counted as open for the owner.
AC3. The 4.5 `wrapped` row (user-sim polish) says the kernel keeps the host's U+FFFD `line`, and
     that the `\xNN` form of 9.8 applies only to bytes that reach the kernel without the host.
AC4. `check-docs.sh` and `factory/gate.sh` pass, and the adversary's HELD list still holds.

## Files

`design/TECHNICAL.md`.

## Changed tests

None. The task changes documents only.

## Out of scope

Adversary RISKs that go to the PO.
