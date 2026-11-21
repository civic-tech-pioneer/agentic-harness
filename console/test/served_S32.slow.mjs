// S32-3: the built page, served with a /watane.js that defines a fake window.watane, opens that kernel's session and not the scripted story.
// Slow: only the built page shows that its script order (watane.js before the bundle) and main.tsx's served branch work together.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

const entry = (i) => JSON.stringify({ v: 1, i, kind: "message", at: 1000 * i, body: { role: "agent", content: [{ type: "text", text: `Item ${i} of 3` }] } });
const fake = `(() => {
  const hs = [];
  const entries = ${JSON.stringify([1, 2, 3].map(entry))};
  window.watane = {
    on(h) { hs.push(h); return () => {}; },
    async call(fn, args) {
      if (fn === "sessions.list") return { ok: [] };
      if (fn === "probe.start") return { ok: "p1" };
      if (fn === "sessions.attach") {
        setTimeout(() => hs.forEach((h) => h({ type: "stream", stream: "st1", items: entries.map((body, k) => ({ seq: k + 1, body })).concat([{ seq: 4, body: JSON.stringify({ _meta: { watane: { type: "synced", through: 0 } } }) }]) })), 100);
        return { ok: "st1" };
      }
      if (fn === "sessions.read") return { ok: { entries, last: 3 } };
      return { ok: null };
    },
  };
})();`;

buildOnce();
const { url, close } = await serveUi(fake);
const browser = await launch();
let code = 0;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(url);
  await page.waitForFunction(() => document.body.textContent.includes("Item 3 of 3"), null, { timeout: 15000 });
  const text = await page.evaluate(() => document.body.textContent);
  const at = [1, 2, 3].map((n) => text.indexOf(`Item ${n} of 3`));
  assert.ok(at.every((x) => x >= 0) && at[0] < at[1] && at[1] < at[2], `items out of order: ${at}`);
  assert.equal(text.split("Item 1 of 3").length, 2, "Item 1 shown once");
  assert.ok(!text.includes("Fix the retry bug"), "the scripted story is not shown");
  console.log("ok served_S32");
} catch (e) { console.error("FAIL served_S32:", e.message); code = 1; }
await browser.close();
await close();
process.exit(code);
