# Where the vendored WIT comes from

| File | Source | Commit |
|---|---|---|
| `deps/watane-host/host.wit` | watane `host/wit/host.wit` (`watane_host::HOST_WIT`) | watane `c2a7d4d` (main, "plan: S101 done (PR #14)") |
| `deps/claude-code/world.wit` | this repository's `provider/wit/world.wit`, as `watane-dev add ../../provider --dir kernel/wasm` copies it | this branch |

`host.wit` is byte-exact: `watane-dev check` rewrites it whenever its bytes differ from the
`HOST_WIT` of the watane-dev that runs (dev TECHNICAL 5.1 step 2), so it carries no comment of its
own and its commit is recorded here. `provider/wit/deps/watane-host/host.wit` is the same file.
When a newer watane-dev rewrites it, update the commit in this table in the same commit.
