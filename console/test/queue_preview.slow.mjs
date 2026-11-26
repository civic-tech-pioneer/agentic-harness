// A real browser is needed to verify that an overflowing preview is visibly clamped.
import assert from "node:assert/strict";
import { buildOnce, launch, serveUi } from "./slow-kit.mjs";

function fixture() {
  const listeners = new Set();
  window.watane = {
    on(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    async call(fn) {
      if (fn === "sessions.list") return { ok: [{ id: "s1", title: "Notes", backend: "claude", profile: "scratch" }] };
      if (fn === "sessions.attach") {
        const events = [
          { type: "synced", through: 0 },
          { type: "turn", state: { state: "running", since: Date.now() } },
          { type: "queue", inputs: [{ n: 1, input: "x".repeat(255 * 1024) }] },
        ];
        const items = events.map((watane, i) => ({ seq: i + 1, body: JSON.stringify({ _meta: { watane } }) }));
        for (const listener of listeners) listener({ type: "stream", stream: "st1", items });
        return { ok: "st1" };
      }
      return { ok: null };
    },
  };
}

buildOnce();
const { url, close } = await serveUi(`(${fixture.toString()})()`);
const browser = await launch();
try {
  const page = await browser.newPage({ viewport: { width: 700, height: 800 } });
  await page.goto(url);
  const preview = page.locator(".wa-queue-text");
  await preview.waitFor();
  await page.evaluate(() => document.fonts.ready);
  const bounds = await preview.evaluate((node) => ({
    height: node.getBoundingClientRect().height,
    scrollHeight: node.scrollHeight,
    lineHeight: parseFloat(getComputedStyle(node).lineHeight),
    length: node.textContent.length,
  }));
  assert.equal(bounds.length, 241);
  assert.ok(bounds.height <= 3 * bounds.lineHeight + 1 && bounds.scrollHeight > bounds.height,
    `the preview is longer than three lines but displays at most three: ${JSON.stringify(bounds)}`);
} finally {
  await browser.close();
  await close();
}
