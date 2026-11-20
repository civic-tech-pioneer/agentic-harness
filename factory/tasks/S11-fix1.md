Folder: kernel
Effort: medium
Depends on: S11-1
Slice: S11 — a kernel maker runs `cargo test` in `kernel/` and sees a session's transcript stored and read back as 9.4 and 9.1 say (this task: paging through a part group always makes progress).

## Goal

Forward paging with `read` through an entry larger than one page always ends, and the client
joins the entry's parts across pages byte for byte. Today `read` moves an `after` that falls
inside a part group back to the group's first part (S11-1 AC7), so a client that continues from
the last part of a page gets the same page again and loops (the validator's probe: `max` 1 over
3 parts stalls at `after` 2; a 3 MiB entry at `max` 256 stalls at `after` 9). Foreman decision,
pending the product owner (BLOCKED question of S11): option (a). `read` takes `after` as given,
both ways; only `replay` (the decision of `attach`) moves `after` to the group's first part.
A client that lost a group's start reads again with `after = i - n` itself, since every part
carries its `n`.

## Acceptance

AC1. `read` never moves `after`: `read(after, before, max)` with `after` inside a group returns
     the entries with `i > after` (forwards) or the newest with `after < i < b` (backwards),
     starting with the next part. `replay(after)` still moves it to the group's first part.
AC2. Progress (cases 34, 49): forward paging (`after` = the last `i` of each page) through a
     transcript with a 3 MiB entry (12+ parts) between small entries ends, at `max` 1, 2, 7 and
     256, returns every index exactly once, and `Joiner` fed the pages in order yields the 3 MiB
     entry once, byte for byte equal to its serialization, with `i` = its last part's index.
     Backward paging (`before` = the first `i` of each page, `after` 0) also ends and returns
     every index once; the parts, put back in ascending order, join byte for byte.
AC3. `ac10_joiner` joins a group really split across two forward pages (no repeated part 1 in
     the second page: assert the second page starts with part n > 1).
AC4. The AC7 test of S11-1 changes as below; nothing else changes; `cargo test` and
     `cargo clippy --all-targets -- -D warnings` pass in `kernel/`.

## Spec

TECHNICAL 9.4 ("A page ... may end inside a group, and the client joins the parts across
pages", "paging always makes progress", `attach`'s start at the group's first part); 16 cases
34 and 49. The 9.4 sentence that also names `read` for the move is the open question: do not
edit `design/`.

## Interface

None changes: `Transcript::read`, `replay`, `Joiner` keep their signatures.

## Files

`kernel/src/transcript.rs`, `kernel/tests/it/transcript.rs`.

## Changed tests

`ac7_after_inside_group_moves_to_group_start`: after, `read` with `after` inside a group starts
at the part after `after` (forwards and in `before` reads); `replay` still starts at the group's
first part. `ac10_joiner`: as AC3.

## Out of scope

`design/`; a `restart` flag on `read` (option (c)); anything else of S11-1.
