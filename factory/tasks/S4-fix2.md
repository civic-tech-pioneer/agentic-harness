Folder: console
Effort: medium
Depends on: none
Slice: S4 — Alice opens the console on a scripted fake kernel and sees a session as TECHNICAL 18 describes, and after a reload the same transcript with nothing doubled.

## Goal

The adversary's rerun breaches are closed: an entry that arrives both on the stream and from the
refetch shows once; an attach answered `behind` at or below the last index cannot spin the
console into a busy loop of attaches; and the build's policy scan catches nine more ways of
running string code or reaching an outside site.

## Acceptance

AC1. The 11 red tests of commit 67fc4e1 pass, assertions unchanged:
     `adv_S4_user_entry_on_the_stream_and_from_read_shows_twice_under_one_key` and
     `adv_S4_behind_at_or_below_the_last_index_reattaches_without_end_or_pause` in
     `console/kit/adv_S4.test.tsx`; the 9 `adv_S4_rerun_policy_scan_misses: …` in
     `console/test/build.test.ts`. Every other test keeps passing.
AC2. Dedup by key is a reducer invariant: whatever path adds a row (stream, refetch `settle`,
     replay), a row whose key is already in `rows` replaces nothing twice and never appears a
     second time; an entry that finishes a `~p<n>` row from the stream removes that `~p` row, so
     the later refetch finds no `~p` row and changes nothing. A test asserts unique keys after
     each of these orders: stream then read, read then stream, read returning null.
AC3. `connect()` never attaches twice without a pause: any `behind` that `pageIn` cannot page
     (at or below `lastIndex`, not an integer, above `MAX_SAFE_INTEGER`) ends paging with an
     `unknown` row naming the cause, and the next attach waits at least 1 s; at most 3 such
     attempts, then the console stops attaching and shows the `unknown` row.
AC4. `npm run build` rebuilt; the committed `ui/` matches it (the build test).

## Spec

TECHNICAL 9.4, 18.2, 18.3; the S4-fix1 brief. Do not edit `design/`, `kernel/`, `factory/` or
`AGENTS.md`. The scan is a regression net, not the boundary (the host's page policy, host 8.3,
is); add patterns for these nine cases without trying to make it complete.

## Interface

Unchanged.

## Files

`console/kit/**`, `console/test/build.test.ts`, `ui/**` (built only).

## Changed tests

none (the red tests are new and stay as written).

## Out of scope

Index order checks; a CSP; `reach` in the backend spec (reported to the product owner).
