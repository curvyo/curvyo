# CLAUDE.md

Curvyo: one vector app to draw, prepare and produce work on laser cutters, cutting plotters,
embroidery machines and CNC mills. Desktop first (Linux, Windows, macOS), browser second,
mobile companion later. Loaded into every session: keep it short, details live in `docs/`.

## 1. Language and reporting

- Everything in the repo is English (code, docs, commits, PRs, UI). Plain, specific, no marketing.
- Talk to the customer in German, informal "du".
- **Final reply to the customer: exactly three short lines.**
  1. 🟢/🟡/🔴 what changed and its status. 🟢 only with the exact check that was run; 🟡 not
     tested; 🔴 blocked or a guess.
  2. What the customer has to do ("nichts" if nothing).
  3. What is still left to do ("nichts" if nothing).
  Details go into PR descriptions, specs and plans, not into chat.
- **Questions** only when a decision is the customer's (§3). Format: own block
  `Frage <n> (<KURZ-ID>): Titel`, options A/B/C, recommendation, default if unanswered.

## 2. Roles

| Who | Does | Does not |
|---|---|---|
| Customer | goals, scope, accepts story PRs, decides `needs-customer` ADRs | code, tickets |
| Lead (main session) | plans, delegates, integrates, opens PRs, reports | product code beyond glue |
| `product-owner` | `specification.md`, backlog, criteria | `src/` |
| `architect` | ADRs, `adrs.md`, crate layout, guards §5 | features |
| `implementer` | `plan.md`, builds one story test-first | scope, criteria |
| `ux-engineer` | design system, UX notes, UI review | domain logic |
| `tester` | acceptance tests from the spec, full gate, PASS/FAIL | production code |

Agent definitions: `.claude/agents/`. Subagents report in at most ~30 lines plus file paths,
never spawn their own subagents, and never open PRs (the lead does).

## 3. Decide or ask

Decide without asking: anything inside an accepted story and reversible in one PR; naming,
module structure, tests, behaviour-preserving refactors; a permissive dependency that passes
`cargo deny` (justify it in the PR). Mark own choices **(Proposal)**, vetoable at the demo.

Ask the customer: new features or scope changes; ADRs on platform, UI framework, document
model, persistence/sync, plugins, licence, accounts/payment; anything with accounts, paid
content, telemetry or data leaving the machine; removing or changing accepted behaviour; the
one-line meaning of a new operation before it is built. Batch questions; never block on a
question with a safe default.

No self-made hard limits the customer did not set (e.g. wasm size): report and track them in
`docs/technical-debt.md` instead.

## 4. Workflow

One folder per feature, `specs/<NNNN-slug>/` (convention: `specs/README.md`):
`specification.md` (PO), `adrs.md` (architect), `ux-notes.md` (UX, if UI), `plan.md`
(implementer, tasks name their criteria).

1. **Ready**: spec, adrs and UX notes exist; open questions have defaults.
2. **Build**: `plan.md` first, then branch `story/<slug>`, tests first for core logic.
3. **Verify**: tester writes acceptance tests from the spec before reading the code, runs the
   full gate; UX review for UI; architect review for crates, public APIs, dependencies, format.
4. **PR**: one PR per slice, opened only when the customer can try it end to end. Invisible
   work (model, kernel, plumbing) is commits on the same branch, never its own PR.
5. **Done**: customer accepts; rebase-merge; spec gets `Status: Done (PR #n)`.

