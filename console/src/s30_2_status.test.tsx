// S30-2: the status line agrees with the cards once this console answered (TECHNICAL 18.5, 18.6, 9.4 rule 4).
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { decided, secondAsk } from "../fake/asks";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

afterEach(cleanup);

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const edit = (id: string) => ({ toolCallId: id, title: `Edit src/${id}.ts`, kind: "edit", status: "pending", locations: [{ path: `src/${id}.ts` }], content: [{ type: "diff", path: `src/${id}.ts`, oldText: "a", newText: "b" }] });
const run = (i: number) => w({ type: "turn", state: { state: "running", since: Date.now() } });
/** The second ask, opening with its own `expires-at`, 15 minutes out (the story's a8 and the turn say 10). */
const farAsk = (at: number): string[] => [
  w({ type: "entry", entry: { v: 1, i: 9, kind: "ask", at: Date.now(), body: { id: "a9", kind: "ask", toolCallId: "e3", reach: "edits in ~/work/sync", input: { file_path: "src/e3.ts" }, "expires-at": at, toolCall: edit("e3") } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8", "a9"], "expires-at": at - 300_000 } }),
];
const S = story.length;
const open = async (script: string[][], steps: number, asks: number) => {
  const f = fakeKernel(script);
  for (let n = 0; n < steps; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: `Needs you (${asks})` });
  await screen.findByText("also add a test");
  f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return new Promise(() => {}); }) as typeof f.answer;
  return f;
};
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`)!;
const statusLine = () => document.querySelector<HTMLElement>(".wa-status")!;
const word = () => statusLine().querySelector(".wa-status-word")!.textContent;
const allow = (call: string) => userEvent.setup().click(within(card(call)).getByRole("button", { name: "Allow once" }));

describe("S30-2", () => {
  it("ac1_one_ask_answered_reads_Answered_waiting_with_no_clock", async () => {
    const f = await open([...story, [w({ type: "entry", entry: { v: 1, i: 10, kind: "ask", at: Date.now(), body: { id: "a8", kind: "ask", toolCallId: "e2", input: { file_path: "src/e2.ts" }, "expires-at": 1, decision: { outcome: "allow", by: "alice" }, toolCall: edit("e2") } } }), run(0)]], S, 1);
    expect(statusLine().textContent).toContain("if unanswered");
    await allow("e2");
    expect(word()).toBe("Answered, waiting");
    expect(screen.getByRole("img", { name: "Waiting for you" }).getAttribute("data-shape")).toBe("diamond");
    expect(statusLine().textContent).not.toMatch(/denied in|if unanswered/);
    expect(statusLine().querySelector(".wa-status-clock")).toBeNull();
    act(() => f.play(S));
    await waitFor(() => expect(word()).not.toBe("Answered, waiting"));
    expect(statusLine().getAttribute("data-state")).toBe("running");
  });

  it("ac2_checked_mode_the_line_reads_the_cards_judge_words", async () => {
    const checking = w({ type: "turn", state: { state: "running", since: Date.now(), activity: { kind: "checking" } } });
    const f = await open([...story, [checking]], S, 1);
    await allow("e2");
    act(() => f.play(S));
    await waitFor(() => expect(within(card("e2")).getByRole("status")).toHaveTextContent("Checking with your judge"));
    expect(word()).toBe(within(card("e2")).getByRole("status").textContent);
  });

  it("ac3_two_asks_the_first_answered_names_the_next_with_its_own_clock", async () => {
    await open([...story, farAsk(Date.now() + 900_000)], S + 1, 2);
    await allow("e2");
    expect(word()).toBe("Waiting for you · Edit src/e3.ts");
    expect(statusLine().querySelector(".wa-status-clock")!.textContent).toMatch(/denied in 1[45]:\d\d if unanswered/); // a9's own, not the turn's 10:00
    await allow("e3");
    expect(word()).toBe("Answered, waiting");
    expect(statusLine().textContent).not.toMatch(/denied in|if unanswered/);
  });

  it("ac4_a_refused_answer_brings_the_ask_and_its_clock_back", async () => {
    const f = fakeKernel(story, { refuseAnswer: "busy" });
    for (let n = 0; n < S; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByText("also add a test");
    await userEvent.setup().click(within(card("e2")).getByRole("button", { name: "Allow once" }));
    await waitFor(() => expect(within(card("e2")).getByRole("alert")).toBeInTheDocument());
    expect(word()).toBe("Waiting for you · Edit src/sync.ts");
    expect(statusLine().textContent).toContain("if unanswered");
  });

  it("ac6_decided_elsewhere_follows_the_turn", async () => {
    const f = await open([...story, secondAsk, decided(10, "a8", "e2")], S + 1, 2);
    act(() => f.play(S + 1));
    await waitFor(() => expect(card("e2").textContent).toContain("Allowed"));
    expect(word()).not.toBe("Answered, waiting");
    expect(word()).toBe("Waiting for you · Edit src/e3.ts");
  });
});
