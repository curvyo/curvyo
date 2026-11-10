# Plan for Laser fixtures (0102), Part A

Branch: `story/0102-laser-fixtures`. Scope of this run: Part A only (no editor-wasm session work,
no Part B). Frage 171 to 175 at their defaults (A). Formats: `format_version` 40 is decided in
ADR 0049 but not yet written in code (task 2 below).

## Affected crates/modules

- `curvyo-laser-core`: new modules `fixture_kind.rs` (kinds, slot shapes, the stored suffix),
  `fixture_fit.rs` (fit rule), `fixture_snap.rs` (snap points of the fixtures and the grid
  lattice), `grid_layout.rs` (major lines and tapes from origin corner and offset).
- `curvyo-ui-core`: `history_names.rs` and `step_scope.rs` (new labels, ADR 0049 §9, adrs.md §4).
- `curvyo-document-core`: `document.rs` (`CURRENT_FORMAT_VERSION`), `manufacturing_fixtures.rs`,
  goldens in `tests/fixtures/`. Not started in this run.
- `curvyo-laser-core` profile record (`machine_profile_file.rs`, `machine_profile.rs`,
  `machine_profile_copy.rs`): record format 2. Not started in this run.

## Tasks

- [x] 1. Laser-core fixture model: kinds (`engraved_grid`, `holder`, `clamp`, `stop`, unknown
  kept as `Unknown`), slot shapes (rectangle, circle), typed parameter ranges with clamping
  (fulfils AC 2, 15, 19, 20, 22, 23; damage reads as default is document-core's, AC 33).
  Tests first.
- [x] 2. Fit rule `fit_slot(item, slots, tolerance)`: within 0.1 mm (customer, 2026-11-10), unturned before turned,
  a square item in a square slot is never turned (fulfils AC 25, 26; spec change 6).
- [x] 3. Snap points: slot centre, corners, edge midpoints; board corners and midpoints; stop
  outer edge ends and corner; clamp none; each through the matrix (fulfils AC 27, 24 point set).
  Grid lattice nearest intersection by rounding per axis (AC 27a, adrs §5).
- [x] 4. Grid layout: minor and major lines from the origin corner plus the offset, every n-th
  line major; tapes on chosen edges with 5 mm ticks, 10 mm long ticks, numbers every 50 mm
  counted from the grid origin (fulfils AC 15, 16, 17; Frage 174 B and C; test values of AC 16 and 17).
- [x] 5. ui-core labels: `add_grid`, `add_holder`, `add_clamp`, `add_stop`, `rename_fixture`,
  `set_fixture_active`, `set_fixture_inactive`, `move_fixture`, `rotate_fixture`, `add_slot`,
  `remove_slot`, `set_slot`, `move_to_slot` (MOVE), with names; classified once each (fulfils
  adrs §4 table). The old `add_fixture` label stays until document-core stops emitting it (task 6).
- [x] 6. document-core: `format_version` 40, fixture nodes, `active`, `fixture_matrix`, slot child
  nodes, commands, limits, unknown kinds kept, damage reads, goldens `fixtures_v40`,
  `fixtures_unknown_kind_v40`, `fixtures_damaged_v40`, `fixtures_slot_orphan_v40`,
  `fixtures_grid_v39`; the superseded format pins of tester tests updated (AC 2 to 6, 14, 19, 20,
  31 to 34). Modules `manufacturing_fixtures.rs` (reads), `manufacturing_fixture_commands.rs`
  (commands); `manufacturing.rs` shrinks. laser-core `fixture_stored.rs` reads the values typed
  with damage counts, and `grid_spec` takes the 0090 `grid_origin` corner (Frage 174 B).
- [x] 7. Profile record format 2 with `fixtures` (ADR 0050; AC 28 to 30) and its goldens; Pick and
  Reset replace the page's fixtures in one step (Generic laser removes them); Save as and Update
  store the page's fixtures.

## Parts B and C (editor-wasm session, view JSON, budgets)

Affected: `curvyo-editor-wasm/src/session/fixtures/` (new, one `FixtureState` field on
`Session`), `session/manufacturing/` (fixture glue moves out), `session/snapping.rs` (fixture
targets), `session/select.rs` (fit during a move), `session/row_drag.rs` (fit of a placed page
from the page list), `keys.rs` (Delete, arrows, Escape on a fixture), `wasm_fixtures.rs` (new
bridge); `curvyo-render-core` `fixture_look.rs` (draw code); `curvyo-document-core`
`snapping.rs` (`snapping_fixtures`); `curvyo-ui-core` `snapping.rs` (`TargetKind::Fixture`).

- [x] 8. `session/fixtures/`: state, typed read of the page's fixtures, selection transitions
  (object selection, page switch, Escape clear it) (AC 8, 10, 11, 12).
- [x] 9. Commands: four add buttons with the Proposal sizes centred, one grid per page, rename,
  armed delete, Active, eye, grid/holder/clamp/stop parameters with clamping notices, slot add
  (50 max) / remove / edit with the outside notice, typed X, Y and angle (AC 1 to 6, 13 to 15,
  18 to 20, 22, 23).
- [x] 10. Hit on the outline of an Active, shown fixture of the active laser page, slot outline
  selects its holder, inside never; move and rotate drags (22.5 degree snap with Ctrl), arrow
  nudges 1/10 mm, Delete arms then removes, Delete on a grid gives the notice (AC 8, 9, 11, 13, 14).
- [x] 11. `snapping_fixtures` register and switch (one undo step); fixture snap targets (slot,
  board, stop points; grid lines) built once at drag start (AC 24, 27).
- [x] 12. Fit during a Select move and a page-row drag of a placed page: 0.1 mm, centre to centre,
  turn by 90 degrees, one step "Move to slot", the fit cue in the drag answer (AC 25, 26).
- [x] 13. "Differs from profile" over the set, "Fixtures in profile: N", a new laser page starts
  with the profile's set (AC 28, 29).
- [x] 14. Drawing (render-core `fixture_look`): grid minor and major lines with fade, tapes on
  the chosen edges with ticks, holder board with slots (dashed outside), clamp hatch in ink, stop
  bar, Active off and eye off not drawn, selection box with the rotate glyph, slot cue; tape
  numbers and names as labels JSON at 1-2-5 steps (AC 5, 6, 7, 16 to 18, 20 to 23).
- [x] 15. Delete of a laser page removes its fixtures; undo restores them (AC 4, 36).
- [x] 16. View JSON: one serde type `FixturesView`, shared fixtures in
  `tests/fixtures/fixtures_view/`, Rust contract test (AC 1, 32, 33; adrs §7).
- [x] 17. wasm entry points in `wasm_fixtures.rs`.
- [x] 18. Budgets in thread CPU time, `tests/fixtures_budget.rs`, in the CI performance job
  (AC 35 a to d).

Notes on Parts B and C (Proposal unless said otherwise, for the lead):

- The grid's only outline is the work-area edge, where the page-edge drag of `0098` already takes
  the press; the page drag wins, so the grid is selected by its row (criterion 10's routes).
- The Layers tree's fixture child rows (criterion 12) are built in Part D: `RowKind::Fixture` under
  the Fixtures row; the press selects the fixture, the eye shows or hides it.
- Grid snapping uses the grid's lines as line targets (an intersection when both axes capture)
  instead of the laser-core `Lattice`; 2,002 lines at 1 mm stay within budget (35 a).
- A fit needs only the Fixtures switch, not the master snapping switch; a holder turned by an
  angle that is not a multiple of 90 degrees offers no fit; a page row dropped into a slot is one
  step "Place page" (a creation), a Select move into a slot one step "Move to slot".
- Arrow nudges of a fixture are one step each (no `0044` run coalescing).
- `curvyo_ui_core::fixture_drawing`, laser-core `engraved_grid` and extension-core `FixtureDrawing`
  are removed in Part D (lead sign-off, 2026-11-10).

## Part D (frontend and Layers, 2026-11-10)

- [x] D1. Decoder `frontend/src/lib/fixturesView.ts` (checked decode, reads the Rust fixtures),
  test `tests/fixturesView.test.ts` (AC 1, 32, 33).
- [x] D2. Row logic `frontend/src/lib/fixtureRows.ts`: add kinds, names, second line, slot count,
  fit words, key actions, tab stop, fit cue decode; tests `tests/fixtureRows.test.ts`.
- [x] D3. Hook `frontend/src/hooks/useFixtures.ts` (not in `useEditorSession`).
- [x] D4. Fixtures block `FixtureList` (`components/workbench/FixturesBlock.tsx`) in the Document
  tab: add group, rows with Active, eye, armed trash, rename by F2 and double-click, typed fields,
  slots, tapes, roving focus, Enter or Delete on the armed row removes (AC 1 to 6, 13 to 15, 18 to 20).
- [x] D5. Layers child rows (criterion 12): ui-core `RowKind::Fixture`, editor-wasm
  `fixture_tree`, Layers JSON `fixture`, frontend press and eye; tests in ui-core, editor-wasm,
  `tests/layerRows.test.ts`.
- [x] D6. Snap switch "Fixtures" in the Snapping pane on laser pages, one undo step (AC 24).
- [x] D7. "Fixtures in profile: N" row in the Machine block; "Differs from profile" already
  reads the fixture set (AC 28, 29).
- [x] D8. Fit cue in the move's drop badge (`fixture_fit_cue_json`, `fitWord`): word and turned
  (AC 25, 26). Canvas render of the highlight is Rust's (`fixture_look`).
- [x] D9. Dead code: `curvyo_ui_core::fixture_drawing`, laser-core `engraved_grid`,
  extension-core `FixtureDrawing` (CLAUDE.md section 5).
- Not done: the Chromium check (UX review); the keyboard routes of the Left and Right inside a row
  (Tab moves through its controls); the "Save as profile" and "Reset from profile" rows are from
  0026 and unchanged.

Validation: session tests in `session/fixtures/tests*.rs`; the contract test
`tests/fixtures_view_contract.rs`; the budget test; fmt, clippy `-D warnings`, nextest for
editor-wasm, render-core, ui-core, document-core; wasm32 build and clippy.

## Validation

- Unit tests in each new laser-core module (tolerances passed as arguments, no I/O).
- Tests for the exact numbers of AC 16 (61/41 lines, 13/9 major) and AC 17 (121 ticks, 61 long,
  13 numbers) on the 600 x 400 mm work area.
- ui-core guard `every_command_label_is_classified_once` covers the new labels.
- Gate for this run: `cargo fmt --all --check`; clippy `-D warnings` for laser-core, document-core,
  ui-core; `cargo nextest run` for the three crates; wasm32 build and clippy for laser-core,
  document-core, ui-core and curvyo-editor-wasm.

## Revision 2 (review of PR #313, 2026-11-10): rectangle handles, no size fields, screw

Affected: laser-core `fixture_kind.rs` (`Screw`, ranges, `stop_legs`, `screw_diameters`),
`fixture_stored.rs` (`screw_spec`), `machine_profile_fixtures.rs` (rigid matrix); document-core
`affine.rs` (`is_rigid_within`), `manufacturing_fixtures.rs` (rigid read),
`manufacturing_fixture_commands.rs` (`add_screw`, `resize_fixture`, thread clamp); ui-core
visibility (`EntryField::for_param`, `format_mm`, `CENTRE_YIELD_PX`, `local_delta_for_size`) and
`MoveEntry::for_box`; render-core `ScrewLook`; editor-wasm `session/fixtures/` (new `handles.rs`,
`edit.rs`, `entry.rs`, `chips.rs`); frontend chips, block and decoder.

- [x] R1. Kind `screw` with `thread_diameter_mm` and `head_diameter_mm` (0.5 to 1,000 mm, 6 and
  10 mm), thread never above the head in laser-core and in the document-core commands (AC 2, 37,
  39). Goldens regenerated: `fixtures_v40` (one of each of the five kinds), `fixtures_damaged_v40`
  (a text thread, a scaled matrix); profile goldens `format2_fixtures` (a screw),
  `format2_damaged_screw`, `format2_damaged_matrix` (AC 30, 31, 33).
- [x] R2. Handles of the selected fixture from the rectangle's box layer (`transform_handles`,
  `hit_transform_handle`, `OrientedBox`): holder and clamp 8 squircles, 4 rotate glyphs, centre;
  stop leg squircles, thickness knob on the top edge (outer edges move, inner corner fixed), rotate, centre; screw head and thread
  knobs, centre; no skew, no corner knobs. Handles hit before the outline (AC 8, 14, 18, 22, 23,
  38).
- [x] R3. One resolver `edit.rs` for drags and chips; one step per release or Enter
  (`move_fixture`, `rotate_fixture`, `resize_fixture`); an N or W resize rewrites the slot centres
  in the same step (AC 14, 18, adrs §6a.5).
- [x] R4. Chips: double-click on a handle (its chip), M (Move chip, no Copy), R (Angle, not on a
  screw), S (W and H; Leg 1, Leg 2 and Thickness; Head and Thread) through the rectangle's
  `TransformEntryChip` and `MoveEntryChip`; a typed value past its limit is set to it with the
  Action notice (AC 14, 18, 22, 23, 38; D24, D27).
- [x] R5. View: `selected_pose`, `size` and `stop` removed; `selection_announcement` added;
  `FixtureSelectBar.tsx` deleted; the Select bar shows nothing for a fixture; no block field for
  holder, clamp, stop or screw; five add buttons, 44 px; the screw's symbol and its two rings on
  the canvas (AC 1, 2, 14, 37, 38; UX 5.5, 6.4, 11).
- [x] R6. Budget (e) of adrs §8: a typed resize of a holder with 50 slots commits within 16 ms.
- [x] R7. The step labels of adrs §4: one per grid field (`set_grid_spacing`, `set_grid_major`,
  `set_tape_edge`, `set_grid_origin`), per slot field (`rename_slot`, `set_slot_shape`,
  `set_slot_position`, `set_slot_size`) and the snap switch (`set_fixtures_snap_on/off`).
- [x] R8. A screw diameter that does not read (text, out of range) reads as its default and is
  counted in document-core too (AC 33, tester FAIL of the revision 2 tests).
- [x] R9. The stop's thickness follows criterion 23 (lead, 2026-11-11): the knob on the top edge
  of leg 1, the outer edges move, the inner corner stays. A typed value at its limit shows the
  Action notice at the chip's handle (the rectangle's limit readout); a screw knob drag shows
  "⌀ 6.0 mm", "⌀ 10.0 mm max" or "⌀ 6.0 mm min" at the pointer.
- [x] R10. UX review of the rework: the rectangle's cursors over fixture handles and the box; the
  live readouts ("W × H mm", the angle, "Δ x, y mm", "t ... mm") and the drag ghost of the
  start outline; an edge squircle opens its one axis; the Angle chip's prefill without "°"; a fit
  exactly at 0.1 mm holds (explicit epsilon); one notice names both clamped sizes.
- Not done in revision 2: the reference marks of the Move chip on a fixture; the Chromium check.
