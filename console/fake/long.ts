// TECHNICAL 9.4, 18.3, 18.7: a long session as a kernel script, for the console's large-transcript tests and `?long=N` in dev.
// Step 0 holds `n` finished entries (indexes 1..n) and a running turn; each later step is one agent delta of a fast stream.
// The one ask that is still open sits near the start of the history (entry 7), so Needs you has to travel far.
const j = JSON.stringify;
const T0 = Date.now();
const text = (t: string) => ({ type: "text", text: t });
const upd = (u: object, meta?: { i?: number; at?: number }) =>
  j({ ...u, ...(meta ? { _meta: { watane: { ...meta, ...(meta.at !== undefined ? { at: T0 + meta.at } : {}) } } } : {}) });
const call = (id: string, title: string, kind: string, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call", toolCallId: id, title, kind, status: "pending", ...extra });
const done = (id: string, i: number, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call_update", toolCallId: id, status: "completed", ...extra }, { i, at: i * 1000 });
const diff = (path: string, oldText: string, newText: string) => [{ type: "diff", path, oldText, newText }];
const ask = (i: number, id: string, call: object, decision?: object) => j({ _meta: { watane: { type: "entry", entry: { v: 1, i, kind: "ask", at: T0 + i * 1000,
  body: { id: `a${id}`, kind: "ask", toolCallId: id, reach: "edits in ~/work/sync", input: { file_path: `src/f${id}.ts` }, "expires-at": T0 + 3_600_000, toolCall: call, ...(decision ? { decision } : {}) } } } } });

const OPEN_ASK_AT = 7;

/** The messages that make entry `i` (1-based), the entries of one cycle of 8 taking turns. */
function entryMessages(i: number): string[] {
  const t = `${i}`;
  switch ((i - 1) % 8) {
    case 0: return [upd({ sessionUpdate: "user_message_chunk", content: text(`Step ${t}: please look at module ${t} and tidy its retry handling.`) }, { i, at: i * 1000 })];
    case 1: return [
      upd({ sessionUpdate: "agent_thought_chunk", content: text(`Module ${t} retries without a delay. `) }, { at: i * 1000 - 400 }),
      upd({ sessionUpdate: "agent_thought_chunk", content: text("A backoff would fix it.") }, { at: i * 1000 - 200 }),
      upd({ sessionUpdate: "agent_message_chunk", content: text(`I will read module ${t} first, then change the delay.`) }, { i, at: i * 1000 }),
    ];
    case 2: return [call(`r${t}`, `Read mod${t}.ts`, "read", { locations: [{ path: `src/mod${t}.ts` }] }), done(`r${t}`, i)];
    case 3: return [call(`s${t}`, `Search retry in mod${t}`, "search"), done(`s${t}`, i)];
    case 4: {
      const path = `src/mod${t}.ts`, a = "delay = 0;", b = "delay = base * 2 ** attempt;";
      return [call(`e${t}`, `Edit ${path}`, "edit", { locations: [{ path, line: 12 }] }), done(`e${t}`, i, { content: diff(path, a, b) })];
    }
    case 5: return [call(`x${t}`, `npm test -- mod${t}`, "execute"),
      done(`x${t}`, i, { content: [{ type: "content", content: text(Array.from({ length: 6 }, (_, k) => `ok  mod${t} case ${k + 1}`).join("\n")) }] })];
    case 6: {
      const c = { toolCallId: `k${t}`, title: `Edit src/f${t}.ts`, kind: "edit", status: "pending", locations: [{ path: `src/f${t}.ts` }],
        content: diff(`src/f${t}.ts`, "a = 1;", "a = 2;") };
      return [ask(i, `k${t}`, c, i === OPEN_ASK_AT ? undefined : { outcome: "allow", by: "you" })];
    }
    default: return [upd({ sessionUpdate: "agent_message_chunk", content: text(`Done with module ${t}: the delay now backs off.`) }, { i, at: i * 1000 })];
  }
}

/** `n` finished entries, then (as the first step's last message) a running turn; then `deltas` steps of one agent delta each. */
export function longStory(n: number, deltas = 3000): string[][] {
  const first: string[] = [];
  for (let i = 1; i <= n; i++) first.push(...entryMessages(i));
  first.push(j({ _meta: { watane: { type: "turn", state: { state: "running", since: T0 + n * 1000 } } } }));
  return [first, ...Array.from({ length: deltas }, (_, k) => [upd({ sessionUpdate: "agent_message_chunk", content: text(`word${k} `) })])];
}
