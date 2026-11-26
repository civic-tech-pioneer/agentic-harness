// S131-4: S38's stage C outcomes on watane's real host (S103, host 19.7), not the double of S38:
// outcomes 1 to 3 (and 4) on a participant home whose app is a fake `Confirm` the test answers
// (test/host/home, `stage-c-home`), outcomes 2 and 4 on `watane-dev serve` (a preview refuses every
// consent at once, host 19.7 "Previews"). Each outcome is checked in the console's own page in
// Chromium and at the kernel. By hand, like test:host: `npm run test:host:c` in console/.
// Needs `watane-dev` and `fake-claude` of watane S103 or later on PATH, and the watane checkout they
// were built from (WATANE_SRC, else the checkout that holds `watane-dev` in its target/), to link
// the home harness against the same host. Exit 0: no FAIL. Exit 1: a failed check. Exit 2: not run
// (a prerequisite the line names).
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync, chmodSync } from "node:fs";
import http from "node:http";
import { homedir, tmpdir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { launch } from "../slow-kit.mjs";
import { NotRun, outcomeOf } from "./outcome.mjs";

const consoleDir = fileURLToPath(new URL("../..", import.meta.url));
const root = join(consoleDir, "..");
const tmp = mkdtempSync(join(tmpdir(), "stage-c-"));
const kernelDir = join(tmp, "agent");
const env = { ...process.env, WATANE_HOME: join(tmp, "dev-home"), WATANE_DEV_CACHE: join(tmp, "cache") };
const vendor = join(process.env.WATANE_DEV_CACHE ?? join(process.env.HOME ?? "", ".cache", "watane-dev"), "vendor");
if (existsSync(vendor)) cpSync(vendor, join(tmp, "cache", "vendor"), { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (s) => console.log(s);

let pass = 0, fail = 0;
const notes = [];
async function check(name, f) {
  if (process.env.ONLY && !new RegExp(process.env.ONLY).test(name)) return;
  try { await f(); pass++; say(`PASS ${name}`); }
  catch (e) { fail++; say(`FAIL ${name}: ${e instanceof Error ? e.message : String(e)}`); }
}
/** A known gap (S131 r1 F1.2, the deputy's ruling): the check runs and says so. It is not counted as a failure while the gap holds,
 * and it fails once the gap is closed (XPASS), so the line is removed rather than left to pass silently. */
let gaps = 0;
async function knownGap(name, why, f) {
  if (process.env.ONLY && !new RegExp(process.env.ONLY).test(name)) return;
  try { await f(); fail++; say(`FAIL ${name}: XPASS, the known gap is closed (${why}): turn this into a plain check`); }
  catch (e) { gaps++; say(`KNOWN GAP ${name} (${why}): ${e instanceof Error ? e.message : String(e)}`); }
}
function ensure(c, msg) { if (!c) throw new Error(typeof msg === "function" ? msg() : msg); }
async function until(f, ms, what) { const t = Date.now(); for (;;) { const v = await f(); if (v) return v; if (Date.now() - t > ms) throw new Error(`timed out after ${ms} ms: ${what}`); await sleep(50); } }
const keyOf = () => `host-c-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

// ---- the host's HTTP, at a page origin (host 8) ----
function post(url, fn, args) {
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
async function ok(url, fn, args) {
  const v = await post(url, fn, args);
  if (v && typeof v === "object" && "ok" in v) return v.ok;
  throw new Error(`${fn} refused: ${JSON.stringify(v)}`);
}
async function transcript(url, session) {
  const out = []; let after = 0;
  for (;;) {
    const page = await ok(url, "sessions.read", { session, after, before: null, max: 200 });
    const es = page.entries.map((e) => (typeof e === "string" ? JSON.parse(e) : e));
    if (!es.length) break;
    out.push(...es); after = es[es.length - 1].i;
    if (after >= page.last) break;
  }
  return out;
}
const lastTurn = (es) => es.filter((e) => e.kind === "turn").at(-1)?.body;
const textOf = (e) => (Array.isArray(e.body?.content) ? e.body.content.map((c) => c.text ?? "").join("") : "");

// ---- the fake claude: dev's fake behind a wrapper that records each start's working folder ----
// The fake reads `fake-claude.json` beside its argv[0], so the wrapper keeps argv[0] and execs the real one.
function fakeDir(dir, real) {
  mkdirSync(dir, { recursive: true });
  const link = join(dir, "fake-claude");
  writeFileSync(link, `#!/usr/bin/python3 -I
import os, sys
with open(os.path.join(os.path.dirname(os.path.abspath(sys.argv[0])), "cwd-%d.txt" % os.getpid()), "w") as f:
    f.write(os.getcwd() + "\\n" + " ".join(sys.argv[1:]))
os.execv(${JSON.stringify(real)}, [sys.argv[0]] + sys.argv[1:])
`);
  chmodSync(link, 0o755);
  return {
    link,
    script: (steps) => writeFileSync(join(dir, "fake-claude.json"), JSON.stringify({ steps })),
    // the session starts (a `--version` probe of the host is not one), each `{cwd, args}`
    starts: () => readdirSync(dir).filter((f) => /^cwd-\d+\.txt$/.test(f)).map((f) => { const [cwd, args = ""] = readFileSync(join(dir, f), "utf8").split("\n"); return { cwd, args, pid: Number(/\d+/.exec(f)[0]) }; })
      .filter(({ args }) => !/(^| )(--version|-v)( |$)/.test(args)),
    cwds() { return this.starts().map(({ cwd }) => cwd); },
  };
}

// ---- the participant home: test/host/home's `stage-c-home` ----
function startHome({ bin, home, claude, folders }) {
  const child = spawn(bin, ["--home", home, "--fake-claude", claude, ...folders], { stdio: ["pipe", "pipe", "inherit"] });
  const h = { child, asked: [], dropped: [], done: [], ready: null };
  const lines = createInterface({ input: child.stdout });
  h.started = new Promise((resolve, reject) => {
    child.on("exit", (code) => reject(new Error(`stage-c-home exited ${code} before its ready line`)));
    lines.on("line", (line) => {
      let m; try { m = JSON.parse(line); } catch { return; }
      if (m.event === "error") reject(new Error(`stage-c-home: ${m.message}`));
      else if (m.event === "asked") h.asked.push(m);
      else if (m.event === "dropped") h.dropped.push(m.i);
      else if (m.event === "done") h.done.push(m);
      else if (m.watane) { h.ready = m; resolve(m); }
    });
  });
  h.send = async (op) => {
    const n = h.done.length;
    child.stdin.write(JSON.stringify(op) + "\n");
    const d = await until(() => h.done[n], 10_000, `the home's answer to ${op.op}`);
    if (d.error) throw new Error(`${op.op}: ${d.error}`);
    return d.value;
  };
  h.stop = async () => { if (child.exitCode !== null) return; child.stdin.end(JSON.stringify({ op: "quit" }) + "\n"); await until(() => child.exitCode !== null, 15_000, "the home to stop").catch(() => child.kill("SIGKILL")); };
  return h;
}

function onPath(name) { return (process.env.PATH ?? "").split(delimiter).map((d) => join(d, name)).find((f) => existsSync(f)); }
function wataneSrc() {
  if (process.env.WATANE_SRC) return resolve(process.env.WATANE_SRC);
  const dev = onPath("watane-dev");
  if (!dev) return null;
  for (let d = dirname(realpathSync(dev)); d !== dirname(d); d = dirname(d)) if (existsSync(join(d, "host", "Cargo.toml")) && existsSync(join(d, "Cargo.lock"))) return d;
  return null;
}

// ---- the kernel's stored session records, read from a stopped host's state (S131 r1 F1.3: `sessions.list` shows no folder or
// stream; test/host/seed's `--get` reads `session/<id>` of `local/agent`, a test-only reach into the host's private state) ----
const seedBin = join(consoleDir, "test", "host", "seed", "target", "release", "seed-state");
function storedRecord(homeDir, id) {
  const r = spawnSync(seedBin, ["--get", homeDir, "local/agent", `session/${id}`], { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`no stored record of ${id} in ${homeDir}: ${r.stderr || `exit ${r.status}`}`);
  return JSON.parse(r.stdout);
}
/** The provider streams the record still holds: every stream but the owner's live one (an `attach`), TECHNICAL 9.1. */
const providerStreams = (rec) => (rec.streams ?? []).filter((x) => x !== rec.live);
/** A launch the host refused or ended: no folder (no file key, 9.6), no first init seen, no provider stream held, the turn idle failed. */
function ensureRefusedRecord(rec, cause) {
  ensure(rec.folder === undefined && !rec.init_seen, `the record keeps a folder: ${JSON.stringify({ folder: rec.folder, init_seen: rec.init_seen })}`);
  ensure(providerStreams(rec).length === 0, `the record holds provider streams ${JSON.stringify(providerStreams(rec))} (live ${rec.live})`);
  ensure(rec.turn?.state === "idle" && rec.turn.stop?.kind === "failed" && cause.test(rec.turn.stop.cause ?? ""), `the stored turn: ${JSON.stringify(rec.turn)}`);
}
const zombieOrGone = (pid) => { try { return /^\d+ \(.*\) Z /.test(readFileSync(`/proc/${pid}/stat`, "utf8")); } catch { return true; } };
async function killHost(homeDir) {
  const pid = Number(JSON.parse(readFileSync(join(homeDir, "run.json"), "utf8")).pid);
  process.kill(pid, "SIGTERM");
  await until(() => zombieOrGone(pid), 15_000, "the preview host to stop");
}

// ---- the page ----
async function openPage(browser, url) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 } });
  page.on("pageerror", (e) => notes.push(`pageerror: ${e.message}`));
  await page.goto(url);
  return page;
}
/** The console's new-session view: a folder path, a first message, Enter. */
async function startInPage(page, folder, input) {
  await page.waitForSelector(".wa-newbtn", { timeout: 15_000 });
  if (!(await page.getByText("A new scratch folder if left blank").count())) await page.locator(".wa-newbtn").click();
  await page.getByText("A new scratch folder if left blank").waitFor({ timeout: 15_000 });
  await page.getByLabel(/Folder path/).fill(folder);
  await page.getByRole("textbox", { name: "Message" }).fill(input);
  await page.keyboard.press("Enter");
}
const statusWord = (page) => page.evaluate(() => document.querySelector(".wa-status-word")?.textContent ?? "");
async function sessionOf(url, input) {
  return until(async () => {
    for (const s of await ok(url, "sessions.list", { after: null, max: 256 })) {
      if ((await transcript(url, s.id)).some((e) => e.kind === "message" && e.body?.role === "user" && textOf(e) === input)) return s;
    }
    return null;
  }, 15_000, `the session of ${JSON.stringify(input)}`);
}

