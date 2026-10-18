# Lead scripts

Small helpers for merging pull requests safely. Both need `gh` logged in.

| Script | What it does |
|---|---|
| `strict-merge.sh <pr> [--rebase\|--squash]` | Waits for CI, then merges only if at least `MIN_CHECKS` (default 10) checks exist and every one passed. "No checks reported" counts as not green. |
| `merge-chain.sh <pr>:<worktree>:<branch> ...` | For several docs or chore PRs in a row: rebase the worktree onto `origin/main`, push with `--force-with-lease`, retarget the PR to `main`, run `strict-merge.sh`. Stops at the first conflict or red check. |

Run `merge-chain.sh` from the project directory (the one that holds `base/`) and
start it detached (`setsid nohup ... &`), so a restart of the session does not
kill it. `GH_REPO` overrides the repository (default `curvyo/curvyo`).

The rules these scripts serve are in `docs/guides/git-workflow.md` (section 3).
