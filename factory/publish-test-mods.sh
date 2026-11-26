#!/usr/bin/env bash
# TECHNICAL 1, 17. Assemble and publish the two mods to a local Watane site.
set -euo pipefail
if [[ $# != 2 ]]; then
  echo "Usage: $0 <publisher-home> <publisher-channel-link>" >&2
  exit 2
fi

root=$(cd "$(dirname "$0")/.." && pwd)
publisher_home=$(cd "$1" && pwd)
channel=$2
if [[ ! $channel =~ ^watane:([ybndrfg8ejkmcpqxot1uwisza345h769]{52})@http://(127\.0\.0\.1|localhost):[0-9]+/$ ]]; then
  echo "Use the publisher app's channel link with a loopback site URL and port." >&2
  exit 2
fi
publisher=${BASH_REMATCH[1]}

stage=$(mktemp -d "${TMPDIR:-/tmp}/agent-release.XXXXXXXX")
trap 'rm -rf "$stage"' EXIT

# Only copy package inputs. In particular, omit the provider's source so dev
# uses the prebuilt logic that pins the publishing channel's agent identity.
mkdir -p "$stage/claude-code" "$stage/agent"
for file in mod.toml wit tests; do
  cp -R "$root/provider/$file" "$stage/claude-code/"
  cp -R "$root/kernel/wasm/$file" "$stage/agent/"
done

build_component() {
  local crate=$1 artifact=$2 destination=$3
  shift 3
  cargo build --locked --manifest-path "$root/$crate/Cargo.toml" \
    --release --target wasm32-wasip2 "$@" >&2
  local target
  target=$(cargo metadata --manifest-path "$root/$crate/Cargo.toml" --no-deps --format-version 1 |
    node -e 'console.log(JSON.parse(require("fs").readFileSync(0, "utf8")).target_directory)')
  cp "$target/wasm32-wasip2/release/$artifact.wasm" "$stage/$destination/logic.wasm"
}

export AGENT_KERNEL_IDENTITY="$publisher/agent"
cargo test --locked --manifest-path "$root/provider/logic/Cargo.toml" --lib ac2_ >&2
build_component kernel agent_mod agent -p agent-mod
build_component provider/logic claude_code claude-code
(
  cd "$root/console"
  npm ci
  npm exec -- vite build --outDir "$stage/agent/ui" --emptyOutDir
  node scripts/hash.mjs "$stage/agent/ui"
) >&2

publish() {
  local slug=$1
  local result expected_link="$channel#$slug"
  echo "Approve the $slug release in the publisher app." >&2
  if ! result=$(watane-dev publish --json --no-source --home "$publisher_home" --dir "$stage/$slug"); then
    printf '%s\n' "$result" >&2
    return 1
  fi
  if ! node -e 'process.exit(JSON.parse(require("fs").readFileSync(0, "utf8")).result.link === process.argv[1] ? 0 : 1)' "$expected_link" <<<"$result"; then
    echo "The published link did not match the configured test channel." >&2
    printf '%s\n' "$result" >&2
    return 1
  fi
}

publish claude-code
watane-dev add --release "$channel#claude-code" --json --home "$publisher_home" --dir "$stage/agent" >&2
watane-dev check --json --dir "$stage/agent" >&2
publish agent
printf '%s\n' "$channel#agent"
