// Slow: the checked provider, fake Claude, scripted judge and built console on a real Watane host.
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";
import { NotRun } from "./outcome.mjs";

const consoleDir = fileURLToPath(new URL("../..", import.meta.url));
const root = join(consoleDir, "..");
const temp = mkdtempSync(join(tmpdir(), "stage-e-"));
const agent = join(temp, "agent");
const provider = join(temp, "claude-code-checked");
const script = join(temp, "judge.json");
const judge = "https://judge.test";
const env = {
  ...process.env,
  WATANE_HOME: join(temp, "home"),
  WATANE_DEV_CACHE: join(temp, "cache"),
  JUDGE_ORIGIN: judge,
  AGENT_KERNEL_IDENTITY: "local/agent",
};
const run = (bin, args, cwd) => execFileSync(bin, args, { cwd, env, stdio: ["ignore", "pipe", "inherit"], encoding: "utf8", timeout: 600_000 });
function dev(...args) {
  const r = spawnSync("watane-dev", [...args, "--json", "--dir", agent], { env, encoding: "utf8", timeout: 600_000 });
  if (r.error) throw r.error;
  assert.equal(r.status, 0, `watane-dev ${args[0]}: ${r.stdout || r.stderr}`);
  return JSON.parse(r.stdout).result;
}
function stage(from, to) {
  cpSync(from, to, {
    recursive: true,
    filter: (path) => !path.split("/").some((part) => part === "target" || part === "logic.wasm"),
  });
}
function component(cwd, name, destination, args = []) {
  run("cargo", ["build", "--locked", "--release", "--target", "wasm32-wasip2", ...args], cwd);
  const { target_directory } = JSON.parse(run("cargo", ["metadata", "--no-deps", "--format-version", "1"], cwd));
  cpSync(join(target_directory, "wasm32-wasip2", "release", name), join(destination, "logic.wasm"));
}

// Dev plays a raw System One response because the checked provider reads scores, not verdict names.
function score(unsafe, ms = 0) {
  const body = JSON.stringify({ answers: { unsafe: { noul: unsafe }, off_task: { noul: 0.1 } } });
  const raw = { raw: { status: 200, headers: [["content-type", "application/json"]], body: [...Buffer.from(body)] } };
  return ms ? { slow: { ms, then: raw } } : raw;
}

