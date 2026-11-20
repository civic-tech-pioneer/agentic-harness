// TECHNICAL 9.4, 18.2: the assumed shape of a kernel stream message. Nothing else reads these fields.
import type { Activity, PlanItem, ToolCall, TurnState, Ask } from "./model";

export interface TurnExtra { seconds?: number; tools?: number; tokens?: number }
export type CallPatch = Partial<ToolCall> & { id: string };
export type Msg =
  | { t: "message"; i: number; role: "user" | "agent"; text: string; thought?: { text: string; seconds: number } }
  | { t: "tool"; i: number; call: CallPatch }
  | { t: "ask"; i: number; ask: Ask }
  | { t: "allow"; i: number; id: string; reach: string; given: boolean; by: string }
  | { t: "verdict"; i: number; toolCallId: string; outcome: string; reasons: string[]; by: string }
  | { t: "turn"; i?: number; state: TurnState; extra: TurnExtra }
  | { t: "compaction"; i: number }
  | { t: "unreadable"; i?: number; raw: unknown }
  | { t: "chunk"; part: "agent" | "thought" | "user"; text: string; i?: number; at?: number }
  | { t: "call"; call: CallPatch; i?: number; at?: number }
  | { t: "plan"; items: PlanItem[]; i?: number; at?: number }
  | { t: "synced"; through: number }
  | { t: "queue"; inputs: { n: number; input: string }[] }
  | { t: "usage"; used: number; window: number; compactAt?: number };

type Rec = Record<string, unknown>;
const isRec = (x: unknown): x is Rec => typeof x === "object" && x !== null && !Array.isArray(x);
const str = (x: unknown): string | undefined => (typeof x === "string" ? x : undefined);
const num = (x: unknown): number | undefined => (typeof x === "number" && Number.isFinite(x) ? x : undefined);
const KINDS = ["read", "edit", "delete", "move", "search", "execute", "think", "fetch", "other"];
const STATUSES = ["pending", "in_progress", "completed", "failed"];
const PLAN = ["pending", "in_progress", "completed"];

function textOf(content: unknown): string {
  const blocks = Array.isArray(content) ? content : [content];
  return blocks.map((b) => (isRec(b) && b.type === "text" && typeof b.text === "string" ? b.text : "")).join("");
}

function parseCall(o: Rec): CallPatch | undefined {
  const id = str(o.toolCallId);
  if (id === undefined) return undefined;
  const c: CallPatch = { id };
  if (typeof o.title === "string") c.title = o.title;
  if (typeof o.kind === "string" && KINDS.includes(o.kind)) c.kind = o.kind as ToolCall["kind"];
  if (typeof o.status === "string" && STATUSES.includes(o.status)) c.status = o.status as ToolCall["status"];
  if (Array.isArray(o.content)) c.content = o.content as ToolCall["content"];
  if (Array.isArray(o.locations)) c.locations = o.locations as ToolCall["locations"];
  if ("rawInput" in o) c.rawInput = o.rawInput;
  if ("rawOutput" in o) c.rawOutput = o.rawOutput;
  return c;
}

function parseTurn(o: unknown): { state: TurnState; extra: TurnExtra } | undefined {
  if (!isRec(o)) return undefined;
  switch (o.state) {
    case "running": {
      const a = o.activity;
      let activity: Activity | undefined;
      if (isRec(a) && a.kind === "checking") activity = { kind: "checking", ...(str(a.on) !== undefined ? { on: a.on as string } : {}) };
      else if (isRec(a) && a.kind === "compacting") activity = { kind: "compacting" };
      else if (isRec(a) && a.kind === "consent" && str(a.what) !== undefined) activity = { kind: "consent", what: a.what as string };
      return { state: { state: "running", ...(activity ? { activity } : {}), since: num(o.since) ?? 0 }, extra: {} };
    }
    case "waiting":
      if (!Array.isArray(o.asks) || num(o.expiresAt) === undefined) return undefined;
      return { state: { state: "waiting", asks: o.asks.filter((a): a is string => typeof a === "string"), expiresAt: o.expiresAt as number }, extra: {} };
    case "paused":
      if (num(o.until) === undefined) return undefined;
      return { state: { state: "paused", until: o.until as number }, extra: {} };
    case "idle": {
      const s = o.stop;
      let stop: Extract<TurnState, { state: "idle" }>["stop"];
      if (!isRec(s)) return undefined;
      if (s.kind === "completed" || s.kind === "interrupted") stop = { kind: s.kind };
      else if (s.kind === "stopped" && str(s.limit) !== undefined) stop = { kind: "stopped", limit: s.limit as string };
      else if (s.kind === "failed" && str(s.error) !== undefined) stop = { kind: "failed", error: s.error as string };
      else return undefined;
      const extra: TurnExtra = {};
      for (const k of ["seconds", "tools", "tokens"] as const) { const n = num(o[k]); if (n !== undefined) extra[k] = n; }
      return { state: { state: "idle", stop }, extra };
    }
    case "unknown": return { state: { state: "unknown" }, extra: {} };
    case "resume-lost": return { state: { state: "resume-lost" }, extra: {} };
  }
  return undefined;
}