async function main() {
  for (const [p, a] of [["watane-dev", ["--version"]], ["cargo", ["--version"]], ["python3", ["--version"]]]) if (spawnSync(p, a).error) throw new NotRun(`${p} cannot run: not on PATH`);
  const realFake = process.env.WATANE_FAKE_CLAUDE ?? [onPath("watane-dev") && join(dirname(onPath("watane-dev")), "fake-claude"), onPath("fake-claude")].find((f) => f && existsSync(f));
  if (!realFake) throw new NotRun("fake-claude is not found (WATANE_FAKE_CLAUDE, beside watane-dev, or on PATH)");
  const src = wataneSrc();
  if (!src) throw new NotRun("no watane checkout to link the home harness against: set WATANE_SRC (the checkout watane-dev was built from)");

  // ---- build: the kernel and its page (as test:host), the provider through dev, the home harness ----
  const have = spawnSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" });
  if (!have.error && !/^wasm32-wasip2$/m.test(have.stdout)) execFileSync("rustup", ["target", "add", "wasm32-wasip2"], { stdio: "inherit" });
  execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip2", "-p", "agent-mod"], { cwd: join(root, "kernel"), stdio: "inherit" });
  const tdir = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { cwd: join(root, "kernel"), encoding: "utf8" })).target_directory;
  execFileSync("npm", ["run", "build"], { cwd: consoleDir, stdio: "inherit" });
  cpSync(join(root, "kernel", "wasm"), kernelDir, { recursive: true, filter: (p) => !p.includes("/target") && p !== join(root, "kernel", "wasm", "ui") && p !== join(root, "kernel", "wasm", "logic.wasm") });
  cpSync(join(tdir, "wasm32-wasip2", "release", "agent_mod.wasm"), join(kernelDir, "logic.wasm"));
  cpSync(join(root, "ui"), join(kernelDir, "ui"), { recursive: true });
  const providerDir = join(tmp, "claude-code");
  cpSync(join(root, "provider"), providerDir, { recursive: true, filter: (p) => !p.includes("/target") && p !== join(root, "provider", "logic.wasm") });
  const dev = (args) => {
    const r = spawnSync("watane-dev", [...args, "--json", "--dir", kernelDir], { env, encoding: "utf8", timeout: 900_000 });
    let j; try { j = JSON.parse(r.stdout); } catch { j = null; }
    if (r.status !== 0) { const e = j?.errors?.[0] ?? j?.error; const err = new Error(`watane-dev ${args[0]}: ${e ? `${e.code} ${e.message}` : (r.stderr || r.stdout).slice(0, 400)}`); if (/allowlist/.test(err.message) && !existsSync(vendor)) throw new NotRun(err.message); throw err; }
    return j.result;
  };
  dev(["add", providerDir]);
  execFileSync("cargo", ["build", "--release", "-q"], { cwd: join(consoleDir, "test", "host", "seed"), stdio: "inherit" });
  const harness = join(consoleDir, "test", "host", "home");
  const target = join(src, "target");
  execFileSync("cargo", ["build", "-q", "--config", `patch."https://github.com/civic-tech-pioneer/watane".watane-host.path=${JSON.stringify(join(src, "host"))}`],
    { cwd: harness, stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: target } });
  const homeBin = join(target, "debug", "stage-c-home");

  let browser;
  try { browser = await launch(); } catch (e) { throw new NotRun(`Chromium cannot start: ${e.message.split("\n")[0]}`); }
  let home;
  try {
    // ---- the preview: outcomes 2 and 4 (every consent refused at once) ----
    const served = dev(["serve"]);
    const pv = served.pages.find((p) => p.slug === "agent" || p.local === "agent").url;
    const pvFake = join(dirname(served.home), "claude", "fake-claude.json");
    writeFileSync(pvFake, JSON.stringify({ steps: [{ do: "expect-user" }, { do: "message", text: "I read the notes." }, { do: "result" }, { do: "idle" }] }));
    const notes1 = mkdtempSync(join(tmp, "notes-pv-"));
    let pvRefused;
    await check("preview, outcome 2: a folder session is refused at once, the page names the folder, no stream stays open, no folder key", async () => {
      const page = await openPage(browser, pv);
      try {
        await startInPage(page, notes1, "read my notes (preview)");
        const folder = realpathSync(notes1);
        await page.waitForFunction((f) => document.querySelector(".wa-status-word")?.textContent === `Failed: Workspace access was refused for ${f}`, folder, { timeout: 20_000 });
        const s = await sessionOf(pv, "read my notes (preview)");
        const t = lastTurn(await transcript(pv, s.id));
        ensure(t?.state === "idle" && t.stop?.kind === "failed" && /^consent-refused\(workspace:/.test(t.stop.cause ?? ""), `the turn: ${JSON.stringify(t)}`);
        pvRefused = s.id; // its stored record is checked once the preview host stops, below
      } finally { await page.close(); }
    });
    await check("preview, outcome 4: a folder in ~/.ssh is refused before any question, and the page names the cause", async () => {
      await refusedSsh(browser, pv, "preview");
    });
    // the mods the preview built and installed, copied before its stop clears its home; the host is stopped first so its state can be read
    const mods = join(tmp, "built");
    cpSync(join(served.home, "mods"), mods, { recursive: true });
    await killHost(served.home);
    await check("preview, outcome 2 (stored): the refused session's record holds no folder and no provider stream, its turn failed consent-refused", async () => {
      ensure(pvRefused, "no refused session from the preview");
      ensureRefusedRecord(storedRecord(served.home, pvRefused), /^consent-refused\(workspace:/);
    });
    spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", kernelDir], { env, encoding: "utf8", timeout: 60_000 });

    // ---- the participant home with a fake Confirm: outcomes 1 to 4 ----
    const fake = fakeDir(join(tmp, "claude"), realFake);
    home = startHome({ bin: homeBin, home: join(tmp, "home"), claude: fake.link, folders: [join(mods, "claude-code"), join(mods, "agent")] });
    await home.started;
    const url = home.ready.mods.find((m) => m.local === "agent").page;
    say(`participant home: ${home.ready.home} (${url})`);

    const notesDir = mkdtempSync(join(tmp, "notes-"));
    const folder = realpathSync(notesDir);
    const pendingFolder = `workspace:local/claude-code:${folder}`; // `via` is the provider's identity (host 19.7)
    fake.script([{ do: "expect-user", contains: "read my notes" }, { do: "delta", text: "I read the notes." }, { do: "message", text: "I read the notes." }, { do: "result" }, { do: "idle" },
      { do: "expect-user" }, { do: "delta", text: "Working through them" }, { do: "message", text: "Working through them." }, { do: "sleep", ms: 60_000 }, { do: "message", text: "never sent" }]);
    let first, withdrawn, refusedAgain, refusedOther, otherFolder;
    const page = await openPage(browser, url);
    try {
      await check("home, outcome 1: while the app holds the question the page shows the consent wait; once granted the fake runs in that folder and the page shows the turn", async () => {
        await startInPage(page, notesDir, "read my notes");
        const q = await until(() => home.asked[0], 20_000, "the app's question");
        ensure(q.what === pendingFolder && q.origin === "agent" && q.via === "claude-code", `the question: ${JSON.stringify(q)}`);
        await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Waiting for your answer in watane", null, { timeout: 15_000 });
        await knownGap("home, outcome 1: the consent wait names the folder it waits for", "console/src shows only 'Waiting for your answer in watane' (S38 P2.6, Upkeep)", async () => {
          const shown = await page.evaluate(() => document.querySelector(".wa-status")?.textContent ?? document.querySelector(".wa-status-word")?.parentElement?.textContent ?? "");
          ensure(shown.includes(folder), `the status line reads ${JSON.stringify(shown)}, not the folder ${folder}`);
        });
        first = await sessionOf(url, "read my notes");
        await sleep(400);
        ensure(fake.cwds().length === 0, () => `the fake started before the answer: ${JSON.stringify(fake.starts())}`);
        await home.send({ op: "answer", i: q.i, answer: "granted" });
        await page.getByText("I read the notes.", { exact: true }).first().waitFor({ timeout: 20_000 });
        await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Idle", null, { timeout: 20_000 });
        ensure(fake.cwds().includes(folder), `the fake ran in ${JSON.stringify(fake.cwds())}, not ${folder}`);
        const es = await transcript(url, first.id);
        ensure(es.some((e) => e.kind === "message" && e.body?.role === "agent" && textOf(e) === "I read the notes."), "the agent's message is not a transcript entry");
        const listed = (await ok(url, "sessions.list", { after: null, max: 256 })).find((s) => s.id === first.id);
        ensure(listed?.workspace === folder, `the session's workspace is ${JSON.stringify(listed?.workspace)}`);
        const consents = await home.send({ op: "consents", local: "agent" });
        ensure(JSON.stringify(consents).includes(folder), `the home kept no consent: ${JSON.stringify(consents)}`);
      });
      await check("home, outcome 3: withdrawing the consent mid-turn ends that session, keeps its transcript to the last finished message, the page names the cause; the next session there asks again", async () => {
        ensure(first, "no session from outcome 1");
        await page.getByRole("textbox", { name: "Message" }).fill("go on");
        await page.keyboard.press("Enter");
        await page.getByText("Working through them.", { exact: true }).first().waitFor({ timeout: 20_000 });
        await home.send({ op: "withdraw", local: "agent", what: pendingFolder });
        const at = Date.now();
        await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Failed: Connection to Claude Code ended", null, { timeout: 15_000 });
        say(`  withdrawal to the page's words: ${Date.now() - at} ms`);
        const es = await transcript(url, first.id);
        const t = lastTurn(es);
        ensure(t?.state === "idle" && t.stop?.kind === "failed" && t.stop.cause === "gone", `the turn: ${JSON.stringify(t)}`);
        ensure(es.some((e) => e.kind === "message" && textOf(e) === "Working through them."), "the last finished message is gone");
        ensure(!es.some((e) => textOf(e) === "never sent"), "a message after the withdrawal");
        // the withdrawal ended the stream at the host: the fake claude of that session exits (host 19.7 "Withdrawal")
        const live = (pid) => { try { return !/^\d+ \(.*\) Z /.test(readFileSync(`/proc/${pid}/stat`, "utf8")); } catch { return false; } };
        await until(() => fake.starts().every(({ pid }) => !live(pid)), 5_000, "the session's fake claude to exit");
        withdrawn = first.id;
        await page.reload();
        await page.getByText("Working through them.", { exact: true }).first().waitFor({ timeout: 15_000 });
        const asked = home.asked.length;
        fake.script([{ do: "expect-user" }, { do: "message", text: "again" }, { do: "result" }, { do: "idle" }]);
        const next = await ok(url, "sessions.start-in-folder", { key: keyOf(), profile: "scratch", overlay: "{}", input: "read my notes again", folder: notesDir });
        const q = await until(() => home.asked[asked], 15_000, "a new question for the same folder");
        ensure(q.what === pendingFolder, `the question: ${JSON.stringify(q)}`);
        await home.send({ op: "answer", i: q.i, answer: "refused" });
        await until(async () => lastTurn(await transcript(url, next))?.state === "idle", 15_000, "the refused launch to end");
        refusedAgain = next;
      });
      await check("home, outcome 2: a refused consent ends the launch, the page names the folder, no stream stays open, no folder key", async () => {
        const other = mkdtempSync(join(tmp, "other-"));
        const asked = home.asked.length;
        await startInPage(page, other, "read the other notes");
        const q = await until(() => home.asked[asked], 20_000, "the app's question");
        await page.waitForFunction(() => document.querySelector(".wa-status-word")?.textContent === "Waiting for your answer in watane", null, { timeout: 15_000 });
        await home.send({ op: "answer", i: q.i, answer: "refused" });
        await page.waitForFunction((f) => document.querySelector(".wa-status-word")?.textContent === `Failed: Workspace access was refused for ${f}`, realpathSync(other), { timeout: 15_000 });
        const s = await sessionOf(url, "read the other notes");
        const t = lastTurn(await transcript(url, s.id));
        ensure(t?.stop?.cause === `consent-refused(${q.what})`, `the turn: ${JSON.stringify(t)}`);
        ensure(!fake.cwds().includes(realpathSync(other)), `the fake claude started in the refused folder: ${JSON.stringify(fake.starts())}`);
        refusedOther = s.id; otherFolder = realpathSync(other); // its stored record is checked once the home stops, below
        const consents = await home.send({ op: "consents", local: "agent" });
        ensure(!JSON.stringify(consents).includes(realpathSync(other)), `a refused consent was kept: ${JSON.stringify(consents)}`);
      });
    } finally { await page.close(); }
    await check("home, outcome 4: a folder in ~/.ssh is refused before any question reaches the app, and the page names the cause", async () => {
      const asked = home.asked.length;
      await refusedSsh(browser, url, "home");
      await sleep(500);
      ensure(home.asked.length === asked, `the app was asked: ${JSON.stringify(home.asked.slice(asked))}`);
    });
    // the stored records, read once the home stopped
    await home.stop();
    await check("home, outcomes 2 and 3 (stored): refused launches hold no folder and no provider stream; the withdrawn session holds no provider stream and its turn failed gone", async () => {
      ensure(refusedOther && refusedAgain && withdrawn, `missing sessions: ${JSON.stringify({ refusedOther, refusedAgain, withdrawn })}`);
      ensureRefusedRecord(storedRecord(home.ready.home, refusedOther), new RegExp(`^consent-refused\\(workspace:local/claude-code:${otherFolder.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\)$`));
      ensureRefusedRecord(storedRecord(home.ready.home, refusedAgain), /^consent-refused\(workspace:/);
      const w = storedRecord(home.ready.home, withdrawn);
      ensure(providerStreams(w).length === 0, `the withdrawn session still holds provider streams ${JSON.stringify(providerStreams(w))}`);
      ensure(w.turn?.state === "idle" && w.turn.stop?.cause === "gone", `the withdrawn session's stored turn: ${JSON.stringify(w.turn)}`);
    });
  } finally {
    await home?.stop();
    await browser.close();
  }
}

async function refusedSsh(browser, url, where) {
  const ssh = join(homedir(), ".ssh");
  mkdirSync(ssh, { recursive: true, mode: 0o700 });
  const dir = mkdtempSync(join(ssh, "stage-c-"));
  const page = await openPage(browser, url);
  try {
    const before = (await ok(url, "sessions.list", { after: null, max: 256 })).length;
    await startInPage(page, dir, `ssh notes (${where})`);
    const words = await until(async () => {
      const t = await page.evaluate(() => document.body.innerText);
      return /bad-launch: workspace: .*credentials/.test(t.replace(dir, "")) ? t : null;
    }, 15_000, "the page to name the refusal").catch(async (e) => { throw new Error(`${e.message}; the page reads: ${(await page.evaluate(() => document.body.innerText)).replace(/\s+/g, " ").slice(0, 400)}`); });
    const line = words.split("\n").find((l) => /bad-launch: workspace: .*credentials/.test(l.replace(dir, "")));
    say(`  ${where} names: ${line}`);
    const after = await ok(url, "sessions.list", { after: null, max: 256 });
    ensure(after.length === before, `a session was opened for ~/.ssh: ${JSON.stringify(after.slice(0, 2))}`);
  } finally { await page.close(); rmSync(dir, { recursive: true, force: true }); }
}

let code = 0;
try { await main(); }
catch (e) {
  const o = outcomeOf(e);
  if (o.code === 2) code = 2; else fail++;
  say(o.line.replace("test:host", "test:host:c"));
} finally {
  try { spawnSync("watane-dev", ["serve", "--stop", "--json", "--dir", kernelDir], { env, encoding: "utf8", timeout: 60_000 }); } catch {}
  try { rmSync(tmp, { recursive: true, force: true }); } catch {}
}
if (code !== 2) {
  for (const n of notes) say(`note: ${n}`);
  say(`stage C on the host: ${pass} pass, ${fail} fail, ${gaps} known gap${gaps === 1 ? "" : "s"}`);
  code = fail ? 1 : 0;
}
process.exit(code);
