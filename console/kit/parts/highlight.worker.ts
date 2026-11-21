// The Shiki side of kit/parts/highlight.ts (TECHNICAL 18.2, 18.6): runs in a module Worker, so a grammar that backtracks cannot freeze
// the page. JavaScript regex engine, no WebAssembly; languages and themes are same-origin chunks loaded by dynamic import.
import type { HighlighterCore } from "shiki/core";
import type { Lines, WorkerReply, WorkerRequest } from "./highlight";

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

let core: Promise<HighlighterCore> | undefined;
const loaded = new Set<string>();

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

/** Loads Shiki and the language (not part of a block's time budget). */
export async function loadLang(l: string): Promise<void> {
  const h = await highlighter();
  if (!loaded.has(l)) { await h.loadLanguage((await L[l]()).default as never); loaded.add(l); }
}

/** The tokens of one complete block; the language must be loaded. */
export async function tokenize(l: string, code: string): Promise<Lines> {
  const h = await highlighter();
  return h.codeToTokens(code, { lang: l, themes: { light: "github-light", dark: "github-dark" }, defaultColor: false }).tokens
    .map((line) => line.map((t) => ({ t: t.content, ...(t.htmlStyle ? { s: t.htmlStyle as Record<string, string> } : {}) })));
}

/** Answers one request. */
export async function handle(m: WorkerRequest): Promise<WorkerReply> {
  try {
    if (m.op === "load") { await loadLang(m.lang); return { id: m.id }; }
    return { id: m.id, lines: await tokenize(m.lang, m.code ?? "") };
  } catch (e) { return { id: m.id, error: String(e) }; }
}

const scope = globalThis as unknown as { WorkerGlobalScope?: unknown; postMessage(m: WorkerReply): void; onmessage: ((e: { data: WorkerRequest }) => void) | null };
if (typeof scope.WorkerGlobalScope === "function" && globalThis instanceof (scope.WorkerGlobalScope as typeof Object)) {
  scope.onmessage = (e) => { void handle(e.data).then((r) => scope.postMessage(r)); };
}
