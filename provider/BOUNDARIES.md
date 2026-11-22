# Boundaries of the unchecked provider

The provider rows of `design/TECHNICAL.md` section 14 of the agentic-harness repository, copied at its
commit `7443416e86e4` (refresh them, and this commit, when they change). Paths in "Files" are relative to
that repository or to the watane checkout, as there.

| Property | Holds by | Files |
|---|---|---|
| A failing check cannot let a tool run | `onFailure: block` confirmed in `initialize` (4.5) | watane `host/src/agent/claude/init.rs`, `control.rs`; checked provider's `mod.toml`, `src/judge.rs` |
| "Safe" cannot widen the mode | checks answer `none` or `deny`, never `allow` (4.5); the checked provider answers an ask only `deny`, a safe ask goes to the owner (10) | watane `host/src/agent/claude/control.rs`; checked provider's `src/judge.rs` |
| A judge key goes only to its judge | the account binding of `judge` (identity, value, origin parse and lifetime as 5): the provider it was entered for, `POST`, the account's origin in the provider's `[http]`, no redirects (5) | watane `host/src/agent/https_stream.rs`, `app/` judge account; checked provider's `mod.toml`, `src/judge.rs` |
| A mod in the middle gains no authority | consent per origin client (8); the providers' `sessions` refuse any caller but the kernel (2.2, 10) | watane `host/` consent per origin client, `app/` consent storage; `mod.toml`; the providers' `mod.toml` and `src/lib.rs` (the unchecked one, until R-B1: `provider/mod.toml`, `provider/wit/world.wit`, `provider/logic/src/lib.rs`, `provider/BOUNDARIES.md`) |
