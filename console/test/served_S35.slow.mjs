// S35-4 AC2, AC3, AC4c: the built page on a fake `window.watane` that behaves like the kernel with the fake claude: the new-session view, Start,
// the stream (Writing, then the entry), Working then Idle, a reload that shows the same transcript once, the draft kept, no blank page between.
// Slow: only Chromium shows the paint order of the first frame, the layout of the new-session view, and the page's script order with watane.js.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

const J = JSON.stringify;
const meta = (m) => J({ _meta: { watane: m } });
const fake = `(() => {
  const hs = [];
  const get = () => { try { return JSON.parse(localStorage.getItem("fake") || "null"); } catch { return null; } };
  const T = Date.now();
  // the real shape (9.4, S35-1): stored entries user i=1, turn running i=2, agent i=3, turn idle i=4; live, the agent message is a delta, then the update
  // that finishes it with i and the WHOLE text, then a turn item
  const ent = (i, kind, body) => JSON.stringify({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind, at: i, body } } } });
  const entries = [
    ent(1, "message", { role: "user", content: [{ type: "text", text: "hi" }] }),
    ent(2, "turn", { state: "running", since: T }),
    ent(3, "message", { role: "agent", content: [{ type: "text", text: "This is the fake claude." }] }),
    ent(4, "turn", { state: "idle", stop: { kind: "completed" } }),
  ];
  const chunk = (meta) => JSON.stringify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "This is the fake claude." }, _meta: { watane: meta } });
  const push = (stream, bodies, seq) => setTimeout(() => hs.forEach((h) => h({ type: "stream", stream, items: bodies.map((body, k) => ({ seq: seq + k, body })) })), 0);
  const turn = (state) => ${J(meta({ type: "turn" }))}.replace("}}}", ",\\"state\\":" + JSON.stringify(state) + "}}}");
  window.watane = {
    on(h) { hs.push(h); return () => {}; },
    async call(fn, args) {
      if (fn === "sessions.list") return { ok: get() ? [{ id: "n1", title: "hi", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }] : [] };
      if (fn === "sessions.start") {
        if (args.profile !== "scratch" || args.overlay !== "{}") return { err: { backend: "wrong start" } };
        localStorage.setItem("fake", "1"); localStorage.setItem("input", args.input);
        return { ok: "n1" };
      }
      if (fn === "sessions.attach") {
        const live = localStorage.getItem("input") !== null && !localStorage.getItem("done");
        const queue = ${J(meta({ type: "queue", inputs: [] }))};
        if (live) {
          // the kernel's attach right after sessions.start: the stored entries, synced, the turn item, the queue
          localStorage.setItem("done", "1");
          const st = "st" + Math.random();
          push(st, [entries[0], entries[1], ${J(meta({ type: "synced", through: 2 }))}, turn({ state: "running", since: T }), queue], 1);
          // the test steps the turn once it has seen each word (a timer races a loaded machine: 20 browsers run side by side)
          const steps = [() => push(st, [chunk({ at: T + 1 })], 6), () => push(st, [chunk({ at: T + 2, i: 3 }), turn({ state: "idle", stop: { kind: "completed" } })], 7)];
          window.__step = () => steps.shift()?.();
          return { ok: st };
        }
        const st = "st" + Math.random();
        push(st, [...entries, ${J(meta({ type: "synced", through: 4 }))}, turn({ state: "idle", stop: { kind: "completed" } }), queue], 1);
        return { ok: st };
      }
      if (fn === "sessions.read") return { ok: { entries: [], last: 4 } };
      // S40 r1 F1.1: the view lists the kernel's profiles only
      if (fn === "sessions.profiles") return [{ name: "scratch", backend: "claude", model: null }];
      if (fn === "sessions.check") return [];
      return { ok: null };
    },
  };
})();`;

buildOnce();
const { url, close } = await serveUi(fake);
const browser = await launch();
let code = 0;
try {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await ctx.newPage();
  // no frame with an empty #root once something has painted (AC4c)
  await page.addInitScript(() => {
    window.__blank = 0; window.__painted = false;
    new PerformanceObserver(() => { window.__painted = true; }).observe({ type: "paint", buffered: true });
    const tick = () => { if (window.__painted) { const r = document.getElementById("root"); if (!r || r.children.length === 0) window.__blank++; } requestAnimationFrame(tick); };
    requestAnimationFrame(tick);
  });
  const text = () => page.evaluate(() => document.body.innerText);
  const word = () => page.evaluate(() => document.querySelector(".wa-status-word")?.textContent);

  await page.goto(url);
  await page.getByText("A new scratch folder").waitFor({ timeout: 15000 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, "the new-session view fits the window");
  const box = page.getByRole("textbox", { name: "Message" });
  assert.equal(await box.getAttribute("placeholder"), "Message Claude…");

  // the draft is kept across a reload until sent (AC4b)
  await box.fill("half a thought");
  await page.reload();
  await page.getByRole("textbox", { name: "Message" }).waitFor();
  assert.equal(await page.getByRole("textbox", { name: "Message" }).inputValue(), "half a thought");

  // Start (AC2) and the stream (AC3)
  await page.getByRole("textbox", { name: "Message" }).fill("hi");
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => document.querySelector(".wa-title")?.textContent === "hi", null, { timeout: 15000 });
  const t0 = await text();
  assert.ok(t0.includes("Claude Code (unchecked)") && t0.includes("Scratch folder"), "header facts");
  assert.ok(!/\(1\d\.\d\)/.test(t0), "no section number is shown");
  await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Working", null, { timeout: 5000 });
  await page.evaluate(() => window.__step());
  await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Writing", null, { timeout: 5000 });
  await page.evaluate(() => window.__step());
  await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Idle", null, { timeout: 8000 });
  const once = async () => {
    const t = await text();
    assert.equal(t.split("This is the fake claude.").length, 2, "the agent's message once");
    assert.equal(t.split("\nhi\n").length >= 2, true, "her message shows");
  };
  await once();

  // reload: the same transcript, nothing doubled, no blank frame (AC4c)
  await page.evaluate(() => { window.__blank = 0; });
  await page.reload();
  await page.getByText("This is the fake claude.").waitFor({ timeout: 15000 });
  await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Idle", null, { timeout: 8000 });
  await once();
  assert.equal(await word(), "Idle");
  assert.equal(await page.evaluate(() => window.__blank), 0, "no frame with an empty #root after the first paint");
  console.log("ok served_S35");
} catch (e) { console.error("FAIL served_S35:", e.message); code = 1; }
await browser.close();
await close();
process.exit(code);
