// S36-5, S36 r1 F1.8: the stage B budgets (TECHNICAL 17, row B) end to end on watane's own host with dev's fake `claude`, measured in
// the browser: 8 sessions, 4 for each of 2 owners (the console's `page`, each session shown in its own Chromium page, and the test mod
// `loadclient` as `local/loadclient`), each on the fake's `load` with `rate {deltas: 50, entries: 5, bytes: 4096}`, all started together.
// Item delay: the fake's stdout write to the item's arrival in the Chromium page that shows its session (the page's `watane.on`
// handler, before the console's own), p95 over the page's 4 sessions, budget 100 ms. Stop: the page's Stop button to
// `idle {interrupted}` in that page, p95, budget 1 s. Each page session is paired with its fake by identity (the fake's first input,
// `load <k>`, which its stamp file names), and every line its fake wrote before Stop must reach its page exactly once. Latency counts
// only once all 8 fakes loaded together through a common window of at least 6 s ending at the first Stop, with no gap between
// consecutive stamps of a source (the window's edges included) over three times the nominal spacing (60 ms for deltas, 600 ms for
// entries), and only the items written in it; Stop ends at exactly `idle {interrupted}` (S36 r2 F2.3, test/host/load_check.mjs).
// The state bytes per entry are the kernel crate's (kernel/tests/it/s36_load.rs). By hand: `npm run test:load` in console/. Exit 0
// within budget, 1 over or failed, 2 not run (a prerequisite missing: watane-dev, fake-claude, cargo, python3, Chromium).
//
// Two test-only reaches, each named where it is used: the second owner's `access/local/loadclient` is written into the stopped preview's
// kernel state by test/host/seed (no kernel call writes `access/`, 9.1), and the preview's fake-claude link is replaced by a wrapper that
// stamps each stdout line with its write time and names the fake's first input (the fake writes no time of its own).
//
// `--home` (S131-4, `npm run test:load:c`): the stage C budgets (TECHNICAL 17, row C, "as B" on the fake, the kernel and host part)
// on a participant home: the mods the preview built are installed into a home whose app is test/host/home's fake `Confirm`
// (`--grant-all`: each consent is asked and granted at once), and each of the 8 sessions starts with `start-in-folder` in a folder new to
// it, so every launch takes the consent path; Stop's budget is 2 s. Needs WATANE_SRC (or watane-dev inside a watane checkout's target/).
import { execFileSync, spawnSync } from "node:child_process";
import { spawn } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, readlinkSync, realpathSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { createInterface } from "node:readline";
import http from "node:http";
import { tmpdir, loadavg, cpus } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";
import { isInterrupted, loadWindow } from "./load_check.mjs";
import { writerBusy } from "./stage-d-tools/metrics.mjs";

const consoleDir = fileURLToPath(new URL("../..", import.meta.url));
const root = join(consoleDir, "..");
const tmp = mkdtempSync(join(tmpdir(), "load-b-"));
const env = { ...process.env, WATANE_HOME: join(tmp, "home"), WATANE_DEV_CACHE: join(tmp, "cache") };
const vendor = join(process.env.HOME ?? "", ".cache", "watane-dev", "vendor");
if (process.env.HOME && existsSync(vendor)) cpSync(vendor, join(tmp, "cache", "vendor"), { recursive: true });
const kernelDir = join(tmp, "agent"), clientDir = join(tmp, "loadclient");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (s) => console.log(s);
class NotRun extends Error {}
const SESSIONS = 4, SECONDS = 10, RATE = { deltas: 50, entries: 5, bytes: 4096, seconds: SECONDS };
const HOME = process.argv.includes("--home");
const STAGE = HOME ? "C" : "B", COMMAND = HOME ? "test:load:c" : "test:load";
const BUDGET = { delay: 100, stop: HOME ? 2000 : 1000 }; // TECHNICAL 17: Stop to idle, B 1 s, C 2 s
const WINDOW = 6; // seconds all 8 sources must overlap at the rate, before Stop

