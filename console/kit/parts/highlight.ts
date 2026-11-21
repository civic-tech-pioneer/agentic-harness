// TECHNICAL 18.2: Shiki with its JavaScript regex engine (no WebAssembly), languages and themes as same-origin chunks loaded
// by dynamic import when a block of that language is first highlighted. P14: once per (language, code), from a bounded cache.
// TECHNICAL 18.6 (long sessions): Shiki runs in a module Worker (highlight.worker.ts); the main thread never tokenizes. Blocks go
// one at a time; a block not back within HIGHLIGHT_BUDGET_MS stays plain and its Worker is ended, the next block gets a new one.

/** One highlighted token: its text and the two theme colours as CSS custom properties (`--shiki-light`, `--shiki-dark`). */
export type Tok = { t: string; s?: Record<string, string> };
export type Lines = Tok[][];

export const MAX_CODE = 64 * 1024;
export const CACHE_ENTRIES = 200;
export const CACHE_BYTES = 2 * 1024 * 1024;
/** Wall clock a block has once the Worker has its code (the language is loaded before the clock starts). */
export const HIGHLIGHT_BUDGET_MS = 250;
/** A language chunk that does not load in this time ends the Worker too, so the queue cannot stall. */
const LOAD_LIMIT_MS = 10_000;

const ALIAS: Record<string, string> = {
  ts: "typescript", typescript: "typescript", tsx: "tsx", js: "javascript", javascript: "javascript", jsx: "jsx", json: "json",
  bash: "bash", sh: "bash", shell: "bash", zsh: "bash", rust: "rust", rs: "rust", python: "python", py: "python",
  toml: "toml", yaml: "yaml", yml: "yaml", diff: "diff", patch: "diff", html: "html", css: "css", md: "markdown", markdown: "markdown",
};

/** The language Shiki is loaded with for a fence label, or undefined (plain). */
export const known = (lang: string | undefined): string | undefined => (lang ? ALIAS[lang.toLowerCase()] : undefined);

// null: the block stayed plain (over budget or failed); it is not retried.
const cache = new Map<string, Lines | null>();
let bytes = 0;
const keyOf = (lang: string, code: string) => `${lang}\0${code}`;

/** The cached tokens for (language, code), without highlighting. */
export function peek(lang: string | undefined, code: string): Lines | undefined {
  const l = known(lang);
  return l ? cache.get(keyOf(l, code)) ?? undefined : undefined;
}

function put(key: string, code: string, v: Lines | null) {
  cache.set(key, v);
  bytes += code.length;
  for (const [k, old] of cache) {
    if (cache.size <= CACHE_ENTRIES && bytes <= CACHE_BYTES) break;
    cache.delete(k);
    bytes -= k.slice(k.indexOf("\0") + 1).length;
    void old;
  }
}

/** What the highlighter needs of a Worker (the browser's Worker, or a fake in a test). */
export type WorkerLike = {
  postMessage(m: WorkerRequest): void;
  terminate(): void;
  onmessage: ((e: { data: WorkerReply }) => void) | null;
  onerror: ((e: unknown) => void) | null;
};
export type WorkerRequest = { id: number; op: "load" | "tokens"; lang: string; code?: string };
export type WorkerReply = { id: number; lines?: Lines; error?: string };

const browserWorker = (): WorkerLike =>
  new Worker(new URL("./highlight.worker.ts", import.meta.url), { type: "module" }) as unknown as WorkerLike;
let factory: (() => WorkerLike) | undefined = typeof Worker === "undefined" ? undefined : browserWorker;

type Job = { key: string; lang: string; code: string; done: (v: Lines | null) => void };
const queue: Job[] = [];
let worker: WorkerLike | undefined;
let workerLangs = new Set<string>();
let busy = false;
let seq = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let epoch = 0;

function endWorker() {
  clearTimeout(timer);
  const w = worker;
  worker = undefined;
  workerLangs = new Set();
  epoch++;
  if (w) { w.onmessage = null; w.onerror = null; try { w.terminate(); } catch { /* already gone */ } }
}

/** Test seam: where there is no Worker (jsdom, node) the block stays plain unless a factory is set. Drops the current Worker and the queue's state. */
export function setWorkerFactory(f: (() => WorkerLike) | undefined) {
  endWorker();
  factory = f;
  busy = false;
  for (const j of queue.splice(0)) j.done(null);
}

function finish(job: Job, v: Lines | null) {
  put(job.key, job.code, v);
  busy = false;
  job.done(v);
  pump();
}

function pump() {
  if (busy) return;
  const job = queue.shift();
  if (!job) return;
  if (!factory) { finish(job, null); return; }
  busy = true;
  let w: WorkerLike;
  try { w = worker ??= factory(); } catch { worker = undefined; finish(job, null); return; }
  const mine = epoch;
  const fail = () => { if (epoch !== mine) return; endWorker(); finish(job, null); };
  const send = (m: WorkerRequest, limit: number, onReply: (r: WorkerReply) => void) => {
    const id = ++seq;
    w.onmessage = (e) => {
      if (epoch !== mine || e.data.id !== id) return;
      clearTimeout(timer);
      if (e.data.error !== undefined) fail(); else onReply(e.data);
    };
    w.onerror = fail;
    timer = setTimeout(fail, limit);
    w.postMessage({ ...m, id });
  };
  const tokens = () => send({ id: 0, op: "tokens", lang: job.lang, code: job.code }, HIGHLIGHT_BUDGET_MS, (r) => finish(job, r.lines ?? null));
  if (workerLangs.has(job.lang)) tokens();
  else send({ id: 0, op: "load", lang: job.lang }, LOAD_LIMIT_MS, () => { workerLangs.add(job.lang); tokens(); });
}

const pending = new Map<string, Promise<Lines | null>>();

/** Highlights a complete block; null when the language is unknown, the code is over 64 KiB, or it took over the budget (it stays plain). */
export function highlight(lang: string | undefined, code: string): Promise<Lines | null> {
  const l = known(lang);
  if (!l || code.length > MAX_CODE) return Promise.resolve(null);
  const key = keyOf(l, code);
  if (cache.has(key)) return Promise.resolve(cache.get(key) ?? null);
  let p = pending.get(key);
  if (!p) {
    p = new Promise<Lines | null>((done) => { queue.push({ key, lang: l, code, done }); pump(); }).finally(() => pending.delete(key));
    pending.set(key, p);
  }
  return p;
}
