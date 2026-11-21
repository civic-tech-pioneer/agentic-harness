// TECHNICAL 18.4-18.7: component tests of the shell on the scripted fake kernel.
import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { decisionJson, emptyView, type Actions, type SessionView } from "../kit";
import { App } from "./App";
import { storyInfo as INFO } from "../fake/page";
import { Header } from "./Header";
import { Dock } from "./Dock";

const idle = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "idle", stop: { kind: "interrupted" } } } } });
const played = () => {
  const f = fakeKernel([...story, [idle]]);
  for (let n = 0; n < story.length; n++) f.play(n);
  return f;
};
const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const rowsOf = (c: HTMLElement) => [...c.querySelectorAll(".wa-row")].map((r) => `${r.getAttribute("data-kind")}:${r.textContent}`);
// The played story reaches the DOM in steps: wait for all of its rows, not only the first ask.
const ready = async () => {
  const button = await screen.findByRole("button", { name: "Needs you (1)" });
  await waitFor(() => {
    const kinds = new Set([...document.querySelectorAll(".wa-row")].map((r) => r.getAttribute("data-kind")));
    for (const k of ["user", "thought", "agent", "explored", "tool"]) expect(kinds).toContain(k);
    expect(screen.getByText("also add a test")).toBeInTheDocument();
  });
  return button;
};

describe("the page", () => {
  it("ac1_app_shows_sidebar_header_rows_dock_on_the_played_story", async () => {
    const { container } = render(<App kernel={played()} info={INFO} />);
    await ready();
    const side = screen.getByRole("navigation", { name: "Sessions" });
    expect(within(side).getByText(INFO.title)).toBeInTheDocument();
    const head = screen.getByRole("banner");
    expect(within(head).getByText("Profile", { exact: false })).toHaveTextContent("Profile Careful");
    for (const t of [INFO.title, "Claude Code · no judge", INFO.workspace, "21%"]) expect(within(head).getByText(t)).toBeInTheDocument();
    expect(within(head).getByRole("meter")).toHaveAttribute("aria-valuetext", "42k / 200k tokens");
    const kinds = new Set([...container.querySelectorAll(".wa-row")].map((r) => r.getAttribute("data-kind")));
    for (const k of ["user", "thought", "agent", "explored", "tool"]) expect(kinds).toContain(k);
    expect(screen.getByText("Fix the retry bug in sync.ts", { selector: ".wa-user" })).toBeInTheDocument();
    expect(screen.getByText("Needs you (1)")).toBeInTheDocument();
    expect(screen.getByText("Plan · 1 of 3 done")).toBeInTheDocument();
    expect(screen.getByText("also add a test")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Waiting for you");
    expect(screen.getByRole("textbox", { name: "Message" })).toBeInTheDocument();
  });

  it("ac2_needs_you_scrolls_to_the_first_ask_and_focuses_allow_once", async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    render(<App kernel={played()} info={INFO} />);
    await userEvent.setup().click(await ready());
    expect(scroll).toHaveBeenCalled();
    expect(scroll.mock.contexts[0]).toBe(document.querySelector('[data-call="e2"]'));
    expect(scroll.mock.calls[0][0]).toEqual({ block: "start" });
    const allow = screen.getByRole("button", { name: "Allow once" });
    expect(document.activeElement).toBe(allow);
  });

  it("ac2_needs_you_scrolls_a_card_that_fits_to_start_and_a_taller_one_to_its_ask_bar", async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    const { container } = render(<App kernel={played()} info={INFO} />);
    const button = await ready();
    Object.defineProperty(container.querySelector(".wa-scroll")!, "clientHeight", { configurable: true, value: 600 });
    const card = document.querySelector<HTMLElement>('[data-call="e2"]')!;
    card.getBoundingClientRect = () => ({ height: 200 }) as DOMRect;
    await userEvent.setup().click(button);
    expect(scroll.mock.contexts[0]).toBe(card);
    expect(scroll.mock.calls[0][0]).toEqual({ block: "start" });
    card.getBoundingClientRect = () => ({ height: 900 }) as DOMRect;
    await userEvent.setup().click(button);
    expect(scroll.mock.contexts[1]).toBe(card.querySelector(".wa-askbar"));
    expect(scroll.mock.calls[1][0]).toEqual({ block: "end" });
  });

  it("ac2_needs_you_focuses_allow_once_without_scrolling", async () => {
    Element.prototype.scrollIntoView = vi.fn();
    const focus = vi.spyOn(HTMLElement.prototype, "focus");
    render(<App kernel={played()} info={INFO} />);
    await userEvent.setup().click(await ready());
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    focus.mockRestore();
  });

  it("ac2_esc_in_the_input_stops_and_the_status_says_stopping_until_idle", async () => {
    const fake = played();
    render(<App kernel={fake} info={INFO} />);
    await ready();
    await userEvent.setup().type(screen.getByRole("textbox", { name: "Message" }), "{Escape}");
    expect(fake.calls.some((c) => c.name === "interrupt")).toBe(true);
    expect(screen.getByRole("status")).toHaveTextContent("Stopping…");
    act(() => fake.play(story.length));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Idle"));
  });

  it("answers_reach_the_kernel_as_the_decision_json_of_technical_2_3", async () => {
    const fake = played();
    render(<App kernel={fake} info={INFO} />);
    await ready();
    await userEvent.setup().click(screen.getByRole("button", { name: /^Allow .+ this session$/ }));
    expect(fake.calls.find((c) => c.name === "answer")?.args.slice(2)).toEqual(["e2", '{"allow":true,"for":"session"}']);
    expect(decisionJson({ outcome: "allow" })).toBe('{"allow":true}');
    expect(decisionJson({ outcome: "deny", reason: "not now" })).toBe('{"deny":true,"reason":"not now"}');
  });

  it("ac2_enter_sends_through_the_kernel", async () => {
    const fake = played();
    render(<App kernel={fake} info={INFO} />);
    await ready();
    await userEvent.setup().type(screen.getByRole("textbox", { name: "Message" }), "go on{Enter}");
    expect(fake.calls.find((c) => c.name === "say")?.args[2]).toBe("go on");
  });
});