function dev(args, dir) {
  const r = spawnSync("watane-dev", [...args, "--json", "--dir", dir], { env, encoding: "utf8", timeout: 900_000 });
  if (r.error) throw new NotRun(`watane-dev cannot run: ${r.error.message}`);
  let j; try { j = JSON.parse(r.stdout); } catch { j = null; }
  if (r.status !== 0) { const e = j?.errors?.[0] ?? j?.error; throw new Error(`watane-dev ${args[0]}: ${e ? `${e.code} ${e.message}` : (r.stderr || r.stdout).slice(0, 400)}`); }
  return j.result;
}
let preview = null;
function serve() {
  const res = dev(["serve"], clientDir);
  const pageOf = (slug) => res.pages.find((p) => p.slug === slug || (p.local ?? "").includes(slug))?.url;
  preview = { home: res.home, agent: pageOf("agent"), client: pageOf("loadclient") };
  if (!preview.agent || !preview.client) throw new Error(`serve gave no page for the kernel or the client: ${JSON.stringify(res.pages)}`);
}
const stop = () => { const pid = Number(JSON.parse(readFileSync(join(preview.home, "run.json"), "utf8")).pid); process.kill(pid, "SIGTERM"); return pid; };
/** Whether `pid` exited, also as a zombie no one reaps (Linux, a container whose init does not reap: the review's r6 run). */
const zombie = (pid) => { try { return /^\d+ \(.*\) Z /.test(readFileSync(`/proc/${pid}/stat`, "utf8")); } catch { return false; } };
const gone = async (pid) => { for (let k = 0; k < 150; k++) { try { process.kill(pid, 0); } catch { return; } if (zombie(pid)) return; await sleep(100); } throw new Error("the host did not stop"); };

function post(url, fn, args, connection) {
  const u = new URL(url);
  const body = JSON.stringify(connection ? { fn, args, connection } : { fn, args });
  return new Promise((resolve, reject) => {
    const req = http.request({ host: "127.0.0.1", port: Number(u.port), path: "/call", method: "POST", agent: false,
      headers: { Host: u.host, Origin: `http://${u.host}`, "content-type": "application/json", "content-length": Buffer.byteLength(body) } }, (res) => {
      let s = ""; res.setEncoding("utf8"); res.on("data", (c) => (s += c));
      res.on("end", () => { try { const j = JSON.parse(s); if ("value" in j) resolve(j.value); else reject(new Error(`${fn}: ${s.slice(0, 200)}`)); } catch { reject(new Error(`${fn}: HTTP ${res.statusCode} ${s.slice(0, 200)}`)); } });
    });
    req.setTimeout(20_000, () => req.destroy(new Error(`${fn}: no answer in 20 s`)));
    req.on("error", reject); req.end(body);
  });
}
async function ok(url, fn, args, connection) {
  const v = await post(url, fn, args, connection);
  if (v && typeof v === "object" && "ok" in v) return v.ok;
  throw new Error(`${fn} refused: ${JSON.stringify(v)}`);
}
/** Runs in each Chromium page before watane.js: every stream item the page's `watane.on` gets, with its arrival time, before the
 * console's handler sees it; and when `sessions.attach` resolved. */
