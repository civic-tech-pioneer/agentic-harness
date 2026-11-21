// TECHNICAL 9.4, 18.6: a second open ask for the story's session, and the entry that decides an ask: fixtures of the asks' tests.
import { expiresAt } from "./story";
import { partsOf } from "./kernel";

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

/** The full ask entry `i` of ask `id` on call `call` (what the story's `a8` is, for any id); stored as is, or as `parts` through `partsOf`. */
export const askEntry = (i: number, id: string, call: string, kind: "ask" | "check" = "ask"): object =>
  ({ v: 1, i, kind: "ask", at: Date.now(), body: { id, kind, toolCallId: call, input: { file_path: `src/${call}.ts` }, "expires-at": expiresAt, toolCall: edit(call) } });

/** The short decision entry `i` of the ask at entry `askI` (9.4 "The short decision", S29): no ask fields, only `ask-i` and the decision. */
export const shortDecided = (i: number, askI: number, id: string, outcome: "allow" | "deny" | "none" = "allow", kind: "ask" | "check" = "ask"): string[] => [
  w({ type: "entry", entry: { v: 1, i, kind: "ask", at: Date.now(), body: { id, kind, "ask-i": askI, decision: { outcome, by: "alice" } } } }),
];

/**
 * A story step for a short decision that the view holds the ask of: the story's `a8` (entry 8) decided short at entry 10.
 * Play it after the story's steps 0-3.
 */
export const shortHeldStep: string[] = shortDecided(10, 8, "a8");

/**
 * The fixture of the not-held form: a stored session whose ask entry (`a20` on `e20`, entry 40, stored as `parts` of 120 bytes, so 
 * entries 31..40) is long past, followed by its short decision (entry 41). Pass `stored` as the fake kernel's `preload` and `step` as a script step.
 */
export const shortNotHeld = (() => {
  const ask = askEntry(40, "a20", "e20");
  const parts = partsOf(ask, 120);
  const first = 40 - parts.length + 1;
  const pad = Array.from({ length: first - 1 }, (_, k) => JSON.stringify({ v: 1, i: k + 1, kind: "message", at: k, body: { role: "agent", content: [{ type: "text", text: `m${k + 1}` }] } }));
  const decisionEntry = { v: 1, i: 41, kind: "ask", at: Date.now(), body: { id: "a20", kind: "ask", "ask-i": 40, decision: { outcome: "deny", by: "kernel", reason: "cancel" } } };
  return { stored: [...pad, ...parts], decisionEntry, step: [JSON.stringify(decisionEntry)] };
})();
