// TECHNICAL 2.3, 9.4: a scripted fake kernel with the shape of `attach` and `read`, for the console's tests.
import type { Kernel } from "../kit/kernel";

export interface FakeKernel extends Kernel { drop(): void; play(step?: number): void; calls: { name: string; args: unknown[] }[] }

type Rec = Record<string, any>;

/** 9.4: an entry's serialization cut into `part` entries at UTF-8 character boundaries, each `text` at most `size` bytes; the parts take the indexes `i-m+1 .. i`. */
export function partsOf(entry: object, size: number): string[] {
  const e = entry as Rec;
  const enc = new TextEncoder();
  const texts: string[] = [];
  let cur = "", bytes = 0;
  for (const ch of JSON.stringify(entry)) {
    const b = enc.encode(ch).length;
    if (bytes + b > size && cur) { texts.push(cur); cur = ""; bytes = 0; }
    cur += ch; bytes += b;
  }
  texts.push(cur);
  const m = texts.length;
  return texts.map((text, k) => JSON.stringify({ v: 1, i: e.i - m + 1 + k, kind: "part", ...(e.at !== undefined ? { at: e.at } : {}), body: { n: k + 1, m, text } }));
}

/** `script` is steps of stream messages; `opts.preload` are entries (JSON) already stored; `opts.refuseAnswer` makes `answer` reject the way a kernel refusal does (`busy`, or a refusal whose field is `ask`: already decided). */
export function fakeKernel(script: string[][], opts?: { preload?: string[]; refuseAnswer?: "busy" | "ask" }): FakeKernel {
  const store: Rec[] = (opts?.preload ?? []).map((s) => JSON.parse(s));
  const calls: FakeKernel["calls"] = [];
  let stream: { onMessage: (j: string) => void; onEnd: (r: string) => void } | null = null;
  let next = 0, started = 0;
  const acc = { agent: "", thought: "", thoughtAt: undefined as number | undefined, thoughtEnd: undefined as number | undefined, user: "", tools: new Map<string, Rec>() };
  const items: Record<string, string> = {};
  const last = () => store[store.length - 1]?.i ?? 0;

  // What the kernel would store for a live message: an entry, once a message or call is finished.
  function record(json: string) {
    const o = JSON.parse(json) as Rec;
    const w = o._meta?.watane as Rec | undefined;
    if (w?.type === "turn" || w?.type === "queue" || w?.type === "usage") { items[w.type] = json; return; }
    if (o.sessionUpdate === "plan") { items.plan = json; return; }
    if (w?.type === "entry") { if (w.entry.i > last()) store.push(w.entry); return; }
    if (!o.sessionUpdate) { if (o.kind && o.i > last()) store.push(o); return; }
    const { i, at = 0 } = w ?? {};
    const text = o.content?.type === "text" ? (o.content.text as string) : "";
    const put = (kind: string, body: Rec) => { if (i !== undefined && i > last()) store.push({ v: 1, i, kind, at, body }); };
    switch (o.sessionUpdate) {
      case "agent_thought_chunk": acc.thought += text; acc.thoughtAt ??= at; acc.thoughtEnd = at; break; // seconds: first to last chunk (9.4)
      case "user_message_chunk":
        acc.user += text;
        if (i !== undefined) { put("message", { role: "user", content: [{ type: "text", text: acc.user }] }); acc.user = ""; }
        break;
      case "agent_message_chunk":
        acc.agent = i !== undefined ? text : acc.agent + text; // 9.4: the finishing update carries the whole message
        if (i !== undefined) {
          put("message", { role: "agent", content: [{ type: "text", text: acc.agent }],
            ...(acc.thoughtAt !== undefined ? { thought: { text: acc.thought, seconds: Math.round((acc.thoughtEnd! - acc.thoughtAt) / 1000) } } : {}) });
          acc.agent = ""; acc.thought = ""; acc.thoughtAt = acc.thoughtEnd = undefined;
        }
        break;
      case "tool_call":
      case "tool_call_update": {
        const { sessionUpdate: _s, _meta: _m, ...fields } = o;
        const merged = { ...acc.tools.get(o.toolCallId), ...fields };
        acc.tools.set(o.toolCallId, merged);
        put("tool", merged);
        break;
      }
    }
  }

  return {
    calls,
    async attach(session, after, onMessage, onEnd) {
      calls.push({ name: "attach", args: [session, after] });
      // 9.4: an `after` inside a group starts the replay at the group's first part
      const at = store.find((e) => e.i > after);
      if (at?.kind === "part" && at.body.n > 1) after = at.i - at.body.n;
      const entries = store.filter((e) => e.i > after);
      if (entries.length > 128) return { behind: last() };
      stream?.onEnd("replaced");
      stream = { onMessage, onEnd };
      for (const e of entries) onMessage(JSON.stringify(e));
      const writing = acc.agent !== "" || acc.thought !== "";
      onMessage(JSON.stringify({ _meta: { watane: { type: "synced", through: Math.max(after, last()), ...(writing ? { writing } : {}) } } }));
      for (const k of ["turn", "queue", "usage", "plan"]) if (items[k]) onMessage(items[k]);
      return { ok: true };
    },
    async read(session, after, max) {
      calls.push({ name: "read", args: [session, after, max] });
      return store.filter((e) => e.i > after).slice(0, Math.min(max, 256)).map((e) => JSON.stringify(e));
    },
    /** A new session that answers like the story: its stored entries are cleared and the story's first step plays. */
    async start(key, profile, overlay, input) {
      calls.push({ name: "start", args: [key, profile, overlay, input] });
      store.length = 0; for (const k of Object.keys(items)) delete items[k];
      acc.agent = acc.thought = acc.user = ""; acc.thoughtAt = acc.thoughtEnd = undefined; acc.tools.clear();
      setTimeout(() => { if (script[0]) this.play(0); }, 0);
      return `new${++started}`;
    },
    async say(session, key, input) { calls.push({ name: "say", args: [session, key, input] }); return 0; },
    async answer(session, key, ask, decision) {
      calls.push({ name: "answer", args: [session, key, ask, decision] });
      if (opts?.refuseAnswer === "busy") throw Object.assign(new Error("busy: the session's provider stream is not free"), { kind: "busy" });
      if (opts?.refuseAnswer === "ask") throw Object.assign(new Error("refused"), { kind: "refused", refusals: [{ field: "ask", reason: "the ask is already decided" }] });
    },
    async interrupt(session, replacement) { calls.push({ name: "interrupt", args: [session, replacement] }); },
    async takeBack(session, allow) { calls.push({ name: "takeBack", args: [session, allow] }); },
    drop() { const s = stream; stream = null; s?.onEnd("dropped"); },
    play(step) {
      const n = step ?? next++;
      const msgs = script[n];
      if (!msgs) throw new Error(`no step ${n} in the script`);
      for (const j of msgs) { record(j); stream?.onMessage(j); }
    },
  };
}
