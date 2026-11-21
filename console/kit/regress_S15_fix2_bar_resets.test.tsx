// S15-fix2: when an ask card switches from a decided entry to another open entry, the ask bar starts fresh.
import { describe, expect, it, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const ask = (i: number, id: string, extra: object = {}) =>
  entry(i, "ask", { id, kind: "ask", toolCallId: "t", input: { n: id }, "expires-at": 9, toolCall: { toolCallId: "t", title: "mcp__x__run", rawInput: { n: id } }, ...extra });
const feed = (msgs: string[], v: SessionView = emptyView) => msgs.reduce(reduce, v);
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const decided = (i: number) => ask(i, "a1", { decision: { outcome: "allow", by: "alice" } });

function setup() {
  const actions = mk();
  const v1 = feed([ask(1, "a1"), ask(2, "a2")]);
  const ui = (v: SessionView) => <ThreadProvider view={v} actions={actions}><Transcript view={v} actions={actions} /></ThreadProvider>;
  const r = render(ui(v1));
  return { r, ui, v1 };
}

describe("regress S15 fix2", () => {
  it("ac1_deny_form_and_reason_do_not_carry_to_the_next_entry", async () => {
    const { r, ui, v1 } = setup();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Deny…" }));
    await user.type(screen.getByRole("textbox"), "no way");
    expect(r.container.querySelector("[data-ask]")!.getAttribute("data-ask")).toBe("a1");
    await act(async () => { r.rerender(ui(feed([decided(3)], v1))); });
    expect(r.container.querySelector("[data-ask]")!.getAttribute("data-ask")).toBe("a2");
    expect(screen.queryByRole("textbox")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Deny…" }));
    expect((screen.getByRole("textbox") as HTMLInputElement).value).toBe("");
  });
  it("ac2_button_focus_does_not_carry_to_the_next_entry", async () => {
    const { r, ui, v1 } = setup();
    const once = screen.getByRole("button", { name: "Allow once" });
    act(() => once.focus());
    expect(document.activeElement).toBe(once);
    await act(async () => { r.rerender(ui(feed([decided(3)], v1))); });
    expect(r.container.querySelector("[data-ask]")!.getAttribute("data-ask")).toBe("a2");
    const bar = screen.getByRole("group", { name: "Your answer" });
    expect(bar.contains(document.activeElement)).toBe(false);
  });
});
