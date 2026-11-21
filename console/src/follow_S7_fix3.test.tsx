// S7-fix2: the follow of the end pauses while focus is in an open ask's card; the "New activity below" pill (18.6 Place).
import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
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

// AC4: the ask decided while focus is in its card: the follow does not stay off once focus is not in an open ask's card.
const decided = JSON.stringify({ _meta: { watane: { type: "entry", entry: { v: 1, i: 9, kind: "ask", at: 9009, body: { id: "a8", kind: "ask", toolCallId: "e2", reach: "edits in ~/work/sync", input: { file_path: "src/sync.ts" }, "expires-at": 9_999_999_999_999, decision: { outcome: "allow", by: "you" } } } } } });
const decide = async () => {
  const f = fakeKernel([...story, [decided], [more(20, "one")], [more(21, "two")]]);
  for (let n = 0; n < story.length; n++) f.play(n);
  const { container } = render(<App kernel={f} info={INFO} />);
  await screen.findByText("also add a test");
  const s = container.querySelector(".wa-scroll")!;
  await userEvent.setup().click(await screen.findByRole("button", { name: "Needs you (1)" }));
  const allow = screen.getByRole("button", { name: "Allow once" });
  expect(document.activeElement).toBe(allow);
  await userEvent.setup().click(allow);
  expect(f.calls.some((c) => c.name === "answer")).toBe(true);
  await act(async () => { f.play(story.length); }); // the decision closes the ask while focus was in its card
  await screen.findByText(/Allowed: Edit src\/sync.ts/);
  expect(s.querySelector("[data-asking]")).toBeNull();
  return { f, s };
};

describe("an ask decided with focus in its card", () => {
  it("ac4_after_the_decision_a_scroll_within_40px_of_the_end_resumes_the_follow", async () => {
    const b = box();
    try {
      Element.prototype.scrollIntoView = () => {};
      const { f, s } = await decide();
      const top = b.top;
      b.h = 1200;
      await act(async () => { f.play(story.length + 1); });
      await screen.findByText("one");
      expect(b.top).toBe(top); // still off: no scroll yet
      b.top = 870; fireEvent.scroll(s); // 30 px from the end
      b.h = 1500;
      await act(async () => { f.play(story.length + 2); });
      await screen.findByText("two");
      expect(b.top).toBe(1200);
    } finally { unbox(); }
  });

  it("ac4_after_the_decision_the_pill_resumes_the_follow", async () => {
    const b = box();
    try {
      Element.prototype.scrollIntoView = () => {};
      const { f } = await decide();
      b.h = 1200;
      await act(async () => { f.play(story.length + 1); });
      fireEvent.click(await screen.findByRole("button", { name: "New activity below" }));
      expect(b.top).toBe(900);
      b.h = 1500;
      await act(async () => { f.play(story.length + 2); });
      await screen.findByText("two");
      expect(b.top).toBe(1200);
    } finally { unbox(); }
  });
});

// AC2: focus already in a card when its ask opens (no focus event): the follow is off all the same.
describe("an ask that opens on the card that has focus", () => {
  it("ac2_the_follow_is_off_when_the_ask_opens_where_focus_already_is", async () => {
    const b = box();
    try {
      Element.prototype.scrollIntoView = () => {};
      const [call, ...ask] = story[3]!;
      const f = fakeKernel([...story.slice(0, 3), [call!], ask, [more(20, "one")], [more(21, "two")]]);
      for (let n = 0; n < 3; n++) f.play(n);
      const { container } = render(<App kernel={f} info={INFO} />);
      const s = container.querySelector(".wa-scroll")!;
      await screen.findByText("I will read the sync code first."); // attached: a live call shows now
      await act(async () => { f.play(3); });
      const head = await waitFor(() => { const h = container.querySelector<HTMLElement>('[data-call="e2"] .wa-tool-head'); if (!h) throw new Error("no card"); return h; });
      act(() => head.focus());
      expect(b.top).toBe(600);
      await act(async () => { f.play(4); });
      await screen.findByRole("button", { name: "Allow once" });
      expect(document.activeElement).toBe(head);
      expect(head.closest("[data-asking]")).not.toBeNull();
      const top = b.top;
      b.h = 1200;
      await act(async () => { f.play(5); });
      await screen.findByText("one");
      expect(b.top).toBe(top);
      await screen.findByRole("button", { name: "New activity below" });
      b.h = 1500;
      fireEvent.scroll(s);
      await act(async () => { f.play(6); });
      await screen.findByText("two");
      expect(b.top).toBe(top);
    } finally { unbox(); }
  });
});
