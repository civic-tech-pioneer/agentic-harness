// S32-fix4 AC2: a refused say restores the text unless typed since; the alert shows the message until the next edit.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { longInfo, longKernel } from "../fake/page";
import { App } from "./App";

afterEach(cleanup);
const tick = () => act(async () => { await new Promise((r) => setTimeout(r, 30)); });

describe("S32_fix4", () => {
  it("ac2_refused_say_restores_text_and_alerts", async () => {
    const kernel = longKernel(5);
    kernel.say = () => Promise.reject(new Error("quota: too many"));
    render(<App kernel={kernel} info={longInfo(5)} />);
    const box = document.querySelector<HTMLTextAreaElement>(".wa-composer textarea")!;
    fireEvent.change(box, { target: { value: "hi" } });
    fireEvent.keyDown(box, { key: "Enter" });
    await tick();
    expect(box.value).toBe("hi");
    expect(document.querySelector("[role=alert].wa-send-err")?.textContent).toBe("quota: too many");
    fireEvent.change(box, { target: { value: "hi!" } });
    expect(document.querySelector(".wa-send-err")).toBeNull();
  });
  it("ac2_alert_clears_on_next_send", async () => {
    const kernel = longKernel(5);
    let refuse = true;
    kernel.say = () => (refuse ? Promise.reject(new Error("paused")) : Promise.resolve(1));
    render(<App kernel={kernel} info={longInfo(5)} />);
    const box = document.querySelector<HTMLTextAreaElement>(".wa-composer textarea")!;
    fireEvent.change(box, { target: { value: "hi" } });
    fireEvent.keyDown(box, { key: "Enter" });
    await tick();
    expect(document.querySelector(".wa-send-err")?.textContent).toBe("paused");
    refuse = false;
    fireEvent.keyDown(box, { key: "Enter" });
    expect(document.querySelector(".wa-send-err")).toBeNull();
    await tick();
    expect(box.value).toBe("");
    expect(document.querySelector(".wa-send-err")).toBeNull();
  });
  it("ac2_typed_since_keeps_typed", async () => {
    const kernel = longKernel(5);
    let fail!: (e: Error) => void;
    kernel.say = () => new Promise<number>((_, rej) => { fail = rej; });
    render(<App kernel={kernel} info={longInfo(5)} />);
    const box = document.querySelector<HTMLTextAreaElement>(".wa-composer textarea")!;
    fireEvent.change(box, { target: { value: "first" } });
    fireEvent.keyDown(box, { key: "Enter" });
    fireEvent.change(box, { target: { value: "second" } });
    fail(new Error("busy"));
    await tick();
    expect(box.value).toBe("second");
    expect(document.querySelector(".wa-send-err")).toBeNull();
  });
});
