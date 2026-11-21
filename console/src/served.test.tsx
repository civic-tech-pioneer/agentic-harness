// S32-3 AC4, AC5: the page path on a fake `window.watane`.
import { describe, expect, it } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { App } from "./App";
import { isServed, openFailed, served } from "./served";
import { entry, fakeWatane, items, synced } from "../kit/watane_fake";

const LIST = [
  { id: "a", title: "Old probe", profile: "p", backend: "probe", turn: "idle", updated: 5 },
  { id: "b", title: "Other", profile: "p", backend: "claude-code", turn: "idle", updated: 99 },
  { id: "c", title: "New probe", profile: "p", backend: "probe", turn: "idle", updated: 50 },
];

describe("S32-3 AC4", () => {
  it("ac4_opens_the_newest_probe_session", async () => {
    const f = fakeWatane({ "sessions.list": () => ({ ok: LIST }) });
    const x = await served(f.w);
    expect(f.calls[0]).toEqual({ fn: "sessions.list", args: { after: null, max: 256 } });
    expect(x.info).toMatchObject({ id: "c", title: "New probe", provider: "Stage A probe" });
    expect(x.sessions.map((s) => s.title)).toEqual(["Old probe", "Other", "New probe"]);
    expect(f.calls.some((c) => c.fn === "probe.start")).toBe(false);
  });
  it("ac4_starts_a_probe_when_there_is_none", async () => {
    const f = fakeWatane({ "sessions.list": () => ({ ok: [] }), "probe.start": () => ({ ok: "new" }) });
    const x = await served(f.w);
    expect(f.calls[1]).toEqual({ fn: "probe.start", args: { key: expect.stringMatching(/^console-probe-/), count: 1000, "every-ms": 1000 } });
    expect(x.info.id).toBe("new");
  });
  it("ac4_a_started_probe_takes_its_title_from_list_and_shows_in_the_sidebar", async () => {
    let started = false;
    const probe = { id: "new", title: "Numbered items", profile: "", backend: "probe", turn: "idle", updated: 1 };
    const f = fakeWatane({ "sessions.list": () => ({ ok: started ? [probe] : [] }), "probe.start": () => { started = true; return { ok: "new" }; } });
    const x = await served(f.w);
    expect(x.info).toMatchObject({ id: "new", title: "Numbered items", provider: "Stage A probe" });
    expect(x.sessions.map((s) => s.id)).toEqual(["new"]);
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
      "sessions.list": () => ({ ok: [] }), "probe.start": () => ({ ok: "p1" }),
      "sessions.attach": (a) => ({ ok: `st${++n}`, after: a.after }),
      "sessions.read": () => ({ ok: { entries: log, last: 3 } }),
    });
    const x = await served(f.w);
    const mount = () => render(<App kernel={x.kernel} info={x.info} sessions={x.sessions} />);
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
  it("ac1_a_fresh_key_each_start", async () => {
    const keys: string[] = [];
    const mk = () => fakeWatane({ "sessions.list": () => ({ ok: [] }), "probe.start": (a) => { keys.push(a.key as string); return { ok: "new" }; } });
    await served(mk().w);
    await served(mk().w);
    expect(keys[0]).not.toBe(keys[1]);
    for (const k of keys) { expect(k).toMatch(/^console-probe-\d+-[a-z0-9]+$/); expect(k.length).toBeLessThanOrEqual(64); }
  });
  it("ac2_served_is_chosen_by_origin", () => {
    expect(isServed("probe.localhost")).toBe(true);
    expect(isServed("localhost")).toBe(false);
    expect(isServed("127.0.0.1")).toBe(false);
  });
});
