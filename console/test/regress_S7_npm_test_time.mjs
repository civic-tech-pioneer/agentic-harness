// Red (S7 user-sim): S7 outcome 1 and brief S7-1 AC5, `npm test` wall time is not above the 8.6 s baseline of
// main 1b5d324 (280 tests, 21 files). On slice/S7 it is 13.6-15.4 s on an idle 4-core machine (main: 8.8-9.2 s
// measured the same way, A/B): src/long_S7.test.tsx alone takes 5.9-8.6 s inside the default run.
// Not a *.slow.mjs: the slow runner runs its scripts side by side, which would load the machine and skew the time.
// Run by hand on an idle machine: node test/regress_S7_npm_test_time.mjs (exit 0 = holds, 1 = fails).
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const BUDGET_S = 8.6;
const root = fileURLToPath(new URL("..", import.meta.url));
const t = Date.now();
const r = spawnSync(process.execPath, [root + "node_modules/vitest/vitest.mjs", "run"], { cwd: root, encoding: "utf8" });
const secs = (Date.now() - t) / 1000;
const tests = /Tests\s+(.*)/.exec(r.stdout ?? "")?.[1]?.trim();
if (r.status !== 0) { console.error(`FAIL regress_S7_npm_test_time: npm test failed (exit ${r.status})`); process.exit(1); }
if (secs > BUDGET_S) { console.error(`FAIL regress_S7_npm_test_time: npm test took ${secs.toFixed(1)} s, over the ${BUDGET_S} s budget (${tests})`); process.exit(1); }
console.log(`ok regress_S7_npm_test_time ${secs.toFixed(1)} s (${tests})`);
