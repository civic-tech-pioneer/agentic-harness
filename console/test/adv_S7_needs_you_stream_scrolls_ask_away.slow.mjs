// S7 adversary, TECHNICAL 14 "An ask shows the tool call that will run" (18.6 Asks, Place): Needs you scrolls to the open ask
// and focuses Allow once. When that card sits within 40 px of the end, the transcript keeps following the end, so the agent's
// stream (here the fast stream of `?long=13`, which runs while the ask is open) carries the card, its input and then the focused
// Allow once out of the transcript area within seconds. Focus stays on Allow once: Enter or Space allows a call the participant
// no longer sees, with only the agent's new text in view. adv_S9 and regress_S9 measure only the instant after Needs you.
// Slow: jsdom computes no layout, so this builds ui/, serves it, and measures it in Chromium.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();

const browser = await launch();
const fails = [];
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(`${url}?long=13`);
  await page.mouse.move(1410, 300);
  await page.getByRole("button", { name: "Needs you (1)" }).click({ timeout: 30_000 });
  for (let t = 0; t < 16; t++) {
    await page.waitForTimeout(250);
    const m = await page.evaluate(() => {
      const f = document.activeElement;
      const s = document.querySelector(".wa-scroll").getBoundingClientRect();
      const b = f.getBoundingClientRect();
      const hit = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
      const input = f.closest("[data-asking]")?.querySelector(".wa-input")?.getBoundingClientRect();
      return { focus: f.textContent, buttonOnScreen: hit === f || f.contains(hit),
        inputInView: !!input && input.bottom > s.top && input.top < s.bottom };
    });
    if (m.focus !== "Allow once") break; // focus moved away: Enter no longer allows
    if (!m.buttonOnScreen || !m.inputInView) { fails.push(`after ${(t + 1) * 250} ms: ${JSON.stringify(m)}`); break; }
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, [], "adv_S7_needs_you_stream_scrolls_ask_away: Allow once keeps focus while the stream scrolls its call out of view");
console.log("ok adv_S7_needs_you_stream_scrolls_ask_away");
