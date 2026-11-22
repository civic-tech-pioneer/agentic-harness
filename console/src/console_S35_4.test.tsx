// S35-4 AC2, AC4: the new-session view, Start in place, a refused start, the kept draft (component tests on a fake `window.watane`).
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Console } from "./Console";
import { served } from "./served";
import { fakeWatane } from "../kit/watane_fake";

afterEach(cleanup);
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 60)); });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const text = () => document.body.textContent ?? "";

function setup(over: Record<string, (a: any) => unknown> = {}) {
  let started = false;
  const row = { id: "n1", title: "Say hi", profile: "scratch", backend: "claude-code", turn: "running", updated: 9 };
  const f = fakeWatane({
    "sessions.list": () => ({ ok: started ? [row] : [] }),
    "sessions.start": () => { started = true; return { ok: "n1" }; },
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
    ...over,
  });
  return f;
}

describe("S35-4 AC2 the new-session view", () => {
  it("ac2_with_no_session_the_view_shows_the_stage_B_words_only", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
    expect(text()).toContain("Scratch");
    expect(text()).toContain("A new scratch folder");
    expect(box()).toHaveAttribute("placeholder", "Message Claude…");
    expect(screen.getByRole("button", { name: "Start session" })).toBeDisabled();
    expect(f.calls.map((c) => c.fn)).toEqual(["sessions.list"]); // no session was started by looking
  });
  it("ac2_send_starts_with_a_fresh_scratch_key_and_opens_the_session_in_place", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    const start = f.calls.find((c) => c.fn === "sessions.start")!;
    expect(start.args).toEqual({ key: expect.stringMatching(/^console-\d+-[a-z0-9]+$/), profile: "scratch", overlay: "{}", input: "hi" });
    expect(f.calls.some((c) => c.fn === "sessions.attach" && c.args.session === "n1")).toBe(true);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Say hi");
    expect(text()).toContain("scratch");
    expect(text()).toContain("Claude Code (unchecked)");
    expect(text()).toContain("Scratch folder");
    expect(screen.getAllByText("Say hi").length).toBeGreaterThan(1); // the sidebar lists it too
    expect(screen.queryByText("A new scratch folder")).toBeNull();
  });
  it("ac2_the_button_starts_too_and_a_new_session_after_it_shows_the_view_again", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.click(screen.getByRole("button", { name: "Start session" }));
    await tick();
    expect(f.calls.filter((c) => c.fn === "sessions.start")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
  });
  it("ac2_with_sessions_the_newest_opens_and_New_session_shows_the_view", async () => {
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "o", title: "Older", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }, { id: "z", title: "Newest", profile: "scratch", backend: "claude-code", turn: "idle", updated: 7 }] }) });
    render(<Console served={await served(f.w)} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Newest");
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
  });
  it("ac2_a_refused_start_keeps_the_text_and_alerts_in_words", async () => {
    const f = setup({ "sessions.start": () => ({ err: "busy" }) });
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(box().value).toBe("hi");
    expect(screen.getByRole("alert").textContent).toBe("Claude is busy with another session; try again in a moment.");
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
  });
});

describe("S35-4 AC4", () => {
  it("ac4b_the_draft_is_back_after_a_reload_until_sent", async () => {
    const f = setup();
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "half a thought" } });
    a.unmount();
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("half a thought");
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    cleanup();
    render(<Console served={await served(setup({ "sessions.list": () => ({ ok: [] }) }).w)} />);
    expect(box().value).toBe("");
  });
  it("ac4b_a_session_keeps_its_own_draft", async () => {
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "s", title: "T", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }] }) });
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "later" } });
    a.unmount();
    expect(sessionStorage.getItem("wa-draft:s")).toBe("later");
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("later");
  });
  it("ac4b_a_page_without_sessionStorage_still_works", async () => {
    const f = setup();
    const real = Object.getOwnPropertyDescriptor(window, "sessionStorage")!;
    Object.defineProperty(window, "sessionStorage", { configurable: true, get() { throw new Error("denied"); } });
    try {
      render(<Console served={await served(f.w)} />);
      fireEvent.change(box(), { target: { value: "hi" } });
      expect(box().value).toBe("hi");
    } finally { Object.defineProperty(window, "sessionStorage", real); }
  });
  it("ac4d_no_section_number_is_shown_or_read", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    expect(document.body.innerHTML).not.toMatch(/\(1\d\.\d\)|18\.8/);
    expect(document.getElementById("wa-off-new")?.textContent).toBe(undefined);
    fireEvent.change(box(), { target: { value: "x" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(document.body.innerHTML).not.toMatch(/\(1\d\.\d\)|18\.8/);
    expect(document.getElementById("wa-off-menu")?.textContent).toBe("Not in this console yet.");
  });
});
