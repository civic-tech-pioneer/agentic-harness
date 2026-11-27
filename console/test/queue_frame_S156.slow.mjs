// S156-2: the queue and what it took, as Main-2 draws them, at 390 px and at desktop width. Needs a real browser: the row heights, the
// one-line text and the two-line error are layout, which jsdom does not do. TECHNICAL 18.6 "Queue"; frame Main-2 `.queue`, `.qitem`, `.qundo`.
import assert from "node:assert/strict";
import { buildOnce, launch, serveUi } from "./slow-kit.mjs";

function fixture() {
  const listeners = new Set();
  let inputs = [
    { n: 1, input: "Also merge to-read.md into it, keeping my comments" },
    { n: 2, input: "Then sort the list by author" },
    { n: 3, input: "And tell me which books are new" },
  ];
  const push = (watane) => { for (const l of listeners) l({ type: "stream", stream: "st1", items: [{ seq: ++seq, body: JSON.stringify({ _meta: { watane } }) }] }); };
  let seq = 0;
  window.watane = {
    on(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    async call(fn, args) {
      if (fn === "sessions.list") return { ok: [{ id: "s1", title: "Notes", backend: "claude", profile: "scratch" }] };
      if (fn === "sessions.attach") {
        setTimeout(() => { push({ type: "synced", through: 0 }); push({ type: "turn", state: { state: "running", since: Date.now() } }); push({ type: "queue", inputs }); }, 0);
        return { ok: "st1" };
      }
      if (fn === "sessions.unqueue") { inputs = inputs.filter((q) => q.n !== args.n); setTimeout(() => push({ type: "queue", inputs }), 0); return { ok: null }; }
      if (fn === "sessions.interrupt") return { err: { backend: "the provider stream is not open yet, so this message could not be sent now; try again once the session has started and Claude has answered" } };
      return { ok: null };
    },
  };
}

const px = (s) => parseFloat(s);
buildOnce();
const { url, close } = await serveUi(`(${fixture.toString()})()`);
const browser = await launch();
try {
  for (const width of [390, 1440]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    await page.goto(url);
    const rows = page.locator(".wa-queue li");
    await rows.first().waitFor();
    await page.evaluate(() => document.fonts.ready);
    assert.equal(await page.locator(".wa-queue-label").textContent(), "Queued · 3 of 32 · sent when this turn ends");
    // AC1: each row is one line of text; at 1440 the controls sit beside it, at 390 under it
    for (const box of await rows.evaluateAll((lis) => lis.map((li) => {
      const t = li.querySelector(".wa-queue-text"), cs = getComputedStyle(t);
      return { row: li.getBoundingClientRect().height, text: t.getBoundingClientRect().height, line: parseFloat(cs.lineHeight) || 1.5 * parseFloat(cs.fontSize), dashed: getComputedStyle(li).borderTopStyle };
    }))) {
      assert.equal(box.dashed, "dashed");
      assert.ok(box.text <= box.line + 1, `at ${width}px a row's text is one line: ${JSON.stringify(box)}`);
      const most = width >= 1000 ? 36 + 8 + 2 + 1 : box.line + 36 + 8 + 2 + 1;
      assert.ok(box.row <= most, `at ${width}px a row is at most ${most}px tall: ${JSON.stringify(box)}`);
    }
    // AC2: quiet 500-weight buttons, a muted × named "Remove from queue"
    const edit = page.getByRole("button", { name: "Edit queued message 1" });
    const x = page.getByRole("button", { name: "Remove from queue" }).first();
    const [e, r] = await Promise.all([edit, x].map((l) => l.evaluate((b) => { const s = getComputedStyle(b); return { weight: s.fontWeight, bg: s.backgroundColor, color: s.color, ink: getComputedStyle(document.body).color }; })));
    assert.equal(e.weight, "500");
    assert.equal(e.bg, "rgba(0, 0, 0, 0)");
    assert.notEqual(r.color, e.color, "the × is muted, not ink");
    // AC5: "Enter a message first." only after an empty send
    const composer = page.locator(".wa-composer");
    assert.equal(await composer.getByText("Enter a message first.").count(), 0);
    await page.getByRole("textbox", { name: "Message" }).press("Enter");
    await composer.getByText("Enter a message first.").waitFor();
    // AC3: a refused Stop and send: the error shows once, at most two lines, the item at most four lines tall at 390
    await page.getByRole("button", { name: "Stop and send queued message 1" }).click();
    const taken = page.getByRole("list", { name: "Taken from the queue" });
    await taken.getByRole("alert").waitFor();
    assert.equal(await composer.getByRole("alert").count(), 0, "the error is not repeated above the composer");
    await page.getByRole("textbox", { name: "Message" }).fill("");
    await taken.getByRole("button", { name: "Stop and send" }).waitFor();
    const t = await taken.locator("li").first().evaluate((li) => {
      const err = li.querySelector(".wa-taken-error"), text = li.querySelector(".wa-taken-text"), cs = getComputedStyle(err);
      const line = parseFloat(cs.lineHeight) || 1.5 * parseFloat(cs.fontSize);
      return { li: li.getBoundingClientRect().height, err: err.getBoundingClientRect().height, text: text.getBoundingClientRect().height, line };
    });
    assert.ok(t.err <= 2 * t.line + 1, `at ${width}px the error is at most two lines: ${JSON.stringify(t)}`);
    assert.ok(t.text <= t.line + 1, `at ${width}px the taken text is one line: ${JSON.stringify(t)}`);
    assert.ok(t.li <= 3 * t.line + 36 + 1, `at ${width}px the taken item is at most two error lines, a text line and a button line: ${JSON.stringify(t)}`);
    // AC5: a Removed line offers Undo and no Discard
    await page.getByRole("button", { name: "Remove from queue" }).first().click();
    const removed = taken.locator("li").filter({ hasText: "Removed" });
    await removed.waitFor();
    assert.deepEqual(await removed.getByRole("button").allTextContents(), ["Undo"]);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
