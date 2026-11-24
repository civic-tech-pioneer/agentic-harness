// TECHNICAL 9.4, 9.5, 18.3: a ChatGPT session (profile `Sort`) as a kernel stream: text deltas, a toolset call with its output,
// the turn's footer, then a usage-limit pause that the kernel later lifts on its own.
const j = JSON.stringify;
const w = (m: object) => j({ _meta: { watane: m } });
const T0 = Date.now();
const upd = (u: object, meta?: { i?: number; at?: number }) =>
  j({ ...u, ...(meta ? { _meta: { watane: { ...meta, ...(meta.at !== undefined ? { at: T0 + meta.at } : {}) } } } : {}) });
const text = (t: string) => ({ type: "text", text: t });

/** The pause lasts to this time; the page shows it as `hh:mm`. */
export const pausedUntil = T0 + 3_600_000;

export const chatgptStory: string[][] = [
  [
    upd({ sessionUpdate: "user_message_chunk", content: text("Tag the untagged notes") }, { i: 1, at: 1000 }),
    w({ type: "turn", state: { state: "running", since: T0 + 1000 } }),
    upd({ sessionUpdate: "agent_message_chunk", content: text("I will list ") }, { at: 1500 }),
    upd({ sessionUpdate: "agent_message_chunk", content: text("the notes first.") }, { at: 1600 }),
    upd({ sessionUpdate: "agent_message_chunk", content: text("I will list the notes first.") }, { i: 2, at: 1700 }),
  ],
  [
    upd({ sessionUpdate: "tool_call", toolCallId: "n1", title: "notes: list", kind: "other", status: "pending", rawInput: { untagged: true } }),
    upd({ sessionUpdate: "tool_call_update", toolCallId: "n1", status: "completed", rawOutput: "3 untagged notes: a.md, b.md, c.md" }, { i: 3, at: 2000 }),
    upd({ sessionUpdate: "agent_message_chunk", content: text("Three notes need tags.") }, { i: 4, at: 2500 }),
    w({ type: "usage", "context-used": 12000, "context-window": 128000 }),
    w({ type: "turn", state: { state: "idle", stop: { kind: "completed" }, seconds: 2, tools: 1, tokens: 900 } }),
  ],
  [w({ type: "turn", state: { state: "paused", until: pausedUntil } })],
  [w({ type: "turn", state: { state: "running", since: T0 + 5000 } })],
];