let browser;
let started = false;
let preview;
try {
  for (const bin of ["watane-dev", "cargo"]) {
    if (spawnSync(bin, ["--version"]).error?.code === "ENOENT") throw new NotRun(`${bin} is missing from PATH`);
  }
  browser = await launch().catch((e) => { throw new NotRun(`Chromium cannot start: ${e.message.split("\n")[0]}`); });
  const vendor = join(process.env.WATANE_DEV_CACHE ?? join(process.env.HOME ?? "", ".cache", "watane-dev"), "vendor");
  if (!env.WATANE_DEV_VENDOR && existsSync(vendor)) env.WATANE_DEV_VENDOR = vendor;
  stage(join(root, "kernel", "wasm"), agent);
  stage(join(root, "provider-checked"), provider);
  const manifest = join(provider, "mod.toml");
  writeFileSync(manifest, readFileSync(manifest, "utf8").replace('"api.typesafe.ai"', '"judge.test"'));
  component(join(root, "kernel"), "agent_mod.wasm", agent, ["-p", "agent-mod"]);
  component(join(provider, "logic"), "claude_code.wasm", provider);
  // Dev builds source with a clean environment. Serve the fixture component already built for judge.test.
  rmSync(join(provider, "logic"), { recursive: true });
  run("npm", ["run", "build"], consoleDir);
  cpSync(join(root, "ui"), join(agent, "ui"), { recursive: true });
  dev("add", provider);
  writeFileSync(script, JSON.stringify({ plays: [score(0.1), score(0.1, 700), score(0.9, 700), score(0.5, 700), score(0.8, 700)] }));
  preview = dev("serve", "--script", script);
  started = true;
  assert.equal(preview.judge_origin, judge);
  const url = preview.pages.find(({ slug, local }) => slug === "agent" || local === "agent")?.url;
  assert.ok(url, "the preview did not serve the agent page");
  const fake = join(dirname(preview.home), "claude", "fake-claude.json");
  if (!existsSync(fake)) throw new NotRun("the preview could not find fake-claude");
  const check = (id) => ({ do: "check", tool: "Read", tool_use_id: `toolu_${id}`, input: { file_path: join(temp, `${id}.txt`) }, await: true });
  const betweenChecks = { do: "sleep", ms: 250 };
  const readyMarker = "Starting the fixture tool checks.";
  const safeMarker = "Claude continued after the safe check.";
  writeFileSync(fake, JSON.stringify({ steps: [
    { do: "expect-user", contains: "Read the fixture notes" },
    { do: "message", text: readyMarker }, betweenChecks, check("safe"), { do: "message", text: safeMarker },
    betweenChecks, check("unsafe"),
    betweenChecks, check("unsure"),
    betweenChecks, check("breaker"),
    { do: "expect-interrupt" },
  ] }));

  const page = await browser.newPage({ viewport: { width: 1100, height: 1000 } });
  await page.addInitScript((readyMarker) => {
    window.stageEStatus = [];
    let ready = false;
    new MutationObserver(() => {
      ready ||= document.querySelector(".wa-transcript")?.textContent.includes(readyMarker);
      const status = document.querySelector(".wa-status-word")?.textContent;
      if (ready && status && window.stageEStatus.at(-1) !== status) window.stageEStatus.push(status);
    }).observe(document, { childList: true, characterData: true, subtree: true });
  }, readyMarker);
  await page.goto(url);
  await page.getByRole("textbox", { name: "Message" }).fill("Read the fixture notes");
  await page.getByRole("button", { name: "Start session" }).click();
  await page.locator(".wa-status-word").filter({ hasText: /^Checking with your judge$/ }).waitFor({ timeout: 15_000 });
  await page.getByText(safeMarker, { exact: true }).waitFor({ timeout: 15_000 });
  for (const words of [/unsafe: 0\.90; off_task: 0\.10/, /unsafe: 0\.50; off_task: 0\.10/, /3 denials in a row/]) {
    await page.locator(".wa-transcript").getByText(words).first().waitFor({ timeout: 15_000 });
  }
  await page.locator(".wa-decision").filter({ hasText: "Stopped by your judge's breaker" }).waitFor({ timeout: 15_000 });
  const result = await page.evaluate(async () => {
    const { ok: sessions } = await window.watane.call("sessions.list", { after: null, max: 10 });
    const { ok: transcript } = await window.watane.call("sessions.read", { session: sessions[0].id, after: 0, before: null, max: 100 });
    return { status: window.stageEStatus, entries: transcript.entries.map((entry) => JSON.parse(entry)) };
  });
  assert.equal(result.status.filter((words) => words === "Checking with your judge").length, 4,
    `expected four visible tool checks, saw ${JSON.stringify(result.status)}`);
  for (const [id, verdict, decision] of [
    ["safe", "safe", "none"], ["unsafe", "unsafe", "deny"], ["unsure", "unsure", "deny"], ["breaker", "unsafe", "deny"],
  ]) {
    const call = `toolu_${id}`;
    assert.ok(result.entries.some(({ kind, body }) => kind === "verdict" && body.toolCallId === call && body.outcome === verdict),
      `missing ${verdict} verdict for ${call}`);
    assert.ok(result.entries.some(({ kind, body }) => kind === "ask" && body.toolCallId === call &&
      body.decision?.outcome === decision && body.decision.by === "checked"), `missing ${decision} decision for ${call}`);
  }
  console.log("PASS Stage E: checking, safe continuation, unsafe and unsure reasons, and the breaker");
} catch (e) {
  const notRun = e instanceof NotRun;
  console.error(`${notRun ? "NOT RUN" : "FAIL"} test:host:e: ${e.stack ?? e}`);
  if (preview?.log && existsSync(preview.log)) console.error(readFileSync(preview.log, "utf8").split("\n").slice(-20).join("\n"));
  process.exitCode = notRun ? 2 : 1;
} finally {
  await browser?.close();
  if (started) {
    try { dev("serve", "--stop"); } catch (e) { console.error(e.message); process.exitCode ||= 1; }
  }
  rmSync(temp, { recursive: true, force: true });
}
