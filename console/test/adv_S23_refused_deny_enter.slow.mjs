// Adversary S23 pass 8: in the page, a busy refusal of a Deny with a reason moves focus to Allow once while the deny row
// stays open with the reason; the participant's Enter to send the deny again allows the call.
import assert from "node:assert/strict";
import { buildOnce, serveUi, launch } from "./slow-kit.mjs";

buildOnce();
const { url, close } = await serveUi();
const browser = await launch();
let got;
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(url + "?refuse=busy");
  await page.getByRole("button", { name: "Allow once" }).first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Deny…" }).first().click();
  await page.keyboard.type("not this one");
  await page.keyboard.press("Enter");
  await page.getByRole("alert").first().waitFor({ timeout: 5_000 });
  got = await page.evaluate(() => ({
    focus: document.activeElement?.textContent ?? "",
    reason: document.querySelector(".wa-deny input")?.value ?? null,
  }));
  await page.close();
} finally {
  await browser.close();
  await close();
}
// the deny row still shows the reason; Enter must not land on Allow once
assert.notEqual(got.reason, null, "the deny row is open");
assert.notEqual(got.focus, "Allow once", `adv_S23_refused_deny_enter: focus on ${got.focus} with the deny row open (reason ${got.reason})`);
console.log("ok adv_S23_refused_deny_enter");
