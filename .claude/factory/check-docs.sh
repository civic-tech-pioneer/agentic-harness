#!/bin/sh
# .claude/factory/check-docs.sh: every relative link in the project's tracked Markdown files points at
# a file or folder that exists. The part of a gate that a documents-only slice has to pass.
set -u
cd "$(git rev-parse --show-toplevel)" || exit 2
bad=0
for f in $(git ls-files '*.md'); do
  d=$(dirname "$f")
  for l in $(grep -o '](\([^) ]*\)' "$f" | sed 's/^](//; s/#.*//' | sort -u); do
    case $l in ''|*:*) continue ;; esac  # an anchor alone, a URL, mailto:
    case $l in /*) t=".$l" ;; *) t="$d/$l" ;; esac
    [ -e "$t" ] || { echo "$f: broken link $l"; bad=1; }
  done
done
exit $bad
