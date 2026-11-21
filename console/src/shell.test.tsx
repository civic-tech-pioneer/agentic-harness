// S9-3: header chips, context meter, ⋯ button, sidebar groups (TECHNICAL 18.4-18.6, 18.8).
import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fakeKernel } from "../fake/kernel";
import { story } from "../fake/story";
import { emptyView, type Actions, type SessionView } from "../kit";
import { App } from "./App";
import { Header } from "./Header";
import { Sidebar, type SidebarSession } from "./Sidebar";
import { storyInfo, storySessions } from "../fake/page";

const mk = (): Actions => ({ say: vi.fn(), answer: vi.fn(), stop: vi.fn(), takeBack: vi.fn() });
const vw = (p: Partial<SessionView>): SessionView => ({ ...emptyView, ...p });
const turn = (state: object) => JSON.stringify({ _meta: { watane: { type: "turn", state } } });

describe("header", () => {
  it("ac1_profile_chip_provider_chip_folder_chip", () => {
    const { container } = render(<Header info={storyInfo} view={vw({})} actions={mk()} />);
    const head = screen.getByRole("banner");
    expect(within(head).getByText("Profile", { exact: false }).textContent).toBe("Profile Careful");
    expect(within(head).getByText("Careful").tagName).toBe("B");
    const prov = within(head).getByText("Claude Code · no judge").closest(".wa-chip")!;
    expect(prov.querySelector("svg")).not.toBeNull();
    expect(prov).not.toHaveClass("checked");
    expect(prov).toHaveAttribute("title", "No judge reads this provider's calls. Your rules and asks still apply.");
    const folder = within(head).getByText(storyInfo.workspace);
    expect(folder.tagName).toBe("B");
    expect(folder.closest(".wa-chip")!.querySelector("svg")).not.toBeNull();
    expect(container.querySelectorAll(".wa-chip").length).toBe(3);
  });
  it("ac1_judge_gives_the_checked_look_and_title", () => {
    render(<Header info={{ ...storyInfo, judge: true }} view={vw({})} actions={mk()} />);
    const prov = screen.getByText("Claude Code · your judge checks calls").closest(".wa-chip")!;
    expect(prov).toHaveClass("checked");
    expect(prov).toHaveAttribute("title", "Your judge reads each call before it runs and can deny it.");
  });
  it("ac2_meter_word_bar_percent_and_warn", () => {
    render(<Header info={storyInfo} view={vw({ usage: { used: 42000, window: 200000, compactAt: 160000 } })} actions={mk()} />);
    expect(screen.getByText("Context")).toBeInTheDocument();
    expect(screen.getByText("21%")).toBeInTheDocument();
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuetext", "42k / 200k tokens");
    expect(document.querySelector(".wa-bar")).not.toHaveClass("hi");
  });
  it("ac2_warn_tints_the_bar", () => {
    render(<Header info={storyInfo} view={vw({ usage: { used: 128000, window: 200000, compactAt: 160000 } })} actions={mk()} />);
    expect(document.querySelector(".wa-bar")).toHaveClass("hi");
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuetext", "128k / 200k tokens, filling up");
    expect(screen.getByText("64%")).toBeInTheDocument();
  });
  it("ac3_menu_button_is_shown_off", async () => {
    render(<Header info={storyInfo} view={vw({})} actions={mk()} />);
    const b = screen.getByRole("button", { name: "Session menu: fork, rewind, rename, delete" });
    expect(b).toHaveAttribute("aria-disabled", "true");
    expect(b).toHaveAccessibleDescription(/not in this console yet/i);
    await userEvent.setup().click(b);
    expect(screen.queryByRole("menu")).toBeNull();
  });
});

