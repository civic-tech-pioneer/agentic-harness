#!/usr/bin/env bash
# Slow production timer-origin check with Watane S136 or later and the scripted Responses server.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../../.." && pwd)
watane=$(cd "${WATANE_SRC:-$root/../watane}" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cargo build --manifest-path "$root/kernel/Cargo.toml" --release --target wasm32-wasip2 -p agent-mod
cargo build --manifest-path "$root/provider/logic/Cargo.toml" --release --target wasm32-wasip2
target_dir() {
    cargo metadata --manifest-path "$1" --no-deps --format-version 1 |
        python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])'
}
mkdir -p "$tmp/agent" "$tmp/claude-code" "$tmp/check/src"
cp -R "$root/kernel/wasm/"{mod.toml,deps.json,wit} "$tmp/agent"
cp "$(target_dir "$root/kernel/Cargo.toml")/wasm32-wasip2/release/agent_mod.wasm" "$tmp/agent/logic.wasm"
cp -R "$root/provider/"{mod.toml,wit} "$tmp/claude-code"
cp "$(target_dir "$root/provider/logic/Cargo.toml")/wasm32-wasip2/release/claude_code.wasm" "$tmp/claude-code/logic.wasm"
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
sha2 = "0.10"
tempfile = "3"
""")
for name in ("dev", "host"):
    print(f"watane-{name} = {{ path = {json.dumps(str(watane / name))} }}")
PY
cp "$watane/Cargo.lock" "$tmp/check/Cargo.lock"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$watane/target}" cargo run --manifest-path "$tmp/check/Cargo.toml" -- \
    "$tmp/agent" "$tmp/claude-code" "$watane/host/tests/fixtures/mods/echo"
