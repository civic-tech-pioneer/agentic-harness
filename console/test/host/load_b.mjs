// S36-5: the stage B budgets (TECHNICAL 17, row B) end to end on watane's own host with dev's fake `claude`: 8 sessions, 4 for each of
// 2 owners (the console's `page`, and the test mod `loadclient` as `local/loadclient`), each on the fake's `load` with
// `rate {deltas: 50, entries: 5, bytes: 4096}`. Measures item delay from the fake's stdout write to the page connection (p95 over the
// page's 4 sessions, budget 100 ms) and `interrupt` to `idle {interrupted}` at the page (p95, budget 1 s). The state bytes per entry are
// the kernel crate's (kernel/tests/it/s36_load.rs). By hand: `npm run test:load` in console/. Exit 0 within budget, 1 over or failed,
// 2 not run (a prerequisite missing: watane-dev, fake-claude, cargo, python3).
//
// Two test-only reaches, each named where it is used: the second owner's `access/local/loadclient` is written into the stopped preview's
// kernel state by test/host/seed (no kernel call writes `access/`, 9.1), and the preview's fake-claude link is replaced by a wrapper that
// stamps each stdout line with its write time (the fake writes no time of its own).
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, readlinkSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import http from "node:http";
import net from "node:net";
import { tmpdir, loadavg, cpus } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

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
const BUDGET = { delay: 100, stop: 1000 };

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
const gone = async (pid) => { for (let k = 0; k < 150; k++) { try { process.kill(pid, 0); } catch { return; } await sleep(100); } throw new Error("the host did not stop"); };

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
/** A page connection to the kernel's origin: each SSE message with its arrival time. */
function events(url) {
  const u = new URL(url);
  const c = { msgs: [], buf: "", connection: null };
  let raw = Buffer.alloc(0), head = true;
  c.ready = new Promise((resolve, reject) => {
    const s = net.connect({ host: "127.0.0.1", port: Number(u.port) });
    c.sock = s;
    s.on("connect", () => s.write(`GET /events HTTP/1.1\r\nHost: ${u.host}\r\nAccept: text/event-stream\r\n\r\n`));
    s.on("data", (d) => {
      const at = Date.now();
      raw = Buffer.concat([raw, d]);
      if (head) { const i = raw.indexOf("\r\n\r\n"); if (i < 0) return; raw = raw.subarray(i + 4); head = false; }
      for (;;) {
        const i = raw.indexOf("\r\n"); if (i < 0) return;
        const n = parseInt(raw.subarray(0, i).toString(), 16);
        if (n === 0 || raw.length < i + 2 + n + 2) return;
        c.buf += raw.subarray(i + 2, i + 2 + n).toString("utf8"); raw = raw.subarray(i + 2 + n + 2);
        for (let j; (j = c.buf.indexOf("\n\n")) >= 0;) {
          const line = c.buf.slice(0, j).split("\n").find((l) => l.startsWith("data: ")); c.buf = c.buf.slice(j + 2);
          if (!line) continue;
          const m = JSON.parse(line.slice(6)); m.at = at; c.msgs.push(m);
          if (m.type === "connection") { c.connection = m.id; resolve(c); }
        }
      }
    });
    s.on("error", reject);
  });
  return c;
}
/** Each item of `stream` with its arrival: a text delta `delta N `, a finished `entry N`, or a turn. */
function arrivals(c, stream) {
  const out = [];
  for (const m of c.msgs) if (m.type === "stream" && m.stream === stream) for (const it of m.items) {
    let o; try { o = JSON.parse(it.body); } catch { continue; }
    const w = o._meta?.watane;
    const text = o.update?.content?.text ?? w?.entry?.body?.content?.[0]?.text;
    const d = /^delta (\d+) $/.exec(o.update?.sessionUpdate === "agent_message_chunk" ? text ?? "" : "");
    const e = /^entry (\d+) /.exec(w?.type === "entry" && w.entry?.kind === "message" ? text ?? "" : "");
    if (d) out.push({ kind: "delta", n: +d[1], at: m.at });
    else if (e) out.push({ kind: "entry", n: +e[1], at: m.at });
    else if (w?.type === "turn" || w?.entry?.kind === "turn") out.push({ kind: "turn", state: w.state ?? w.entry?.body, at: m.at });
  }
  return out;
}

