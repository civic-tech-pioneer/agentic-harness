// S35-4: stage B on watane's own host with the fake `claude` (dev 18.4), not a stand-in: the console starts a Claude Code session from the new-session
// view and reads it live and after a reload; and the stage A cases that apply to a Claude session, TECHNICAL 16 cases 5, 6, 20 (the part with no restart
// of a turn), 21 (the transcript part), 36 and 51 (kernel part) and 58 (host part). Against `watane-dev serve` of a copy of kernel/wasm with ../../provider
// added (`watane-dev add`). By hand, like the probes (not in the gate nor in test:slow). Run: `npm run test:host` in console/.
// Exit 0: no FAIL. Exit 1: a failed check, or a failure before the checks (an add or serve error, a first turn that ends provider-ended or does not end).
// Exit 2: not run, only for a prerequisite the line names: `watane-dev` missing or too old, `fake-claude` missing or not linked (no-binary), no
// provider folder, no Chromium, no IPv6 loopback, or a host without S99's imports (test/host/outcome.mjs).
// The fake's script is `<preview>/claude/fake-claude.json`, which each check rewrites (the fake reads it at every start).
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import http from "node:http";
import net from "node:net";
import { tmpdir } from "node:os";
import { delimiter, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";
import { NotRun, outcomeOf } from "./outcome.mjs";

const consoleDir = fileURLToPath(new URL("../..", import.meta.url));
const root = join(consoleDir, "..");
const wasmSrc = join(root, "kernel", "wasm");
const wasmDir = join(tmpdir(), "stage-b-mod-" + process.pid); // a copy: watane-dev rewrites the staged world (wit/deps) of the folder it serves
const tmp = mkdtempSync(join(tmpdir(), "stage-b-"));
const env = { ...process.env, WATANE_HOME: join(tmp, "home"), WATANE_DEV_CACHE: join(tmp, "cache") };
// the preview's fresh cache has no vendor/ (the provider's allowlist check needs it): seed it from the person's own cache when there is one
const vendor = join(process.env.HOME ?? "", ".cache", "watane-dev", "vendor");
if (process.env.HOME && existsSync(vendor)) cpSync(vendor, join(tmp, "cache", "vendor"), { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (s) => console.log(s);

let pass = 0, fail = 0, skip = 0;
const notes = [];
async function check(name, f) {
  try { await f(); pass++; say(`PASS ${name}`); }
  catch (e) { fail++; say(`FAIL ${name}: ${e instanceof Error ? e.message : String(e)}`); }
}
function skipped(name, why) { skip++; say(`SKIP ${name} (${why})`); }
function ensure(c, msg) { if (!c) throw new Error(typeof msg === "function" ? msg() : msg); }

// ---- watane-dev ----
function dev(args) {
  const r = spawnSync("watane-dev", [...args, "--json", "--dir", wasmDir], { env, encoding: "utf8", timeout: 600_000 });
  if (r.error) return { spawnError: r.error };
  let j; try { j = JSON.parse(r.stdout); } catch { j = null; }
  return { status: r.status, json: j, out: r.stdout, err: r.stderr };
}
function stopPreview() { try { spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", wasmDir], { env, encoding: "utf8", timeout: 60_000 }); } catch {} }
let preview = null; // {url, home, log, api}
function serve() {
  const r = dev(["serve"]);
  if (r.spawnError) throw Object.assign(new Error(`watane-dev cannot run: ${r.spawnError.message}`), { notRun: true });
  const res = r.json?.result;
  if (r.status !== 0 || !res) {
    const e = r.json?.errors?.[0] ?? r.json?.error;
    const text = e ? `${e.code ?? ""} ${e.message ?? ""}`.trim() : (r.err || r.out).trim().split("\n")[0];
    throw Object.assign(new Error(text || `watane-dev serve exited ${r.status}`), { serveFailed: true, code: e?.code });
  }
  const page = res.pages[res.pages.length - 1];
  preview = { url: page.url, home: res.home, log: res.log, api: res.api, reused: res.reused, out: r.out };
  return preview;
}

// ---- HTTP to the mod origin (host 8.1 to 8.4) ----
const parts = () => { const u = new URL(preview.url); return { host: u.host, port: Number(u.port), origin: `http://${u.host}` }; };
function post(fn, args, connection) {
  const p = parts();
  const body = JSON.stringify(connection ? { fn, args, connection } : { fn, args });
  return new Promise((resolve, reject) => {
    const req = http.request({ host: "127.0.0.1", port: p.port, path: "/call", method: "POST", agent: false,
      headers: { Host: p.host, Origin: p.origin, "content-type": "application/json", "content-length": Buffer.byteLength(body) } }, (res) => {
      let s = ""; res.setEncoding("utf8"); res.on("data", (c) => (s += c));
      res.on("end", () => { try { const j = JSON.parse(s); if ("value" in j) resolve(j.value); else reject(new Error(`${fn}: ${JSON.stringify(j.error ?? j)}`)); } catch { reject(new Error(`${fn}: HTTP ${res.statusCode} ${s.slice(0, 200)}`)); } });
    });
    req.setTimeout(20_000, () => req.destroy(new Error(`${fn}: no answer in 20 s`)));
    req.on("error", reject); req.end(body);
  });
}
async function ok(fn, args, connection) {
  const v = await post(fn, args, connection);
  if (v && typeof v === "object" && "ok" in v) return v.ok;
  throw new Error(`${fn} refused: ${JSON.stringify(v)}`);
}
/** A raw /events client on a bare socket (node:http keeps reading a paused response): `stop()` stops reading the socket; `go()` resumes it. */
function events() {
  const p = parts();
  const c = { msgs: [], ended: false, endedAt: null, buf: "", connection: null, error: null };
  let raw = Buffer.alloc(0), head = true, chunked = false;
  const sse = (text) => {
    c.buf += text;
    for (let i; (i = c.buf.indexOf("\n\n")) >= 0;) {
      const block = c.buf.slice(0, i); c.buf = c.buf.slice(i + 2);
      const line = block.split("\n").find((l) => l.startsWith("data: "));
      if (!line) continue;
      const m = JSON.parse(line.slice(6)); c.msgs.push(m);
      if (m.type === "connection") { c.connection = m.id; c.resolve(c); }
    }
  };
  c.ready = new Promise((resolve, reject) => {
    c.resolve = resolve;
    const s = net.connect({ host: "127.0.0.1", port: p.port });
    c.sock = s;
    s.on("connect", () => s.write(`GET /events HTTP/1.1\r\nHost: ${p.host}\r\nAccept: text/event-stream\r\n\r\n`));
    s.on("data", (d) => {
      raw = Buffer.concat([raw, d]);
      if (head) { const i = raw.indexOf("\r\n\r\n"); if (i < 0) return; const h = raw.subarray(0, i).toString(); c.status = Number(h.split(" ")[1]); chunked = /transfer-encoding:\s*chunked/i.test(h); raw = raw.subarray(i + 4); head = false; }
      if (!chunked) { sse(raw.toString("utf8")); raw = Buffer.alloc(0); return; }
      for (;;) {
        const i = raw.indexOf("\r\n"); if (i < 0) return;
        const n = parseInt(raw.subarray(0, i).toString(), 16);
        if (n === 0) { c.ended = true; c.endedAt = Date.now(); return; }
        if (raw.length < i + 2 + n + 2) return;
        sse(raw.subarray(i + 2, i + 2 + n).toString("utf8")); raw = raw.subarray(i + 2 + n + 2);
      }
    });
    s.on("end", () => { c.ended = true; c.endedAt ??= Date.now(); });
    s.on("close", () => { c.ended = true; c.endedAt ??= Date.now(); });
    s.on("error", (e) => { c.error = e; c.ended = true; reject(e); });
  });
  c.stop = () => c.sock.pause(); c.go = () => c.sock.resume(); c.close = () => c.sock.destroy();
  return c;
}
const indexOf = (body) => { try { return JSON.parse(body).i; } catch { return undefined; } };
/** The stream's items as the console reads them (join.ts): an `entry` item wraps the entry under `_meta.watane`; other `_meta` items (synced, turn, queue) carry no index. */
const itemsOf = (c, stream) => c.msgs.filter((m) => m.type === "stream" && m.stream === stream).flatMap((m) => m.items)
  .map((it) => {
    const o = (() => { try { return JSON.parse(it.body); } catch { return {}; } })();
    const w = o._meta?.watane;
    // an ACP update (no `type`) carries its entry's index in `_meta.watane.i` when it finishes a message or call; the turn items carry none
    const e = w ? (w.type === "entry" && w.entry && typeof w.entry === "object" ? w.entry : w.type === undefined ? { i: w.i } : {}) : o;
    return { seq: it.seq, i: e.i, kind: e.kind, meta: w };
  });

// ---- the transcript, read as the kernel gives it ----
async function transcript(session) {
  const out = []; let after = 0;
  for (;;) {
    const page = await ok("sessions.read", { session, after, before: null, max: 200 });
    const es = page.entries.map((e) => (typeof e === "string" ? JSON.parse(e) : e));
    if (!es.length) break;
    out.push(...es); after = es[es.length - 1].i;
    if (after >= page.last) break;
  }
  return out;
}
const textOf = (e) => e.body?.content?.[0]?.text;
/** Entry indexes 1, 2, 3 ... with no gap and no repeat (a Claude session's transcript holds the user message and turn entries too). */
function ascending(entries) {
  const is = entries.map((e) => e.i);
  for (let k = 0; k < is.length; k++) if (is[k] !== k + 1) return `index ${is[k]} at position ${k + 1} (indexes ${is.slice(0, 12).join(",")}...)`;
  return null;
}
/** The agent's `Item k of N` messages, in transcript order, must be 1, 2, 3 ... (up to `count` when it is given). */
function items(entries, count) {
  const ks = entries.filter((e) => e.kind === "message" && e.body?.role === "agent").map(textOf).map((t) => /^Item (\d+) of \d+$/.exec(t ?? "")).filter(Boolean).map((m) => +m[1]);
  for (let k = 0; k < ks.length; k++) if (ks[k] !== k + 1) return `item ${ks[k]} at position ${k + 1} (${ks.slice(0, 12).join(",")}...)`;
  if (count !== undefined && ks.length !== count) return `${ks.length} items, expected ${count}`;
  return null;
}

// ---- the fake claude's script, and a Claude session on it ----
const scriptPath = () => join(dirname(preview.home), "claude", "fake-claude.json");
const writeScript = (steps) => writeFileSync(scriptPath(), JSON.stringify({ steps }));
/** `Item k of N` as a finished message each `every` ms, then the turn's end: what stage A did with its probe, now as a Claude session. */
const itemSteps = (count, every) => [{ do: "expect-user" }, ...Array.from({ length: count }, (_, k) => [{ do: "message", text: `Item ${k + 1} of ${count}` }, { do: "sleep", ms: every }]).flat(), { do: "result" }, { do: "idle" }];
const startClaude = (input = "go") => ok("sessions.start", { key: `host-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, profile: "scratch", overlay: "{}", input });
async function startItems(count, every) { writeScript(itemSteps(count, every)); return startClaude(); }
async function sessionCount(id) { return (await transcript(id)).length; }
async function itemCount(id) { return (await transcript(id)).filter((e) => e.kind === "message" && e.body?.role === "agent").length; }
async function until(f, ms, what) { const t = Date.now(); for (;;) { const v = await f(); if (v) return v; if (Date.now() - t > ms) throw new Error(`timed out after ${ms} ms: ${what}`); await sleep(50); } }


// ---- the page ----
const SEEN = `(() => {
  window.__seen = {};
  const scan = () => { const t = document.body.textContent; for (const m of t.matchAll(/Item (\\d+) of (\\d+)/g)) { const k = +m[1]; if (!(k in window.__seen)) window.__seen[k] = Date.now(); } };
  new MutationObserver(scan).observe(document.body, { subtree: true, childList: true, characterData: true }); scan();
})()`;
const pageItems = (page) => page.evaluate(() => [...document.body.textContent.matchAll(/Item (\d+) of (\d+)/g)].map((m) => +m[1]));
const spy = (page) => page.evaluate(() => { window.__msgs = []; window.watane.on((m) => { if (m && m.type !== "changed") window.__msgs.push(m); }); });
function badRun(a) { for (let k = 0; k < a.length; k++) if (a[k] !== k + 1) return `position ${k + 1} shows item ${a[k]} (${a.slice(0, 12).join(",")}...)`; return null; }
// the transcript is windowed to the rows near the viewport, at most 100 (TECHNICAL 18.6): a tall viewport and a session of 90 items keep every item in the page
async function openPage(browser) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 4000 } });
  page.on("pageerror", (e) => notes.push(`pageerror: ${e.message}`));
  await page.goto(preview.url);
  return page;
}

const kill9 = () => { const pid = Number(JSON.parse(readFileSync(join(preview.home, "run.json"), "utf8")).pid); process.kill(pid, "SIGKILL"); return pid; };

/** Why the run cannot start (exit 2), before anything is built: a program or folder it needs is not here. */
function missing() {
  const w = spawnSync("watane-dev", ["--version"], { encoding: "utf8" });
  if (w.error) return `watane-dev cannot run: ${w.error.message}`;
  const dirs = (process.env.PATH ?? "").split(delimiter);
  const beside = dirs.map((d) => join(d, "watane-dev")).find((f) => existsSync(f));
  const fakes = [process.env.WATANE_FAKE_CLAUDE, beside && join(dirname(beside), "fake-claude"), ...dirs.map((d) => join(d, "fake-claude"))].filter(Boolean);
  if (!fakes.some((f) => existsSync(f))) return "fake-claude is not found (WATANE_FAKE_CLAUDE, beside watane-dev, or on PATH; a symbolic link to a program that is not built does not count): a preview cannot start Claude Code";
  if (!existsSync(join(root, "provider", "mod.toml"))) return "provider/ is not built here (the unchecked provider, S35-2): nothing to link claude-code";
  return null;
}

async function main() {
  const why = missing();
  if (why) throw new NotRun(why);
  // ---- build ----
  if (!existsSync(join(wasmSrc, "wit"))) throw new Error("kernel/wasm/wit is missing");
  const have = spawnSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" });
  if (!have.error && !/^wasm32-wasip2$/m.test(have.stdout)) execFileSync("rustup", ["target", "add", "wasm32-wasip2"], { stdio: "inherit" });
  execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip2", "-p", "agent-mod"], { cwd: join(root, "kernel"), stdio: "inherit" });
  const tdir = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { cwd: join(root, "kernel"), encoding: "utf8" })).target_directory;
  cpSync(join(tdir, "wasm32-wasip2", "release", "agent_mod.wasm"), join(wasmSrc, "logic.wasm"));
  execFileSync("npm", ["run", "build"], { cwd: consoleDir, stdio: "inherit" });
  rmSync(join(wasmSrc, "ui"), { recursive: true, force: true });
  cpSync(join(root, "ui"), join(wasmSrc, "ui"), { recursive: true });
  cpSync(wasmSrc, wasmDir, { recursive: true });
  // the provider is linked into the copy by its absolute folder (the copy is not under kernel/, so `../../provider` would not reach it)
  const added = dev(["add", join(root, "provider")]);
  if (added.spawnError || added.status !== 0) {
    const e = added.json?.errors?.[0] ?? added.json?.error;
    throw Object.assign(new Error(`watane-dev add provider: ${e ? `${e.code ?? ""} ${e.message ?? ""}` : (added.err || added.out).trim().split("\n")[0]}`), { addFailed: true, code: e?.code, out: added.out, err: added.err });
  }

  let browser;
  try { browser = await launch(); } catch (e) { throw new NotRun(`Chromium cannot start: ${e.message.split("\n")[0]}`); } // before serve: launch exits 2 without Playwright, which would skip the stop of the preview
  try {
    // ---- serve ----
    try { serve(); } catch (e) { if (e.serveFailed && /allowlist/.test(`${e.code} ${e.message}`) && !existsSync(vendor)) e.notRun = true, e.message = `watane-dev serve fails ${e.code ?? "env.allowlist"} for the provider and there is no ${vendor} to seed the preview cache from: ${e.message}`; throw e; }
    say(`served page: ${preview.url}`);
    say(`preview home: ${preview.home}`);
    const logOffset = () => (existsSync(preview.log) ? statSync(preview.log).size : 0);
    const logSince = (n) => readFileSync(preview.log, "utf8").slice(n);
    if (/preview\.fake_claude_missing/.test(preview.out ?? "")) throw new NotRun("watane-dev serve warns preview.fake_claude_missing: no fake claude is linked in the preview");

    // a first session on the default fake turn: if the host cannot run claude-code, the turn ends `no-binary` and nothing below means anything
    writeScript([{ do: "expect-user" }, { do: "delta", text: "This is the fake claude." }, { do: "message", text: "This is the fake claude." }, { do: "result" }, { do: "idle" }]);
    const first = await startClaude("hello");
    const firstLog = await until(async () => { const t = JSON.stringify(await transcript(first)); return /no-binary|provider-ended/.test(t) || /"idle"/.test(t) ? t : null; }, 30_000, "the first turn to end").catch(() => "");
    if (/no-binary/.test(firstLog)) throw Object.assign(new Error("the first fake turn ended no-binary: the preview has no fake claude to run"), { firstTurn: "no-binary" });
    if (/provider-ended/.test(firstLog)) throw Object.assign(new Error(`the first fake turn ended provider-ended: ${firstLog.slice(0, 300)}`), { firstTurn: "provider-ended" });
    if (!firstLog) throw Object.assign(new Error("the first fake turn did not end in 30 s"), { firstTurn: "stalled" });
    await ok("sessions.delete", { session: first });

    // AC5: the page starts a session on the fake claude, her message and the stream, Working then Idle, a reload that shows each entry once
    writeScript([{ do: "expect-user", contains: "hi" }, { do: "sleep", ms: 800 }, { do: "delta", text: "This is the fake claude.", chunk: 8 }, { do: "sleep", ms: 800 }, { do: "message", text: "This is the fake claude." }, { do: "result" }, { do: "idle" }]);
    const WORDS = `(() => { window.__words = []; const scan = () => { const w = document.querySelector(".wa-status-word")?.textContent; if (w && window.__words[window.__words.length - 1] !== w) window.__words.push(w); };
      new MutationObserver(scan).observe(document, { subtree: true, childList: true, characterData: true }); })()`;
    let started;
    {
      const page = await openPage(browser);
      await page.evaluate(WORDS);
      await check("page: Alice starts a session, types hi, and the fake claude's text streams in; the status line goes Working then Idle", async () => {
        await page.getByText("A new scratch folder").waitFor({ timeout: 15_000 });
        await page.getByRole("textbox", { name: "Message" }).fill("hi");
        await page.keyboard.press("Enter");
        await page.waitForFunction(() => document.querySelector(".wa-title")?.textContent !== "New session", null, { timeout: 15_000 });
        await page.waitForFunction(() => /This is the fake claude\./.test(document.body.textContent), null, { timeout: 15_000 });
        await page.waitForFunction(() => window.__words.includes("Idle"), null, { timeout: 15_000 });
        const words = await page.evaluate(() => window.__words);
        const w = words.indexOf("Working"), idle = words.lastIndexOf("Idle");
        ensure(w >= 0 && idle > w, `the status words were ${JSON.stringify(words)}`);
        const list = await ok("sessions.list", { after: null, max: 256 });
        // the page's session is the scratch one whose first message is hi (others, left by earlier checks, may still be listed)
        for (const l of list) if (l.profile === "scratch" && (await transcript(l.id)).some((e) => e.kind === "message" && e.body?.role === "user" && textOf(e) === "hi")) started = l.id;
        ensure(started, `no scratch session of hi among ${JSON.stringify(list).slice(0, 200)}`);
        say(`  status words: ${words.join(" > ")}`);
      });
      await check("reload: the same entries, each once, indexes unique and ascending, nothing doubled on the page", async () => {
        ensure(started, "no session was started by the page");
        await page.reload();
        await page.waitForFunction(() => /This is the fake claude\./.test(document.body.textContent), null, { timeout: 15_000 });
        await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Idle", null, { timeout: 15_000 });
        const text = await page.evaluate(() => document.body.innerText);
        ensure(text.split("This is the fake claude.").length === 2, `the agent's message shows ${text.split("This is the fake claude.").length - 1} times after the reload`);
        const es = await transcript(started);
        const bad = ascending(es); ensure(!bad, () => `entries: ${bad}`);
        ensure(es.filter((e) => e.kind === "message" && e.body?.role === "user" && textOf(e) === "hi").length === 1, "her message is not one entry");
        ensure(es.filter((e) => e.kind === "message" && e.body?.role === "agent" && textOf(e) === "This is the fake claude.").length === 1, "the agent's message is not one entry");
        say(`  reload: ${es.length} entries, indexes ${es.map((e) => e.i).join(",")}`);
      });
      await page.close();
    }
    if (started) await ok("sessions.delete", { session: started }).catch(() => {});

    // the states outcome 3 names, on a page of the frames' width (design/frames: 1280 x 820), with folded thinking; screenshots when STAGE_B_SCREENS names a folder
    await check("states: a session starting, a turn streaming with its thinking folded, finished entries, Working then Idle, the same after a reload", async () => {
      const shots = process.env.STAGE_B_SCREENS;
      const shot = async (pg, name) => { if (shots) await pg.screenshot({ path: join(shots, `${name}.png`) }); };
      const thinkDelta = (t) => JSON.stringify({ type: "stream_event", session_id: "fake", uuid: "fake-uuid", event: { type: "content_block_delta", index: 0, delta: { type: "thinking_delta", thinking: t } } });
      const thinkBlock = (t) => JSON.stringify({ type: "assistant", session_id: "fake", uuid: "fake-think", message: { id: "msg_think", type: "message", role: "assistant", model: "fake-claude", content: [{ type: "thinking", thinking: t, signature: "s" }], stop_reason: null, stop_sequence: null, usage: { input_tokens: 1, output_tokens: 1 } } });
      const thought = "The folder is new and empty, so there is nothing to read first. A short answer is enough.";
      const answer = "The scratch folder is empty. Tell me what to build in it and I will start there.";
      writeScript([{ do: "expect-user", contains: "What is in this folder?" }, { do: "sleep", ms: 2500 },
        { do: "raw", line: thinkDelta(thought.slice(0, 40)) }, { do: "sleep", ms: 600 }, { do: "raw", line: thinkDelta(thought.slice(40)) }, { do: "sleep", ms: 2500 },
        { do: "raw", line: thinkBlock(thought) }, { do: "sleep", ms: 300 },
        { do: "delta", text: answer.slice(0, 30) }, { do: "sleep", ms: 2500 }, { do: "delta", text: answer.slice(30) }, { do: "sleep", ms: 600 },
        { do: "message", text: answer }, { do: "result" }, { do: "idle" }]);
      const pg = await browser.newPage({ viewport: { width: 1280, height: 820 } });
      pg.on("pageerror", (e) => notes.push(`pageerror: ${e.message}`));
      let id;
      try {
        await pg.goto(preview.url);
        await pg.getByText("A new scratch folder").waitFor({ timeout: 15_000 });
        await pg.getByRole("textbox", { name: "Message" }).fill("What is in this folder?");
        await shot(pg, "1-new-session");
        await pg.evaluate(WORDS);
        await pg.keyboard.press("Enter");
        await pg.waitForFunction(() => document.querySelector(".wa-title")?.textContent !== "New session", null, { timeout: 15_000 });
        await pg.getByText("What is in this folder?").first().waitFor({ timeout: 10_000 });
        const startWord = await pg.evaluate(() => document.querySelector(".wa-status-word")?.textContent);
        ensure(startWord === "Working", `while the session starts the status word is ${JSON.stringify(startWord)}`);
        await shot(pg, "2-starting-working");
        await pg.waitForFunction(() => document.querySelector(".wa-thought[aria-busy='true']"), null, { timeout: 15_000 });
        const busyWord = await pg.evaluate(() => document.querySelector(".wa-status-word")?.textContent);
        ensure(busyWord === "Thinking", `while the thought streams the status word is ${JSON.stringify(busyWord)}`);
        await shot(pg, "3-thinking-streams");
        await pg.waitForFunction((t) => document.body.textContent.includes(t), answer.slice(0, 30), { timeout: 15_000 });
        const folded = await pg.evaluate(() => { const t = document.querySelector(".wa-thought"); return t && { open: t.getAttribute("data-open"), busy: t.getAttribute("aria-busy"), label: t.querySelector(".wa-fold")?.textContent, body: !!t.querySelector(".wa-fold-body") }; });
        ensure(folded && !folded.body, `the folded thought shows its text: ${JSON.stringify(folded)}`);
        ensure(folded && folded.open === "false" && folded.busy !== "true" && /^Thought/.test(folded.label ?? ""), `once the text streams the thought is ${JSON.stringify(folded)}, not folded`);
        ensure(!(await pg.evaluate((t) => document.body.textContent.includes(t), answer)), "the whole answer showed before its last delta");
        await shot(pg, "4-text-streams-thought-folded");
        await pg.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Idle", null, { timeout: 15_000 });
        ensure(await pg.evaluate((t) => document.body.textContent.includes(t), answer), "the finished answer is not shown");
        const words = await pg.evaluate(() => window.__words);
        ensure(words.indexOf("Working") >= 0 && words.lastIndexOf("Idle") > words.indexOf("Working"), `the status words were ${JSON.stringify(words)}`);
        await shot(pg, "5-finished-idle");
        const list = await ok("sessions.list", { after: null, max: 256 });
        for (const l of list) if ((await transcript(l.id)).some((e) => e.kind === "message" && textOf(e) === "What is in this folder?")) id = l.id;
        ensure(id, "no session holds her message");
        const es = await transcript(id);
        const agent = es.filter((e) => e.kind === "message" && e.body?.role === "agent");
        ensure(agent.length === 1 && agent[0].body?.thought?.text === thought && textOf(agent[0]) === answer, () => `the agent's entries: ${JSON.stringify(agent).slice(0, 300)}`);
        const before = await pg.evaluate(() => document.querySelector(".wa-transcript").innerText);
        await pg.reload();
        await pg.waitForFunction((t) => document.body.textContent.includes(t), answer, { timeout: 15_000 });
        await pg.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Idle", null, { timeout: 15_000 });
        await sleep(500);
        const after = await pg.evaluate(() => document.querySelector(".wa-transcript").innerText);
        await shot(pg, "6-after-reload");
        ensure(after.split(answer).length === 2 && after.split("What is in this folder?").length === 2, "after the reload a message shows more than once");
        const label = await pg.evaluate(() => document.querySelector(".wa-thought .wa-fold")?.textContent);
        ensure(/^Thought/.test(label ?? ""), `after the reload the thought reads ${JSON.stringify(label)}`);
        ensure(before.replace(/\s+/g, " ").trim() === after.replace(/\s+/g, " ").trim(), () => `the transcript reads differently after the reload:\n${JSON.stringify(before.slice(0, 300))}\n---\n${JSON.stringify(after.slice(0, 300))}`);
        say(`  states: words ${words.join(" > ")}; ${es.length} entries${shots ? `; screenshots in ${shots}` : ""}`);
      } finally { await pg.close(); if (id) await ok("sessions.delete", { session: id }).catch(() => {}); }
    });

    // case 8: a retried start with the same key makes one session; after delete the key still returns the old result
    await check("case 8: a start retried with its key gives the same session, one in the list; after delete the key still gives it", async () => {
      writeScript([{ do: "expect-user" }, { do: "message", text: "Item 1 of 1" }, { do: "result" }, { do: "idle" }]);
      const key = `k8-${Date.now()}`;
      const args = { key, profile: "scratch", overlay: "{}", input: "go" };
      const a = await ok("sessions.start", args), b = await ok("sessions.start", args);
      ensure(a === b, `the retry gave ${b}, the first ${a}`);
      const listed = (await ok("sessions.list", { after: null, max: 256 })).filter((s) => s.id === a);
      ensure(listed.length === 1, `the session is listed ${listed.length} times`);
      await until(async () => (await itemCount(a)) >= 1, 15_000, "the turn's message");
      ensure((await itemCount(a)) === 1, "the retry ran the turn again");
      await ok("sessions.delete", { session: a });
      const c = await ok("sessions.start", args);
      ensure(c === a, `after delete the key gave ${c}, not ${a}`);
      ensure(!(await ok("sessions.list", { after: null, max: 256 })).some((s) => s.id === a), "the deleted session came back");
    });

    // outcome 1 (S32 R1) and case 12's first half: a refused start commits none of its writes on watane's host
    await check("R1 and case 12: a start refused for a subagent's permissionMode leaves no session and no key; the key then starts one", async () => {
      writeScript([{ do: "expect-user" }, { do: "message", text: "Item 1 of 1" }, { do: "result" }, { do: "idle" }]);
      const before = (await ok("sessions.list", { after: null, max: 256 })).map((s) => s.id).sort();
      const key = `r1-${Date.now()}`;
      const overlay = JSON.stringify({ subagents: [{ name: "a", description: "d", prompt: "p", permissionMode: "bypassPermissions" }] });
      const r = await post("sessions.start", { key, profile: "scratch", overlay, input: "go" });
      ensure(r && "err" in r, `the start gave ${JSON.stringify(r)}`);
      ensure(/permissionMode|subagents/.test(JSON.stringify(r.err)), `the refusal names no subagent field: ${JSON.stringify(r.err)}`);
      const after = (await ok("sessions.list", { after: null, max: 256 })).map((s) => s.id).sort();
      ensure(JSON.stringify(after) === JSON.stringify(before), `the list went from ${before.length} to ${after.length} sessions`);
      const s = await ok("sessions.start", { key, profile: "scratch", overlay: "{}", input: "go" });
      ensure(!before.includes(s), "the key gave an old session");
      await until(async () => (await itemCount(s)) >= 1, 15_000, "the new session's turn");
      await ok("sessions.delete", { session: s }).catch(() => {});
    });

    // case 11 (the say half): a slash command is refused at start and at say, and the turn is not touched
    await check("case 11: say(\"/x\") and a start with \" /x\" are refused; the session stays Idle and takes a later say", async () => {
      writeScript([{ do: "expect-user" }, { do: "message", text: "Item 1 of 2" }, { do: "result" }, { do: "idle" }, { do: "expect-user", contains: "next" }, { do: "message", text: "Item 2 of 2" }, { do: "result" }, { do: "idle" }]);
      const r0 = await post("sessions.start", { key: `k11-${Date.now()}`, profile: "scratch", overlay: "{}", input: " /x" });
      ensure(r0 && "err" in r0, `a start with " /x" gave ${JSON.stringify(r0)}`);
      const s = await startClaude();
      await until(async () => /"idle"/.test(JSON.stringify((await transcript(s)).slice(-2))), 15_000, "the first turn's end");
      const n = await sessionCount(s);
      const r = await post("sessions.say", { session: s, key: `say-${Date.now()}`, input: "/x" });
      ensure(r && "err" in r, `say("/x") gave ${JSON.stringify(r)}`);
      await sleep(800);
      ensure((await sessionCount(s)) === n, "the refused say wrote an entry");
      await ok("sessions.say", { session: s, key: `say2-${Date.now()}`, input: "next" });
      await until(async () => (await itemCount(s)) >= 2, 15_000, "the later turn's message");
      const es = await transcript(s);
      const bad = ascending(es) ?? items(es, 2);
      ensure(!bad, () => `after the later say: ${bad}`);
      await ok("sessions.delete", { session: s }).catch(() => {});
    });

    // case 5: an unknown stream-json line is an `unknown` entry and the session goes on (maker script with a `raw` line)
    await check("case 5: a raw line the kernel does not know becomes an unknown entry, and the turn goes on to idle", async () => {
      writeScript([{ do: "expect-user" }, { do: "message", text: "Item 1 of 2" }, { do: "raw", line: JSON.stringify({ type: "not_a_stream_json_line", n: 1 }) }, { do: "message", text: "Item 2 of 2" }, { do: "result" }, { do: "idle" }]);
      const s = await startClaude();
      const es = await until(async () => { const t = await transcript(s); return t.filter((e) => e.kind === "message" && e.body?.role === "agent").length >= 2 ? t : null; }, 20_000, "both messages");
      const u = es.findIndex((e) => e.kind === "unknown");
      ensure(u >= 0, () => `no unknown entry among ${es.map((e) => e.kind).join(",")}`);
      const after = es.findIndex((e, k) => k > u && textOf(e) === "Item 2 of 2");
      ensure(after > u, "the message after the unknown line is not stored after it");
      const bad = ascending(es); ensure(!bad, () => `entries: ${bad}`);
      await until(async () => /"idle"/.test(JSON.stringify((await transcript(s)).slice(-2))), 10_000, "the turn's end");
      await ok("sessions.delete", { session: s }).catch(() => {});
    });

    // case 6 (attach, replaced), live and reload: one Claude session, items every 200 ms
    const p1 = await startItems(90, 200);
    const page = await openPage(browser);
    await page.evaluate(SEEN);
    await spy(page);
    await check("live: items arrive in order, none doubled, within 2 s", async () => {
      await page.waitForFunction(() => /Item 8 of 90/.test(document.body.textContent), null, { timeout: 15_000 });
      await sleep(1500);
      const items = await pageItems(page);
      const bad = badRun(items); ensure(!bad, () => `the page shows ${bad}`);
      const es = await transcript(p1);
      const seen = await page.evaluate(() => window.__seen);
      let worst = 0, n = 0;
      for (const e of es) { const m = /^Item (\d+) of/.exec(textOf(e) ?? ""); const s = m && seen[+m[1]]; if (!s) continue; n++; worst = Math.max(worst, s - e.at); }
      ensure(n >= 5, `only ${n} items were timed`);
      ensure(worst <= 2000, `an item reached the page ${worst} ms after its entry's time (${n} timed)`);
      say(`  live: ${items.length} items on the page, worst delay ${worst} ms over ${n}`);
    });
    await check("reload: every item once, and new ones keep arriving", async () => {
      const before = (await pageItems(page)).length;
      await page.reload();
      await page.waitForFunction((n) => (document.body.textContent.match(/Item \d+ of 90/g) ?? []).length >= n, before, { timeout: 15_000 });
      let items = await pageItems(page);
      let bad = badRun(items); ensure(!bad, () => `after reload the page shows ${bad}`);
      const shown = items.length;
      await page.waitForFunction((n) => (document.body.textContent.match(/Item \d+ of 90/g) ?? []).length >= n + 4, shown, { timeout: 10_000 });
      items = await pageItems(page); bad = badRun(items);
      ensure(!bad, () => `after reload and new items the page shows ${bad}`);
      say(`  reload: ${shown} items at once, ${items.length} after waiting`);
    });
    await check("case 6 replaced: a second page ends the first's stream `replaced`, the second stays live and contiguous", async () => {
      await spy(page);
      const page2 = await openPage(browser);
      try {
        const explain = async (e) => { const t = await page2.evaluate(() => document.body.textContent.slice(0, 300)).catch(() => "?"); throw new Error(`${e.message.split("\n")[0]}; page 2 text: ${JSON.stringify(t)}; page 1 messages: ${JSON.stringify(await page.evaluate(() => window.__msgs).catch(() => "?")).slice(0, 300)}`); };
        await page2.waitForFunction(() => /Item \d+ of 90/.test(document.body.textContent), null, { timeout: 15_000 }).catch(explain);
        await until(() => page.evaluate(() => window.__msgs.some((m) => m.type === "stream-end")), 10_000, "the first page's stream-end");
        const end = await page.evaluate(() => window.__msgs.find((m) => m.type === "stream-end"));
        const outcome = typeof end.outcome === "string" ? end.outcome : Object.keys(end.outcome)[0];
        ensure(outcome === "replaced", `the first stream ended ${JSON.stringify(end.outcome)}`);
        const n1 = (await pageItems(page2)).length;
        await page2.waitForFunction((n) => (document.body.textContent.match(/Item \d+ of 90/g) ?? []).length >= n + 4, n1, { timeout: 10_000 }).catch(explain);
        const items = await pageItems(page2); const bad = badRun(items);
        ensure(!bad, () => `the second page shows ${bad}`);
        say(`  replaced: first page stopped at ${(await pageItems(page)).length} items, second at ${items.length}`);
      } finally { await page2.close(); }
    });
    await page.close();
    await ok("sessions.delete", { session: p1 }).catch(() => {});

    // cases 36 (kernel part), 51 (kernel part), 58 (host part): a raw client that stops reading
    await check("lagging (36, 51, 58): a consumer that stops reading; a reattach after the last index read misses and doubles nothing", async () => {
      const count = 240;
      const p = await startItems(count, 100);
      const c = events(); await c.ready;
      const stream = await ok("sessions.attach", { session: p, after: 0, caps: "{}" }, c.connection);
      await until(() => itemsOf(c, stream).some((x) => x.i >= 3), 10_000, "the first items");
      c.stop();
      const t0 = Date.now(), readAtStop = c.sock.bytesRead;
      await until(async () => (await itemCount(p)) >= count, 60_000, "the session's last item");
      await sleep(500);
      const pausedMs = Date.now() - t0;
      c.go(); await sleep(1000);
      const ends = c.msgs.filter((m) => m.type === "stream-end" && m.stream === stream);
      const got = itemsOf(c, stream).filter((x) => x.i !== undefined);
      const last = got.length ? Math.max(...got.map((x) => x.i)) : 0;
      let what;
      if (ends.length) what = `the stream ended ${JSON.stringify(ends[0].outcome)} with ${got.length} entries read`;
      else if (c.ended) what = `the connection ended with no stream-end (after ${got.length} entries read)`;
      else what = `the stream did not end: the client read ${readAtStop} bytes, stopped reading the socket for ${Math.round(pausedMs / 1000)} s while the session wrote ${count} items, and the host and loopback buffers held the rest (${got.length} entries, ${c.sock.bytesRead} bytes in all); neither the in-flight limit (256 items or 4 MiB) nor the 16 MiB per connection was reached; the reattach is checked after the client closes the connection`;
      say(`  lagging: ${what}`);
      notes.push(`lagging (36, 51, 58): ${what}`);
      c.close();
      ensure(last >= 3, `the client read only up to index ${last}`);
      // reattach after the last index read, on a new connection
      const total = (await transcript(p)).length;
      const c2 = events(); await c2.ready;
      const s2 = await ok("sessions.attach", { session: p, after: last, caps: "{}" }, c2.connection);
      await until(() => itemsOf(c2, s2).filter((x) => x.i !== undefined).length >= total - last, 10_000, `entries ${last + 1}..${total} on the reattach`);
      await sleep(500);
      const again = itemsOf(c2, s2).filter((x) => x.i !== undefined).map((x) => x.i);
      c2.close();
      // every stored entry goes out live once with its index (9.4, S35-fix1: a message or tool as its finishing update, the rest, turn entries too,
      // as an `entry` item), so the indexes read live and then reattached are 1..n, each once
      const all = [...got.map((x) => x.i), ...again];
      ensure(all.length === total && all.every((x, k) => x === k + 1), () => `read up to ${last} live (${got.length} indexes), then ${again.length} reattached: indexes ${all.slice(Math.max(0, last - 3), last + 4).join(",")} around the join, ${all.length} of ${total}`);
      const es = await transcript(p);
      ensure(total === es.length, `the transcript grew from ${total} to ${es.length} during the check`);
      const bad = ascending(es) ?? items(es, count);
      await ok("sessions.delete", { session: p }).catch(() => {}); // the live and reload checks below open the newest session: leave none of this one
      ensure(!bad, () => `sessions.read: ${bad}`);
    });

    // case 20 (the part with no turn to restart): what a deleted or interrupted Claude session leaves behind across a host kill
    await check("case 20: a deleted session stays gone and an interrupted one stays as it was after a kill -9 and restart, no error in watane.log", async () => {
      const d = await startItems(20, 3000), x = await startItems(20, 3000);
      await until(async () => (await itemCount(d)) >= 1 && (await itemCount(x)) >= 1, 15_000, "item 1 of both");
      const off = logOffset();
      await ok("sessions.delete", { session: d });
      await ok("sessions.interrupt", { session: x, replacement: null }).catch(() => {});
      await sleep(1500);
      const nx = await sessionCount(x);
      kill9();
      await sleep(5000);
      serve();
      await sleep(4000);
      const got = await post("sessions.read", { session: d, after: 0, before: null, max: 10 });
      ensure(got && "err" in got, `a deleted session reads ${JSON.stringify(got).slice(0, 100)}`);
      const listed = await ok("sessions.list", { after: null, max: 256 });
      ensure(!listed.some((s) => s.id === d), "the deleted session is listed");
      const nx2 = await sessionCount(x);
      ensure(nx2 === nx, `an interrupted session went from ${nx} to ${nx2} entries`);
      const bad = logSince(off).split("\n").filter((l) => /panic|trap|"level":"error"|\bERROR\b/.test(l));
      ensure(!bad.length, () => `watane.log: ${bad.slice(0, 3).join(" | ")}`);
      await ok("sessions.delete", { session: x }).catch(() => {});
    });

    // case 21 (the transcript part): 10 kill -9 at random points while a Claude session writes
    await check("case 21: 10 kill -9 at random points, then the transcript is ascending, whole, and its items are 1, 2, 3 ...", async () => {
      const count = 150;
      const p = await startItems(count, 100);
      const lens = [];
      for (let k = 0; k < 10; k++) {
        await sleep(Math.floor(Math.random() * 1500));
        kill9();
        await sleep(Math.floor(Math.random() * 300));
        serve();
        lens.push(await sessionCount(p));
      }
      await sleep(1500);
      const es = await transcript(p);
      const bad = ascending(es) ?? items(es);
      ensure(!bad, () => `after the kills: ${bad}`);
      ensure(lens.every((n, k) => k === 0 || n >= lens[k - 1]), `the transcript shrank: ${lens.join(",")}`);
      say(`  case 21: transcript lengths after each restart: ${lens.join(",")}`);
    });
  } finally { await browser.close(); }
  skipped("case 20 timer due during downtime", "a Claude session's timers are an ask's expiry and the 10-minute idle deadline; a fake turn makes neither due without an ask and a restarted turn (S36)");
  skipped("case 21 the effect of an answer after a kill, and the turn going on to its count", "needs an ask to answer and a turn that resumes after a restart (S36)");
  skipped("case 7 a failed batch", "no consumer mod that fails a batch is linked in a preview; host only");
  skipped("case 36 the defer half", "no consumer mod that answers `defer` is linked in a preview; host only");
  skipped("asks, the queue, Stop and Try again on the real kernel", "need asks and restart handling (S36)");
  skipped("case 9 two answers to one ask", "the Claude adapter does not map can_use_tool into an ask yet (asks are S36)");
  skipped("case 10 a hook callback that errors or times out", "no stage B launch registers a hook; hook callbacks come with asks and checks (S36, stage E)");
  skipped("case 11 compact", "the kernel interface has no compact call in stage B");
  skipped("case 12 depth 0 removes Task", "not built: the kernel does not remove Task at depth 0 yet (no kernel test either); delegation is stage F (9.7)");
}

let code = 0;
try { await main(); }
catch (e) {
  const o = outcomeOf(e);
  if (o.code === 2) code = 2; else fail++;
  say(o.line);
} finally {
  stopPreview();
  try { rmSync(tmp, { recursive: true, force: true }); rmSync(wasmDir, { recursive: true, force: true }); } catch {}
}
if (code !== 2) {
  for (const n of notes) say(`note: ${n}`);
  say(`stage B on the fake claude: ${pass} pass, ${fail} fail, ${skip} skip`);
  code = fail ? 1 : 0;
}
process.exit(code);
