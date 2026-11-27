# Instrumented Watane tools for the Stage D load run

Run on a quiet Linux machine with Chromium and Python 3. The journey test
`test:host:d` continues to use the ordinary Watane preview binaries. The
budget test needs a test build so it can pace the scripted Responses server
and time actual commits, with fsync included.

Apply the fixture patch on a disposable Watane worktree at
`6cfe4c38762bf9310a8424de021ae21f6fc279aa` using
`git apply --unidiff-zero /path/to/agentic-harness/console/test/host/stage-d-tools/watane.patch`.
Use that exact revision. Agentic-harness uses the host's resolved timer firing
origin, which the older `548b4e1b` fixture revision does not export.
In that worktree run `cargo build --locked -p watane-host -p watane-dev --bins`
and put its `target/debug` on PATH. If `WATANE_BIN` or
`WATANE_SCRIPTED_RESPONSES` is set, point it at the matching instrumented
binary. Run `npm run test:load:d` in the harness's `console/` directory.

The runner sets `S128_LOAD_CLOCK` to a file in its disposable preview directory.
The patched scripted server alternates the two owners and phases their eight sessions 125 ms apart on a fixed
one-second grid. It timestamps each sample immediately before the TLS write.
The source trace tags each tool call and completion with its step. The host
logs a receipt for each marked delta and tool call whose request sink returned
`taken`. A missing page marker with a receipt by the stored interrupt fails
coverage. The mismatch report includes wire writes, sink receipts, page items
and the stored interrupt. The browser stamps receipt and Stop with `Date.now()`
on the same machine. Each paced reply
carries its step marker in a live delta and a final assistant message. The
runner measures the delta and checks the transcript for the final marker as
evidence of admission. The fake Claude binary and its pipe wrapper both stamp
their writes, so the load report can locate a gap before or after the pipe. The runner keeps its
existing forwarding cadence and latency checks.
The host logs the start and end of both host and mod commits. The runner clips
the union of those commit intervals to its measured workload window. The
instrumentation is disabled outside the budget run.

To reproduce the Stop race, run `npm run test:load:d -- --controlled-cut` with
the same binaries. The fixture holds the first OpenAI page's final marked step
in the host before it is queued for the request sink. The runner clicks Stop
once the server has written the complete step and the host reports the hold.
It checks that the host cancelled the hold, no held marker reached the sink or
page, and the page's interrupted turn was stored. The ordinary run leaves the
hold disabled.
