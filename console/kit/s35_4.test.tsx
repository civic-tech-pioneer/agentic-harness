// S35-4 AC1, AC3, AC4a: `start` on the watane kernel, the status line from the fake claude's recorded turn, the alert sentences.
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { emptyView, type SessionView } from "./model";
import { ThreadProvider, Transcript } from "./thread";
import { reduce } from "./reduce";
import { StatusLine } from "./status";
import { errorWords } from "./errors";
import { wataneKernel } from "./watane";
import { fakeWatane } from "./watane_fake";
import fixture from "./fixtures/fake_claude_turn.json";

const { live: turn, replay } = fixture as { live: string[]; replay: string[] };
import { fakeKernel } from "../fake/kernel";

afterEach(cleanup);
const actions = { say: () => {}, answer: () => {}, stop: () => {}, takeBack: () => {} };
const status = (v: SessionView) => render(<StatusLine view={v} now={1795255996000} stopping={false} actions={actions} onNeedsYou={() => {}} />);
const word = () => screen.getByRole("status").textContent;
const shape = () => document.querySelector("[data-shape]")!.getAttribute("data-shape");
const feed = (n: number) => turn.slice(0, n).reduce<SessionView>(reduce, emptyView);
// the live fixture's steps: the attach replay (user entry, turn entry, synced, turn running, queue), the delta, the finishing update, turn idle
const ATTACHED = 5, DELTA = 6, FINISHED = 7, IDLE = 8;

describe("S35-4 AC1", () => {
  it("ac1_wataneKernel_start_calls_sessions_start_and_resolves_the_id", async () => {
    const f = fakeWatane({ "sessions.start": () => ({ ok: "sess1" }) });
    expect(await wataneKernel(f.w).start("k1", "scratch", "{}", "hi")).toBe("sess1");
    expect(f.calls).toEqual([{ fn: "sessions.start", args: { key: "k1", profile: "scratch", overlay: "{}", input: "hi" } }]);
  });
  it("ac1_a_refused_start_rejects_with_its_case", async () => {
    const f = fakeWatane({ "sessions.start": () => ({ err: { backend: "boom" } }) });
    await expect(wataneKernel(f.w).start("k", "scratch", "{}", "hi")).rejects.toMatchObject({ kind: "backend", payload: "boom" });
  });
  it("ac1_the_scripted_fake_starts_a_new_session_that_plays_its_story", async () => {
    const old = JSON.stringify({ v: 1, i: 1, kind: "message", at: 0, body: { role: "user", content: [{ type: "text", text: "old" }] } });
    const k = fakeKernel([[turn[0]!]], { preload: [old] });
    const id = await k.start("k1", "scratch", "{}", "hi");
    expect(id).not.toBe(await k.start("k2", "scratch", "{}", "hi"));
    expect(k.calls[0]).toEqual({ name: "start", args: ["k1", "scratch", "{}", "hi"] });
    await new Promise((r) => setTimeout(r, 10));
    const read = (await k.read(id, 0, 256)).map((e) => JSON.parse(e).body.content[0].text);
    expect(read).toEqual(["hi"]); // the story's first step, not the entries stored before
  });
});

