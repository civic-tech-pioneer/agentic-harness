// A fake `window.watane` for tests: `call` answers from handlers, `push` sends a stream message to the handlers `on` registered.
import type { Watane } from "./watane";

export function fakeWatane(handlers: Record<string, (args: any) => unknown>) {
  const hs: ((m: any) => void)[] = [];
  const calls: { fn: string; args: any }[] = [];
  const w: Watane = {
    async call(fn, args) {
      calls.push({ fn, args });
      const h = handlers[fn];
      if (!h) throw Object.assign(new Error(`no ${fn}`), { code: "unknown" });
      return h(args);
    },
    on(h) { hs.push(h); return () => {}; },
  };
  return { w, calls, push: (m: object) => hs.forEach((h) => h(m)) };
}

export const entry = (i: number, text: string) =>
  JSON.stringify({ v: 1, i, kind: "message", at: 1000 * i, body: { role: "agent", content: [{ type: "text", text }] } });
export const synced = (through: number) => JSON.stringify({ _meta: { watane: { type: "synced", through } } });
export const items = (...xs: [number, string][]) => ({ type: "stream", items: xs.map(([seq, body]) => ({ seq, body })) });
