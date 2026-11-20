#!/bin/sh
# Run by hand: proves factory/gate.sh (S3-3 AC3) in scratch copies; never touches the real kernel/ or console/.
set -e
src=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkcopy() { # name
  mkdir -p "$tmp/$1/factory" "$tmp/$1/.claude"
  cp "$src/factory/gate.sh" "$tmp/$1/factory/"
  cp -R "$src/.claude/factory" "$tmp/$1/.claude/factory"
}
fail() { echo "FAIL: $1"; exit 1; }

mkcopy none
out=$(cd "$tmp/none" && factory/gate.sh 2>&1) || fail "none: expected exit 0: $out"
echo "$out" | grep -q "kernel skipped" || fail "none: no kernel skip line"
echo "$out" | grep -q "console skipped" || fail "none: no console skip line"
echo "ok none"

mkcopy kern
mkdir -p "$tmp/kern/kernel/src"
printf '[package]\nname = "t"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n' > "$tmp/kern/kernel/Cargo.toml"
printf '#[test]\nfn t() { assert!(false); }\n' > "$tmp/kern/kernel/src/lib.rs"
if out=$(cd "$tmp/kern" && factory/gate.sh 2>&1); then fail "kernel: expected non-zero"; fi
echo "$out" | grep -q "test result: FAILED" || fail "kernel: did not fail in cargo test: $out"
echo "$out" | grep -q "kernel cargo clippy" && fail "kernel: did not stop at the first failure"
echo "ok kernel failing test"

mkcopy cons
mkdir -p "$tmp/cons/console"
printf '{"name":"c","version":"0.0.0","scripts":{"test":"exit 1"}}\n' > "$tmp/cons/console/package.json"
echo '{"name":"c","version":"0.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"c","version":"0.0.0"}}}' > "$tmp/cons/console/package-lock.json"
if out=$(cd "$tmp/cons" && factory/gate.sh 2>&1); then fail "console: expected non-zero"; fi
echo "$out" | grep -q "gate: console npm test" || fail "console: did not reach npm test: $out"
echo "ok console failing test"
