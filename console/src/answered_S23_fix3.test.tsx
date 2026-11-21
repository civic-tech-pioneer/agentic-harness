// S23-fix3 AC3: a late refusal of an earlier answer cannot clear a newer answer to a replaced ask.
import { describe, expect, it } from "vitest";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { expiresAt, story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`);
const reopen = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e9", input: { command: "ls" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e9", title: "ls", kind: "execute", status: "pending" } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];
/** a8 opened again on its own call e2, for another input, with no decision between. */
const replace = (i: number): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e2", input: { file_path: "src/other.ts" }, "expires-at": expiresAt,
    toolCall: { toolCallId: "e2", title: "Edit src/other.ts", kind: "edit", status: "pending", locations: [{ path: "src/other.ts" }] } } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
];

describe("answered S23 fix3", () => {
  it("ac3_late_refusal_of_the_first_answer_keeps_the_second_answered", async () => {
    const f = fakeKernel([...story, reopen(10)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    const rejects: ((e: unknown) => void)[] = [];
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return new Promise<void>((_, r) => rejects.push(r)); }) as typeof f.answer;
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const user = userEvent.setup();
    await user.click(within(card("e2")!).getByRole("button", { name: "Allow once" }));
    act(() => f.play(story.length));
    await waitFor(() => expect(card("e9")).not.toBeNull());
    await user.click(within(card("e9")!).getByRole("button", { name: "Allow once" }));
    expect(within(card("e9")!).getByText("Answered, waiting")).toBeInTheDocument();
    await act(async () => { rejects[0]!({ code: "busy" }); await Promise.resolve(); });
    expect(within(card("e9")!).getByText("Answered, waiting")).toBeInTheDocument();
  });

  it("ac3_refused_line_belongs_to_the_ask_it_was_set_for", async () => {
    const f = fakeKernel([...story, replace(10)]);
    for (let n = 0; n < story.length; n++) f.play(n);
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return Promise.reject({ code: "busy" }); }) as typeof f.answer;
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await userEvent.setup().click(within(card("e2")!).getByRole("button", { name: "Allow once" }));
    await waitFor(() => expect(within(card("e2")!).getByText("The answer was not taken. Try again.")).toBeInTheDocument());
    act(() => f.play(story.length)); // a8 replaced on the same card: the refusal was of the old ask
    await waitFor(() => expect(card("e2")!.querySelector(".wa-tool-arg")!.textContent).toBe("src/other.ts"));
    expect(within(card("e2")!).queryByText("The answer was not taken. Try again.")).toBeNull();
  });
});
