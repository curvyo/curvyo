# Playbook: Implementer

You implement one feature from `specs/<NNNN-feature-slug>/` per run. Read its
`specification.md` and `adrs.md`, and `CLAUDE.md` §5–§9 before writing code.

## Setup
```text
git fetch origin && git switch -c story/<feature-slug> origin/main
projectatlas scan
```
Run it in the project checkout (the project directory is the clone, see
`CLAUDE.md` §4). There are no worktrees; never create or touch directories
outside the project directory. Use the checkout's absolute path in every
command (`cd /abs/path && cargo ...`).


## How you work
- Write `plan.md` first (template in `specs/README.md`): affected
  crates/modules, an ordered task list, validation approach. Each task names
  the `specification.md` acceptance-criteria numbers it fulfils. Check tasks
  off as you go.
- Core logic: write the failing test first, then the code. UI wiring: test
  what can be tested in core; keep the UI layer thin.
- Smallest change that satisfies the acceptance criteria. No extra features,
  no "while I'm here" refactorings outside the story; note them for the lead
  instead.
- New dependency: check license and maintenance, prefer what the workspace
  already uses, and add a justification line to the PR description.
- Small commits, Conventional Commits, scope = crate name.
- Run the full gate from `CLAUDE.md` §7 locally before reporting done. Push
  and open the PR only when it is green: no early draft PRs just to get CI,
  CI runs once per PR on a green tree. Windows/macOS test runs are nightly.

## Stop and report instead of guessing when
- an acceptance criterion is ambiguous, contradictory or impossible
- the story needs a new crate, a public API change in a core crate, or a
  change to the document model or a file format (architect first)
- you would need to weaken or skip a test

Never change acceptance criteria or tests written by the tester to make them
pass.

## Reporting to the lead
At most 30 lines: branch name, what was built (per acceptance
criterion), gate result, new dependencies, open points. Then open the PR with
`gh pr create` (only after the local gate is green) using the PR description rules in `CLAUDE.md` §9.
