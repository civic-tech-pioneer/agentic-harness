// Red (S7 user-sim): TECHNICAL 18.4/18.6, the page is a fixed frame (sidebar, header, transcript, composer); only the
// transcript scrolls. On the long session (`?long=300`) the visually hidden "Added:"/"Removed:" labels of the diffs
// (`.wa-sr`, position absolute, no positioned ancestor inside the scroller) sit at their place deep in the transcript
// and make the document itself as tall as the transcript (472 352 px at 5 000 entries). Then Needs you scrolls the
// whole page up by the header's height (the header and Profile chips go off-screen, a blank band under the composer),
// and PageDown with focus on the page scrolls the console away to a blank screen. Slow: jsdom computes no layout.
// Run: node test/regress_S7_page_never_scrolls.slow.mjs (exit 0 = holds, 1 = fails, 2 = not run).
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();

const browser = await launch();
let code = 0;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(url + "?long=300");
  const needs = page.getByRole("button", { name: "Needs you (1)" });
  await needs.waitFor({ timeout: 15000 });
  await page.waitForTimeout(300);
  const doc = () => page.evaluate(() => ({ top: document.scrollingElement.scrollTop, height: document.scrollingElement.scrollHeight, inner: innerHeight }));
  const open = await doc();
  await needs.click();
  await page.waitForTimeout(600);
  const afterNeeds = await doc();
  const active = await page.evaluate(() => document.activeElement?.textContent?.trim());
  await page.mouse.click(1250, 400); // blank margin right of the column: focus goes to the page
  await page.keyboard.press("PageDown");
  await page.waitForTimeout(300);
  const afterPageDown = await doc();
  const m = { open, afterNeeds, active, afterPageDown };
  try {
    assert.ok(open.height <= open.inner, `the document is not taller than the window on open (only the transcript scrolls); it is ${open.height} px for ${open.inner}`);
    assert.equal(active, "Allow once", "Needs you focuses Allow once");
    assert.equal(afterNeeds.top, 0, `Needs you leaves the page itself unscrolled (header in view); the page scrolled by ${afterNeeds.top} px`);
    assert.equal(afterPageDown.top, 0, `PageDown on the page does not scroll the console away; the page scrolled by ${afterPageDown.top} px`);
    console.log("ok regress_S7_page_never_scrolls", JSON.stringify(m));
  } catch (e) { console.error("FAIL regress_S7_page_never_scrolls:", e.message, JSON.stringify(m)); code = 1; }
} finally {
  await browser.close();
  await close();
}
process.exit(code);
