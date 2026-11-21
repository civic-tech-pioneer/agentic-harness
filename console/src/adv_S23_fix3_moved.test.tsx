// Adversary S23 pass 3: an ask id moved to another call leaves the old card holding the stale ask, still asking.
import { describe, expect, it } from "vitest";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { expiresAt, story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
/** The kernel opens a8 again on another call (e9), with another input, before any answer. */
const elsewhere = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e9", input: { command: "rm -rf ~" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e9", title: "rm -rf ~", kind: "execute", status: "pending" } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`);

describe("adv S23 fix3 moved", () => {
  it("adv_S23_fix3_moved_ask_old_card_has_no_bar", async () => {
    const f = fakeKernel([...story, elsewhere(10)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    act(() => f.play(story.length));
    await waitFor(() => expect(card("e9")).not.toBeNull());
    // a8 now runs "rm -rf ~" on e9: the Edit card of e2 must no longer offer to allow a8
    expect(within(card("e2")!).queryByRole("button", { name: "Allow once" })).toBeNull();
  });
});
