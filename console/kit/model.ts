// TECHNICAL 18.2, 18.3: the session view the console shows, built from a kernel stream.
export type ToolKind = "read"|"edit"|"delete"|"move"|"search"|"execute"|"think"|"fetch"|"other";
export type ToolStatus = "pending"|"in_progress"|"completed"|"failed";
export type ToolContent =
  | { type: "content"; content: { type: "text"; text: string } }
  | { type: "diff"; path: string; oldText: string | null; newText: string }
  | { type: "terminal"; terminalId: string };
export interface ToolCall { id: string; title: string; kind: ToolKind; status: ToolStatus;
  content: ToolContent[]; locations: { path: string; line?: number }[]; rawInput?: unknown; rawOutput?: unknown }
export interface Decision { outcome: "allow"|"deny"|"none"; for?: "session"; reason?: string; by: string }
export interface Ask { id: string; kind: "ask"|"check"; toolCallId: string; input: unknown; expiresAt: number; reach?: string; decision?: Decision }
/** The one argument of an ask's input the card names (9.4): `file_path`, `path`, `command`, `url`, `pattern`, in that order. */
export function askArg(input: unknown): string | undefined {
  if (typeof input !== "object" || input === null) return undefined;
  for (const k of ["file_path", "path", "command", "url", "pattern"]) {
    const v = (input as Record<string, unknown>)[k];
    if (typeof v === "string" && v !== "") return v;
  }
  return undefined;
}
/** The card's name for a call (9.4): with an ask, its input's argument; else the call's first path; else the title. */
export function cardName(call: ToolCall, ask?: Ask): string {
  const path = ask ? askArg(ask.input) : call.locations[0]?.path;
  return path ? `${call.kind.charAt(0).toUpperCase() + call.kind.slice(1)} ${path}` : call.title;
}
export type Activity = { kind: "checking"; on?: string } | { kind: "compacting" } | { kind: "consent"; what: string };
export type TurnState =
  | { state: "running"; activity?: Activity; since: number }
  | { state: "waiting"; asks: string[]; expiresAt: number }
  | { state: "paused"; until?: number }
  | { state: "idle"; stop: { kind: "completed" } | { kind: "interrupted" } | { kind: "stopped"; limit: string } | { kind: "failed"; error: string } }
  | { state: "unknown" } | { state: "resume-lost" };
export type Row =
  | { kind: "user"; key: string; i?: number; text: string }
  | { kind: "agent"; key: string; i?: number; text: string; writing: boolean }
  | { kind: "thought"; key: string; i?: number; text: string; streaming: boolean; seconds?: number }
  | { kind: "explored"; key: string; reads: number; searches: number; calls: ToolCall[] }
  | { kind: "tool"; key: string; i?: number; call: ToolCall; ask?: Ask; checking?: true; ended?: true }
  | { kind: "decision"; key: string; i?: number; who: string; what: string; reason?: string }
  | { kind: "footer"; key: string; seconds: number; tools: number; tokens?: number }
  | { kind: "unknown"; key: string; i?: number; raw: unknown };
export interface PlanItem { content: string; status: "pending"|"in_progress"|"completed" }
/** The reducer's own bookkeeping; a reader of the view ignores it. */
export interface Internal { skip?: boolean; thoughtAt?: number; seq?: number; open?: { ask: Ask; title: string; call: ToolCall }[] }
export interface SessionView {
  rows: Row[]; openAsks: string[]; turn: TurnState | null;
  queue: { n: number; input: string }[]; plan: PlanItem[] | null;
  usage: { used: number; window: number; compactAt?: number } | null;
  allows: { id: string; reach: string }[]; lastIndex: number; synced: boolean;
  /** Paging in the history behind the attach: the turn is not known yet. */
  loading: boolean;
  x?: Internal;
}
export const emptyView: SessionView = {
  rows: [], openAsks: [], turn: null, queue: [], plan: null, usage: null,
  allows: [], lastIndex: 0, synced: false, loading: false,
};
export interface Actions {
  /** The asks this console answered that have no decision entry yet (9.4 rule 4): the one source for the card's line and the dock's count. */
  answered?: ReadonlySet<string>;
  say(text: string): void;
  answer(ask: string, decision: { outcome: "allow"|"deny"; for?: "session"; reason?: string }): void | Promise<void>;
  stop(): void;
  takeBack(allowId: string): void;
}
/** 2.3: did the kernel refuse an `answer` because the ask is already decided (a refusal whose field is `ask`, case 9)? */
export function isAskRefusal(e: unknown): boolean {
  const o = e as { field?: unknown; refusals?: { field?: unknown }[]; refusal?: { field?: unknown } } | null;
  return o?.field === "ask" || !!o?.refusals?.some((r) => r?.field === "ask") || o?.refusal?.field === "ask";
}
/** The event the page sends its scroller when the column changed size, or a card changed what holds focus: what is measured is read again. */
export const MEASURE = "wa-measure";
