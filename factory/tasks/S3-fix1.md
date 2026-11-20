Folder: kernel
Effort: medium
Depends on: S3-1, S3-2
Slice: S3 — a client mod's maker runs `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>` and sees every refusal at once, or none.

## Goal

The adversary's six red tests in `kernel/tests/it/adversary.rs` (commit 661266a) pass: an
overlay cannot turn a subagent into an all-tools one, a profile read through serde cannot drop
a key, duplicate subagent or skill names are refused, a skill's frontmatter cannot carry more
than `name` and `description`, skill file paths cannot name one file two ways, and the five
runner siblings are refused. Nothing else changes.

## Acceptance

AC1. `cargo test --test it adv_s3` in `kernel/`: all six tests pass, unchanged (do not edit
     `adversary.rs`).
AC2. Every earlier test passes unchanged; `cargo clippy --all-targets -- -D warnings` and
     `cargo fmt --check` are clean.
AC3. Foreman's decisions, each with a test of its own beside the red ones (in
     `tests/it/grammar.rs` or `tests/it/overlay.rs`):
     a. **No tools means none, and the document says so.** `Subagent.tools` (and `Profile.tools`)
        always serialize, `[]` included; an overlay `tools-` that empties a subagent's list is
        accepted and the compiled subagent carries `"tools": []`. (S3-1 decision: absent and
        `[]` mean no built-in tools; the launch slice passes them explicitly.)
     b. `Profile`, `Subagent`, `Skill`, `SystemPrompt` and `PreToolCheck` have
        `#[serde(deny_unknown_fields)]`.
     c. Two subagents with one `name`, or two skills with one `name` (in the profile, or in the
        compiled profile), are refused naming `subagents[i].name` / `skills[i].name` of the
        later one.
     d. A skill whose `SKILL.md` has YAML frontmatter (between leading `---` lines) with any key
        besides `name` and `description` is refused naming `skills[i].files` and the key. A
        `SKILL.md` without frontmatter, or with only those keys, passes. Refuse, never strip.
     e. Skill file paths must be plain ASCII `[A-Za-z0-9._-]` segments joined by single `/`,
        with no `.` or `..` segment and no leading or trailing `/`; two paths in one skill equal
        after ASCII lowercasing are refused (case-insensitive file systems).
     f. The allow runner list gains `gawk`, `mawk`, `nawk`, `busybox`, `toybox`, `setsid`,
        `ionice`, `chrt`, `taskset`, `unshare`, `flock`, `watch`; and `find` is refused when
        its rule carries `-exec`, `-execdir`, `-ok` or `-okdir` (or when the rule is `find:*`
        or `find *` style, which allows any arguments). `find . -name x` stays accepted.

## Spec

TECHNICAL 4.1 (subagents, skills, allow grammar), 9.2, 11; FUNCTIONAL AGT-F-3, AGT-F-4. Do not
edit `design/` (slice S2 rewrites it now). The runner list stays a deny list (the spec's
"..."); whether it should become an allow list is noted for the product owner, not decided here.

## Interface

Unchanged: the public names of S3-1 and S3-2 keep their signatures.

## Files

`kernel/src/profile.rs`, `kernel/src/overlay.rs`, `kernel/src/refuse.rs` (only if needed),
`kernel/tests/it/grammar.rs`, `kernel/tests/it/overlay.rs`, `kernel/tests/fixtures/`. Not
`kernel/tests/it/adversary.rs`.

## Changed tests

none, unless a fixture or an earlier test builds a profile with duplicate names, a skill path
with `./`, or frontmatter keys besides `name`/`description`: then fix the fixture, not the
rule, and say so.

## Out of scope

An allow list for `Bash` rules; JSON-escaped size of `--agents=`; Unicode normalisation (non-
ASCII paths are refused by AC3e instead).
