// Adversary S23 pass 6: A -> B -> A before B's first view. App's held view is still tagged A, so on the way back
// the closed A subscription's old view (its open ask cards) shows again before the new A subscription's first view,
// and an answer on such a card goes out to A, where the ask id may now name another tool call.
import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { decided } from "../fake/asks";
import { expiresAt, story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";
import type { Kernel } from "../kit";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
/** While the reader is away, s1's a8 is decided by another participant and a later stream opens a8 again on src/secrets.ts. */
const reopen = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e2", input: { file_path: "src/secrets.ts" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e2", title: "Edit src/secrets.ts", kind: "edit", status: "pending", locations: [{ path: "src/secrets.ts" }],
      content: [{ type: "diff", path: "src/secrets.ts", oldText: "a", newText: "b" }] } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];

describe("adv S23 fix6 A-B-A", () => {
  it("adv_S23_fix6_aba_stale_card_of_closed_subscription_answers_reused_id", async () => {
    const f = fakeKernel([...story, decided(10, "a8", "e2"), reopen(11)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    let s1attach = 0;
    // s2 never answers its attach; s1's second attach (the way back) is slow too
    const k: Kernel = { ...f, attach: (s, a, m, e) => (s === "s2" || (s === INFO.id && s1attach++ > 0) ? new Promise(() => {}) : f.attach(s, a, m, e)) };
    const { rerender } = render(<App kernel={k} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    rerender(<App kernel={k} info={{ ...INFO, id: "s2", title: "another session" }} />);
    act(() => { f.play(story.length); f.play(story.length + 1); }); // s1 moves on while the reader is on s2
    rerender(<App kernel={k} info={INFO} />);
    await act(async () => { await Promise.resolve(); });
    const card = document.querySelector<HTMLElement>('[data-ask="a8"][data-asking]');
    const allow = card?.querySelector<HTMLElement>("[data-ask-first]") ?? null;
    if (allow) await act(async () => { fireEvent.click(allow); await Promise.resolve(); });
    const sent = f.calls.filter((c) => c.name === "answer");
    // the new s1 subscription has drawn nothing yet: no card may answer, and no answer may go to s1 for the old src/e2.ts card
    expect(sent, "answered a8 in s1 from the closed subscription's src/e2.ts card").toEqual([]);
    expect(card, `a card from the closed subscription shows: ${card?.textContent?.slice(0, 80)}`).toBeNull();
  });
});
