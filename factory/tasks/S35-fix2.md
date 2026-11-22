Folder: provider
Effort: low
Depends on: none
Slice: S35 — stage B, end to end: Alice has a first conversation with Claude on dev's fake `claude`

## Goal

The console sends a plain text input (`"hi"`), which the kernel reads as one text block
(`kernel/src/sessions.rs` `user_content` and `input_text`: an input that is not a JSON array is
its own text). The provider reads it as "no text" and never says it, so on watane's real host the
first message never reaches `claude` and the turn stays Working. After this task the provider
reads an input exactly as the kernel's `input_text` does.

## Acceptance

AC1 (red first). `input_text("hi")` is `Some("hi")`; the start of a plain input `"hi"` records
     start, `say S hi`, hand-off.
AC2. A JSON array of blocks: as today (text blocks joined with "\n\n"; `None` when none). An
     empty input `""` is `None` (no say). Any other JSON value or non-JSON text: the input itself.
AC3. `ac3_no_text_means_no_say` keeps its array cases and drops the plain-text case it had
     (`"plain"` now says). `watane-dev test --json --dir provider` and the gate's provider lines pass.

## Spec

`design/TECHNICAL.md` 2.1 "The kernel's calls on a provider stream" (the input's text), 10
"Unchecked".

## Interface

None changes.

## Files

provider/logic/src/lib.rs.

## Changed tests

`provider/logic/src/lib.rs` `ac3_no_text_means_no_say`: its plain-text case now says (AC3).

## Out of scope

Non-text blocks (later slice).
