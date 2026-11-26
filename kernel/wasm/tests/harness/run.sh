#!/usr/bin/env bash
# The Harness exposes client calls and subscriptions. Add an adapter to the disposable
# component so the contract steps can invoke the production host-only timer callback.
set -euo pipefail
kernel=$(cd "$(dirname "$0")/../../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/kernel/wasm" "$tmp/agent/tests"
cp -R "$kernel/"{Cargo.toml,Cargo.lock,src} "$tmp/kernel"
cp -R "$kernel/wasm/"{Cargo.toml,mod.toml,deps.json,src,wit} "$tmp/kernel/wasm"
cp "$kernel/wasm/tests/harness/timer_probe.rs" "$tmp/kernel/wasm/src"
cat >>"$tmp/kernel/wasm/src/component.rs" <<'RUST'
include!("timer_probe.rs");
RUST
python3 - "$tmp/kernel/wasm/wit/world.wit" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
world = path.read_text()
path.write_text(world.replace("world agent {", """interface timer-probe {
  session-firing-keeps-pause: func() -> bool;
}

world agent {
  export timer-probe;""", 1))
PY
export CARGO_TARGET_DIR="$tmp/target"
cargo build --manifest-path "$tmp/kernel/Cargo.toml" --release --target wasm32-wasip2 -p agent-mod
cp -R "$tmp/kernel/wasm/"{mod.toml,deps.json,wit} "$tmp/agent"
cp "$CARGO_TARGET_DIR/wasm32-wasip2/release/agent_mod.wasm" "$tmp/agent/logic.wasm"
cat >"$tmp/agent/tests/default.json" <<'JSON'
{"steps":[{"call":"timer-probe.session-firing-keeps-pause","returns":false}]}
JSON
cat >"$tmp/agent/tests/override.json" <<'JSON'
{"fakes":{"watane:host/timers":{"firing-origin":"local/probe-owner"}},"steps":[{"call":"timer-probe.session-firing-keeps-pause","returns":true}]}
JSON
WATANE_HOME="$tmp/home" WATANE_DEV_CACHE="$tmp/cache" watane-dev test --json --dir "$tmp/agent"
