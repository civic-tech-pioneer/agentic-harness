# Performance review of the spec chain (2026-11-20)

A record, not a contract. This is a performance review of `design/VISION.md`,
`FUNCTIONAL.md` and `TECHNICAL.md` read as one specification. Two reviewers
did it independently: a Claude session, and Codex (`gpt-6.1-sol`, high
effort, read-only). Neither ran a benchmark; figures come from the documents
and from the measurements recorded in `ARCHITECTURE-PROPOSAL.md` (V6: 20 µs
for a call that writes nothing, 4 ms for one that writes, about 240 writing
calls per second for the whole host).

**Severity.**
- *must-do*: the spec is wrong or contradicts itself, so a slice that follows
  it ships a defect.
- *should*: a real risk that is worth a spec change.

## Applied (commit 5e46864, and AGT-F-8 here)

Claude's must-do findings. Each is now in the spec.

| Id | Finding | Change |
|---|---|---|
| C1 | Replaying a message still being written (added a few hours earlier) needs the partial text in state. The kernel is a fresh instance per call, so that would mean a state write per delta: 4 ms each, on the host's one writer. That is exactly what V6 warned against. | TECHNICAL 9.4, "No partial replay": the console shows "Writing…" until the message's entry arrives. Case 30 counts the commits. |
| C2 | `attach` replays every entry after `after` in one call, but a stream holds at most 256 items or 4 MiB in flight. Once a session is long enough, reattaching failed with `full`. | 9.4: a replay holds at most 128 entries and 2 MiB; beyond that, `behind {through}`, and the client pages with `read` (capped at 256 entries and 2 MiB). AGT-F-8 says each call's work has a bound that does not grow with the transcript. |
| C3 | ChatGPT sign-in refuses bodies over about 1 MiB, but compaction happens on the server, by tokens, only after a request is accepted. A long session hit the cap before compaction could start, and nothing said what then. | 9.5, "Body budget": `compact_threshold` is at most 200 000 tokens; the body is measured before sending and trimmed (old encrypted reasoning, then old tool outputs); still over: `failed {too-large}`, then continue from a summary. |
| C4 | Entry chunks are limited by count *or* size, so index *i* cannot be mapped to a chunk, and state has no prefix scan. Every attach or read would start from chunk 0. | 9.1 and 9.3: the session record's `chunks` lists the first index of each chunk. |
| C5 | A batch is limited to 1 MiB, but one item may be up to 4 MiB. | 3: a batch always holds at least one item. |
| C6 | The new `running {activity}` was in the stored turn state. With a checked provider that meant two writes and two entries per tool call. | 9.3: the activity is pushed live only and never stored. |

## Not applied: must-do, from Codex

These came in after the product owner had handed the spec files to slice S2,
so they go to the product owner to fold into a slice.

| Id | Finding | Smallest fix |
|---|---|---|
| P01 | No limit on the serialized size of an entry. A 3 MiB message, ask input or unknown event fits a 4 MiB stream item, but not a 1 MiB transcript chunk or a 2 MiB `read` page. Only tool output is cut (64 KiB). | A limit per entry (for example 256 KiB), and lossless continuation entries for anything larger, so that paging always makes progress. |
| P02 | The whole queue (up to 32 inputs) lives in `session/<id>`, which is capped at 4 MiB. Five inputs of 900 KiB each fit the host's 1 MiB `/call` limit one at a time but overflow the record. A full `queue` update can also pass the 4 MiB item limit. | A limit per queued input and for the whole queue (for example 64 KiB and 2 MiB), and bounds on the record's other variable fields. |
| P03 | Backpressure stops at a forwarding mod. When the checked provider's own stream to the kernel is `full`, it has only two ways to answer an incoming batch: accept it (and lose what it could not forward) or fail it, and two failures end the upstream stream `consumer-failed`. `ready` signals space downstream but does not hold upstream delivery. | A "defer" outcome for `stream-sink.items`: the host keeps the batch and redelivers it after `ready`, and this does not count towards the two-failure rule. |
| P04 | The checked provider's `start` judges first and starts `claude` second, inside one call. But `https-stream.start` takes effect only after the call commits, so the verdict cannot arrive in that call. The judge deadline can also reach 118 s (`timeout-s` up to 120, minus 2), against a 10 s call budget. | A pending state for the launch and each input: return the provider's stream at once, judge across item calls, start or forward only after a safe verdict, and enforce the deadline with a timer (a late verdict is ignored, and the call is denied). |
| P05 | Every tool job is a stream, and all OpenAI sessions share the kernel's 32 open streams as a consumer. One response with 33 parallel calls is over the limit by itself. The loop has no state for waiting until a slot is free. | Keep pending calls in the loop state and start them in waves under a kernel-wide stream budget, with slots kept back for model requests; free a slot on every end. |
| P06 | Case 2 ("each request extends the previous one without moving its prefix") contradicts the body budget, which trims old items and so changes the prefix. | Case 2 holds for ordinary steps; trimming, compaction and a summary restart each start a new prefix. Add a test for each of the three. Instructions stay byte-stable. |

## Should, from both

| Id | Finding | Suggested change |
|---|---|---|
| P07 | Nothing says which wire items a compaction item replaces, so repair and request building grow with the session's whole history. | An active history: a compaction replaces the prefix it covers. The V23 probe already sends the compaction item back without the old prefix. |
| P08 | Appending to a 1 MiB chunk rewrites the whole chunk. With 4 KiB entries, one chunk costs about 128 MiB of writes. | A smaller mutable tail (for example 64 KiB) or immutable segments; measure the bytes rewritten per entry. |
| P09 | An item call carries a stream id, and there is no stream-to-session index. Finding the session must not mean reading sessions. | A `stream/<id>` → session key; a call that only forwards a delta reads no transcript or wire history. |
| P10 | 256 live timers per mod against ask expiries, retries and resumes for many owners (5 owners × 64 paused sessions = 320). Nothing handles `too-many`. | The kernel runs its own deadline list on one timer, or has a global limit with a named refusal. |
| P11 | No limit on host memory for queues as a whole: 32 streams × 4 MiB = 128 MiB per consumer, and page queues are lossless. | Byte budgets for the host and per origin; page buffering counts against them; a stalled connection ends, and the page recovers by replaying. |
| P12 | 64 MiB per owner, but the kernel mod has 256 MiB in all. Kept keys are never charged, and no space is reserved to record a quota stop. | Accounting for the whole kernel, keys included; space reserved for control records. |
| P13 | "Newest pages first" in the console, but `read` only pages forward. | `read` with a `before` cursor and a high-water index. |
| P14 / C7 | No rule bounds the console's work on a long transcript or a fast stream. | Virtualize the transcript; render at most once per animation frame; highlight code once its block is complete and visible; add a large-transcript fixture with limits for input, Stop and scroll. |
| P15 / C8 | Stage A is done when "V8 measured and recorded", with no passing number. One kernel mod serves every session through one call slot. | Pass/fail budgets per stage: p95 item delay from source to page, p95 Stop latency, the writer's busy share, bytes read and rewritten per step. Measured with 8 `claude` processes and concurrent OpenAI sessions. A step's work is bounded, so one session cannot hold the slot. |
| C9 | The body cap is for uncompressed bodies; a gzip-encoded body might raise the limit cheaply. | A probe (V28) of `Content-Encoding: gzip` on the plan endpoint. |
