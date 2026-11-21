// Adversary S23: the "Answered, waiting" line is console-local state; a hostile stream must not make it stand for an ask the participant never answered.
import { describe, expect, it } from "vitest";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { decided } from "../fake/asks";
import { expiresAt, story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
/** The kernel opens an ask again under the id `a8`, on the same call, now for a different input. */
const reopen = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e2", input: { file_path: "src/secrets.ts" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e2", title: "Edit src/secrets.ts", kind: "edit", status: "pending", locations: [{ path: "src/secrets.ts" }],
      content: [{ type: "diff", path: "src/secrets.ts", oldText: "a", newText: "b" }] } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`)!;

describe("adv S23 answered", () => {
  it("adv_S23_reopened_ask_id_shows_answered_waiting_for_an_ask_never_answered", async () => {
    const f = fakeKernel([...story, decided(10, "a8", "e2"), reopen(11)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await userEvent.setup().click(within(card("e2")).getByRole("button", { name: "Allow once" }));
    act(() => f.play(story.length)); // the decision of the first a8
    await waitFor(() => expect(card("e2")).not.toHaveAttribute("data-asking"));
    act(() => f.play(story.length + 1)); // a new open ask, same id, other input: the participant has not answered it
    await waitFor(() => expect(card("e2").querySelector(".wa-tool-arg")!.textContent).toBe("src/secrets.ts"));
    // the participant never answered this ask: the card asks, with its bar, and Needs you counts it
    expect(within(card("e2")).queryByText("Answered, waiting")).toBeNull();
    expect(within(card("e2")).getByRole("button", { name: "Allow once" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Needs you (1)" })).toBeInTheDocument();
  });

  it("adv_S23_session_switch_carries_answered_waiting_to_the_same_ask_id_of_another_session", async () => {
    const f = fakeKernel(story);
    for (let n = 0; n < story.length; n++) f.play(n);
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return new Promise(() => {}); }) as typeof f.answer; // s1's answer is in flight
    const r = render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await userEvent.setup().click(within(card("e2")).getByRole("button", { name: "Allow once" }));
    expect(within(card("e2")).getByText("Answered, waiting")).toBeInTheDocument();
    // the page switches to another session whose ask has the same id (ids repeat across sessions)
    const g = fakeKernel([...story, [JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "in s2" }, _meta: { watane: { i: 30, at: 9030 } } })]]);
    for (let n = 0; n <= story.length; n++) g.play(n);
    r.rerender(<App kernel={g} info={{ ...INFO, id: "s2" }} />);
    await screen.findByText("in s2"); // s2's own stream is shown
    expect(within(card("e2")).queryByText("Answered, waiting")).toBeNull();
    expect(within(card("e2")).getByRole("button", { name: "Allow once" })).toBeInTheDocument();
  });
});