const viewWith = (p: Partial<SessionView>): SessionView => ({ ...emptyView, ...p });

describe("dock", () => {
  const dock = (view: SessionView, actions = mk()) => render(<Dock view={view} actions={actions} onNeedsYou={() => {}} status={<p>status</p>} />);

  it("ac2_needs_you_shows_only_while_an_ask_is_open", () => {
    const a = dock(viewWith({}));
    expect(screen.queryByText(/Needs you/)).toBeNull();
    a.unmount();
    dock(viewWith({ openAsks: ["a", "b"] }));
    expect(screen.getByRole("button", { name: "Needs you (2)" })).toBeInTheDocument();
  });

  it("ac2_queue_shows_three_then_n_more", () => {
    dock(viewWith({ queue: [1, 2, 3, 4, 5].map((n) => ({ n, input: `q${n}` })) }));
    expect(screen.getByText("q3")).toBeInTheDocument();
    expect(screen.queryByText("q4")).toBeNull();
    expect(screen.getByText("+ 2 more")).toBeInTheDocument();
  });

  it("ac2_full_queue_turns_send_off_and_the_placeholder_says_why", () => {
    dock(viewWith({ queue: Array.from({ length: 32 }, (_, n) => ({ n, input: `q${n}` })) }));
    expect(screen.getByRole("button", { name: "Send message" })).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "Message" })).toHaveAttribute("placeholder", "The queue is full: 32 messages wait. Remove one, or stop the turn.");
  });

  it("ac2_enter_says_shift_enter_is_a_new_line_esc_stops", async () => {
    const actions = mk();
    dock(viewWith({}), actions);
    const box = screen.getByRole("textbox", { name: "Message" });
    const u = userEvent.setup();
    await u.type(box, "one{Shift>}{Enter}{/Shift}two");
    expect(actions.say).not.toHaveBeenCalled();
    expect(box).toHaveValue("one\ntwo");
    await u.type(box, "{Enter}");
    expect(actions.say).toHaveBeenCalledWith("one\ntwo");
    expect(box).toHaveValue("");
    await u.type(box, "{Escape}");
    expect(actions.stop).toHaveBeenCalledTimes(1);
  });
});

