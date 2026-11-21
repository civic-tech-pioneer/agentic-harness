// S7 adversary (rerun on S7-fix2), TECHNICAL 14 "An ask shows the tool call that will run" (18.6 Asks, Place): the fix pauses
// the follow of the end while focus is inside an open ask's card, but any reader scroll that lands within 40 px of the end
// resumes it, with focus still on Allow once. A keyboard reader who presses ArrowDown once (a 40 px scroll, focus stays on the
// button) to read a line more re-arms the follow; the stream of `?long=13` then carries the card, its input and the focused
// Allow once out of view within seconds, and Enter allows a call the participant no longer sees.
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
  const look = () => page.evaluate(() => {
    const f = document.activeElement, sc = document.querySelector(".wa-scroll");
    const s = sc.getBoundingClientRect(), b = f.getBoundingClientRect();
    const hit = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
    const input = f.closest("[data-asking]")?.querySelector(".wa-input")?.getBoundingClientRect();
    return { focus: f.textContent, buttonOnScreen: hit === f || f.contains(hit),
      inputInView: !!input && input.bottom > s.top && input.top < s.bottom, gap: sc.scrollHeight - sc.clientHeight - sc.scrollTop };
  });
  // the follow is paused now: let the stream add a little below, then scroll one arrow step toward it
  await page.waitForFunction(() => { const s = document.querySelector(".wa-scroll"); return s.scrollHeight - s.clientHeight - s.scrollTop >= 20; });
  const before = await look();
  assert.equal(before.focus, "Allow once");
  assert.ok(before.buttonOnScreen && before.inputInView, `the ask is in view before the key: ${JSON.stringify(before)}`);
  await page.keyboard.press("ArrowDown");
  for (let t = 0; t < 16; t++) {
    await page.waitForTimeout(250);
    const m = await look();
    if (m.focus !== "Allow once") break;
    if (!m.buttonOnScreen || !m.inputInView) { fails.push(`after ${(t + 1) * 250} ms: ${JSON.stringify(m)}`); break; }
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, [], "adv_S7_arrow_key_resumes_follow_with_ask_focused: one ArrowDown re-arms the follow and the stream scrolls the focused Allow once out of view");
console.log("ok adv_S7_arrow_key_resumes_follow_with_ask_focused");
