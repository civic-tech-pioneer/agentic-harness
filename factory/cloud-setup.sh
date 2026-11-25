#!/bin/sh
# factory/cloud-setup.sh: the project's own SessionStart hook (.claude/settings.json), beside the
# factory's .claude/factory/cloud-setup.sh (sync.sh writes that one, so this project's tools live here).
# Readies an Anthropic cloud container for the console's browser and host tests. Does nothing on the
# Mac. Runs at every session start: each step skips itself when its tool is present, prints nothing,
# and the hook always exits 0.
[ "${CLAUDE_CODE_REMOTE:-}" = true ] || exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

# Linux packages, only those missing: .github/workflows/installed-agent.yml's line (WebKitGTK for the
# watane app that test:host, test:installed and test:load drive; xvfb, which that job runs as
# `xvfb-run -a dbus-run-session -- npm --prefix console run test:installed`), plus dbus-x11 for
# dbus-run-session. gh: the factory's scripts call it; the Debian and Ubuntu archives have it.
apt_sudo=
if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then apt_sudo="sudo -n"; else apt_sudo=none; fi
fi
if [ "$apt_sudo" != none ] && command -v apt-get >/dev/null 2>&1; then
  missing=
  for p in libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev xvfb xauth dbus-x11; do
    dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q 'ok installed' || missing="$missing $p"
  done
  command -v gh >/dev/null 2>&1 || missing="$missing gh"
  if [ -n "$missing" ]; then
    # shellcheck disable=SC2086 # split on purpose: the sudo prefix and the package list
    $apt_sudo env DEBIAN_FRONTEND=noninteractive apt-get update -qq >/dev/null 2>&1
    # shellcheck disable=SC2086
    $apt_sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends $missing >/dev/null 2>&1 ||
      for p in $missing; do # one name the archive lacks (gh on an old release) must not block the rest
        $apt_sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends "$p" >/dev/null 2>&1
      done
  fi
fi

# Playwright for `npm run test:slow` in console/: not a dependency, so console/test/slow-kit.mjs
# requires it from the global install through NODE_PATH. Installed globally with its Chromium (as
# this user, into its cache) and that browser's system libraries (install-deps, through apt).
playwright_version=1.65.1
if command -v npm >/dev/null 2>&1; then
  if [ "$apt_sudo" = none ]; then npm_sudo=; else npm_sudo=$apt_sudo; fi
  groot=$(npm root -g 2>/dev/null)
  if [ -n "$groot" ] && [ ! -d "$groot/playwright" ]; then
    # shellcheck disable=SC2086
    npm install -g --silent "playwright@$playwright_version" >/dev/null 2>&1 ||
      $npm_sudo npm install -g --silent "playwright@$playwright_version" >/dev/null 2>&1
  fi
  if [ -n "$groot" ] && [ -f "$groot/playwright/cli.js" ]; then
    browsers="${PLAYWRIGHT_BROWSERS_PATH:-$HOME/.cache/ms-playwright}"
    if ! ls -d "$browsers"/chromium-* >/dev/null 2>&1; then
      node "$groot/playwright/cli.js" install chromium >/dev/null 2>&1
      if [ "$apt_sudo" != none ]; then
        # shellcheck disable=SC2086
        $apt_sudo env PATH="$PATH" node "$groot/playwright/cli.js" install-deps chromium >/dev/null 2>&1
      fi
    fi
    # The session's shell gets NODE_PATH from the file Claude Code hands SessionStart hooks.
    if [ -n "${CLAUDE_ENV_FILE:-}" ] && ! grep -qs 'NODE_PATH=' "$CLAUDE_ENV_FILE"; then
      # shellcheck disable=SC2016 # ${NODE_PATH...} expands when the session sources the file
      printf 'export NODE_PATH="%s${NODE_PATH:+:$NODE_PATH}"\n' "$groot" >>"$CLAUDE_ENV_FILE"
    fi
  fi
fi
exit 0
