# Boundaries of the checked provider

The provider rows of `design/TECHNICAL.md` section 14 of this repository (agentic-harness, which holds
this provider in `provider-checked/`, R-B1), copied for S127 (refresh them when they change). Section 14 is the
one boundary list (`factory/config.toml` `boundaries`); this is a copy.
Paths in "Files" are relative to the repository root or to the watane checkout, as there.

| Property | Holds by | Files |
|---|---|---|
| A failing check cannot let a tool run | `onFailure: block` confirmed in `initialize` (4.5) | watane `host/src/agent/claude/init.rs`, `control.rs`; `provider-checked/mod.toml`, `provider-checked/logic/src/judge.rs`, `provider-checked/logic/src/lib.rs` |
| "Safe" cannot widen the mode | checks answer `none` or `deny`, never `allow` (4.5); the checked provider answers an ask only `deny`, a safe ask goes to the owner (10) | watane `host/src/agent/claude/control.rs`; `provider-checked/logic/src/judge.rs`, `provider-checked/logic/src/lib.rs` (a safe ask goes on unanswered) |
| A judge key goes only to its judge | the account binding of `judge` (identity, value, origin parse and lifetime as 5): the provider it was entered for, `POST`, the account's origin in the provider's `[http]`, no redirects (5) | watane `host/src/agent/https_stream.rs`, `app/` judge account; `provider-checked/mod.toml`, `provider-checked/logic/src/judge.rs`, `provider-checked/logic/src/lib.rs` |
| A mod in the middle gains no authority | host consent (8); the providers' `sessions` compare one pinned kernel identity (2.2, 10) | watane `host/` consent, `app/` consent storage; `mod.toml`; the providers' `mod.toml` and `src/lib.rs` (the unchecked one: `provider/mod.toml`, `provider/wit/world.wit`, `provider/logic/src/lib.rs`, `provider/BOUNDARIES.md`); the checked one: `provider-checked/mod.toml`, `provider-checked/wit/world.wit`, `provider-checked/logic/src/lib.rs`, `provider-checked/BOUNDARIES.md`; `factory/publish-test-mods.sh` pins the release identity |
