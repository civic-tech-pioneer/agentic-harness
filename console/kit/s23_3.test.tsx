// S23-3: the governor clamps a skewed clock (AC1); a session paging in says Loading, never Idle (AC2).
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { governor, openSession, type Kernel } from "./kernel";
import { emptyView, type SessionView } from "./model";
import { StatusLine } from "./status";
import { longKernel } from "../fake/page";

afterEach(() => { cleanup(); vi.useRealTimers(); vi.restoreAllMocks(); });

describe("S23-3", () => {
  it("ac1_a_clock_set_back_an_hour_never_stalls_the_governor", async () => {
    vi.useFakeTimers();
    const g = governor();
    let ran = 0;
    for (let i = 0; i < 9; i++) void g.take().then(() => ran++);
    await vi.advanceTimersByTimeAsync(0);
    expect(ran).toBe(8); // the burst
    vi.setSystemTime(Date.now() - 3_600_000);
    await vi.advanceTimersByTimeAsync(500);
    expect(ran).toBe(9); // the 9th waits at most one refill interval (250 ms) past its due time, not an hour
    for (let i = 0; i < 4; i++) void g.take().then(() => ran++);
    await vi.advanceTimersByTimeAsync(10);
    expect(ran).toBe(9); // no burst of free tokens: the rate holds
    g.stop();
  });

  it("ac1_a_clock_jumped_forward_refills_to_the_burst_only", async () => {
    vi.useFakeTimers();
    const g = governor();
    let ran = 0;
    for (let i = 0; i < 8; i++) void g.take().then(() => ran++);
    await vi.advanceTimersByTimeAsync(0);
    vi.setSystemTime(Date.now() + 86_400_000);
    for (let i = 0; i < 12; i++) void g.take().then(() => ran++);
    await vi.advanceTimersByTimeAsync(0);
    expect(ran).toBe(16);
    g.stop();
  });

  it("ac2_emptyView_is_not_loading", () => { expect(emptyView.loading).toBe(false); });

  it("ac2_the_long_fixture_says_Loading_while_it_pages_in_never_Idle", async () => {
    const k: Kernel = longKernel(1200);
    const views: SessionView[] = [];
    const s = openSession(k, "s", (v) => views.push(v));
    await vi.waitFor(() => expect(views.at(-1)?.loading === false && views.at(-1)!.lastIndex >= 1200).toBe(true), { timeout: 5000 });
    s.close();
    const unknownTurn = views.filter((v) => v.turn === null);
    expect(views.some((v) => v.loading)).toBe(true);
    for (const v of unknownTurn) {
      expect(v.loading).toBe(true);
      const { unmount } = render(<StatusLine view={v} now={0} stopping={false} actions={{} as never} onNeedsYou={() => {}} />);
      expect(screen.getByRole("status").textContent).toBe("Loading");
      expect(screen.getByRole("img", { name: "Loading" }).getAttribute("data-shape")).toBe("loading");
      unmount();
    }
    expect(views.at(-1)!.turn).not.toBeNull();
  });

  it("ac2_a_failed_page_in_ends_loading", async () => {
    const k: Kernel = { say: async () => 0, start: async () => "s", answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      attach: async () => ({ behind: 10 }), read: async () => { throw new Error("x"); } };
    let last: SessionView | undefined;
    const s = openSession(k, "s", (v) => { last = v; });
    await vi.waitFor(() => expect(last).toBeDefined());
    await vi.waitFor(() => expect(last!.loading).toBe(false));
    s.close();
  });

  it("ac2_loading_ends_when_the_reattach_after_a_page_in_gives_up", async () => {
    vi.useFakeTimers();
    let attaches = 0;
    const k: Kernel = { say: async () => 0, start: async () => "s", answer: async () => {}, interrupt: async () => {}, takeBack: async () => {},
      attach: async () => { if (attaches++ === 0) return { behind: 1 }; throw new Error("down"); },
      read: async () => [JSON.stringify({ v: 1, i: 1, kind: "message", at: 0, body: { role: "user", content: [{ type: "text", text: "hi" }] } })] };
    let last: SessionView | undefined;
    const s = openSession(k, "s", (v) => { last = v; });
    await vi.advanceTimersByTimeAsync(60_000);
    expect(attaches).toBe(6); // the first, then five failed tries: the console has stopped
    expect(last!.loading).toBe(false);
    s.close();
  });
});
