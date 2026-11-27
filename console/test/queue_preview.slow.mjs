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
      if (fn === "sessions.interrupt") return { err: { backend: "boom" } };
      return { ok: null };
    },
  };
}

buildOnce();
const { url, close } = await serveUi(`(${fixture.toString()})()`);
const browser = await launch();
try {
  const page = await browser.newPage({ viewport: { width: 390, height: 800 } });
  await page.goto(url);
  const preview = page.locator(".wa-queue-text");
  await preview.waitFor();
  await page.evaluate(() => document.fonts.ready);
  for (const width of [390, 700]) {
    await page.setViewportSize({ width, height: 800 });
    const bounds = await preview.evaluate((node) => ({
      height: node.getBoundingClientRect().height,
      scrollWidth: node.scrollWidth,
      clientWidth: node.clientWidth,
      ellipsis: getComputedStyle(node).textOverflow,
      lineHeight: parseFloat(getComputedStyle(node).lineHeight) || 1.5 * parseFloat(getComputedStyle(node).fontSize),
      length: node.textContent.length,
    }));
    assert.equal(bounds.length, 241);
    // S156: one line cut with an ellipsis, as Main-2's `.qitem` (was: at most three lines)
    assert.ok(bounds.height <= bounds.lineHeight + 1 && bounds.scrollWidth > bounds.clientWidth && bounds.ellipsis === "ellipsis",
      `at ${width}px the overflowing preview displays one line with an ellipsis: ${JSON.stringify(bounds)}`);
  }
  await page.setViewportSize({ width: 390, height: 800 });
  await page.getByRole("button", { name: "Stop and send queued message 1" }).click();
  const taken = page.getByRole("list", { name: "Taken from the queue" });
  const error = taken.getByRole("alert").filter({ hasText: "The kernel could not do that: boom" });
  await error.waitFor({ state: "visible" });
  assert.equal(await page.locator(".wa-composer").getByRole("alert").count(), 0);
  const errorBounds = await error.evaluate((node) => ({
    width: node.getBoundingClientRect().width,
    rowWidth: node.closest("li")?.getBoundingClientRect().width,
  }));
  assert.ok(errorBounds.width >= errorBounds.rowWidth - 1,
    `the error has its own full-width line: ${JSON.stringify(errorBounds)}`);
} finally {
  await browser.close();
  await close();
}
