// S7-3: code blocks in an agent row, highlighted once complete and near the viewport, once per (language, code).
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";

const shiki = vi.hoisted(() => ({ calls: 0 }));
vi.mock("shiki/core", async () => {
  const real = await vi.importActual<typeof import("shiki/core")>("shiki/core");
  return {
    ...real,
    createHighlighterCore: async (o: Parameters<typeof real.createHighlighterCore>[0]) => {
      const h = await real.createHighlighterCore(o);
      const f = h.codeToTokens.bind(h);
      h.codeToTokens = ((...a: Parameters<typeof f>) => { shiki.calls++; return f(...a); }) as typeof f;
      return h;
    },
  };
});

import { MessageText } from "./parts/code";
import { CACHE_BYTES, CACHE_ENTRIES, highlight, peek } from "./parts/highlight";

let io: { cb: (e: { isIntersecting: boolean }[]) => void; opts: IntersectionObserverInit }[] = [];
beforeEach(async () => {
  io = []; await warm(); shiki.calls = 0;
  vi.stubGlobal("IntersectionObserver", class {
    constructor(cb: never, opts: IntersectionObserverInit) { io.push({ cb, opts }); }
    observe() {} disconnect() {} unobserve() {} takeRecords() { return []; }
  });
});
afterEach(() => { vi.unstubAllGlobals(); });
const see = () => act(async () => { io.forEach((o) => o.cb([{ isIntersecting: true }])); await new Promise((r) => setTimeout(r, 50)); });
const settle = () => act(async () => { await new Promise((r) => setTimeout(r, 100)); });
const calls = (n: number) => vi.waitFor(() => expect(shiki.calls).toBe(n), { timeout: 5000 });
const warm = () => highlight("ts", "warm"); // loads Shiki and the language once, so later checks are fast

describe("MessageText", () => {
  it("ac1_prose_and_fenced_block_split_with_a_language_label", () => {
    const { container } = render(<MessageText text={"Before\n```ts\nconst a = 1;\n```\nAfter"} writing={false} />);
    const pre = container.querySelector("pre.wa-code");
    expect(pre).not.toBeNull();
    expect(pre!.querySelector("code")!.textContent).toBe("const a = 1;");
    expect(pre!.textContent).toContain("ts");
    expect(screen.getByText("Before")).toBeInTheDocument();
    expect(screen.getByText("After")).toBeInTheDocument();
  });

  it("ac1_text_without_fences_is_plain_text", () => {
    const { container } = render(<div>{<MessageText text={"a\n  b"} writing={false} />}</div>);
    expect(container.firstElementChild!.innerHTML).toBe("a\n  b");
  });

  it("ac2_streaming_open_fence_is_not_highlighted_then_closing_it_while_visible_highlights_once", async () => {
    const open = "```ts\nconst streamA = 1;";
    const { rerender } = render(<MessageText text={open} writing={true} />);
    await see();
    expect(shiki.calls).toBe(0);
    rerender(<MessageText text={open + "\n```"} writing={true} />);
    await see();
    await calls(1);
  });

  it("ac2_complete_block_never_intersected_is_not_highlighted_and_the_margin_is_a_viewport", async () => {
    render(<MessageText text={"```ts\nconst neverSeen = 1;\n```"} writing={false} />);
    await settle();
    expect(shiki.calls).toBe(0);
    expect(io[0].opts.rootMargin).toBe(`${window.innerHeight || 800}px 0px`);
  });

  it("ac2_finished_message_completes_an_unclosed_fence", async () => {
    render(<MessageText text={"```ts\nconst finished = 1;"} writing={false} />);
    await see();
    await calls(1);
  });

  it("ac3_same_language_and_code_is_highlighted_once", async () => {
    const t = "```ts\nconst same = 1;\n```";
    const { unmount } = render(<><MessageText text={t} writing={false} /><MessageText text={t} writing={false} /></>);
    await see();
    await calls(1);
    unmount();
    const { container } = render(<MessageText text={t} writing={false} />);
    expect(container.querySelector("code span[style]")).not.toBeNull(); // from the cache, on the first render
    await see();
    await calls(1);
  });

  it("ac3_cache_is_bounded", async () => {
    for (let i = 0; i < CACHE_ENTRIES + 5; i++) await highlight("json", `{"k": ${i}}`);
    expect(peek("json", `{"k": 0}`)).toBeUndefined();
    expect(peek("json", `{"k": ${CACHE_ENTRIES + 4}}`)).toBeDefined();
  });

  it("ac3_cache_is_bounded_by_2_MiB_of_code_oldest_out", async () => {
    // 35 blocks of 60 000 chars are 2 100 000 > 2 MiB (2 097 152); 34 fit, so only the oldest goes
    expect(CACHE_BYTES).toBe(2 * 1024 * 1024);
    const code = (i: number) => String(i).padStart(2, "0") + "+".repeat(59_998);
    for (let i = 0; i < 35; i++) await highlight("diff", code(i));
    expect(peek("diff", code(0))).toBeUndefined();
    expect(peek("diff", code(1))).toBeDefined();
    expect(peek("diff", code(34))).toBeDefined();
  });

  it("ac5_unknown_language_and_oversize_code_stay_plain", async () => {
    expect(await highlight("klingon", "x")).toBeNull();
    expect(await highlight(undefined, "x")).toBeNull();
    expect(await highlight("ts", "a".repeat(64 * 1024 + 1))).toBeNull();
    expect(shiki.calls).toBe(0);
    for (const l of ["ts", "tsx", "js", "jsx", "json", "bash", "sh", "rust", "python", "toml", "yaml", "diff", "html", "css", "md"]) {
      expect(await highlight(l, "x"), l).not.toBeNull();
    }
  });

  it("ac6_code_text_with_markup_shows_as_text_and_adds_no_element", async () => {
    const evil = "<img src=x onerror=alert(1)>";
    const { container } = render(<MessageText text={"```html\n" + evil + "\n```"} writing={false} />);
    await see();
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("code span[style]")).not.toBeNull();
    expect(container.querySelector("code")!.textContent).toBe(evil);
  });
});

describe("code block change (S7-fix2)", () => {
  it("ac4_code_that_grows_past_64_KiB_shows_the_whole_new_text_and_none_of_the_old_tokens", async () => {
    const { container, rerender } = render(<MessageText text={"```ts\nconst old = 1;\n```"} writing={false} />);
    await see();
    await vi.waitFor(() => expect(container.querySelectorAll("code span span").length).toBeGreaterThan(0));
    const big = "const big = 1;\n".repeat(5000);
    rerender(<MessageText text={"```ts\n" + big + "\n```"} writing={false} />);
    await see();
    const code = container.querySelector("code")!;
    expect(code.querySelectorAll("span").length).toBe(0);
    expect(code.textContent).toBe(big);
  });
});
