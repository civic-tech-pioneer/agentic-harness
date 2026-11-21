// S23-1 AC6 (RISK a, 18.6 Place): a pointer press on the transcript's scrollbar, or on text in an open ask's card that cannot take focus,
// drops focus to the page body; it must not resume the follow: the stream goes on arriving and the transcript stays where the reader is.
// Slow: jsdom has no scrollbar and no layout, so this builds ui/, serves it, and presses the real scrollbar and text in Chromium.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();
const browser = await launch();
const fails = [];
try {
  for (const where of ["scrollbar", "text"]) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.goto(`${url}?long=13`);
    await page.getByRole("button", { name: "Needs you (1)" }).click({ timeout: 30_000 });
    assert.equal(await page.evaluate(() => document.activeElement.textContent), "Allow once");
    await page.waitForFunction(() => { const s = document.querySelector(".wa-scroll"); return s.scrollHeight - s.clientHeight - s.scrollTop >= 20; });
    if (where === "scrollbar") {
      const r = await page.evaluate(() => { const b = document.querySelector(".wa-scroll").getBoundingClientRect(); return { x: b.right - 4, y: b.bottom - 6 }; });
      await page.mouse.move(r.x, r.y);
      await page.mouse.down();
      await page.waitForTimeout(1500); // the held press on the track pages the transcript down to the end
      await page.mouse.up();
    } else {
      await page.getByText("Allow this edit?").click();
    }
    // the reader scrolls to the end after the press (that is what used to resume the follow, with focus on the body)
    const c = await page.evaluate(() => { const b = document.querySelector(".wa-scroll").getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; });
    await page.mouse.move(c.x, c.y);
    await page.mouse.wheel(0, 100000);
    await page.waitForTimeout(300);
    const body = await page.evaluate(() => document.activeElement === document.body);
    const at = await page.evaluate(() => { const s = document.querySelector(".wa-scroll"); return { top: s.scrollTop, h: s.scrollHeight }; });
    await page.waitForTimeout(2500);
    const m = await page.evaluate(() => {
      const s = document.querySelector(".wa-scroll");
      return { top: s.scrollTop, h: s.scrollHeight, pill: !!document.querySelector(".wa-pill") };
    });
    console.log(where, JSON.stringify({ body, at, m }));
    if (m.h <= at.h) fails.push(`${where}: the stream did not grow the transcript (${JSON.stringify({ at, m })}): nothing was tested`);
    else if (Math.abs(m.top - at.top) > 1) fails.push(`${where}: the transcript followed the stream after the press (${JSON.stringify({ at, m })})`);
    else if (!m.pill) fails.push(`${where}: "New activity below" is not shown`);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, [], "adv_S23_press_in_ask_card_keeps_follow_off");
console.log("ok adv_S23_press_in_ask_card_keeps_follow_off");
