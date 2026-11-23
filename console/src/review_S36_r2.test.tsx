// S36 review round 2 (Codex): a queue action's message stays recoverable in its own session (F2.1), and a late
// queue-full refusal closes Queue even when she has typed since (F2.2). S36 r3 replaced the r2 rule with take it back
// first; these still hold under it. TECHNICAL 18.6 "Queue", "Send".
import { StrictMode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel, type FakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { storyInfo as INFO } from "../fake/page";
import { App } from "./App";

afterEach(() => { cleanup(); vi.useRealTimers(); try { sessionStorage.clear(); } catch { /* none */ } });
const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const queue = (...xs: string[]) => [w({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 7, input })) })];
const refusal = (kind: string) => Object.assign(new Error(kind), { kind });

async function open(strict = false) {
  const f = fakeKernel([...story, queue("first one", "second one")]);
  for (let n = 0; n < story.length + 1; n++) f.play(n);
  const app = <App kernel={f} info={INFO} />;
  const r = render(strict ? <StrictMode>{app}</StrictMode> : app);
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

describe("S36 r2", () => {
  it("regress_S36_r2_edit_refusal_keeps_input_and_concurrent_draft (F2.1)", async () => {
    const { f } = await open();
    let release!: () => void;
    const held = new Promise<void>((r) => { release = r; });
    const unqueue = f.unqueue.bind(f);
    f.unqueue = async (s, n) => { await held; return unqueue(s, n); };
    refuse(f, "say", "queue-full");
    await userEvent.setup().click(screen.getByRole("button", { name: "Edit queued message 1" }));
    fireEvent.change(box(), { target: { value: "new draft" } }); // she types while the Edit is under way
    await act(async () => { release(); await new Promise((r) => setTimeout(r, 20)); });
    expect(box().value).toBe("new draft");
    expect(recoverable("first one")).toBe(true);
  });
  it("regress_S36_r2_kept_recovery_survives_session_switch (F2.1)", async () => {
    const { f, rerender } = await open();
    const u = userEvent.setup();
    await u.type(box(), "my draft");
    refuse(f, "interrupt", "quota");
    await u.click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    rerender(<App kernel={f} info={{ ...INFO, id: "other-session" }} />);
    await settle();
    rerender(<App kernel={f} info={INFO} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await settle();
    expect(recoverable("first one")).toBe(true);
  });
  it("regress_S36_r2_strict_mode_keeps_refused_replacement (F2.1)", async () => {
    const { f } = await open(true);
    const u = userEvent.setup();
    await u.type(box(), "my draft");
    refuse(f, "interrupt", "quota");
    refuse(f, "say", "queue-full");
    await u.click(screen.getByRole("button", { name: "Stop and send queued message 1" }));
    await settle();
    expect(box().value).toBe("my draft");
    expect(recoverable("first one")).toBe(true);
  });
  it("regress_S36_r2_late_queue_full_disables_send_with_new_draft (F2.2)", async () => {
    const { f } = await open();
    let reject!: (e: unknown) => void;
    (f as unknown as Record<string, unknown>).say = (...args: unknown[]) => { f.calls.push({ name: "say", args }); return new Promise((_, no) => { reject = no; }); };
    const u = userEvent.setup();
    await u.type(box(), "a long one");
    await u.click(screen.getByRole("button", { name: "Queue message" }));
    await u.type(box(), "new draft");
    await act(async () => { reject(refusal("queue-full")); });
    await settle();
    expect(box()).toHaveValue("new draft");
    expect(screen.getByRole("button", { name: "Queue message" })).toBeDisabled();
  });
});
