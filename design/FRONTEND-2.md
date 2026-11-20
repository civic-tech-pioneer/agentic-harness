# The agent frontend, revision 2: after the UX review (draft, 2026-11-20)

A record, not a contract, and a separate proposal: `FRONTEND.md` stays as it
was. This one reviews that brief and its canvas frames against three skills
(Nielsen's 10 usability heuristics, the Laws of UX, and UI polish) and says
what revision 2 changes. The revised frames are the second row of the design
canvas "watane agent frontend" (`Main-2`, `States-2`, `NewSession-2`); the
first row is unchanged.

Everything in `FRONTEND.md` holds unless a row below changes it.

## How it was reviewed

By task, not by page: send a message, read while it streams, answer an ask,
deny with a reason, queue while busy, stop, start a session on another
backend. Severity on the NN/g scale: 4 catastrophe, 3 major, 2 minor,
1 cosmetic. The frames were read as code; they were not opened in a browser,
so how they render and move is not verified.

## Findings

| # | Sev | Where | Rule | Problem | Revision 2 |
|---|---|---|---|---|---|
| 1 | 3 | Main transcript | H3 user control; selective attention | Every update scrolls to the bottom, so a reader who scrolled up to reread is pulled down while the agent streams. | Follow the output only while the reader is at the bottom; otherwise show a "New activity below" pill. |
| 2 | 3 | Status glyphs, all frames | H1, H4; contradicts principle 1 | Running, idle and failed are the same dot in different colours; idle is `--line` on white, about 1.2:1, nearly invisible. Principle 1 promises a shape that does not rely on colour. | One shape per state: running a dot, checking a square, needs you a diamond, idle a ring, paused two bars, unknown a dashed ring, failed a triangle. A legend on States-2. |
| 3 | 3 | Ask bar | H3 control; H5 prevention | "Allow edits this session" does not name its reach, and once given it cannot be seen or taken back. | The button says "Allow edits in ~/Notes this session"; the header then shows an "Edits allowed this session ×" chip that takes it back, with a decision line. |
| 4 | 3 | Running glyph, caret | UI polish: motion behind reduced motion | Endless pulse animations with no `prefers-reduced-motion` gate. | All motion (pulse, caret, chevron, meter, press) only under `no-preference`. |
| 5 | 2 | Status line, transcript | H1; accessibility | `role="status"` held the seconds counter, so a screen reader would announce every second; streamed text mutates a live log. | Only the word and its subject are in the live region; the clock is `aria-hidden`; a streaming message is `aria-busy` until done. |
| 6 | 2 | Main status while waiting | H1; H4 against States | Main counts up while waiting; States shows the expiry ("denied in 9:12 if unanswered"). Main used the running glyph while the judge checks; States used a checking glyph. | Main counts down to the expiry, and uses the checking shape and a `checking` phase. "Needs you" now scrolls to the ask and puts focus on Allow once, so Enter answers it. |
| 7 | 2 | Queue | H3; H9; brief promised edit | Queued messages could not be edited; removing one could not be undone; a full queue (32) made Send do nothing without a word. | Edit moves the message back into the composer; Remove shows Undo for 6 s; a full queue disables Send and the placeholder says why. More than 3 queued: "+ n more". |
| 8 | 2 | Dock | H8 minimalism | Needs you, an open plan, the queue, the status line and the input stack up to about a third of the window and push the transcript up. | The plan folds to its one-line label while an ask waits or messages are queued, unless opened by hand. |
| 9 | 2 | States | H9 errors; principle 4 | Principle 4 says a failed call always opens, but no frame drew a failed call. | States-2 adds a failed Edit card: what failed, why in plain words, that nothing was written, and what happens next. |
| 10 | 2 | Sidebar | Fitts; change blindness | The current session jumps between Needs you, Running and Recent as its state changes, possibly under the pointer. | While the pointer is over the list, rows keep their place; the glyph still changes at once. |
| 11 | 2 | Deny row | H3 emergency exit | Opening "Deny…" had no way back but answering; Esc in the composer stops the whole turn. | A Cancel button; Esc in the reason field closes the row and does not stop the turn. Esc in the composer still stops (Jakob: Claude Code and Codex do this). |
| 12 | 2 | Context meter | H1 before consequential change | The meter never warns; compaction arrives unannounced. | The bar turns amber at 80 %, its tooltip says what happens at 90 %; States-2 shows the near-full warning with "Make room now". |
| 13 | 2 | Provider chip, new session | H2 real-world words | "unchecked" reads as broken or unverified. | "no judge" and "your judge checks calls", each with a one-line tooltip. |
| 14 | 2 | All hover styles | UI polish: hover only on hover pointers | `:hover` latches on touch and looks selected. | All hover rules under `@media (hover: hover)`. |
| 15 | 1 | Buttons across frames | H4 consistency | Buttons were 32, 36 and 40 px tall on the three frames; header chips 28 px. | Buttons 36 px everywhere; chips 32 px. |
| 16 | 1 | Composer, queue item, plan | UI polish: outer radius = inner + padding | Input 14 px around a 10 px Send with 8 px padding; queue item 10 around a 9 px button with 4 px padding; plan 12 around 8 + 6. | Input 18 px; the queue's remove button 6 px; plan 14 px. |
| 17 | 1 | Selected session row, segmented control | UI polish: forced colours | A `box-shadow` ring with no border vanishes in forced-colours mode. | A transparent 1 px border under the ring. |
| 18 | 1 | Buttons | UI polish: press feedback | No pressed state. | Scale to 0.96 over 150 ms, ease-out, never when disabled. |
| 19 | 1 | Turn footer | Peak-end rule | The end of a turn lists time, tools and tokens, but not what changed. | "Completed in 9 s · 1 file changed · …"; it is where a review surface would open. |
| 20 | 1 | New session, States | H2 | "Toolsets", "Permissions" and "Waiting in watane" are our words, not the participant's. | "watane tools", "Asks", "Waiting for your answer in watane". |
| 21 | 1 | Sidebar | UI polish: contain overlay scroll | Scrolling past the end of the session list scrolls the page. | `overscroll-behavior: contain`. |

What already holds, and stays: one dominant action per view (Von Restorff);
at most three choices per ask (Hick); Needs you first in the list (serial
position); the plan's "2 of 3 done" (goal gradient); Stop answers at once with
"Stopping…" (Doherty); Enter sends and Esc stops (Jakob); Start disabled with
the reason as its label (H5); every decision kept with who made it (H1, H6).

## Principles, changed

Principle 1 now reads: a word, **a shape per state that does not rely on
colour**, the elapsed time or the time left, and the key that acts on it.

Two principles are added:

10. **The reader keeps their place.** The transcript follows new output only
    while the reader is at the bottom, and rows in the session list do not
    move under the pointer.
11. **A wide permission stays visible and can be taken back.** Anything
    allowed for the rest of a session shows in the header until the session
    ends or the participant takes it back.

## Open, not drawn

- **Keyboard answers to asks** (H7). Revision 2 makes the path Needs you,
  then Enter. A dedicated key per answer (as Claude Code's 1, 2, 3) would
  collide with typing in the composer; decide with a usability pass.
- **Several asks at once** (parallel calls). "Needs you (n)" should step
  through them in order; only one is drawn.
- **Esc while composing a queued message** still stops the turn. Kept for
  convention; watch for it in a usability pass.

## New spec gaps

In addition to the 14 in `FRONTEND.md`:

15. A kernel call to take back a session-wide allow, and an entry recording it.
16. A failed tool call carries a plain cause the page can show (for example
    "the file changed since it was read"), not only an error string.
17. The compaction threshold, so the page can warn before it is reached.
