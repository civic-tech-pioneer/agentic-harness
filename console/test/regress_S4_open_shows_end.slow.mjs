// Red (S4 acceptance): TECHNICAL 18.3 "so the end of the transcript shows at once"; story: Alice sees the open
// ask with its bar. Opening the played story at 1280x800 leaves the transcript scrolled to the top, so the ask
// bar (Allow once) is below the fold until she scrolls or presses Needs you. Slow: jsdom computes no scroll.
// Run: node test/regress_S4_open_shows_end.slow.mjs (exit 0 = holds, 1 = fails, 2 = not run).
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const require = createRequire(import.meta.url);
let playwright;
try { playwright = require("playwright"); } catch {
  console.error("NOT RUN: `playwright` cannot be required here.");
  process.exit(2);
}

const root = fileURLToPath(new URL("..", import.meta.url));
const ui = join(root, "..", "ui");
execSync("npm run build", { cwd: root, stdio: "ignore" });

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2" };
const server = createServer(async (req, res) => {
  const p = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
  try {
    const f = p.endsWith("/") ? p + "index.html" : p;
    res.writeHead(200, { "content-type": types[extname(f)] ?? "application/octet-stream" }).end(await readFile(join(ui, f)));
  } catch { res.writeHead(404).end(); }
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const url = `http://127.0.0.1:${server.address().port}/`;

const browser = await playwright.chromium.launch();
let code = 0;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(url);
  await page.getByRole("button", { name: "Needs you (1)" }).waitFor();
  await page.waitForTimeout(500);
  const m = await page.evaluate(() => {
    const s = document.querySelector(".wa-scroll");
    const allow = [...document.querySelectorAll("button")].find((b) => b.textContent === "Allow once");
    const sr = s.getBoundingClientRect(), ar = allow.getBoundingClientRect();
    return { gap: s.scrollHeight - s.clientHeight - s.scrollTop, allowVisible: ar.top >= sr.top && ar.bottom <= sr.bottom };
  });
  try {
    assert.ok(m.gap <= 40, `on open the transcript is within 40 px of its end; it is ${m.gap} px above it`);
    assert.ok(m.allowVisible, "on open the open ask's Allow once is in view");
    console.log("ok regress_S4_open_shows_end", JSON.stringify(m));
  } catch (e) { console.error("FAIL regress_S4_open_shows_end:", e.message); code = 1; }
} finally {
  await browser.close();
  server.close();
}
process.exit(code);
