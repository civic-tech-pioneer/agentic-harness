// S35-4 AC2, AC4: the new-session view, Start in place, a refused start, the kept draft (component tests on a fake `window.watane`).
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Console } from "./Console";
import { served } from "./served";
import { fakeWatane } from "../kit/watane_fake";

afterEach(cleanup);
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 60)); });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const text = () => document.body.textContent ?? "";

function setup(over: Record<string, (a: any) => unknown> = {}) {
  let started = false;
  const row = { id: "n1", title: "Say hi", profile: "scratch", backend: "claude-code", turn: "running", updated: 9 };
  const f = fakeWatane({
    "sessions.list": () => ({ ok: started ? [row] : [] }),
    "sessions.start": () => { started = true; return { ok: "n1" }; },
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
    ...over,
  });
  return f;
}

describe("S35-4 AC2 the new-session view", () => {
  it("ac2_with_no_session_the_view_shows_the_stage_B_words_only", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
    expect(screen.getByTitle("Profile").textContent).toBe("Profile scratch"); // the stored name, as the session header shows it (S35 r1 D1.1)
    expect(text()).toContain("A new scratch folder");
    expect(box()).toHaveAttribute("placeholder", "Message Claude…");
    expect(screen.getByRole("button", { name: "Start session" })).toBeDisabled();
    expect(f.calls.map((c) => c.fn)).toEqual(["sessions.list"]); // no session was started by looking
  });
  it("ac2_send_starts_with_a_fresh_scratch_key_and_opens_the_session_in_place", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    const start = f.calls.find((c) => c.fn === "sessions.start")!;
    expect(start.args).toEqual({ key: expect.stringMatching(/^console-\d+-[a-z0-9]+$/), profile: "scratch", overlay: "{}", input: "hi" });
    expect(f.calls.some((c) => c.fn === "sessions.attach" && c.args.session === "n1")).toBe(true);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Say hi");
    expect(text()).toContain("scratch");
    expect(text()).toContain("Claude Code (unchecked)");
    expect(text()).toContain("Scratch folder");
    expect(screen.getAllByText("Say hi").length).toBeGreaterThan(1); // the sidebar lists it too
    expect(screen.queryByText("A new scratch folder")).toBeNull();
  });
  it("ac2_the_button_starts_too_and_a_new_session_after_it_shows_the_view_again", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.click(screen.getByRole("button", { name: "Start session" }));
    await tick();
    expect(f.calls.filter((c) => c.fn === "sessions.start")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
  });
  it("ac2_with_sessions_the_newest_opens_and_New_session_shows_the_view", async () => {
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "o", title: "Older", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }, { id: "z", title: "Newest", profile: "scratch", backend: "claude-code", turn: "idle", updated: 7 }] }) });
    render(<Console served={await served(f.w)} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Newest");
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
  });
  it("ac2_a_refused_start_keeps_the_text_and_alerts_in_words", async () => {
    const f = setup({ "sessions.start": () => ({ err: "busy" }) });
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(box().value).toBe("hi");
    expect(screen.getByRole("alert").textContent).toBe("Claude is busy with another session; try again in a moment.");
    expect(screen.getByRole("heading", { name: "New session" })).toBeInTheDocument();
  });
});

describe("S35-4 AC4", () => {
  it("ac4b_the_draft_is_back_after_a_reload_until_sent", async () => {
    const f = setup();
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "half a thought" } });
    a.unmount();
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("half a thought");
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    cleanup();
    render(<Console served={await served(setup({ "sessions.list": () => ({ ok: [] }) }).w)} />);
    expect(box().value).toBe("");
  });
  it("ac4b_a_session_keeps_its_own_draft", async () => {
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "s", title: "T", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }] }) });
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "later" } });
    a.unmount();
    expect(sessionStorage.getItem("wa-draft:s")).toBe("later");
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("later");
  });
  it("ac4b_a_page_without_sessionStorage_still_works", async () => {
    const f = setup();
    const real = Object.getOwnPropertyDescriptor(window, "sessionStorage")!;
    Object.defineProperty(window, "sessionStorage", { configurable: true, get() { throw new Error("denied"); } });
    try {
      render(<Console served={await served(f.w)} />);
      fireEvent.change(box(), { target: { value: "hi" } });
      expect(box().value).toBe("hi");
    } finally { Object.defineProperty(window, "sessionStorage", real); }
  });
  it("ac4d_no_section_number_is_shown_or_read", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    expect(document.body.innerHTML).not.toMatch(/\(1\d\.\d\)|18\.8/);
    expect(document.getElementById("wa-off-new")?.textContent).toBe(undefined);
    fireEvent.change(box(), { target: { value: "x" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(document.body.innerHTML).not.toMatch(/\(1\d\.\d\)|18\.8/);
    expect(document.getElementById("wa-off-menu")?.textContent).toBe("Not in this console yet.");
  });
});

