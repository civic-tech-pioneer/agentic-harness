# .claude/factory/cfg.sh, sourced: sets F (this folder), W (the project root), R (the run logs in the
# main checkout, so they outlive a worktree) and cfg <key> (a one-line value of factory/config.toml).
F=$(cd "$(dirname "$0")" && pwd); W=$(cd "$F/../.." && pwd)
R=$(dirname "$(git -C "$W" rev-parse --path-format=absolute --git-common-dir)")/factory/runs
cfg() { sed -n "s/^$1 *= *\"\(.*\)\".*/\1/p" "$W/factory/config.toml" | head -1; }
