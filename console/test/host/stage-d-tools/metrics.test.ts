// @vitest-environment node
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, it, vi } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { clickStop, openaiCadence, openaiPageCoverage, p95, pageItem, pageProbe, writerBusy } from "./metrics.mjs";

it("charges only commit time in the common window", () => {
  expect(writerBusy([[13, 15], [5, 12], [11, 14], [19, 22]], 10, 20)).toBe(60);
  expect(() => writerBusy([], 10, 20)).toThrow();
});
it("counts a nested interval once and clips one that starts before the window", () => {
  expect(writerBusy([[0, 10], [2, 3]], 0, 20)).toBe(50);
  expect(writerBusy([[5, 15], [6, 8]], 10, 20)).toBe(50);
});
it("rejects both a burst and a missed step", () => {
  expect(openaiCadence([100, 1100, 2100, 3100, 4100, 5100], 0, 6000)).toBe(true);
  expect(openaiCadence([100, 101, 2100, 3100, 4100, 5100], 0, 6000)).toBe(false);
  expect(openaiCadence([100, 1100, 3100, 4100, 5100], 0, 6000)).toBe(false);
});
it("starts a session's window at its second step, so only a late first step is forgiven", () => {
  // the first request waited for the describe retry and landed 300 ms before the next beat
  expect(openaiCadence([1700, 2000, 3000, 4000, 5000, 6000, 7000], 1000, 7500)).toBe(true);
  // a late later step still fails
  expect(openaiCadence([1700, 2000, 3000, 3300, 5000, 6000, 7000], 1000, 7500)).toBe(false);
  expect(openaiCadence([2000], 1000, 7500)).toBe(false);
});
it("the stamp wrapper's rewrite keeps the JSON valid when the padding holds escapes", () => {
  const tools = fileURLToPath(new URL(".", import.meta.url));
  const pad = 'ab"cd\\efégh\n'.repeat(6);
  const line = JSON.stringify({ session_id: "fake-session", message: { content: [{ text: `entry 7 ${pad}` }] } }, null, 0)
    .replace("é", "\\u00e9"); // one \u escape too, as a JSON writer may emit it
  const out = execFileSync("python3", ["-I", "-B", "-c", [
    "import sys",
    `sys.path.insert(0, ${JSON.stringify(tools)})`,
    "from stamp_claude import rewrite",
    "out, marker = rewrite(sys.stdin.buffer.read(), b'0' * 32)",
    "sys.stdout.buffer.write(marker + b'\\n' + out)",
  ].join("\n")], { input: line, encoding: "utf8" });
  const [marker, rewritten] = [out.slice(0, out.indexOf("\n")), out.slice(out.indexOf("\n") + 1)];
  expect(marker).toBe(`cl-${"0".repeat(32)}-entry-7`);
  const o = JSON.parse(rewritten); // valid JSON: no escape was cut in half
  expect(o.session_id).toBe("0".repeat(32));
  const text = o.message.content[0].text;
  expect(text.startsWith(`${marker} ab"cd\\ef`)).toBe(true);
  // the entry keeps its encoded length but for the session id (20 bytes longer) and at most one escape (5 bytes)
  const grown = Buffer.byteLength(rewritten) - Buffer.byteLength(line);
  expect(grown).toBeGreaterThan(14);
  expect(grown).toBeLessThanOrEqual(20);
});

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

it("ac1_excludes_the_batch_that_Stop_cut_before_the_sink_even_if_it_was_written_64ms_before_click", () => {
  const admitted = step(2, 2000);
  const tail = step(3, 3000);
  const wire = [...admitted, ...tail];
  const seen = new Set(admitted.map(({ marker }) => marker));
  const stored = [...storedTools(2, 10), interrupted];
  const coverage = openaiPageCoverage(wire, seen, stored, 1000, 3064, 3064);
  expect(coverage.faults).toEqual([]);
  expect(coverage.cut).toEqual(tail);
  expect(coverage.interrupted).toEqual(interrupted);
});

it("ac1_reports_missing_page_markers_for_a_step_recorded_before_the_interrupted_turn", () => {
  const earlier = step(2, 2000);
  const last = step(3, 3000);
  const seen = new Set(earlier.map(({ marker }) => marker));
  const accepted = storedTools(3, 14)[0];
  // A job aborted by Stop is persisted as a failed tool with the output "aborted".
  const aborted = { ...accepted, body: { ...accepted.body, status: "failed",
    content: [{ type: "content", content: { type: "text", text: "aborted" } }] } };
  const stored = [...storedTools(2, 10), aborted, interrupted];
  const coverage = openaiPageCoverage([...earlier, ...last], seen, stored, 1000, 3064, 3064);
  expect(coverage.cut).toEqual([]);
  expect(coverage.faults.join(" ")).toContain("oa-s1-3");
  for (let i = 0; i < 4; i++) expect(coverage.faults.join(" ")).toContain(`call_3_${i}`);
});

it("ac1_does_not_excuse_an_earlier_hole_or_an_unconfirmed_interruption", () => {
  const earlier = step(2, 2000);
  const tail = step(3, 3000);
  const wire = [...earlier, ...tail];
  const coverage = openaiPageCoverage(wire, new Set(), [interrupted], 1000, 3064, 3064);
  expect(coverage.faults.join(" ")).toContain("oa-s1-2");
  expect(coverage.cut).toEqual(tail);
  const noStop = openaiPageCoverage(wire, new Set(), [], 1000, 3064, 3064);
  expect(noStop.cut).toEqual([]);
  expect(noStop.faults.join(" ")).toMatch(/stored interrupted turn/);
  const later = { ...interrupted, at: 5000 };
  const lateStop = openaiPageCoverage(wire, new Set(), [later], 1000, 3064, 5000);
  expect(lateStop.cut).toEqual([]);
  expect(lateStop.faults.join(" ")).toContain("oa-s1-3");
});

