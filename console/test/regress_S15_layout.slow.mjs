// S15-3 (18.6): after "Needs you" the pill shows no focus ring and Allow once does; at 760 px the ask bar ends with Allow once.
// Slow: jsdom computes no layout or :focus-visible, so this builds ui/, serves it, and measures it in Chromium.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();

const browser = await launch();
try {
  for (const [width, height] of [[1440, 900], [1280, 800], [760, 900]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(url);
    await page.mouse.move(width - 30, 300);
    const pill = page.getByRole("button", { name: "Needs you (1)" });
    await pill.click();
    await page.waitForTimeout(800);
    const m = await page.evaluate(() => {
      const pill = document.querySelector(".wa-needs");
      const f = document.activeElement;
      const bar = document.querySelector(".wa-askbar");
      const r = (e) => e.getBoundingClientRect();
      const btns = [...bar.querySelectorAll("button")].map((b) => ({ t: b.textContent, ...r(b).toJSON() }));
      const q = bar.querySelector(".askq");
      const ps = pill ? getComputedStyle(pill) : null;
      return {
        pillFocusVisible: pill ? pill.matches(":focus-visible") : false,
        pillOutline: ps ? ps.outlineStyle + " " + ps.outlineWidth + " " + ps.boxShadow : "",
        focus: f.textContent,
        ring: getComputedStyle(f).outlineStyle,
        focusVisible: f.matches(":focus-visible"),
        bar: r(bar).toJSON(), btns, q: r(q).toJSON(), sw: document.documentElement.scrollWidth, iw: innerWidth,
      };
    });
    assert.equal(m.focus, "Allow once", `${width}: focus on Allow once`);
    assert.equal(m.pillFocusVisible, false, `${width}: the pill has no focus-visible ring`);
    assert.ok(/^none|^auto 0px/.test(m.pillOutline) || m.pillOutline.startsWith("none"), `${width}: pill draws no outline (${m.pillOutline})`);
    assert.notEqual(m.ring, "none", `${width}: Allow once outline drawn`);
    // Allow once is the right-most button of the last line, nothing cut or overlapping.
    const last = m.btns[m.btns.length - 1];
    assert.equal(last.t, "Allow once");
    const lastLine = m.btns.filter((b) => Math.abs(b.top - last.top) < 4);
    for (const b of lastLine) assert.ok(b.right <= last.right + 0.5, `${width}: ${b.t} is not right of Allow once`);
    for (const b of m.btns) {
      assert.ok(b.left >= m.bar.left - 0.5 && b.right <= m.bar.right + 0.5, `${width}: ${b.t} inside the bar`);
    }
    for (let i = 0; i < m.btns.length; i++) for (let j = i + 1; j < m.btns.length; j++) {
      const a = m.btns[i], b = m.btns[j];
      assert.ok(a.right <= b.left + 0.5 || b.right <= a.left + 0.5 || a.bottom <= b.top + 0.5 || b.bottom <= a.top + 0.5, `${width}: ${a.t} overlaps ${b.t}`);
    }
    assert.ok(last.right >= m.bar.right - 20, `${width}: Allow once at the bar's right end`);
    if (width >= 1280) assert.ok(m.bar.height < 70 && m.btns.every((b) => Math.abs(b.top - last.top) < 4), `${width}: the bar is one row`);
    assert.ok(m.sw <= m.iw, `${width}: no horizontal scroll`);
    // The pill itself, focused after a pointer click, draws no ring; reached by Tab, it does (:focus-visible).
    // S23-1 AC8: Needs you hides while its card holds focus; focus goes to the composer first, which shows it again (the assertions below are unchanged)
    await page.getByRole("textbox", { name: "Message" }).focus();
    await pill.evaluate((e) => e.focus());
    const mouse = await pill.evaluate((e) => [e.matches(":focus"), e.matches(":focus-visible"), getComputedStyle(e).outlineStyle, getComputedStyle(e).boxShadow]);
    assert.deepEqual(mouse, [true, false, "none", "none"], `${width}: pill focused after a click draws no ring`);
    // (S23-1: away from the pill and back by keys inside the dock; Shift+Tab from the pill would reach Allow once, whose card hides the pill)
    await page.keyboard.press("Tab");
    await page.keyboard.press("Shift+Tab");
    const key = await pill.evaluate((e) => [e.matches(":focus-visible"), getComputedStyle(e).outlineStyle]);
    assert.deepEqual(key, [true, "solid"], `${width}: pill reached by Tab shows its ring`);
    console.log(`ok ${width}x${height}`, JSON.stringify(m.btns.map((b) => [b.t, Math.round(b.left), Math.round(b.top)])));
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
