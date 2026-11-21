// Adversary, slice S9 rerun 5 (TECHNICAL 14 "An ask shows the tool call that will run", AGT-F-42 "its full input"):
// while asking, the diff and the Input sit in a box capped at 30vh with the ask bar outside it. Input comes after the
// diff, so any diff taller than the cap pushes the input (replace_all, a file_path that differs from the header, a
// command under an edit card) below the box's fold. Chromium shows no scrollbar width and nothing else says the box
// scrolls; the participant sees the header, part of the diff and "Allow once", already focused by Needs you.
// The fake story's own ask (14 diff lines) already does it at 1440x900, 1280x800 and 760x900.
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
const fails = [];
try {
  for (const [width, height] of [[1440, 900], [1280, 800], [760, 900]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(url);
    await page.getByRole("button", { name: "Needs you (1)" }).click();
    await page.waitForTimeout(800);
    const m = await page.evaluate(() => {
      const box = document.querySelector(".wa-ask-scroll").getBoundingClientRect();
      const input = document.querySelector(".wa-ask-scroll .wa-input").getBoundingClientRect();
      return { inputShown: input.top >= box.top - 0.5 && input.bottom <= box.bottom + 0.5 };
    });
    if (!m.inputShown) fails.push(`${width}x${height}: the ask's Input is below the fold of the capped box`);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
assert.deepEqual(fails, [], "adv_S9_ask_scroll_hides_input: the call's input is not in view when the participant can allow");
console.log("ok adv_S9_ask_scroll_hides_input");
