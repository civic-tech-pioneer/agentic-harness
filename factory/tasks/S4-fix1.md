Folder: console
Effort: medium
Depends on: none
Slice: S4 — Alice opens the console on a scripted fake kernel and sees a session as TECHNICAL 18 describes, and after a reload the same transcript with nothing doubled.

## Goal

The adversary's breaches of slice S4 are closed: the session-allow button names the reach the
kernel grants, not a reach guessed from the agent's input; a malformed or newer entry cannot
make the console read without end or repeat a row; an ask whose tool call names another id still
shows as an open ask; and the build's policy scan catches the seven ways of running string code
or loading an outside resource that it missed.

## Acceptance

AC1. The 12 red tests pass, unchanged in what they assert, except tests 1 and 2 as stated in
     AC2: `console/kit/adv_S4.test.tsx` (5) and the "adversary S4: the policy scan" describe in
     `console/test/build.test.ts` (7). Every other test keeps passing.
AC2. Session-allow label (TECHNICAL 9.6: "The rule names that reach for the page"). Wire
     assumption, in `console/kit/wire.ts` only: an `ask` entry body may carry `reach` (string,
     given by the kernel, e.g. "Bash" or "edits in ~/work/sync"). The ask bar's second button
     reads "Allow <reach> this session" with that string, verbatim and escaped; without
     `reach` the button is not shown (the participant can still allow once or deny).
     `reachOf` and any reading of vendor input fields (`command`, `file_path`, `path`) for the
     label are removed. The story's ask carries `reach: "edits in ~/work/sync"`. Tests 1 and 2
     are rewritten to this rule: a hostile `command`/`path` in the input never appears in the
     label; the label shows the kernel's reach; no reach, no button.
AC3. The refetch of a `~p<n>` row reads entry n at most once per finishing update: if the entry
     read back is unreadable, of another kind, or a user message, the row is replaced by what
     the entry is (an `unknown` row for an unreadable one, the message for a message) or removed,
     and no `~p` row is left; a failed `read` retries at most 3 times, 1 s apart, then the row
     becomes an `unknown` row "could not be read". No row key appears twice.
AC4. An ask whose `toolCall.toolCallId` differs from its `toolCallId` is shown as an open ask on
     the row of its own `toolCallId` (the tool call's other fields used, its id ignored);
     `reduce` throws for no input (a test feeds each wire shape with wrong types and asserts no
     throw); an exception in `reduce` inside the stream callback is caught and the message
     becomes an `unknown` row.
AC5. Row keys are unique for any feed: keys built from `rows.length` are replaced by a counter
     kept in the view's internal `x`; a test feeds a thought-less message entry that replaces a
     streaming thought and agent row, then an unknown event, and finds every row in the DOM.
AC6. `behind` paging stops when a page makes no progress (`hi - 256 === hi`, an empty `read`, or
     a `through` above `Number.MAX_SAFE_INTEGER`): the view gets an `unknown` row naming the
     cause and `openSession` stops paging (test with `behind: 2**60`).
AC7. Ask arguments keep their line breaks: `.wa-args dd` (and program output, diff lines) has
     `white-space: pre-wrap`; a layout test reads the rule from the kit CSS.
AC8. `npm run build` rebuilt; the committed `ui/` matches it (the build test).

## Spec

TECHNICAL 9.4 (indexes, unreadable), 9.6 (session allows: the reach comes from the kernel's
rule), 14 ("The console has no power a client lacks"), 18.2 (page policy), 18.6 (Asks).
FUNCTIONAL AGT-F-43, AGT-F-49. The adversary's report is in the commits 18b0cf5 and 8cc6779 on
this branch (their messages and tests). Do not edit `design/`, `kernel/`, `factory/` or
`AGENTS.md`.

## Interface

Unchanged: the S4-1, S4-2 and S4-3 interfaces. `Ask` in `console/kit/model.ts` gains
`reach?: string`.

## Files

`console/kit/**`, `console/fake/**`, `console/src/**`, `console/test/build.test.ts`, `ui/**`
(built only).

## Changed tests

`console/kit/adv_S4.test.tsx` tests 1 and 2 (session-allow label): rewritten to AC2.
`console/kit/thread.test.tsx` `ac5_open_ask_shows_input_diff_and_bar_and_answers` and
`ac5_reach_falls_back_to_the_call_title`: the label now comes from `ask.reach`; the fallback
test becomes "no reach, no session-allow button". Any test that builds the story's ask label
("edits in ~/work/sync"): the same text, now from `reach`.

## Out of scope

Checking that indexes rise by one (the kernel is the authority; reported to the product owner);
a session id on stream messages; a CSP (the host's, host 8.3).
