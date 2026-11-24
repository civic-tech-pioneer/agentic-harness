// TECHNICAL 17, S39. Slow: only the desktop app can prove release consent,
// launcher wiring, the selected Claude binary, and persistence across app exits.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { access, copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const bin = process.env.WATANE_BIN_DIR;
assert(bin, "Set WATANE_BIN_DIR to the Watane build directory with watane-app, watane-dev and fake-claude.");
await Promise.all(["watane-app", "watane-dev", "fake-claude"].map((name) => access(resolve(bin, name))));
const root = fileURLToPath(new URL("../../../", import.meta.url));
const work = await mkdtemp(join(tmpdir(), "s39-installed-"));
const publisherHome = join(work, "publisher");
const aliceHome = join(work, "alice");
const site = join(publisherHome, "dist", "site");

async function until(description, probe, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  do {
    const result = await probe();
    if (result) return result;
    await delay(100);
  } while (Date.now() < deadline);
  throw new Error(`Timed out waiting for ${description}`);
}

function startApp(home) {
  // Keep the mod window mapped so WebKit schedules its animation frames.
  const child = spawn(resolve(bin, "watane-app"), ["--home", home, "--port", "0"], {
    env: { ...process.env, WATANE_DRIVER_SCREEN: "background" },
    stdio: ["pipe", "pipe", "inherit"],
  });
  const replies = new Map();
  let sequence = 0;
  const lines = createInterface({ input: child.stdout });
  const ready = once(lines, "line").then(([line]) => {
    const event = JSON.parse(line);
    assert.equal(event.event, "ready", line);
  });
  lines.on("line", (line) => {
    const message = JSON.parse(line);
    replies.get(message.id)?.(message);
  });
  const done = once(child, "exit");
  function request(op, fields = {}) {
    const id = ++sequence;
    return new Promise((complete, reject) => {
      const timer = setTimeout(() => {
        replies.delete(id);
        reject(new Error(`App driver did not answer ${op}`));
      }, 35_000);
      replies.set(id, ({ ok, err }) => {
        clearTimeout(timer);
        replies.delete(id);
        if (err) reject(new Error(err));
        else complete(ok);
      });
      child.stdin.write(JSON.stringify({ id, op, ...fields }) + "\n");
    });
  }
  return {
    ready,
    request,
    async eval(label, js) {
      const result = await request("eval", { label, js });
      return result === "" ? null : JSON.parse(result);
    },
    wait: (label) => request("wait", { label, timeoutMs: 0 }),
    async quit() {
      child.stdin.end(JSON.stringify({ op: "quit" }) + "\n");
      const [code] = await done;
      assert.equal(code, 0);
    },
    kill: () => child.kill(),
  };
}

const server = createServer(async (req, res) => {
  try {
    const path = resolve(site, "." + new URL(req.url, "http://localhost").pathname);
    if (!path.startsWith(site + sep)) throw new Error("Outside site");
    const bytes = await readFile(path);
    res.writeHead(200, { "content-type": "application/octet-stream" }).end(bytes);
  } catch {
    res.writeHead(404).end();
  }
});

const main = "main";
const consolePage = "mod-agent";
const consent = "consent";

async function approve(app) {
  await until("consent to render", () => app.eval(consent, "document.body.dataset.ready === 'true'"));
  // This driver test exercises the consent contents and the commit. Watane's
  // own foreground tests exercise the focus gate.
  await app.eval(consent, "document.querySelector('#approve').disabled = false; document.querySelector('#approve').click(); null");
  await until("consent to close", async () => !await app.wait(consent));
}

