Folder: design
Effort: low
Depends on: S1-2
Slice: S1 — a maker runs `python3 -I proto/claude-probes/probes.py subagent-slash loopback` on the Mac and sees, for the current `claude`, whether a subagent prompt that starts with `/` runs a skill the profile denies, and whether the shell grant's sandbox blocks a connection to loopback.

## Goal

The adversary found a breach of V27 (commit 47b7b3e, case `adv_S1_localhost_allowlist` in
`probe_loopback`): with `allowedDomains: ["localhost"]`, `curl --noproxy '' http://localhost:<port>/`
goes through the sandbox's own proxy and reaches the loopback server. Direct connections stay
blocked. The foreman reran it: `factory/runs/S1-probes-loopback.txt` (11 of 12 held, the FAIL line
is the breach). Sketch outcome 3: the V27 row records the breach as the design effect. The probe
case stays red; it reports `claude`'s behaviour, which this slice does not change.

## Acceptance

AC1. V27's Result cell in `design/ARCHITECTURE-PROPOSAL.md` section 18 gives the rerun: the three
     targets (by IP, by name, through the sandbox proxy with `--noproxy ''`) under the control, an
     empty allowlist, `["example.com"]` and `["localhost"]`, quoting which reached the server; date
     2026-11-21, claude 2.1.330. It says that the adversary also found these held (one short list):
     `[::1]`, `0.0.0.0`, decimal and hex IPs, `127.1`, `localhost.`, a CONNECT tunnel, socks5h,
     Host-header and userinfo tricks through the proxy, python and `nc` clients.
AC2. V27's Effect cell starts with `**Breach.**`. It says: the sandbox's network proxy connects to
     any allowlisted name, loopback included, so "loopback denied" (TECHNICAL 4.2) holds only if no
     name in the shell grant's allowlist resolves to loopback. `localhost` (and `localhost.`) can be
     refused by the allowlist rule; a public name that resolves to 127.0.0.1 (a wildcard DNS service,
     rebinding) was not probed and needs a design decision (product owner): TECHNICAL 4.2 and 14 to
     change, not here.
AC3. V26's row and every other line are unchanged. `.claude/factory/check-docs.sh` and the gate pass.

## Spec

`design/ARCHITECTURE-PROPOSAL.md` section 18; `design/TECHNICAL.md` 4.2 (the `claude-code-shell`
row), 14.

## Interface

None.

## Files

`design/ARCHITECTURE-PROPOSAL.md` (row V27 only).

## Changed tests

None (a record table). The red probe case stays red on purpose.

## Out of scope

TECHNICAL 4.2 and 14 (the product owner's decision; slice S2 or later); probing DNS names that
resolve to loopback.
