// S23-1: answered asks, two open asks, Needs you, the pill's focus, a pointer press that does not leave the card (TECHNICAL 18.3, 18.5, 18.6, 9.4 rule 4).
import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { decided, secondAsk } from "../fake/asks";
import { story } from "../fake/story";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";

const more = (i: number, t: string) => JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: t }, _meta: { watane: { i, at: 9000 + i } } });
const checking = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "running", since: Date.now(), activity: { kind: "checking" } } } } });
type Opts = Parameters<typeof fakeKernel>[1];
/** The story (step 0-3) then: step 4 a second ask, 5 the decision of a8, 6 the decision of a9, 7 checking, 8-11 more text. */
const script = () => [...story, secondAsk, decided(10, "a8", "e2"), decided(11, "a9", "e3"), [checking], [more(20, "one")], [more(21, "two")], [more(22, "three")], [more(23, "four")]];
const S = story.length;
const open = async (opts?: Opts, asks = 1) => {
  const f = fakeKernel(script(), opts);
  for (let n = 0; n < S + (asks - 1); n++) f.play(n);
  const r = render(<App kernel={f} info={INFO} />);
  await screen.findByRole("button", { name: `Needs you (${asks})` });
  await screen.findByText("also add a test");
  return { f, ...r, scroller: r.container.querySelector<HTMLElement>(".wa-scroll")! };
};
const card = (call: string) => document.querySelector<HTMLElement>(`[data-call="${call}"]`)!;
const needs = (n: number) => screen.queryByRole("button", { name: `Needs you (${n})` });

const answers = (f: ReturnType<typeof fakeKernel>) => f.calls.filter((c) => c.name === "answer").map((c) => c.args[2]);
const rig = () => {
  const h = { h: 900, top: 0 };
  const d = (p: string, o: PropertyDescriptor) => Object.defineProperty(HTMLElement.prototype, p, { configurable: true, ...o });
  d("scrollHeight", { get: () => h.h }); d("clientHeight", { get: () => 300 });
  d("scrollTop", { get: () => h.top, set: (v: number) => { h.top = Math.min(v, h.h - 300); } });
  Element.prototype.scrollIntoView = () => {};
  return h;
};
const unrig = () => { for (const p of ["scrollHeight", "clientHeight", "scrollTop"]) delete (HTMLElement.prototype as any)[p]; };
const press = (target: Element) => { fireEvent.mouseDown(target); (document.activeElement as HTMLElement).blur(); fireEvent.mouseUp(target); }; // a press on text drops focus to the body

