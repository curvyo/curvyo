# Plan for Object history (0041)

One branch `story/object-history`, one PR, opened when the maker can try the slice end to end in
the app (Ctrl+U and Ctrl+Shift+U on one selected object, the Object scope of the History tab, a
clone that keeps its history, the wipe of one object's history) and the full local gate is green
(`CLAUDE.md` §7, the frontend checks, the boolean budgets and the history budget tests). The
milestones are commits on the branch; none is a PR. Criteria 18 and 27 (groups) wait for `0023`
and are written down in `docs/technical-debt.md`.

## Affected crates/modules

- `curvyo-document-core`: `history/object_index.rs` (object to steps index, the object stack
  machine, targets), `history/object_rows.rs` (the object timeline, its rows and statuses),
  `history/lineage.rs` (clone origin codec, inherited rows, floor), `history/object_restore.rs`
  (object undo and redo, restore into inherited rows), `history/object_wipe.rs`; changes in
  `history/restore.rs` (scope), `history/restore_plan.rs`, `history/step_index.rs` (marks,
  positions, shown-id cache), `history/commit.rs` (own step registration), `history/rows.rs`
  (revert marks, "undone by"), `objects.rs` (Duplicate writes `clone_of`, drops the source's
  `clone_of` and `history_floor`), `path_model.rs` (`NodeId::short_id`). No new crate, no
  dependency, no `format_version` change (the optional keys `clone_of` and `history_floor` are
  additive, ADR 0014 Q2 A).
- `curvyo-ui-core`: `history_names.rs` (reserved labels), `history_notice.rs` (object notices and
  row texts).
- `curvyo-editor-wasm`: `session/object_history.rs` (object undo, redo, wipe, rows, info),
  `session/history.rs` and `session/keys.rs` (Ctrl+U, Ctrl+Shift+U in the one gate),
  `wasm_history.rs`, `wasm_keys.rs`.
- `curvyo-app`: `menu.rs` (the two Edit items).
- `frontend`: `lib/historyText.ts`, `lib/historyRows.ts`, `hooks/useHistory.ts`,
  `hooks/useHistoryList.ts`, `components/panel/HistoryTab.tsx` (scope group, Object list, buttons,
  wipe footer), `components/panel/PropertiesPanel.tsx` (subject line).
- Docs: `docs/technical-debt.md` (groups 18 and 27, inherited anchors), the
  `specification.md` and `adrs.md` of this feature (amendment notes), `docs/adr/0014` (amendments
  only where the build differs).

## Design notes (deviations from `adrs.md` are listed at the end)

- A timeline is derived: `ObjectIndex` maps a `TreeID` to the steps whose operations touched it
  (one pass over the JSON updates, extended per commit or import). Rows are built per visible
  page, with the touched set of that page's steps only.
- Which step Ctrl+U takes back and what Ctrl+Shift+U brings back come from the pure
  `StackMachine` run over the object's entries, oldest first: an ordinary step is `Do`, an object
  undo mark that is in effect (its own status is Applied) is `Undo(target)`, an object redo mark
  is `Redo(target)`. Nothing else is special-cased: Ctrl+Z on an object undo is the plain undo of
  that commit.
- Object undo and redo use the engine of `0020` with a scope: the step's touched set is cut to one
  object for a Change, Create or Delete step; a Replace step is restored whole. The commit is a new
  step with the header `object_undo;s;t;ou=<step>;o=<object>` (redo: `object_redo`, `od`).
- Floor: the wipe step writes `history_floor` on the object (its one operation) and the timeline
  derives the floor from the wipe step's start version, which is the same value. Reading it from
  the wipe steps lets a clone made earlier keep the floor as it was at its duplication.
- Clone origin: `clone_of = "<peer>:<counter>@<frontiers>"` (the existing frontier encoding of
  `history_wiped`, `peer@counter` pairs).
- Object undo into inherited rows restores the source's plain registers (frame, style, rotation)
  at the inherited step's before-version into the clone; anchor lists are not restored (a copy
  has its own anchor ids).

## Tasks

### Milestone 1: object index, timeline rows, Object scope list (AC 1 to 4, 21, 22, 35, 36)

- [x] 1.1 `NodeId::short_id` and `full_id`; step index keeps positions, marks (`ou`, `od`, `o`
  from headers) and a cached shown-id map (AC 2, 3, 36)
- [x] 1.2 `ObjectIndex` and its extension per commit and import; a step that touched N objects is
  in N lists; deleted objects resolve (AC 21, 22, 36)
