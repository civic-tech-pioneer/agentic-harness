// Validator S23-fix1: an answered ask replaced under the same id with no decision between is a new ask (9.4 rule 4).
import { describe, expect, it } from "vitest";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { expiresAt, story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const reopen = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e2", input: { file_path: "src/secrets.ts" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e2", title: "Edit src/secrets.ts", kind: "edit", status: "pending", locations: [{ path: "src/secrets.ts" }],
      content: [{ type: "diff", path: "src/secrets.ts", oldText: "a", newText: "b" }] } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`)!;

describe("validator S23-fix1", () => {
  it("an_answered_ask_replaced_under_its_id_without_a_decision_shows_its_bar_again", async () => {
    const f = fakeKernel([...story, reopen(10)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return new Promise(() => {}); }) as typeof f.answer; // the answer stays in flight
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await userEvent.setup().click(within(card("e2")).getByRole("button", { name: "Allow once" }));
    expect(within(card("e2")).getByText("Answered, waiting")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Needs you \(/ })).toBeNull();
    act(() => f.play(story.length)); // another ask under a8, no decision entry between
    await waitFor(() => expect(card("e2").querySelector(".wa-tool-arg")!.textContent).toBe("src/secrets.ts"));
    expect(within(card("e2")).queryByText("Answered, waiting")).toBeNull();
    expect(within(card("e2")).getByRole("button", { name: "Allow once" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Needs you (1)" })).toBeInTheDocument();
  });
});