const PROBE = `(() => {
  const A = (window.__arrivals = []); window.__attached = null;
  const t = () => performance.timeOrigin + performance.now();
  let real;
  Object.defineProperty(window, "watane", { configurable: true, get: () => real, set(v) {
    real = Object.freeze({ ...v,
      on(h) { return v.on((m) => { if (m && m.type === "stream" && Array.isArray(m.items)) { const at = t(); for (const it of m.items) A.push({ stream: m.stream, seq: it.seq, body: it.body, at }); } h(m); }); },
      async call(fn, args) { const r = await v.call(fn, args); if (fn === "sessions.attach" && r && "ok" in r) window.__attached = { at: t(), session: args.session, stream: r.ok }; return r; } });
  } });
})();`;
/** Each item a page got: a text delta `delta N `, a finished `entry N`, or a turn; with its stream and arrival. */
function arrivals(raw) {
  const out = [];
  for (const it of raw) {
    let o; try { o = JSON.parse(it.body); } catch { continue; }
    const w = o._meta?.watane;
    // an ACP update is the item itself (`sessionUpdate`); a finished message comes as an update with `_meta.watane.i`, or as an `entry` item
    const text = o.content?.text ?? w?.entry?.body?.content?.[0]?.text ?? "";
    const d = /^delta (\d+) $/.exec(text);
    const e = /^entry (\d+) /.exec(text);
    if (d) out.push({ kind: "delta", n: +d[1], at: it.at, stream: it.stream });
    else if (e) out.push({ kind: "entry", n: +e[1], at: it.at, stream: it.stream });
    else if (w?.type === "turn" || w?.entry?.kind === "turn") out.push({ kind: "turn", state: w.state ?? w.entry?.body, at: it.at, stream: it.stream });
  }
  return out;
}

// The stamping wrapper (test-only): the host starts <P>/claude/fake-claude; the wrapper runs the real fake with the same argv[0] (so it
// finds its script and log beside the link) and writes `<ns since epoch>\t<kind>\t<n>` per stdout line to stamps-<pid>.tsv.
const WRAPPER = `#!/usr/bin/python3 -I
import os, re, subprocess, sys, time, threading, uuid
real = REAL
me = os.path.abspath(sys.argv[0])
p = subprocess.Popen([me] + sys.argv[1:], executable=real, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
out = open(os.path.join(os.path.dirname(me), "stamps-%d.tsv" % os.getpid()), "w", buffering=1)
lock = threading.Lock()
def feed():
    named = False
    for line in sys.stdin.buffer:
        if not named:
            m = re.search(rb'"(?:text|content)":"(load (?:client )?\\d+)"', line)
            if m:
                named = True
                with lock: out.write("input\\t%s\\n" % m.group(1).decode())
        try:
            p.stdin.write(line); p.stdin.flush()
        except (BrokenPipeError, ValueError):
            break
    try: p.stdin.close()
    except Exception: pass
threading.Thread(target=feed, daemon=True).start()
pat = re.compile(rb'"text":"(delta|entry) (\\d+) ')
# the fake's fixed session id becomes this process's own: watane binds a Claude session id to its first owner (as stage D's wrapper)
own = b'"session_id":"' + uuid.uuid4().hex.encode() + b'"'
for line in p.stdout:
    sys.stdout.buffer.write(line.replace(b'"session_id":"fake-session"', own)); sys.stdout.buffer.flush()
    m = pat.search(line[:400])
    if m:
        with lock: out.write("%d\\t%s\\t%s\\n" % (time.time_ns(), m.group(1).decode(), m.group(2).decode()))
sys.exit(p.wait())
`;
function wrapFake(link = join(dirname(preview.home), "claude", "fake-claude"), real = readlinkSync(link)) {
  if (existsSync(link)) unlinkSync(link);
  writeFileSync(link, WRAPPER.replace("REAL", JSON.stringify(real)));
  chmodSync(link, 0o755);
  writeFileSync(join(dirname(link), "fake-claude.json"), JSON.stringify({ steps: [{ do: "expect-user" }, { do: "message", text: "ready" }, { do: "result" }, { do: "idle" }, { do: "expect-user" }, { do: "load", rate: RATE }, { do: "expect-user" }] }));
  return dirname(link);
}
/** Each fake's stamps by the first input it got (`load <k>`, `load client <k>`): its identity. */
function stamps(dir) {
  const by = new Map();
  for (const f of readdirSync(dir).filter((f) => /^stamps-\d+\.tsv$/.test(f))) {
    let input; const xs = [];
    for (const l of readFileSync(join(dir, f), "utf8").split("\n").filter(Boolean)) {
      const [a, b, c] = l.split("\t");
      if (a === "input") input = b; else xs.push({ at: Number(BigInt(a)) / 1e6, kind: b, n: +c });
    }
    if (input === undefined) continue; // a fake with no load input (none in this run)
    if (by.has(input)) throw new Error(`two fakes got the input ${input}`);
    by.set(input, xs);
  }
  return by;
}
const p95 = (xs) => { const s = [...xs].sort((a, b) => a - b); return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * 0.95))] : NaN; };

