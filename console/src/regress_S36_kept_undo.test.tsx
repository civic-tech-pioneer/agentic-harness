// S36 user-sim (Alice on the real host): a refused Undo with a draft in the composer keeps the message as
// "The queue is full. ... remove a queued message." with Undo again. Alice does what it says and removes another
// queued message: that Remove's own "Removed. Undo" replaces the kept one, and the first message is gone for good.
// TECHNICAL 18.6 "Queue": a message a queue action took out is never lost to a refusal.
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel, type FakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(() => { cleanup(); vi.useRealTimers(); });
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queue = (...xs: string[]) => [w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) })];
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });

describe("S36 user-sim", () => {
  it("regress_S36_kept_undo_survives_the_next_remove", async () => {
    const f: FakeKernel = fakeKernel([...story, queue("first one", "second one")]);
    for (let n = 0; n < story.length + 1; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    // the queue filled up meanwhile: Undo is refused queue-full while she has a draft
    const say = f.say.bind(f);
    f.say = async (...args) => { f.calls.push({ name: "say", args }); throw refusal("queue-full"); };
    await u.type(box(), "my draft");
    await u.click(screen.getByRole("button", { name: "Undo" }));
    await settle();
    expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument(); // kept, with the error
    // she follows the error's advice: removes a queued message, then uses every Undo the page offers
    f.say = say;
    const mark = f.calls.length;
    await u.click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    for (let k = 0; k < 3; k++) {
      const b = screen.queryByRole("button", { name: "Undo" });
      if (!b) break;
      await u.click(b);
      await settle();
    }
    const sent = f.calls.slice(mark).filter((c) => c.name === "say").map((c) => c.args[2]);
    expect(sent).toContain("second one");
    expect(box().value === "first one" || sent.includes("first one"), `after the second Remove and its Undo she sent ${JSON.stringify(sent)}, the composer holds ${JSON.stringify(box().value)}: "first one" is lost`).toBe(true);
  });
});
