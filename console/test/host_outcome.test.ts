// S35 r1 F1.6: `npm run test:host` exits 2 only for a missing prerequisite it names; on a supported watane anything else fails (exit 1).
import { describe, expect, it } from "vitest";
// @ts-expect-error: a plain .mjs module of the host runner
import { NotRun, outcomeOf } from "./host/outcome.mjs";

const err = (message: string, flags: object) => Object.assign(new Error(message), flags);

describe("test:host's exit for an error before its checks", () => {
  it("a generic watane-dev add failure (an invalid provider manifest) fails, exit 1", () => {
    expect(outcomeOf(err("watane-dev add provider: mod.invalid [mod-toml] provider/mod.toml: unknown field `x`", { addFailed: true, code: "mod.invalid", out: "{}", err: "" })).code).toBe(1);
  });
  it("a serve failure that names the provider but no missing host part fails, exit 1", () => {
    expect(outcomeOf(err("mod.invalid [mod-toml] ../../provider/mod.toml: bad version", { serveFailed: true, code: "mod.invalid" })).code).toBe(1);
  });
  it("a first turn that ends provider-ended or stalls fails, exit 1", () => {
    expect(outcomeOf(err("the first turn ended provider-ended", { firstTurn: "provider-ended" })).code).toBe(1);
    expect(outcomeOf(err("the first fake turn did not end in 30 s", { firstTurn: "stalled" })).code).toBe(1);
  });
  it("a loopback bind failure fails, exit 1: watane c2a7d4d serves 127.0.0.1 alone when the machine has no IPv6 (F2.4)", () => {
    expect(outcomeOf(err("cannot bind loopback port 0", { serveFailed: true })).code).toBe(1);
    expect(outcomeOf(err("cannot bind loopback port [::1]:0", { serveFailed: true })).code).toBe(1);
  });
  it("a named missing prerequisite is not run, exit 2", () => {
    expect(outcomeOf(new NotRun("fake-claude is not found")).code).toBe(2);
    expect(outcomeOf(err("watane-dev cannot run: spawn watane-dev ENOENT", { notRun: true })).code).toBe(2);
    expect(outcomeOf(err("watane-dev add provider: error: unrecognized subcommand 'add'", { addFailed: true, out: "", err: "error: unrecognized subcommand 'add'" })).code).toBe(2);
    expect(outcomeOf(err("mod.invalid [wit] world imports watane:host/host-info@0.1.0, which this host does not have", { serveFailed: true, code: "mod.invalid" })).code).toBe(2);
    expect(outcomeOf(err("the first turn ended no-binary", { firstTurn: "no-binary" })).code).toBe(2);
    // S40 r1 P1.1: a host without stage D's `https-stream` import is a missing prerequisite too
    expect(outcomeOf(err("mod.invalid [wit] world imports watane:host/https-stream@0.1.0, which this host does not have", { serveFailed: true, code: "mod.invalid" })).code).toBe(2);
  });
});
