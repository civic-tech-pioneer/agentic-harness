// Slow: only a scripted Watane preview with the production component and Chromium
// can exercise the stage D host routing and the console together.
import { execFileSync, spawnSync } from "node:child_process";
import { once } from "node:events";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import http from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";
import { NotRun, outcomeOf } from "./outcome.mjs";

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
const env = { ...process.env, WATANE_HOME: join(tmp, "home"), WATANE_DEV_CACHE: join(tmp, "cache") };
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
  preview = { url: page.url, clientUrl: client.url, home: res.home };
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

// Exercise both owner caps with both backends active in one preview. The
// separately metered test in the kernel checks the state I/O half of the budget.
async function mixedLoad() {
  const fake = join(dirname(preview.home), "claude", "fake-claude.json");
  ensure(existsSync(fake), "the preview needs watane-dev's fake-claude");
  writeFileSync(fake, JSON.stringify({ steps: [
    { do: "expect-user" }, { do: "message", text: "ready" }, { do: "result" }, { do: "idle" },
    { do: "expect-user" }, { do: "load", rate: { deltas: 50, entries: 5, bytes: 4096, seconds: 20 } },
    { do: "idle" },
  ] }));
  const sessions = [];
  async function startBatch(backend) {
    const results = await Promise.allSettled(Array.from({ length: 8 }, async (_, index) => {
      const own = index < 4;
      const profile = backend === "openai" ? "Tools" : "scratch";
      const args = { key: keyOf(), input: `load ${backend} ${index}` };
      const id = own
        ? await ok("sessions.start", { ...args, profile, overlay: "{}" })
        : await ok(backend === "openai" ? "api.start-openai" : "api.start", args, preview.clientUrl);
      const session = { id, own, backend };
      sessions.push(session);
      return session;
    }));
    return results.map((result) => {
      if (result.status === "rejected") throw result.reason;
      return result.value;
    });
  }
  async function entriesOf({ id, own }) {
    const args = { session: id, after: 0, max: 40 };
    const entries = own
      ? (await ok("sessions.read", { ...args, before: null })).entries
      : await ok("api.read", args, preview.clientUrl);
    return entries.map((entry) => typeof entry === "string" ? JSON.parse(entry) : entry);
  }
  try {
    const claude = await startBatch("claude");
    await Promise.all(claude.map((session) => until(async () => (
      (await entriesOf(session)).filter((entry) => entry.kind === "turn").at(-1)?.body.state === "idle"
    ), 20_000, `Claude session ${session.id} to finish its warm-up turn`)));
    await Promise.all(claude.map(({ id, own }) => {
      const args = { session: id, key: keyOf(), input: "begin load" };
      return own ? ok("sessions.say", args) : ok("api.say", args, preview.clientUrl);
    }));
    await startBatch("openai");
    await Promise.all(sessions.map((session) => until(async () => {
      const entries = await entriesOf(session);
      return session.backend === "openai"
        ? entries.filter((entry) => entry.kind === "tool" && entry.body.status === "completed").length >= 4
        : entries.some((entry) => entry.kind === "message" && entry.body.content?.[0]?.text?.startsWith("entry "));
    }, 30_000, `${session.backend} session ${session.id} to produce entries under mixed load`)));
  } finally {
    await Promise.all(sessions.map(({ id, own }) => own
      ? ok("sessions.interrupt", { session: id, replacement: null })
      : ok("api.interrupt", { session: id }, preview.clientUrl)));
  }
}

async function main() {
  if (spawnSync("watane-dev", ["--version"], { encoding: "utf8" }).error) throw new NotRun("watane-dev cannot run: not on PATH");
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
  let redirectsFollowed = 0;
  redirectTarget = http.createServer((_request, response) => {
    redirectsFollowed++;
    response.writeHead(500).end();
  });
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
    { redirect: `http://127.0.0.1:${redirectTarget.address().port}/never` },
  ] }));

  let browser;
  if (!load) {
    try { browser = await launch(); }
    catch (e) { throw new NotRun(`Chromium cannot start: ${e.message.split("\n")[0]}`); }
  }
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
      await check("mixed load: eight Claude streams and eight four-tool OpenAI sessions across two owners", mixedLoad);
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
          await page.locator(".wa-status-word", { hasText: /^Paused until / }).waitFor({ timeout: 10_000 });
          await page.getByText("I continued after the limit.", { exact: true }).waitFor({ timeout: 20_000 });
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
  say(o.line.replace("test:host", "test:host:d"));
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
