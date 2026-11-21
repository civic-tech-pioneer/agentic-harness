// S7-2 AC1-AC3 (TECHNICAL 18.3, 18.6; P14): the windowed transcript on a scripted view. jsdom has no layout, so the scroller's
// size and the rows' heights are set here; what real layout does with them is test/long_session.slow.mjs.
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyView, type Actions, type SessionView } from "./model";
import { reduce } from "./reduce";
import { MEASURE, ROW_ESTIMATE, ROW_GAP, ThreadProvider, Transcript } from "./thread";

const SLOT = ROW_ESTIMATE + ROW_GAP, VIEW = 600;
const j = JSON.stringify;
const msg = (i: number, role: string, t: string, extra: object = {}) =>
  j({ v: 1, i, kind: "message", at: 1000 * i, body: { role, content: [{ type: "text", text: t }], ...extra } });
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const feed = (...m: string[]): SessionView => m.reduce(reduce, emptyView);
const users = (n: number, from = 1) => Array.from({ length: n }, (_, k) => msg(from + k, "user", `row ${from + k - 1}`));

let top = 0;
let heightOf: (key: string) => number = () => 0;
/** A scroller of VIEW px with the transcript in it; `top` is its scrollTop. */
function mount(view: SessionView, actions = mk()) {
  const r = render(<div className="wa-scroll"><div className="wa-column"><ThreadProvider view={view} actions={actions}><Transcript view={view} actions={actions} /></ThreadProvider></div></div>);
  const scroller = r.container.querySelector<HTMLElement>(".wa-scroll")!;
  const total = () => Number(scroller.querySelector<HTMLElement>(".wa-transcript")!.dataset.total);
  Object.defineProperty(scroller, "clientHeight", { configurable: true, get: () => VIEW });
  Object.defineProperty(scroller, "scrollHeight", { configurable: true, get: () => total() });
  Object.defineProperty(scroller, "scrollTop", { configurable: true, get: () => top, set: (v: number) => { top = Math.max(0, Math.min(v, total() - VIEW)); } });
  const scrollTo = (y: number) => { act(() => { top = Math.max(0, Math.min(y, total() - VIEW)); fireEvent.scroll(scroller); }); };
  const indexes = () => [...scroller.querySelectorAll<HTMLElement>(".wa-row")].map((e) => Number(e.dataset.index));
  return { ...r, scroller, total, scrollTo, indexes };
}
afterEach(() => { top = 0; heightOf = () => 0; });
Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get() { return (this as HTMLElement).classList.contains("wa-row") ? heightOf((this as HTMLElement).dataset.key!) : 0; } });