function parseEntry(e: Rec): Msg {
  const i = num(e.i);
  if (i === undefined) return { t: "unreadable", raw: e };
  const bad: Msg = { t: "unreadable", i, raw: e };
  if (e.v !== 1 || !isRec(e.body)) return bad;
  const b = e.body;
  switch (e.kind) {
    case "message": {
      if (b.role !== "user" && b.role !== "agent") return bad;
      const m: Msg = { t: "message", i, role: b.role, text: textOf(b.content) };
      if (isRec(b.thought) && str(b.thought.text) !== undefined) m.thought = { text: b.thought.text as string, seconds: num(b.thought.seconds) ?? 0 };
      return m;
    }
    case "tool": {
      const call = parseCall(b);
      return call ? { t: "tool", i, call } : bad;
    }
    case "ask": {
      const id = str(b.toolCallId);
      const expiresAt = num(b.expiresAt);
      if (id === undefined || expiresAt === undefined) return bad;
      const ask: Ask = { toolCallId: id, input: b.input, expiresAt };
      const d = b.decision;
      if (isRec(d)) {
        if ((d.outcome !== "allow" && d.outcome !== "deny") || str(d.by) === undefined) return bad;
        ask.decision = { outcome: d.outcome, by: d.by as string, ...(d.for === "session" ? { for: "session" as const } : {}), ...(str(d.reason) !== undefined ? { reason: d.reason as string } : {}) };
      }
      return { t: "ask", i, ask };
    }
    case "allow": {
      const id = str(b.id), reach = str(b.reach), by = str(b.by);
      if (id === undefined || reach === undefined || by === undefined) return bad;
      return { t: "allow", i, id, reach, by, given: b.given === true };
    }
    case "verdict": {
      const id = str(b.toolCallId), outcome = str(b.outcome), by = str(b.by);
      if (id === undefined || outcome === undefined || by === undefined) return bad;
      const reasons = Array.isArray(b.reasons) ? b.reasons.filter((r): r is string => typeof r === "string") : [];
      return { t: "verdict", i, toolCallId: id, outcome, reasons, by };
    }
    case "turn": {
      const t = parseTurn(b);
      return t ? { t: "turn", i, ...t } : bad;
    }
    case "compaction": return { t: "compaction", i };
    case "unknown": return { t: "unreadable", i, raw: b.raw };
  }
  return bad;
}

function parseUpdate(o: Rec, w: Rec | undefined): Msg {
  const i = num(w?.i);
  const at = num(w?.at);
  const ex = { ...(i !== undefined ? { i } : {}), ...(at !== undefined ? { at } : {}) };
  switch (o.sessionUpdate) {
    case "agent_message_chunk": return { t: "chunk", part: "agent", text: textOf(o.content), ...ex };
    case "agent_thought_chunk": return { t: "chunk", part: "thought", text: textOf(o.content), ...ex };
    case "user_message_chunk": return { t: "chunk", part: "user", text: textOf(o.content), ...ex };
    case "tool_call":
    case "tool_call_update": {
      const call = parseCall(o);
      return call ? { t: "call", call, ...ex } : { t: "unreadable", raw: o };
    }
    case "plan": {
      if (!Array.isArray(o.entries)) return { t: "unreadable", raw: o };
      const items: PlanItem[] = o.entries.filter(isRec).map((p) => ({
        content: str(p.content) ?? "",
        status: PLAN.includes(p.status as string) ? (p.status as PlanItem["status"]) : "pending",
      }));
      return { t: "plan", items, ...ex };
    }
  }
  return { t: "unreadable", raw: o };
}

function parseItem(w: Rec, o: Rec): Msg {
  switch (w.type) {
    case "synced": { const n = num(w.through); return n === undefined ? { t: "unreadable", raw: o } : { t: "synced", through: n }; }
    case "turn": { const t = parseTurn(w.state); return t ? { t: "turn", ...t } : { t: "unreadable", raw: o }; }
    case "queue": {
      if (!Array.isArray(w.inputs)) return { t: "unreadable", raw: o };
      const inputs = w.inputs.filter(isRec).flatMap((q) => (num(q.n) !== undefined && str(q.input) !== undefined ? [{ n: q.n as number, input: q.input as string }] : []));
      return { t: "queue", inputs };
    }
    case "usage": {
      const used = num(w["context-used"]), window = num(w["context-window"]), compactAt = num(w["compact-at"]);
      if (used === undefined || window === undefined) return { t: "unreadable", raw: o };
      return { t: "usage", used, window, ...(compactAt !== undefined ? { compactAt } : {}) };
    }
    case "entry": return isRec(w.entry) ? parseEntry(w.entry) : { t: "unreadable", raw: o };
  }
  return { t: "unreadable", raw: o };
}

/** One stream message (JSON text) as the console understands it; anything else is `unreadable`. */
export function parse(message: string): Msg {
  let o: unknown;
  try { o = JSON.parse(message); } catch { return { t: "unreadable", raw: message }; }
  if (!isRec(o)) return { t: "unreadable", raw: o };
  const w = isRec(o._meta) && isRec(o._meta.watane) ? o._meta.watane : undefined;
  if (typeof o.sessionUpdate === "string") return parseUpdate(o, w);
  if (w && typeof w.type === "string") return parseItem(w, o);
  if ("kind" in o && "i" in o) return parseEntry(o);
  return { t: "unreadable", raw: o };
}
