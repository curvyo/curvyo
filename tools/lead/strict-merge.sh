#!/usr/bin/env bash
# Merge a pull request only when CI has really run and every check passed.
# Usage: tools/lead/strict-merge.sh <pr> [--rebase|--squash]   (default --rebase)
# Env:   GH_REPO (default curvyo/curvyo), MIN_CHECKS (default 10).
# An empty check list ("no checks reported") counts as not green, never as pass.
set -u
pr=${1:?usage: strict-merge.sh <pr> [--rebase|--squash]}
method=${2:---rebase}
repo=${GH_REPO:-curvyo/curvyo}
min=${MIN_CHECKS:-10}

sleep 45   # give a fresh push time to register its runs
gh -R "$repo" pr checks "$pr" --watch --interval 30 >/dev/null 2>&1
# A cancelled sibling run (push and pull_request both trigger CI) leaves two entries with an
# unexpanded matrix name ("${{ matrix... }}") that stay "fail"; they carry no information.
out=$(gh -R "$repo" pr checks "$pr" 2>/dev/null | grep -vF '${{' | cut -f2)
total=$(printf '%s\n' "$out" | grep -c .)
pass=$(printf '%s\n' "$out" | grep -c '^pass$')
echo "#$pr checks total=$total pass=$pass"
if [ "$total" -ge "$min" ] && [ "$total" = "$pass" ]; then
  gh -R "$repo" pr merge "$pr" "$method" --delete-branch 2>&1 | tail -1
  gh -R "$repo" pr view "$pr" --json state -q .state
else
  echo NOT-MERGED
  gh -R "$repo" pr checks "$pr" | grep -vF '${{' | grep -v pass
  exit 1
fi
