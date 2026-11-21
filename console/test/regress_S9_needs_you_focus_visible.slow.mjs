// S9 acceptance (18.6): "Needs you" puts focus on Allow once, and the person must be able to see the button that has
// focus. When the ask card is taller than the transcript (1280x800, 760x900), it now scrolls the card's header into
// view and leaves the focused Allow once clipped below the transcript, under the dock.
// Slow: jsdom computes no layout, so this builds ui/, serves it, and measures it in Chromium.
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
  console.error("NOT RUN: `playwright` cannot be required here (install it globally, or set NODE_PATH to it).");
  process.exit(2);
}

const root = fileURLToPath(new URL("..", import.meta.url));
const ui = join(root, "..", "ui");
execSync("npm run build", { cwd: root, stdio: "inherit" });

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2" };
const server = createServer(async (req, res) => {
  const p = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
  try {
    const f = p.endsWith("/") ? p + "index.html" : p;
    const body = await readFile(join(ui, f));
    res.writeHead(200, { "content-type": types[extname(f)] ?? "application/octet-stream" }).end(body);
  } catch { res.writeHead(404).end(); }
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const url = `http://127.0.0.1:${server.address().port}/`;

const browser = await playwright.chromium.launch();
const failures = [];
try {
  for (const [width, height] of [[1440, 900], [1280, 800], [760, 900]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(url);
    await page.mouse.move(width - 30, 300);
    await page.getByRole("button", { name: "Needs you (1)" }).click();
    await page.waitForTimeout(800);
    const m = await page.evaluate(() => {
      const f = document.activeElement;
      const b = f.getBoundingClientRect();
      const hit = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
      return { focus: f.textContent, top: b.top, bottom: b.bottom, visible: hit === f || f.contains(hit) };
    });
    assert.equal(m.focus, "Allow once", `focus on Allow once at ${width}x${height}`);
    if (!m.visible) failures.push(`${width}x${height}: focused Allow once is not visible (${JSON.stringify(m)})`);
    else console.log(`ok ${width}x${height}`);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
assert.deepEqual(failures, [], "after Needs you, the focused Allow once is on screen");