describe("header", () => {
  it("ac3_allow_chip_takes_back", async () => {
    const actions = mk();
    render(<Header info={INFO} view={viewWith({ allows: [{ id: "al1", reach: "edits in src" }] })} actions={actions} />);
    const chip = screen.getByRole("button", { name: "edits in src this session ×" });
    await userEvent.setup().click(chip);
    expect(actions.takeBack).toHaveBeenCalledWith("al1");
  });

  const meter = (usage: SessionView["usage"]) => {
    render(<Header info={INFO} view={viewWith({ usage })} actions={mk()} />);
    return screen.getByRole("meter");
  };
  it("ac3_meter_shows_used_over_window_with_a_text_equivalent", () => {
    const m = meter({ used: 42000, window: 200000, compactAt: 160000 });
    expect(m).toHaveAttribute("aria-valuenow", "42000");
    expect(screen.getByText("21%")).toBeInTheDocument();
    expect(m).toHaveAttribute("aria-valuetext", "42k / 200k tokens");
  });
  it("ac3_meter_turns_amber_with_a_word_at_80_percent_of_compact_at", () => {
    meter({ used: 128000, window: 200000, compactAt: 160000 });
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuetext", "128k / 200k tokens, filling up");
    expect(document.querySelector(".wa-meter")).toHaveAttribute("data-warn", "true");
    expect(document.querySelector(".wa-bar")).toHaveClass("hi");
  });
  it("ac3_meter_turns_amber_at_80_percent_of_the_window_without_compact_at", () => {
    meter({ used: 160000, window: 200000 });
    expect(document.querySelector(".wa-bar")).toHaveClass("hi");
  });
});

describe("reload", () => {
  it("ac4_a_fresh_app_on_the_same_fake_shows_the_same_rows_none_doubled", async () => {
    const fake = played();
    const a = render(<App kernel={fake} info={INFO} />);
    await ready();
    const before = rowsOf(a.container);
    expect(before.length).toBeGreaterThan(4);
    a.unmount();
    const b = render(<App kernel={fake} info={INFO} />);
    await ready();
    expect(rowsOf(b.container)).toEqual(before);
  });

  it("ac4_a_dropped_stream_reattaches_after_the_last_index_and_shows_the_same", async () => {
    const fake = played();
    const a = render(<App kernel={fake} info={INFO} />);
    await ready();
    const before = rowsOf(a.container);
    act(() => fake.drop());
    await waitFor(() => expect(fake.calls.filter((c) => c.name === "attach")).toHaveLength(2));
    const attaches = fake.calls.filter((c) => c.name === "attach");
    expect(attaches[0].args[1]).toBe(0);
    expect(attaches[1].args[1]).toBe(8);
    await ready();
    expect(rowsOf(a.container)).toEqual(before);
  });

  it("ac2_open_scrolls_the_transcript_to_its_end", async () => {
    const set = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollHeight", { configurable: true, get: () => 900 });
    const desc = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollTop");
    Object.defineProperty(HTMLElement.prototype, "scrollTop", { configurable: true, get: () => 0, set });
    try {
      render(<App kernel={played()} info={INFO} />);
      await ready();
      expect(set).toHaveBeenCalledWith(900);
    } finally {
      delete (HTMLElement.prototype as any).scrollHeight;
      if (desc) Object.defineProperty(HTMLElement.prototype, "scrollTop", desc); else delete (HTMLElement.prototype as any).scrollTop;
    }
  });

  it("ac3_plan_is_folded_while_an_ask_waits_and_opens_by_the_participant", async () => {
    render(<App kernel={played()} info={INFO} />);
    await ready();
    expect(screen.queryByText("Add backoff")).toBeNull();
    await userEvent.setup().click(screen.getByRole("button", { name: "Plan · 1 of 3 done" }));
    expect(screen.getByText("Add backoff")).toBeInTheDocument();
  });

  it("ac3_plan_is_folded_while_inputs_are_queued", () => {
    const v: SessionView = { ...emptyView, plan: [{ content: "A", status: "completed" }], queue: [{ n: 1, input: "x" }] };
    render(<Dock view={v} actions={mk()} onNeedsYou={() => {}} status={null} />);
    expect(screen.getByRole("button", { name: "Plan · 1 of 1 done" })).toBeInTheDocument();
    expect(screen.queryByText("A")).toBeNull();
  });

  it("ac3_plan_is_listed_when_nothing_waits", () => {
    const v: SessionView = { ...emptyView, plan: [{ content: "A", status: "completed" }] };
    render(<Dock view={v} actions={mk()} onNeedsYou={() => {}} status={null} />);
    expect(screen.getByText("A")).toBeInTheDocument();
  });

  it("ac4_esc_and_cancel_close_the_deny_field_and_focus_deny", async () => {
    render(<App kernel={played()} info={INFO} />);
    await ready();
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.keyboard("{Escape}");
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Deny…" })));
    await u.click(screen.getByRole("button", { name: "Deny…" }));
    await u.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Deny…" })));
  });
});

