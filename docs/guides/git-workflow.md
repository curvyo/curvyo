# Git workflow: linear history and stacked PRs

Customer decision of 2026-10-14. `main` has a linear history of a few
self-contained commits per PR. Summary rules live in `CLAUDE.md` §4 and §9;
this guide has the details and the commands.

## 1. A good commit

- One logical change, with its tests and docs. Reverting it alone leaves a
  working tree.
- **Every commit builds** (`cargo check --workspace --all-targets`) and the
  tests of the crates it touches pass. **The branch tip passes the full gate**
  (`CLAUDE.md` §7). We do not run the full gate per commit.
- Conventional Commit subject with the crate as scope, e.g.
  `feat(document-core): add clone node kind`. The body says why, not what.
- No `wip`, `fix review`, `address comments`, `fmt` or `merge main` commits in
  the final history. Fold them into the commit they belong to.
- A PR has **1-5 commits**. The milestone commits of a slice stay separate if
  each is self-contained (model, then UI, then docs); otherwise fold them.
  Mechanical moves or renames get their own commit.

## 2. Tidying history (agents, no interactive editor)

The Bash tool cannot run `git rebase -i` or `git add -p`. Do not try.

**Default: tidy before the first push, with a soft reset.** Nothing is shared
yet, so no force push is ever needed.

```text
git fetch origin
git rebase origin/main                    # resolve conflicts once, here
git reset --soft origin/main              # keep the tree, drop the commits
git restore --staged .
git add crates/document-core docs/adr     # group 1: paths of one logical change
git commit -m "feat(document-core): ..."
git add crates/editor-wasm frontend       # group 2
git commit -m "feat(editor-wasm): ..."
git diff origin/main HEAD --stat          # the tree must equal the old tip
```

Check that the tree did not change: save `git rev-parse HEAD^{tree}` before
the reset and compare after the last commit. A file that belongs to two
groups goes to the earlier one; do not split hunks.

**Fixups after the PR is opened** (review findings, a small correction to an
earlier commit):

```text
git commit --fixup <sha>                  # one per correction
GIT_SEQUENCE_EDITOR=true git rebase -i --autosquash origin/main
git push --force-with-lease origin story/<slug>
```

**Other tools:** `git rebase --onto <new-base> <old-base> <branch>` to move a
branch (section 4); `git rebase origin/main` to catch up. Never `git merge
origin/main` into a story branch.

**Safety rules**
- Rewrite only your own `story/`, `fix/` or `chore/` branch, in the
  project checkout. Never `main`, never a branch another agent works on.
- Force pushes name the branch, prefer `--force-with-lease`, and only ever
  target your own `story/`, `fix/` or `chore/` branch. Never `main`, never a
  wildcard refspec, never a branch someone else pushes to.
- Before any rewrite, `git branch backup/<slug>` locally. Delete it after the
  PR is merged.
- If a PR has a human review in progress (the customer commented on a
  commit), prefer new fixup commits and squash them only at the end.

**Settings (customer decision FORCE-LEASE, 2026-10-14).** `.claude/settings.json`
no longer denies `git push --force`, `-f` or `--force-with-lease`; `git push`
is allowed like any other push. The safety rules above are the guard: it is
the agent's own branch, and the agent is responsible for what it rewrites.
Prefer `--force-with-lease`; a plain `--force` is allowed on your own branch
when the lease keeps failing for a reason you understand (for example after
you rewrote the branch from another machine). `git reset --hard` stays denied.

## 3. Merging

GitHub has no true fast-forward merge. We use **Rebase and merge**, which
keeps the history linear and every commit separate (the repo has
`allow_rebase_merge: true`). Squash and merge commits are not used any more.

1. `git fetch origin && git rebase origin/main` on the PR branch, push, wait
   for green CI. A branch that is already up to date merges with the same
   commits where GitHub allows it; if main moved, the SHAs change and that is
   fine.
2. `gh pr merge <n> --rebase --delete-branch`.
   `tools/lead/strict-merge.sh <n>` does the wait-and-check in one step: it refuses to
   merge unless at least 10 checks exist and all passed (an empty check list is not green);
   `tools/lead/merge-chain.sh` does it for a series of PRs (see `tools/lead/README.md`).
3. Customer rules stay: `story/` PRs need customer acceptance each time;
   `chore/` and `fix/` PRs merge on green CI without asking (standing
   permission of 2026-10-03). `gh pr merge` stays in the `ask` list.

## 4. Stacked branches

**Not by default.** One story at a time; the next starts after the merge. Only when the customer
says so (he is away at night or on a weekend) may one branch build on an unmerged one, and never
more than **2 deep** (customer, 2026-10-25). `chore/` and `fix/` branches from `main` stay allowed
at any time.

```text
git switch -c story/<upper> story/<lower>
```

**Rules**
- A PR is always opened against `main`. The upper branch gets its PR only after the lower PR is
  merged and the upper branch is rebased onto `main` (`git rebase --onto origin/main <old lower
  tip>`). Lesson of #176: a PR whose base is another story branch merges into that branch, not
  into `main`, once the lower PR is gone.
- Stack only when the upper slice needs the lower one's code; independent work goes parallel
  from `main`.
- The agent that built the stack keeps it current: after every change to the lower branch,
  rebase the upper one and run the gate there.

## 5. Testing and absence

- **The customer tests** the open PR against `main`; an upper branch is tried after its lower PR
  is merged and it has its own PR.
- **While the customer is away** (and only if he said to work ahead) nothing merges that needs
  acceptance; `chore/` and `fix/` PRs still merge on green CI. The depth limit of 2 holds;
  further story work waits.
