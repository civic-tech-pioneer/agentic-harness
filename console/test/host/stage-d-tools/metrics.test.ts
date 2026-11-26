// @vitest-environment node
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
// @ts-expect-error: a plain ES module of the host runner
import { openaiCadence, p95, writerBusy } from "./metrics.mjs";

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
