Folder: design
Effort: medium
Depends on: S18-1
Slice: S18 — a foreman or maker who reads the spec finds every rule S14 built and S19 builds for odd host items, the accepted limits in 14, and AGT-F-10 as S17 left it.

## Goal

This fixes the adversary's breach B1 on S18, and three of its RISKs that are cheap and stay inside
the slice. Today `by: answered` closes an ask or check with no decision entry, even when the
kernel did not make that answer: the checked provider can deny an ask or answer a check itself
(10, step 6). The transcript then keeps an open-looking ask for a call that was already decided.

**Foreman's decision.** The host's `answered` item carries the outcome the host forwarded to
`claude`, and the kernel writes the decision entry only for an answer it did not make itself.

## Acceptance

AC1. In 4.5 step 4, the `answered` row is
     `{"type":"ask-resolved","id":…,"by":"answered","outcome":"allow"|"deny"|"none"}`. `outcome` is
     the answer the host sent to `claude`: `none` only for a check. Step 5 says the same in one
     clause. `expired` and `cancelled` items have no `outcome`.
AC2. Rewrite 9.8 rule 4 so it says:
     - `answered` on an ask or check the kernel answered itself (its own `answer`, 9.6) closes it
       and writes no entry, because that answer wrote the decision;
     - `answered` on one the kernel did not answer gives the decision entry, with `outcome` from
       the item. Its `by` is `checked` on a checked provider's stream (`direct: false`, 10) and
       `host` otherwise;
     - an `answered` whose `outcome` is missing or not one of the three values, or `none` on an
       ask, is `unknown`, and the ask stays open;
     - the decision of `expired` and `cancelled` has outcome `deny`, `by: host`, and `reason` =
       the `by`.
     Remove the sentence that leaves "a check answered by the checked provider becomes a
     `by: checked` decision" to stage E, or reduce it to what stage E still builds (the
     provider's side).
AC3. The 9.4 "Code to change" item for a later kernel slice says what AC2 says: no entry for the
     kernel's own answer, the decision entry from `outcome` otherwise. Case 72 matches.
AC4. 4.5 step 4 gains the host rule that 14's nested-duplicate limit relies on: "The host builds
     each `ask`, `check` and `ask-resolved` item from its own parsed value and serializes it
     itself; it never copies bytes of a `claude` line into an item." 14's "Not protected" clause
     points to that sentence. Section 15's watane checklist line for H2 (the one that lists the
     4.5 host items) names it too, if such a line exists.
AC5. 9.8 rule 1 (or 2) also says that an `ask` or `check` whose `tool` is not a string is
     `unknown`. The kernel's code already reads it that way (`str_of(v, "tool")`).
AC6. `sh .claude/factory/check-docs.sh` and `factory/gate.sh` pass. Only `design/TECHNICAL.md`
     changes. The adversary's HELD list still holds:
     - section 14 loses no property, holder or file;
     - the two "Not protected" limits are no wider than the PO decision of 2026-11-21;
     - rules 1 to 6 still make every odd item `unknown`;
     - an unknown or missing `by` is still `unknown` with the ask kept open;
     - a stdout line typed `ask-resolved` is still wrapped (4.5);
     - joined part groups read as before.

## Spec

`design/TECHNICAL.md`:
- 4.5 steps 4 and 5;
- 9.4 "Code to change (S19)";
- 9.6 (the kernel's own answer);
- 9.8 rules 1 to 6;
- 10 step 6;
- 14 "Not protected";
- 15;
- 16 case 72.

## Interface

None. S19 builds no `answered` handling. The "Code to change" item carries it.

## Files

`design/TECHNICAL.md`.

## Changed tests

None. The task changes documents only.

## Out of scope

The rest of the adversary's RISKs: when a decision is written (at the kernel's `answer` or at
the host's `ask-resolved`, RISKs 2 and 3), and a checked provider that pushes its own
`ask-resolved` (RISK 4). These go to the product owner.
