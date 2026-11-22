#!/bin/bash
# .claude/factory/check.sh <task> [<base commit>]: runs the project's gate (factory/config.toml `gate`)
# for a task, in the current checkout: the build team after each task, the review team once per round.
# Log: factory/runs/<task>.check.log of the main checkout. Prints `CHECK PASS <commit>` or
# `CHECK FAIL <commit> (log <path>)`; exits 1 on a failure.
set -u
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
. "$(dirname "$0")/cfg.sh"
ID=$1 BASE=${2:-$(git -C "$W" merge-base HEAD main)}
GATE=$(cfg gate); [ -n "$GATE" ] || { echo "no gate in $W/factory/config.toml" >&2; exit 2; }
mkdir -p "$R"; LOG=$R/$ID.check.log
cd "$W" || exit 2
HEAD=$(git rev-parse --short HEAD); [ -n "$(git status --porcelain)" ] && HEAD="$HEAD+dirty"
echo "# $ID at $HEAD, base $BASE, $(date): $GATE" > "$LOG"
if TASK=$ID BASE=$BASE bash -c "$GATE" >> "$LOG" 2>&1; then
  echo "CHECK PASS $HEAD" | tee -a "$LOG"; exit 0
fi
echo "CHECK FAIL $HEAD (log $LOG)" | tee -a "$LOG"; exit 1
