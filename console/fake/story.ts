// TECHNICAL 9.4, 18.3: the slice's session as a kernel stream: steps 0-2 finish every call, step 3 leaves an ask open.
const j = JSON.stringify;
const w = (m: object) => j({ _meta: { watane: m } });
/** The story's times are relative to when this module loads, so a story played to a running step shows seconds, not hours. */
const T0 = Date.now();
const upd = (u: object, meta?: { i?: number; at?: number }) =>
  j({ ...u, ...(meta ? { _meta: { watane: { ...meta, ...(meta.at !== undefined ? { at: T0 + meta.at } : {}) } } } : {}) });
const text = (t: string) => ({ type: "text", text: t });
const call = (id: string, title: string, kind: string, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call", toolCallId: id, title, kind, status: "pending", ...extra });
const done = (id: string, i: number, at: number, extra: object = {}) =>
  upd({ sessionUpdate: "tool_call_update", toolCallId: id, status: "completed", ...extra }, { i, at });
const diff = (path: string, oldText: string, newText: string) => [{ type: "diff", path, oldText, newText }];

/** The ask waits 10 minutes from when the fake is made, so the status line counts down. */
export const expiresAt = T0 + 600_000;

const syncOld = [...Array.from({ length: 6 }, (_, k) => `const step${k + 1} = ${k + 1};`), "retry(0);", ...Array.from({ length: 12 }, (_, k) => `const keep${k + 1} = ${k + 1};`), "export default sync;"].join("\n");
const syncNew = syncOld.replace("retry(0);", "retry(base);").replace("export default sync;", "export default sync;\nexport { retry };");

export const story: string[][] = [
  [
    upd({ sessionUpdate: "user_message_chunk", content: text("Fix the retry bug in sync.ts") }, { i: 1, at: 1000 }),
    w({ type: "turn", state: { state: "running", since: T0 + 1000 } }),
    upd({ sessionUpdate: "agent_thought_chunk", content: text("The retry loop ") }, { at: 1500 }),
    upd({ sessionUpdate: "agent_thought_chunk", content: text("never backs off.") }, { at: 2500 }),
    upd({ sessionUpdate: "agent_message_chunk", content: text("I will read the sync code first.") }, { i: 2, at: 4500 }),
  ],
  [
    call("r1", "Read sync.ts", "read", { locations: [{ path: "src/sync.ts" }] }),
    done("r1", 3, 5000),
    call("s1", "Search retry", "search"),
    done("s1", 4, 5200),
    call("r2", "Read retry.ts", "read", { locations: [{ path: "src/retry.ts" }] }),
    done("r2", 5, 5400),
    call("r3", "Read sync.test.ts", "read", { locations: [{ path: "src/sync.test.ts" }] }),
    done("r3", 6, 5600),
  ],
  [
    call("e1", "Edit src/retry.ts", "edit", { locations: [{ path: "src/retry.ts", line: 12 }] }),
    done("e1", 7, 6000, { content: diff("src/retry.ts", "delay = 0", "delay = base * 2 ** attempt") }),
    upd({ sessionUpdate: "plan", entries: [
      { content: "Read the retry code", priority: "medium", status: "completed" },
      { content: "Add backoff", priority: "medium", status: "in_progress" },
      { content: "Add a test", priority: "medium", status: "pending" },
    ] }, { at: 6100 }),
  ],
  [
    call("e2", "Edit src/sync.ts", "edit", { locations: [{ path: "src/sync.ts" }], content: diff("src/sync.ts", syncOld, syncNew) }),
    w({ type: "entry", entry: { v: 1, i: 8, kind: "ask", at: T0 + 7000, body: { id: "a8", kind: "ask", toolCallId: "e2", reach: "edits in ~/work/sync", input: { file_path: "src/sync.ts" }, "expires-at": expiresAt,
      toolCall: { toolCallId: "e2", title: "Edit src/sync.ts", kind: "edit", status: "pending", locations: [{ path: "src/sync.ts" }], content: diff("src/sync.ts", syncOld, syncNew) } } } }),
    w({ type: "turn", state: { state: "waiting", asks: ["a8"], "expires-at": expiresAt } }),
    w({ type: "queue", inputs: [{ n: 1, input: "also add a test" }] }),
    w({ type: "usage", "context-used": 42000, "context-window": 200000, "compact-at": 160000 }),
  ],
];