async function publish(app, channel) {
  const child = spawn(join(root, "factory", "publish-test-mods.sh"), [publisherHome, channel], {
    env: {
      ...process.env,
      PATH: resolve(bin) + ":" + process.env.PATH,
      WATANE_DEV_CACHE: join(work, "dev-cache"),
    },
    stdio: ["ignore", "pipe", "inherit"],
  });
  let link = "";
  child.stdout.setEncoding("utf8").on("data", (chunk) => { link += chunk; });
  const closed = once(child, "close");
  let finished = false;
  void closed.then(() => { finished = true; });
  try {
    for (const slug of ["claude-code", "agent"]) {
      await until(`publish consent for ${slug}`, () => {
        assert(!finished, `Publishing exited before the ${slug} consent`);
        return app.wait(consent);
      }, 600_000);
      await until("publish consent to render", () => app.eval(consent, "document.body.dataset.ready === 'true'"));
      const body = await app.eval(consent, "document.body.textContent");
      assert.match(body, /Sign a release/i);
      assert(body.includes(slug), body);
      await approve(app);
    }
    const [code] = await closed;
    assert.equal(code, 0);
    assert.equal(link.trim(), channel + "#agent");
    return link.trim();
  } finally {
    if (!finished) child.kill();
  }
}

async function openConsole(app) {
  await until("agent launcher tile", () => app.eval(main, "!!document.querySelector('[data-tile=agent]')"));
  await app.eval(main, "document.querySelector('[data-tile=agent]').click(); document.querySelector('[data-local=agent] .open').click(); null");
  await until("agent console window", () => app.wait(consolePage));
}

async function send(app, message) {
  await until("message composer", () => app.eval(consolePage, "!!document.querySelector('textarea[aria-label=Message]:enabled')"));
  await app.eval(consolePage, `(() => {
    const field = document.querySelector('textarea[aria-label=Message]');
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(field, ${JSON.stringify(message)});
    field.dispatchEvent(new Event('input', { bubbles: true }));
    return null;
  })()`);
  await until("Send to become available", () => app.eval(consolePage, "!!document.querySelector('.wa-send:enabled')"));
  await app.eval(consolePage, "document.querySelector('.wa-send').click(); null");
}