describe("the window", () => {
  it("ac1_only_rows_near_the_viewport_are_in_the_dom_and_the_scroll_height_stands_for_all", () => {
    const m = mount(feed(...users(2000)));
    expect(m.total()).toBe(2000 * SLOT - ROW_GAP); // every row stands for its estimate
    expect(m.indexes().at(-1)).toBe(1999); // open: the end
    for (const y of [0, 5000, 80_000, 1e9, 33_333, 0]) {
      m.scrollTo(y);
      expect(m.indexes().length).toBeLessThanOrEqual(120);
      const at = Math.floor(Math.min(y, m.total() - VIEW) / SLOT);
      expect(m.indexes(), `at ${y}`).toContain(at);
      expect(m.indexes(), `at ${y}`).toContain(Math.min(1999, at + 5));
    }
    expect(m.indexes()).toContain(0); // the first row can be reached
    expect(screen.getByText("row 0")).toBeInTheDocument();
  });

  it("ac1_a_short_session_shows_every_row", () => {
    const m = mount(feed(...users(12)));
    expect(m.indexes()).toEqual(Array.from({ length: 12 }, (_, k) => k));
  });

  it("ac2_a_row_that_holds_focus_stays_mounted_until_it_does_not", async () => {
    const odd = j({ x: 1 });
    const m = mount(feed(odd, ...users(1000, 1)));
    m.scrollTo(0);
    const raw = screen.getByRole("button", { name: "Show raw" });
    act(() => raw.focus());
    m.scrollTo(1e9);
    expect(m.indexes()).toContain(0);
    expect(document.activeElement).toBe(raw);
    expect(m.indexes().length).toBeLessThanOrEqual(120);
    act(() => raw.blur());
    m.scrollTo(1e9 - 1);
    expect(m.indexes()).not.toContain(0);
  });

  it("ac2_a_row_keeps_focus_while_the_window_has_lost_it", () => {
    const m = mount(feed(j({ x: 1 }), ...users(1000, 1)));
    m.scrollTo(0);
    const raw = screen.getByRole("button", { name: "Show raw" });
    act(() => raw.focus());
    act(() => { fireEvent.blur(raw); }); // the window lost focus: a blur event, but the element still holds the document's focus
    m.scrollTo(1e9);
    expect(m.indexes()).toContain(0);
    expect(document.activeElement).toBe(raw);
  });

  it("ac2_folded_state_survives_the_row_leaving_the_window_and_coming_back", async () => {
    const m = mount(feed(
      j({ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "deep thoughts" }, _meta: { watane: { at: 1000 } } }),
      msg(1, "agent", "ok", { thought: { text: "deep thoughts", seconds: 2 } }),
      ...users(1000, 2)));
    m.scrollTo(0);
    const open = screen.getByRole("button", { name: /Thought for 2 s/ });
    await userEvent.setup().click(open);
    expect(screen.getByText("deep thoughts")).toBeInTheDocument();
    act(() => (document.activeElement as HTMLElement).blur()); // the clicked button held focus, which keeps its row
    m.scrollTo(1e9);
    expect(screen.queryByText("deep thoughts")).toBeNull();
    expect(m.indexes()).not.toContain(1);
    m.scrollTo(0);
    expect(screen.getByRole("button", { name: /Thought for 2 s/ })).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("deep thoughts")).toBeInTheDocument();
  });
});

describe("place", () => {
  it("ac3_heights_measured_above_the_viewport_do_not_move_what_the_reader_looks_at", () => {
    const m = mount(feed(...users(400)));
    m.scrollTo(200 * SLOT); // the middle: not near the end
    const s0 = top, a = Math.floor(s0 / SLOT), from0 = Math.min(...m.indexes());
    expect(from0).toBeLessThan(a); // there are rows above the reader in the window
    heightOf = () => 120; // the rows are taller than the estimate: those above the reader push it down by 44 px each
    act(() => { m.scroller.dispatchEvent(new Event(MEASURE)); });
    const moved = top - s0;
    expect(Math.abs(moved - (a - from0) * 44)).toBeLessThan(1); // the scroller followed: the row at the top is where it was
    expect(Math.abs(moved)).toBeGreaterThan(SLOT); // (and it was more than a row's worth, so the test shows something)
  });

  it("ac3_following_the_end_the_window_stays_at_the_end_as_rows_arrive", () => {
    const rows = users(300);
    const m = mount(feed(...rows));
    expect(m.indexes().at(-1)).toBe(299);
    const grown = feed(...rows, ...users(5, 301));
    m.rerender(<div className="wa-scroll"><div className="wa-column"><ThreadProvider view={grown} actions={mk()}><Transcript view={grown} actions={mk()} /></ThreadProvider></div></div>);
    expect(Math.max(...m.indexes())).toBe(304);
  });

  it("ac3_away_from_the_end_new_rows_leave_the_window_where_it_is", () => {
    const rows = users(300);
    const actions = mk();
    const m = mount(feed(...rows), actions);
    m.scrollTo(100 * SLOT);
    const before = m.indexes();
    const grown = feed(...rows, ...users(5, 301));
    m.rerender(<div className="wa-scroll"><div className="wa-column"><ThreadProvider view={grown} actions={actions}><Transcript view={grown} actions={actions} /></ThreadProvider></div></div>);
    expect(m.indexes()).toEqual(before);
  });
});
