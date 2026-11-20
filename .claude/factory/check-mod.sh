#!/bin/sh
# .claude/factory/check-mod.sh: the gate of a watane mod, run from its root: watane-dev doctor, then
# its contract tests. A project without wit/world.wit has no mod code yet, so nothing to check.
set -u
[ -f wit/world.wit ] || { echo "no wit/world.wit: no mod code to check yet"; exit 0; }
WATANE_HOME=$(mktemp -d); export WATANE_HOME; trap 'rm -rf "$WATANE_HOME"' EXIT  # never the real home
watane-dev doctor --json || exit 1
watane-dev test --json
