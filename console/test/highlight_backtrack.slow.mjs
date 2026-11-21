// S23-2 AC5 (TECHNICAL 18.6): a code block whose grammar backtracks catastrophically stays plain, the page stays responsive, and
// the block after it is highlighted. Slow: only a real browser has a Worker and a main thread whose long tasks can be measured.
// The grammar: Shiki's bash (shellscript) on a line of "<" (about 4.5 s in node at 2000 characters, and it grows faster than the length).
// The page is test/harness_highlight.html on vite's dev server, running the real kit/parts/highlight.ts and its Worker.
// Run: node test/highlight_backtrack.slow.mjs (exit 0 = holds, 1 = fails, 2 = not run).
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { launch } from "./slow-kit.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const { createServer } = await import("vite");
const server = await createServer({ root, configFile: false, logLevel: "error", server: { port: 0, host: "127.0.0.1" } });
await server.listen();
const base = `http://127.0.0.1:${server.httpServer.address().port}`;

const browser = await launch();
let code = 0;
try {
  const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${base}/test/harness_highlight.html`);
  await page.waitForFunction(() => window.ready === true);
  // a normal block first, so Shiki and the language are loaded and its time is not part of the check
  await page.evaluate(() => window.h.highlight("bash", "echo warm"));

  const r = await page.evaluate(async () => {
    let longest = 0, last = performance.now(), stop = false;
    const tick = () => { const n = performance.now(); longest = Math.max(longest, n - last); last = n; if (!stop) requestAnimationFrame(tick); };
    requestAnimationFrame(tick);
    const tasks = [];
    new PerformanceObserver((l) => tasks.push(...l.getEntries().map((e) => e.duration))).observe({ entryTypes: ["longtask"] });
    const bad = "<".repeat(2500), t0 = performance.now();
    const a = await window.h.highlight("bash", bad);
    const took = performance.now() - t0;
    const b = await window.h.highlight("bash", "echo after");
    stop = true;
    await new Promise((res) => setTimeout(res, 100));
    return { a, took, b: !!b && b.length === 1 && b[0].some((t) => t.s), longest, maxTask: Math.max(0, ...tasks), again: await window.h.highlight("bash", bad) };
  });
  try {
    assert.equal(r.a, null, "the backtracking block stays plain");
    assert.equal(r.again, null, "and is not retried");
    assert.ok(r.took < 1500, `it came back within about one budget, not after the backtracking (${Math.round(r.took)} ms)`);
    assert.ok(r.b, "a normal block after it is highlighted");
    assert.ok(r.maxTask <= 100, `no main-thread task over 100 ms (${Math.round(r.maxTask)} ms)`);
    assert.ok(r.longest <= 200, `animation frames kept coming (longest gap ${Math.round(r.longest)} ms)`);
    assert.deepEqual(errors, []);
    console.log("ok highlight_backtrack", JSON.stringify({ took: Math.round(r.took), maxTask: Math.round(r.maxTask), longest: Math.round(r.longest) }));
  } catch (e) { console.error("FAIL highlight_backtrack:", e.message); code = 1; }
} finally {
  await browser.close();
  await server.close();
}
process.exit(code);
