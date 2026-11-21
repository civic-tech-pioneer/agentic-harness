// Adversary S23 pass 8: a late refusal pulls focus from wherever the participant has gone onto Allow once.
import { describe, expect, it, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const ask = (i: number, id: string) =>
  entry(i, "ask", { id, kind: "ask", toolCallId: "t" + id, input: { command: "rm -rf build" }, "expires-at": 9, toolCall: { toolCallId: "t" + id, title: "mcp__x__run", rawInput: { command: "rm -rf build" } } });

describe("adv S23 fix8 steal", () => {
  it("adv_S23_late_refusal_steals_focus_from_the_composer", async () => {
    const v = [ask(1, "a1")].reduce(reduce, emptyView);
    const calls: unknown[] = [];
    let refuse: () => void = () => {};
    const a: Actions = { say: vi.fn(), stop: vi.fn(), takeBack: vi.fn(),
      answer: vi.fn((_id: string, d: unknown) => {
        calls.push(d);
        if (calls.length > 1) return Promise.resolve();
        return new Promise<void>((_, rej) => { refuse = () => rej(Object.assign(new Error("busy"), { kind: "busy" })); });
      }) } as Actions;
    render(<>
      <ThreadProvider view={v} actions={a}><Transcript view={v} actions={a} /></ThreadProvider>
      <input aria-label="Message" />
    </>);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.click(screen.getByRole("button", { name: /^Deny$/ }));
    expect(calls[0]).toEqual({ outcome: "deny" });
    // the participant moves on to write a message; the kernel's busy refusal lands while they type
    await u.click(screen.getByRole("textbox", { name: "Message" }));
    await u.keyboard("keep going");
    await act(async () => { refuse(); await Promise.resolve(); });
    await u.keyboard("{Enter}");
    expect(calls[1]).toBeUndefined(); // the message's Enter never answers the ask
    expect(screen.getByRole("textbox", { name: "Message" })).toHaveFocus();
  });
});
