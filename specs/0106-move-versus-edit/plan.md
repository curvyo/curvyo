# Plan for move versus edit

Implementer, 2026-11-09. Branch `story/0106-move-versus-edit`. `adrs.md` decisions 1 to 9;
Fragen 164, 165 and 166 built at their defaults A. No format change (38), no new dependency.

## Affected crates/modules

- **`curvyo-ui-core`**: `edit_context.rs` (new: `ContextKind`, `Home`, `home_of`, `in_context`,
  `scope`, `double_click_target`), `step_scope.rs` (new: `MOVE_STEPS`, `FREE_STEPS`, `step_scope`
  and the guard test against `history_names::OPERATION_NAMES`), `edit_context_text.rs` (new: the
  Style line and the refusal notice with the 70 and 24 character cuts), `page_membership.rs`
  (placements by ownership), `pages.rs` (`PageViewState::press` gets `hit`), `layer_rows.rs`
  (`RowKind::{PageRoot, PlacedPage, Fixtures}`, `LayerRow::via`, `read_only`), `page_tree_rows.rs`
  (new: the laser page tree builder).
- **`curvyo-editor-wasm`**: `session/edit_context.rs` (new: `EditContext`, the cached scope, the
  last active drawing page, `recording`'s guard, `edit_targets`, the double-click and Enter
  switch, the notice), `history.rs` (`recording` / `recording_unguarded` split and the
  `record_new_steps` debug net), `pages.rs` (the press path of criterion 2), the call sites of
  move and free commands (`recording_unguarded`), `context.rs` (`edit_targets`), the Node, Text and
  Pen tool reads, `nudge.rs` (Select all in context), the sketch workbench's press, move and keys
  for drawing objects, `layers_panel.rs` and `page_tree_panel.rs` (new: the laser tree's JSON and
  presses), the Style and Select bar views (`scope`, `outOfContextLine`), CPU-time tests.
- **`frontend`**: Style tab line and hidden sections, Select bar hidden edit buttons, the laser
  Layers tree rows (root, placed page, inner, Fixtures), the context notice.

## Tasks

- [x] 1. Plan (this file).
- [x] 2. ui-core `edit_context.rs`: home, in-context table, scope, double-click target; table
  test of criterion 1's project and of the targets (AC 1, 4, 13 to 16).
- [x] 3. ui-core `step_scope.rs` with the guard test; `edit_context_text.rs` (AC 9, 11, 14).
- [x] 4. Wasm `session/edit_context.rs`: `EditContext`, cached `scope()`, last active drawing
  page; `recording` guarded, `recording_unguarded` for move, free and input dispatch; the debug
  net; the key notice (AC 4, 5, 6, 9, 21).
- [x] 5. Wasm press path: a press on an out-of-context object selects and keeps the page; an
  empty press activates as built (AC 2); Select all in context; Shift and marquee context-blind
  (AC 3).
- [x] 6. Wasm edit tools: Node, Text, Pen and Offset read `edit_targets()`; the Select tool shows
  no parameter handles on a move-only selection (AC 10).
- [x] 7. Wasm double-click and Enter: activate the home page (drawing, last drawing page,
  laser, placed page source with the object under the pointer), notice without a drawing page
  (AC 13 to 16).
- [x] 8. Wasm sketch page active: a press off sketch geometry selects a drawing object move-only;
  handles, drag, arrows, typed values, align and flip for it; never mixed with sketch geometry
  (AC 3, 7, 8).
- [x] 9. Wasm Style and Select bar views: `scope`, the muted line, the move-only bar (AC 11, 12).
- [x] 10. ui-core `page_tree_rows.rs` and wasm `page_tree_panel.rs`: the laser page tree (AC 17
  to 20).
- [x] 11. CPU-time tests (AC 22); budget test of the tree (AC 20).
- [x] 12. Frontend: Style line, hidden edit sections and buttons, tree rows, notice (AC 11, 17 to
  19).
- [x] 13. Superseded tester tests adjusted; full gate; Chromium check (AC 21, 23).

## Validation