**PR description** (every PR): "What changes in the demo" first (plain list; "Nothing changes in
the demo." if so), then "Review guide": the 3-8 central Rust files and what to look for, every
dependency change with licence and reason, every docs/spec/ADR change, the `format_version`
change, how to try it. The customer reads the central `.rs`, `Cargo.toml`/`Cargo.lock` and
docs, not the TypeScript.

**Definition of Done:** every criterion covered by a test, gate green, docs updated, no new
`TODO` without an issue link, customer accepted.

**One checkout, one branch at a time** (`~/<project>/` on VMs, the clone itself; no worktrees,
nothing outside the project directory). Agents in the same checkout run one at a time when
they build or test; only docs-only agents run in parallel. Only one agent drives a browser.

**Stacked PRs:** a PR is always opened against `main`. A branch stacked on an unmerged one waits
until its base is merged, is rebased onto `main`, then gets its PR.

## 4a. Tokens: model, machine, gate

- **Model per task** (role default in `.claude/agents/`, the lead overrides per dispatch):
  Haiku for mechanical work (clean rebases, dependency/advisory bumps, status rows, lint fixes
  in tests, watching CI); Sonnet for tester, UX notes, spec drafts, browser checks, frontend
  wiring after finished UX notes; Opus for lead, architect and core logic (document model,
  format, geometry, text engine, sync), and for the tester on format or document-model changes.
  **Escalate** to Fable only after two failed attempts on the same problem, with the failure
  handed over; never as a default.
- **Tester on its own VM** (`navi-curvyo`, plain clone `~/curvyo`): the lead sends the branch
  name; the tester pulls it, commits acceptance tests to the branch, pushes, and reports
  PASS/FAIL. The lead's checkout stays free; never two agents building in one checkout.
- **Gate once:** during work only the affected crates; the full gate (§7) once before the push.
- **Verify by risk:** full tester pass for format, document model and public APIs; for
  `chore/` and `fix/` the gate plus one focused test.
- **Short briefs and reports:** give agents the paths and criteria they need, not the history;
  reports stay under ~30 lines.

## 5. Engineering rules

- No trait without a second implementation (or a used test double), no generic without two
  concrete types, no new crate without an ADR, no speculative flags or options.
- One responsibility per module; its doc comment says it in one sentence without "and".
  Functions over ~60 lines or modules over ~500 lines are split or justified in the PR.
- `*-core` crates never depend on UI, platform or I/O crates; they use `#![forbid(unsafe_code)]`;
  `unsafe` elsewhere needs an ADR.
- Errors: `thiserror` in libraries, `anyhow` only in binaries. No `unwrap`/`expect` outside
  tests unless preceded by `// invariant: ...`.
- Units are newtypes, never bare `f64`; every geometric comparison has an explicit tolerance.
- Delete dead code; no commented-out code.
- Every file-format importer/exporter has golden-file tests in `tests/fixtures/`.
- Timing tests measure thread CPU time (e.g. `curvyo-editor-wasm/tests/cpu_time/`), never wall time.
- Lints are enforced by the workspace `Cargo.toml` (`clippy::pedantic`, `unwrap_used` deny,
  `missing_docs` warn).

## 6. Architecture

- Core/platform split per `docs/guides/rust-workspace-blueprint.md` §1: domain logic lives in
  `*-core` crates with no filesystem, network, clock, threads or UI; callers pass everything in.
- Every `*-core` crate builds for `wasm32-unknown-unknown` (checked in CI).
- Targets: desktop → browser → mobile; mobile gets no code until a story needs it.
- Machines: job and output formats in core, device communication in `-io` crates; one machine
  family at a time, no universal machine abstraction before the second family.
- Crate suffixes `-core`, `-app`, `-io`, plus `-wasm` (ADR 0001) and `-server` (ADR 0004).
  Layout: ADR 0011. ADR index: `docs/adr/index.md`. ADR format, requirements split and CI
  structure: `docs/guides/rust-workspace-blueprint.md`. Requirements: `docs/requirements.md`;
  UI rules: `docs/design-system.md`; known debts: `docs/technical-debt.md`.

## 7. Quality gate

Run locally before every push; push and open the PR only when green.

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --message-format=short -- -D warnings
cargo nextest run --workspace --status-level fail
cargo build -q --target wasm32-unknown-unknown -p <each core crate and curvyo-editor-wasm>
cargo clippy --target wasm32-unknown-unknown -p <the same> -- -D warnings
cargo deny check
RUSTDOCFLAGS="-D warnings" cargo doc -q --workspace --no-deps
cd frontend && npm run build:wasm && npx tsc --noEmit && npm test && npx oxlint
```

During work run only the affected crates; the full gate once at the end. A failing hook means
fix it, never work around it. CI also reports the editor wasm size (`metrics/wasm-size`).

## 8. Git and GitHub

- Trunk-based; `main` always releasable. Never push to `main`, never force-push shared branches.
- Branches `story/<slug>`, `fix/<slug>`, `chore/<slug>`; Conventional Commits, scope = crate.
- 1-5 self-contained commits per PR; rebase-merge (`docs/guides/git-workflow.md`).
- `chore/` and `fix/` PRs merge without asking once CI is green; `story/` PRs need customer
  acceptance every time.
- CI: host gate on ubuntu plus wasm32 builds on every PR; Windows and macOS nightly only.
- Renovate updates dependencies; the lead batches them into one weekly PR.
- Code navigation: ProjectAtlas MCP (`projectatlas init` once, `scan` after a pull or switch;
  see `.claude/agents/implementer.md`).
