// @vitest-environment node
import { expect, it } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { openaiCadence, p95, writerBusy } from "./metrics.mjs";

it("charges only commit time in the common window", () => {
  expect(writerBusy([[13, 15], [5, 12], [11, 14], [19, 22]], 10, 20)).toBe(60);
  expect(() => writerBusy([], 10, 20)).toThrow();
});
it("rejects both a burst and a missed step", () => {
  expect(openaiCadence([100, 1100, 2100, 3100, 4100, 5100], 0, 6000)).toBe(true);
  expect(openaiCadence([100, 101, 2100, 3100, 4100, 5100], 0, 6000)).toBe(false);
  expect(openaiCadence([100, 1100, 3100, 4100, 5100], 0, 6000)).toBe(false);
});
it("uses the nearest-rank p95 and requires samples", () => {
  expect(p95(Array.from({length: 20}, (_, i) => 20 - i))).toBe(19);
  expect(() => p95([])).toThrow();
});
