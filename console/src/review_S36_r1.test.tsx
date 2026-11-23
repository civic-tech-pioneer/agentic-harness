// S36 review round 1 (Codex): the queue's actions stay in their session and never lose a message
// (F1.4, F1.5), Stopping ends with the turn it stopped (F1.6), a queue-full refusal closes Send (F1.7).
// TECHNICAL 18.5, 18.6 "Queue", "Send".
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
const T = Date.now();
const running = (since: number) => w({ type: "turn", state: { state: "running", since } });
const idle = w({ type: "turn", state: { state: "idle", stop: { kind: "interrupted" } } });
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });

async function open(extra: string[][] = []) {
  const f = fakeKernel([...story, queue("first one", "second one"), ...extra]);
  for (let n = 0; n < story.length + 1; n++) f.play(n);
  const r = render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  return { f, ...r };
}
const refuse = (f: FakeKernel, name: "say" | "interrupt", kind: string) => {
  (f as unknown as Record<string, unknown>)[name] = async (...args: unknown[]) => { f.calls.push({ name, args }); throw refusal(kind); };
};
const items = () => [...document.querySelectorAll(".wa-queue li .wa-queue-text")].map((e) => e.textContent);
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });
const recoverable = (t: string) => items().includes(t) || box().value === t || !!screen.queryByRole("button", { name: "Undo" });

describe("S36 r1", () => {
  it("regress_s36_undo_stays_in_its_session (F1.4)", async () => {
    const { f, rerender } = await open();
    await userEvent.setup().click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    const undo = await screen.findByRole("button", { name: "Undo" });
    rerender(<App kernel={f} info={{ ...INFO, id: "other-session" }} />);
    if (document.body.contains(undo)) await userEvent.setup().click(undo);
    await settle();
    expect(f.calls.filter((c) => c.name === "say" && c.args[0] === "other-session")).toEqual([]);
  });
  it("regress_s36_pending_edit_stays_in_its_session (F1.4)", async () => {
    const { f, rerender } = await open();
    let release!: () => void;
    const held = new Promise<void>((r) => { release = r; });
    const unqueue = f.unqueue.bind(f);
    f.unqueue = async (s, n) => { await held; return unqueue(s, n); };
    await userEvent.setup().click(screen.getByRole("button", { name: "Edit queued message 1" }));
    rerender(<App kernel={f} info={{ ...INFO, id: "other-session" }} />);
    await act(async () => { release(); await new Promise((r) => setTimeout(r, 20)); });
    expect(box().value).toBe("");
    expect(f.calls.filter((c) => c.name === "say" && c.args[0] === "other-session")).toEqual([]);
  });
  it("regress_s36_stop_and_send_can_recover_on_refusal (F1.5)", async () => {
    const { f } = await open();
    refuse(f, "interrupt", "quota");
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    expect(recoverable("first one")).toBe(true);
  });
  it("regress_s36_undo_refusal_keeps_recovery (F1.5)", async () => {
    const { f } = await open();
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    refuse(f, "say", "queue-full");
    await u.click(screen.getByRole("button", { name: "Undo" }));
    await settle();
    expect(recoverable("first one")).toBe(true);
  });
  it("regress_s36_replacement_does_not_show_stopping (F1.6)", async () => {
    const { f } = await open([[running(T + 5000)], [idle, running(T + 9000), JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Replacing." } })]]);
    act(() => f.play(story.length + 1));
    await settle();
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    expect(document.querySelector(".wa-status-word")).toHaveTextContent("Stopping…");
    act(() => f.play(story.length + 2));
    await settle();
    expect(document.querySelector(".wa-status-word")).not.toHaveTextContent("Stopping");
    expect(document.querySelector(".wa-shape")!.getAttribute("data-shape")).toBe("dot");
  });
  it("regress_s36_queue_full_refusal_disables_send (F1.7)", async () => {
    const { f } = await open([[queue("first one")[0]!]]);
    refuse(f, "say", "queue-full");
    const u = userEvent.setup();
    await u.type(box(), "a long one");
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await settle();
    expect(box().value).toBe("a long one");
    expect(screen.getByRole("button", { name: "Queue message" })).toBeDisabled();
    expect(box().placeholder).toMatch(/queue is full/i);
    // capacity returns: the queue shrinks
    act(() => f.play(story.length + 1));
    await settle();
    expect(screen.getByRole("button", { name: "Queue message" })).toBeEnabled();
  });
});
