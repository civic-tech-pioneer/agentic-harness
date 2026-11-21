// Adversary S23 pass 8: a refusal moves focus to Allow once while the deny row the participant was using stays open.
import { describe, expect, it, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type Ask } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const ask = (i: number, id: string) =>
  entry(i, "ask", { id, kind: "ask", toolCallId: "t" + id, input: { command: "rm -rf build" }, "expires-at": 9, toolCall: { toolCallId: "t" + id, title: "mcp__x__run", rawInput: { command: "rm -rf build" } } });

describe("adv S23 fix8", () => {
  it("adv_S23_retrying_a_refused_deny_with_enter_denies", async () => {
    const v = [ask(1, "a1")].reduce(reduce, emptyView);
    const answered = new Map<string, Ask>();
    let rerender: () => void = () => {};
    const calls: unknown[] = [];
    let fail = true;
    const a: Actions = { say: vi.fn(), stop: vi.fn(), takeBack: vi.fn(), answeredAsks: answered,
      answer: vi.fn((id: string, d: unknown) => {
        calls.push(d);
        if (!fail) return Promise.resolve();
        // the page marks the ask answered (the bar gives way to the line), then the kernel refuses: busy
        answered.set(id, v.rows.find((r) => r.kind === "tool")!.ask!);
        rerender();
        return new Promise<void>((_, rej) => setTimeout(() => {
          answered.delete(id); rerender();
          rej(Object.assign(new Error("busy"), { kind: "busy" }));
        }, 0));
      }) } as Actions;
    const tree = (x: Actions) => <ThreadProvider view={v} actions={x}><Transcript view={v} actions={x} /></ThreadProvider>;
    const r = render(tree(a));
    rerender = () => r.rerender(tree({ ...a }));
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.keyboard("not this one{Enter}"); // Deny with a reason
    expect(calls[0]).toEqual({ outcome: "deny", reason: "not this one" });
    await act(async () => { await new Promise((res) => setTimeout(res, 5)); });
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    // the deny row is still open with the reason: the participant presses Enter to send the deny again
    expect(screen.getByRole("textbox")).toHaveValue("not this one");
    fail = false;
    await u.keyboard("{Enter}");
    expect(calls[1]).not.toEqual({ outcome: "allow" });
  });
});
