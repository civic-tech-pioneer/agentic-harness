# Deputy decisions awaiting a factory change

The 2026-11-27 S153 process decision remains a proposal to the factory owner:
when a Codex build team cannot run the adversary's exploit-development step,
the build team supplies defensive boundary checks and guard tests, and the
Claude review team runs the adversary in round 1. The check moves; it is not
dropped. The factory's foreman instructions should state this division of work.
The design boundary in TECHNICAL 14 is unchanged.

The earlier product decisions have been reconciled into their owning sections
of `design/TECHNICAL.md`; superseded choices and slice scheduling notes are
historical records, not active design rules.
## Reconciliation index

| Former LOG decisions | Owning contract |
|---|---|
| S138 timer origin; S38 consent keys and endings; S127 stream and judge rulings; S39 provider pin; S40 restart and persistence | TECHNICAL 2.1, 8, 9.3, 9.5, 10, 14 |
| S35–S36 composer, queue, activity and accepted limits | TECHNICAL 9.3, 9.4, 18.5, 18.6 |
| S6–S34 profile grammar, asks, scope keys, quotas, adapters and restarts | TECHNICAL 4.1, 4.5, 9.1–9.8, 11, 14 |
| S151 profile access and the human's later H2 answer | FUNCTIONAL 3.1–3.2; TECHNICAL 2.1, 9.10, 14–15 |
| Factory scheduling, model choice, delivery and merge procedure | `.claude/factory/README.md`, `.claude/skills/foreman/SKILL.md`, `factory/config.toml`, `factory/PLAN.md` |

The removed LOG lines remain in Git history; this index carries no separate
design authority.