const includes = (app, text) => app.eval(consolePage, `document.body.textContent.includes(${JSON.stringify(text)})`);
const idle = (app) => app.eval(consolePage, "document.querySelector('.wa-status-word')?.textContent === 'Idle'");
let publisher;
let alice;
try {
  await Promise.all([mkdir(publisherHome), mkdir(aliceHome)]);
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const url = `http://127.0.0.1:${server.address().port}/`;
  publisher = startApp(publisherHome);
  await publisher.ready;
  await publisher.eval(main, `window.__site = null;
    window.__TAURI_INTERNALS__.invoke('site_set_url', {url: ${JSON.stringify(url)}})
      .then(value => { window.__site = value; }, error => { window.__site = {error}; });
    null`);
  const status = await until("publisher site configuration", () => publisher.eval(main, "window.__site"));
  assert(!status.error, JSON.stringify(status.error));
  assert(status.site?.link, JSON.stringify(status));
  const link = await publish(publisher, status.site.link);
  await publisher.quit();

  alice = startApp(aliceHome);
  await alice.ready;
  await until("fresh app manager to initialise", () => alice.eval(main, "document.body.dataset.ready === 'true'"));
  await alice.eval(main, `document.querySelector('#first-link').value = ${JSON.stringify(link)}; document.querySelector('#first-link-go').click(); null`);
  await until("install consent", async () => {
    const error = await alice.eval(main, "document.querySelector('#error').textContent");
    assert(!error, error);
    return alice.wait(consent);
  });
  await until("install consent to render", () => alice.eval(consent, "document.body.dataset.ready === 'true'"));
  const sections = await alice.eval(consent, "Array.from(document.querySelectorAll('main section'), s => ({name:s.querySelector('h2').textContent, words:s.textContent}))");
  assert.deepEqual(sections.map(({ name }) => name).sort(), ["Agentic harness", "Claude Code, unchecked"]);
  const agentWords = sections.find(({ name }) => name === "Agentic harness").words;
  assert.match(agentWords, /Keep its own data on this device/);
  assert.match(agentWords, /Call mods:claude-code/);
  assert.match(sections.find(({ name }) => name === "Claude Code, unchecked").words, /Can start Claude Code with your Claude account/);
  await approve(alice);
  await until("both installed launcher tiles", () => alice.eval(main, "!!document.querySelector('[data-tile=agent]') && !!document.querySelector('[data-tile=claude-code]')"));
  console.log("PASS AC1 one consent installs the agent and provider");

  const fake = join(work, "fake-claude");
  await copyFile(resolve(bin, "fake-claude"), fake);
  await writeFile(join(work, "fake-claude.json"), JSON.stringify({ steps: [
    { do: "expect-user", contains: "hello from the installed console" },
    { do: "sleep", ms: 1500 },
    { do: "delta", text: "A live fragment" },
    { do: "sleep", ms: 2000 },
    { do: "ask", tool: "Read", input: { file_path: "s39.txt" }, await: true },
    { do: "message", text: "The tool answer arrived." },
    { do: "result" },
    { do: "idle" },
    { do: "expect-user", contains: "start the long turn" },
    { do: "long-turn", entries: 200, "every-ms": 100 },
  ] }));
  await alice.request("menu", { item: "settings" });
  await until("Claude Code settings", () => alice.eval(main, "!document.querySelector('#settings').hidden && !document.querySelector('#claude-choose').disabled"));
  await alice.request("pick", { paths: [fake] });
  await alice.eval(main, "document.querySelector('#claude-choose').click(); null");
  await until("Claude Code setting to show the selected binary", () => alice.eval(main, "document.querySelector('#claude-status').textContent === 'Claude Code 2.1.295'"));
  await alice.request("menu", { item: "mods" });
  await openConsole(alice);
  await until("scratch session composer", () => includes(alice, "A new scratch folder"));
  await send(alice, "hello from the installed console");
  try {
    await until("streamed delta before the finished reply", async () => await includes(alice, "A live fragment") && !await idle(alice));
  } catch (error) {
    const page = await alice.eval(consolePage, "document.body.innerText");
    const log = await readFile(join(work, "fake-claude.log"), "utf8").catch(() => "Fake Claude did not log a launch");
    throw new Error(`${error.message}\n${page}\n${log.slice(-3000)}`, { cause: error });
  }
  const allowOnce = "Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === 'Allow once')";
  await until("tool ask", () => alice.eval(consolePage, `!!(${allowOnce})`));
  await alice.eval(consolePage, `(${allowOnce}).click(); null`);
  await until("reply after tool approval", async () => await includes(alice, "The tool answer arrived.") && await idle(alice));
  await send(alice, "start the long turn");
  await until("first part of the long turn", () => includes(alice, "entry 1 "));
  await alice.eval(consolePage, "document.querySelector('.wa-stop').click(); null");
  await until("interrupted turn back at Idle", () => idle(alice));
  const wire = await readFile(join(work, "fake-claude.log"), "utf8");
  const received = wire.trim().split("\n").map(JSON.parse)
    .filter(({ stdin }) => stdin).map(({ stdin }) => JSON.parse(stdin));
  assert(received.some(({ request }) => request?.subtype === "interrupt"), "Claude must receive Stop");
  assert(received.some(({ response }) => response?.response?.behavior === "allow"), "Claude must receive the tool approval");

  await alice.quit();
  alice = startApp(aliceHome);
  await alice.ready;
  await openConsole(alice);
  await until("restored transcript after app restart", () => includes(alice, "The tool answer arrived."));
  console.log("PASS AC2 launcher, live reply, tool ask, Stop, and transcript after restart");
  await alice.quit();
} finally {
  publisher?.kill();
  alice?.kill();
  server.closeAllConnections();
  server.close();
  await rm(work, { recursive: true, force: true });
}
