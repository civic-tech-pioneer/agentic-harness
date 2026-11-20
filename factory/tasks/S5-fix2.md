Folder: .
Effort: medium
Depends on: S5-fix1
Slice: S5 — a foreman of stage A, B or D reads TECHNICAL and finds a bound for every size and wait the performance review found missing, each with the test case that proves it.

## Goal

User-sim rejected S5 on five readings a foreman could not brief from. After this task each
reading has one rule and a numbered case, and the smaller gaps it noted are closed. Documents
only (`design/TECHNICAL.md`, `design/FUNCTIONAL.md` if a requirement changes).

## Acceptance

AC1. Deferral always ends (3, 2.1, 9.4, 10, 15). 2.1: `stream-producer` gains
     `ended: func(s: string, outcome: outcome)` (host-only export), so "each side gets `ended`"
     (3) has a function on the producer side; a 15 row (host H1, pkg 4.5) says so. Rules:
     - a forwarding mod resumes its deferred upstream when its downstream stream ends (any
       outcome: closed, `replaced`, `lagging`, page gone); the kernel then has no live stream
       and stores entries without pushing them; the checked provider whose downstream ends
       cancels its `claude` stream;
     - the kernel holds a deferral of a session's upstream for at most 10 s (timer
       `lagging/<session>`); then it closes the owner's live stream `lagging` and resumes
       upstream (so a page that stays connected but stops reading cannot freeze a session);
     - the host backstop: a stream deferred for 5 minutes without `resume` ends
       `consumer-failed`.
     Case 51 covers: owner stream ends mid-deferral -> upstream resumes, nothing lost from the
     transcript; a page that stops reading -> `lagging` after 10 s, reattach misses and doubles
     nothing; a consumer that never resumes -> `consumer-failed` after 5 minutes.
AC2. Provider slots (9.3, 9.5, 12, 2.3). A `start` or a `say` that needs a Claude session's
     provider stream when its owner holds 4, or all 8 are held, is refused with a new kernel
     error `busy` (2.3 `error` gains `busy`; 12 lists it; the message names the cap). It never
     waits. A Claude session idle for 10 minutes releases its provider stream (the kernel
     cancels it; the next `say` resumes the Claude session with `session {resume}`, 4.1), so
     slots do not stay held by idle sessions. Case 52.
AC3. Trimming moves the prefix once (9.5 "Body budget"). When a body is over 896 KiB, the
     kernel sets a trim mark at the first item of the last 4 steps, and trims every item before
     the mark (reasoning `encrypted_content` left out oldest first, then tool outputs cut to
     8 KiB, marked) until the body is at most 768 KiB. The mark is kept in the loop state and
     does not move until a body is over 896 KiB again; every later request sends the items
     before it trimmed the same way, so it extends the trimmed request. Still over 1 MiB after
     trimming everything before the mark: `failed {too-large}`, as before. Case 40's trimming
     test says "the next 3 requests extend the trimmed one".
AC4. Cases: B's "Done when" names "36 (kernel part)". New case 53: a client key over 256
     bytes and a `rename` over 256 bytes are `too-large(256)`; the 65th session allow is
     `too-large(65536)` and its ask stays open; a session at 16 384 chunks ends its turn
     `stopped {quota}` and new input is `quota`.
AC5. `allows` cap is stated in bytes: "at most 64 session allows and 64 KiB in all, each at
     most 1 KiB"; past it `too-large(65536)`. `too-large(u64)` always carries bytes.
AC6. Small gaps, one sentence each where they belong:
     - 17: "Writer busy share: the share of wall time the host's state writer (host 7, one per
       host) spends in commits, fsync included, over the load run."
     - 17 loads: B and D run with 2 owners (two client mods), so the per-owner caps hold;
       stage A's 8 host streams come from a test source in the host's own tests (no H2–H4).
     - 9.3: `stopped {limit}` names the limit; the chunks cap ends the turn
       `stopped {limit: quota}` (use that form everywhere it says `stopped {quota}`).
     - 9.3: the `queue` update and `queued` carry at most 2 MiB of inputs, under the 4 MiB item.
     - 9.5: a FIFO entry whose session no longer waits (interrupt, cancel, delete, restart) is
       skipped and removed when reached.
     - 9.4 / 12: `behind {through}` gives the session's high-water index (`last` of `read`).
     - 10 step 5: an unsafe verdict on a later input denies that input only: a verdict item,
       the turn ends `idle {failed {judge}}`, the `claude` stream stays; only the launch's
       unsafe verdict cancels it (step 4). AGT-F-32 unchanged.
AC7. `git diff --stat` lists only the two design files; nothing else changes meaning; every
     new or changed rule's case is named in its stage's "Done when" (51: A host part, B kernel
     part, E provider part; 52: B; 53: B).

## Spec

`factory/tasks/S5-1.md`, `factory/tasks/S5-fix1.md` (the slice's rules); `design/TECHNICAL.md`
2.1, 2.3, 3, 9.3–9.5, 10, 12, 15, 16, 17.

## Interface

None with a sibling. Fixed names: `stream-producer.ended`, `busy`, `lagging/<session>`,
`too-large(256)`, `too-large(65536)`, `stopped {limit: quota}`.

## Files

- `design/TECHNICAL.md`
- `design/FUNCTIONAL.md` (only if a requirement's words change; none is expected)

## Changed tests

none (documents only).

## Out of scope

P07–P14; `kernel/`, `console/`, `ui/`, `factory/`.
