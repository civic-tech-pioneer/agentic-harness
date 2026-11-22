#!/bin/sh
# .claude/factory/watch-pr.sh <slice id> [<state seen>] [<max seconds>]: the product owner's one watcher
# for a slice in the two-team loop. Returns when the state of slice/<id> differs from <state seen>: its
# PR opened, closed or merged, its head moved (the build team pushed) or it got a comment (a review, an
# answer). Prints the new state on line 1 (pass it back as <state seen> next time), then the head
# commit's message and the last comment. With <max seconds> it also returns after that long (exit 3), a
# timer for a Codex task, whose result shows only on its task page. Run it from the project root.
ID=$1 SEEN=${2:-} MAX=${3:-0} T0=$(date +%s)
state() {
  for B in "slice/$ID" "slice/$(echo "$ID" | tr '[:upper:]' '[:lower:]')"; do  # watane: S95 on slice/s95
    S=$(gh pr list --head "$B" --state all --json number,state,headRefOid,comments \
        --jq '.[0] // empty | "PR \(.number) \(.state) \(.headRefOid) \(.comments|length)"' 2>/dev/null)
    [ -n "$S" ] && { echo "$S"; return; }
    H=$(git ls-remote origin "refs/heads/$B" | cut -c1-40); [ -n "$H" ] && { echo "nopr $B $H"; return; }
  done
  echo none
}
while :; do
  S=$(state)
  if [ "$S" != "$SEEN" ]; then
    echo "$S"; set -- $S
    case $1 in
      PR) gh pr view "$2" --json commits,comments \
            --jq '(.commits[-1] | "head: \(.messageHeadline)\n\(.messageBody)"), (.comments[-1] // empty | "last comment by \(.author.login):\n\(.body)")' ;;
      nopr) git fetch -q origin "$2" && git log -1 --format=%B FETCH_HEAD ;;
    esac
    exit 0
  fi
  [ "$MAX" -gt 0 ] && [ $(( $(date +%s) - T0 )) -ge "$MAX" ] && { echo "$S"; echo "timer: no change in ${MAX}s"; exit 3; }
  sleep 120
done