// The stamping wrapper (test-only): the host starts <P>/claude/fake-claude; the wrapper runs the real fake with the same argv[0] (so it
// finds its script and log beside the link) and writes `<ns since epoch>\t<kind>\t<n>` per stdout line to stamps-<pid>.tsv.
const WRAPPER = `#!/usr/bin/python3 -I
import os, re, subprocess, sys, time, threading
real = REAL
me = os.path.abspath(sys.argv[0])
p = subprocess.Popen([me] + sys.argv[1:], executable=real, stdin=sys.stdin, stdout=subprocess.PIPE)
out = open(os.path.join(os.path.dirname(me), "stamps-%d.tsv" % os.getpid()), "w", buffering=1)
pat = re.compile(rb'"text":"(delta|entry) (\\d+) ')
for line in p.stdout:
    sys.stdout.buffer.write(line); sys.stdout.buffer.flush()
    m = pat.search(line[:400])
    if m: out.write("%d\\t%s\\t%s\\n" % (time.time_ns(), m.group(1).decode(), m.group(2).decode()))
sys.exit(p.wait())
`;
function wrapFake() {
  const link = join(dirname(preview.home), "claude", "fake-claude");
  const real = readlinkSync(link);
  unlinkSync(link);
  writeFileSync(link, WRAPPER.replace("REAL", JSON.stringify(real)));
  chmodSync(link, 0o755);
  writeFileSync(join(dirname(link), "fake-claude.json"), JSON.stringify({ steps: [{ do: "expect-user" }, { do: "load", rate: RATE }, { do: "expect-user" }] }));
  return dirname(link);
}
/** The stamp files in the order their fakes started (by the first stamp). */
function stamps(dir) {
  return readdirSync(dir).filter((f) => /^stamps-\d+\.tsv$/.test(f)).map((f) => readFileSync(join(dir, f), "utf8").split("\n").filter(Boolean).map((l) => { const [ns, kind, n] = l.split("\t"); return { at: Number(BigInt(ns) / 1_000_000n), kind, n: +n }; }))
    .filter((s) => s.length).sort((a, b) => a[0].at - b[0].at);
}
const p95 = (xs) => { const s = [...xs].sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.floor(s.length * 0.95))]; };

async function main() {
  for (const [p, a] of [["watane-dev", ["--version"]], ["cargo", ["--version"]], ["python3", ["--version"]]]) if (spawnSync(p, a).error) throw new NotRun(`${p} cannot run`);
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
  const claudeDir = wrapFake();

  // 8 sessions: the page's 4 attached on their own page connections, the client's 4 unattached
  const conns = [], streams = [], pageSessions = [], clientSessions = [];
  for (let k = 0; k < SESSIONS; k++) {
    const c = events(preview.agent); await c.ready; conns.push(c);
    const s = await ok(preview.agent, "sessions.start", { key: `load-page-${k}`, profile: "scratch", overlay: "{}", input: `load ${k}` });
    pageSessions.push(s);
    streams.push(await ok(preview.agent, "sessions.attach", { session: s, after: 0, caps: "{}" }, c.connection));
    await sleep(150); // the fakes start in this order, so stamp file k is session k
    const t = await post(preview.client, "api.start", { key: `load-client-${k}`, input: `load client ${k}` });
    if (!t || !("ok" in t)) throw new Error(`the second owner's start was refused: ${JSON.stringify(t)}`);
    clientSessions.push(t.ok);
    await sleep(150);
  }
  say(`8 sessions started (page: ${pageSessions.length}, local/loadclient: ${clientSessions.length}); loading for ${SECONDS - 2} s`);
  await sleep((SECONDS - 2) * 1000);
  // Stop: interrupt each page session and time its idle {interrupted} at the page connection
  const stopAt = [];
  for (const s of pageSessions) { stopAt.push(Date.now()); await ok(preview.agent, "sessions.interrupt", { session: s, replacement: null }); }
  for (const s of clientSessions) await post(preview.client, "api.interrupt", { session: s });
  await sleep(3000);

  const files = stamps(claudeDir);
  if (files.length !== 2 * SESSIONS) throw new Error(`${files.length} stamped fakes, expected ${2 * SESSIONS}`);
  const delays = [], stops = [], lines = [];
  for (let k = 0; k < SESSIONS; k++) {
    const written = new Map(files[2 * k].map((x) => [`${x.kind} ${x.n}`, x.at]));
    const got = arrivals(conns[k], streams[k]);
    let missing = 0, n = 0;
    for (const a of got) if (a.kind !== "turn") { const w = written.get(`${a.kind} ${a.n}`); if (w === undefined) missing++; else { delays.push(a.at - w); n++; } }
    lines.push(`  session ${k}: ${files[2 * k].length} lines written, ${n} matched at the page, ${missing} unmatched`);
    const end = got.find((a) => a.kind === "turn" && a.at >= stopAt[k] && JSON.stringify(a.state).includes("interrupted"));
    stops.push(end ? end.at - stopAt[k] : Infinity);
  }
  for (const l of lines) say(l);
  const dp95 = p95(delays), sp95 = p95(stops);
  say(`item delay source to page: p95 ${dp95} ms over ${delays.length} items (budget ${BUDGET.delay}); max ${Math.max(...delays)} ms`);
  say(`interrupt to idle {interrupted} at the page: p95 ${sp95} ms (budget ${BUDGET.stop}); each ${stops.join(", ")} ms`);
  say(`machine: ${process.platform} ${cpus().length} cpus, load average ${loadavg().map((x) => x.toFixed(2)).join(" ")}`);
  for (const c of conns) c.sock.destroy();
  return dp95 <= BUDGET.delay && sp95 <= BUDGET.stop;
}

let code = 0;
try { code = (await main()) ? 0 : 1; say(code ? "OVER BUDGET" : "WITHIN BUDGET"); }
catch (e) { code = e instanceof NotRun ? 2 : 1; say(`${code === 2 ? "NOT RUN" : "FAIL"} test:load: ${e.message}`); }
finally {
  try { spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", clientDir], { env, encoding: "utf8", timeout: 60_000 }); } catch {}
  try { rmSync(tmp, { recursive: true, force: true }); } catch {}
}
process.exit(code);
