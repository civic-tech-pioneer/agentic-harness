// TECHNICAL 9.4, 18.6: a second open ask for the story's session, and the entry that decides an ask: fixtures of the asks' tests.
import { expiresAt } from "./story";

const w = (m: object) => JSON.stringify({ _meta: { watane: m } });
const edit = (id: string) => ({ toolCallId: id, title: `Edit src/${id}.ts`, kind: "edit", status: "pending", locations: [{ path: `src/${id}.ts` }],
  content: [{ type: "diff", path: `src/${id}.ts`, oldText: "a", newText: "b" }] });

/** A second ask, `a9` on the call `e3`, opening after the story's `a8`; it takes entry 9. */
export const secondAsk: string[] = [
  w({ type: "entry", entry: { v: 1, i: 9, kind: "ask", at: Date.now(), body: { id: "a9", kind: "ask", toolCallId: "e3", reach: "edits in ~/work/sync", input: { file_path: "src/e3.ts" }, "expires-at": expiresAt, toolCall: edit("e3") } } }),
  w({ type: "turn", state: { state: "waiting", asks: ["a8", "a9"], "expires-at": expiresAt } }),
];

/** The decision entry `i` of ask `id` on call `call` (an allow by alice); 9.4 repeats the ask in it. */
export const decided = (i: number, id: string, call: string, outcome: "allow" | "deny" = "allow"): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id, kind: "ask", toolCallId: call, input: { file_path: `src/${call}.ts` }, "expires-at": expiresAt,
    decision: { outcome, by: "alice" }, toolCall: edit(call) } } }),
];
