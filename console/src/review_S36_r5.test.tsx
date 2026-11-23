// S36 review round 5 (Codex), with the deputy's ruling (TECHNICAL 18.6 "Queue"): a taken item leaves its list only when the
// kernel accepts its `say` or `interrupt`, or when she discards it (F5.1, F5.3); memory is the list's truth while the page lives
// (F5.2); the capacity epoch is bumped where updates arrive and the Send lock is bound to it (F5.4).
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";
import { forgetTaken, take, takenOf } from "./taken";

afterEach(() => { cleanup(); vi.useRealTimers(); vi.restoreAllMocks(); try { sessionStorage.clear(); } catch { /* none */ } });
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queueOf = (xs: string[]) => w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) });
const running = (since: number) => w({ type: "turn", state: { state: "running", since } });
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const takenText = () => document.querySelector(".wa-taken")?.textContent ?? "";
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });
const set = (f: object, name: string, fn: unknown) => { (f as Record<string, unknown>)[name] = fn; };

async function open(script: string[][]) {
  const f = fakeKernel([...story, ...script]);
  for (let n = 0; n < story.length + 1; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  return f;
}

describe("S36 r5", () => {
  it("regress_s36_two_edits_resolving_together_preserve_both (F5.1)", async () => {
    const f = await open([[queueOf(["first one", "second one", "third one"])]]);
    const done: Array<() => void> = [];
    set(f, "unqueue", (...args: unknown[]) => { f.calls.push({ name: "unqueue", args }); return new Promise<void>((r) => { done.push(r); }); });
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await u.click(screen.getByRole("button", { name: "Edit queued message 2" }));
    await act(async () => { done.forEach((r) => r()); });
    await settle();
    const available = box().value + takenText();
    expect(available).toContain("first one");
    expect(available).toContain("second one");
  });
  it("regress_s36_refused_edit_send_after_a_newer_draft_keeps_the_taken_text (F5.1)", async () => {
    const f = await open([[queueOf(["first one", "second one"])]]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(box().value).toBe("first one");
    let reject!: (e: unknown) => void;
    set(f, "say", (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise((_, no) => { reject = no; }); });
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await u.type(box(), "a newer draft");
    await act(async () => { reject(refusal("quota")); });
    await settle();
    expect(box().value).toBe("a newer draft");
    expect(takenText()).toContain("first one");
  });
  it("an edited item leaves the list once its send is accepted (F5.1)", async () => {
    const f = await open([[queueOf(["first one", "second one"])]]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(takenText()).toContain("first one"); // copied to the composer, still listed
    await u.type(box(), ", edited{Enter}");
    await settle();
    expect(f.calls.filter((c) => c.name === "say").map((c) => c.args[2])).toEqual(["first one, edited"]);
    expect(document.querySelector(".wa-taken")).toBeNull();
  });
  it("regress_s36_taken_items_survive_storage_quota (F5.2)", () => {
    sessionStorage.setItem("wa-taken:quota", JSON.stringify([{ id: "k1", text: "already kept", tried: "edit" }]));
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("full", "QuotaExceededError"); });
    take("quota", "newly taken", "edit");
    expect(takenOf("quota").map((x) => x.text)).toEqual(["already kept", "newly taken"]);
  });
  it("regress_s36_undo_deadline_never_drops_the_item (F5.3)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await open([[queueOf(["first one", "second one"])]]);
    await userEvent.setup({ advanceTimers: vi.advanceTimersByTime }).click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    act(() => { vi.advanceTimersByTime(6001); });
    expect(takenOf("s1").map((x) => x.text)).toEqual(["first one"]);
    expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
    expect(takenText()).toContain("first one");
  });
  it("regress_s36_undo_deadline_holds_after_a_reload (F5.3)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await open([[queueOf(["first one", "second one"])]]);
    await userEvent.setup({ advanceTimers: vi.advanceTimersByTime }).click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    expect(screen.getAllByRole("button", { name: "Undo" })).toHaveLength(1);
    cleanup(); forgetTaken(); // the reload: the page's memory goes, `sessionStorage` stays
    act(() => { vi.advanceTimersByTime(6400); });
    await open([[queueOf(["second one"])]]);
    await settle();
    expect(screen.queryAllByRole("button", { name: "Undo" })).toHaveLength(0);
    expect(takenText()).toContain("first one");
  });
  it("regress_s36_refusal_after_receipt_before_the_frame_is_stale (F5.4)", async () => {
    const f = await open([[queueOf(["first one", "second one"])], [queueOf(["first one", "third one"])]]);
    let reject!: (e: unknown) => void;
    set(f, "say", (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise((_, no) => { reject = no; }); });
    const u = userEvent.setup();
    await u.type(box(), "a long one");
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await act(async () => { f.play(story.length + 1); reject(refusal("queue-full")); }); // received, not yet shown
    await settle();
    expect(screen.getByRole("button", { name: "Queue message" })).toBeEnabled();
  });
  it("regress_s36_lock_clears_when_its_epoch_changes_at_an_equal_count (F5.4)", async () => {
    const f = await open([[queueOf(["first one", "second one"])], [running(42)]]);
    set(f, "say", async (...args: unknown[]) => { f.calls.push({ name: "say", args }); throw refusal("queue-full"); });
    const u = userEvent.setup();
    await u.type(box(), "a long one");
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await settle();
    expect(screen.getByRole("button", { name: "Queue message" })).toBeDisabled(); // the lock, in its epoch
    await act(async () => { f.play(story.length + 1); }); // a replacement turn, the same count
    await settle();
    expect(screen.getByRole("button", { name: "Queue message" })).toBeEnabled();
  });
});
