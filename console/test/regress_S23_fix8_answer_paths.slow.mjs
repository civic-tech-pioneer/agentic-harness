// S23-fix8 AC3: every way to answer an ask, a busy refusal, and Needs you onto a second ask at the transcript's end: no page error,
// the root stays mounted, at 1440 and 760 px. Slow: jsdom has no layout, so the browser's clamp of scrollTop never happens there.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();
const browser = await launch();
const fails = [];
const cases = [
  ["Enter on Allow once", "", async (p) => { await p.getByRole("button", { name: "Allow once" }).focus(); await p.keyboard.press("Enter"); }],
  ["Allow this session", "", (p) => p.getByRole("button", { name: /Allow .* this session/ }).click()],
  ["Deny with a reason", "", async (p) => {
    await p.getByRole("button", { name: "Deny…" }).click();
    await p.getByRole("textbox", { name: "Tell the agent why" }).fill("not now");
    await p.getByRole("button", { name: "Deny", exact: true }).click();
  }],
  ["busy refusal", "?refuse=busy", async (p) => { await p.getByRole("button", { name: "Allow once" }).click(); }],
  ["Needs you to a second ask", "?second=1", async (p) => {
    await p.getByRole("button", { name: "Allow once" }).first().click();
    await p.waitForTimeout(300);
    await p.getByRole("button", { name: /^Needs you \(/ }).click();
  }],
];
try {
  for (const width of [1440, 760]) {
    for (const [name, q, act] of cases) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await page.goto(url + q);
      await page.getByRole("button", { name: "Allow once" }).first().waitFor({ timeout: 30_000 });
      await act(page);
      await page.waitForTimeout(600);
      const mounted = await page.evaluate(() => document.getElementById("root").children.length > 0);
      if (!mounted || errors.length) fails.push(`${width} ${name}: mounted=${mounted} ${errors[0]?.slice(0, 80) ?? ""}`);
      await page.close();
    }
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, [], "regress_S23_fix8_answer_paths");
console.log("ok regress_S23_fix8_answer_paths");
