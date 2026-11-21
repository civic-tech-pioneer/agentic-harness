// S7-fix2: the follow of the end pauses while focus is in an open ask's card; the "New activity below" pill (18.6 Place).
import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const more = (i: number, t: string) => JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: t }, _meta: { watane: { i, at: 9000 + i } } });
const box = () => {
  const b = { h: 900, top: 0 };
  const d = (p: string, o: PropertyDescriptor) => Object.defineProperty(HTMLElement.prototype, p, { configurable: true, ...o });
  d("scrollHeight", { get: () => b.h }); d("clientHeight", { get: () => 300 });
  d("scrollTop", { get: () => b.top, set: (v: number) => { b.top = Math.min(v, b.h - 300); } });
  return b;
};
const unbox = () => { for (const p of ["scrollHeight", "clientHeight", "scrollTop"]) delete (HTMLElement.prototype as any)[p]; };
const open = async () => {
  const f = fakeKernel([...story, [more(20, "one")], [more(21, "two")], [more(22, "three")], [more(23, "four")]]);
  for (let n = 0; n < story.length; n++) f.play(n);
  const { container } = render(<App kernel={f} info={INFO} />);
  const needs = await screen.findByRole("button", { name: "Needs you (1)" });
  await screen.findByText("also add a test");
  return { f, needs, s: container.querySelector(".wa-scroll")! };
};


// S7-fix3: no scroll resumes the follow while focus is in an open ask's card.
describe("follow stays off while focus is in an open ask", () => {
  it("ac2_scrolls_at_the_end_do_not_resume_until_focus_leaves_the_card", async () => {
    const b = box();
    try {
      Element.prototype.scrollIntoView = () => {};
      const { f, needs, s } = await open();
      await userEvent.setup().click(needs);
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Allow once" }));
      b.h = 1200;
      await act(async () => { f.play(story.length); });
      await screen.findByText("one");
      expect(b.top).toBe(600);
      b.top = 900; fireEvent.scroll(s); // gap 0
      b.h = 1500;
      await act(async () => { f.play(story.length + 1); });
      await screen.findByText("two");
      expect(b.top).toBe(900);
      b.top = 1160; fireEvent.scroll(s); // a later delta, 40 px from the end
      b.h = 1800;
      await act(async () => { f.play(story.length + 2); });
      await screen.findByText("three");
      expect(b.top).toBe(1160);
      const composer = screen.getByRole("textbox");
      act(() => composer.focus());
      b.top = 1470; fireEvent.scroll(s);
      b.h = 2100;
      await act(async () => { f.play(story.length + 3); });
      await screen.findByText("four");
      expect(b.top).toBe(1800);
    } finally { unbox(); }
  });
});
