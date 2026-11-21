// @vitest-environment node
// S30-1: the console draws a short decision (TECHNICAL 9.4 "The short decision", 18.3 the `ask` rows, 9.8 rule 4).
//
// TECHNICAL 9.4, "The short decision" (S29): "A decision written while the owner or the kernel's account is past (9.3 "Past") does not
// repeat the ask. Its body is {"id":"<ask id>","kind":"ask"|"check","ask-i":<i>,"decision":{...}}. `ask-i` is the index of the ask entry;
// `decision` is as above. It has no `toolCallId`, `toolCall`, `input`, `expires-at` or `reach` of its own; a reader tells it from a full
// decision by `ask-i`. A client draws its card from the ask entry at `ask-i`: from the entries it holds when that one is among them, else by
// reading that one entry (`read` with `after: ask-i - 1`, `max: 1`; when that line is a `part` with `m > 1`, `read` with `after: ask-i - m`,
// `max: m`, and join the parts)."
import { describe, expect, it } from "vitest";
import { fakeKernel, partsOf } from "../fake/kernel";
import { askEntry, shortDecided, shortNotHeld } from "../fake/asks";
import { emptyView, type Row, type SessionView } from "./model";
import { openSession } from "./kernel";
import { openAsk, reduce, settleShort } from "./reduce";
import { parse } from "./wire";

const J = JSON.stringify;
const entry = (i: number, kind: string, body: object) => J({ v: 1, i, kind, at: i, body });
const ask = (i: number, id: string, call: string, kind = "ask", extra: object = {}) => entry(i, "ask", { id, kind, toolCallId: call, input: { file_path: `src/${call}.ts` }, "expires-at": 99, toolCall: { toolCallId: call, title: `Edit src/${call}.ts`, kind: "edit", status: "pending", locations: [{ path: `src/${call}.ts` }] }, ...extra });
const short = (i: number, askI: number, id: string, d: object = { outcome: "allow", by: "alice" }, kind = "ask", extra: object = {}) => entry(i, "ask", { id, kind, "ask-i": askI, decision: d, ...extra });
const full = (i: number, id: string, call: string, d: object, kind = "ask") => ask(i, id, call, kind, { decision: d });
const run = (...ms: string[]) => ms.reduce(reduce, emptyView);
const tool = (v: SessionView, call: string) => v.rows.find((r): r is Extract<Row, { kind: "tool" }> => r.kind === "tool" && r.call.id === call);
const line = (v: SessionView, i: number) => v.rows.find((r) => r.key === `e${i}`);

describe("AC1 parse", () => {
  it("ac1_a_short_decision_needs_only_id_kind_ask_i_and_a_decision", () => {
    expect(parse(short(7, 3, "a1", { outcome: "deny", by: "kernel", reason: "cancel", for: "session" }))).toEqual({ t: "short", i: 7, id: "a1", kind: "ask", askI: 3, decision: { outcome: "deny", by: "kernel", reason: "cancel", for: "session" } });
    expect(parse(short(7, 3, "c1", { outcome: "none", by: "checked" }, "check"))).toMatchObject({ t: "short", kind: "check", decision: { outcome: "none", by: "checked" } });
  });
  it("ac1_bad_ask_i_or_a_missing_or_bad_decision_is_unreadable", () => {
    const bad = (m: string) => expect(parse(m).t).toBe("unreadable");
    for (const k of [0, -1, 1.5, "3", null, 7, 8, 100]) bad(entry(7, "ask", { id: "a", kind: "ask", "ask-i": k, decision: { outcome: "allow", by: "x" } }));
    bad(entry(7, "ask", { id: "a", kind: "ask", "ask-i": 3 }));
    bad(entry(7, "ask", { id: "a", kind: "ask", "ask-i": 3, decision: { outcome: "maybe", by: "x" } }));
    bad(entry(7, "ask", { id: "a", kind: "ask", "ask-i": 3, decision: { outcome: "allow" } }));
    bad(entry(7, "ask", { id: "a", kind: "other", "ask-i": 3, decision: { outcome: "allow", by: "x" } }));
    bad(entry(7, "ask", { id: 5, kind: "ask", "ask-i": 3, decision: { outcome: "allow", by: "x" } }));
  });
  it("ac1_stray_fields_are_ignored_and_a_full_decision_parses_as_before", () => {
    const m = parse(short(7, 3, "a1", undefined, "ask", { toolCallId: "STRAY", input: { x: 1 }, "expires-at": 5, reach: "r", toolCall: { toolCallId: "STRAY", title: "stray" } }));
    expect(m).toEqual({ t: "short", i: 7, id: "a1", kind: "ask", askI: 3, decision: { outcome: "allow", by: "alice" } });
    const f = parse(full(7, "a1", "e1", { outcome: "allow", by: "alice" }));
    expect(f).toMatchObject({ t: "ask", ask: { toolCallId: "e1", decision: { outcome: "allow", by: "alice" } } });
    expect(parse(entry(7, "ask", { id: "a1", kind: "ask", input: {}, "expires-at": 1 })).t).toBe("unreadable"); // no toolCallId, no ask-i
  });
});

