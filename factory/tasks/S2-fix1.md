Folder: .
Effort: medium
Depends on: S2-1, S2-2
Slice: S2 — fix the adversary's breaches of the N10 judge account and of section 14's Files column.

## Goal

The judge account of `design/TECHNICAL.md` (N10) gives a hostile mod nothing: only the kernel
can drive the checked provider, so no other mod can make it spend the participant's judge key;
the account's identity, value grammar, lifetime and origin check are exact. Section 14 names
every file that holds a boundary, the manifests and provider files included, and says how a
provider repository's slices get the adversary step. Documents only; the acceptance checks are
readings and greps you run and quote.

## Acceptance

AC1 (breach 1). Section 2.2 and section 10: every function of the providers' `sessions`
     refuses (`not-owner`, add to 12 under providers) a caller whose `caller.identity()` is not
     the kernel `mods:agent`'s identity; the checked provider makes no judge call for a call it
     refuses. A new case in 16: "A mod that is not the kernel calls a provider's
     `sessions.start` or `say`: `not-owner`; no request with `account: "judge"` leaves (counted at
     the scripted judge server), and no verdict reaches the caller." Section 14 row "A mod in the
     middle gains no authority" also holds by this, and names the providers' `src/lib.rs`.
AC2 (breach 2). Section 14 Files: `mod.toml` of this repository in rows "No mod sees a
     credential", "A mod in the middle gains no authority" and "The console has no power a
     client lacks"; the checked provider's `mod.toml` and `src/judge.rs` in rows "A judge key
     goes only to its judge" and "A failing check cannot let a tool run"; `src/adapters/` in a
     new row "An ask shows the tool call that will run" (with `src/broker.rs`). A sentence
     after the preamble: each provider repository's `factory/config.toml` `boundaries` points at
     this section 14, so its slices that touch a provider file named here get the adversary
     step; and section 17 (stages B and E) says a provider repository is built only with that
     in place.
AC3 (risks). Section 5, judge account:
     - `<provider mod id>` is the provider's `caller.identity()` (`<publisher>/<slug>`), compared
       exactly; the unchecked provider, another publisher's mod or a preview gets
       `no-account`.
     - `account` is exactly `openai` or `judge`; any other value is refused `destination`.
     - The origin is compared after one parse used for both the check and the connection:
       scheme `https`, no userinfo, host lower-cased without a trailing dot, the port with
       443 as default; the same parse decides `[http]`.
     - Lifetime: the key is shown (never its value) and removable on the provider's manager
       page; removing it ends the provider's live judge streams; uninstalling the provider
       deletes its file; a later mod with the same identity starts with no key.
     - The app never prefills the judge origin from the provider's state or plan, and shows
       the origin next to the key before saving.
     - It stays bound to the judge's **origin** (the human's answer, N10), not to one path; the
       "Not protected" line of 14 adds: "the judge account reaches any `POST` path of the origin
       the participant entered".
AC4. Section 16 case 19 names both bindings; a new case: "the judge binding: another caller,
     `GET`, another origin, an origin not in `[http]`, userinfo, `:443` or upper case forms,
     an `account` other than `openai` or `judge`, and a redirect are each refused before a byte
     leaves; a removed key gives `no-account`."
AC5. Section 15's checklist rows for host and app carry AC3's rules (by reference to 5). No
     other meaning changes; `git diff --stat` touches only `design/TECHNICAL.md` (and
     `design/FUNCTIONAL.md` only if AGT-F-53 must say "removable": add "The participant shall
     see that a key is kept and remove it." to AGT-F-53).

## Spec

`design/TECHNICAL.md` sections 2.2, 5, 8, 10, 12, 14, 15, 16, 17; `design/FUNCTIONAL.md`
AGT-F-53; `factory/tasks/S2-1.md` (the N10 rule this refines).

## Interface

Fixed: the kernel's identity is the one `caller.identity()` returns for `mods:agent`; error
`not-owner` (providers) and `no-account`, `destination` (https-stream) as named above.

## Files

- `design/TECHNICAL.md`
- `design/FUNCTIONAL.md` (AGT-F-53 only)

## Changed tests

none (documents only).

## Out of scope

Binding the judge account to one URL path (the human said origin); any code; watane.
