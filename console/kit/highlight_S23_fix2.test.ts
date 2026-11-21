import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { highlight, setWorkerFactory, type WorkerLike } from "./parts/highlight";

beforeEach(() => { vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); setWorkerFactory(undefined); });

it("ac3_postmessage_that_throws_fails_the_block_at_once_and_the_next_starts", async () => {
  let ended = 0;
  setWorkerFactory(() => {
    const w: WorkerLike = {
      onmessage: null, onerror: null, terminate() { ended++; },
      postMessage(m) {
        if (m.op === "tokens" && m.code === "bad") throw new Error("DataCloneError");
        queueMicrotask(() => w.onmessage?.({ data: m.op === "load" ? { id: m.id } : { id: m.id, lines: [[{ t: m.code! }]] } }));
      },
    };
    return w;
  });
  const bad = highlight("ts", "bad");
  const good = highlight("ts", "good");
  await vi.advanceTimersByTimeAsync(10);
  expect(await bad).toBeNull();
  expect(ended).toBe(1);
  expect(await good).toEqual([[{ t: "good" }]]);
  expect(vi.getTimerCount()).toBe(0);
});
