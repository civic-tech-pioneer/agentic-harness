// Adversary S23 pass 2: the replaced-ask check compares the first row holding the id; an ask reopened under the same id on another call slips past it.
import { describe, expect, it } from "vitest";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { expiresAt, story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
/** The kernel opens an ask again under the id `a8`, now on another call (e9) with another input, no decision entry between. */
const elsewhere = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e9", input: { command: "rm -rf ~" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e9", title: "rm -rf ~", kind: "execute", status: "pending" } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`);

describe("adv S23 fix answered", () => {
  it("adv_S23_fix_ask_reopened_under_its_id_on_another_call_shows_answered_without_a_bar", async () => {
    const f = fakeKernel([...story, elsewhere(10)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return new Promise(() => {}); }) as typeof f.answer; // the answer stays in flight
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await userEvent.setup().click(within(card("e2")!).getByRole("button", { name: "Allow once" }));
    expect(within(card("e2")!).getByText("Answered, waiting")).toBeInTheDocument();
    act(() => f.play(story.length)); // a new open ask a8 on e9: the participant never saw it
    await waitFor(() => expect(card("e9")).not.toBeNull());
    // the new ask is not answered: its card asks, with its bar, and Needs you counts it
    expect(within(card("e9")!).queryByText("Answered, waiting")).toBeNull();
    expect(within(card("e9")!).getByRole("button", { name: "Allow once" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Needs you (1)" })).toBeInTheDocument();
  });
});
