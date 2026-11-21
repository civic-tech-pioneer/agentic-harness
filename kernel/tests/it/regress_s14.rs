//! Acceptance regressions of S14 (9.4: every rendered line, and a joined part group, takes the
//! `i` of its place, readable or not).

use agent_kernel::transcript::Joiner;
use serde_json::json;

/// Parts as `read` hands them out of a corrupt chunk: each part line is readable and carries
/// the `i` of its place, but the joined text is not an entry object. The joined whole comes
/// back with no `i` (and no `kind`), so a client cannot page past it or drop it as shown.
#[test]
fn regress_s14_joined_non_object_whole_has_no_index() {
    let part = |i: u64, n: u64, text: &str| {
        json!({"v":1,"i":i,"kind":"part","at":1,"body":{"n":n,"m":2,"text":text}}).to_string()
    };
    let mut j = Joiner::new();
    assert!(j.push(&part(10, 1, "[1,")).is_none());
    let whole = j
        .push(&part(11, 2, "2]"))
        .expect("a complete group yields a whole");
    assert_eq!(
        whole.get("i").and_then(|v| v.as_u64()),
        Some(11),
        "the joined whole must take its last part's i, readable or not; got {whole}"
    );
}
