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
  const f = fakeKernel([...story, [more(20, "one")], [more(21, "two")], [more(22, "three")]]);
  for (let n = 0; n < story.length; n++) f.play(n);
  const { container } = render(<App kernel={f} info={INFO} />);
  const needs = await screen.findByRole("button", { name: "Needs you (1)" });
  await screen.findByText("also add a test");
  return { f, needs, s: container.querySelector(".wa-scroll")! };
};

describe("follow pause and pill", () => {
  it("ac2_focus_in_an_open_asks_card_stops_the_follow_and_leaving_it_does_not_jump", async () => {
    const b = box();
    try {
      Element.prototype.scrollIntoView = () => {};
      const { f, needs, s } = await open();
      expect(b.top).toBe(600);
      await userEvent.setup().click(needs);
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Allow once" }));
      b.h = 1200;
      await act(async () => { f.play(story.length); });
      await screen.findByText("one");
      expect(b.top).toBe(600);
      (document.activeElement as HTMLElement).blur();
      b.h = 1500;
      await act(async () => { f.play(story.length + 1); });
      await screen.findByText("two");
      expect(b.top).toBe(600);
      b.top = 1190; fireEvent.scroll(s); // back within 40 px of the end: the follow resumes
      b.h = 1800;
      await act(async () => { f.play(story.length + 2); });
      await screen.findByText("three");
      expect(b.top).toBe(1500);
    } finally { unbox(); }
  });

  it("ac3_pill_shows_when_output_arrives_below_a_paused_follow_and_returns_to_the_end", async () => {
    const b = box();
    try {
      Element.prototype.scrollIntoView = () => {};
      const { f, needs } = await open();
      expect(screen.queryByRole("button", { name: "New activity below" })).toBeNull();
      await userEvent.setup().click(needs);
      const allow = document.activeElement;
      b.h = 1200;
      await act(async () => { f.play(story.length); });
      const pill = await screen.findByRole("button", { name: "New activity below" });
      expect(document.activeElement).toBe(allow);
      fireEvent.click(pill);
      expect(b.top).toBe(900);
      expect(screen.queryByRole("button", { name: "New activity below" })).toBeNull();
      b.h = 1500;
      await act(async () => { f.play(story.length + 1); });
      await screen.findByText("two");
      expect(b.top).toBe(1200);
      expect(screen.queryByRole("button", { name: "New activity below" })).toBeNull();
    } finally { unbox(); }
  });
});
