// S36-6: Alice manages what she queued: Edit, Remove with Undo, Stop and send (TECHNICAL 18.6 "Queue").
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(() => { cleanup(); vi.useRealTimers(); });
const queue = (...xs: string[]) => [JSON.stringify({ _meta: { watane: { type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) } } })];

async function open(opts?: { refuseUnqueue?: boolean }) {
  const f = fakeKernel([...story, queue("first one", "second one")], opts);
  for (let n = 0; n < story.length + 1; n++) f.play(n);
  render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: "Needs you (1)" });
  const calls = (name: string) => f.calls.filter((c) => c.name === name);
  return { f, calls };
}
const items = () => [...document.querySelectorAll(".wa-queue li .wa-queue-text")].map((e) => e.textContent);
const lines = () => [...document.querySelectorAll(".wa-queue-note")];
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 20)); });

describe("S36-6", () => {
  it("ac1_edit_puts_the_text_in_the_composer_and_unqueues_once_its_send_is_accepted", async () => {
    const { f, calls } = await open();
    expect(items()).toEqual(["first one", "second one"]);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(box().value).toBe("first one");
    expect(calls("unqueue")).toEqual([]); // the queue keeps it until the edited send is accepted (S36 r2)
    expect(items()).toEqual(["first one", "second one"]);
    await u.type(box(), ", edited{Enter}");
    await settle();
    expect(f.calls.filter((c) => c.name === "say" || c.name === "unqueue").map((c) => [c.name, c.args.at(-1)])).toEqual([["say", "first one, edited"], ["unqueue", 7]]);
    expect(items()).toEqual(["second one"]);
  });
  it("ac2_edit_with_typed_text_changes_nothing", async () => {
    const { calls } = await open();
    const u = userEvent.setup();
    await u.type(box(), "typing");
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await settle();
    expect(calls("unqueue")).toHaveLength(0);
    expect(lines()).toHaveLength(1);
    expect(lines()[0]!.getAttribute("role")).toBe("status");
    expect(lines()[0]!.textContent).toContain("Finish or clear your message first");
    expect(items()).toEqual(["first one", "second one"]);
    expect(box().value).toBe("typing");
  });
  it("ac3_remove_shows_undo_for_6_s_and_undo_says_the_input_again", async () => {
    const { calls } = await open();
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Remove queued message 2" }));
    await settle();
    expect(calls("unqueue").map((c) => c.args)).toEqual([["s1", 8]]);
    expect(lines()[0]!.textContent).toContain("Removed.");
    await u.click(screen.getByRole("button", { name: "Undo" }));
    expect(calls("say").map((c) => c.args[2])).toEqual(["second one"]);
    expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
  });
  it("ac3_undo_is_gone_after_6_s", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { calls } = await open();
    await userEvent.setup({ advanceTimers: vi.advanceTimersByTime }).click(screen.getByRole("button", { name: "Remove queued message 1" }));
    await settle();
    expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument();
    act(() => { vi.advanceTimersByTime(5900); });
    expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument();
    act(() => { vi.advanceTimersByTime(200); });
    expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
    expect(calls("say")).toHaveLength(0);
  });
  it("ac4_stop_and_send_interrupts_with_the_input_then_unqueues_it", async () => {
    const { calls, f } = await open();
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop and send queued message 2" }));
    await settle();
    expect(f.calls.filter((c) => c.name === "unqueue" || c.name === "interrupt").map((c) => [c.name, ...c.args])).toEqual([["interrupt", "s1", "second one"], ["unqueue", "s1", 8]]);
    expect(calls("interrupt")).toHaveLength(1);
    expect(document.body.textContent).toContain("Stopping…");
  });
  it("ac5_unknown_input_says_so", async () => {
    const { f, calls } = await open({ refuseUnqueue: true });
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    expect(lines()[0]!.textContent).toContain("That message was already sent");
    await u.click(screen.getByRole("button", { name: "Edit queued message 1" }));
    await u.type(box(), "{Enter}");
    await settle();
    expect(lines()[0]!.textContent).toContain("That message was already sent");
    expect(f.calls.filter((c) => !["attach", "read"].includes(c.name)).map((c) => c.name)).toEqual(["interrupt", "unqueue", "say", "unqueue"]);
    expect(calls("unqueue")).toHaveLength(2);
  });
  it("ac6_each_control_is_a_named_button_reachable_by_keyboard", async () => {
    await open();
    for (const n of [1, 2]) for (const w of ["Edit", "Remove", "Stop and send"]) {
      const b = screen.getByRole("button", { name: `${w} queued message ${n}` });
      expect(b.tabIndex).toBeGreaterThanOrEqual(0);
    }
    const u = userEvent.setup();
    screen.getByRole("button", { name: "Edit queued message 1" }).focus();
    await u.tab();
    expect(screen.getByRole("button", { name: "Remove queued message 1" })).toHaveFocus();
    await u.keyboard("{Enter}");
    await settle();
    expect(items()).toEqual(["second one"]);
  });
});
