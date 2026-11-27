// S156: screenshots of the views this slice changes, at 390 px and at desktop width, for the compare against the frames
// Main-2 and States-2. Not a test: `node test/screens_S156.mjs [out dir]` (default factory/runs/S156/screens).
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildOnce, launch, serveUi } from "./slow-kit.mjs";

const out = process.argv[2] ?? fileURLToPath(new URL("../../factory/runs/S156/screens", import.meta.url));
mkdirSync(out, { recursive: true });

/** A page-side fake `window.watane`: one session whose attach replays `events`; `interrupt` is refused. */
function fixture(events) {
  const listeners = new Set();
  let seq = 0, inputs = events.find((e) => e.type === "queue")?.inputs ?? [];
  const push = (watane) => { for (const l of listeners) l({ type: "stream", stream: "st1", items: [{ seq: ++seq, body: JSON.stringify({ _meta: { watane } }) }] }); };
  window.watane = {
    on(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    async call(fn, args) {
      if (fn === "sessions.list") return { ok: [{ id: "s1", title: "Tidy the reading notes", backend: "claude", profile: "notes" }] };
      if (fn === "sessions.attach") {
        const items = events.map((watane) => ({ seq: ++seq, body: JSON.stringify(watane.sessionUpdate ? watane : { _meta: { watane } }) }));
        setTimeout(() => { for (const listener of listeners) listener({ type: "stream", stream: "st1", items }); }, 0);
        return { ok: "st1" };
      }
      if (fn === "sessions.unqueue") { inputs = inputs.filter((q) => q.n !== args.n); setTimeout(() => push({ type: "queue", inputs }), 0); return { ok: null }; }
      if (fn === "sessions.interrupt") return { err: { quota: "The session's quota is used up for today." } };
      return { ok: null };
    },
  };
}

const now = Date.now();
const entry = (i, kind, body) => ({ type: "entry", entry: { v: 1, i, kind, at: now, body } });
const user = (i, text) => entry(i, "message", { role: "user", content: [{ type: "text", text }] });
const agent = (i, text) => entry(i, "message", { role: "agent", content: [{ type: "text", text }] });
const running = { type: "turn", state: { state: "running", since: now - 12_000 } };
const queue = (...xs) => ({ type: "queue", inputs: xs.map((input, k) => ({ n: k + 1, input })) });
const QUEUED = ["Also merge to-read.md into it, keeping my comments", "x".repeat(255 * 1024), "Then sort the list by author", "And tell me which books are new"];

const editCall = (id, path) => ({ toolCallId: id, title: `Edit ${path}`, kind: "edit", status: "pending", content: [], locations: [{ path }] });
const ask = (i, id, path) => entry(i, "ask", { id: `a${id}`, kind: "ask", toolCallId: `t${id}`, input: { file_path: path }, "expires-at": now + 600_000, toolCall: editCall(`t${id}`, path) });
const short = (i, askI, id, decision) => entry(i, "ask", { id: `a${id}`, kind: "ask", "ask-i": askI, decision });

const scenes = {
  // Main-2's dock: the queue under a running turn
  queue: [user(1, "Find my notes about books and tidy them"), agent(2, "Reading the notes now."), { type: "synced", through: 2 }, running, queue(...QUEUED)],
  // S154 r1 P1.2: Stop during a launch: two asks the kernel cancelled, one answer she gave that Claude never got
  stopped: [
    user(1, "Merge the two reading lists"), { type: "turn", state: { state: "running", since: now - 3000 } },
    ask(2, 1, "reading-list.md"), ask(3, 2, "books-2026.md"), ask(4, 3, "to-read.md"),
    short(5, 4, 3, { outcome: "allow", by: "alice" }),
    short(6, 2, 1, { outcome: "deny", by: "kernel", reason: "cancel" }),
    short(7, 3, 2, { outcome: "deny", by: "kernel", reason: "cancel" }),
    entry(8, "turn", { state: "idle", stop: { kind: "interrupted" }, seconds: 3 }),
    { type: "synced", through: 8 },
  ],
  // States-2: a thought streaming
  thinking: [user(1, "Which notes overlap?"), { type: "synced", through: 1 }, running, { sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "The user wants overlaps. " } }],
};

buildOnce();
const browser = await launch();
try {
  for (const [name, events] of Object.entries(scenes)) {
    const { url, close } = await serveUi(`(${fixture.toString()})(${JSON.stringify(events)})`);
    for (const [label, width, height] of [["390", 390, 844], ["desktop", 1440, 900]]) {
      const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 2 });
      await page.goto(url);
      await page.locator(".wa-composer").waitFor();
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(300);
      await page.screenshot({ path: join(out, `${name}-${label}.png`) });
      if (name === "queue") {
        // a refused Stop and send: its text in the composer with the error; cleared, the row carries it
        await page.getByRole("button", { name: "Stop and send queued message 1" }).click();
        await page.waitForTimeout(200);
        await page.screenshot({ path: join(out, `refused-${label}.png`) });
        await page.getByRole("textbox", { name: "Message" }).fill("");
        await page.getByRole("button", { name: "Remove from queue" }).first().click();
        await page.waitForTimeout(200);
        await page.screenshot({ path: join(out, `taken-${label}.png`) });
        // an empty send
        await page.getByRole("textbox", { name: "Message" }).press("Enter");
        await page.waitForTimeout(100);
        await page.screenshot({ path: join(out, `empty-send-${label}.png`) });
      }
      await page.close();
    }
    await close();
  }
} finally {
  await browser.close();
}
console.log(`screens in ${out}`);
