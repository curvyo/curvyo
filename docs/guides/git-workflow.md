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
- Rewrite only your own `story/`, `fix/` or `chore/` branch, in your own
  worktree. Never `main`, never a branch another agent works on.
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
you rewrote the branch from another worktree). `git reset --hard` stays denied.

## 3. Merging

GitHub has no true fast-forward merge. We use **Rebase and merge**, which
keeps the history linear and every commit separate (the repo has
`allow_rebase_merge: true`). Squash and merge commits are not used any more.

1. `git fetch origin && git rebase origin/main` on the PR branch, push, wait
   for green CI. A branch that is already up to date merges with the same
   commits where GitHub allows it; if main moved, the SHAs change and that is
   fine.
2. `gh pr merge <n> --rebase --delete-branch`.
3. Customer rules stay: `story/` PRs need customer acceptance each time;
   `chore/` and `fix/` PRs merge on green CI without asking (standing
   permission of 2026-10-03). `gh pr merge` stays in the `ask` list.

## 4. Stacked PRs

Allowed so work can continue while the customer is away. **Maximum depth 3**
(one PR on main, two on top). Any number of parallel PRs from `main` stays
allowed (two implementers at once, `CLAUDE.md` §4).

**Build**
```text
git worktree add ../<upper-slug> -b story/<upper> story/<lower>
gh pr create --base story/<lower> --head story/<upper> ...
```
The upper PR's base is the lower story branch, so its diff shows only its own
commits. Stack only when the upper slice needs the lower one's code; independent
work goes parallel from `main`.

**Stack table.** Every PR in a stack starts its description (before "What
changes in the demo") with:
`Stack: #A -> #B -> #C. Merge in this order. This is #B.`
Update all descriptions when the stack changes.

**Descriptions.** "What changes in the demo" and the Review guide cover only
the diff against the PR's base. The upper PR says "Builds on #A". Dependency
and ADR changes appear in the PR that makes them.

**Rules**
- Merge strictly bottom-up. Never merge an upper PR first.
- CI runs on every PR against its own base. The wasm32 build and the gate run
  per PR, locally before each push as usual. The upper PR is not green-for-merge
  until it has been rebased onto `main` after the lower one merged.
- The agent that built the stack maintains it (rebases, retargets, table). If
  a lower PR changes after review, fix it there, then rebase each upper
  branch: `git rebase --onto story/<lower> <old-lower-tip> story/<upper>`
  and push with `--force-with-lease`. Note `<old-lower-tip>` before the fix.
- One slice is still one PR (`CLAUDE.md` §4): a stack is several slices, never
  one slice cut into invisible pieces.
- `format_version` numbers (`specs/README.md`) are taken at merge, in merge
  order. A stacked PR with a format change uses the next number after the
  PRs below it and says so in the table; renumber if the order changes.

**After the lowest PR is merged** (rebase-merge gives new SHAs, so the upper
branch still contains the old commits)
```text
git fetch origin
OLD=$(git rev-parse origin/story/<lower>)   # record this before the merge
# ... lower PR merged and branch deleted ...
git rebase --onto origin/main $OLD story/<upper>
git push --force-with-lease origin story/<upper>
gh pr edit <upper-pr> --base main
```
Then rerun the gate, update the stack table, and repeat for the next PR.
If `OLD` was lost, use `git log --oneline origin/main..story/<upper>` and pick
the first commit that belongs to the upper PR.

## 5. Testing and absence

- **Customer tests a stack** by checking out the top PR branch (`gh pr
  checkout <n>`) to try everything at once, or by accepting and merging
  bottom-up one PR at a time. The lead's demo message says which.
- **While the customer is away** stacks may grow to depth 3 and wait for
  acceptance. Nothing merges that needs acceptance. `chore/` and `fix/` PRs
  still merge on green CI; if a stack sits on top of main and main moves,
  the maintaining agent rebases it.
- At depth 3, new story work starts in parallel from `main` or waits.
