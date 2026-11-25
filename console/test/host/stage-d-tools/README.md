# Instrumented Watane tools for the Stage D load run

Run on a quiet Linux machine with Chromium and Python 3. The journey test
`test:host:d` continues to use the ordinary Watane preview binaries. The
budget test needs a test build so it can pace the scripted Responses server
and time actual commits, with fsync included.

Apply the fixture patch on a disposable Watane worktree at
`548b4e1b610fb152c2b5359d6e2aeebe6d847440` using
`git apply --unidiff-zero /path/to/agentic-harness/console/test/host/stage-d-tools/watane.patch`.
In that worktree run `cargo build --locked -p watane-host -p watane-dev --bins`
and put its `target/debug` on PATH. If `WATANE_BIN` or
`WATANE_SCRIPTED_RESPONSES` is set, point it at the matching instrumented
binary. Run `npm run test:load:d` in the harness's `console/` directory.

The runner sets `S128_LOAD_CLOCK` to a file in its disposable preview directory.
The patched scripted server alternates the two owners and phases their eight sessions 125 ms apart on a fixed
one-second grid. It timestamps each sample immediately before the TLS write.
The host logs the start and end of both host and mod commits. The runner clips
the union of those commit intervals to its measured workload window. The
instrumentation is disabled outside the budget run.