describe("AC2 held", () => {
  const decisions = [{ outcome: "allow", by: "alice" }, { outcome: "deny", by: "alice", reason: "no" }, { outcome: "none", by: "checked" }, { outcome: "allow", by: "alice", for: "session" }, { outcome: "deny", by: "kernel", reason: "cancel", for: "session" }];
  for (const kind of ["ask", "check"]) for (const d of decisions) {
    it(`ac2_held_short_equals_full_${kind}_${d.outcome}_${d.reason ?? "noreason"}_${d.for ?? "once"}`, () => {
      const pre = [entry(1, "message", { role: "user", content: [{ type: "text", text: "hi" }] }), ask(2, "a1", "e1", kind), ask(3, "a2", "e2", "ask")];
      const f = run(...pre, full(4, "a1", "e1", d, kind));
      const s = run(...pre, short(4, 2, "a1", d, kind));
      // the keys are the same too: both rows are `e4`
      expect(s).toEqual(f);
      expect(s.openAsks).toEqual(["a2"]);
      expect(openAsk(s, "a1")).toBeUndefined();
      expect(tool(s, "e1")!.ask!.decision).toEqual(d);
    });
  }
  it("ac2_held_short_equals_full_for_an_ask_without_its_own_call", () => {
    const bare = entry(2, "ask", { id: "a1", kind: "ask", toolCallId: "e1", input: { command: "ls" }, "expires-at": 9 });
    expect(run(bare, short(3, 2, "a1"))).toEqual(run(bare, entry(3, "ask", { id: "a1", kind: "ask", toolCallId: "e1", input: { command: "ls" }, "expires-at": 9, decision: { outcome: "allow", by: "alice" } })));
  });
});

