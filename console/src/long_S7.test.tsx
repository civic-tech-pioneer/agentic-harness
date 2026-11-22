// S7-2 AC1, AC7 (TECHNICAL 17, 18.7; P14): the console on a session of 5 000 entries with a fast stream running:
// what it shows (the end, at most 120 rows, input, Needs you, stream, Stop, scroll). `npm test` checks that with no time limit,
// so a loaded machine cannot fail it (S30-3); the limits for input, Stop, scroll and stream delay and the opening of the end
// are checked when WA_LONG_TIMING=1, which test/long_timing.slow.mjs sets (in `npm run test:slow`, alone on the machine). jsdom computes no layout, so the
// scroller's size is set here and rows stand for their estimated heights; real layout is test/long_session.slow.mjs.
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { fireEvent, screen } from "@testing-library/react";
import { fakeKernel, type FakeKernel } from "../fake/kernel";
import { longStory } from "../fake/long";
import { storyInfo } from "../fake/page";
import { ROW_ESTIMATE, ROW_GAP } from "../kit/thread";
import { App } from "./App";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = false; // the stream updates the page on its own frames
const N = 5000, VIEW = 800, PAD = 32, SLOT = ROW_ESTIMATE + ROW_GAP;
const idle = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "idle", stop: { kind: "interrupted" } } } } });
const running = JSON.stringify({ _meta: { watane: { type: "turn", state: { state: "running", since: 1 } } } });
const p95 = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.ceil(xs.length * 0.95) - 1]!;
const now = () => performance.now();
const TIMED = process.env.WA_LONG_TIMING === "1";
/** A time limit: asserted only in the timing run. */
const within = (name: string, ms: number, limit: number) => { if (TIMED) expect(ms, name).toBeLessThanOrEqual(limit); };
const hasButton = (name: string) => [...document.querySelectorAll("button")].some((b) => b.textContent === name || b.getAttribute("aria-label") === name);
const rowCount = () => document.querySelectorAll(".wa-row").length;

/** The page settles: one animation frame, then an idle wait, so render work left by the last step does not run inside the next sample. */
const settled = () => new Promise<void>((r) => requestAnimationFrame(() => setTimeout(r, 30)));

/** Resolves when pred holds, checked at once and after every DOM change. */
function until(pred: () => boolean, ms = TIMED ? 3000 : 60_000): Promise<void> {
  return new Promise((resolve, reject) => {
    if (pred()) return resolve();
    const mo = new MutationObserver(() => { if (pred()) { mo.disconnect(); clearTimeout(t); resolve(); } });
    const t = setTimeout(() => { mo.disconnect(); reject(new Error("timed out")); }, ms);
    mo.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true });
  });
}

const started = now();
const script = longStory(N, 4000);
const IDLE = script.length;
script.push([idle], [running]);
let fake: FakeKernel, scroller: HTMLElement, opened = 0, next = 1, stream: ReturnType<typeof setInterval>;
let top = 0;
let root: ReturnType<typeof createRoot>;
const delays: number[] = [];
let watcher: MutationObserver;
const total = () => Number(document.querySelector<HTMLElement>(".wa-transcript")!.dataset.total);
const scrollTo = (y: number) => { top = Math.max(0, Math.min(y, total() + PAD - VIEW)); fireEvent.scroll(scroller); };
const results: Record<string, number> = {};