describe("sidebar", () => {
  const rows: SidebarSession[] = [
    { id: "a", title: "Reply to Ivy", place: "Mail", meta: "Mail · asks to send", state: "needs" },
    { id: "b", title: "Weekly review", place: "Desk", meta: "Desk · ChatGPT", state: "running" },
    { id: "c", title: "Sort the inbox", place: "Mail", meta: "Mail · paused", state: "paused" },
  ];
  it("ac4_brand_and_new_session_shown_off", async () => {
    render(<Sidebar info={storyInfo} turn="idle" sessions={[]} />);
    const nav = screen.getByRole("navigation", { name: "Sessions" });
    expect(within(nav).getByText("Agents")).toBeInTheDocument();
    const b = within(nav).getByRole("button", { name: /New session/ });
    expect(b).toHaveAttribute("aria-disabled", "true");
    expect(b).toHaveAccessibleDescription(/not in this console yet/i);
    await userEvent.setup().click(b);
  });
  it("ac5_groups_rows_avatar_subtitle_shape", () => {
    render(<Sidebar info={storyInfo} turn="running" sessions={rows} />);
    const labels = [...document.querySelectorAll(".wa-glabel")].map((e) => e.textContent);
    expect(labels).toEqual(["Needs you", "Running", "Recent"]);
    const cur = screen.getByRole("button", { current: true });
    expect(cur).toHaveTextContent("Fix the retry bug in sync.ts");
    expect(cur.closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Running");
    expect(cur.querySelector(".wa-mono")!.textContent).toBe("S");
    expect(cur.querySelector(".wa-smeta")!.textContent).toBe("sync · Claude Code");
    expect(within(cur).getByRole("img", { name: "Running" })).toHaveClass("g-run");
    const ivy = screen.getByRole("button", { name: /Reply to Ivy/ });
    expect(within(ivy).getByRole("img", { name: "Needs you" })).toHaveClass("g-ask");
    expect(ivy.querySelector(".wa-mono")!.textContent).toBe("M");
    expect(within(screen.getByRole("button", { name: /Sort the inbox/ })).getByRole("img", { name: "Paused" })).toHaveClass("g-pause");
  });
  it("ac5_empty_groups_left_out", () => {
    render(<Sidebar info={storyInfo} turn="idle" sessions={[]} />);
    expect([...document.querySelectorAll(".wa-glabel")].map((e) => e.textContent)).toEqual(["Recent"]);
  });
  it("ac5_shapes_per_state", () => {
    const all: SidebarSession[] = (["idle", "unknown"] as const).map((s) => ({ id: s, title: s, place: "X", meta: "X · " + s, state: s }));
    render(<Sidebar info={storyInfo} turn="running" sessions={all} />);
    expect(within(screen.getByRole("button", { name: /unknown/ })).getByRole("img", { name: "Unknown" })).toHaveClass("g-unknown");
    expect(within(screen.getByRole("button", { name: /idle/ })).getByRole("img", { name: "Idle" })).toHaveClass("g-idle");
    expect(screen.getByRole("button", { name: /idle/ }).closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Recent");
  });
  it("ac5_rows_hold_their_group_while_the_pointer_is_over_the_list", () => {
    const { rerender } = render(<Sidebar info={storyInfo} turn="running" sessions={rows} />);
    const nav = screen.getByRole("navigation", { name: "Sessions" });
    fireEvent.mouseEnter(nav);
    rerender(<Sidebar info={storyInfo} turn="waiting" sessions={rows} />);
    const cur = screen.getByRole("button", { current: true });
    expect(cur.closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Running");
    expect(within(cur).getByRole("img", { name: "Needs you" })).toBeInTheDocument();
    fireEvent.mouseLeave(nav);
    expect(screen.getByRole("button", { current: true }).closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Needs you");
  });
});

describe("S15-3 sidebar hold", () => {
  it("ac2_the_hold_begins_once_the_turn_is_known", () => {
    const rows: SidebarSession[] = [];
    const { rerender } = render(<Sidebar info={storyInfo} turn={undefined} sessions={rows} />);
    const nav = screen.getByRole("navigation", { name: "Sessions" });
    fireEvent.mouseEnter(nav);
    rerender(<Sidebar info={storyInfo} turn="waiting" sessions={rows} />);
    expect(screen.getByRole("button", { current: true }).closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Needs you");
    rerender(<Sidebar info={storyInfo} turn="running" sessions={rows} />);
    expect(screen.getByRole("button", { current: true }).closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Needs you");
    fireEvent.mouseLeave(nav);
    expect(screen.getByRole("button", { current: true }).closest(".wa-group")!.querySelector(".wa-glabel")!.textContent).toBe("Running");
  });
});

describe("app", () => {
  it("ac6_story_sessions_have_the_frames_mix", () => {
    expect(storySessions.map((s) => s.state).sort()).toEqual(["idle", "needs", "paused", "running", "unknown"]);
    expect(storyInfo).toMatchObject({ place: "sync", model: "Claude Sonnet", mode: "Ask before edits" });
    expect("judge" in storyInfo && (storyInfo as { judge?: boolean }).judge).toBeFalsy();
  });
  it("ac5_open_row_follows_the_turn_in_the_app", async () => {
    const f = fakeKernel([...story, [turn({ state: "idle", stop: { kind: "completed" } })]]);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={storyInfo} sessions={storySessions} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await waitFor(() => expect(document.querySelectorAll(".wa-row").length).toBeGreaterThan(4));
    const cur = screen.getByRole("button", { current: true });
    expect(within(cur).getByRole("img", { name: "Needs you" })).toBeInTheDocument();
    act(() => f.play(story.length));
    expect(within(screen.getByRole("button", { current: true })).getByRole("img", { name: "Idle" })).toBeInTheDocument();
  });
  it("ac6_without_sessions_only_the_open_one_shows", () => {
    render(<App kernel={fakeKernel([])} info={storyInfo} />);
    expect(within(screen.getByRole("navigation", { name: "Sessions" })).getAllByRole("button").length).toBe(2);
  });
  it("ac7_app_passes_model_mode_and_stopping_to_the_dock", async () => {
    const f = fakeKernel(story);
    for (let n = 0; n < story.length; n++) f.play(n);
    render(<App kernel={f} info={storyInfo} />);
    await screen.findByRole("button", { name: "Needs you (1)" });
    await waitFor(() => expect(document.querySelectorAll(".wa-row").length).toBeGreaterThan(4));
    // Each tag is its own box; the "·" between them is aria-hidden, so the words never run together.
    expect(Array.from(document.querySelectorAll(".wa-tag"), (t) => t.lastChild?.textContent)).toEqual(["Claude Sonnet", "Ask before edits"]);
    await userEvent.setup().click(screen.getByRole("button", { name: "Stop" }));
    expect(screen.getByRole("button", { name: "Stopping…" })).toBeInTheDocument();
  });
});
