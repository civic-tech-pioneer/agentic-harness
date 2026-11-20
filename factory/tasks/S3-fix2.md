Folder: kernel
Effort: high
Depends on: S3-fix1
Slice: S3 — a client mod's maker runs `cargo run --manifest-path kernel/Cargo.toml --example check -- <profile.json> <overlay.json> <backend> <model>` and sees every refusal at once, or none.

## Goal

The adversary's second pass (commit 9918b26) got `allowed-tools` past the frontmatter check in
three spellings and `-exec` past the `find` rule by shell expansion. Both checks copied another
program's parser and lost. This task replaces them with strict rules that refuse whatever is
not plainly safe, instead of guessing how Claude Code or bash reads it. The decision at the
security boundary (4.1 skills and allow grammar): **refuse anything not in a small, plain
form**, not "detect the dangerous forms".

## Acceptance

AC1. `cargo test --test it adv_s3` in `kernel/`: all eight tests pass, `adversary.rs`
     unchanged. Every earlier test passes, or is changed only as `## Changed tests` says.
AC2. **Strict frontmatter.** For each skill file whose name lowercased is `skill.md`: strip
     leading characters that are Unicode whitespace or U+FEFF; if what remains starts with
     `---`, the file has frontmatter and is refused (naming `skills[i].files` and the reason)
     unless all of these hold:
     - the file starts exactly with `---\n` or `---\r\n` at byte 0 (no BOM, nothing before);
     - the block up to the next line that is exactly `---` (with optional `\r`) is printable
       ASCII plus `\n`, with `\r` only right before `\n`, and no tab;
     - each line of the block is either `name: <value>` or `description: <value>` starting at
       column 0, or a continuation line that starts with at least one space; `name` and
       `description` appear at most once each.
     A file that, after that stripping, does not start with `---` has no frontmatter and passes.
     Tests: the three spellings of the red test, a flow mapping, an indented top-level block,
     and a valid two-key block with a folded multi-line description (passes).
AC3. **Shell-safe allow rules.** An `allow` rule `Bash(<arg>)` is refused when `<arg>`
     contains any of `` ` `` `$` `{` `}` `;` `|` `&` `<` `>` `'` `"` `\` or a non-ASCII
     character, naming `allow` and the character; and when its first word contains `/` ("a
     path executable the agent may rewrite"). The runner list and `find` rule stay as they are
     and now see only plain words. `Bash(git status:*)`, `Bash(npm test)`,
     `Bash(cargo build --release)` still pass. `deny` rules are not restricted by this.
AC4. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean.

## Spec

TECHNICAL 4.1 (allow grammar, skills frontmatter), 14 (first row); FUNCTIONAL AGT-F-3. Do not
edit `design/` (slice S2 rewrites it now). These are foreman decisions for the product owner's
record; the spec text follows in a later design slice.

## Interface

Unchanged.

## Files

`kernel/src/profile.rs`, `kernel/tests/it/grammar.rs`, `kernel/tests/fixtures/`. Not
`kernel/tests/it/adversary.rs`.

## Changed tests

Search `kernel/tests/it/` for allow rules that AC3 now refuses (quotes, backslashes, `/` in the
first word, `$`): the S3-1 validator's `ac5_rule_grammar_has_no_bypass` and the S3-fix1
validator's `ac3f_more_runners_and_find_exec` list rules that must be *refused*; they stay
refused (perhaps for a new reason; keep asserting refusal, adjust only an asserted reason
text). A test that asserts such a rule is *accepted* (for example a path like
`/usr/bin/git`) now asserts refusal; list each one in your report. Frontmatter tests of
S3-fix1 that assert a pass for an indented top-level block now assert refusal.

## Out of scope

An allow list for Bash commands; the openai `SKILL.md` exact-name compare in `refuse.rs`
(it refuses, so it is safe).