beforeAll(async () => {
  fake = fakeKernel(script);
  fake.play(0);
  const t0 = now();
  // mounted by hand: one session serves every test of the file (the suite's cleanup would unmount a render() after the first)
  const host = document.body.appendChild(document.createElement("div"));
  root = createRoot(host);
  flushSync(() => root.render(<App kernel={fake} info={storyInfo} />));
  scroller = document.querySelector<HTMLElement>(".wa-scroll")!;
  Object.defineProperty(scroller, "clientHeight", { configurable: true, get: () => VIEW });
  Object.defineProperty(scroller, "scrollHeight", { configurable: true, get: () => total() + PAD });
  Object.defineProperty(scroller, "scrollTop", { configurable: true, get: () => top, set: (v: number) => { top = Math.max(0, Math.min(v, total() + PAD - VIEW)); } });
  fireEvent.scroll(scroller);
  await until(() => document.body.textContent!.includes(`Done with module ${N}`));
  opened = now() - t0;
  // the rest of the history is paged in (the governor lets 4 calls per second through) and shown: some 5 600 rows
  const paged = until(() => total() > 5000 * SLOT, TIMED ? 15_000 : 120_000); // its own guard timer is real
  // the governor's pacing is not what the limits are about: its clock (setTimeout, Date) is advanced by hand while the history comes in,
  // to the next timer only when one is pending, so the fake Date ends some seconds ahead (not hours, which a fixed step in a busy loop gave)
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date"] });
  let done = false;
  paged.then(() => { done = true; }, () => { done = true; });
  while (!done) { if (vi.getTimerCount()) await vi.advanceTimersToNextTimerAsync(); await new Promise<void>((r) => setImmediate(r)); }
  vi.useRealTimers();
  await paged;
  // the console attaches again once the history is in (the governor may make it wait for a token); a delta played before that is the fake's "message being written", which the console shows as Writing... until an entry ends it
  for (let k = 0; k < 150 && fake.calls.at(-1)?.name !== "attach"; k++) await new Promise((r) => setTimeout(r, 20));
  // 50 deltas a second; for each, the time from giving it to the fake until its text is in the DOM
  const sent = new Map<string, number>();
  const watch = new MutationObserver(() => {
    if (!sent.size) return;
    const text = document.body.textContent!;
    for (const [w, t] of sent) if (text.includes(w)) { sent.delete(w); delays.push(now() - t); }
  });
  watch.observe(document.body, { subtree: true, childList: true, characterData: true });
  watcher = watch;
  stream = setInterval(() => { if (next < IDLE) { sent.set(`word${next}`, now()); fake.play(next++); } }, 20);
  // the 5 000 rows reach the runtime after the paint: wait until the main thread is free again (one task takes under 30 ms)
  for (let k = 0; k < 40; k++) { const t = now(); await new Promise((r) => setTimeout(r, 5)); if (k > 2 && now() - t < 30) break; }
}, TIMED ? 30_000 : 240_000);
afterAll(() => { clearInterval(stream); watcher.disconnect(); root.unmount(); console.log("S7-2 p95 (ms):", JSON.stringify(results), "open end:", Math.round(opened)); });

vi.setConfig({ testTimeout: TIMED ? 5000 : 120_000 }); // no time limit in `npm test`: a loaded machine only runs it slower

