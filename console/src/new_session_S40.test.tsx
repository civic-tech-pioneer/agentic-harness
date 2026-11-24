// S40-6 AC1, AC2, AC3, AC6: the new-session view lists the kernel's profiles, shows the backend and the kernel's refusals, and keeps Start off while one holds.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { Console } from "./Console";
import { served } from "./served";
import { fakeWatane } from "../kit/watane_fake";
import { wataneKernel } from "../kit/watane";
import { fakeKernel, fakeProfiles } from "../fake/kernel";

afterEach(cleanup);
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 60)); });
const box = () => screen.getByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
const start = () => screen.getByRole("button", { name: "Start session" });

const refusal = (field: string, reason: string) => ({ field, backend: "openai", model: "gpt-5", reason });
function setup(over: Record<string, (a: any) => unknown> = {}) {
  return fakeWatane({
    "sessions.list": () => ({ ok: [] }),
    "sessions.profiles": () => ({ ok: [{ name: "scratch", backend: "claude", model: null }, { name: "Sort", backend: "openai", model: "gpt-5" }, { name: "Tidy", backend: "openai", model: "gpt-5" }] }),
    "sessions.check": (a: any) => ({ ok: a.profile === "Tidy" ? [refusal("tools", "ChatGPT runs no file tools."), refusal("allow", "An allow rule needs Claude Code.")] : [] }),
    "sessions.start": () => ({ ok: "n1" }),
    "sessions.attach": () => ({ ok: "st1" }),
    "sessions.read": () => ({ ok: { entries: [], last: 0 } }),
    ...over,
  });
}

describe("S40-6 AC1 the profiles", () => {
  it("ac1_profiles_is_called_once_and_each_profile_is_a_radio_row_the_first_selected", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(f.calls.filter((c) => c.fn === "sessions.profiles")).toHaveLength(1);
    const rows = screen.getAllByRole("radio");
    expect(rows.map((r) => r.closest("label")!.textContent)).toEqual(["scratchClaude Code", "SortChatGPT · gpt-5", "TidyChatGPT · gpt-5"]);
    expect(rows.map((r) => (r as HTMLInputElement).checked)).toEqual([true, false, false]);
  });
  it("ac1_runs_on_marks_the_selected_backend_and_the_other_is_not_selectable", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    await tick();
    const g = within(screen.getByRole("group", { name: "Runs on" }));
    expect(g.getByRole("button", { name: "Claude Code" })).toHaveAttribute("aria-pressed", "true");
    const other = g.getByRole("button", { name: "ChatGPT" });
    expect(other).toHaveAttribute("aria-pressed", "false");
    expect(other).toBeDisabled();
    expect(other).toHaveAttribute("title", "Set in the profile");
    fireEvent.click(screen.getByRole("radio", { name: /Sort/ }));
    await tick();
    expect(g.getByRole("button", { name: "ChatGPT" })).toHaveAttribute("aria-pressed", "true");
    expect(g.getByRole("button", { name: "Claude Code" })).toBeDisabled();
  });
  it("ac1_scratch_alone_looks_as_before_plus_the_runs_on_line", async () => {
    const f = setup({ "sessions.profiles": () => ({ ok: [{ name: "scratch", backend: "claude", model: null }] }) });
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(screen.getAllByRole("radio")).toHaveLength(1);
    expect(screen.getByTitle("Profile").textContent).toBe("Profile scratch");
    expect(screen.getByRole("group", { name: "Runs on" })).toBeInTheDocument();
    expect(box()).toHaveAttribute("placeholder", "Message Claude…");
  });
});

