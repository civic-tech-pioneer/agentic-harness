#!/bin/sh
# .claude/factory/watch-cloud.sh <slice id> [<commit already seen>]: returns when a cloud foreman has
# reported: a pushed commit whose subject is `DELIVERED <id>` or `BLOCKED <id>` (any branch on origin;
# prints the branch, the commit and its message), or the PR of slice/<id> is no longer open (merged or
# closed; prints its state, link and body). The PR check matters: a rebase merge drops the empty
# DELIVERED commit and GitHub deletes the branch, often before the next poll. Pass the last BLOCKED
# commit to wait for the next report. Run it from the project root. The id matches in any case.
ID=$1 SEEN=${2:-}
while :; do
  git fetch -q --prune origin 2>/dev/null
  C=$(git log --remotes=origin -1 --format=%H -E -i --grep="^(DELIVERED|BLOCKED) $ID\$")
  if [ -n "$C" ] && [ "$C" != "$SEEN" ]; then
    git branch -r --contains "$C" | head -1; echo "$C"; git log -1 --format=%B "$C"; exit 0
  fi
  P=$(gh pr list --head "slice/$ID" --state all --json state,url,body \
      --jq '.[0] | select(.state != "OPEN") | "PR \(.state) \(.url)\n\(.body)"' 2>/dev/null)
  [ -n "$P" ] && { echo "$P"; exit 0; }
  sleep 120
done