- [x] 1.3 Object timeline entries and rows: kind, role, other-object count, sources of a created
  object, statuses including "Undone by"; paged (AC 1, 3, 19, 20, 21, 28)
- [x] 1.4 Document list rows carry the reverted step of an object undo or redo and "Undone by"
  (AC 8)
- [x] 1.5 Budget test, release: 1,000 steps of an object in 20,000 steps (AC 35)

### Milestone 2: object undo and redo (AC 5 to 17, 40, 41)

- [x] 2.1 Restore scope in the engine; `object_undo` and `object_redo` headers; own step
  registration; field counts kept later (AC 6, 7, 8, 12, 40)
- [x] 2.2 Object stack machine and targets: latest applied, redoable, cleared by a new step
  (AC 9, 10, 11, 13, 14, 41)
- [x] 2.3 `ui-core` notices and names (AC 6, 7, 12, 13, 14, 15)
- [x] 2.4 Session: `object_undo`, `object_redo`, gate, tools, selection, off-screen, records the
  step on the undo stack; Ctrl+U and Ctrl+Shift+U in the key table; busy state (AC 5, 9, 10, 15,
  16, 17)
- [x] 2.5 wasm API, frontend keys, canvas notice, Edit menu items (AC 5, 16)
- [x] 2.6 Budget tests for object undo, a step 1,000 steps back (AC 17)

### Milestone 3: clone origin and inherited rows (AC 19, 20, 23 to 28 without groups, 37, 39)

- [x] 3.1 `clone_of` codec; Duplicate writes it in the copy commit and drops `clone_of` and
  `history_floor` of the source (AC 23, 37, 39)
- [x] 3.2 Inherited rows from the source's steps in the causal past of the version, recursive,
  with the floor at that version (AC 24, 26, 33)
- [x] 3.3 Object undo into inherited rows as a restore of plain registers (AC 25)
- [x] 3.4 Created-from rows for Boolean, Combine, Break apart, Offset: not clones (AC 19, 20, 28)

### Milestone 4: wipe of one object (AC 29 to 34, 38)

- [x] 4.1 `Document::wipe_object_history`: the step, the `history_floor` register, floor in the
  timeline, undo stack entries that touched only this object are dropped (AC 30, 31, 32, 33, 34,
  38)
- [x] 4.2 Session and wasm: availability, blocked states, notice (AC 29, 34)

### Milestone 5: the Object scope in the app (AC 1, 2, 16, 29)

- [x] 5.1 Scope group, subject line, Object list rows (clone start, inherited, floor), buttons with
  tooltips, wipe footer with the inline confirm (AC 1, 2, 3, 16, 29)
- [x] 5.2 Docs: technical debt entries for groups, spec and ADR notes; PR description draft

## Validation

- Tests first for the core logic in `curvyo-document-core/tests/` (`object_index_timeline.rs`,
  `object_undo_redo.rs`, `object_clone_lineage.rs`, `object_wipe.rs`) and for the session in
  `curvyo-editor-wasm/tests/` (`object_history_session.rs`). Two-replica tests for another
  author's step and a concurrent wipe.
- `document.json` equality at each point of the tests of criteria 9 and 10.
- Release `#[ignore]` budget tests for criteria 17 and 35 (`cargo test --release -- --ignored`).
- Loro canary (`loro_revival_canary`) stays green: object undo of a creation uses the same
  injected move.
- Full gate: fmt, clippy, nextest, wasm32 builds of every touched core crate, deny, doc, the
  frontend `tsc`, `oxlint` and tests, the boolean budgets.

## Deviations and open points (written at the end of the slice)

- Criteria 18 and 27 (groups) wait for `0023`: `docs/technical-debt.md`.
- Version encoding, floor read from the wipe step, the lineage machine, redo of inherited steps and
  the plan's reading of revived nodes: the build notes in `adrs.md`.
- Two tester tests of `0010` (`acceptance_edit_polish_pr4_tester.rs`) compared a copy's meta keys
  with its original's; they now allow exactly the key `clone_of` that criterion 37 adds. Two
  mechanical type changes in tests: `history_target_name` returns a `String`, and the report types
  gained fields (`kept`, `step`).
- Object undo into inherited rows restores plain registers only; an object undo whose parts were
  all skipped stays the latest step (technical debt).
- Ctrl+Shift+U on WebKitGTK (spec Question 5) is a check by hand in the app; the Edit menu items are
  the fallback and are in.
- No browser run in this slice: the UX review of the History tab comes with the tester pass.
