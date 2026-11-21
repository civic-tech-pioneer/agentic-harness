// S23-1 AC9, AC10 (18.6 Place): while "New activity below" shows, the transcript's last row is not under it; a click on it
// puts focus on the newest row and the view at the end. Slow: needs real layout, so this builds ui/ and measures it in Chromium.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();
const browser = await launch();
const fails = [];
try {
  for (const [width, height] of [[1440, 900], [760, 900]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(`${url}?long=13`);
    await page.getByRole("button", { name: "Needs you (1)" }).click({ timeout: 30_000 });
    await page.waitForSelector(".wa-pill", { timeout: 30_000 });
    // to the very end of the scroller, read in the same task (the stream goes on adding rows below)
    const m = await page.evaluate(() => {
      const s = document.querySelector(".wa-scroll");
      s.scrollTop = s.scrollHeight;
      const rows = s.querySelectorAll(".wa-transcript > .wa-row"), last = rows[rows.length - 1].getBoundingClientRect();
      const pill = document.querySelector(".wa-pill")?.getBoundingClientRect();
      return { lastBottom: last.bottom, pillTop: pill?.top, pill: !!pill };
    });
    if (!m.pill) fails.push(`${width}: the pill went away at the end of the scroller`);
    else if (m.lastBottom > m.pillTop + 0.5) fails.push(`${width}: the last row (bottom ${m.lastBottom}) is under the pill (top ${m.pillTop})`);
    else console.log(`ok ${width} no cover`, JSON.stringify(m));
    await page.locator(".wa-pill").click();
    await page.waitForTimeout(300);
    const f = await page.evaluate(() => {
      const s = document.querySelector(".wa-scroll"), a = document.activeElement;
      const rows = s.querySelectorAll(".wa-transcript > .wa-row");
      return { newest: a === rows[rows.length - 1], tab: a.getAttribute("tabindex"), body: a === document.body, gap: s.scrollHeight - s.clientHeight - s.scrollTop };
    });
    if (!f.newest || f.tab !== "-1") fails.push(`${width}: after the pill focus is not on the newest row (${JSON.stringify(f)})`);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, [], "S23_pill_does_not_cover");
console.log("ok S23_pill_does_not_cover");
