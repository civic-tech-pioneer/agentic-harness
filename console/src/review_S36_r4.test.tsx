// S36 review round 4 (Codex), with the deputy's ruling (TECHNICAL 18.6 "Queue"): taken text is a per-session list outside
// the dock, left only on acceptance or her discard (F4.1); a queue-full refusal applies only in the capacity epoch its
// `say` was sent in (F4.2).
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import type { Kernel } from "../kit";
import { App } from "./App";

afterEach(() => { cleanup(); vi.useRealTimers(); try { sessionStorage.clear(); } catch { /* none */ } });
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queueOf = (xs: string[]) => w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) });
const idle = w({ type: "turn", state: { state: "idle", stop: { kind: "completed" } } });
const running = w({ type: "turn", state: { state: "running", since: 9 } });
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });
const set = (f: object, name: string, fn: unknown) => { (f as Record<string, unknown>)[name] = fn; };

describe("S36 r4", () => {
  it("regress_s36_refused_undo_survives_edit_of_another_row (F4.1)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const f = fakeKernel([...story, [queueOf(["first one", "second one", "third one"])]]);
    for (let n = 0; n < story.length + 1; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const u = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await u.click(screen.getAllByRole("button", { name: "Remove from queue" })[0]!);
    await settle();
    set(f, "say", async (...args: unknown[]) => { f.calls.push({ name: "say", args }); throw refusal("queue-full"); });
    await u.click(screen.getByRole("button", { name: "Undo" }));
    await settle();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" })); // another row frees room
    await settle();
    act(() => { vi.advanceTimersByTime(7000); });
    // r5 (the deputy's rule): the Edit's item stays listed too, until its send is accepted; the refused Undo keeps its row and Undo
    const row = [...document.querySelectorAll(".wa-taken li")].find((li) => li.textContent?.includes("first one"));
    expect(row?.querySelector("button")?.textContent).toBe("Undo");
  });
  it("regress_s36_refused_interrupt_after_a_switch_keeps_the_taken_text (F4.1)", async () => {
    const f = fakeKernel([...story, [queueOf(["first one", "second one"])]]);
    for (let n = 0; n < story.length + 1; n++) f.play(n);
    const k: Kernel = { ...f, attach: (s, a, m, e) => (s === "s2" ? new Promise(() => {}) : f.attach(s, a, m, e)) };
    let reject!: (e: unknown) => void;
    k.interrupt = (...args: unknown[]) => { f.calls.push({ name: "interrupt", args }); return new Promise((_, no) => { reject = no; }); };
    const { rerender } = render(<App kernel={k} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    await u.type(box(), "a newer draft");
    rerender(<App kernel={k} info={{ ...INFO, id: "s2", title: "another session" }} />);
    await settle();
    await act(async () => { reject(refusal("quota")); });
    rerender(<App kernel={k} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    expect(box().value).toBe("a newer draft");
    expect(document.querySelector(".wa-taken")?.textContent ?? "").toContain("first one");
    expect(screen.getAllByRole("button", { name: "Discard" })).toHaveLength(1);
  });
  it("regress_s36_equal_count_does_not_make_an_old_refusal_current (F4.2)", async () => {
    const f = fakeKernel([...story, [queueOf(["first one", "second one"])], [queueOf([]), idle], [running, queueOf(["first one", "second one"])]]);
    for (let n = 0; n < story.length + 1; n++) f.play(n);
    render(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    let reject!: (e: unknown) => void;
    set(f, "say", (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise((_, no) => { reject = no; }); });
    const u = userEvent.setup();
    await u.type(box(), "a long one");
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await act(async () => { f.play(story.length + 1); }); // the queue drains and the turn ends
    await settle();
    await act(async () => { f.play(story.length + 2); }); // another turn, with the same count as before
    await settle();
    await act(async () => { reject(refusal("queue-full")); });
    await settle();
    expect(box()).toHaveValue("a long one");
    expect(screen.getByRole("button", { name: "Queue message" })).toBeEnabled();
  });
});
