---
id: D001
date: 2026-11-26
rung: 2
trigger: door-C
door: [C]
docs: [design/TECHNICAL.md#9.5, design/TECHNICAL.md#8, design/FUNCTIONAL.md#3.5]
blind_agree: true
rounds: 0
codex: local
outcome: concluded
human: pending
---

# D001: whose identity a console session's timer firing must carry (S138 F1.1)

Steward: Deputy Scissors. Counterpart: Codex (local, read-only, fresh context, branch origin/slice/S138).

## Frame

Question: When the agent kernel (a watane mod, `mods:agent`) handles a timer firing of an OpenAI session whose owner is `page` (a session started from the kernel's own page, the console), what identity must `timers.firing-origin()` return for the kernel to treat the firing as carrying the session owner's authority?

Facts (from the repository):
- watane resolves the origin of a page-origin chain to the page's mod, i.e. the kernel's own installed identity (e.g. `local/agent`); a page firing therefore never reads `page`. watane's read: "In the timer owner's top-level `timer-sink.fired(id)`, it returns `some(identity)` with the mod identity ... that the host resolved for the firing's chain. ... If the stored origin is missing or disabled, or removal rewrote the row to the timer owner, the read returns the timer owner's identity." (agentic-harness design/TECHNICAL.md §2.1, copied from watane host TECHNICAL 19.6)
- `caller.identity()` returns "<publisher>/<slug>, local/<slug>, page, or agent:<handle-name>" (§2.1).
- The PR under review adds AGT-T-138 to design/TECHNICAL.md §9.5: "The expected owner is the authenticated immediate `caller.identity()` that owns the session (2.3), or the root owner's identity for a delegated child. The kernel uses this identity for comparison only. A front mod can make the host's resolved origin differ from that identity. The kernel's own identity is never substituted for the session's owner, including for a page-owned session, whose expected identity is `page`. Before handling OpenAI deadlines or rearming an early OpenAI firing, the resolved origin must equal the bound owner. Otherwise the kernel stops the loop, clears the deadlines, denies open asks with reason `timer-origin`, and records an `idle {failed}` turn ..."
- A reviewer reports that with AGT-T-138 every page-owned session loses its timed resume after a usage limit, its consent-pending retries, and the first toolset retry that every toolset session needs on watane; each ends `idle {failed}`.

Clauses:
- design/FUNCTIONAL.md §3.5 AGT-F-24: "When OpenAI reports a usage limit, the kernel shall pause the session until the reset time and show it. When it reports an error that can be retried, the kernel shall retry later with backoff, never inside a call."
- design/TECHNICAL.md §8 (Consent): "Origin client: the page's mod for a page call; the mod of the handle for an agent's call; for an item, job or timer, the origin of the call that opened it. The host carries it with the chain."
- design/VISION.md: "Never a second way around consent. Folders, accounts and toolsets are consented per origin client. A mod in the middle gains nothing." and "V5 Safe by default | ... A session is private to the client that started it. Limits live in code. A failing check denies."
- Known limit being closed (F6.1): "when the owner mod of an OpenAI session is disabled during a timed pause, the firing may send one more model request under the kernel's origin; a fix needs the host-resolved origin of a timer firing through the production adapter."
- Review note on the same slice (S133 P1.5): "the expected owner is the arming call's `caller.identity()` ... and that the expected owner is never the kernel itself. The kernel's own timers return its own identity, so it cannot tell 'own' from 'fallback'."

Door: C (a security check's rule, and a FUNCTIONAL promise). Kind: judgement.

## Blind position, Claude (written before reading Codex)

Claude's blind position (written before reading Codex's).
Options:
O1 Keep AGT-T-138 strict (page sessions need an owner action to continue); amend AGT-F-24 to say so. + no new exception in a security rule; + "a failing check denies". - narrows a FUNCTIONAL promise for the console, the kernel's own main user; - breaks the first toolset retry every toolset session needs on watane, so console toolset sessions fail by default (a regression, not a limit).
O2 For a session whose owner is `page`, the expected resolved origin is the kernel's own installed identity (the origin client of a page call, TECHNICAL §8); every other owner stays strict, and the kernel's identity is never accepted for any non-page owner. + matches §8's definition: the page's mod IS the origin client of a page call, so this is the true owner's authority, not a substitute; + keeps AGT-F-24 as written; + F6.1 stays closed: it concerns an owner mod other than the kernel, whose fallback reads the kernel's identity and is still refused; - P1.5's sentence "never the kernel itself" must become "never the kernel itself for a non-page owner"; - a page session's timer armed inside another mod's call (a FIFO drain) whose origin is later removed falls back to the kernel's identity and continues; but the page session then acts with the page's own authority, the authority it already had, so nothing is gained (VISION "a mod in the middle gains nothing").
O3 Ask watane to resolve page-origin firings to `page`: a host contract change across projects, same WIT. + no exception in the kernel; - a cross-project interface change for a value watane defines on purpose as the mod identity; - slower.
Choice: O2.

## Blind position, Codex (verbatim)

My position: **for a console-owned session, the appropriate resolved origin is the kernel’s exact installed mod identity**, such as `some("local/agent")` in development. The session’s API owner should remain `page`.

But **under AGT-T-138 as written, no value returned by the specified production host can satisfy the check**: the kernel requires `page`, while the host returns a mod identity. That is a contract mismatch, not evidence that every console timer lacks authority.

I checked `origin/slice/S138`. The reported failure follows from its comparison and the documented host behavior; I did not run the integration tests.

1. **Keep AGT-T-138 unchanged — the option requiring no document changes.**

   **For:** This follows its explicit rule: “the resolved origin must equal the bound owner,” including the statement that a page-owned session’s expected identity is `page`. It also conservatively closes F6.1 for external owners: a disabled owner’s timer resolves to the kernel, differs from the stored owner, and stops without another request. That implements VISION’s “A failing check denies.”

   The concern behind S133 P1.5 is real. Section 2.1 says the read supplies “no previous origin, local name, generation or reason for fallback.” Consequently, the kernel cannot generally interpret its own identity as evidence that an external session’s owner remains authorized.

   **Against:** This makes legitimate console timers systematically fail. Section 8 defines the origin client as “the page’s mod for a page call”; §2.1 reports that resolved mod identity. Equality with `page` is therefore impossible even when nothing has been disabled or removed.

   That defeats AGT-F-24’s promise to “retry later with backoff,” and §9.5’s usage-limit transition back to a request. It also prevents the documented toolset initialization retry. The branch’s qualification “when the timer carries its owner’s authority” cannot resolve the mismatch by making every healthy console firing fail. Under the specified document hierarchy, retaining this behavior is not an adequate completed design.

2. **Distinguish session ownership from the origin used for consent, and compare the latter — my choice.**

   Keep `session.owner == "page"` for access, listing, transcript privacy, quotas and owner calls. For timer authorization, define its expected resolved origin as the kernel’s authenticated installed identity. For ordinary directly owning mods, continue comparing against their owner identity. Delegated children inherit the corresponding expectation from their root.

   **For:** This matches §8’s explicit definition rather than treating two different identity domains as interchangeable. It restores AGT-F-24 without creating a new consent principal: VISION says accounts and toolsets are “consented per origin client,” and the console’s origin client already is the kernel mod.

   Crucially, this is a **session-specific comparison**, not permission to accept the kernel’s identity for any session. If an external owner is `local/notes` and its disabled-owner fallback returns `local/agent`, the comparison still fails. F6.1 remains closed. A console session legitimately expects `local/agent` because that is already the principal under which its ordinary page calls obtain consent.

   Section 14’s “A session is private” remains enforced through the unchanged API owner. Its “console has no power a client lacks” also remains intact: resolving a page to its containing mod is the host’s general page rule, not an additional console interface.

   **Against:** This explicitly changes a security rule. AGT-T-138 currently says the kernel’s identity is “never substituted,” including for `page`; implementation alone cannot override that clause. Section 2.1’s comparison wording and the affected §9.5 explanations must be reconciled as well.

   Nor does this recover provenance the host discards. A kernel-origin firing does not prove that its chain originally began at the console. The justification is narrower: the resolved principal equals the console session’s existing consent principal. All ordinary access, consent and apply-time checks must still hold. The expected installed identity must come from trusted deployment information, not a caller argument or an assumed development name.

3. **Extend the host read to distinguish preserved origins from fallback.**

   For example, expose authenticated provenance or an explicit fallback status alongside the resolved identity.

   **For:** This directly addresses the information loss documented in §2.1 and the S133 observation that the kernel “cannot tell ‘own’ from ‘fallback’.” It supports a stronger policy if distinguishing those histories is itself required.

   **Against:** It requires a host contract and adapter change. Moreover, provenance alone does not fix the `page` versus mod-identity mismatch: the design must still define which identity is expected. Section 8 already supplies a coherent consent principal, so richer provenance is not necessary merely to make healthy console retries work.

The question is **partly fair, but conflates two questions**: what the proposed literal check accepts, and what identity represents the owner’s consent authority. The answers are respectively “`page`, which production never returns” and “the page’s installed mod identity.”

The fallback warning is relevant but overbroad if used to prohibit that identity for every session. I would approve option 2 only with explicit contract changes and production coverage showing both successful console retries and continued denial of disabled external-owner firings.
## Tree

```
Q  the identity a page-owned session's timer firing must read to carry the owner's authority
   clauses: FUNCTIONAL 3.5 AGT-F-24; TECHNICAL 8 "Origin client: the page's mod for a page call"; VISION "consented per origin client. A mod in the middle gains nothing", V5 "A failing check denies"; TECHNICAL 9.5 AGT-T-138 (this PR); S133 P1.5
   drivers: AGT-F-24's resume and retry; F6.1 stays closed; no new consent principal
O1 keep AGT-T-138 strict; amend AGT-F-24  changes: FUNCTIONAL 3.5  door: C  undo: restore the sentence
   + no exception in a security rule [both, V5]
   - no production value can ever match `page`; every console retry and the toolset first retry fail [both, TECHNICAL 2.1, 8; reviewer's red test; diagnostic: yes] status: open
O2 page-owned sessions expect the kernel's own installed identity (their origin client, 8); all other owners strict  changes: TECHNICAL 9.5 (AGT-T-138), 2.1 comparison text  door: C  undo: one comparison and two sentences
   + matches 8's definition; keeps AGT-F-24 [both, diagnostic: yes]
   + F6.1 stays closed: a non-page owner's fallback to the kernel still differs [both, diagnostic: yes]
   - changes a security rule's text [both] status: accepted cost
   - a kernel-origin firing does not prove the chain began at the console [codex] status: rebutted: the session then acts with the page's own authority, which it already holds (VISION "A mod in the middle gains nothing"); nothing is gained
   - the expected identity must come from trusted deployment information, not a caller argument [codex] status: accepted as a constraint on the build
O3 watane resolves page firings to `page`, or reports fallback provenance  changes: watane host 19.6, 2.1  door: C (cross-project)  undo: a host change
   + no exception in the kernel [both]
   - a cross-project change; provenance alone does not settle which identity is expected [codex, diagnostic: yes] status: open
Positions: blind: claude O2, codex O2; r1: not run (blind agreement)
```

## The six conditions

1. Same option at the end: yes (O2, blind).
2. Every change of choice names its argument: yes (no change).
3. Every diagnostic con against O2 rebutted or accepted: yes.
4. Keeps every VISION clause, no Door H: yes.
5. The deciding fact is in the repository: yes (TECHNICAL 8 and 2.1; the reviewer's red test `regress_s138_a_page_owned_pause_resumes_at_the_reset`).
6. Each option steelmanned by a side that did not choose it: yes (O1 and O3 by both in the blind positions).

## Outcome

Concluded at rung 2: O2. The build team writes it into TECHNICAL 9.5 (AGT-T-138) and 2.1 with spec-change, with tests that a console session resumes and retries, and that a non-page owner's fallback to the kernel identity is still refused. Reported to the human as a Door C conclusion for veto.