describe("a session of 5 000 entries with the stream running", () => {
  it("ac7_opening_shows_the_end", () => {
    within("open end ms", opened, 2000);
    expect(rowCount()).toBeLessThanOrEqual(120);
    expect(rowCount()).toBeGreaterThan(5);
  });

  it("ac7_input_reaches_the_box", () => {
    const box = screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
    const ms: number[] = [];
    for (let k = 1; k <= 20; k++) {
      const t = now();
      fireEvent.change(box, { target: { value: "x".repeat(k) } });
      expect(box.value).toHaveLength(k);
      ms.push(now() - t);
    }
    results.input = p95(ms);
    within("input p95", results.input!, 50);
  });

  it("ac4_needs_you_reaches_the_open_ask_far_from_the_window_and_focuses_allow_once", () => {
    const card = document.querySelector<HTMLElement>("[data-asking]"); // an open ask is never unmounted, so it is there though its row is entry 7
    expect(card).not.toBeNull();
    expect(Number(card!.closest<HTMLElement>(".wa-row")!.dataset.index)).toBeLessThan(10);
    const scrollIntoView = vi.fn();
    Element.prototype.scrollIntoView = scrollIntoView;
    fireEvent.click(screen.getByRole("button", { name: "Needs you (1)" }));
    expect(scrollIntoView.mock.contexts[0]).toBe(card);
    expect(scrollIntoView.mock.calls[0]![0]).toEqual({ block: "start" }); // a card that fits: its top
    expect(document.activeElement).toBe(card!.querySelector("[data-ask-first]"));
    expect(document.activeElement!.textContent).toBe("Allow once");
  });

  it("ac2_a_typed_deny_reason_and_its_ask_bar_survive_scrolling_away_and_back", () => {
    fireEvent.click(screen.getByRole("button", { name: "Deny…" }));
    const reason = screen.getByRole("textbox", { name: "Tell the agent why" }) as HTMLInputElement;
    fireEvent.change(reason, { target: { value: "not this file" } });
    for (const y of [150_000, 1e9, 0, 1e9]) {
      scrollTo(y);
      expect(screen.getByRole("textbox", { name: "Tell the agent why" })).toBe(reason); // the very same element
      expect(reason.value).toBe("not this file");
      expect(rowCount()).toBeLessThanOrEqual(120);
    }
  });

  it("ac7_stream_delta_reaches_its_text", async () => {
    await settled();
    delays.length = 0;
    await until(() => delays.length >= 20, TIMED ? 5000 : 60_000); // the next 20 deltas of the stream that is playing
    const ms = delays.slice(0, 20);
    results.stream = p95(ms);
    within("stream p95", results.stream!, 100);
  });

  it("ac7_stop_and_esc_stop", async () => {
    const ms: number[] = [];
    const interrupts = () => fake.calls.filter((c) => c.name === "interrupt").length;
    for (let k = 0; k < 20; k++) {
      await settled();
      const before = interrupts();
      const t = now();
      if (k % 2) fireEvent.click(screen.getByRole("button", { name: "Stop" }));
      else fireEvent.keyDown(screen.getByRole("textbox", { name: "Message" }), { key: "Escape" });
      await until(() => interrupts() > before && screen.queryAllByText("Stopping…").length > 0);
      ms.push(now() - t);
      fake.play(IDLE); // the turn ends, then a new one runs, so the next Stop shows
      await until(() => !hasButton("Stopping…") && !document.body.textContent!.includes("Stopping…")); // reset steps poll the DOM cheaply: role queries cost more than the page
      fake.play(IDLE + 1);
      await until(() => hasButton("Stop"));
    }
    results.stop = p95(ms);
    within("stop p95", results.stop!, 100);
  });

  it("ac1_ac7_scroll_to_a_far_offset_shows_its_rows_and_at_most_120_rows", () => {
    const ms: number[] = [];
    const most = { n: 0 };
    for (let k = 0; k < 20; k++) {
      const y = 100_000 + ((k * 7919) % 200) * 1000; // far offsets, none the same
      const t = now();
      scrollTo(y);
      const idx = Math.floor((y - 16) / SLOT) + 2;
      expect(document.querySelector(`.wa-row[data-index="${idx}"]`), `row ${idx} at ${y}`).not.toBeNull();
      ms.push(now() - t);
      most.n = Math.max(most.n, rowCount());
    }
    results.scroll = p95(ms);
    within("scroll p95", results.scroll!, 100);
    expect(most.n).toBeLessThanOrEqual(120);
    // to the top: the first row is reachable; back to the end: the last rows are there
    scrollTo(0);
    expect(document.querySelector('.wa-row[data-index="0"]')).not.toBeNull();
    expect(rowCount()).toBeLessThanOrEqual(120);
    scrollTo(1e9);
    expect(document.querySelector(`.wa-row[data-index="${Math.round((total() + ROW_GAP) / SLOT) - 1}"]`)).not.toBeNull(); // the last row
    expect(rowCount()).toBeLessThanOrEqual(120);
  });

  it("ac7_the_whole_file_runs_within_10s_when_timed", () => {
    within("whole file ms", now() - started, 10_000);
  });
});