describe("S40-6 AC2 the kernel's refusals", () => {
  it("ac2_check_is_called_on_each_selection_and_its_refusals_are_listed_field_by_field", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(f.calls.filter((c) => c.fn === "sessions.check").map((c) => c.args)).toEqual([{ profile: "scratch", overlay: "{}" }]);
  });
  it("ac2_a_profile_with_refusals_shows_them_in_a_polite_status_region_and_keeps_Start_off", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    await tick();
    fireEvent.change(box(), { target: { value: "tidy up" } });
    expect(start()).toBeEnabled();
    fireEvent.click(screen.getByRole("radio", { name: /Tidy/ }));
    await tick();
    expect(f.calls.filter((c) => c.fn === "sessions.check").map((c) => c.args.profile)).toEqual(["scratch", "Tidy"]);
    const region = document.querySelector(".wa-refusals")!;
    expect(region).toHaveAttribute("role", "status");
    expect(region).toHaveAttribute("aria-live", "polite");
    expect(region.textContent).toContain("Can't run on ChatGPT");
    expect([...region.querySelectorAll(".wa-ref")].map((r) => [r.querySelector("code")!.textContent, r.querySelector("span")!.textContent])).toEqual([
      ["tools", "ChatGPT runs no file tools."], ["allow", "An allow rule needs Claude Code."]]);
    expect(start()).toBeDisabled();
    expect(document.querySelector(".wa-send-off")!.textContent).toContain("Tidy cannot run on ChatGPT");
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(f.calls.some((c) => c.fn === "sessions.start")).toBe(false);
    fireEvent.click(screen.getByRole("radio", { name: /Sort/ }));
    await tick();
    expect(region.textContent).toBe("");
    expect(start()).toBeEnabled();
  });
  it("ac2_a_failed_check_shows_no_refusal_the_kernel_did_not_report", async () => {
    const f = setup({ "sessions.check": () => { throw new Error("down"); } });
    render(<Console served={await served(f.w)} />);
    await tick();
    expect(document.querySelector(".wa-refusals")!.textContent).toBe("");
  });
});

describe("S40-6 AC3 a refused start", () => {
  it("ac3_a_refused_start_lists_each_field_and_reason_and_puts_the_text_back", async () => {
    const f = setup({ "sessions.start-in-folder": () => ({ err: { refused: [refusal("folder", "ChatGPT cannot work in a folder.")] } }) });
    render(<Console served={await served(f.w)} />);
    await tick();
    fireEvent.click(screen.getByRole("radio", { name: /Sort/ }));
    await tick();
    fireEvent.change(screen.getByRole("textbox", { name: "Folder path" }), { target: { value: "/home/a" } });
    fireEvent.change(box(), { target: { value: "sort them" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    const call = f.calls.find((c) => c.fn === "sessions.start-in-folder")!;
    expect(call.args).toMatchObject({ profile: "Sort", folder: "/home/a" });
    expect(screen.getByRole("alert").textContent).toBe("The kernel refused that: folder: ChatGPT cannot work in a folder.");
    expect(box().value).toBe("sort them");
  });
  it("ac3_a_started_profile_is_named_in_start", async () => {
    const f = setup();
    render(<Console served={await served(f.w)} />);
    await tick();
    fireEvent.click(screen.getByRole("radio", { name: /Sort/ }));
    await tick();
    fireEvent.change(box(), { target: { value: "go" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await tick();
    expect(f.calls.find((c) => c.fn === "sessions.start")!.args).toMatchObject({ profile: "Sort", overlay: "{}", input: "go" });
  });
});

describe("S40-6 AC6 the kit and the fake kernel", () => {
  it("ac6_the_kit_maps_profiles_and_check_to_sessions_profiles_and_sessions_check", async () => {
    const f = setup();
    const k = wataneKernel(f.w);
    expect((await k.profiles!()).map((p) => p.name)).toEqual(["scratch", "Sort", "Tidy"]);
    expect((await k.check!("Tidy", "{}")).map((r) => r.field)).toEqual(["tools", "allow"]);
    expect(f.calls.map((c) => [c.fn, c.args])).toEqual([["sessions.profiles", {}], ["sessions.check", { profile: "Tidy", overlay: "{}" }]]);
  });
  it("ac6_the_fake_kernel_answers_three_profiles", async () => {
    const k = fakeKernel([]);
    expect(await k.profiles!()).toEqual(fakeProfiles);
    expect(fakeProfiles.map((p) => [p.name, p.backend, p.model ?? null])).toEqual([["scratch", "claude", null], ["Sort", "openai", "gpt-5"], ["Tidy", "openai", "gpt-5"]]);
    expect(await k.check!("Sort", "{}")).toEqual([]);
    expect((await k.check!("Tidy", "{}")).map((r) => r.field)).toEqual(["tools", "allow"]);
  });
});
