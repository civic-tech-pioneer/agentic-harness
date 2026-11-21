// Adversary S23: a hostile code block's fence label must not stall the highlight queue past the 250 ms budget (TECHNICAL 18.6).
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HIGHLIGHT_BUDGET_MS, highlight, known, setWorkerFactory, type WorkerLike } from "./parts/highlight";

/** A Worker that copies each message the way the browser does (structured clone) and answers at once. */
const factory = () => () => {
  const w: WorkerLike = {
    onmessage: null, onerror: null, terminate() {},
    postMessage(m) {
      const c = structuredClone(m); // a function or a prototype in the message throws DataCloneError, as in the browser
      queueMicrotask(() => w.onmessage?.({ data: c.op === "load" ? { id: c.id } : { id: c.id, lines: [[{ t: c.code! }]] } }));
    },
  };
  return w;
};
beforeEach(() => { vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); setWorkerFactory(undefined); });

describe("adv S23 highlight", () => {
  it("adv_S23_fence_label_constructor_is_not_a_language", () => {
    // ```constructor and ```__proto__ pass the fence pattern; they name inherited members of the alias table, not languages
    expect(known("constructor")).toBeUndefined();
    expect(known("__proto__")).toBeUndefined();
    expect(known("toString")).toBeUndefined();
  });

  it("adv_S23_fence_label_constructor_stalls_the_queue_past_the_budget", async () => {
    setWorkerFactory(factory());
    const hostile = highlight("constructor", "x").catch(() => null);
    const legit = highlight("ts", "const a = 1");
    let got: unknown = "pending";
    void legit.then((v) => { got = v; });
    await vi.advanceTimersByTimeAsync(HIGHLIGHT_BUDGET_MS * 4);
    // the legit block is next in line: it must not wait behind the hostile one for the 10 s language-load limit
    expect(got).toEqual([[{ t: "const a = 1" }]]);
    await hostile;
  });
});
