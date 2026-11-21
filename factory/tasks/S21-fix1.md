Folder: kernel
Effort: medium
Depends on: none
Slice: S21 — a kernel maker drives sessions through 9.3 on the in-memory host-7 state of S11
## Goal
The queue's 2 MiB cap counts the bytes the queue actually stores and pushes, so inputs of an
allowed size can never take `queue/<id>` or the `{"type":"queue"}` push past the host's 4 MiB
value (adversary breach 1 of S21).
## Acceptance
AC1 The red test `adv_s21_queue_of_escaped_inputs_passes_the_host_value` in
    `kernel/tests/it/adv_s21.rs` passes as written... except its expectation of `Ok(8)` for the
    8th input: decided by the foreman, the cap counts each input's stored size (its serialization
    as a JSON string, escapes included), so escape-heavy inputs reach `QueueFull` sooner. Change
    that one expectation to: every `say` returns `Ok` or `Err(QueueFull)`, never `TooLarge`, and
    the stored `queue/<id>` and `push_value()` serialization stay at most 2 MiB plus 64 KiB of
    framing. Keep the rest of the test.
AC2 A test: 3 inputs of 256 KiB of control characters (6x when escaped) give `QueueFull` before
    the stored queue passes 2 MiB + 64 KiB; plain-text inputs still fit 8 x 256 KiB less framing
    (assert the 8th plain input of `262_144 - 64` bytes fits, or note the exact count).
AC3 Nothing else changes; all other tests pass unchanged.
## Spec
TECHNICAL 9.3 "Sizes" (queue at most 2 MiB "so the queue update and queued carry at most 2 MiB
of inputs, under the 4 MiB item").
## Interface
`Queue::push` keeps its signature; the counted size is the stored size.
## Files
kernel/src/queue.rs, kernel/tests/it/adv_s21.rs (only that test's one expectation),
kernel/tests/it/record.rs (only if an AC5 test of the queue depends on raw bytes; say which).
## Changed tests
`adv_s21_queue_of_escaped_inputs_passes_the_host_value`: `Ok(8)` becomes "Ok or QueueFull, never
TooLarge, stored value bounded". Any `ac5_*` queue test in record.rs that fills 2 MiB with plain
inputs: adjust only if the framing changes its count.
## Out of scope
Everything outside the queue.