describe("AC4 not held: closes at once, a placeholder stands", () => {
  const pre = [entry(1, "message", { role: "user", content: [{ type: "text", text: "hi" }] })];
  it("ac4_page_lacks_the_ask_a_placeholder_row_names_the_id", () => {
    const v = run(...pre, short(41, 40, "a20", { outcome: "deny", by: "kernel", reason: "cancel" }));
    expect(v.rows[1]).toEqual({ kind: "decision", key: "e41", i: 41, who: "kernel", what: "Denied: a20", reason: "cancel" });
    expect(v.x!.shorts).toEqual([{ i: 41, askI: 40, id: "a20", kind: "ask", decision: { outcome: "deny", by: "kernel", reason: "cancel" } }]);
    expect(v.openAsks).toEqual([]);
  });
  it("ac4_an_open_ask_of_another_index_closes_and_its_card_loses_the_bar", () => {
    const v = run(...pre, ask(3, "a1", "e1"), short(6, 5, "a1"));
    expect(v.openAsks).toEqual([]);
    expect(openAsk(v, "a1")).toBeUndefined();
    expect(tool(v, "e1")!.ask).toBeUndefined();
    expect(line(v, 6)).toMatchObject({ kind: "decision", what: "Allowed: a1" });
  });
  it("ac4_a_check_that_is_not_an_open_ask_leaves_openasks_alone", () => {
    const v = run(ask(2, "a2", "e2"), ask(3, "c1", "e1", "check"), short(6, 5, "c1", undefined, "check"));
    expect(v.openAsks).toEqual(["a2"]);
    expect(tool(v, "e1")!.ask).toBeUndefined();
  });
  it("ac4_the_ask_entry_read_draws_the_card_and_the_line_at_the_placeholders_place", () => {
    const v0 = run(...pre, short(6, 5, "a1"), entry(7, "message", { role: "agent", content: [{ type: "text", text: "after" }] }));
    expect(v0.rows.map((r) => r.key)).toEqual(["e1", "e6", "e7"]);
    const v = settleShort(v0, 6, ask(5, "a1", "e1"));
    expect(v.rows.map((r) => r.key)).toEqual(["e1", "ce1", "e6", "e7"]); // the card stands before its decision line
    expect(tool(v, "e1")!.ask).toMatchObject({ id: "a1", decision: { outcome: "allow", by: "alice" } });
    expect(tool(v, "e1")!.call.title).toBe("Edit src/e1.ts");
    expect(line(v, 6)).toMatchObject({ what: "Allowed: Edit src/e1.ts", who: "alice" });
    expect(v.x!.shorts).toEqual([]);
    expect(v.openAsks).toEqual([]);
  });
  it("ac4_a_card_the_page_held_stays_where_it_was_and_takes_the_asks_call", () => {
    const v0 = run(J({ sessionUpdate: "tool_call", toolCallId: "e1", title: "streamed", kind: "edit", status: "in_progress" }), entry(2, "message", { role: "agent", content: [{ type: "text", text: "x" }] }), short(6, 5, "a1"));
    const v = settleShort(v0, 6, ask(5, "a1", "e1"));
    expect(v.rows.map((r) => r.key)).toEqual(["ce1", "e2", "e6"]);
    expect(tool(v, "e1")!.call.title).toBe("Edit src/e1.ts");
    expect(tool(v, "e1")!.call.status).toBe("pending");
    expect(tool(v, "e1")!.ask!.decision!.outcome).toBe("allow");
  });
  it("ac4_the_read_entry_of_a_read_call_leaves_the_explored_row_alone", () => {
    const rd = J({ sessionUpdate: "tool_call", toolCallId: "r1", title: "Read a", kind: "read", status: "completed" });
    const v0 = run(rd, short(6, 5, "a1"));
    const v = settleShort(v0, 6, entry(5, "ask", { id: "a1", kind: "ask", toolCallId: "r2", input: { file_path: "b" }, "expires-at": 9, toolCall: { toolCallId: "r2", title: "Read b", kind: "read" } }));
    expect(v.rows.map((r) => r.key)).toEqual(["xr1", "cr2", "e6"]);
  });
});

describe("AC5 fail closed", () => {
  const base = () => run(short(6, 5, "a1", { outcome: "deny", by: "kernel", reason: "cancel" }));
  const closed = (v: SessionView) => {
    expect(v.rows.filter((r) => r.kind === "tool")).toEqual([]);
    expect(v.openAsks).toEqual([]);
    expect(line(v, 6)).toEqual({ kind: "decision", key: "e6", i: 6, who: "kernel", what: "Denied: a1", reason: "cancel" });
    expect(v.x!.shorts).toEqual([]);
  };
  const cases: [string, string | null][] = [
    ["the read failed", null],
    ["another id", ask(5, "a2", "e1")],
    ["another kind", ask(5, "a1", "e1", "check")],
    ["a decision of its own", full(5, "a1", "e1", { outcome: "allow", by: "x" })],
    ["another index", ask(4, "a1", "e1")],
    ["a short decision", short(5, 3, "a1")],
    ["not an ask", entry(5, "message", { role: "user", content: [] })],
    ["does not parse", "{not json"],
    ["unreadable ask", entry(5, "ask", { id: "a1", kind: "ask" })],
  ];
  for (const [name, e] of cases) it(`ac5_${name.replace(/\W+/g, "_")}_draws_no_card`, () => closed(settleShort(base(), 6, e)));
});