describe("S35 r1 F1.5 every row's title and meta", () => {
  const user = (i: number, text: string) => JSON.stringify({ v: 1, i, kind: "message", at: 1000 * i, body: { role: "user", content: [{ type: "text", text }] } });
  it("r1_f1_5_an_untitled_older_row_reads_its_first_user_message", async () => {
    const f = setup({
      "sessions.list": () => ({ ok: [{ id: "o", title: "", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }, { id: "z", title: "Newest", profile: "scratch", backend: "claude-code", turn: "idle", updated: 7 }] }),
      "sessions.read": (a: { session: string }) => ({ ok: { entries: a.session === "o" ? [user(1, "Earlier question about the folder")] : [], last: a.session === "o" ? 1 : 0 } }),
    });
    render(<Console served={await served(f.w)} />);
    expect(await screen.findByRole("button", { name: /Earlier question/ }))
      .toBeInTheDocument();
  });
  it("r1_f1_5_an_openai_scratch_session_is_not_a_claude_scratch_folder", async () => {
    const openAiScratch = setup({ "sessions.list": () => ({ ok: [{ id: "a", title: "T", profile: "scratch", backend: "openai", turn: "idle", updated: 1 }] }) });
    expect((await served(openAiScratch.w)).sessions[0]?.meta)
      .toBe("openai · scratch");
  });
});

describe("S35 r1 D1.1, D1.2: the profile's one spelling; model and mode on the real serve", () => {
  it("r1_d1_1_the_new_session_view_and_the_header_spell_the_profile_alike", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    const before = screen.getByTitle("Profile").textContent;
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(screen.getByTitle("Profile").textContent).toBe(before);
  });
  it("r1_d1_2_a_claude_scratch_session_shows_no_guessed_model_and_no_meter_without_usage", async () => {
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "s", title: "T", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }] }) });
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(document.querySelector(".wa-irow")?.textContent).not.toContain("manual"); // 18.4, S35 r2 F2.5
    expect(document.querySelector(".wa-meter, [aria-label^='Context']")).toBeNull();
  });
  it("r1_d1_2_a_session_started_in_place_shows_none_either", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(document.querySelector(".wa-irow")?.textContent).not.toContain("manual");
  });
  it("r1_d1_2_another_profile_or_backend_shows_no_guessed_model", async () => {
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "a", title: "T", profile: "scratch", backend: "openai", turn: "idle", updated: 1 }] }) });
    expect((await served(f.w)).info?.model).toBeUndefined();
  });
});

describe("S35 r2 F2.2, F2.3, F2.5", () => {
  it("r2_f2_2_a_settled_older_send_leaves_a_newer_composers_draft", async () => {
    let refuse!: (e: Error) => void;
    const f = setup({
      "sessions.start": ({ input }) => input === "first"
        ? new Promise((_, reject) => { refuse = reject; })
        : { ok: "n1" },
    });
    const page = render(<Console served={await served(f.w)} />);
    const submit = (value: string) => {
      fireEvent.change(box(), { target: { value } });
      fireEvent.keyDown(box(), { key: "Enter" });
    };
    submit("first");
    submit("second");
    await screen.findByRole("heading", { name: "second" });
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    fireEvent.change(box(), { target: { value: "third remains unsent" } });
    await act(async () => refuse(new Error("refused")));
    page.unmount();
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("third remains unsent");
  });
  it("r2_f2_3_an_untitled_row_whose_first_message_came_in_parts_reads_it", async () => {
    const line = JSON.stringify({
      v: 1, i: 2, at: 1, kind: "message",
      body: { role: "user", content: [{ type: "text", text: "a".repeat(262144) }] },
    });
    const entries = [line.slice(0, 261900), line.slice(261900)].map((text, n) =>
      JSON.stringify({ v: 1, i: n + 1, at: 1, kind: "part", body: { n: n + 1, m: 2, text } }),
    );
    const f = setup({
      "sessions.list": () => ({ ok: [
        { id: "older", title: "", profile: "scratch", backend: "claude", updated: 1 },
        { id: "newer", title: "Newest", profile: "scratch", backend: "claude", updated: 2 },
      ] }),
      "sessions.read": () => ({ ok: { entries, last: 2 } }),
    });
    expect((await served(f.w)).sessions[0]?.title).toBe("a".repeat(60) + "…");
  });
  it("r2_f2_5_a_stored_scratch_profile_never_claims_the_default_model_open_or_reloaded", async () => {
    // a stored `profile/scratch` (say, model haiku) lists exactly as the built-in: the console cannot tell them apart (18.4)
    const f = setup({ "sessions.list": () => ({ ok: [{ id: "s", title: "T", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 }] }) });
    const a = render(<Console served={await served(f.w)} />);
    await tick();
    expect(text()).not.toContain("default model");
    a.unmount();
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(text()).not.toContain("default model");
    expect((await served(f.w)).info?.model).toBeUndefined();
  });
  it("r2_f2_5_a_session_started_in_place_claims_no_model_either", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "hi" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(text()).not.toContain("default model");
  });
});

