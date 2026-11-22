// S32-fix5: opening Deny… scrolls the transcript just enough that the whole reason form is in view, clear of the dock and the pill.
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AskBar } from "../kit/parts/askbar";

const VIEW = 300; // the scroller's visible part: 0..300 in page px
const FORM_AT = 500, FORM_H = 60; // the form's top and height, in scroller content px
let top = 0;
let pillTop: number | null = null;
const rect = (t: number, h: number) => ({ top: t, bottom: t + h, left: 0, right: 100, width: 100, height: h, x: 0, y: t, toJSON() {} }) as DOMRect;

function mount(formAt: number, pill: number | null = null, start = 0) {
  top = start; pillTop = pill;
  const proto = HTMLElement.prototype;
  vi.spyOn(proto, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    if (this.classList.contains("wa-scroll")) return rect(0, VIEW);
    if (this.classList.contains("wa-pill")) return rect(pillTop ?? 0, 30);
    if (this.tagName === "FORM") return rect(formAt - top, FORM_H);
    return rect(0, 0);
  });
  Object.defineProperty(proto, "scrollTop", { configurable: true, get: () => top, set: (v: number) => { top = Math.max(0, Math.min(v, 1000)); } });
  const set = vi.fn();
  render(
    <div className="wa-scroll" onScroll={set}>
      <AskBar call={{ kind: "execute" } as never} askId="a1" actions={{ say() {}, answer() {}, stop() {}, takeBack() {} } as never} />
      {pill !== null ? <button className="wa-pill">Back to the end</button> : null}
    </div>,
  );
}
afterEach(() => { vi.restoreAllMocks(); delete (HTMLElement.prototype as any).scrollTop; });

const formBox = () => { const f = document.querySelector("form")!.getBoundingClientRect(); return f; };

describe("S32 fix5", () => {
  it("ac1_deny_scrolls_the_whole_form_above_the_pill_and_focuses_the_field", async () => {
    mount(FORM_AT, 250);
    await userEvent.setup().click(screen.getByRole("button", { name: "Deny…" }));
    expect(screen.getByRole("textbox")).toHaveFocus();
    const f = formBox();
    expect(f.top).toBeGreaterThanOrEqual(0);
    expect(f.bottom).toBeLessThanOrEqual(250);
  });
  it("ac1_deny_scrolls_the_form_into_the_visible_part_without_a_pill", async () => {
    mount(FORM_AT);
    await userEvent.setup().click(screen.getByRole("button", { name: "Deny…" }));
    const f = formBox();
    expect(f.top).toBeGreaterThanOrEqual(0);
    expect(f.bottom).toBeLessThanOrEqual(VIEW);
  });
  it("ac3_scrolls_the_least_that_shows_the_form", async () => {
    mount(FORM_AT);
    await userEvent.setup().click(screen.getByRole("button", { name: "Deny…" }));
    expect(top).toBe(FORM_AT + FORM_H - VIEW); // the form's bottom meets the visible part's bottom
  });
  it("ac2_a_form_already_in_view_scrolls_nothing", async () => {
    mount(100, null, 40);
    await userEvent.setup().click(screen.getByRole("button", { name: "Deny…" }));
    expect(top).toBe(40);
  });
});
