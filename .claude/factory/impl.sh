#!/bin/bash
# .claude/factory/impl.sh <task> [resume]: runs the Sol implementer (Codex) on factory/tasks/<task>.md,
# or sends it back with feedback (read from stdin) in the same Codex session. Run it from the slice's
# worktree. The brief's header may set `Folder: <path>` (default: the project root) and
# `Effort: <low|medium|high|xhigh>`. Output goes to factory/runs/ of the main checkout:
# <task>.{jsonl,last.md,session,base}, and one line per run in ledger.jsonl; a new run of a task that
# ran before first moves the old files to <task>.<n>.*. Then check.sh runs the gate; its last line
# (CHECK PASS/FAIL) and its exit are impl.sh's.
set -u
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
. "$(dirname "$0")/cfg.sh"
G=$(git -C "$W" rev-parse --path-format=absolute --git-common-dir); GD=$(git -C "$W" rev-parse --absolute-git-dir)
ID=$1 MODE=${2:-new} T=$W/factory/tasks/$1.md
[ -f "$T" ] || { echo "no brief $T" >&2; exit 2; }
DIR=$(sed -n 's/^Folder: *//p' "$T" | head -1); EFFORT=$(sed -n 's/^Effort: *//p' "$T" | head -1)
[ -d "$W/${DIR:-.}" ] || { echo "no folder ${DIR} in $W" >&2; exit 2; }
mkdir -p "$R"
# ponytail: Codex's own workspace-write sandbox plus network; widen only when a gate needs it.
OPTS=(-m gpt-6.1-sol -c "model_reasoning_effort=\"${EFFORT:-medium}\"" -c 'sandbox_mode="workspace-write"'
      -c 'sandbox_workspace_write.network_access=true' --json -o "$R/$ID.last.md"
      # writable: the worktree, both git dirs (Codex protects .git unless each is named), runs, cargo home.
      -c "sandbox_workspace_write.writable_roots=[\"$W\",\"$G\",\"$GD\",\"$R\",\"${CARGO_HOME:-$HOME/.cargo}\"${CARGO_TARGET_DIR:+,\"$CARGO_TARGET_DIR\"}]")
cd "$W/${DIR:-.}" || exit 2
if [ "$MODE" = new ] && [ -e "$R/$ID.jsonl" ]; then
  N=1; while [ -e "$R/$ID.$N.jsonl" ]; do N=$((N+1)); done
  for X in jsonl last.md session base check.log; do [ -e "$R/$ID.$X" ] && mv "$R/$ID.$X" "$R/$ID.$N.$X"; done
fi
[ "$MODE" = new ] && git -C "$W" rev-parse HEAD > "$R/$ID.base"
L0=$(cat "$R/$ID.jsonl" 2>/dev/null | wc -l)
T0=$(date +%s)
if [ "$MODE" = resume ]; then
  SID=$(cat "$R/$ID.session") || exit 2
  codex exec resume "$SID" "${OPTS[@]}" - >> "$R/$ID.jsonl" 2>&1; RC=$?
else
  { cat "$F/implementer.md"; printf '\n\n# THE TASK\n\n'; cat "$T"; } |
    codex exec "${OPTS[@]}" - > "$R/$ID.jsonl" 2>&1; RC=$?
  grep -m1 -o '"thread_id":"[^"]*"' "$R/$ID.jsonl" | cut -d'"' -f4 > "$R/$ID.session"
fi
U=$(tail -n +$((L0+1)) "$R/$ID.jsonl" | grep '"turn.completed"' | tail -1 | grep -o '"usage":{[^}]*}')
T1=$(date +%s)
CHECK=$("$F/check.sh" "$ID" "$(cat "$R/$ID.base" 2>/dev/null)"); CRC=$?
printf '{"task":"%s","mode":"%s","effort":"%s","start":%s,"end":%s,"exit":%s,"check":%s,"check_end":%s,%s}\n' \
  "$ID" "$MODE" "${EFFORT:-medium}" "$T0" "$T1" "$RC" "$CRC" "$(date +%s)" "${U:-\"usage\":null}" >> "$R/ledger.jsonl"
echo "$ID $MODE exit=$RC"; tail -5 "$R/$ID.last.md" 2>/dev/null; echo "$CHECK"
[ $RC -ne 0 ] && exit $RC; exit $CRC
