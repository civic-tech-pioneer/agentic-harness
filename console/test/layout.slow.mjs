// Slow: jsdom computes no layout, so this builds ui/, serves it, and measures it in Chromium at two widths.
// Playwright is not a dependency: it is resolved from the machine's global install (NODE_PATH).
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();

const browser = await launch();
try {
  for (const [width, height] of [[1280, 800], [700, 800]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    const errors = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    await page.goto(url);
    await page.getByRole("button", { name: "Needs you (1)" }).waitFor();
    const m = await page.evaluate(() => {
      const box = (s) => { const e = document.querySelector(s); return e ? { w: e.getBoundingClientRect().width, display: getComputedStyle(e).display } : null; };
      const d = document.documentElement;
      return { sidebar: box(".wa-sidebar"), column: box(".wa-column"), scrollW: d.scrollWidth, clientW: d.clientWidth, bodyScrollW: document.body.scrollWidth };
    });
    assert.deepEqual(errors, [], `page errors at ${width}`);
    if (width === 1280) {
      assert.equal(m.sidebar.w, 260, "sidebar is 260 px wide at 1280");
      assert.ok(m.column.w <= 780, `transcript column at most 780 px, got ${m.column.w}`);
    } else {
      assert.equal(m.sidebar.display, "none", "sidebar is not displayed at 700");
    }
    assert.ok(m.scrollW <= m.clientW && m.bodyScrollW <= m.clientW, `no horizontal scroll at ${width}: ${JSON.stringify(m)}`);
    console.log(`ok ${width} x ${height}`, JSON.stringify(m));
    await page.close();
  }
  // S9-fix6: after Needs you, the asking card's header and its ask bar are both inside the transcript area.
  for (const [width, height] of [[1440, 900], [1280, 800], [760, 900]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(url);
    await page.getByRole("button", { name: "Needs you (1)" }).click();
    await page.waitForTimeout(800);
    const m = await page.evaluate(() => {
      const area = document.querySelector(".wa-scroll").getBoundingClientRect();
      const card = document.querySelector('[data-asking="true"]');
      const inside = (s) => { const b = card.querySelector(s).getBoundingClientRect(); return b.top >= area.top - 0.5 && b.bottom <= area.bottom + 0.5; };
      return { head: inside(".wa-tool-head"), bar: inside(".wa-askbar") };
    });
    assert.deepEqual(m, { head: true, bar: true }, `asking card header and ask bar in view at ${width}x${height}`);
    console.log(`ok needs you ${width} x ${height}`);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