Unit tests in `curvyo-ui-core` for the table, the step classification and the tree builder,
written before the code; session tests in `curvyo-editor-wasm` for presses, double-clicks, keys,
the guard (byte-identical document, no history step) and the sketch page; CPU-time tests under
`curvyo-editor-wasm/tests/cpu_time/`; the `bring_content_in_v38` golden keeps passing; a Chromium
pass with screenshots.

## Notes for the lead

- **(Proposal) Guard by call site, as `adrs.md` decision 4 says.** `recording` is guarded and
  returns `R::default()` (so 19 outcome types gained `Default`, each its "ignored" variant);
  move, free and dispatcher entries call `recording_unguarded`. The debug net in
  `recording_unguarded` asserts no Edit-labelled step is written for a move-only selection.
- **(Proposal) Panel edits** (rename, opacity, row reorder, Move up/down) go through
  `panel_edit`, which answers a move-only selection with the refusal notice; eye, lock, row
  presses, Duplicate and Delete stay `panel_action`.
- **(Proposal) Notice only from keys.** A guarded command refused during a key sets the notice
  once (`take_context_notice`); Ctrl+G with one object stays a no-op as built (Group needs two).
- **(Proposal) Sketch page.** A Sketch Select press off sketch geometry on a drawing object (or
  its box and handles) runs the vector Select tool and sets the vector tool to Select; arrows,
  Delete, Flip, the typed entry keys and Escape go to the drawing selection; Ctrl+A drops it.
  While dragging, the moved copy is drawn over the original (no hiding of the original).
- **(Proposal) Fixtures row eye** shows "shown" when every fixture of the page is visible and
  toggles them all (one step per fixture; a laser page has one by default). There is no block
  eye in the Document tab to mirror.
- **(Proposal) Second pass (coordinator, 2026-11-09):** the rail's Grouping, Path and Boolean
  cards and the Layers Opacity field are hidden for a move-only selection (opacity is an edit,
  criterion 9); the laser tree's root row is pinned, with muted "Empty" and "Nothing on this page
  yet." rows; out-of-context rows on a drawing page get the tooltip line "On Laser 1"; the Style
  tab shows the reduced clone block (name, Select original, Select root original; the block has
  no Select clones button today); home page labels are semibold; the status bar adds "Rectangle ·
  Drawing 2" after the cursor and the announcer ", on Drawing 2" (a single object with no
  announcement of its own is announced as "Rectangle, on Drawing 2").
- **(Proposal) Tester findings (2026-11-09):** with the Select tool an empty press activates
  its page on the release of a click (≤4 px), so neither a move nor a marquee switches the page;
  a press on the selection's box or handles counts as a hit; a click that never left the dead
  zone is no cloning drop (a Shift-click onto a laser item toggles it). `ux-notes.md`: Ctrl+Alt+C
  dropped from the hidden chords.
- **(Proposal) UX review (2026-11-09):** the Node tool shows the Select box and handles on a
  move-only selection; its press on a foreign object selects it, but its drag writes nothing (the
  tester's ac9 keeps the Node tool free of moves). Laser tree rows are named as UX notes §5, a
  placed page row's tooltip line is "Live, read-only"; the status bar names every home; a placed
  page shows no style sections; Linked clone shows no success notice (customer rule
  2026-10-31; `acceptance_0049_tester.rs`, `acceptance_0055_v2_tester.rs` and
  `clone_tests.rs` amended).
- **Budgets** (criterion 22, 20) are asserted in release builds
  (`tests/move_versus_edit_budget.rs`, added to the `performance-budgets` CI job); in debug a
  press with 1,000 objects costs ~35 ms, the same in and out of context (existing cost).
- **Superseded tests amended:** `tests/acceptance_0059.rs` (ac12, Select all in context),
  `tests/acceptance_0093_tester.rs` (ac12, Ctrl+A on an unknown page), `tests/acceptance_0099_tester.rs`
  (ac3 press and drag move-only; ac13 clones moved to the pasteboard for Ctrl+A), `tests/pages.rs`
  (Select all after a pasteboard press), `src/session/sketch_use_tests.rs` (0099 criterion 3
  press).
