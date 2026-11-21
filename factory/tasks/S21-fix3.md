Folder: kernel
Effort: low
Depends on: none
Slice: S21 — a kernel maker drives sessions through 9.3 on the in-memory host-7 state of S11
## Goal
Continue after a restart works with a full kept queue (user-sim rejection of S21, R5): a `say`
that starts a turn on a session with queued inputs takes the oldest first and then queues the
new input, so the queue never passes its caps and the call is not refused `queue-full`.
## Acceptance
AC1 The red test `regress_s21_continue_full_queue` in `kernel/tests/it/usersim_s21.rs` passes
    unchanged.
AC2 The same holds for `idle` and `paused {until: None}` sessions with a full queue (32 inputs):
    `say` starts the oldest, the new input joins the end, the queue holds 32; a test.
AC3 When taking the oldest and adding the new input would still pass 2 MiB (stored bytes,
    S21-fix1), the call is refused `QueueFull` and writes nothing (no key, no request).
AC4 The adversary's held attacks still hold: `adv_s21_queue_of_escaped_inputs_passes_the_host_value`,
    `ac2_queue_cap_counts_stored_bytes`, and every other test passes unchanged.
## Spec
TECHNICAL 9.3 "Restart" R5, queue bullets and Sizes.
## Interface
none changes.
## Files
kernel/src/sessions.rs, kernel/src/queue.rs (only if needed), kernel/tests/it/sessions.rs (new tests).
## Changed tests
none.
## Out of scope
Everything else.
