// S7-2 AC8 (P14): the console on a session of 5 000 entries with a stream running (`?long=5000`), in Chromium on the built ui/.
// Slow: jsdom computes no layout, so only a real browser shows what the window does with real row heights: the end on
// open, at most 120 rows in the DOM while scrolling over a scroll height that is mostly estimates, the first row
// reachable at the top, and Needs you landing on a card whose row was outside the window with Allow once focused and
// its ask bar inside the transcript area (as regress_S9_needs_you_focus_visible.slow.mjs measures).
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();

const browser = await launch();
const errors = [];
let code = 0;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  page.on("console", (m) => { if (m.type() === "error") errors.push(`console.error: ${m.text()}`); });
  await page.goto(`${url}?long=5000`);

  const rows = () => page.evaluate(() => document.querySelectorAll(".wa-row").length);
  const gap = () => page.evaluate(() => { const s = document.querySelector(".wa-scroll"); return s.scrollHeight - s.clientHeight - s.scrollTop; });

  // the end shows on open: the newest page first
  await page.getByText("Done with module 5000", { exact: false }).waitFor({ timeout: 15_000 });
  await page.waitForTimeout(300);
  assert.ok((await gap()) <= 40, `on open the end shows (${await gap()} px above it)`);
  assert.ok((await rows()) <= 120, `at most 120 rows on open (${await rows()})`);

  // the rest of the history pages in (4 calls a second) and the end still shows
  await page.getByRole("button", { name: "Needs you (1)" }).waitFor({ timeout: 30_000 });
  await page.waitForTimeout(500);
  assert.ok((await gap()) <= 40, `after the whole history is in, the end still shows (${await gap()} px above it)`);
  assert.ok((await rows()) <= 120, `at most 120 rows once all is in (${await rows()})`);

  // Needs you: the ask is entry 7, thousands of rows above; its card is brought into view with Allow once focused
  await page.mouse.move(1250, 300);
  await page.getByRole("button", { name: "Needs you (1)" }).click();
  await page.waitForTimeout(800);
  const m = await page.evaluate(() => {
    const f = document.activeElement;
    const b = f.getBoundingClientRect();
    const s = document.querySelector(".wa-scroll").getBoundingClientRect();
    const hit = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
    const bar = f.closest("[data-asking]")?.querySelector(".wa-askbar")?.getBoundingClientRect();
    return { focus: f.textContent, visible: hit === f || f.contains(hit), barInside: !!bar && bar.top >= s.top - 1 && bar.bottom <= s.bottom + 1,
      rows: document.querySelectorAll(".wa-row").length };
  });
  assert.equal(m.focus, "Allow once", "Needs you focuses Allow once");
  assert.ok(m.visible, `the focused Allow once is on screen (${JSON.stringify(m)})`);
  assert.ok(m.barInside, `the ask bar is inside the transcript area (${JSON.stringify(m)})`);
  assert.ok(m.rows <= 120, `at most 120 rows after Needs you (${m.rows})`);

  // scroll from here to the end and back to the top in steps, counting the rows in the DOM
  let most = 0;
  const jump = async (to) => {
    await page.evaluate((t) => { const s = document.querySelector(".wa-scroll"); s.scrollTop = t === "end" ? s.scrollHeight : t; }, to);
    await page.waitForTimeout(60);
    most = Math.max(most, await rows());
  };
  const height = await page.evaluate(() => document.querySelector(".wa-scroll").scrollHeight);
  for (let y = 0; y <= height; y += Math.round(height / 25)) await jump(y);
  await jump("end");
  // in the middle, rows above the reader are measured as they come in: the row at the top of the view must stay put (within 1 row)
  const anchor = () => page.evaluate(() => {
    const s = document.querySelector(".wa-scroll"), t = s.getBoundingClientRect().top;
    const r = [...document.querySelectorAll(".wa-row")].find((e) => e.getBoundingClientRect().bottom > t + 1);
    return { index: Number(r.dataset.index), rel: r.getBoundingClientRect().top - t, h: r.getBoundingClientRect().height };
  });
  await jump(Math.round(height / 2));
  const a0 = await anchor();
  await page.mouse.move(640, 300);
  for (let n = 0; n < 6; n++) { await page.mouse.wheel(0, -500); await page.waitForTimeout(120); }
  await page.waitForTimeout(500);
  const a1 = await anchor();
  assert.ok(a1.index < a0.index, `wheeling up moves up the rows (${a0.index} -> ${a1.index})`);
  await jump(Math.round(height / 2));
  await page.waitForTimeout(100);
  const a2 = await anchor();
  await page.waitForTimeout(700);
  const a3 = await anchor();
  assert.ok(Math.abs(a3.rel - a2.rel) <= Math.max(a3.h, 80) && a3.index === a2.index, `the row at the top stays put while heights come in (${JSON.stringify([a2, a3])})`);
  for (let y = height; y >= 0; y -= Math.round(height / 25)) await jump(y);
  await jump(0);
  await page.waitForTimeout(400);
  assert.ok(most <= 120, `at most 120 rows while scrolling (${most})`);
  const top = await page.evaluate(() => {
    const first = document.querySelector('.wa-row[data-index="0"]');
    const s = document.querySelector(".wa-scroll");
    const r = first?.getBoundingClientRect();
    return { text: first?.textContent ?? null, scrollTop: s.scrollTop, shown: !!r && r.bottom > s.getBoundingClientRect().top && r.top < s.getBoundingClientRect().bottom };
  });
  assert.ok(top.text?.startsWith("Step 1:") && top.shown && top.scrollTop <= 20, `scrolling to the top shows the first row (${JSON.stringify(top)})`);
  console.log("ok long_session", JSON.stringify({ mostRows: most, scrollHeight: height }));
} catch (e) { console.error("FAIL long_session:", e.message); code = 1; }
finally {
  await browser.close();
  await close();
}
if (errors.length) { console.error("FAIL long_session: page errors:", errors); code = 1; }
process.exit(code);