describe("AC6 later state wins", () => {
  it("ac6_a_reused_id_keeps_its_open_card_and_bar_when_the_read_returns", () => {
    const v0 = run(ask(4, "a1", "e1"), short(6, 5, "a1"), ask(9, "a1", "e2"));
    expect(v0.openAsks).toEqual(["a1"]);
    const open = openAsk(v0, "a1")!;
    const v = settleShort(v0, 6, ask(5, "a1", "e1"));
    expect(openAsk(v, "a1")).toBe(open);
    expect(v.openAsks).toEqual(["a1"]);
    expect(tool(v, "e2")!.ask).toBe(open);
    expect(tool(v, "e1")!.ask!.decision).toBeTruthy();
    expect(v.turn).toBe(v0.turn);
    expect(tool(v, "e2")).toBe(tool(v0, "e2"));
  });
  it("ac6_a_reuse_on_the_same_call_keeps_the_open_ask_and_draws_no_second_card", () => {
    const v0 = run(ask(4, "a1", "e1"), short(6, 5, "a1"), ask(9, "a1", "e1"));
    const open = openAsk(v0, "a1")!;
    const v = settleShort(v0, 6, ask(5, "a1", "e1"));
    expect(tool(v, "e1")!.ask).toBe(open);
    expect(v.rows.filter((r) => r.kind === "tool").length).toBe(1);
    expect(line(v, 6)).toMatchObject({ what: "Allowed: Edit src/e1.ts" });
  });
  it("ac6_settling_twice_or_for_a_row_that_is_gone_changes_nothing", () => {
    const v = settleShort(run(short(6, 5, "a1")), 6, ask(5, "a1", "e1"));
    expect(settleShort(v, 6, ask(5, "a1", "e1"))).toBe(v);
  });
});

// the kernel loop
const flush = (ms = 80) => new Promise((r) => setTimeout(r, ms));
function open(k: ReturnType<typeof fakeKernel>) {
  const views: SessionView[] = [];
  const s = openSession(k, "s1", (v) => views.push(v));
  return { s, views, last: () => views[views.length - 1]! };
}
const reads = (k: ReturnType<typeof fakeKernel>) => k.calls.filter((c) => c.name === "read").map((c) => c.args.slice(1));

describe("AC4 the read", () => {
  it("ac4_not_held_reads_the_entry_once_and_draws_the_card_at_the_placeholder", async () => {
    const k = fakeKernel([shortNotHeld.step], { preload: shortNotHeld.stored });
    const att = k.attach.bind(k);
    k.attach = (s, after, ...r) => att(s, Math.max(after, 40), ...r); // this console attached past the ask entry: it holds no ask
    const a = open(k);
    await flush();
    k.play(0);
    await flush();
    const v = a.last();
    a.s.close();
    const parts = JSON.parse(shortNotHeld.stored[shortNotHeld.stored.length - 1]!).body.m as number;
    expect(parts).toBeGreaterThan(1);
    expect(reads(k).filter((r) => r[0] === 39 && r[1] === 1)).toHaveLength(1); // read(session, ask-i - 1, 1)
    expect(reads(k).some((r) => r[0] === 40 - parts && r[1] === parts)).toBe(true); // the whole group
    expect(tool(v, "e20")!.ask).toMatchObject({ id: "a20", decision: { outcome: "deny", by: "kernel" } });
    expect(v.rows.findIndex((r) => r.kind === "tool")).toBeLessThan(v.rows.findIndex((r) => r.key === "e41"));
    expect(line(v, 41)).toMatchObject({ what: "Denied: Edit src/e20.ts", reason: "cancel" });
    expect(v.openAsks).toEqual([]);
  });
  it("ac4_a_failed_read_retries_three_times_then_fails_closed", async () => {
    const k = fakeKernel([[short(6, 5, "a1")]], { preload: [] });
    let n = 0;
    const read = k.read.bind(k);
    k.read = async (s, after, max) => { if (after === 4 && max === 1) { n++; throw new Error("down"); } return read(s, after, max); };
    const a = open(k);
    await flush();
    k.play(0);
    await flush(2400);
    a.s.close();
    expect(n).toBe(3);
    expect(a.last().rows.filter((r) => r.kind === "tool")).toEqual([]);
    expect(line(a.last(), 6)).toMatchObject({ what: "Allowed: a1" });
  });
  it("ac4_a_read_that_returns_a_non_ask_draws_no_card", async () => {
    const k = fakeKernel([[short(6, 5, "a1")]], { preload: [entry(5, "message", { role: "agent", content: [{ type: "text", text: "m" }] })] });
    const a = open(k);
    await flush(); k.play(0); await flush();
    a.s.close();
    expect(a.last().rows.filter((r) => r.kind === "tool")).toEqual([]);
    expect(reads(k).filter((r) => r[0] === 4)).toHaveLength(1);
  });
});

