#!/usr/bin/env bash
# Rebase, push and merge a series of docs or chore PRs one after another.
# Usage: tools/lead/merge-chain.sh <pr>:<checkout>:<branch> ...
#   run from the project directory (the clone); <checkout> is a path relative
#   to it, normally `.`. For every entry: rebase onto origin/main, push with
#   --force-with-lease, retarget the PR to main, then tools/lead/strict-merge.sh.
#   Stops at the first conflict, red check or missing check; resolve it by hand.
# Start it detached so a restart of the session does not kill it:
#   setsid nohup tools/lead/merge-chain.sh 143:.:chore/x ... > chain.log 2>&1 &
set -u
here=$(cd "$(dirname "$0")" && pwd)
repo=${GH_REPO:-curvyo/curvyo}
[ "$#" -gt 0 ] || { echo "usage: merge-chain.sh <pr>:<checkout>:<branch> ..."; exit 2; }

for entry in "$@"; do
  IFS=: read -r pr wt branch <<<"$entry"
  git -C "$wt" fetch -q origin
  if ! git -C "$wt" rebase origin/main >/dev/null 2>&1; then
    echo "CONFLICT #$pr in $wt: $(git -C "$wt" status -s | grep -E '^(UU|AA)' | tr '\n' ' ')"
    git -C "$wt" rebase --abort
    exit 1
  fi
  git -C "$wt" push -q --force-with-lease origin "$branch" 2>&1 | grep -v '^remote'
  gh -R "$repo" pr edit "$pr" --base main >/dev/null 2>&1
  "$here/strict-merge.sh" "$pr" --rebase || exit 1
done
echo ALL-DONE
