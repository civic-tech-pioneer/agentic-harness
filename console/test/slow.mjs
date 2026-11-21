// Runs every test/*.slow.mjs against one build of ui/. Exit 0 all pass, 1 any failed, 2 none failed but some not run.
import { spawn } from "node:child_process";
import { readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { buildOnce } from "./slow-kit.mjs";

const dir = fileURLToPath(new URL(".", import.meta.url));
buildOnce();
const all = readdirSync(dir).filter((f) => f.endsWith(".slow.mjs")).sort();
// Timing scripts measure p95s, so they run alone after the parallel ones (S30-3): the others' load would skew them.
const ALONE = ["long_timing.slow.mjs"];
const scripts = all.filter((f) => !ALONE.includes(f));
const env = { ...process.env, WA_UI_BUILT: "1" };
// The scripts only read the one build, so they run side by side; their output is printed per script, in order.
const run = (s) => new Promise((done) => {
  const t = Date.now();
  const p = spawn(process.execPath, [dir + s], { env });
  let out = "";
  p.stdout.on("data", (d) => (out += d));
  p.stderr.on("data", (d) => (out += d));
  p.on("close", (code) => done({ s, out, code: code ?? 1, secs: ((Date.now() - t) / 1000).toFixed(1) }));
});
const results = await Promise.all(scripts.map(run));
for (const s of ALONE) if (all.includes(s)) results.push(await run(s));
let worst = 0;
for (const { s, out, code, secs } of results) {
  process.stdout.write(out);
  console.log(`${code === 0 ? "PASS" : code === 2 ? "NOT RUN" : "FAIL"} ${s} ${secs}s`);
  if (code !== 0 && code !== 2) worst = 1;
  else if (code === 2 && worst === 0) worst = 2;
}
process.exit(worst);
