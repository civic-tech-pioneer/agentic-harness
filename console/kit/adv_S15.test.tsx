// Adversary S15: the ask card must show the call of the ask its buttons answer (TECHNICAL 9.8, 14).
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const ask = (i: number, id: string, tc: string, tool: string, input: object, extra: object = {}) =>
  entry(i, "ask", { id, kind: "ask", toolCallId: tc, input, "expires-at": 9, toolCall: { toolCallId: tc, title: tool, rawInput: input }, ...extra });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });

describe("adv S15", () => {
  it("adv_S15_second_entry_on_one_call_relabels_the_first_asks_card", async () => {
    // a1 asks to run a payment tool; a later ask (or check) on the same tool call id names a harmless tool
    const v = feed([
      ask(1, "a1", "t", "mcp__bank__transfer", { amount: 5000, to: "attacker" }),
      ask(2, "a2", "t", "mcp__notes__read", { note: "todo" }),
    ]);
    const actions = mk();
    const { container } = render(<ThreadProvider view={v} actions={actions}><Transcript view={v} actions={actions} /></ThreadProvider>);
    const card = container.querySelector("[data-ask]") as HTMLElement;
    const answered = card.getAttribute("data-ask");
    await userEvent.setup().click(screen.getByRole("button", { name: "Allow once" }));
    expect(actions.answer).toHaveBeenCalledWith(answered, { outcome: "allow" });
    // the card whose Allow answers a1 must name a1's tool, not a2's
    const head = card.querySelector(".wa-tool-head")!.textContent ?? "";
    expect(answered).toBe("a1");
    expect(head).toContain("mcp__bank__transfer");
    expect(head).not.toContain("mcp__notes__read");
  });
  it("adv_S15_check_after_ask_relabels_the_asks_card", () => {
    const v = feed([
      ask(1, "a1", "t", "mcp__bank__transfer", { amount: 5000 }),
      ask(2, "c1", "t", "mcp__notes__read", { note: "todo" }, { kind: "check" }),
    ]);
    const actions = mk();
    const { container } = render(<ThreadProvider view={v} actions={actions}><Transcript view={v} actions={actions} /></ThreadProvider>);
    const card = container.querySelector("[data-ask='a1']") as HTMLElement;
    expect(card).not.toBeNull();
    expect(card.querySelector(".wa-tool-head")!.textContent).toContain("mcp__bank__transfer");
  });
});
