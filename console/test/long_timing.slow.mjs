// S30-3 AC2: the time limits of the long session (S7-2 AC7, TECHNICAL 18.6): input p95 <= 50 ms, stream delta to its text p95 <= 100 ms,
// Stop/Esc p95 <= 100 ms, scroll to a far offset p95 <= 100 ms (20 samples each), the end shown <= 2 s after open, on the 5 000-entry fixture
// with the stream running. They are asserted by src/long_S7.test.tsx when WA_LONG_TIMING=1; `npm test` runs the same file with no limit.
// The file prints the p95s ("S7-2 p95 (ms): ..."). test/slow.mjs runs this script alone, after the others, so their load does not skew it.
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const r = spawnSync(process.execPath, [root + "node_modules/vitest/vitest.mjs", "run", "src/long_S7.test.tsx"], {
  cwd: root, encoding: "utf8", env: { ...process.env, WA_LONG_TIMING: "1" },
});
process.stdout.write((r.stdout ?? "") + (r.stderr ?? ""));
process.exit(r.status === 0 ? 0 : 1);
