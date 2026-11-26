// S36 review round 3 (Codex), with the deputy's ruling that replaces the r2 rule (TECHNICAL 18.6 "Queue"):
// take it back first. Edit, Stop and send and Remove call `unqueue(n)` first; `unknown-input` leaves everything
// alone; a later refused `say` or `interrupt` puts the text in the composer as Send does (F3.1, F3.2). A late
// queue-full refusal applies only while the capacity it describes holds (F3.3). Undo is one `say` in flight, and a
// refused Undo keeps its line and button (F3.4).
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel, type FakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(() => { cleanup(); vi.useRealTimers(); try { sessionStorage.clear(); } catch { /* none */ } });
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queueOf = (xs: string[]) => w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) });
const idle = w({ type: "turn", state: { state: "idle", stop: { kind: "completed" } } });
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });

async function open(xs: string[], more: string[][] = [], opts?: { refuseUnqueue?: boolean }) {
  const f = fakeKernel([...story, [queueOf(xs)], ...more], opts);
  for (let n = 0; n < story.length + 1; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  return f;
}
const names = (f: FakeKernel) => f.calls.filter((c) => !["attach", "read"].includes(c.name)).map((c) => c.name);
const items = () => [...document.querySelectorAll(".wa-queue li .wa-queue-text")].map((e) => e.textContent);
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });

describe("S36 r3", () => {
  it("regress_S36_edit_takes_the_item_back_before_it_is_sent_again (F3.1)", async () => {
    const f = await open(["first one", "second one"]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(names(f)).toEqual(["unqueue"]); // taken back first: a turn can no longer take the original
    expect(items()).toEqual(["second one"]);
    expect(box().value).toBe("first one");
    await u.type(box(), ", edited{Enter}");
    await settle();
    expect(f.calls.filter((c) => c.name === "say" || c.name === "unqueue").map((c) => [c.name, c.args.at(-1)]))
      .toEqual([["unqueue", 7], ["say", "first one, edited"]]);
  });
  it("regress_S36_edit_of_a_taken_item_leaves_everything_alone (F3.1)", async () => {
    const f = await open(["first one"], [], { refuseUnqueue: true });
    await userEvent.setup().click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(names(f)).toEqual(["unqueue"]);
    expect(box().value).toBe("");
    expect(document.querySelector(".wa-queue-note")!.textContent).toContain("That message was already sent");
  });
  it("regress_S36_stop_and_send_of_a_taken_item_leaves_the_turn_alone (F3.1)", async () => {
    const f = await open(["first one"], [], { refuseUnqueue: true });
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    expect(names(f)).toEqual(["unqueue"]); // no interrupt
    expect(document.querySelector(".wa-queue-note")!.textContent).toContain("That message was already sent");
  });
  it("regress_S36_stop_and_send_unqueues_then_interrupts_and_a_refusal_goes_to_the_composer (F3.1)", async () => {
    const f = await open(["first one", "second one"]);
    (f as unknown as Record<string, unknown>).interrupt = async (...args: unknown[]) => { f.calls.push({ name: "interrupt", args }); throw refusal("quota"); };
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop and send queued message 2" }));
    await settle();
    expect(f.calls.filter((c) => c.name === "unqueue" || c.name === "interrupt").map((c) => [c.name, ...c.args]))
      .toEqual([["unqueue", "s1", 8], ["interrupt", "s1", "second one"]]);
    expect(box().value).toBe("second one");
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
  it("regress_s36_edit_can_replace_an_item_at_the_count_limit (F3.2)", async () => {
    await open(Array.from({ length: 32 }, (_, k) => `item ${k}`));
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    await u.type(box(), "!");
    expect(screen.getByRole("button", { name: "Queue message" })).toBeEnabled();
  });
  it("regress_s36_late_queue_full_cannot_relock_an_idle_composer (F3.3)", async () => {
    const f = await open(["first one", "second one"], [[queueOf([]), idle]]);
    let reject!: (e: unknown) => void;
    (f as unknown as Record<string, unknown>).say = (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise((_, no) => { reject = no; }); };
    const u = userEvent.setup();
    await u.type(box(), "a long one");
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await act(async () => { f.play(story.length + 1); }); // the queue drains and the turn ends before the reply
    await settle();
    await act(async () => { reject(refusal("queue-full")); });
    await settle();
    expect(box()).toHaveValue("a long one");
    expect(screen.getByRole("button", { name: "Send message" })).toBeEnabled();
  });
  it("regress_s36_undo_only_sends_once_while_pending (F3.4)", async () => {
    const f = await open(["first one", "second one"]);
    const u = userEvent.setup();
    await u.click(screen.getAllByRole("button", { name: "Remove from queue" })[0]!);
    await settle();
    let release!: () => void;
    (f as unknown as Record<string, unknown>).say = (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise<number>((r) => { release = () => r(0); }); };
    const undo = screen.getByRole("button", { name: "Undo" });
    await u.click(undo);
    if (document.body.contains(undo)) await u.click(undo);
    await act(async () => { release(); });
    expect(f.calls.filter((c) => c.name === "say")).toHaveLength(1);
  });
  it("regress_s36_refused_undo_remains_available (F3.4)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const f = await open(["first one", "second one"]);
    const u = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await u.click(screen.getAllByRole("button", { name: "Remove from queue" })[0]!);
    await settle();
    (f as unknown as Record<string, unknown>).say = async (...args: unknown[]) => { f.calls.push({ name: "say", args }); throw refusal("queue-full"); };
    await u.click(screen.getByRole("button", { name: "Undo" }));
    await settle();
    act(() => { vi.advanceTimersByTime(7000); });
    expect(screen.queryByRole("button", { name: "Undo" })).toBeInTheDocument();
  });
});
