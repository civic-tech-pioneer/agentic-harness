// S23-fix4 AC3: App sends an answer only for the open ask object the card drew.
import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import type { Actions, Ask } from "../kit";

let seen: Actions | undefined;
vi.mock("../kit", async (orig) => {
  const k = await orig<typeof import("../kit")>();
  return { ...k, Transcript: (p: { view: unknown; actions: Actions }) => { seen = p.actions; return null; } };
});
const { App } = await import("./App");

describe("S23-fix4 stale answer", () => {
  it("ac3_answer_with_a_stale_ask_object_sends_nothing", async () => {
    const f = fakeKernel([...story]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const stale = { id: "a8", kind: "ask", toolCallId: "e2", input: {}, "expires-at": 1 } as unknown as Ask;
    await act(async () => { await Promise.resolve(seen!.answerDrawn!(stale, { outcome: "allow" })).catch(() => {}); });
    expect(f.calls.some((c) => c.name === "answer")).toBe(false);
    await waitFor(() => expect(screen.getByRole("button", { name: "Needs you (1)" })).toBeTruthy());
  });
  it("ac3_answer_for_an_id_with_no_open_ask_sends_nothing", async () => {
    const f = fakeKernel([...story]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await act(async () => { await Promise.resolve(seen!.answer("zz", { outcome: "allow" })).catch(() => {}); });
    expect(f.calls.some((c) => c.name === "answer")).toBe(false);
  });
});