describe("AC6 and AC7 in flight", () => {
  const gate = () => { let go!: () => void; const p = new Promise<void>((r) => { go = r; }); return { p, go }; };
  it("ac6_later_entries_are_reduced_while_the_read_waits_and_the_read_touches_only_its_own", async () => {
    const k = fakeKernel([[ask(4, "a1", "e1")], [short(6, 5, "a1")], [ask(9, "a1", "e2")]], { preload: [ask(5, "a1", "e1")] });
    const att = k.attach.bind(k);
    k.attach = (s, after, ...r) => att(s, Math.max(after, 5), ...r); // the ask at 5 is stored, but this console attached past it
    const g = gate();
    const read = k.read.bind(k);
    k.read = async (s, a, m) => { if (a === 4 && m === 1) await g.p; return read(s, a, m); };
    const a = open(k);
    await flush();
    k.play(1); k.play(2);
    await flush();
    expect(a.last().openAsks).toEqual(["a1"]); // reduced while the read is in flight
    g.go();
    await flush();
    const v = a.last();
    a.s.close();
    expect(v.openAsks).toEqual(["a1"]);
    expect(tool(v, "e2")!.ask!.decision).toBeUndefined();
    expect(openAsk(v, "a1")!.toolCallId).toBe("e2");
    expect(tool(v, "e1")!.ask!.decision!.outcome).toBe("allow");
  });
  it("ac7_a_read_that_returns_after_close_changes_nothing", async () => {
    const k = fakeKernel([[short(6, 5, "a1")]], { preload: [] });
    const g = gate();
    const read = k.read.bind(k);
    k.read = async (s, a, m) => { if (a === 4) await g.p; return read(s, a, m); };
    const a = open(k);
    await flush(); k.play(0); await flush();
    const n = a.views.length;
    a.s.close();
    g.go();
    await flush();
    expect(a.views.length).toBe(n);
  });
});

describe("AC8 paging", () => {
  it("ac8_the_ask_in_an_older_page_of_the_range_is_held_and_nothing_is_read_for_it", async () => {
    const pad = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, n) => entry(a + n, "message", { role: "agent", content: [{ type: "text", text: `m${a + n}` }] }));
    // 600 entries behind: the ask at 20 is in the oldest page, its short decision at 590 in the newest
    const stored = [...pad(1, 19), ask(20, "a1", "e1"), ...pad(21, 589), short(590, 20, "a1"), ...pad(591, 600)];
    const k = fakeKernel([], { preload: stored });
    const a = open(k);
    await flush(300);
    const v = a.last();
    a.s.close();
    expect(v.lastIndex).toBe(600);
    expect(reads(k).some((r) => r[1] === 1 && r[0] === 19)).toBe(false);
    expect(v.x?.shorts ?? []).toEqual([]);
    expect(tool(v, "e1")!.ask!.decision!.outcome).toBe("allow");
    expect(line(v, 590)).toMatchObject({ what: "Allowed: Edit src/e1.ts" });
    expect(v.openAsks).toEqual([]);
  });
});

describe("AC9 fake", () => {
  it("ac9_the_fixtures_are_short_decisions_and_the_not_held_ask_is_stored_as_parts", () => {
    expect(parse(shortDecided(10, 8, "a8")[0]!).t).toBe("short");
    expect(shortNotHeld.stored.some((e) => JSON.parse(e).kind === "part")).toBe(true);
    expect(partsOf(askEntry(40, "a20", "e20"), 120).length).toBeGreaterThan(1);
  });
});
