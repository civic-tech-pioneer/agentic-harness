// S23 outcome 1 (acceptance): Alice answers the story's one open ask and the card must say "Answered, waiting".
// Instead the page goes blank: the card shrinks (its bar becomes one line), the transcript holds the card's row still
// by asking for a scrollTop the shorter content cannot reach (154 when the most is 116 at 1440x900), and every commit
// asks again: React's "Maximum update depth exceeded" unmounts the whole App. Every answer path does it (Allow once,
// Enter, Allow ... this session, Deny), and so does Needs you onto a second ask at the transcript's end.
// Slow: jsdom has no layout, so the browser's clamp of scrollTop never happens there.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();
const browser = await launch();
const fails = [];
try {
  for (const width of [1440, 760]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(url);
    await page.getByRole("button", { name: "Allow once" }).click({ timeout: 30_000 });
    await page.waitForTimeout(500);
    const r = await page.evaluate(() => ({
      mounted: document.getElementById("root").children.length > 0,
      answered: [...document.querySelectorAll("[role=status]")].some((e) => e.textContent === "Answered, waiting"),
    }));
    console.log(width, JSON.stringify(r), errors.map((e) => e.slice(0, 60)));
    if (!r.mounted) fails.push(`${width}: the page went blank after Allow once (${errors[0]?.slice(0, 60)})`);
    else if (!r.answered) fails.push(`${width}: no "Answered, waiting" after Allow once`);
    await page.close();
  }
} finally {
  await browser.close();
  await close();
}
assert.deepEqual(fails, [], "regress_S23_answer_blanks_page");
console.log("ok regress_S23_answer_blanks_page");
