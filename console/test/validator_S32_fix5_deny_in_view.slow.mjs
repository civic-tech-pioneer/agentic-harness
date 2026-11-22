// S32-fix5 (18.6 Asks): in the page, with the ask bar at the foot of the transcript, Deny… leaves the whole reason form in view:
// inside the scroller's visible part, above the dock (and its Needs you), clear of "New activity below". Slow: real layout and
// the transcript's own anchor hold, which a jsdom test does not have (the anchor undid the reveal before this check).
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();
const browser = await launch();
const fails = [];
try {
  for (const [q, w, h] of [["", 1440, 900], ["?long=13", 760, 700], ["?long=13", 1440, 600]]) {
    const page = await browser.newPage({ viewport: { width: w, height: h } });
    await page.goto(url + q);
    await page.getByRole("button", { name: "Allow once" }).first().waitFor({ timeout: 30_000 });
    await page.waitForTimeout(300);
    await page.evaluate(() => { const s = document.querySelector(".wa-scroll"), b = document.querySelector(".wa-askbar"); s.scrollTop += b.getBoundingClientRect().bottom - s.getBoundingClientRect().bottom + 2; });
    await page.waitForTimeout(100);
    await page.getByRole("button", { name: "Deny…" }).first().click();
    await page.waitForTimeout(300);
    const m = await page.evaluate(() => {
      const r = (s) => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return { top: b.top, bottom: b.bottom }; };
      return { focus: document.activeElement?.tagName, scroll: r(".wa-scroll"), form: r(".wa-deny"), dock: r(".wa-dock"), pill: r(".wa-pill") };
    });
    const ok = m.focus === "INPUT" && m.form && m.form.top >= m.scroll.top - 0.5 && m.form.bottom <= Math.min(m.scroll.bottom, m.dock.top) + 0.5
      && (!m.pill || m.form.bottom <= m.pill.top + 0.5 || m.form.top >= m.pill.bottom - 0.5);
    if (!ok) fails.push(`${q || "default"} ${w}x${h}: the reason form is not fully in view ${JSON.stringify(m)}`);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, []);
console.log("ok validator_S32_fix5_deny_in_view");
