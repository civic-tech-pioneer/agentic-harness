Folder: kernel
Effort: high
Depends on: none
Slice: S21 — a kernel maker drives sessions through 9.3 on the in-memory host-7 state of S11
## Goal
`start` uses only a profile the caller may use (`access/<identity>`, 9.1; adversary breach 2),
and the client keys and pushes are bound tightly (adversary RISKs): a kept key belongs to one
owner and one session, its state key cannot collide across owners whose identities contain
`/`, and every push names its session. Decision at a security boundary (named here): the
`access/<identity>` value is a JSON array of profile names; an absent or unreadable value
allows none (fail closed).
## Acceptance
AC1 The red test `adv_s21_start_uses_a_profile_the_client_has_no_access_to` passes unchanged:
    `start` reads `access/<caller>` and refuses a profile not named there (and any profile when
    the key is absent or not a JSON array of strings) with one refusal `{field: "profile"}`,
    writing nothing. Tests that start sessions seed `access/<caller>` (update the helpers of
    sessions.rs and restart.rs).
AC2 Key state path: the owner segment of `key/<owner>/<key>` and `owner/<owner>/<n>` is encoded
    so the map from (owner, key) to the state key is injective: `%` -> `%25`, `/` -> `%2F` in the
    owner. A test: owner `page/x` with key `k` and owner `page` with key `x/k` are separate keys,
    both succeed (AC2 of S21-3: "the same key of another owner is a separate key").
AC3 A kept key also stores the session it acted on (`say`, `answer`); a replay of it naming
    another session is refused `{field: "key"}` and changes nothing; a replay on a session the
    caller does not own is `NotOwner` (owner check before the replay's answer).
AC4 `Called.pushes` becomes `Vec<Push>` with `pub struct Push { pub session: String, pub value: Value }`,
    so a push from `timer("restart")` names the session it came from; tests updated mechanically.
AC5 Every earlier test passes (with the mechanical changes above); nothing else changes.
## Spec
TECHNICAL 9.1 (`access/<identity>`, `key/<owner>/<key>`, `owner/<owner>/<n>`), 2.1
(`caller.identity()` forms: `<publisher>/<slug>`, `local/<slug>`, `page`, `agent:<handle-name>`),
14 ("A session is private", "A repeated call changes nothing", "A profile or overlay cannot widen
what the participant set").
## Interface
`Push { session, value }` in `kernel/src/sessions.rs`, exported from lib.rs; `Called<T> { value, pushes: Vec<Push> }`.
## Files
kernel/src/sessions.rs, kernel/src/lib.rs, kernel/tests/it/sessions.rs, kernel/tests/it/restart.rs,
kernel/tests/it/adv_s21.rs (add tests; do not change the access red test).
## Changed tests
Helpers in sessions.rs and restart.rs seed `access/<caller>`; tests reading `pushes` read
`.value`; tests that read `key/...` or `owner/...` state keys by literal path use the encoded
owner (searched: only keys of owners without `/` or `%` are read literally, so unchanged).
## Out of scope
The queue (S21-fix1), open-ask caps (deadlines slice), streams.