it("ac1_rejects_a_completed_batch_with_duplicate_or_missing_source_calls", () => {
  const tail = step(3, 3000);
  const malformed = tail.filter(({ marker }) => marker !== "call_3_2");
  malformed.splice(-1, 0, { ...tail[2] });
  expect(openaiPageCoverage(malformed, new Set(), [interrupted], 1000, 3064, 3064).faults.join(" "))
    .toContain("instead of four distinct calls");
});

it("ac1_requires_a_partial_source_batch_to_overlap_the_stored_Stop", () => {
  const tail = step(3, 3000).slice(0, 3);
  expect(openaiPageCoverage(tail, new Set(), [interrupted], 1000, 3064, 3064).faults.join(" "))
    .toContain("missing page marker oa-s1-3");
  tail[2].at = 3066;
  const coverage = openaiPageCoverage(tail, new Set(), [interrupted], 1000, 3064, 3064);
  expect(coverage.cut).toEqual(tail);
  expect(coverage.faults).toEqual([]);
});

it("ac1_reports_no_Stop_cut_when_all_page_markers_arrived", () => {
  const tail = step(3, 3000);
  const coverage = openaiPageCoverage(tail, new Set(tail.map(({ marker }) => marker)), [interrupted], 1000, 3064, 3064);
  expect(coverage.cut).toEqual([]);
  expect(coverage.faults).toEqual([]);
});

it("ac1_checks_stored_message_admission_even_for_a_batch_near_the_click", () => {
  const tail = step(3, 3000);
  const message = { kind: "message", i: 19, at: 3001,
    body: { role: "agent", content: [{ type: "text", text: "oa-s1-3 committed" }] } };
  const coverage = openaiPageCoverage(tail, new Set(), [message, interrupted], 1000, 3064, 3064);
  expect(coverage.cut).toEqual([]);
  expect(coverage.faults).toContain("missing page marker oa-s1-3");
});

it("ac1_requires_the_click_and_rejects_an_old_missing_marker_in_a_recent_batch", () => {
  const tail = step(3, 2963);
  tail.at(-1)!.at = 3000;
  expect(openaiPageCoverage(tail, new Set(), [interrupted], 1000, 3064, 3064).faults)
    .toContain("missing page marker oa-s1-3");
  expect(openaiPageCoverage(step(3, 3000), new Set(), [interrupted], 1000, 3064).faults)
    .toContain("missing page marker oa-s1-3");
});

it("ac3_requires_a_complete_tail_to_be_close_to_the_stored_interrupt_as_well_as_the_click", () => {
  const tail = step(3, 3000);
  const late = { ...interrupted, at: 5100 };
  const coverage = openaiPageCoverage(tail, new Set(), [late], 1000, 3064, 3064);
  expect(coverage.cut).toEqual([]);
  expect(coverage.faults).toContain("missing page marker oa-s1-3");
});

it("ac3_does_not_count_a_durable_OpenAI_message_as_its_live_delta", () => {
  const text = "oa-s1-3";
  const live = { sessionUpdate: "agent_message_chunk", content: { type: "text", text } };
  const entry = { _meta: { watane: { entry: { kind: "message", body: { role: "agent",
    content: [{ type: "text", text }] } } } } };
  expect(pageItem({ at: 1000, body: JSON.stringify(live) }).marker).toBe(text);
  const indexed = { ...live, _meta: { watane: { i: 19 } } };
  expect(pageItem({ at: 1001, body: JSON.stringify(indexed) }).marker).toBeUndefined();
  expect(pageItem({ at: 1001, body: JSON.stringify(entry) }).marker).toBeUndefined();
  const tail = step(3, 3000);
  const coverage = openaiPageCoverage(tail, new Set(), [
    { ...entry._meta.watane.entry, i: 19, at: 3001 }, interrupted,
  ], 1000, 3064, 3064);
  expect(coverage.faults).toContain(`missing page marker ${text}`);
  expect(coverage.hasStoredMarker).toBe(true);
  expect(openaiPageCoverage(tail, new Set(), [interrupted], 1000, 3064, 3064).hasStoredMarker).toBe(false);
});

it("ac3_stamps_Stop_on_the_wall_clock_at_the_click", () => {
  const clock = vi.spyOn(Date, "now").mockReturnValue(12_345);
  const click = vi.fn();
  vi.stubGlobal("document", { querySelector: () => ({ click }) });
  try {
    expect(clickStop()).toBe(12_345);
    expect(click).toHaveBeenCalledOnce();
  } finally {
    vi.unstubAllGlobals();
    clock.mockRestore();
  }
});

it("ac3_stamps_receipt_on_the_wall_clock_before_dispatch", () => {
  const clock = vi.spyOn(Date, "now").mockReturnValue(12_346);
  const page = { watane: { on: (receive: (event: object) => void) => {
    receive({ type: "stream", stream: "st1", items: [{ body: "one" }] });
  } }, loadItems: [] as unknown[] };
  vi.stubGlobal("window", page);
  try {
    const real = page.watane;
    pageProbe();
    page.watane = real;
    page.watane.on(() => expect(page.loadItems).toEqual([{ at: 12_346, stream: "st1", body: "one" }]));
  } finally {
    vi.unstubAllGlobals();
    clock.mockRestore();
  }
});
