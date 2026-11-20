Folder: .
Effort: medium
Depends on: S5-1
Slice: S5 — a foreman of stage A, B or D reads TECHNICAL and finds a bound for every size and wait the performance review found missing, each with the test case that proves it.

## Goal

The adversary found three breaches in the rules S5-1 wrote (TECHNICAL 10, Checked) and named
several risks. After this task the red cases 42–44 are in TECHNICAL 16 and the rules make them
pass as written; the risks the foreman settled (below) are rules with cases 45–49. Documents
only: nothing of S5-1 changes apart from what is named here.

## Acceptance

AC1. Case 42 (breach 1, "Safe" cannot widen the mode): 10 step 6 says a safe verdict on an
     `ask` lets the ask go on to the kernel unanswered (the kernel's broker and the owner decide,
     9.6); the provider answers an `ask` only with `deny` (unsafe, unsure, error, deadline). For
     a `check`, safe is `none`, as before. An owner's `allow` to an ask whose verdict is pending
     is forwarded only after a safe verdict; after an unsafe one, the ask is already denied and the
     answer is `unknown-ask`. Case 42 is added with the adversary's text.
AC2. Case 43 (breach 2, outcome 5): 10 says every input (the launch input, every `say`, an
     `interrupt` replacement) waits for `initialized` before its judge call; inputs that arrive
     before it are kept in `pending/<own stream>` in order (at most the kernel's queue, 9.3) and
     judged after it, in order; once the `claude` stream has ended (consent refused or withdrawn,
     or any end), no input is judged: the provider closes its own stream and drops them. Case 43
     is added with the adversary's text.
AC3. Case 44 (breach 3): 10 step 3 says the first readable verdict is final, and so is the
     timer; a new attempt starts only after an error or an ending with no readable verdict.
     Case 44 is added with the adversary's text.
AC4. Effects of a deferred call: 2.1 and 7 list `claude-session.say`, `answer`, `interrupt` and
     `compact` as requests (applied after the call commits), so a rolled-back (deferred or
     failed) call sends nothing to `claude`; `streams.resume` made in the call that returns
     `defer` is dropped with it. A 15 row (host H2) says so. Case 45: "An idle line with one
     queued input arrives while the owner stream is full; after `resume`, the fake `claude`
     received that input exactly once; a `timers.cancel` in a deferred call is dropped with it
     and made again on redelivery."
AC5. Judge ids: the timer is `judge/<own stream>/<on>`; `on` is the host's ask or check id, or
     `launch`, or `input/<n>` (the kernel's input number, 9.3); the provider's judge state is
     keyed by (own stream, `on`). Case 46: two sessions with the same `on` keep their own
     deadlines and verdicts.
AC6. Interrupt: an `interrupt` drops every input of the session still waiting for its verdict
     or for `initialized`; a later verdict for one sends nothing. The provider acts only on the
     first `initialized` of a stream. Case 47 covers both.
AC7. FUNCTIONAL AGT-F-32: "before a session starts" reads "before a session's first input
     reaches Claude Code"; and "If the judge says safe, then the provider shall leave the decision
     to the profile" stays (now true for asks too).
AC8. Fairness (AGT-F-55): one owner holds at most half of each slot kind (4 provider, 4 request,
     8 job slots), on top of the per-session cap; 9.5 "Slots" says so. A deferring consumer's
     `resume` inside the deferring call is dropped (AC4), so it cannot spin. Case 48.
AC9. Paging inside a part group: 9.4 says a page (`read` either way, a replay) may end inside a
     group, and the client joins parts across pages; `attach`/`read` with `after` inside a group
     still start at its first part. Case 49: paging backwards through a 3 MiB entry joins it
     whole.
AC10. `git diff --stat` lists only `design/TECHNICAL.md` and `design/FUNCTIONAL.md`; every
     other rule reads as before.

## Spec

- `factory/tasks/S5-1.md` (the slice's rules); `design/TECHNICAL.md` 2.1, 3, 4.5, 7, 9.3–9.6,
  10, 15, 16; `design/FUNCTIONAL.md` 3.6, 3.8.

## Interface

None with a sibling; the names above (`judge/<own stream>/<on>`, `launch`, `input/<n>`) are fixed.

## Files

- `design/TECHNICAL.md`
- `design/FUNCTIONAL.md`

## Changed tests

none (documents only).

## Out of scope

Everything outside TECHNICAL 2.1, 3, 7, 9.4, 9.5, 10, 15, 16 and AGT-F-32; P07–P14.

## The adversary's red cases (write them so, in the file's style)

42. (10, 14 "Safe" cannot widen the mode) A checked session in `manual` mode; the fake `claude`
    raises an ask and the scripted judge answers safe. The provider sends `claude-session.answer`
    only for `deny`; the safe ask reaches the owner unanswered, and the tool runs only after the
    owner's `allow`. Unsafe, or the deadline, gives `deny`.
43. (10, outcome 5) With the workspace consent pending, (a) `interrupt` with a replacement and
    (b) `interrupt` then `say` each leave the scripted judge server's count at zero until
    `initialized`; with the consent refused or withdrawn, a later `say` never reaches it; every
    input a `say` gives before `initialized` is judged after it, in order.
44. (10) The scripted judge answers `unsafe` (and, in a second test, `unsure`), then ends its
    stream normally: it counts exactly one request, and the check is answered `deny`. A new
    attempt starts only after an error or an ending with no readable verdict; the first readable
    verdict, or the timer, is final.
