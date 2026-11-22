// S32-fix2 AC2: the built page on a `<local>.localhost` origin (host 8.1) without window.watane shows the cause, never the fake (18.2).
// Slow: only the built page shows main.tsx's choice on a real host name; Chromium resolves *.localhost to loopback.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi(""); // /watane.js answers but defines nothing, as a page whose script did not load
const browser = await launch();
let code = 0;
try {
  const page = await browser.newPage();
  await page.goto(url.replace("127.0.0.1", "claude.localhost"));
  // the frame of index.html shows first (S35 AC4c); main.tsx then replaces it with the cause
  await page.waitForFunction(() => document.getElementById("root").textContent === "watane.js did not load", null, { timeout: 15000 });
  const text = await page.evaluate(() => document.getElementById("root").textContent);
  assert.equal(text, "watane.js did not load");
  assert.ok(!(await page.evaluate(() => document.body.textContent)).includes("Fix the retry bug"), "the scripted story is not shown");
  console.log("ok served_S32_no_watane");
} catch (e) { console.error("FAIL served_S32_no_watane:", e.message); code = 1; }
await browser.close();
await close();
process.exit(code);
