// Shared by the slow scripts (test/*.slow.mjs): build once, serve ui/, launch Chromium.
// Playwright is not a dependency: it is resolved from the machine's global install (NODE_PATH).
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const ui = join(root, "..", "ui");
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2" };

// Builds ui/ unless the runner (test/slow.mjs) already did and said so with WA_UI_BUILT=1.
export function buildOnce() {
  if (process.env.WA_UI_BUILT === "1") return;
  execSync("npm run build", { cwd: root, stdio: "inherit" });
}

export async function serveUi() {
  const server = createServer(async (req, res) => {
    const p = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
    // host 8.5 serves /watane.js; these pages run without a host, so it is an empty script and the page opens the fake
    if (p === "/watane.js") return void res.writeHead(200, { "content-type": "text/javascript" }).end("");
    try {
      const f = p.endsWith("/") ? p + "index.html" : p;
      const body = await readFile(join(ui, f));
      res.writeHead(200, { "content-type": types[extname(f)] ?? "application/octet-stream" }).end(body);
    } catch { res.writeHead(404).end(); }
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  return {
    url: `http://127.0.0.1:${server.address().port}/`,
    close: () => new Promise((r) => server.close(() => r())),
  };
}

export async function launch() {
  let playwright;
  try { playwright = createRequire(import.meta.url)("playwright"); } catch {
    console.error("NOT RUN test:slow: `playwright` cannot be required here (install it globally, or set NODE_PATH to it); the check in a real browser did not run.");
    process.exit(2);
  }
  return playwright.chromium.launch();
}
