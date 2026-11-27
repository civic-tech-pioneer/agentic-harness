#!/usr/bin/env bash
# The Harness exposes client calls and subscriptions. Add an adapter to the disposable
# component so the contract steps can invoke the production host-only timer callback.
set -euo pipefail
kernel=$(cd "$(dirname "$0")/../../.." && pwd)
watane=$(cd "${WATANE_SRC:-$kernel/../../watane}" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/kernel/wasm" "$tmp/agent/tests"
cp -R "$kernel/"{Cargo.toml,Cargo.lock,src} "$tmp/kernel"
cp -R "$kernel/wasm/"{Cargo.toml,mod.toml,deps.json,src,wit} "$tmp/kernel/wasm"
cp "$kernel/wasm/tests/harness/timer_probe.rs" "$tmp/kernel/wasm/src"
cat >>"$tmp/kernel/wasm/src/component.rs" <<'RUST'
include!("timer_probe.rs");
RUST
mkdir -p "$tmp/kernel/wasm/wit/deps/s153-client"
cat >"$tmp/kernel/wasm/wit/deps/s153-client/world.wit" <<'WIT'
package mods:s153-client@0.1.0;
interface events {
  client-profiles: func();
}
WIT
python3 - "$tmp/kernel/wasm/wit/world.wit" "$tmp/kernel/wasm/deps.json" <<'PY'
from pathlib import Path
import json
import sys

path = Path(sys.argv[1])
world = path.read_text()
path.write_text(world.replace("world agent {", """interface timer-probe {
  session-firing-keeps-pause: func() -> bool;
  seed-drain-owed: func();
  drain-owed: func() -> bool;
}

world agent {
  export timer-probe;
  export mods:s153-client/events@0.1.0;""", 1))
deps = Path(sys.argv[2])
value = json.loads(deps.read_text())
value["mods:s153-client"] = None
deps.write_text(json.dumps(value))

PY
CARGO_TARGET_DIR="$tmp/target" cargo build --manifest-path "$tmp/kernel/Cargo.toml" --release --target wasm32-wasip2 -p agent-mod
cp -R "$tmp/kernel/wasm/"{mod.toml,deps.json,wit} "$tmp/agent"
cp "$tmp/target/wasm32-wasip2/release/agent_mod.wasm" "$tmp/agent/logic.wasm"
cat >"$tmp/agent/tests/default.json" <<'JSON'
{"steps":[{"call":"timer-probe.session-firing-keeps-pause","returns":false}]}
JSON
cat >"$tmp/agent/tests/override.json" <<'JSON'
{"fakes":{"watane:host/timers":{"firing-origin":"local/probe-owner"}},"steps":[{"call":"timer-probe.session-firing-keeps-pause","returns":true}]}
JSON
WATANE_HOME="$tmp/home" WATANE_DEV_CACHE="$tmp/cache" watane-dev test --json --dir "$tmp/agent"

# The JSON runner checks return values. Use the same Harness API to assert the
# timer requests it records and to deliver an ordinary mod-origin subscription.
mkdir -p "$tmp/check/src"
cp "$kernel/wasm/tests/harness/client_guard.rs" "$tmp/check/src/main.rs"
python3 - "$watane" >"$tmp/check/Cargo.toml" <<'PYTHON'
import json
import sys
from pathlib import Path

print("""[package]
name = "agent-client-guard-test"
version = "0.0.0"
edition = "2024"

[dependencies]
serde_json = "1"
""")
print(f"watane-host = {{ path = {json.dumps(str(Path(sys.argv[1]) / 'host'))} }}")
PYTHON
cp "$watane/Cargo.lock" "$tmp/check/Cargo.lock"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$watane/target}" cargo run --manifest-path "$tmp/check/Cargo.toml" -- "$tmp/agent"
