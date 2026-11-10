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
- [ ] 8. Later Part (session): `session/fixtures/`, the Fixtures block, hit, snap, fit, drawing
  with major lines and tapes, `snapping_fixtures`, "Differs" over the set, a new page starting with
  the profile's set.

## Validation

- Unit tests in each new laser-core module (tolerances passed as arguments, no I/O).
- Tests for the exact numbers of AC 16 (61/41 lines, 13/9 major) and AC 17 (121 ticks, 61 long,
  13 numbers) on the 600 x 400 mm work area.
- ui-core guard `every_command_label_is_classified_once` covers the new labels.
- Gate for this run: `cargo fmt --all --check`; clippy `-D warnings` for laser-core, document-core,
  ui-core; `cargo nextest run` for the three crates; wasm32 build and clippy for laser-core,
  document-core, ui-core and curvyo-editor-wasm.
