// S32-3 AC4, AC5: the page path on a fake `window.watane`.
import { describe, expect, it } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { App } from "./App";
import { isServed, openFailed, served } from "./served";
import { entry, fakeWatane, items, synced } from "../kit/watane_fake";

const LIST = [
  { id: "a", title: "Old probe", profile: "p", backend: "probe", turn: "idle", updated: 5 },
  { id: "b", title: "Other", profile: "scratch", backend: "claude-code", turn: "idle", updated: 99 },
  { id: "c", title: "New probe", profile: "p", backend: "probe", turn: "idle", updated: 50 },
];

describe("S32-3 AC4 / S35-4 AC2: what opens", () => {
  it("ac2_opens_the_newest_session_by_updated_whatever_its_backend", async () => {
    const f = fakeWatane({ "sessions.list": () => ({ ok: LIST }) });
    const x = await served(f.w);
    expect(f.calls[0]).toEqual({ fn: "sessions.list", args: { after: null, max: 256 } });
    expect(x.info).toMatchObject({ id: "b", title: "Other", profile: "scratch", provider: "Claude Code (unchecked)", workspace: "Scratch folder" });
    expect(x.sessions.map((s) => s.title)).toEqual(["Old probe", "Other", "New probe"]);
    expect(f.calls.map((c) => c.fn)).toEqual(["sessions.list"]);
  });
  it("ac2_with_no_session_nothing_is_started_and_the_new_session_view_is_wanted", async () => {
    const f = fakeWatane({ "sessions.list": () => ({ ok: [] }) });
    const x = await served(f.w);
    expect(x.info).toBeNull();
    expect(f.calls.map((c) => c.fn)).toEqual(["sessions.list"]);
  });
});

describe("S32-3 failure", () => {
  it("served_with_a_failing_list_shows_the_cause_in_root", async () => {
    const f = fakeWatane({ "sessions.list": () => ({ err: "not-owner" }) });
    const root = document.createElement("div");
    await served(f.w).catch((e) => openFailed(root, e));
    expect(root.textContent).toMatch(/could not open a session.*sessions.list failed.*not-owner/);
  });
});

describe("S32-3 AC5", () => {
  it("ac5_entries_appear_in_order_live_and_after_a_remount_once_each", async () => {
    const log = [1, 2, 3].map((n) => entry(n, `Item ${n} of 3`));
    let n = 0;
    const f = fakeWatane({
      "sessions.list": () => ({ ok: [{ id: "p1", title: "Items", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }] }),
      "sessions.attach": (a) => ({ ok: `st${++n}`, after: a.after }),
      "sessions.read": () => ({ ok: { entries: log, last: 3 } }),
    });
    const x = await served(f.w);
    const mount = () => render(<App kernel={x.kernel} info={x.info!} sessions={x.sessions} />);
    mount();
    await new Promise((r) => setTimeout(r, 50));
    f.push({ ...items([1, log[0]], [2, log[1]]), stream: "st1" });
    f.push({ ...items([3, log[2]], [4, synced(3)]), stream: "st1" });
    for (const t of ["Item 1 of 3", "Item 2 of 3", "Item 3 of 3"]) await screen.findByText(t);
    const order = (c: Element) => [...c.querySelectorAll("*")].filter((e) => e.children.length === 0 && /^Item \d of 3$/.test(e.textContent ?? "")).map((e) => e.textContent);
    expect(order(document.body)).toEqual(["Item 1 of 3", "Item 2 of 3", "Item 3 of 3"]);
    cleanup();
    // the reload: a new mount attaches afresh; the replay shows each entry once
    mount();
    await new Promise((r) => setTimeout(r, 50));
    f.push({ ...items([1, log[0]], [2, log[1]], [3, log[2]], [4, synced(3)]), stream: "st2" });
    await screen.findByText("Item 3 of 3");
    expect(order(document.body)).toEqual(["Item 1 of 3", "Item 2 of 3", "Item 3 of 3"]);
  });
});

describe("S32-fix2", () => {
  it("ac2_served_is_chosen_by_origin", () => {
    expect(isServed("claude.localhost")).toBe(true);
    expect(isServed("localhost")).toBe(false);
    expect(isServed("127.0.0.1")).toBe(false);
  });
});

describe("S35-fix5: titles and meta", () => {
  const two = [
    { id: "a", title: "", profile: "scratch", backend: "claude-code", turn: "idle", updated: 99 },
    { id: "b", title: "", profile: "scratch", backend: "claude-code", turn: "idle", updated: 5 },
  ];
  const mount = async (text: string) => {
    const f = fakeWatane({
      "sessions.list": () => ({ ok: two }),
      "sessions.attach": () => ({ ok: "st1", after: 0 }),
      "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
    });
    const x = await served(f.w);
    render(<App kernel={x.kernel} info={x.info!} sessions={x.sessions} />);
    await new Promise((r) => setTimeout(r, 50));
    f.push({ ...items([1, JSON.stringify({ v: 1, i: 1, kind: "message", at: 1000, body: { role: "user", content: [{ type: "text", text }] } })], [2, synced(1)]), stream: "st1" });
    return screen;
  };
  it("ac1_header_and_sidebar_name_sessions_by_one_rule", async () => {
    const s = await mount("hi");
    await s.findByRole("heading", { level: 1, name: "hi" });
    const rows = [...document.querySelectorAll(".wa-srow")].map((r) => r.textContent);
    expect(rows).toHaveLength(2);
    expect(rows.some((t) => t!.includes("hi"))).toBe(true);
    expect(rows.some((t) => t!.includes("New session"))).toBe(true);
    for (const t of rows) expect(t).toContain("Scratch folder · Claude Code (unchecked)");
    cleanup();
  });
  it("ac2_a_long_first_message_is_cut_to_60_with_an_ellipsis", async () => {
    const s = await mount("x".repeat(100));
    await s.findByRole("heading", { level: 1, name: `${"x".repeat(60)}…` });
    cleanup();
  });
});
