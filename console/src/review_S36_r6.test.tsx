// S36 review round 6 (Codex), with the deputy's ruling (TECHNICAL 18.6 "Queue"): a taken item has one send path at a time and
// one in-flight guard, keyed by its id; the composer that holds its text owns it, also across a reload (F6.1). Each ordinary
// Undo has its own deadline timer, and a click checks the deadline again (F6.2).
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";
import { Dock } from "./Dock";
import { emptyView, type Actions } from "../kit";
import { forgetTaken } from "./taken";

afterEach(() => { cleanup(); forgetTaken(); vi.useRealTimers(); vi.restoreAllMocks(); try { sessionStorage.clear(); } catch { /* none */ } });
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queueOf = (xs: string[]) => w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) });
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const takenText = () => document.querySelector(".wa-taken")?.textContent ?? "";
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });
const set = (f: object, name: string, fn: unknown) => { (f as Record<string, unknown>)[name] = fn; };
const says = (f: ReturnType<typeof fakeKernel>) => f.calls.filter((c) => c.name === "say").map((c) => c.args[2]);

async function open(script: string[][]) {
  const f = fakeKernel([...story, ...script]);
  for (let n = 0; n < story.length + 1; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  return f;
}

describe("S36 r6", () => {
  it("regress_s36_edit_and_undo_cannot_both_send_one_item (F6.1)", async () => {
    const f = await open([[queueOf(["first one", "second one"])]]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(box().value).toBe("first one");
    const held: Array<() => void> = [];
    const submit = vi.fn((...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise<number>((r) => { held.push(() => r(0)); }); });
    set(f, "say", submit);
    await u.click(screen.getByRole("button", { name: "Queue message" })); // the composer's send, held
    for (const b of screen.queryAllByRole("button", { name: "Undo" })) await u.click(b); // the row's
    await act(async () => { held.forEach((r) => r()); });
    await settle();
    expect(submit).toHaveBeenCalledTimes(1);
    expect(document.querySelector(".wa-taken")).toBeNull();
  });
  it("the row shows no Undo while the composer holds its text, and gets it back when the composer is cleared (F6.1)", async () => {
    await open([[queueOf(["first one", "second one"])]]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(takenText()).toContain("first one");
    expect(screen.queryAllByRole("button", { name: "Undo" })).toHaveLength(0);
    await u.clear(box());
    await settle();
    expect(screen.getAllByRole("button", { name: "Undo" })).toHaveLength(1);
  });
  it("regress_s36_restored_edit_draft_sends_its_item_after_a_reload (F6.1)", async () => {
    await open([[queueOf(["first one", "second one"])]]);
    await userEvent.setup().click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    cleanup(); forgetTaken(); // the reload: the page's memory goes, `sessionStorage` stays
    const f = await open([[queueOf(["second one"])]]);
    await settle();
    expect(box().value).toBe("first one");
    expect(screen.queryAllByRole("button", { name: "Undo" })).toHaveLength(0); // still the composer's
    await userEvent.setup().click(screen.getByRole("button", { name: "Queue message" }));
    await settle();
    expect(says(f)).toEqual(["first one"]);
    expect(document.querySelector(".wa-taken")).toBeNull();
  });
  it("a refused send of a held item with a newer draft gives the item back to its row (F6.1)", async () => {
    const f = await open([[queueOf(["first one", "second one"])]]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    let reject!: (e: unknown) => void;
    set(f, "say", (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise((_, no) => { reject = no; }); });
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await u.type(box(), "a newer draft");
    await act(async () => { reject(refusal("quota")); });
    await settle();
    expect(box().value).toBe("a newer draft");
    expect(takenText()).toContain("first one");
    expect(screen.getAllByRole("button", { name: "Undo" })).toHaveLength(1);
  });
  it("regress_s36_refused_undo_does_not_mask_a_later_deadline (F6.2)", async () => {
    // the Dock alone: App's one-second clock renders it again and would hide a missing deadline timer
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const queue = [{ n: 7, input: "first one" }, { n: 8, input: "second one" }];
    const actions: Actions = { say: vi.fn(async () => { throw refusal("quota"); }), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn(), unqueue: vi.fn(async () => {}) };
    const dock = (q: typeof queue) => <Dock sessionId="d1" view={{ ...emptyView, queue: q }} actions={actions} onNeedsYou={() => {}} status={null} />;
    const r = render(dock(queue));
    const u = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await u.click(screen.getAllByRole("button", { name: "Remove from queue" })[0]!);
    await settle();
    await u.click(screen.getByRole("button", { name: "Undo" })); // refused: its retry stays, with no deadline
    await settle();
    act(() => { vi.advanceTimersByTime(6500); });
    r.rerender(dock(queue.slice(1)));
    await u.click(screen.getAllByRole("button", { name: "Remove from queue" })[0]!); // "second one"
    await settle();
    expect(screen.getAllByRole("button", { name: "Undo" })).toHaveLength(2);
    act(() => { vi.advanceTimersByTime(6100); });
    expect(screen.getAllByRole("button", { name: "Undo" })).toHaveLength(1);
    expect(takenText()).toContain("first one");
    expect(takenText()).toContain("second one");
  });
  it("regress_s36_undo_click_past_its_deadline_sends_nothing (F6.2)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const f = await open([[queueOf(["first one", "second one"])]]);
    const u = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await u.click(screen.getAllByRole("button", { name: "Remove from queue" })[0]!);
    await settle();
    const undo = screen.getByRole("button", { name: "Undo" });
    vi.setSystemTime(Date.now() + 6100); // the clock passes the deadline before any timer runs
    undo.click();
    await settle();
    expect(says(f)).toEqual([]);
    expect(takenText()).toContain("first one");
  });
});