describe("S35 r4: the kept draft (deputy's rule: unsent text only; TECHNICAL 18.6 Send)", () => {
  const sessions = (say: () => unknown) => setup({
    "sessions.list": () => ({ ok: [
      { id: "s", title: "S", profile: "scratch", backend: "claude-code", turn: "idle", updated: 1 },
    ] }),
    "sessions.say": say,
  });
  it("r4_rule_1_unsent_text_is_kept_per_session_across_a_reload", async () => {
    const f = sessions(() => ({ ok: 1 }));
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "for s" } });
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    expect(box().value).toBe("");
    fireEvent.change(box(), { target: { value: "for a new one" } });
    a.unmount();
    expect(sessionStorage.getItem("wa-draft:s")).toBe("for s");
    expect(sessionStorage.getItem("wa-draft:new")).toBe("for a new one");
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("for s");
  });
  it("r4_rule_2_send_takes_the_text_out_of_the_composer_and_the_kept_draft_at_once", async () => {
    const f = sessions(() => new Promise(() => {}));
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "pending" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    expect(box().value).toBe("");
    expect(sessionStorage.getItem("wa-draft:s")).toBeNull();
    a.unmount();
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe(""); // the pending send is not offered as unsent (F4.1, F4.3)
  });
  it("r5_f5_1_send_clears_the_kept_draft_before_say_is_dispatched", async () => {
    let draftWhenDispatched: string | null | undefined;
    const f = sessions(() => {
      draftWhenDispatched = sessionStorage.getItem("wa-draft:s");
      return new Promise(() => {});
    });
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "pending" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    expect(draftWhenDispatched).toBeNull();
  });
  it("r4_rule_3_a_refusal_while_mounted_puts_the_text_back_in_the_composer_and_the_kept_draft", async () => {
    let refuse!: (r: unknown) => void;
    const f = sessions(() => new Promise((r) => { refuse = r; }));
    const a = render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "refused words" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await act(async () => refuse({ err: "busy" }));
    expect(box().value).toBe("refused words");
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(sessionStorage.getItem("wa-draft:s")).toBe("refused words");
    a.unmount();
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("refused words");
  });
  it("r4_rule_3_text_typed_since_the_send_wins_over_the_refused_text", async () => {
    let refuse!: (r: unknown) => void;
    const f = sessions(() => new Promise((r) => { refuse = r; }));
    render(<Console served={await served(f.w)} />);
    fireEvent.change(box(), { target: { value: "refused words" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.change(box(), { target: { value: "newer" } });
    await act(async () => refuse({ err: "busy" }));
    expect(box().value).toBe("newer");
    expect(sessionStorage.getItem("wa-draft:s")).toBe("newer");
  });
  it("r4_rule_4_a_refusal_after_its_composer_unmounted_changes_no_later_composer_or_draft", async () => {
    const settle: Array<(r: unknown) => void> = [];
    const f = sessions(() => new Promise((r) => { settle.push(r); }));
    const a = render(<Console served={await served(f.w)} />);
    for (const value of ["older", "newer"]) {
      fireEvent.change(box(), { target: { value } });
      fireEvent.keyDown(box(), { key: "Enter" });
    }
    a.unmount();
    render(<Console served={await served(f.w)} />);
    expect(box().value).toBe("");
    fireEvent.change(box(), { target: { value: "typed later" } });
    await act(async () => { settle[1]!({ err: "busy" }); settle[0]!({ ok: 1 }); });
    expect(box().value).toBe("typed later");
    expect(sessionStorage.getItem("wa-draft:s")).toBe("typed later");
  });
});
