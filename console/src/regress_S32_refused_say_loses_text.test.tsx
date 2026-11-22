// S32 user-sim on watane's host (4f105a9): Alice types into a session's composer and presses Enter. The kernel refuses
// (`sessions.say` -> backend("a probe session takes no input"), 9.9), the composer is already empty, the page shows no error,
// and the refusal is an unhandled rejection (App.tsx: `say: (text) => void kernel.say(...)`). A refused send must not lose
// the participant's text silently: the text stays in the composer, or the page says why it was not sent.
import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { longInfo, longKernel } from "../fake/page";
import { App } from "./App";

afterEach(cleanup);

describe("regress_S32", () => {
  it("regress_S32_refused_say_loses_text", async () => {
    const kernel = longKernel(5);
    const refusal = Object.assign(new Error("sessions.say: backend (the provider takes no input)"), { kind: "backend", payload: "the provider takes no input" });
    let unhandled: unknown = null;
    const onUnhandled = (e: unknown) => { unhandled = e; };
    process.on("unhandledRejection", onUnhandled);
    kernel.say = () => Promise.reject(refusal);
    try {
      render(<App kernel={kernel} info={longInfo(5)} />);
      const box = document.querySelector<HTMLTextAreaElement>(".wa-composer textarea")!;
      fireEvent.change(box, { target: { value: "hello, are you there?" } });
      await act(async () => { fireEvent.keyDown(box, { key: "Enter" }); await new Promise((r) => setTimeout(r, 50)); });
      const kept = box.value === "hello, are you there?";
      const said = document.querySelector("[role=alert]")?.textContent === "The kernel could not do that: the provider takes no input"; // 18.6 Send, AC4a: a sentence, not the raw error
      expect(kept || said, "the refused text is neither kept in the composer nor explained on the page").toBe(true);
      expect(unhandled, "the refusal escapes as an unhandled rejection").toBeNull();
    } finally {
      process.off("unhandledRejection", onUnhandled);
    }
  });
});
