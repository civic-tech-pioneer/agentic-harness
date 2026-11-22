// The exit of `npm run test:host` for an error that ended the run before its checks (stage_b.mjs): 2 "not run" names the prerequisite
// that is missing, anything else is 1, a failure. Its own module so the rule is tested without a host (outcome.test.ts).

/** A missing prerequisite the run names (exit 2). */
export class NotRun extends Error {}

/** The run's end for `e`: `{ code: 1 | 2, line }`. */
export function outcomeOf(e) {
  const msg = e instanceof Error ? e.message : String(e);
  if (e?.serveFailed && /cannot bind loopback port/.test(msg))
    return { code: 2, line: "NOT RUN test:host: the host binds 127.0.0.1 and [::1] both (host 8.1) and this machine has no IPv6 loopback" };
  if (e instanceof NotRun || e?.notRun) return { code: 2, line: `NOT RUN test:host: ${msg}` };
  if (e?.addFailed || (e?.serveFailed && /claude-code|claude_code|fake-claude|provider/i.test(msg)) || (e?.serveFailed && e.code === "mod.invalid" && /watane:host\/streams/.test(msg)))
    return { code: 2, line: `NOT RUN test:host: the host cannot link claude-code or watane-dev is too old (watane S99 not landed, or watane-dev older than rev 4f105a9): ${msg.split("\n")[0].slice(0, 240)}` };
  if (e?.firstTurn) return { code: 2, line: `NOT RUN test:host: ${msg}` };
  return { code: 1, line: `FAIL setup: ${e instanceof Error ? e.stack ?? e.message : String(e)}` };
}
