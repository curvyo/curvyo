# Plan for Undo and redo (0020)

One branch `story/undo-redo`, one PR, opened when the maker can try undo and redo end to end in the
app (Ctrl+Z, Ctrl+Shift+Z, the History tab) and the full local gate is green (`CLAUDE.md` §7, the
frontend checks, the budget tests). Milestones are commits on the branch; none is a PR.

## Affected crates/modules

- `curvyo-document-core`: new module directory `history/` (step header, step id, step log, touched
  objects, stack machine, restore engine, revival, checkout guard, wipe); `document.rs` (commit path,
  `new_document` commit, step state); `container.rs` untouched (no format change). Root
  `Cargo.toml`: `loro` pinned `=1.16.2`.
- `curvyo-ui-core`: `history_names.rs` (operation names, the guard of AC 41), `history_rows.rs`
  (summaries, notice texts), `undo_stack.rs` (selection per step), `transform_commit.rs` and
  `group_transform.rs` (one commit per gesture, labels).
- `curvyo-editor-wasm`: `session/history.rs` (undo and redo commands, busy threshold, warm_history),
  `session/keys.rs` (the one gate), `wasm_history.rs`, the entry wrapper that records steps with their
  selection.
- `curvyo-app`: Edit menu (AC 8).
- `frontend`: key handling, the canvas notice slot, Undo and Redo buttons, the History tab, the
  removal of "No undo yet." texts.
- Specs and docs: the "No undo yet." phrase in the specs listed in AC 52, `docs/technical-debt.md`,
  the amended-by lines.

## Design notes (deviations from `adrs.md` are listed in the PR description)

- Steps are delimited by the commit message header `<label>;s=<seq>;t=<sec>[;k=v]*`. A step is all
  commits of one peer with the same header.
- Undo reads the step's values before and after with a guarded checkout pair (ADR 0014 §12 guard)
  and compares them with the live values field by field (a three-way merge over Loro container
  values, `history/merge.rs`). `diff` is not used; this is an amendment of ADR 0014 §1 of 2026-10-10
  (architect review of milestone 1). The budgets are measured (ADR 0014 §13, "Measured").
- Deleted objects come back through the injected tree move of ADR 0014 §11, same header.

## Tasks

### Milestone 1: engine, keys, stacks, notices (AC 1 to 31, 41, 52, 54 to 58)

- [x] 1.1 Pin `loro = "=1.16.2"`; canary test `loro_revival_canary.rs`: the injected move revives the
  same NodeId, anchor containers, z-order, survives save and load, keeps a peer edit made while the
  node was deleted (ADR 0014 Verification 1, 2, 3) (AC 23, 59)
- [x] 1.2 Step header and step time: `commit_with_label` writes `<label>;s=<n>;t=<sec>`, `new_document`
  commit in `Document::new`, `Document::set_step_time`, `continue_step`, `batch` (AC 13, 14, 54, 57, 58)
- [x] 1.3 Commit-path audit: one test per row of the table "One interaction, one step" counting commits
  and checking the label; `batch` where a gesture has several commits; one commit for several objects
  (`transform_commit.rs`); skew gets its own label (AC 10, 41, 58)
- [x] 1.4 Step log and touched objects: parse headers, group changes into steps, derive created,
  deleted, moved and changed objects from the step's ops (AC 11, 32, 38)
- [x] 1.5 Checkout guard (`history/checkout.rs`), `warm_history` (AC 30c)
- [x] 1.6 Restore engine: undo and redo by three-way merge over before, after and now, revival,
  conflict rules, skipped counts; two-replica tests through `export_updates` and `import_updates`
  (AC 11, 15 to 21, 23)
- [x] 1.7 Stack machine, pure, with the 1,000-event equivalence test against a log-derived replay
  (AC 15, 22, 28, 55, 56)
- [x] 1.8 `ui-core`: operation names with the guard test, summaries, notice texts, selection per step
  (AC 1, 2, 4, 19, 23, 24, 41)
- [x] 1.9 `editor-wasm`: record steps with their selection per entry, `undo`/`redo` on the session,
  key rows and the gate (drag, chip, text field, busy, open Pen path), Ctrl+Y, QWERTZ, repeat, stacks
  limit, busy threshold (AC 1 to 7, 12, 22 to 28, 30)
- [x] 1.10 `wasm-bindgen` facade and frontend: key handling, canvas notice slot, Undo and Redo
  commands, removal of "No undo yet." in texts, tests and specs (AC 1, 2, 4, 24, 52)
- [x] 1.11 Edit menu in `curvyo-app` (AC 8)
- [x] 1.12 Budget tests (release, `#[ignore]`): small step 100 ms, large steps, memory of 500 steps
  (AC 29, 30)

### Milestone 2: step data (AC 32 to 38, 59, 60)

- [x] 2.1 `StepRef` and shown id (FNV-1a 64, base36, collision lengthening), pinned known value (AC 33)
- [x] 2.2 `Document::history_page`: rows with id, author, time, name label, kind, touched summary,
  status from the stack machine over the log (AC 32, 34, 35, 38, 60)
- [x] 2.3 Old files: legacy labels, unknown label shows "Edit", adjacent same-label commits as one row,
  "Unknown time"; golden fixtures (AC 36, 37, 59)
- [x] 2.4 No `format_version` change; `history_wiped` optional-key reader test (AC 37, 59)

### Milestone 3: the History tab (AC 39 to 46)

- [x] 3.1 wasm API for pages, hover link, tab state; React History tab: list, pinned Now row, rows,
  status, tooltips, keyboard, virtualisation, Undo and Redo buttons in the header, footer line,
  Shift+Ctrl+H (AC 9, 39 to 46)

### Milestone 4: wipe (AC 47 to 51)

- [x] 4.1 `Document::wipe_history`, epoch key, `import_updates` epoch and `pending` refusal (AC 48, 49)
- [x] 4.2 Inline confirm in the footer, shared-document line (AC 47, 50, 51)

## Validation

- Unit tests next to the code, tests first for the core logic; acceptance tests per criterion in
  `curvyo-document-core/tests/` and `curvyo-editor-wasm/tests/`.
- Two document replicas that merge for AC 15 to 21; a generated 1,000-event sequence for AC 55.
- Release `#[ignore]` budget tests (`cargo test --release -- --ignored`) for AC 29 and 30.
- Full gate: fmt, clippy, nextest, wasm32 builds of every touched core crate, deny, doc, and the
  frontend tsc, oxlint and tests.

## Deviations and open points (written at the end of the slice)

- `diff` replaced by the guarded checkout pair; the editor also announces the busy state for a step
  more than 1,000 operations behind the latest version (`FAR_STEP_OPS`).
- Group transforms are labelled `transform_resize`, `transform_rotate`, `transform_skew` (the
  tester's `0019` test pins the word "transform"); single-object skew is `skew_path`.
- `render-core`: `SelectDecorationInput::history_hovered` (additive) for the hover link of a row.
- The wasm32 timing factor is not measured (no browser in the gate).
- Z-order and parent moves of existing objects are not restored; reported as skipped (`adrs.md`).
- The hold-to-nudge calls `Document::continue_step()` (`curvyo-ui-core/src/nudge.rs`); a nudge
  after Ctrl+Z starts a new step.
- Not covered by the gate: the native Edit menu accelerators on each OS (AC 8, by hand).
