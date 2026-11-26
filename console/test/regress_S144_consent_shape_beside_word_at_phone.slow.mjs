// The shape and the status words must start on the same line at phone width.
import assert from "node:assert/strict";
import { buildOnce, launch, serveUi } from "./slow-kit.mjs";

function fixture() {
  let receive;
  window.watane = {
    on(handler) { receive = handler; return () => {}; },
    async call(fn) {
      if (fn === "sessions.list") return { ok: [{ id: "s1", title: "Notes", backend: "claude", profile: "scratch" }] };
      if (fn === "sessions.attach") {
        const events = [
          { type: "synced", through: 0 },
          { type: "turn", state: { state: "running", since: Date.now(),
            activity: { kind: "consent", what: "workspace:/home/alice/notes/2026/meeting-notes" } } },
        ];
        receive({ type: "stream", stream: "st1", items: events.map((watane, i) => ({
          seq: i + 1, body: JSON.stringify({ _meta: { watane } }),
        })) });
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
  for (const width of [390, 1280]) {
    const page = await browser.newPage({ viewport: { width, height: 800 } });
    await page.goto(url);
    const shape = page.getByRole("img", { name: "Waiting for you" });
    await shape.waitFor();
    await page.evaluate(() => document.fonts.ready);
    const { diamond, words } = await shape.evaluate((node) => {
      const word = node.parentElement.querySelector(".wa-status-word");
      const range = document.createRange();
      range.selectNodeContents(word);
      return { diamond: node.getBoundingClientRect().toJSON(), words: range.getClientRects()[0].toJSON() };
    });
    assert.ok(diamond.right <= words.left && diamond.top < words.bottom && diamond.bottom > words.top,
      `at ${width}px the consent diamond should sit beside the first line: ${JSON.stringify({ diamond, words })}`);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
