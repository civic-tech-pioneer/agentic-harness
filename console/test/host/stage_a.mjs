// S32-4: the stage A story on watane's own host (watane at 4f105a9), not a stand-in: TECHNICAL 16 cases 20, 21, 36 (kernel part), 51 (kernel part)
// and the console's live and reload story, against `watane-dev serve` of kernel/wasm. By hand, like the probes (not in the gate nor in test:slow:
// CI installs the v0.1.0 watane-dev, which refuses `watane:host/streams`). Run: `npm run test:host` in console/.
// Exit 0: no FAIL. Exit 1: a failed check. Exit 2: not run (watane-dev missing or too old, no Chromium).
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import http from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";

const consoleDir = fileURLToPath(new URL("../..", import.meta.url));
const root = join(consoleDir, "..");
const wasmSrc = join(root, "kernel", "wasm");
const wasmDir = join(tmpdir(), "stage-a-mod-" + process.pid); // a copy: watane-dev rewrites the staged world (wit/deps) of the folder it serves
const tmp = mkdtempSync(join(tmpdir(), "stage-a-"));
const env = { ...process.env, WATANE_HOME: join(tmp, "home"), WATANE_DEV_CACHE: join(tmp, "cache") };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (s) => console.log(s);

let pass = 0, fail = 0, skip = 0;
const notes = [];
async function check(name, f) {
  try { await f(); pass++; say(`PASS ${name}`); }
  catch (e) { fail++; say(`FAIL ${name}: ${e instanceof Error ? e.message : String(e)}`); }
}
function skipped(name) { skip++; say(`SKIP ${name} (stage A has no provider stream; stage B)`); }
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
    throw Object.assign(new Error(text || `watane-dev serve exited ${r.status}`), { serveFailed: true });
  }
  const page = res.pages[res.pages.length - 1];
  preview = { url: page.url, home: res.home, log: res.log, api: res.api, reused: res.reused };
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
/** A raw /events client. `stop()` pauses reading the socket; `go()` resumes it. */
function events() {
  const p = parts();
  const c = { msgs: [], ended: false, endedAt: null, buf: "", connection: null, error: null };
  c.ready = new Promise((resolve, reject) => {
    const req = http.request({ host: "127.0.0.1", port: p.port, path: "/events", agent: false, headers: { Host: p.host, Accept: "text/event-stream" } }, (res) => {
      c.res = res; c.status = res.statusCode; res.setEncoding("utf8");
      res.on("data", (d) => {
        c.buf += d;
        for (let i; (i = c.buf.indexOf("\n\n")) >= 0;) {
          const block = c.buf.slice(0, i); c.buf = c.buf.slice(i + 2);
          const line = block.split("\n").find((l) => l.startsWith("data: "));
          if (!line) continue;
          const m = JSON.parse(line.slice(6)); c.msgs.push(m);
          if (m.type === "connection") { c.connection = m.id; resolve(c); }
        }
      });
      res.on("end", () => { c.ended = true; c.endedAt = Date.now(); });
      res.on("error", () => { c.ended = true; });
    });
    req.on("error", (e) => { c.error = e; c.ended = true; reject(e); });
    req.end(); c.req = req;
  });
  c.stop = () => c.res.pause(); c.go = () => c.res.resume(); c.close = () => c.req.destroy();
  return c;
}
const indexOf = (body) => { try { return JSON.parse(body).i; } catch { return undefined; } };
/** The stream's items as the console reads them (join.ts): an `entry` item wraps the entry under `_meta.watane`; other `_meta` items (synced, turn, queue) carry no index. */
const itemsOf = (c, stream) => c.msgs.filter((m) => m.type === "stream" && m.stream === stream).flatMap((m) => m.items)
  .map((it) => {
    const o = (() => { try { return JSON.parse(it.body); } catch { return {}; } })();
    const w = o._meta?.watane;
    const e = w ? (w.type === "entry" && w.entry && typeof w.entry === "object" ? w.entry : {}) : o;
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
function contiguous(entries, count) {
  const is = entries.map((e) => e.i);
  for (let k = 0; k < is.length; k++) if (is[k] !== k + 1) return `index ${is[k]} at position ${k + 1} (indexes ${is.slice(0, 12).join(",")}...)`;
  for (let k = 0; k < entries.length; k++) if (textOf(entries[k]) !== `Item ${k + 1} of ${count}`) return `entry ${k + 1} says ${JSON.stringify(textOf(entries[k]))}`;
  return null;
}
const startProbe = (count, every) => ok("probe.start", { key: `host-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, count, "every-ms": every });
async function sessionCount(id) { return (await transcript(id)).length; }
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
// the transcript is windowed to the rows near the viewport, at most 100 (TECHNICAL 18.6): a tall viewport and a probe of 90 keep every item in the page
async function openPage(browser) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 4000 } });
  page.on("pageerror", (e) => notes.push(`pageerror: ${e.message}`));
  await page.goto(preview.url);
  return page;
}
const kill9 = () => { const pid = Number(JSON.parse(readFileSync(join(preview.home, "run.json"), "utf8")).pid); process.kill(pid, "SIGKILL"); return pid; };

async function main() {
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

  // ---- serve (AC1) ----
  serve();
  say(`served page: ${preview.url}`);
  say(`preview home: ${preview.home}`);
  const logOffset = () => (existsSync(preview.log) ? statSync(preview.log).size : 0);
  const logSince = (n) => readFileSync(preview.log, "utf8").slice(n);

  const browser = await launch();
  try {
    // AC2 live, AC3 reload, AC4 replaced: one probe, items every 200 ms
    const p1 = await startProbe(90, 200);
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
      for (const e of es) { const s = seen[e.i]; if (s === undefined) continue; n++; worst = Math.max(worst, s - e.at); }
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
    await check("replaced: a second page ends the first's stream `replaced`, the second stays live and contiguous", async () => {
      await spy(page);
      const page2 = await openPage(browser);
      try {
        const why = async (e) => { const t = await page2.evaluate(() => document.body.textContent.slice(0, 300)).catch(() => "?"); throw new Error(`${e.message.split("\n")[0]}; page 2 text: ${JSON.stringify(t)}; page 1 messages: ${JSON.stringify(await page.evaluate(() => window.__msgs).catch(() => "?")).slice(0, 300)}`); };
        await page2.waitForFunction(() => /Item \d+ of 90/.test(document.body.textContent), null, { timeout: 15_000 }).catch(why);
        await until(() => page.evaluate(() => window.__msgs.some((m) => m.type === "stream-end")), 10_000, "the first page's stream-end");
        const end = await page.evaluate(() => window.__msgs.find((m) => m.type === "stream-end"));
        const outcome = typeof end.outcome === "string" ? end.outcome : Object.keys(end.outcome)[0];
        ensure(outcome === "replaced", `the first stream ended ${JSON.stringify(end.outcome)}`);
        const n1 = (await pageItems(page2)).length;
        await page2.waitForFunction((n) => (document.body.textContent.match(/Item \d+ of 90/g) ?? []).length >= n + 4, n1, { timeout: 10_000 }).catch(why);
        const items = await pageItems(page2); const bad = badRun(items);
        ensure(!bad, () => `the second page shows ${bad}`);
        const first = await pageItems(page);
        say(`  replaced: first page stopped at ${first.length} items, second at ${items.length}`);
      } finally { await page2.close(); }
    });
    await page.close();
    await ok("sessions.delete", { session: p1 }).catch(() => {});

    // AC5 lagging: a raw client that stops reading
    await check("lagging: a consumer that stops reading; a reattach after the last index read misses and doubles nothing", async () => {
      const count = 240;
      const p = await startProbe(count, 100);
      const c = events(); await c.ready;
      const stream = await ok("sessions.attach", { session: p, after: 0, caps: "{}" }, c.connection);
      await until(() => itemsOf(c, stream).some((x) => x.i >= 3), 10_000, "the first items");
      c.stop();
      const t0 = Date.now();
      await until(async () => (await sessionCount(p)) >= count, 60_000, "the probe's end");
      await sleep(500);
      c.go(); await sleep(1000);
      const ends = c.msgs.filter((m) => m.type === "stream-end" && m.stream === stream);
      const got = itemsOf(c, stream).filter((x) => x.kind === "message");
      const last = got.length ? Math.max(...got.map((x) => x.i)) : 0;
      let what;
      if (ends.length) what = `the stream ended ${JSON.stringify(ends[0].outcome)} with ${got.length} items read`;
      else if (c.ended) what = `the connection ended with no stream-end (after ${got.length} items read)`;
      else what = `the stream did not end; the socket buffers took all ${got.length} items of ${count} (${Math.round((Date.now() - t0) / 1000)} s); the probe's items (about 150 bytes) cannot reach the host's in-flight limit (256 items or 4 MiB) or the 16 MiB per connection, so the reattach is checked after the client closes the connection`;
      say(`  lagging: ${what}`);
      notes.push(`lagging (36, 51): ${what}`);
      c.close();
      ensure(last >= 3, `the client read only up to index ${last}`);
      // reattach after the last index read, on a new connection
      const c2 = events(); await c2.ready;
      const s2 = await ok("sessions.attach", { session: p, after: last, caps: "{}" }, c2.connection);
      await until(() => itemsOf(c2, s2).filter((x) => x.kind === "message").length >= count - last, 10_000, `items ${last + 1}..${count} on the reattach`);
      await sleep(500);
      const again = itemsOf(c2, s2).filter((x) => x.kind === "message").map((x) => x.i);
      c2.close();
      const all = [...got.map((x) => x.i).filter((i) => i <= last), ...again];
      ensure(all.every((x, k) => x === k + 1), () => `read ${last} then reattached: indexes ${all.slice(Math.max(0, last - 3), last + 4).join(",")} around the join`);
      ensure(all.length === count, `${all.length} of ${count} items after the reattach`);
      const bad = contiguous(await transcript(p), count);
      ensure(!bad, () => `sessions.read: ${bad}`);
    });

    // AC6 case 20: the host killed while a firing is due
    await check("case 20: a firing due while the host is down comes within 2 s of its start", async () => {
      const p = await startProbe(5, 3000);
      await until(async () => (await sessionCount(p)) >= 1, 10_000, "item 1");
      const t = Date.now();
      await sleep(2300); // item 2 is due in about 700 ms; the host dies before it
      kill9();
            await sleep(5000);
      try { serve(); } catch (e) { throw new Error(`restart failed: ${e.message}`); }
      const t1 = Date.now();
      try { await until(async () => (await sessionCount(p)) >= 2, 2000, "the due firing after the restart"); }
      catch (e) { throw new Error(`${e.message} (${await sessionCount(p).catch(() => "?")} items; killed ${Date.now() - t} ms after item 1)`); }
      say(`  case 20: item 2 came ${Date.now() - t1} ms after serve returned`);
      await ok("sessions.delete", { session: p }).catch(() => {});
    });
    await check("case 20: the firing of a deleted or cancelled probe is ignored, no error in watane.log", async () => {
      const d = await startProbe(20, 3000), x = await startProbe(20, 3000);
      await until(async () => (await sessionCount(d)) >= 1 && (await sessionCount(x)) >= 1, 10_000, "item 1 of both");
      const off = logOffset();
      await ok("sessions.delete", { session: d });
      await post("sessions.cancel", { session: x });
      const nx = await sessionCount(x);
      kill9();
      await sleep(5000);
      serve();
      await sleep(4000);
      const got = await post("sessions.read", { session: d, after: 0, before: null, max: 10 });
      ensure(got && "err" in got, `a deleted probe reads ${JSON.stringify(got).slice(0, 100)}`);
      const listed = await ok("sessions.list", { after: null, max: 256 });
      ensure(!listed.some((s) => s.id === d), "the deleted probe is listed");
      const nx2 = await sessionCount(x);
      ensure(nx2 === nx, `a cancelled probe went from ${nx} to ${nx2} items`);
      const log = logSince(off);
      const bad = log.split("\n").filter((l) => /error|panic|fail|trap/i.test(l));
      ensure(!bad.length, () => `watane.log: ${bad.slice(0, 3).join(" | ")}`);
      await ok("sessions.delete", { session: x }).catch(() => {});
    });

    // AC7 case 21: 10 kill -9 at random points
    await check("case 21: 10 kill -9 at random points, then a contiguous transcript and the probe goes on to its count", async () => {
      const count = 150;
      const p = await startProbe(count, 100);
      const lens = [];
      for (let k = 0; k < 10; k++) {
        await sleep(Math.floor(Math.random() * 1500));
        kill9();
        await sleep(Math.floor(Math.random() * 300));
        serve();
        lens.push(await sessionCount(p));
      }
      const bad0 = contiguous(await transcript(p), count);
      ensure(!bad0, () => `after the kills: ${bad0}`);
      await until(async () => (await sessionCount(p)) >= count, 60_000, `the probe to reach ${count}`);
      await sleep(1500);
      const es = await transcript(p);
      ensure(es.length === count, `${es.length} items, expected ${count}`);
      const bad = contiguous(es, count);
      ensure(!bad, () => `at the end: ${bad}`);
      say(`  case 21: transcript lengths after each restart: ${lens.join(",")}`);
    });
  } finally { await browser.close(); }
  skipped("case 7");
  skipped("case 36 defer half");
}

let code = 0;
try { await main(); }
catch (e) {
  if (e.serveFailed && /cannot bind loopback port/.test(e.message)) {
    say("NOT RUN test:host: the host binds 127.0.0.1 and [::1] both (host 8.1) and this machine has no IPv6 loopback");
    code = 2;
  } else if (e.notRun || (e.serveFailed && /streams/.test(e.message))) {
    say(`NOT RUN test:host: ${e.notRun ? e.message : `watane-dev refuses the kernel's world (watane:host/streams): ${e.message.slice(0, 200)}; install watane-dev from watane rev 4f105a9 or later`}`);
    code = 2;
  } else { fail++; say(`FAIL setup: ${e instanceof Error ? e.stack ?? e.message : String(e)}`); }
} finally {
  stopPreview();
  try { rmSync(tmp, { recursive: true, force: true }); rmSync(wasmDir, { recursive: true, force: true }); } catch {}
}
if (code !== 2) {
  for (const n of notes) say(`note: ${n}`);
  say(`stage A on watane 4f105a9: ${pass} pass, ${fail} fail, ${skip} skip`);
  code = fail ? 1 : 0;
}
process.exit(code);
