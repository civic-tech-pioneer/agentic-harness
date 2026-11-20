// TECHNICAL 18.2, 18.3: the session view the console shows, built from a kernel stream.
export type ToolKind = "read"|"edit"|"delete"|"move"|"search"|"execute"|"think"|"fetch"|"other";
export type ToolStatus = "pending"|"in_progress"|"completed"|"failed";
export type ToolContent =
  | { type: "content"; content: { type: "text"; text: string } }
  | { type: "diff"; path: string; oldText: string | null; newText: string }
  | { type: "terminal"; terminalId: string };
export interface ToolCall { id: string; title: string; kind: ToolKind; status: ToolStatus;
  content: ToolContent[]; locations: { path: string; line?: number }[]; rawInput?: unknown; rawOutput?: unknown }
export interface Decision { outcome: "allow"|"deny"; for?: "session"; reason?: string; by: string }
export interface Ask { toolCallId: string; input: unknown; expiresAt: number; reach?: string; decision?: Decision }
export type Activity = { kind: "checking"; on?: string } | { kind: "compacting" } | { kind: "consent"; what: string };
export type TurnState =
  | { state: "running"; activity?: Activity; since: number }
  | { state: "waiting"; asks: string[]; expiresAt: number }
  | { state: "paused"; until: number }
  | { state: "idle"; stop: { kind: "completed" } | { kind: "interrupted" } | { kind: "stopped"; limit: string } | { kind: "failed"; error: string } }
  | { state: "unknown" } | { state: "resume-lost" };
export type Row =
  | { kind: "user"; key: string; i?: number; text: string }
  | { kind: "agent"; key: string; i?: number; text: string; writing: boolean }
  | { kind: "thought"; key: string; i?: number; text: string; streaming: boolean; seconds?: number }
  | { kind: "explored"; key: string; reads: number; searches: number; calls: ToolCall[] }
  | { kind: "tool"; key: string; i?: number; call: ToolCall; ask?: Ask }
  | { kind: "decision"; key: string; i?: number; who: string; what: string; reason?: string }
  | { kind: "footer"; key: string; seconds: number; tools: number; tokens?: number }
  | { kind: "unknown"; key: string; i?: number; raw: unknown };
export interface PlanItem { content: string; status: "pending"|"in_progress"|"completed" }
/** The reducer's own bookkeeping; a reader of the view ignores it. */
export interface Internal { skip?: boolean; thoughtAt?: number; seq?: number }
export interface SessionView {
  rows: Row[]; openAsks: string[]; turn: TurnState | null;
  queue: { n: number; input: string }[]; plan: PlanItem[] | null;
  usage: { used: number; window: number; compactAt?: number } | null;
  allows: { id: string; reach: string }[]; lastIndex: number; synced: boolean;
  x?: Internal;
}
export const emptyView: SessionView = {
  rows: [], openAsks: [], turn: null, queue: [], plan: null, usage: null,
  allows: [], lastIndex: 0, synced: false,
};
export interface Actions {
  say(text: string): void;
  answer(toolCallId: string, decision: { outcome: "allow"|"deny"; for?: "session"; reason?: string }): void;
  stop(): void;
  takeBack(allowId: string): void;
}