describe("S35-4 AC3 (18.3, 18.5 from the fake claude's turn)", () => {
  it("ac3_the_user_message_is_a_right_panel_row", () => {
    expect(feed(ATTACHED).rows).toMatchObject([{ kind: "user", text: "hi" }]);
  });
  it("ac3_running_names_Working_with_the_dot", () => {
    status(feed(ATTACHED));
    expect(word()).toBe("Working");
    expect(shape()).toBe("dot");
  });
  it("ac3_a_streaming_thought_is_Thinking", () => {
    const thought = JSON.stringify({ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "hm" }, _meta: { watane: { at: 1 } } });
    status(reduce(feed(ATTACHED), thought));
    expect(word()).toBe("Thinking");
  });
  it("ac3_a_chunk_shows_Writing_and_the_entry_replaces_it", () => {
    const v = feed(DELTA);
    expect(v.rows[1]).toMatchObject({ kind: "agent", text: "This is the fake claude.", writing: true });
    status(v);
    expect(word()).toBe("Writing");
    expect(v.rows).toHaveLength(2); // the delta alone: one row
    const done = feed(FINISHED); // 9.4: the finishing update carries the whole message and replaces the streamed text
    expect(done.rows[1]).toMatchObject({ kind: "agent", i: 3, text: "This is the fake claude.", writing: false });
    expect(done.rows).toHaveLength(2);
  });
  it("ac3_idle_completed_shows_Idle_with_the_ring", () => {
    const v = feed(IDLE);
    expect(v.turn).toMatchObject({ state: "idle", stop: { kind: "completed" } });
    status(v);
    expect(word()).toBe("Idle");
    expect(shape()).toBe("ring");
    expect(v.rows.filter((r) => r.kind === "agent")).toHaveLength(1);
    expect(v.rows.filter((r) => r.kind === "agent")[0]).toMatchObject({ text: "This is the fake claude." }); // once, not twice
  });
  it("ac3_after_a_reload_the_agent_text_shows_exactly_once_and_Idle", () => {
    const v = replay.reduce<SessionView>(reduce, emptyView);
    expect(v.rows.filter((r) => r.kind === "agent").map((r) => (r as { text: string }).text)).toEqual(["This is the fake claude."]);
    expect(v.rows.filter((r) => r.kind === "user")).toHaveLength(1);
    status(v);
    expect(word()).toBe("Idle");
    expect(shape()).toBe("ring");
  });
  it("ac3_live_and_reload_draw_the_agent_text_once_in_the_page", () => {
    for (const msgs of [turn, replay]) {
      cleanup();
      const v = msgs.reduce<SessionView>(reduce, emptyView);
      render(<ThreadProvider view={v} actions={actions}><Transcript view={v} actions={actions} /></ThreadProvider>);
      expect((document.body.textContent ?? "").split("This is the fake claude.").length - 1).toBe(1);
    }
  });
  it("ac3_failed_no_binary_says_it_in_words_with_the_triangle", () => {
    const v = { ...emptyView, turn: { state: "idle", stop: { kind: "failed", error: "provider-ended", cause: "no-binary" } } } as SessionView;
    status(v);
    expect(word()).toBe("Failed: Claude Code is not available here");
    expect(shape()).toBe("triangle");
  });
  it("ac3_failed_with_another_cause_shows_its_text_and_the_wire_keeps_it", () => {
    const raw = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "idle", stop: { kind: "failed", error: "provider-ended", cause: "version 2.1.200 < 2.1.295" } } } } });
    const v = reduce(emptyView, raw);
    status(v);
    expect(word()).toBe("Failed: version 2.1.200 < 2.1.295");
  });
});

describe("S35-4 AC4a", () => {
  const e = (kind: string, payload?: unknown) => Object.assign(new Error(`sessions.say: ${kind} ${JSON.stringify(payload) ?? ""}`), { kind, payload });
  it("ac4a_each_2_3_error_is_a_sentence_never_json", () => {
    expect(errorWords(e("busy"))).toBe("Claude is busy with another session; try again in a moment.");
    expect(errorWords(e("backend", "disk full"))).toBe("The kernel could not do that: disk full");
    const cases: [string, unknown?][] = [["queue-full"], ["quota"], ["paused", 5], ["refused", [{ field: "profile", reason: "not allowed" }]], ["too-large", 100], ["not-owner"], ["unknown-session"],
      ["unknown-input"], ["unknown-allow"], ["behind", 3], ["host"], ["unreadable"]];
    for (const [k, p] of cases) {
      const w = errorWords(Object.assign(e(k, p), { refusals: k === "refused" ? p : undefined }));
      expect(w, k).toMatch(/^[A-Z][^{}[\]"]*[.:a-z0-9]$/);
      expect(w, k).not.toMatch(/[{}]/);
    }
    expect(errorWords(Object.assign(e("refused", []), { refusals: [{ field: "profile", reason: "not allowed" }] }))).toContain("not allowed");
  });
  it("ac4a_a_plain_error_says_its_message", () => {
    expect(errorWords(new Error("quota: too many"))).toBe("quota: too many");
    expect(errorWords(undefined)).toBe("Could not send that message.");
  });
});
