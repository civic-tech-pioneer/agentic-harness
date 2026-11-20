#!/bin/sh
# The project gate (factory/config.toml `gate`): run from the project root, stops at the first failure.
# Runs, in order: .claude/factory/check-mod.sh (Markdown links; watane-dev when wit/world.wit exists);
# kernel/ `cargo test` then `cargo clippy --all-targets -- -D warnings` when kernel/Cargo.toml exists;
# console/ `npm ci` then `npm test` when console/package.json exists.
# See factory/PLAN.md, slice S3, outcome 5 and the decision "One gate script".
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

echo "gate: check-mod.sh"
.claude/factory/check-mod.sh

if [ -f kernel/Cargo.toml ]; then
  echo "gate: kernel cargo test"
  (cd kernel && cargo test)
  echo "gate: kernel cargo clippy"
  (cd kernel && cargo clippy --all-targets -- -D warnings)
else
  echo "gate: kernel skipped (no kernel/Cargo.toml)"
fi

if [ -f console/package.json ]; then
  echo "gate: console npm ci"
  (cd console && npm ci)
  echo "gate: console npm test"
  (cd console && npm test)
else
  echo "gate: console skipped (no console/package.json)"
fi
