Folder: kernel
Effort: medium
Depends on: S33-1
Slice: S33 — session allows fail closed: a session allow never covers more than the participant saw

## Goal

A `Glob` session allow holds only searches inside the workspace. Today
`broker::reach` builds `Glob(<ws>/**)` from `path` alone, but Claude Code's
Glob searches the base folder of an absolute `pattern` and ignores `path`, so
an allow given for `{path: "/w/src", pattern: "*.rs"}` also answers
`{path: "/w/src", pattern: "/home/*/.ssh/*"}` (adversary breach, red test
`adv_s33::adv_s33_glob_absolute_pattern_escapes_workspace`, commit 932f6b7).

The rule (TECHNICAL 9.6, "As built (S31)", edited by the foreman; quoted):

> a `Glob` also needs a `pattern` that is a string not starting with `/`
> or `~`, with no `\` and no `..` segment (split on `/`), since
> `claude` searches an absolute pattern's own folder and ignores `path`
> (adversary, S33-fix1);

Any other `Glob` input (no `pattern`, a non-string one, or one that breaks
the rule) gives no reach. `Grep` is unchanged (its `glob` only filters under
`path`).

## Acceptance

AC1. The red test `adv_s33::adv_s33_glob_absolute_pattern_escapes_workspace`
passes; nothing else changes beyond the criteria below.
AC2. Unit (`tests/it/broker.rs`), workspace `/w`, `path` `/w/src`: `pattern`
`*.rs`, `src/**/*.ts`, `a/b` -> `Glob(/w/**)`; `pattern` `/etc/*`,
`~/x`, `~`, `../x`, `a/../../x`, `..`, `a\\b`, `5`, absent -> no reach.
AC3. The `Glob` line of `ac1_reach_of_file_tools`
(`{"path": "/w/src"}` -> `Glob(/w/**)`) gets a `pattern` (`"*"`), since a
real Glob always has one, and keeps asserting `Glob(/w/**)`; the
`{"pattern": "*"}` (no path) line keeps `None`.
AC4. The S33 HELD attacks stay held: every test in `adv_s31.rs`,
`adv_s33.rs`, `broker.rs`, `allows.rs` passes.
AC5. `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` and `factory/gate.sh` pass.

## Spec

`design/TECHNICAL.md` 9.6 "As built (S31)"; 14 "A session allow can be taken
back".

## Interface

`broker::reach` signature unchanged.

## Files

`kernel/src/broker.rs`, `kernel/tests/it/broker.rs`.

## Changed tests

`broker.rs` `ac1_reach_of_file_tools`: the `Glob` input gains `"pattern": "*"`
(AC3). Searched: `grep -rn '"Glob"' kernel/tests/it` lists only `broker.rs`
and `adv_s33.rs`.

## Out of scope

Symlinks inside a workspace, odd bytes in paths, `mcp__watane__x__y` (PO
risks in the report); the host.
