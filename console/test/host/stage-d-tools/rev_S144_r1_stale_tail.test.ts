// S144 review r1 validator: the Stop-cut excuse must be bounded by the evidence (a batch the kernel had in hand at the
// click). A whole step written 1000 ms before the click (ten times the 100 ms item-delay budget) that reached neither the
// page nor the store is a lost batch, not a Stop cut.
import { expect, it } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { openaiPageCoverage } from "./metrics.mjs";

const step = (n: number, at: number) => {
  const m = `oa-s1-${n}`;
  return [{ marker: m, step: m, at }, ...[0, 1, 2, 3].map((j) => ({ marker: `call_${n}_${j}`, step: m, at: at + 1 })),
    { marker: `${m}/completed`, step: m, at: at + 2 }];
};

it("rev_S144_r1_a_tail_batch_written_long_before_the_click_and_lost_is_a_fault", () => {
  const wire = [...step(2, 900), ...step(3, 1965)];
  const seen = new Set(step(2, 900).map((x) => x.marker));
  const stored = [{ kind: "tool", i: 5, at: 905, body: { toolCallId: "call_2_0", status: "completed" } },
    { kind: "turn", i: 20, at: 3065, body: { state: "idle", stop: { kind: "interrupted" } } }];
  const coverage = openaiPageCoverage(wire, seen, stored, 0, 2965);
  expect(coverage.cut).toEqual([]);
  expect(coverage.faults.join(" ")).toContain("missing page marker oa-s1-3");
});
