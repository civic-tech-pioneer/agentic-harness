// TECHNICAL 18.2: Shiki with its JavaScript regex engine (no WebAssembly), languages and themes as same-origin chunks loaded
// by dynamic import when a block of that language is first highlighted. P14: once per (language, code), from a bounded cache.
import type { HighlighterCore } from "shiki/core";

/** One highlighted token: its text and the two theme colours as CSS custom properties (`--shiki-light`, `--shiki-dark`). */
export type Tok = { t: string; s?: Record<string, string> };
export type Lines = Tok[][];

export const MAX_CODE = 64 * 1024;
export const CACHE_ENTRIES = 200;
export const CACHE_BYTES = 2 * 1024 * 1024;

type Loader = () => Promise<{ default: unknown }>;
const L: Record<string, Loader> = {
  typescript: () => import("shiki/langs/typescript.mjs"), tsx: () => import("shiki/langs/tsx.mjs"),
  javascript: () => import("shiki/langs/javascript.mjs"), jsx: () => import("shiki/langs/jsx.mjs"),
  json: () => import("shiki/langs/json.mjs"), bash: () => import("shiki/langs/shellscript.mjs"),
  rust: () => import("shiki/langs/rust.mjs"), python: () => import("shiki/langs/python.mjs"),
  toml: () => import("shiki/langs/toml.mjs"), yaml: () => import("shiki/langs/yaml.mjs"),
  diff: () => import("shiki/langs/diff.mjs"), html: () => import("shiki/langs/html.mjs"),
  css: () => import("shiki/langs/css.mjs"), markdown: () => import("shiki/langs/markdown.mjs"),
};
const ALIAS: Record<string, string> = {
  ts: "typescript", typescript: "typescript", tsx: "tsx", js: "javascript", javascript: "javascript", jsx: "jsx", json: "json",
  bash: "bash", sh: "bash", shell: "bash", zsh: "bash", rust: "rust", rs: "rust", python: "python", py: "python",
  toml: "toml", yaml: "yaml", yml: "yaml", diff: "diff", patch: "diff", html: "html", css: "css", md: "markdown", markdown: "markdown",
};

/** The language Shiki is loaded with for a fence label, or undefined (plain). */
export const known = (lang: string | undefined): string | undefined => (lang ? ALIAS[lang.toLowerCase()] : undefined);

const cache = new Map<string, Lines>();
let bytes = 0;
const keyOf = (lang: string, code: string) => `${lang}\0${code}`;

/** The cached tokens for (language, code), without highlighting. */
export function peek(lang: string | undefined, code: string): Lines | undefined {
  const l = known(lang);
  return l ? cache.get(keyOf(l, code)) : undefined;
}

function put(key: string, code: string, v: Lines) {
  cache.set(key, v);
  bytes += code.length;
  for (const [k, old] of cache) {
    if (cache.size <= CACHE_ENTRIES && bytes <= CACHE_BYTES) break;
    cache.delete(k);
    bytes -= k.slice(k.indexOf("\0") + 1).length;
    void old;
  }
}

let core: Promise<HighlighterCore> | undefined;
const loaded = new Set<string>();
const pending = new Map<string, Promise<Lines>>();

function highlighter() {
  core ??= (async () => {
    const [{ createHighlighterCore }, { createJavaScriptRegexEngine }, light, dark] = await Promise.all([
      import("shiki/core"), import("shiki/engine/javascript"),
      import("shiki/themes/github-light.mjs"), import("shiki/themes/github-dark.mjs"),
    ]);
    return createHighlighterCore({ themes: [light.default, dark.default], langs: [], engine: createJavaScriptRegexEngine() });
  })();
  return core;
}

/** Highlights a complete block; null when the language is unknown or the code is over 64 KiB (it stays plain). */
export async function highlight(lang: string | undefined, code: string): Promise<Lines | null> {
  const l = known(lang);
  if (!l || code.length > MAX_CODE) return null;
  const key = keyOf(l, code);
  const hit = cache.get(key);
  if (hit) return hit;
  let p = pending.get(key);
  if (!p) {
    p = (async () => {
      const h = await highlighter();
      if (!loaded.has(l)) { await h.loadLanguage((await L[l]()).default as never); loaded.add(l); }
      const lines = h.codeToTokens(code, { lang: l, themes: { light: "github-light", dark: "github-dark" }, defaultColor: false }).tokens
        .map((line) => line.map((t) => ({ t: t.content, ...(t.htmlStyle ? { s: t.htmlStyle as Record<string, string> } : {}) })));
      put(key, code, lines);
      return lines;
    })().finally(() => pending.delete(key));
    pending.set(key, p);
  }
  return p;
}
