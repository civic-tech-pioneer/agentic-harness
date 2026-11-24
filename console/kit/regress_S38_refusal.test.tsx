import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { emptyView, type SessionView } from "./model";
import { StatusLine } from "./status";
afterEach(cleanup);
const actions = { say: () => {}, answer: () => {}, stop: () => {}, takeBack: () => {} };
const failed = (cause: string) => {
  const v = { ...emptyView, turn: { state: "idle", stop: { kind: "failed", error: "provider-ended", cause } } } as SessionView;
  render(<StatusLine view={v} now={0} stopping={false} actions={actions} onNeedsYou={() => {}} />);
  return screen.getByRole("status").textContent;
};
it.each([
  ["workspace:/home/alice/Notes", "/home/alice/Notes"],              // 19.7 as written
  ["workspace:/home/alice/a:b", "/home/alice/a:b"],                  // 19.7, colon in path
  ["workspace:local/claude-code:/home/alice/a:b", "/home/alice/a:b"], // Q38 (b)
])("refusal_names_the_folder_%s", (what, folder) => {
  expect(failed(`consent-refused(${what})`)).toBe(`Failed: Workspace access was refused for ${folder}`);
});
