#!/usr/bin/env bash
# Slow production timer-origin check with Watane S136 or later and the scripted Responses server.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../../.." && pwd)
watane=$(cd "${WATANE_SRC:-$root/../watane}" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cargo build --manifest-path "$root/kernel/Cargo.toml" --release --target wasm32-wasip2 -p agent-mod
kernel_target=$(cargo metadata --manifest-path "$root/kernel/Cargo.toml" --no-deps --format-version 1 |
    python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
mkdir -p "$tmp/agent" "$tmp/check/src"
cp -R "$root/kernel/wasm/"{mod.toml,deps.json,wit} "$tmp/agent"
cp "$kernel_target/wasm32-wasip2/release/agent_mod.wasm" "$tmp/agent/logic.wasm"
cp "$root/kernel/wasm/tests/production/timer_origin.rs" "$tmp/check/src/main.rs"
python3 - "$watane" >"$tmp/check/Cargo.toml" <<'PY'
import json
from pathlib import Path
import sys

watane = Path(sys.argv[1])
print("""[package]
name = "agent-timer-origin-test"
version = "0.0.0"
edition = "2024"

[dependencies]
blake3 = "1"
redb = "4.5"
serde_json = "1"
tempfile = "3"
""")
for name in ("dev", "host"):
    print(f"watane-{name} = {{ path = {json.dumps(str(watane / name))} }}")
PY
cp "$watane/Cargo.lock" "$tmp/check/Cargo.lock"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$watane/target}" cargo run --manifest-path "$tmp/check/Cargo.toml" -- \
    "$tmp/agent" "$watane/host/tests/fixtures/mods/echo"
