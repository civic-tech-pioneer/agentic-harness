// @vitest-environment node
// S7-2 AC5 (P14): one render per frame. openSession tells the page at most once per animation frame; the reducer still sees every message.
import { afterEach, describe, expect, it, vi } from "vitest";
import { emptyView, type SessionView } from "./model";
import { openSession, type Kernel } from "./kernel";
import { safeReduce } from "./reduce";

const j = JSON.stringify;
const synced = j({ _meta: { watane: { type: "synced", through: 0 } } });
const delta = (k: number) => j({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: `d${k} ` } });
const base = { say: async () => 0, start: async () => "s", answer: async () => {}, interrupt: async () => {}, unqueue: async () => {}, takeBack: async () => {} };
/** A kernel that hands `msgs` to the stream at attach and keeps the stream open so a test can push more. */
function kernelOf(msgs: string[]) {
  let push: (m: string) => void = () => {};
  const k: Kernel = { ...base, read: async () => [], attach: async (_s, _a, on) => { push = on; for (const m of msgs) on(m); return { ok: true }; } };
  return { k, push: (m: string) => push(m) };
}
const settle = async () => { for (let n = 0; n < 20; n++) await Promise.resolve(); };

/** A controlled requestAnimationFrame: callbacks wait until `frame()` runs them. */
function frames() {
  const queue = new Map<number, () => void>();
  let id = 0;
  const cancel = vi.fn((n: number) => { queue.delete(n); });
  vi.stubGlobal("requestAnimationFrame", (cb: () => void) => { queue.set(++id, cb); return id; });
  vi.stubGlobal("cancelAnimationFrame", cancel);
  return { pending: () => queue.size, cancel, frame() { const run = [...queue.values()]; queue.clear(); run.forEach((f) => f()); } };
}
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); });

describe("one render per frame", () => {
  it("ac5_200_deltas_between_two_frames_give_one_render_with_every_delta_reduced", async () => {
    const f = frames();
    const deltas = Array.from({ length: 200 }, (_, k) => delta(k));
    const { k } = kernelOf([synced, ...deltas]);
    const views: SessionView[] = [];
    const s = openSession(k, "s1", (v) => views.push(v));
    await settle();
    expect(views).toHaveLength(0); // nothing reaches the page before the frame
    expect(f.pending()).toBe(1);
    f.frame();
    expect(views).toHaveLength(1);
    const want = [synced, ...deltas].reduce(safeReduce, { ...emptyView, synced: false });
    expect(views[0]).toEqual(want);
    const row = views[0]!.rows.at(-1)!;
    expect(row).toMatchObject({ kind: "agent", text: Array.from({ length: 200 }, (_, n) => `d${n} `).join("") }); // in order, none dropped
    s.close();
  });

  it("ac5_messages_of_the_next_frame_give_the_next_render", async () => {
    const f = frames();
    const { k, push } = kernelOf([synced, delta(0)]);
    const views: SessionView[] = [];
    const s = openSession(k, "s1", (v) => views.push(v));
    await settle();
    f.frame();
    expect(views).toHaveLength(1);
    for (let n = 1; n <= 50; n++) push(delta(n));
    expect(views).toHaveLength(1);
    f.frame();
    expect(views).toHaveLength(2);
    f.frame(); // an empty frame renders nothing
    expect(views).toHaveLength(2);
    s.close();
  });

  it("ac5_without_requestAnimationFrame_a_16_ms_timer_does_the_same", async () => {
    vi.useFakeTimers();
    expect(typeof requestAnimationFrame).toBe("undefined");
    const { k } = kernelOf([synced, ...Array.from({ length: 200 }, (_, n) => delta(n))]);
    const views: SessionView[] = [];
    const s = openSession(k, "s1", (v) => views.push(v));
    await vi.advanceTimersByTimeAsync(15);
    expect(views).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(2);
    expect(views).toHaveLength(1);
    expect(views[0]!.rows.at(-1)).toMatchObject({ kind: "agent" });
    s.close();
  });

  it("ac5_closing_the_session_cancels_a_pending_frame", async () => {
    const f = frames();
    const { k } = kernelOf([synced, delta(0)]);
    const views: SessionView[] = [];
    const s = openSession(k, "s1", (v) => views.push(v));
    await settle();
    expect(f.pending()).toBe(1);
    s.close();
    expect(f.cancel).toHaveBeenCalledTimes(1);
    expect(f.pending()).toBe(0);
    f.frame();
    expect(views).toHaveLength(0);
  });

  it("ac5_closing_the_session_cancels_a_pending_timer", async () => {
    vi.useFakeTimers();
    const { k } = kernelOf([synced, delta(0)]);
    const views: SessionView[] = [];
    const s = openSession(k, "s1", (v) => views.push(v));
    await vi.advanceTimersByTimeAsync(1);
    s.close();
    expect(vi.getTimerCount()).toBe(0);
    await vi.advanceTimersByTimeAsync(100);
    expect(views).toHaveLength(0);
  });
});