describe("S23-1 answered, waiting", () => {
  it("ac1_ac2_ac3_ac5_an_answer_gives_the_bar_to_one_line_and_the_ask_is_not_counted", async () => {
    const { f } = await open(undefined, 2);
    f.answer = ((...a: unknown[]) => { f.calls.push({ name: "answer", args: a }); return new Promise(() => {}); }) as typeof f.answer; // never resolves: the line shows before `answer` does
    const u = userEvent.setup();
    expect(needs(2)).not.toBeNull();
    act(() => within(card("e2")).getByRole("button", { name: "Allow once" }).focus());
    await u.keyboard("{Enter}");
    const c = card("e2");
    expect(within(c).getByRole("status")).toHaveTextContent("Answered, waiting");
    expect(within(c).queryByRole("button", { name: /Allow|Deny/ })).toBeNull();
    expect(c.textContent).toContain("file_path"); // the card keeps its Input, its diff and its header
    expect(c.querySelector("[data-diff]")).not.toBeNull();
    expect(c.querySelector(".wa-tool-arg")!.textContent).toBe("src/sync.ts");
    expect(document.activeElement).toBe(within(c).getByRole("status")); // AC5: focus stays in the card, on the line
    await u.keyboard("{Enter}");
    expect(answers(f)).toEqual(["a8"]); // a second Enter allows nothing else
    // AC3: not counted, not asking, not the status line's subject, still mounted
    expect(needs(2)).toBeNull();
    expect(needs(1)).not.toBeNull();
    expect(c).not.toHaveAttribute("data-asking");
    expect(card("e3")).toHaveAttribute("data-asking");
    expect(screen.getByText("Waiting for you · Edit src/e3.ts")).toBeInTheDocument();
    // AC2: in checked mode the line says so
    act(() => f.play(S + 3));
    await waitFor(() => expect(within(card("e2")).getByRole("status")).toHaveTextContent("Checking with your judge"));
    // the deny form's submit answers the same way
    await u.click(within(card("e3")).getByRole("button", { name: "Deny…" }));
    await u.type(screen.getByRole("textbox", { name: "Tell the agent why" }), "no");
    await u.click(screen.getByRole("button", { name: "Deny" }));
    expect(within(card("e3")).getByRole("status")).toHaveTextContent("Checking with your judge");
    expect(screen.queryByRole("textbox", { name: "Tell the agent why" })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Needs you \(/ })).toBeNull();
    // the decision entry arrives: the card shows the decision, the line goes
    act(() => f.play(S + 1));
    await waitFor(() => expect(within(card("e2")).queryByText("Checking with your judge")).toBeNull());
    expect(card("e2").textContent).toContain("Allowed");
  });

  it("ac4_a_refused_answer_gives_the_bar_back_with_one_line", async () => {
    const { f } = await open({ refuseAnswer: "busy" });
    await userEvent.setup().click(screen.getByRole("button", { name: "Allow once" }));
    await waitFor(() => expect(within(card("e2")).getByRole("button", { name: "Allow once" })).toBeInTheDocument());
    expect(within(card("e2")).getByRole("alert")).toHaveTextContent("The answer was not taken");
    expect(card("e2")).toHaveAttribute("data-asking");
    // S23-fix8 AC5 with AC8: focus is on the card's refused line (never a button), so Needs you hides (the one open ask's card holds focus)
    expect(within(card("e2")).getByRole("alert")).toHaveFocus();
    await waitFor(() => expect(screen.queryByRole("button", { name: /^Needs you/ })).toBeNull());
    act(() => (document.activeElement as HTMLElement).blur()); // focus leaves the card: Needs you is back and still counts the ask
    await screen.findByRole("button", { name: "Needs you (1)" });
    // a kernel that throws before it returns a promise: the card and Needs you agree all the same
    f.answer = (() => { throw new Error("busy"); }) as typeof f.answer;
    await userEvent.setup().click(within(card("e2")).getByRole("button", { name: "Allow once" }));
    await waitFor(() => expect(within(card("e2")).getByRole("button", { name: "Allow once" })).toBeInTheDocument());
    expect(within(card("e2")).getByRole("alert")).toHaveFocus();
    act(() => (document.activeElement as HTMLElement).blur());
    await screen.findByRole("button", { name: "Needs you (1)" });
  });

  it("ac4_a_refusal_on_the_ask_field_keeps_answered_waiting", async () => {
    const { f } = await open({ refuseAnswer: "ask" });
    await userEvent.setup().click(screen.getByRole("button", { name: "Allow once" }));
    await waitFor(() => expect(answers(f)).toEqual(["a8"]));
    await act(async () => { await Promise.resolve(); });
    expect(within(card("e2")).getByRole("status")).toHaveTextContent("Answered, waiting");
    expect(within(card("e2")).queryByRole("alert")).toBeNull();
    expect(needs(1)).toBeNull();
  });
});

describe("S23-1 two open asks and Needs you", () => {
  it("ac7_ac8_needs_you_goes_to_the_next_card_without_focus_and_after_an_answer_to_the_second", async () => {
    const h = rig();
    try {
      const { f } = await open(undefined, 2);
      const u = userEvent.setup();
      const t0 = h.top;
      const once = (c: string) => within(card(c)).getByRole("button", { name: "Allow once" });
      await u.click(screen.getByRole("button", { name: "Needs you (2)" }));
      expect(document.activeElement).toBe(once("e2")); // the first
      await u.click(await screen.findByRole("button", { name: "Needs you (2)" })); // with more open asks it shows the count, and goes to the next
      expect(document.activeElement).toBe(once("e3"));
      await u.click(await screen.findByRole("button", { name: "Needs you (2)" }));
      expect(document.activeElement).toBe(once("e2"));
      await u.keyboard("{Enter}");
      expect(answers(f)).toEqual(["a8"]);
      await u.click(await screen.findByRole("button", { name: "Needs you (1)" }));
      expect(document.activeElement).toBe(once("e3"));
      expect(h.top).toBe(t0); // the follow stayed off throughout
    } finally { unrig(); }
  });

  it("ac7_ac8_the_only_ask_hides_the_button_and_a_second_ask_arriving_moves_neither_focus_nor_scroll", async () => {
    const { f } = await open();
    await userEvent.setup().click(screen.getByRole("button", { name: "Needs you (1)" }));
    const once = within(card("e2")).getByRole("button", { name: "Allow once" });
    expect(document.activeElement).toBe(once);
    await waitFor(() => expect(needs(1)).toBeNull()); // its card is in view and holds focus
    const top = card("e2").getBoundingClientRect().top;
    act(() => f.play(S)); // a9 opens
    await screen.findByRole("button", { name: "Needs you (2)" });
    expect(document.activeElement).toBe(once);
    expect(Math.abs(card("e2").getBoundingClientRect().top - top)).toBeLessThanOrEqual(1);
    act(() => screen.getByRole("textbox", { name: "Message" }).focus());
    expect(needs(2)).not.toBeNull();
  });
});