// ---- stage C: the participant home of test/host/home ----
const onPath = (name) => (process.env.PATH ?? "").split(delimiter).map((d) => join(d, name)).find((f) => existsSync(f));
function wataneSrc() {
  if (process.env.WATANE_SRC) return resolve(process.env.WATANE_SRC);
  const d0 = onPath("watane-dev");
  if (!d0) return null;
  for (let d = dirname(realpathSync(d0)); d !== dirname(d); d = dirname(d)) if (existsSync(join(d, "host", "Cargo.toml"))) return d;
  return null;
}
function homeHarness() {
  const src = wataneSrc();
  if (!src) throw new NotRun("no watane checkout to link the home harness against: set WATANE_SRC");
  const target = join(src, "target");
  execFileSync("cargo", ["build", "-q", "--config", `patch."https://github.com/civic-tech-pioneer/watane".watane-host.path=${JSON.stringify(join(src, "host"))}`],
    { cwd: join(consoleDir, "test", "host", "home"), stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: target } });
  return join(target, "debug", "stage-c-home");
}
/** Starts the home; resolves with its ready line's pages; `stop()` ends it and waits until it exited. */
async function startHome(bin, home, claude, folders) {
  // S131 r1 F1.7: a host built with home/commit-clock.patch logs each state commit as `S128_COMMIT <start ns> <end ns>` on stderr
  // while S128_LOAD_CLOCK is set; a stock host logs none, and the writer's share is then not measured
  const child = spawn(bin, ["--home", home, "--fake-claude", claude, "--grant-all", ...folders],
    { stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, S128_LOAD_CLOCK: join(tmp, "commit-clock") } });
  const asked = [], commits = [];
  createInterface({ input: child.stderr }).on("line", (l) => {
    const m = /^S128_COMMIT (\d+) (\d+)$/.exec(l);
    if (m) commits.push([Number(BigInt(m[1]) / 1000n) / 1000, Number(BigInt(m[2]) / 1000n) / 1000]);
    else process.stderr.write(l + "\n");
  });
  const ready = await new Promise((ok, no) => {
    child.on("exit", (c) => no(new Error(`stage-c-home exited ${c}`)));
    createInterface({ input: child.stdout }).on("line", (l) => {
      let m; try { m = JSON.parse(l); } catch { return; }
      if (m.event === "error") no(new Error(`stage-c-home: ${m.message}`));
      else if (m.event === "asked") asked.push(m);
      else if (m.watane) ok(m);
    });
  });
  const pageOf = (local) => ready.mods.find((m) => m.local === local)?.page;
  return { home, agent: pageOf("agent"), client: pageOf("loadclient"), asked, commits,
    stop: async () => { child.stdin.end('{"op":"quit"}\n'); for (let k = 0; k < 150 && child.exitCode === null; k++) await sleep(100); if (child.exitCode === null) child.kill("SIGKILL"); } };
}

