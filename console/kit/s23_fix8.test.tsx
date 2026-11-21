// S23-fix8: the anchor hold never asks past what the content can reach; a refusal puts focus on Allow once; the answered line is the bar's row.
import { describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions } from "./model";
import { reduce } from "./reduce";
import { ThreadProvider, Transcript } from "./thread";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
const ask = (i: number, id: string) =>
  entry(i, "ask", { id, kind: "ask", toolCallId: "t" + id, input: { n: id }, "expires-at": 9, toolCall: { toolCallId: "t" + id, title: "mcp__x__run", rawInput: { n: id } } });
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });

describe("s23 fix8", () => {
  // slack: the browser's real maximum may sit below scrollHeight - clientHeight (rounding): a write that moves nothing must not render again
  it.each([[350, 0], [450, 2]])("ac2_a_shrinking_card_ends_in_a_bounded_number_of_renders (to %i, slack %i)", async (shrunk, slack) => {
    // a scroller whose scrollTop clamps like a browser's: 0 .. scrollHeight - clientHeight - slack
    let sh = 1000, top = 0, sets = 0, asked = 0;
    const proto = HTMLElement.prototype;
    const desc = (n: string) => Object.getOwnPropertyDescriptor(proto, n);
    const saved = { sh: desc("scrollHeight"), ch: desc("clientHeight"), st: desc("scrollTop") };
    Object.defineProperty(proto, "scrollHeight", { configurable: true, get() { return this.classList.contains("wa-scroll") ? sh : 0; } });
    Object.defineProperty(proto, "clientHeight", { configurable: true, get() { return this.classList.contains("wa-scroll") ? 400 : 0; } });
    Object.defineProperty(proto, "scrollTop", { configurable: true,
      get() { return this.classList.contains("wa-scroll") ? Math.min(top, Math.max(0, sh - 400 - slack)) : 0; },
      set(v: number) { if (this.classList.contains("wa-scroll")) { sets++; asked = Math.max(asked, v); top = Math.min(Math.max(0, v), Math.max(0, sh - 400 - slack)); } } });
    try {
      let renders = 0;
      const Count = () => { renders++; return null; };
      const v = [ask(1, "a1"), ask(2, "a2")].reduce(reduce, emptyView);
      const a = mk();
      const ui = <div className="wa-scroll"><ThreadProvider view={v} actions={a}><Transcript view={v} actions={a} /><Count /></ThreadProvider></div>;
      const r = render(ui);
      await act(async () => { top = 84; r.container.querySelector(".wa-scroll")!.dispatchEvent(new Event("scroll")); });
      sh = shrunk; // the content shrinks below the anchor's place
      renders = 0; sets = 0; asked = 0;
      await act(async () => { r.rerender(<div className="wa-scroll"><ThreadProvider view={v} actions={{ ...a }}><Transcript view={v} actions={{ ...a }} /><Count /></ThreadProvider></div>); });
      expect(renders).toBeLessThan(20);
      expect(sets).toBeLessThan(10);
      expect(asked).toBeLessThanOrEqual(Math.max(0, sh - 400)); // never asks past what the content can reach
    } finally {
      for (const [n, d] of [["scrollHeight", saved.sh], ["clientHeight", saved.ch], ["scrollTop", saved.st]] as const) {
        if (d) Object.defineProperty(proto, n, d); else delete (proto as unknown as Record<string, unknown>)[n];
      }
    }
  });

  it("ac5_a_refusal_puts_focus_on_the_refused_line", async () => {
    const a = mk();
    a.answer = vi.fn(() => { (document.activeElement as HTMLElement).blur(); return Promise.reject(new Error("busy")); }); // focus is lost, as when the bar swaps to the line
    const v = [ask(1, "a1")].reduce(reduce, emptyView);
    render(<ThreadProvider view={v} actions={a}><Transcript view={v} actions={a} /></ThreadProvider>);
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.click(screen.getByRole("button", { name: "Allow once" }));
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveFocus();
  });

  it("ac4_the_answered_line_has_the_bars_padding", () => {
    const css = readFileSync("kit/styles/kit.css", "utf8");
    const pad = (sel: string) => new RegExp("^\\" + sel + " \\{[^}]*padding: ([^;]+);", "m").exec(css)?.[1];
    expect(pad(".wa-answered")).toBe(pad(".wa-askbar"));
    expect(pad(".wa-answered")).toBeDefined();
  });
});