describe("follow at end (18.6 Place)", () => {
  const more = (i: number, t: string) => JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: t }, _meta: { watane: { i, at: 9000 + i } } });
  // A 300 px viewport over a column of height h; setting scrollTop clamps like a browser.
  const box = () => {
    const b = { h: 900, top: 0 };
    const d = (p: string, o: PropertyDescriptor) => Object.defineProperty(HTMLElement.prototype, p, { configurable: true, ...o });
    d("scrollHeight", { get: () => b.h }); d("clientHeight", { get: () => 300 });
    d("scrollTop", { get: () => b.top, set: (v: number) => { b.top = Math.min(v, b.h - 300); } });
    return b;
  };
  const unbox = () => { for (const p of ["scrollHeight", "clientHeight", "scrollTop"]) delete (HTMLElement.prototype as any)[p]; };
  const open = async (extra: string[][]) => {
    const f = fakeKernel([...story, ...extra]);
    for (let n = 0; n < story.length; n++) f.play(n);
    const { container } = render(<App kernel={f} info={INFO} />);
    await ready();
    return { f, s: container.querySelector(".wa-scroll")! };
  };

  it("ac2_a_reader_who_scrolled_up_is_not_pulled_down_and_is_followed_again_at_the_end", async () => {
    const b = box();
    try {
      const { f, s } = await open([[more(20, "one")], [more(21, "two")]]);
      expect(b.top).toBe(600);
      b.top = 100; fireEvent.scroll(s);
      b.h = 1200;
      await act(async () => { f.play(story.length); });
      await screen.findByText("one");
      expect(b.top).toBe(100);
      b.top = 870; fireEvent.scroll(s); // 30 px above the end: within 40
      b.h = 1500;
      await act(async () => { f.play(story.length + 1); });
      await screen.findByText("two");
      expect(b.top).toBe(1200);
    } finally { unbox(); }
  });

  it("ac2_back_at_the_very_spot_the_console_last_scrolled_to_follows_again", async () => {
    const b = box();
    try {
      const { f, s } = await open([[more(20, "one")]]);
      b.top = 100; fireEvent.scroll(s);
      b.top = 600; fireEvent.scroll(s);
      b.h = 1200;
      await act(async () => { f.play(story.length); });
      await screen.findByText("one");
      expect(b.top).toBe(900);
    } finally { unbox(); }
  });

  it("ac2_the_resize_observer_is_disconnected_on_unmount", async () => {
    const observe = vi.fn(), disconnect = vi.fn();
    vi.stubGlobal("ResizeObserver", class { observe = observe; disconnect = disconnect; unobserve() {} });
    try {
      const f = played();
      const r = render(<App kernel={f} info={INFO} />);
      await ready();
      const col = r.container.querySelector(".wa-ask-scroll") ? observe.mock.calls.filter(([el]) => !(el as Element).classList.contains("wa-ask-scroll")) : observe.mock.calls;
      expect(col).toHaveLength(1);
      expect(disconnect).not.toHaveBeenCalled();
      r.unmount();
      expect(disconnect).toHaveBeenCalledTimes(observe.mock.calls.length);
    } finally { vi.unstubAllGlobals(); }
  });
});