async function main() {
  for (const [p, a] of [["watane-dev", ["--version"]], ["cargo", ["--version"]], ["python3", ["--version"]]]) if (spawnSync(p, a).error) throw new NotRun(`${p} cannot run`);
  const harness = HOME ? homeHarness() : null;
  // build: the kernel's logic and page (as test:host does), the seed helper
  execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip2", "-p", "agent-mod"], { cwd: join(root, "kernel"), stdio: "inherit" });
  const tdir = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { cwd: join(root, "kernel"), encoding: "utf8" })).target_directory;
  cpSync(join(tdir, "wasm32-wasip2", "release", "agent_mod.wasm"), join(root, "kernel", "wasm", "logic.wasm"));
  execFileSync("npm", ["run", "build"], { cwd: consoleDir, stdio: "inherit" });
  const seedDir = join(consoleDir, "test", "host", "seed");
  execFileSync("cargo", ["build", "--release", "-q"], { cwd: seedDir, stdio: "inherit" });
  cpSync(join(root, "kernel", "wasm"), kernelDir, { recursive: true, filter: (s) => !s.includes("/target") });
  rmSync(join(kernelDir, "ui"), { recursive: true, force: true });
  cpSync(join(root, "ui"), join(kernelDir, "ui"), { recursive: true });
  cpSync(join(consoleDir, "test", "host", "loadclient"), clientDir, { recursive: true, filter: (s) => !s.includes("/target") });
  dev(["add", join(root, "provider")], kernelDir);
  dev(["add", kernelDir], clientDir);
  serve();
  // the second owner's access, written while the host is stopped (test-only: test/host/seed)
  await gone(stop());
  execFileSync(join(seedDir, "target", "release", "seed-state"), [preview.home, "local/agent", "access/local/loadclient", '["scratch"]'], { stdio: "inherit" });
  serve();
  let claudeDir, homeRun = null, folderOf = null;
  if (!HOME) claudeDir = wrapFake();
  else {
    // the mods the preview built, installed into a participant home (its app the fake Confirm); the second owner's access seeded there
    const real = readlinkSync(join(dirname(preview.home), "claude", "fake-claude"));
    const built = join(tmp, "built");
    cpSync(join(preview.home, "mods"), built, { recursive: true });
    spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", clientDir], { env, encoding: "utf8", timeout: 60_000 });
    claudeDir = join(tmp, "claude");
    mkdirSync(claudeDir, { recursive: true });
    wrapFake(join(claudeDir, "fake-claude"), real);
    const folders = ["claude-code", "agent", "loadclient"].map((m) => join(built, m));
    const home = join(tmp, "home-c");
    await (await startHome(harness, home, join(claudeDir, "fake-claude"), folders)).stop();
    execFileSync(join(seedDir, "target", "release", "seed-state"), [home, "local/agent", "access/local/loadclient", '["scratch"]'], { stdio: "inherit" });
    homeRun = await startHome(harness, home, join(claudeDir, "fake-claude"), []);
    preview = { home, agent: homeRun.agent, client: homeRun.client };
    // a folder new to each session: every launch asks (and the fake app grants) its consent
    folderOf = (name) => { const f = join(tmp, "folders", name.replace(/ /g, "-")); mkdirSync(f, { recursive: true }); return realpathSync(f); };
    say(`participant home ${home}: ${preview.agent}`);
  }

  // 8 sessions: each page session shown in its own Chromium page (the console opens the owner's newest session), the client's 4 unattached
  const browser = await launch();
  const pages = [], pageSessions = [], clientSessions = [];
  const turnOf = async (s) => { const r = await ok(preview.agent, "sessions.read", { session: s, after: 0, before: null, max: 512 }); const t = r.entries.map((e) => (typeof e === "string" ? JSON.parse(e) : e)).filter((e) => e.kind === "turn").pop(); return t?.body?.state; };
  const until = async (f, ms, what) => { const end = Date.now() + ms; for (;;) { const v = await f(); if (v) return v; if (Date.now() > end) throw new Error(`timed out waiting for ${what}`); await sleep(100); } };
  try {
    for (let k = 0; k < SESSIONS; k++) {
      const s = HOME
        ? await ok(preview.agent, "sessions.start-in-folder", { key: `load-page-${k}`, profile: "scratch", overlay: "{}", input: `load ${k}`, folder: folderOf(`load ${k}`) })
        : await ok(preview.agent, "sessions.start", { key: `load-page-${k}`, profile: "scratch", overlay: "{}", input: `load ${k}` });
      pageSessions.push(s);
      await until(async () => (await turnOf(s)) === "idle", 15_000, `session load ${k}'s first turn to end`);
      const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
      await page.addInitScript(PROBE);
      await page.goto(preview.agent);
      await until(async () => (await page.evaluate(() => window.__attached?.session ?? null)) === s, 15_000, `page ${k} to attach to its session`);
      pages.push(page);
      const t = HOME
        ? await post(preview.client, "api.start-in-folder", { key: `load-client-${k}`, input: `load client ${k}`, folder: folderOf(`load client ${k}`) })
        : await post(preview.client, "api.start", { key: `load-client-${k}`, input: `load client ${k}` });
      if (!t || !("ok" in t)) throw new Error(`the second owner's start was refused: ${JSON.stringify(t)}`);
      clientSessions.push(t.ok);
    }
    for (const s of clientSessions) await until(async () => { try { const r = await post(preview.client, "api.say", { session: s, key: `go-${s}`, input: "go" }); return r && "ok" in r; } catch { return false; } }, 15_000, "the client's go");
    for (const s of pageSessions) await ok(preview.agent, "sessions.say", { session: s, key: `go-${s}`, input: "go" });
    say(`8 sessions loading (page: ${pageSessions.length} in Chromium, local/loadclient: ${clientSessions.length}) for ${SECONDS - 2} s`);
    await sleep((SECONDS - 2) * 1000);
    // Stop: each page's own Stop button, timed in that page
    const stopAt = [];
    for (const page of pages) stopAt.push(await page.evaluate(() => { const b = document.querySelector(".wa-stop"); if (!b) return null; const at = performance.timeOrigin + performance.now(); b.click(); return at; }));
    for (const s of clientSessions) await post(preview.client, "api.interrupt", { session: s });
    await sleep(3000);

    const files = stamps(claudeDir);
    for (const k of [...Array(SESSIONS).keys()]) for (const name of [`load ${k}`, `load client ${k}`]) if (!files.has(name)) {
      const client = clientSessions[k] && await post(preview.client, "api.read", { session: clientSessions[k], after: 0, max: 50 }).catch((e) => e.message);
      throw new Error(`no stamped fake got the input ${name} (stamped: ${[...files.keys()].join(", ")}${homeRun ? `; asked: ${homeRun.asked.map((q) => `${q.origin} ${q.what}`).join(", ")}` : ""}; its session reads ${JSON.stringify(client).slice(0, 600)})`);
    }
    const names = [...Array(SESSIONS).keys()].flatMap((k) => [`load ${k}`, `load client ${k}`]);
    const pageStops = stopAt.filter((x) => x !== null);
    const win = loadWindow(new Map(names.map((n) => [n, files.get(n)])), pageStops.length ? Math.min(...pageStops) : -Infinity, RATE, WINDOW);
    const delays = [], stops = [], lines = [], faults = [...win.faults];
    if (!win.faults.length) say(`all 8 sources loaded through a common ${win.seconds.toFixed(1)} s before the first Stop: no gap over three times the spacing (60 ms deltas, 600 ms entries), the window's edges included`);
    for (let k = 0; k < SESSIONS; k++) {
      if (stopAt[k] === null) { faults.push(`page ${k} had no Stop button (its turn was not running)`); continue; }
      const attached = await pages[k].evaluate(() => window.__attached);
      const got = arrivals(await pages[k].evaluate(() => window.__arrivals));
      const foreign = got.filter((a) => a.stream !== attached.stream);
      if (foreign.length) faults.push(`page ${k}: ${foreign.length} items of another stream`);
      const file = files.get(`load ${k}`);
      const written = new Map(file.map((x) => [`${x.kind} ${x.n}`, x.at]));
      const seen = new Map();
      const mine = [];
      for (const a of got) if (a.kind !== "turn") {
        const key = `${a.kind} ${a.n}`, w = written.get(key);
        seen.set(key, (seen.get(key) ?? 0) + 1);
        if (w === undefined) faults.push(`page ${k}: ${key} arrived but its fake never wrote it`);
        else if (seen.get(key) === 1 && w >= win.from && w < win.to) mine.push(a.at - w); // only items written while all 8 loaded
      }
      const twice = [...seen].filter(([, c]) => c > 1).map(([key]) => key);
      if (twice.length) faults.push(`page ${k}: ${twice.length} items arrived more than once (${twice.slice(0, 3).join(", ")})`);
      const lost = file.filter((x) => x.at < stopAt[k] && !seen.has(`${x.kind} ${x.n}`));
      if (lost.length) faults.push(`page ${k}: ${lost.length} of the ${file.filter((x) => x.at < stopAt[k]).length} lines written before Stop never arrived (first: ${lost[0].kind} ${lost[0].n})`);
      if (mine.some((d) => d < -2)) faults.push(`page ${k}: an item arrived before its write`);
      delays.push(...mine);
      const late = mine.findIndex((d) => d > BUDGET.delay);
      lines.push(`  page ${k} (session ${pageSessions[k]}): ${file.length} lines written, ${mine.length} written in the common window arrived once; p95 ${p95(mine).toFixed(1)} ms, max ${Math.max(...mine).toFixed(1)} ms${late >= 0 ? `, first over ${BUDGET.delay} ms at item ${late + 1}` : ""}`);
      const end = got.find((a) => a.kind === "turn" && a.at >= stopAt[k] && isInterrupted(a.state));
      stops.push(end ? end.at - stopAt[k] : Infinity);
    }
    for (const l of lines) say(l);
    for (const f of faults) say(`FAULT ${f}`);
    const dp95 = p95(delays), sp95 = p95(stops);
    say(`item delay, the fake's write to the Chromium page: p95 ${dp95.toFixed(1)} ms over ${delays.length} items (budget ${BUDGET.delay}); max ${Math.max(...delays).toFixed(1)} ms`);
    say(`Stop in the page to idle {interrupted} in the page: p95 ${sp95.toFixed(1)} ms (budget ${BUDGET.stop}); each ${stops.map((x) => x.toFixed(1)).join(", ")} ms`);
    say(`machine: ${process.platform} ${cpus().length} cpus, load average ${loadavg().map((x) => x.toFixed(2)).join(" ")}`);
    let busy = null;
    if (HOME) {
      if (homeRun.commits.length) {
        busy = writerBusy(homeRun.commits, win.from, win.to);
        say(`writer busy share over the common window: ${busy.toFixed(1)}% (${homeRun.commits.filter(([a, b]) => b > win.from && a < win.to).length} commits; budget 50)`);
      } else say("writer busy share: not measured (a stock host logs no commits; build stage-c-home against a watane checkout with console/test/host/home/commit-clock.patch)");
      const consented = new Set(homeRun.asked.map((q) => q.what));
      say(`stage C: ${homeRun.asked.length} consent questions granted by the fake app, for ${consented.size} folders`);
      if (consented.size < 2 * SESSIONS) faults.push(`only ${consented.size} of the ${2 * SESSIONS} folders were asked for`);
    }
    if (faults.length) throw new Error(`${faults.length} load, completeness or identity faults`);
    if (!delays.length) throw new Error("no item was measured");
    return dp95 <= BUDGET.delay && sp95 <= BUDGET.stop && (busy === null || busy <= 50);
  } finally { await browser.close(); await homeRun?.stop(); }
}

let code = 0;
try { code = (await main()) ? 0 : 1; say(`stage ${STAGE}: ${code ? "OVER BUDGET" : "WITHIN BUDGET"}`); }
catch (e) { code = e instanceof NotRun ? 2 : 1; say(`${code === 2 ? "NOT RUN" : "FAIL"} ${COMMAND}: ${e.message}`); }
finally {
  try { spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", clientDir], { env, encoding: "utf8", timeout: 60_000 }); } catch {}
  try { rmSync(tmp, { recursive: true, force: true }); } catch {}
}
process.exit(code);
