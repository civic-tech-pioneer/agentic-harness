// S144 review r1 adversary: the Stop-cut excuse of openaiPageCoverage counts only tool rows as proof that the kernel admitted
// the tail step. The kernel stores an assistant message durably at its `output_item.done` (kernel/src/openai/run.rs, the
// `Sse::Item` arm), before the step's calls or `response.completed`. A tail step whose message entry is stored before the
// interrupted turn, but never reached the page, is lost data the check must report, not a batch Stop cut.
import { expect, it } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { openaiPageCoverage } from "./metrics.mjs";

const step = (n: number, at: number) => {
  const marker = `oa-s1-${n}`;
  return [
    { marker, step: marker, at },
    ...Array.from({ length: 4 }, (_, j) => ({ marker: `call_${n}_${j}`, step: marker, at: at + 1 })),
    { marker: `${marker}/completed`, step: marker, at: at + 2 },
  ];
};
const storedTools = (n: number, after: number) => Array.from({ length: 4 }, (_, j) => ({
  kind: "tool", i: after + j + 1, at: 1000 * n + 3,
  body: { toolCallId: `call_${n}_${j}`, status: "completed" },
}));
const interrupted = { kind: "turn", i: 20, at: 3065, body: { state: "idle", stop: { kind: "interrupted" } } };

it("adv_S144_stored_message_of_the_tail_step_missing_from_the_page_is_a_fault", () => {
  const earlier = step(2, 2000);
  const tail = step(3, 3000);
  const seen = new Set(earlier.map(({ marker }) => marker));
  // the kernel stored the tail step's message before the interrupted turn: the step was admitted
  const message = { kind: "message", i: 15, at: 3000, body: { role: "agent", content: [{ type: "text", text: "oa-s1-3 step three" }] } };
  const coverage = openaiPageCoverage([...earlier, ...tail], seen, [...storedTools(2, 10), message, interrupted], 1000, 3064);
  expect(coverage.cut).toEqual([]);
  expect(coverage.faults.join(" ")).toContain("missing page marker oa-s1-3");
});
