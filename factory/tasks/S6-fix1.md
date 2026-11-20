Folder: .
Effort: medium
Depends on: S6-1, S6-2
Slice: S6 — a foreman of stage B or D reads TECHNICAL and finds the remaining bounds of the performance review, and the rules S3 decided in code, written where they belong.

## Goal

The adversary's breach on S6 is closed in the text, and its RISKs that the foreman settled are
written in. The breach: 4.1 says interpreter and runner prefixes are refused by family ("The
list is in `kernel/src/profile.rs`"), but the code's list lacks many such programs (`tsx`,
`tmux`, `systemd-run`, `script`, `strace`, `at`, `crontab`, ...), so the text approves gaps that
let an allow rule run any command without an ask. After this task 4.1 names every refused first
word itself (the text is the rule; the code's list is a floor that "Code to change" raises), and
the other decisions below are rules. Every choice is fixed in `## Interface`. Documents only:
`design/TECHNICAL.md`. Quote each shell check's output in your report.

## Acceptance

AC1. (the breach) 4.1 lists, by name, every first word an allow `Bash(...)` rule may not start
     with: every entry of `RUNNERS` in `kernel/src/profile.rs` plus each name of `## Interface`
     (Added names). The sentence "The list is in `kernel/src/profile.rs`" is gone or now says the
     code's list must equal this one. Check:
     `for w in tsx ts-node tmux screen runuser pkexec chroot nsenter systemd-run script strace parallel fakeroot sg tclsh rscript julia expect irb elixir at crontab ltrace gdb; do grep -q "\`$w\`" design/TECHNICAL.md || echo "missing $w"; done` prints nothing.
AC2. 9.2 "Code to change" lists: the added names (one line naming them, or "the names of 4.1
     missing from `RUNNERS`"), and the frontmatter rule of AC6.
AC3. 9.6: an expired kernel ask is denied, reason `expired`; the session is not allowed
     anything by it (`## Interface`, Expiry).
AC4. 9.3 "Deadlines and the session timer": one owner holds at most 120 of the 240 session
     timers (half, as 9.5's slots); a child session counts against its root's owner; at the
     owner's cap, the same four outcomes as at the kernel's cap.
AC5. 9.3 "Quota": the writes allowed into the 32 MiB reserve are named (`## Interface`, Reserve);
     everything else past the account is refused `quota`.
AC6. 4.1 frontmatter: an indented (continuation) line may not contain `:`.
AC7. 14: the "A session is private" row also names the files that will hold the 9.1 stream and
     timer routing, and the wrapper's root `src/lib.rs` (`## Interface`, 14); section 1 no longer
     calls `mod.toml` planned (it exists; check `ls mod.toml`, and if it does not exist, leave 1).
     "Not protected" gains the two lines of `## Interface` (Not protected).
AC8. Nothing else changes meaning: `git diff --stat` lists only `design/TECHNICAL.md`;
     `.claude/factory/check-docs.sh` passes.

## Spec

`design/TECHNICAL.md` 1, 4.1, 9.2, 9.3, 9.6, 14; `kernel/src/profile.rs` (`RUNNERS`,
`frontmatter_problem`; read only).

## Interface

**Added names (4.1).** Refused as a first word besides `RUNNERS`: `tsx`, `ts-node`, `tmux`,
`screen`, `runuser`, `pkexec`, `chroot`, `nsenter`, `systemd-run`, `script`, `strace`, `ltrace`,
`gdb`, `parallel`, `fakeroot`, `sg`, `tclsh`, `rscript`, `julia`, `expect`, `irb`, `elixir`,
`at`, `crontab`. Give the whole list (the code's and these) as one alphabetical list in 4.1, with
one line on why: each runs, wraps or schedules another command. A word not on the list is not
thereby safe: the shell grant's sandbox (4.2) is the limit (see Not protected).

**Expiry (9.6).** "A kernel ask that reaches its `ask-expiry` is denied, reason `expired`, as the
host denies a Claude ask at its 10 minutes (4.5); an `ask-expiry` of 0 denies at once."

**Timers per owner (9.3).** At most 120 live session timers per owner (half of 240), a child
counting against its root's owner (as 9.5); past it, the four outcomes of the kernel's cap.

**Reserve (9.3 Quota).** Past the kernel's 224 MiB account (or an owner's quota) only these
writes go on, into the reserve: the entry and turn state that record a stop, ask decisions
(including denials by expiry), `stream/` and `live/` keys of an `attach`, slot and timer counts,
and every `delete`. `rename`, a session allow (`for: session`) and new input are refused `quota`.

**14.** Row "A session is private" adds: `kernel/src/sessions.rs` holds the 9.1 stream routing
(`stream/`, `live/`) and the session timer (9.3); a slice that puts either in another file names
that file here; and the mod's root `src/lib.rs` if the WASM wrapper's scaffold puts the exports
and owner checks there. Not protected gains: "an allowed package-runner word (`test`, `install`,
`add`, `ci`, `uv pip`, `sync`, ...) runs the project's own scripts, and an allowed command's own
options (`git -c`, GNU `sed`'s `e`) can run others: inside the shell grant's sandbox (4.2) only;"
and "kept client keys are charged forever (9.3), so owners acting together can fill the kernel's
account for good: an accepted limit until a later slice frees kept keys (PLAN)".

## Files

`design/TECHNICAL.md`.

## Changed tests

none (documents only).

## Out of scope

Changing `kernel/` (the next kernel slice applies "Code to change"); a TTL for kept keys.