describe("S23-1 RISK (a) and the pill", () => {
  const start = async () => {
    const h = rig();
    const o = await open();
    await userEvent.setup().click(screen.getByRole("button", { name: "Needs you (1)" }));
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Allow once" }));
    return { ...o, h };
  };
  const stream = async (f: ReturnType<typeof fakeKernel>, h: { h: number }, step: number, word: string) => { h.h += 300; await act(async () => { f.play(S + step); }); await screen.findByText(word); };

  it("ac6_a_press_on_the_scrollbar_does_not_resume_the_follow_until_focus_goes_to_another_element", async () => {
    const { f, scroller, h } = await start();
    try {
      press(scroller);
      h.top = 600; fireEvent.scroll(scroller); // the reader drags the scrollbar to the end
      await stream(f, h, 4, "one");
      expect(h.top).toBe(600); // the stream arrived and nothing followed it
      expect(screen.getByRole("button", { name: "New activity below" })).toBeInTheDocument();
      act(() => screen.getByRole("textbox", { name: "Message" }).focus());
      h.top = 900; fireEvent.scroll(scroller);
      await stream(f, h, 5, "two");
      expect(h.top).toBe(h.h - 300);
    } finally { unrig(); }
  });

  it("ac6_the_hold_ends_once_focus_went_elsewhere_even_if_it_drops_to_the_body_again", async () => {
    const { f, scroller, h } = await start();
    try {
      press(scroller);
      act(() => screen.getByRole("textbox", { name: "Message" }).focus()); // she moved away from the card
      act(() => (document.activeElement as HTMLElement).blur()); // then pressed on plain text: focus is on the body again
      h.top = 600; fireEvent.scroll(scroller);
      await stream(f, h, 4, "one");
      expect(h.top).toBe(h.h - 300); // the follow resumed
    } finally { unrig(); }
  });

  it("ac6_a_press_on_text_in_the_card_holds_until_the_ask_is_decided", async () => {
    const { f, scroller, h } = await start();
    try {
      press(within(card("e2")).getByText("Allow this edit?"));
      h.top = 600; fireEvent.scroll(scroller);
      act(() => f.play(S + 1)); // the decision of a8
      await waitFor(() => expect(card("e2")).not.toHaveAttribute("data-asking"));
      h.top = h.h - 300 - 20; fireEvent.scroll(scroller);
      await stream(f, h, 4, "one");
      expect(h.top).toBe(h.h - 300);
    } finally { unrig(); }
  });

  it("ac9_a_click_on_the_pill_puts_focus_on_the_newest_row_at_the_end", async () => {
    const { f, scroller, h } = await start();
    try {
      await stream(f, h, 4, "one");
      await userEvent.setup().click(await screen.findByRole("button", { name: "New activity below" }));
      await waitFor(() => {
        const rows = [...document.querySelectorAll<HTMLElement>(".wa-transcript > .wa-row")];
        expect(document.activeElement).toBe(rows[rows.length - 1]);
      });
      expect(document.activeElement).toHaveAttribute("tabindex", "-1");
      expect(h.top).toBe(h.h - 300);
      expect(scroller.className).not.toContain("wa-room");
    } finally { unrig(); }
  });
});
