// Slow: only a scripted Watane preview with the production component and Chromium
// can exercise the stage D host routing and the console together.
// Exit 2 for a missing prerequisite. An unsupported serve --script exits 1.
import { execFileSync, spawnSync } from "node:child_process";
import { once } from "node:events";
import { cpSync, existsSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import http from "node:http";
import https from "node:https";
import { cpus, loadavg, release, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";
import { NotRun, outcomeOf } from "./outcome.mjs";
import { isInterrupted, loadWindow } from "./load_check.mjs";
import { openaiCadence, p95, pageItem, pageProbe, writerBusy } from "./stage-d-tools/metrics.mjs";

const consoleDir = fileURLToPath(new URL("../..", import.meta.url));
const root = join(consoleDir, "..");
const wasmSrc = join(root, "kernel", "wasm");
const tmp = mkdtempSync(join(tmpdir(), "stage-d-"));
const wasmDir = join(tmp, "agent"); // a copy: dev rewrites the staged world
const script = join(tmp, "responses.json");
const clientDir = join(tmp, "loadclient");
const providerDir = join(tmp, "claude-code");
const tool = "loadclient_api_note";
const load = process.argv.includes("--load");
const command = load ? "test:load:d" : "test:host:d";
const clock = join(tmp, "load-clock");
const env = { ...process.env, WATANE_HOME: join(tmp, "home"), WATANE_DEV_CACHE: join(tmp, "cache") };
if (load) env.S128_LOAD_CLOCK = clock;
else delete env.S128_LOAD_CLOCK;
const vendor = join(process.env.WATANE_DEV_CACHE ?? join(process.env.HOME ?? "", ".cache", "watane-dev"), "vendor");
if (existsSync(vendor)) cpSync(vendor, join(tmp, "cache", "vendor"), { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (s) => console.log(s);

let pass = 0, fail = 0;
const notes = [];
async function check(name, f) {
  try { await f(); pass++; say(`PASS ${name}`); }
  catch (e) { fail++; say(`FAIL ${name}: ${e instanceof Error ? e.message : String(e)}`); }
}
function ensure(c, msg) { if (!c) throw new Error(typeof msg === "function" ? msg() : msg); }
const sse = (event) => ({ raw: `data: ${JSON.stringify(event)}\n\n` });
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

function dev(args, dir = clientDir) {
  const r = spawnSync("watane-dev", [...args, "--json", "--dir", dir], { env, encoding: "utf8", timeout: 600_000 });
  if (r.error) return { spawnError: r.error };
  let j; try { j = JSON.parse(r.stdout); } catch { j = null; }
  return { status: r.status, json: j, out: r.stdout, err: r.stderr };
}
function stopPreview() { try { spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", clientDir], { env, encoding: "utf8", timeout: 60_000 }); } catch {} }
let preview = null;
let redirectTarget;
function serve() {
  const r = dev(["serve", "--script", script]);
  if (r.spawnError) throw Object.assign(new Error(`watane-dev cannot run: ${r.spawnError.message}`), { notRun: true });
  const res = r.json?.result;
  if (r.status !== 0 || !res) {
    const e = r.json?.errors?.[0] ?? r.json?.error;
    const text = e ? `${e.code ?? ""} ${e.message ?? ""}`.trim() : (r.err || r.out).trim().split("\n")[0];
    throw Object.assign(new Error(text || `watane-dev serve exited ${r.status}`), { serveFailed: true, code: e?.code });
  }
  const page = res.pages.find((entry) => entry.slug === "agent" || entry.local === "agent");
  ensure(page, `missing agent page in ${JSON.stringify(res.pages)}`);
  const client = res.pages.find((entry) => entry.slug === "loadclient");
  ensure(client, "missing client page in the preview");
  preview = { url: page.url, clientUrl: client.url, home: res.home, log: res.log };
}
const zombie = (pid) => { try { return /^\d+ \(.*\) Z /.test(readFileSync(`/proc/${pid}/stat`, "utf8")); } catch { return false; } };
async function stopHost() {
  const pid = Number(JSON.parse(readFileSync(join(preview.home, "run.json"), "utf8")).pid);
  process.kill(pid, "SIGTERM");
  for (let k = 0; k < 150; k++) { try { process.kill(pid, 0); } catch { return; } if (zombie(pid)) return; await sleep(100); }
  throw new Error("the host did not stop");
}

function post(fn, args, url = preview.url) {
  const u = new URL(url);
  const body = JSON.stringify({ fn, args });
  return new Promise((resolve, reject) => {
    const req = http.request({ host: "127.0.0.1", port: Number(u.port), path: "/call", method: "POST", agent: false,
      headers: { Host: u.host, Origin: `http://${u.host}`, "content-type": "application/json", "content-length": Buffer.byteLength(body) } }, (res) => {
      let s = ""; res.setEncoding("utf8"); res.on("data", (c) => (s += c));
      res.on("end", () => { try { const j = JSON.parse(s); if ("value" in j) resolve(j.value); else reject(new Error(`${fn}: ${JSON.stringify(j.error ?? j)}`)); } catch { reject(new Error(`${fn}: HTTP ${res.statusCode} ${s.slice(0, 200)}`)); } });
    });
    req.setTimeout(20_000, () => req.destroy(new Error(`${fn}: no answer in 20 s`)));
    req.on("error", reject); req.end(body);
  });
}
async function ok(fn, args, url) {
  const v = await post(fn, args, url);
  if (v && typeof v === "object" && "ok" in v) return v.ok;
  throw new Error(`${fn} refused: ${JSON.stringify(v)}`);
}
async function refused(fn, args) {
  const v = await post(fn, args);
  if (v && typeof v === "object" && "err" in v) return v.err;
  throw new Error(`${fn} was not refused: ${JSON.stringify(v).slice(0, 200)}`);
}
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
async function until(f, ms, what) { const t = Date.now(); for (;;) { const v = await f(); if (v) return v; if (Date.now() - t > ms) throw new Error(`timed out after ${ms} ms: ${what}`); await sleep(50); } }
const fields = (rs) => rs.map((r) => r.field);
const keyOf = () => `host-d-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const TOOL = { backend: "openai", model: "gpt-5", toolsets: ["stage-d"], allow: [tool] };
const TIDY = { backend: "openai", model: "gpt-5", tools: ["Read", "Edit"], allow: ["Read(/tmp/**)"] };
const SORT = { backend: "openai", model: "gpt-5" };

function seed(key, value) {
  execFileSync(join(consoleDir, "test", "host", "seed", "target", "release", "seed-state"), [preview.home, "local/agent", key, value], { stdio: "inherit" });
}

// The dev fake emits a fixed Claude session id. Watane binds that id to
// its first owner, so give each fake process its own id for the two-owner run.
function wrapLoadClaude(folder) {
  const link = join(folder, "fake-claude");
  const real = join(folder, "scripted-claude");
  renameSync(link, real);
  // the rewrite is stage-d-tools/stamp_claude.py's, which metrics.test.ts checks (S131 P2.2)
  writeFileSync(link, `#!/usr/bin/env python3
import subprocess
import sys
import time
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, ${JSON.stringify(join(consoleDir, "test", "host", "stage-d-tools"))})
from stamp_claude import rewrite

child = subprocess.Popen([sys.argv[0], *sys.argv[1:]], executable=${JSON.stringify(real)}, stdout=subprocess.PIPE)
identity = uuid.uuid4().hex.encode()
with open(${JSON.stringify(join(tmp, "claude.stamps"))}, "ab", buffering=0) as stamps:
    for line in child.stdout:
        line, marker = rewrite(line, identity)
        at = time.time_ns()
        sys.stdout.buffer.write(line)
        sys.stdout.buffer.flush()
        if marker:
            stamps.write(str(at).encode() + b" " + marker + b"\\n")
sys.exit(child.wait())
`, { mode: 0o755 });
}

function readStamps(path) {
  ensure(existsSync(path), `missing telemetry: ${path}; see AGENTS.md for the instrumented Watane build`);
  return readFileSync(path, "utf8").trim().split("\n").filter(Boolean).map((line) => {
    const [at, marker, step] = line.split(" ");
    return { at: Number(at) / 1e6, marker, step };
  });
}
const stampTimes = (stamps) => new Map(stamps.map(({ at, marker }) => [marker, at]));

async function mixedLoad(browser) {
  const folder = join(dirname(preview.home), "claude");
  const fake = join(folder, "fake-claude.json");
  ensure(existsSync(fake), "the preview needs watane-dev's fake-claude");
  wrapLoadClaude(folder);
  const rate = { deltas: 50, entries: 5, bytes: 4096, seconds: 20 };
  writeFileSync(fake, JSON.stringify({ steps: [
    { do: "expect-user" }, { do: "message", text: "ready" }, { do: "result" }, { do: "idle" },
    { do: "expect-user" }, { do: "load", rate }, { do: "idle" },
  ] }));
  const sessions = [];
  async function startBatch(backend) {
    // Opening one page at a time lets the real console select the newest session.
    for (let index = 0; index < 8; index++) {
      const own = index < 4;
      const profile = backend === "openai" ? "Tools" : "scratch";
      const args = { key: keyOf(), input: `load ${backend} ${index}` };
      const id = own
        ? await ok("sessions.start", { ...args, profile, overlay: "{}" })
        : await ok(backend === "openai" ? "api.start-openai" : "api.start", args, preview.clientUrl);
      const session = { id, own, backend };
      sessions.push(session);
      if (own) {
        const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
        session.page = page;
        await page.addInitScript(pageProbe);
        await page.goto(preview.url);
        await page.waitForFunction((id) => window.loadAttach?.id === id, id, { timeout: 15_000 });
      }
    }
  }
  async function entriesOf({ id, own }) {
    const args = { session: id, after: 0, max: 512 };
    const entries = own
      ? (await ok("sessions.read", { ...args, before: null })).entries
      : await ok("api.read", args, preview.clientUrl);
    return entries.map((entry) => typeof entry === "string" ? JSON.parse(entry) : entry);
  }
  try {
    await startBatch("claude");
    const claude = sessions.slice();
    await Promise.all(claude.map((session) => until(async () => (
      (await entriesOf(session)).filter((entry) => entry.kind === "turn").at(-1)?.body.stop?.kind === "completed"
    ), 20_000, `Claude session ${session.id} to finish its warm-up turn`)));
    await startBatch("openai");

    const start = Date.now() + 700;
    const openai = sessions.slice(claude.length);
    const schedule = Object.fromEntries(openai.map(({ id }, index) =>
      [id, start + index % 4 * 250 + Math.floor(index / 4) * 125]));
    writeFileSync(clock, JSON.stringify(schedule));
    await Promise.all(claude.map(async ({ id, own }, index) => {
      await sleep(index * 2);
      const args = { session: id, key: keyOf(), input: "begin load" };
      return own ? ok("sessions.say", args) : ok("api.say", args, preview.clientUrl);
    }));
    await sleep(Math.max(0, start + 8050 - Date.now()));
    const pages = sessions.filter((session) => session.page);
    const stopAt = await Promise.all(pages.map(({ page }) => page.evaluate(() => {
      const button = document.querySelector(".wa-stop");
      const at = performance.timeOrigin + performance.now();
      button?.click();
      return button ? at : null;
    })));
    const to = Math.min(...stopAt.map((at) => at ?? Infinity), Date.now());
    await sleep(1250);
    const wire = readStamps(join(tmp, "load-clock.sse"));
    const scripted = stampTimes(wire.filter(({ marker }) => !marker.endsWith("/completed")));
    const claudeStamps = stampTimes(readStamps(join(tmp, "claude.stamps")));
    const claudeByProcess = new Map();
    for (const [marker, at] of claudeStamps) {
      const [, id, kind] = marker.split("-");
      if (!claudeByProcess.has(id)) claudeByProcess.set(id, []);
      claudeByProcess.get(id).push({ at, kind });
    }
    ensure(claudeByProcess.size === 8, `expected eight Claude sources, found ${claudeByProcess.size}`);
    const window = loadWindow(claudeByProcess, to, rate, 6);
    const from = Math.max(window.from, start + 1000);
    const faults = [...window.faults];
    const delays = [], stops = [];
    const claudeSeen = new Set();
    const aiDelays = [], claudeDelays = [];

    for (const [index, session] of pages.entries()) {
      const { id, backend, page } = session;
      const captured = await page.evaluate(() => ({ items: window.loadItems, stream: window.loadAttach.stream }));
      const seen = new Set();
      const items = captured.items.filter((item) => item.stream === captured.stream).map(pageItem);
      const end = stopAt[index];
      const idle = end && items.find(({ at, turn }) => at >= end && isInterrupted(turn));
      stops.push(idle ? idle.at - end : Infinity);
      if (!idle) faults.push(`${id}: no page Stop to interrupted idle (last turn ${JSON.stringify(items.filter(({ turn }) => turn).at(-1)?.turn)})`);
      const markers = new Set(items.map(({ marker }) => marker).filter(Boolean));
      const claudeIds = new Set([...markers].filter((m) => m.startsWith("cl-")).map((m) => m.split("-")[1]));
      if (backend === "claude") {
        if (claudeIds.size !== 1) faults.push(`${id}: expected one Claude source, got ${claudeIds.size}`);
        for (const source of claudeIds) {
          if (claudeSeen.has(source)) faults.push(`${id}: Claude source on two pages`);
          claudeSeen.add(source);
        }
      }
      for (const { at, marker } of items) {
        const sent = claudeStamps.get(marker) ?? scripted.get(marker);
        if (!(sent >= from && sent < to)) continue;
        if (seen.has(marker)) { faults.push(`${id}: duplicate ${marker}`); continue; }
        seen.add(marker);
        if (at < sent - 2) faults.push(`${id}: ${marker} arrived before its write`);
        delays.push(at - sent);
        (backend === "claude" ? claudeDelays : aiDelays).push(at - sent);
      }
      const expected = backend === "claude"
        ? [...claudeStamps].filter(([m]) => claudeIds.has(m.split("-")[1]))
        : [...scripted].filter(([m]) => m.startsWith(`oa-${id}-`));
      const missing = expected.filter(([marker, at]) => at >= from && at < to && !seen.has(marker));
      if (missing.length) faults.push(`${id}: ${missing.length} source items missing from page`);
      if (backend === "openai") {
        const fullSteps = expected.filter(([, at]) => at >= from && at < to - 50).length;
        const delivered = [...seen].filter((marker) => marker.startsWith("call_")).length;
        if (delivered < 4 * fullSteps) faults.push(`${id}: only ${delivered} tool calls reached the page across ${fullSteps} steps`);
        if (missing.length || delivered < 4 * fullSteps) {
          const offsets = ([marker, at]) => ({ marker, beforeStopMs: end === null ? null : Math.round(end - at) });
          const measuredSteps = new Set(expected.filter(([, at]) => at >= from && at < to - 50).map(([marker]) => marker));
          const missingSteps = new Set(missing.map(([marker]) => marker));
          for (const { at, marker, step } of wire) {
            if (at >= from && at < to && measuredSteps.has(step) && marker.startsWith("call_") && !seen.has(marker)) {
              missingSteps.add(step);
            }
          }
          const stored = await entriesOf(session);
          const lastToolCalls = stored.filter(({ kind }) => kind === "tool").slice(-8);
          say(`LOSS ${id} ${JSON.stringify({
            window: { from, to },
            stopAt: end,
            interruptedAt: idle?.at,
            missing: missing.map(offsets),
            // A different attachment may already have received items that the measured stream lacks.
            streams: [...new Set(captured.items.map(({ stream }) => stream))],
            measuredStream: captured.stream,
            missingStepWrites: wire.filter(({ step }) => missingSteps.has(step))
              .map(({ marker, at, step }) => ({ step, ...offsets([marker, at]) })),
            lastPageTools: items.filter(({ marker }) => marker?.startsWith("call_")).slice(-8),
            lastStoredTools: lastToolCalls.map(({ i, at, body }) => ({ i, at, id: body.toolCallId, status: body.status })),
            lastStoredTurn: stored.findLast(({ kind }) => kind === "turn"),
          })}`);
        }
      }
      if (!seen.size) faults.push(`${id}: no page measurements in the common window`);
    }

    let paced = 0;
    for (const session of sessions.filter(({ backend }) => backend === "openai")) {
      const stamps = [...scripted].filter(([marker]) => marker.startsWith(`oa-${session.id}-`)).map(([, at]) => at);
      if (to - from >= 6000 && openaiCadence(stamps, from, to)) paced++;
      else {
        faults.push(`${session.id}: OpenAI cadence missed one step per second (${stamps.length} steps)`);
      }
      const entries = await entriesOf(session);
      const completed = entries.filter((entry) => entry.kind === "tool" && entry.body.status === "completed").length;
      const fullSteps = stamps.filter((at) => at >= from && at < to - 1000).length;
      if (completed < 4 * fullSteps) faults.push(`${session.id}: only ${completed} tools completed across ${fullSteps} measured steps`);
    }
    const commits = [...readFileSync(preview.log, "utf8").matchAll(/^S128_COMMIT (\d+) (\d+)$/gm)]
      .map(([, start, end]) => [Number(start) / 1e6, Number(end) / 1e6]);
    const busy = writerBusy(commits, from, to);
    const delay = p95(delays), stop = p95(stops);
    say(`Budget record Stage D | ${new Date().toISOString().slice(0, 10)} | ${process.platform} ${release()} ${process.arch} ${cpus().length} CPUs, load ${loadavg().map((n) => n.toFixed(2)).join("/")} | common window ${((to - from) / 1000).toFixed(2)} s | item p95 ${delay.toFixed(1)} ms (${delays.length} samples, Claude ${p95(claudeDelays).toFixed(1)}, OpenAI ${p95(aiDelays).toFixed(1)}) | Stop p95 ${stop.toFixed(1)} ms | writer busy ${busy.toFixed(1)}% | cadence ${paced}/8 OpenAI sessions at one step a second`);
    for (const fault of faults.slice(0, 20)) say(`FAULT ${fault}`);
    ensure(!faults.length && delay <= 100 && stop <= 1000 && busy <= 60,
      `over budget or invalid load (${faults.length} workload faults)`);
  } finally {
    await Promise.allSettled(sessions.map(({ id, own }) => own
      ? post("sessions.interrupt", { session: id, replacement: null })
      : post("api.interrupt", { session: id }, preview.clientUrl)));
  }
}

async function main() {
  if (spawnSync("watane-dev", ["--version"], { encoding: "utf8" }).error) throw new NotRun("watane-dev cannot run: not on PATH");
  if (spawnSync("openssl", ["version"], { encoding: "utf8" }).error) throw new NotRun("openssl cannot run: not on PATH (it makes the redirect check's https target)");
  if (!existsSync(join(wasmSrc, "wit"))) throw new Error("kernel/wasm/wit is missing");
  const have = spawnSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" });
  if (!have.error && !/^wasm32-wasip2$/m.test(have.stdout)) execFileSync("rustup", ["target", "add", "wasm32-wasip2"], { stdio: "inherit" });
  execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip2", "-p", "agent-mod"], { cwd: join(root, "kernel"), stdio: "inherit" });
  const tdir = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { cwd: join(root, "kernel"), encoding: "utf8" })).target_directory;
  execFileSync("npm", ["run", "build"], { cwd: consoleDir, stdio: "inherit" });
  execFileSync("cargo", ["build", "--release", "-q"], { cwd: join(consoleDir, "test", "host", "seed"), stdio: "inherit" });
  cpSync(wasmSrc, wasmDir, { recursive: true, filter: (path) =>
    path !== join(wasmSrc, "ui") && path !== join(wasmSrc, "logic.wasm") && !path.includes("/target"),
  });
  cpSync(join(tdir, "wasm32-wasip2", "release", "agent_mod.wasm"), join(wasmDir, "logic.wasm"));
  cpSync(join(root, "ui"), join(wasmDir, "ui"), { recursive: true });

  // Dev stages the linked provider and rewrites its component. Keep it local
  // to this preview so another host test can build the same provider.
  cpSync(join(root, "provider"), providerDir, {
    recursive: true, filter: (path) => path !== join(root, "provider", "logic.wasm") && !path.includes("/target"),
  });
  const added = dev(["add", providerDir], wasmDir);
  if (added.spawnError || added.status !== 0) {
    const e = added.json?.errors?.[0] ?? added.json?.error;
    throw Object.assign(new Error(`watane-dev add provider: ${e ? `${e.code ?? ""} ${e.message ?? ""}` : (added.err || added.out).trim().split("\n")[0]}`), { addFailed: true, code: e?.code, out: added.out, err: added.err });
  }

  cpSync(join(consoleDir, "test", "host", "loadclient"), clientDir, {
    recursive: true, filter: (path) => !path.includes("/target"),
  });
  const clientLink = dev(["add", wasmDir]);
  ensure(clientLink.status === 0, `watane-dev add agent: ${clientLink.err || clientLink.out}`);

  const reply = (text) => ({ sse: [
    { delta: text },
    sse({ type: "response.output_item.done", item: {
      type: "message", id: "msg_1", role: "assistant",
      content: [{ type: "output_text", text }],
    } }),
    { completed: { input_tokens: 12_000, output_tokens: 300 } },
  ] });
  // Always require --script. An unsupported preview fails before a session starts.
  const toolEvent = {
    type: "response.output_item.done",
    item: { type: "function_call", id: "fc_note", call_id: "call_note", name: tool, arguments: "{}" },
  };
  const toolReply = { sse: [
    sse(toolEvent),
    { completed: { input_tokens: 100, output_tokens: 20 } },
  ] };
  // S131 P1.5: an https target, so that zero connections means the host did not follow the 307, not that it
  // refuses plain http; a connection counts even when the TLS handshake then fails on this self-signed name
  let redirectsFollowed = 0;
  execFileSync("openssl", ["req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-nodes", "-days", "1",
    "-subj", "/CN=127.0.0.1", "-addext", "subjectAltName=IP:127.0.0.1", "-keyout", join(tmp, "redirect.key"), "-out", join(tmp, "redirect.crt")], { stdio: "ignore" });
  redirectTarget = https.createServer({ key: readFileSync(join(tmp, "redirect.key")), cert: readFileSync(join(tmp, "redirect.crt")) }, (_request, response) => {
    response.writeHead(500).end();
  });
  redirectTarget.on("connection", () => redirectsFollowed++);
  redirectTarget.listen(0, "127.0.0.1");
  await once(redirectTarget, "listening");
  const loadReplies = () => Array.from({ length: 128 }, (_, step) => ({ sse: [
    ...Array.from({ length: 4 }, (_, index) => sse({
      ...toolEvent,
      item: { ...toolEvent.item, id: `fc_${step}_${index}`, call_id: `call_${step}_${index}` },
    })),
    { completed: { input_tokens: 100, output_tokens: 20 } },
  ] }));
  writeFileSync(script, JSON.stringify({ replies: load ? [...loadReplies(), "stall"] : [
    toolReply,
    reply("I sorted the notes."),
    { "usage-limit": { resets_in_s: 5 } },
    reply("I continued after the limit."),
    { redirect: `https://127.0.0.1:${redirectTarget.address().port}/never` },
  ] }));

  let browser;
  try { browser = await launch(); }
  catch (e) { throw new NotRun(`Chromium cannot start: ${e.message.split("\n")[0]}`); }
  try {
    // serve once so the host has its state, stop it, write the profiles (no kernel call writes `access/`, 9.1), serve again
    try { serve(); } catch (e) { if (e.serveFailed && /allowlist/.test(`${e.code} ${e.message}`) && !existsSync(vendor)) e.notRun = true; throw e; }
    await stopHost();
    // Pre-grant only the fixture tool in this disposable home. Live approval is
    // deliberately unavailable in preview mode.
    writeFileSync(join(preview.home, "toolsets.json"), JSON.stringify({ "stage-d": ["loadclient:api.note"] }));
    const catalogPath = join(preview.home, "installed.json");
    const catalog = JSON.parse(readFileSync(catalogPath, "utf8"));
    const agent = catalog.mods.find((mod) => mod.identity === "local/agent");
    ensure(agent, "missing kernel in the preview catalog");
    agent.consents = [...(agent.consents ?? []), { kind: "toolset", toolset: "stage-d", functions: [tool] }];
    if (load) {
      const client = catalog.mods.find((mod) => mod.identity === "local/loadclient");
      ensure(client, "missing second owner in the preview catalog");
      client.consents = [...(client.consents ?? []), { kind: "toolset", toolset: "stage-d", functions: [tool] }];
      seed("access/local/loadclient", JSON.stringify(["scratch", "Tools"]));
    }
    writeFileSync(catalogPath, JSON.stringify(catalog));
    seed("access/page", JSON.stringify(["scratch", "Sort", "Tidy", "Tools"]));
    seed("profile/Tools", JSON.stringify(TOOL));
    seed("profile/Sort", JSON.stringify(SORT));
    seed("profile/Tidy", JSON.stringify(TIDY));
    serve();
    say(`served page: ${preview.url}`);
    say(`preview home: ${preview.home}`);

    if (load) {
      await check("mixed load: eight Claude streams and eight four-tool OpenAI sessions across two owners", () => mixedLoad(browser));
      return;
    }

    // AC2: the profile list and each refusal named
    await check("profiles: the kernel lists scratch and the three OpenAI profiles in order with their backends", async () => {
      const ps = await post("sessions.profiles", {});
      ensure(same(ps.map((p) => p.name), ["scratch", "Sort", "Tidy", "Tools"]), `names ${JSON.stringify(ps)}`);
      ensure(same(ps.map((p) => p.backend), ["claude", "openai", "openai", "openai"]), `backends ${JSON.stringify(ps)}`);
    });
    await check("check: Tidy is refused naming tools and allow, each once; Sort and scratch name none", async () => {
      const t = await post("sessions.check", { profile: "Tidy", overlay: "{}" });
      ensure(same(fields(t), ["tools", "allow"]), `Tidy refusals ${JSON.stringify(t)}`); // each field once, in the kernel's order (S40 r1 P1.2)
      ensure(t.every((r) => r.backend === "openai" && r.reason), `refusals without backend or reason ${JSON.stringify(t)}`);
      const s = await post("sessions.check", { profile: "Sort", overlay: "{}" });
      ensure(s.length === 0, `Sort refusals ${JSON.stringify(s)}`);
    });
    await check("start-in-folder: Sort with a folder is refused naming folder", async () => {
      const e = await refused("sessions.start-in-folder", { key: keyOf(), profile: "Sort", overlay: "{}", input: "go", folder: "/tmp" });
      const rs = e.refused ?? e;
      ensure(Array.isArray(rs) && fields(rs).includes("folder"), `the refusal was ${JSON.stringify(e)}`);
    });

    // AC3: the page
    await check("page: the new-session view lists the four profiles; Tidy shows Can't run on ChatGPT with each field and Start off; Sort turns Start on", async () => {
      const page = await browser.newPage({ viewport: { width: 1280, height: 820 } });
      page.on("pageerror", (e) => notes.push(`pageerror: ${e.message}`));
      try {
        await page.goto(preview.url);
        await page.locator("label.wa-pcard", { hasText: "Sort" }).waitFor({ timeout: 15_000 });
        const rows = await page.locator("label.wa-pcard b").allTextContents();
        ensure(same(rows, ["scratch", "Sort", "Tidy", "Tools"]), `rows ${JSON.stringify(rows)}`);
        await page.getByRole("textbox", { name: "Message" }).fill("tidy up");
        await page.locator("label.wa-pcard", { hasText: "Tidy" }).click();
        await page.getByText("Can't run on ChatGPT").waitFor({ timeout: 10_000 });
        const refs = await page.locator(".wa-refusals .wa-ref code").allTextContents();
        ensure(same([...new Set(refs)].sort(), ["allow", "tools"]), `fields shown ${JSON.stringify(refs)}`);
        ensure(await page.getByRole("button", { name: "Start session" }).isDisabled(), "Start is on for Tidy");
        await page.locator("label.wa-pcard", { hasText: "Sort" }).click();
        await page.waitForFunction(() => !document.querySelector(".wa-refusals")?.textContent, null, { timeout: 10_000 });
        ensure(await page.getByRole("button", { name: "Start session" }).isEnabled(), "Start is off for Sort");
      } finally { await page.close(); }
    });

    {
      const page = await browser.newPage({ viewport: { width: 1280, height: 820 } });
      page.on("pageerror", (e) => notes.push(`pageerror: ${e.message}`));
      await page.addInitScript(() => {
        let bridge;
        window.scriptedDeltas = [];
        Object.defineProperty(window, "watane", {
          configurable: true,
          get: () => bridge,
          set: (real) => {
            bridge = { ...real, on(handler) {
              return real.on((event) => {
                if (event?.type === "stream") {
                  for (const item of event.items ?? []) {
                    const update = JSON.parse(item.body);
                    if (update.sessionUpdate === "agent_message_chunk" && update._meta?.watane?.i == null) {
                      window.scriptedDeltas.push(update.content?.text);
                    }
                  }
                }
                handler(event);
              });
            } };
          },
        });
      });
      try {
        await page.goto(preview.url);
        await page.locator("label.wa-pcard", { hasText: "Tools" }).click();
        const message = page.getByRole("textbox", { name: "Message" });
        await message.fill("sort my notes");
        await page.getByRole("button", { name: "Start session" }).click();

        await check("reply: the scripted Responses text reaches the console", async () => {
          await page.getByText("ChatGPT · Tools").first().waitFor({ timeout: 15_000 });
          await page.getByText("I sorted the notes.", { exact: true }).waitFor({ timeout: 15_000 });
          const listed = await ok("sessions.list", { after: null, max: 256 });
          ensure(listed.length === 1 && listed[0].backend === "openai", `listed: ${JSON.stringify(listed)}`);
        });
        await check("tool: the fixture tool completes through the preview toolset", async () => {
          const [session] = await ok("sessions.list", { after: null, max: 256 });
          const entries = await transcript(session.id);
          ensure(entries.some((entry) => entry.kind === "tool" &&
            entry.body.status === "completed" && JSON.stringify(entry.body).includes("preview tool result")),
          `tool and turn entries: ${JSON.stringify(entries.filter((entry) => ["tool", "turn"].includes(entry.kind)))}`);
        });
        await check("meter: context used reflects the scripted usage", async () => {
          const meter = page.getByRole("meter", { name: "Context used" });
          await meter.waitFor({ timeout: 10_000 });
          ensure(await meter.getAttribute("aria-valuenow") === "12000", "meter should show 12000 input tokens");
        });

        await check("usage: the page pauses until the reset and resumes without another input", async () => {
          await page.locator(".wa-status-word", { hasText: "Idle" }).waitFor({ timeout: 15_000 });
          await message.fill("continue sorting");
          await page.locator(".wa-send").click();
          // S131 P1.4: a time follows "Paused until", and the pause lasts until the scripted reset (resets_in_s: 5)
          const paused = page.locator(".wa-status-word", { hasText: /^Paused until / });
          await paused.waitFor({ timeout: 10_000 });
          const pausedAt = Date.now();
          const word = await paused.textContent();
          ensure(/^Paused until \S*\d/.test(word ?? ""), `the pause names no time: ${JSON.stringify(word)}`);
          await page.getByText("I continued after the limit.", { exact: true }).waitFor({ timeout: 20_000 });
          const pausedFor = Date.now() - pausedAt;
          ensure(pausedFor >= 4000, `the turn resumed ${pausedFor} ms after "${word}", before the 5 s reset`);
          say(`  paused ${pausedFor} ms: ${word}`);
          ensure(await page.evaluate(() => window.scriptedDeltas.includes("I continued after the limit.")),
            "the resumed reply did not reach the page as a live text chunk");
          await page.locator(".wa-status-word", { hasText: "Idle" }).waitFor({ timeout: 10_000 });
        });

        await check("redirect: the scripted 307 ends the turn as failed", async () => {
          const id = await ok("sessions.start", { key: keyOf(), profile: "Sort", overlay: "{}", input: "redirect check" });
          await until(async () => (await transcript(id)).some((e) =>
            e.kind === "turn" && /failed/.test(JSON.stringify(e.body)) && /307|redirect/i.test(JSON.stringify(e.body))),
          10_000, "an idle failed turn naming the redirect");
          ensure(redirectsFollowed === 0, "a request reached the redirect target");
        });
      } finally { await page.close(); }
    }
  } finally { await browser?.close(); }
}

let code = 0;
try { await main(); }
catch (e) {
  const o = outcomeOf(e);
  if (o.code === 2) code = 2; else fail++;
  say(o.line.replace("test:host", command));
} finally {
  stopPreview();
  if (redirectTarget?.listening) await new Promise((resolve) => redirectTarget.close(resolve));
  try { rmSync(tmp, { recursive: true, force: true }); rmSync(wasmDir, { recursive: true, force: true }); } catch {}
}
if (code !== 2) {
  for (const n of notes) say(`note: ${n}`);
  say(`stage D on the preview: ${pass} pass, ${fail} fail`);
  code = fail ? 1 : 0;
}
process.exit(code);
