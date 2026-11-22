// The exit of `npm run test:host` for an error that ended the run before its checks (stage_b.mjs): 2 "not run" names the prerequisite
// that is missing, anything else is 1, a failure. Its own module so the rule is tested without a host (outcome.test.ts).

/** A missing prerequisite the run names (exit 2). */
export class NotRun extends Error {}

/** The run's end for `e`: `{ code: 1 | 2, line }`. */
export function outcomeOf(e) {
  const msg = e instanceof Error ? e.message : String(e);
  // a loopback bind failure is a failure: watane c2a7d4d serves 127.0.0.1 alone on a machine with no IPv6 (host 8.1), so no bind is a missing prerequisite (S35 r2 F2.4)
  if (e instanceof NotRun || e?.notRun) return { code: 2, line: `NOT RUN test:host: ${msg}` };
  // watane-dev too old for `add` (clap's own words), or a host without the parts S99 added: the world's imports it does not have
  const oldDev = e?.addFailed && /unrecognized subcommand|unknown command|invalid subcommand/i.test(`${e.err ?? ""} ${e.out ?? ""} ${msg}`);
  const oldHost = e?.serveFailed && e.code === "mod.invalid" && /watane:host\/(streams|host-info|claude-session|claude-types)\b/.test(msg);
  if (oldDev || oldHost)
    return { code: 2, line: `NOT RUN test:host: watane-dev or its host is too old for stage B (watane S99 not landed, or watane-dev older than rev 4f105a9): ${msg.split("\n")[0].slice(0, 240)}` };
  // the fake claude is not linked in the preview: the turn ends no-binary (dev 9.1 `preview.fake_claude_missing`)
  if (e?.firstTurn === "no-binary") return { code: 2, line: `NOT RUN test:host: ${msg}` };
  return { code: 1, line: `FAIL setup: ${e instanceof Error ? e.stack ?? e.message : String(e)}` };
}
