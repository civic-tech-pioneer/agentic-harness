// S23-2: code highlighting off the main thread with a time budget (TECHNICAL 18.6). A fake Worker drives the queue, the budget and the end of a Worker.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HIGHLIGHT_BUDGET_MS, highlight, peek, setWorkerFactory, type WorkerLike, type WorkerRequest } from "./parts/highlight";

type Fake = WorkerLike & { terminated: boolean; seen: WorkerRequest[] };
let made: Fake[] = [];
/** A Worker that answers loads at once and a block's tokens at once, unless its code is in `hang` (then never). */
const factory = (hang: string[]) => () => {
  const w: Fake = {
    terminated: false, seen: [], onmessage: null, onerror: null,
    terminate() { w.terminated = true; },
    postMessage(m) {
      w.seen.push(m);
      if (m.op === "tokens" && hang.includes(m.code!)) return;
      queueMicrotask(() => w.onmessage?.({ data: m.op === "load" ? { id: m.id } : { id: m.id, lines: [[{ t: m.code! }]] } }));
    },
  };
  made.push(w);
  return w;
};
beforeEach(() => { made = []; vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); setWorkerFactory(undefined); });

describe("highlight budget", () => {
  it("ac2_the_budget_is_250_ms", () => { expect(HIGHLIGHT_BUDGET_MS).toBe(250); });

  it("ac4_a_block_over_the_budget_stays_plain_its_worker_ends_and_the_next_block_gets_a_new_one", async () => {
    setWorkerFactory(factory(["slow A"]));
    const a = highlight("ts", "slow A");
    const b = highlight("ts", "fast B");
    await vi.advanceTimersByTimeAsync(HIGHLIGHT_BUDGET_MS - 1);
    expect(made).toHaveLength(1);
    expect(made[0].terminated).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(await a).toBeNull();
    expect(made[0].terminated).toBe(true);
    expect(await b).toEqual([[{ t: "fast B" }]]);
    expect(made).toHaveLength(2);
    expect(made[1].seen.filter((m) => m.code === "fast B")).toHaveLength(1);
    expect(peek("ts", "slow A")).toBeUndefined(); // plain: no tokens
    made.length = 0;
    expect(await highlight("ts", "slow A")).toBeNull(); // cached as plain: no retry, no Worker
    expect(made).toHaveLength(0);
  });

  it("ac2_blocks_are_sent_one_at_a_time_in_order_of_request", async () => {
    setWorkerFactory(factory([]));
    const ps = ["one", "two", "three"].map((c) => highlight("ts", c));
    await vi.advanceTimersByTimeAsync(0);
    expect(await Promise.all(ps)).toEqual([[[{ t: "one" }]], [[{ t: "two" }]], [[{ t: "three" }]]]);
    expect(made).toHaveLength(1);
    expect(made[0].seen.filter((m) => m.op === "tokens").map((m) => m.code)).toEqual(["one", "two", "three"]);
    expect(made[0].seen.filter((m) => m.op === "load")).toHaveLength(1);
  });

  it("ac2_a_worker_error_leaves_the_block_plain_and_the_queue_moves_on", async () => {
    let n = 0;
    setWorkerFactory(() => {
      const w = factory([])();
      if (n++ === 0) w.postMessage = (m) => { if (m.op === "tokens") queueMicrotask(() => w.onerror?.(new Error("x"))); else queueMicrotask(() => w.onmessage?.({ data: { id: m.id } })); };
      return w;
    });
    const a = highlight("ts", "err A");
    const b = highlight("ts", "ok B");
    await vi.advanceTimersByTimeAsync(0);
    expect(await a).toBeNull();
    expect(await b).toEqual([[{ t: "ok B" }]]);
    expect(made[0].terminated).toBe(true);
  });

  it("ac3_resetting_the_seam_leaves_the_block_in_flight_and_the_queued_ones_plain", async () => {
    setWorkerFactory(factory(["hung C"]));
    const c = highlight("ts", "hung C");
    const d = highlight("ts", "queued D");
    await vi.advanceTimersByTimeAsync(0);
    setWorkerFactory(undefined);
    expect(await c).toBeNull();
    expect(await d).toBeNull();
    expect(made[0].terminated).toBe(true);
  });

  it("ac3_with_no_worker_and_no_factory_the_block_stays_plain", async () => {
    setWorkerFactory(undefined);
    expect(await highlight("ts", "no worker")).toBeNull();
  });
});
