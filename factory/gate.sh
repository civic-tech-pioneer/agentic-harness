#!/bin/sh
# The project gate (factory/config.toml `gate`): run from the project root, stops at the first failure.
# Runs, in order: when kernel/wasm exists, its build to kernel/wasm/logic.wasm (cargo build --release --target wasm32-wasip2);
# .claude/factory/check-mod.sh (Markdown links; watane-dev when wit/world.wit exists);
# kernel/ `cargo fmt --check`, `cargo test`, then `cargo clippy --all-targets -- -D warnings` when kernel/Cargo.toml exists;
# provider/logic `cargo fmt --check`, `cargo test`, `cargo clippy -- -D warnings` when provider/logic/Cargo.toml exists;
# console/ `npm ci` then `npm test` when console/package.json exists.
# See factory/PLAN.md, slice S3, outcome 5 and the decision "One gate script".
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

# The prebuilt logic (watane dev 7.1): the kernel's component, built before check-mod.sh needs it.
if [ -f kernel/wasm/Cargo.toml ]; then
  echo "gate: kernel/wasm build (logic.wasm)"
  # CI adds the target only for a root wit/world.wit, so add it here when it is missing.
  if command -v rustup >/dev/null 2>&1 && ! rustup target list --installed | grep -qx wasm32-wasip2; then
    rustup target add wasm32-wasip2
  fi
  (cd kernel && cargo build --release --target wasm32-wasip2 -p agent-mod)
  # The target dir may be moved (CARGO_TARGET_DIR): ask cargo where it is.
  tdir=$(cd kernel && cargo metadata --no-deps --format-version 1 | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
  cp "$tdir/wasm32-wasip2/release/agent_mod.wasm" kernel/wasm/logic.wasm
fi

echo "gate: check-mod.sh"
.claude/factory/check-mod.sh

if [ -f kernel/Cargo.toml ]; then
  echo "gate: kernel cargo fmt --check"
  (cd kernel && cargo fmt --all --check)
  echo "gate: kernel cargo test"
  (cd kernel && cargo test)
  echo "gate: kernel cargo clippy"
  (cd kernel && cargo clippy --all-targets -- -D warnings)
else
  echo "gate: kernel skipped (no kernel/Cargo.toml)"
fi

if [ -f provider/logic/Cargo.toml ]; then
  echo "gate: provider cargo fmt --check"
  (cd provider/logic && cargo fmt --check)
  echo "gate: provider cargo test"
  (cd provider/logic && cargo test)
  echo "gate: provider cargo clippy"
  (cd provider/logic && cargo clippy -- -D warnings)
else
  echo "gate: provider skipped (no provider/logic/Cargo.toml)"
fi

if [ -f console/package.json ]; then
  echo "gate: console npm ci"
  (cd console && npm ci)
  echo "gate: console npm test"
  (cd console && npm test)
else
  echo "gate: console skipped (no console/package.json)"
fi
