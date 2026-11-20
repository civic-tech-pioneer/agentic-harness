#!/bin/sh
# .claude/factory/cloud-setup.sh: SessionStart hook (.claude/settings.json). Readies an Anthropic cloud
# container for a mod's gate: the Wasm target and watane-dev, built from watane's release tag (the
# release ships only a macOS build). Does nothing on the Mac, or in a project with no mod code yet.
[ "${CLAUDE_CODE_REMOTE:-}" = true ] || exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
[ -f wit/world.wit ] || exit 0
rustup target add wasm32-wasip2 >/dev/null 2>&1
command -v watane-dev >/dev/null || cargo install -q --locked --git https://github.com/civic-tech-pioneer/watane --tag v0.1.0 watane-dev >/dev/null 2>&1
exit 0
