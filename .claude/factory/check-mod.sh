#!/bin/sh
# .claude/factory/check-mod.sh: the gate of a watane mod, run from its root: watane-dev doctor, then
# its contract tests; before that, the Markdown links. A project without wit/world.wit has no mod code
# yet: only the links are checked.
set -u
"$(dirname "$0")/check-docs.sh" || exit 1
[ -f wit/world.wit ] || { echo "no wit/world.wit: no mod code yet; links checked"; exit 0; }
WATANE_HOME=$(mktemp -d); export WATANE_HOME; trap 'rm -rf "$WATANE_HOME"' EXIT  # never the real home
watane-dev doctor --json || exit 1
watane-dev test --json
