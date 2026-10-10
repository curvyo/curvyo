# Plan for transform polish

One PR on `story/transform-polish`, built as three milestones (customer rule of
2026-10-09: no PR until the customer can try all three parts). Each milestone ends with a
green local gate on its own; the PR is opened once, after C.

State found on `main` (966da00), which differs from the "What exists today" of the
specification: `0019` already shipped criteria 53 to 56. A stretch of a multi-selection by an
edge handle already converts, and a lone polygon or star already stretches by an edge handle
or a typed size. What is missing is the silent part (the notice, the hover line, the
readout line, the entry note, the proportional-only corners), skew for primitives, and free
corner drags. `ConversionCounts`/`ConvertingKind` exist only to feed those texts.

## Affected crates/modules

- `curvyo-ui-core`: `conversion.rs` (the predicate and the paths; the count types go),
  `group_box.rs` (`all_paths`, `converting`, `proportional_corners` go; skew handles for
  every selection), `group_transform.rs` (`apply` shears a converted primitive),
  `group_entry.rs`, `transform_entry.rs`, `skew_entry.rs`, `transform_drag.rs`,
  `transform_primitive.rs` (lone polygon/star resize), `transform_math.rs`
  (`polygon_star_resize_factor` goes), `select_tool*` (no notice state), `handles.rs`.
  C: new `reference_point.rs`; `move_entry.rs` takes the reference; `select_tool` gets the
  mark press and the reference box; `select_tool/move_drag.rs` exposes the live bounds.
- `curvyo-document-core`: none expected (the conversion commit exists:
  `Document::transform_objects` with a path result for a primitive).
- `curvyo-render-core`: C: the reference box and marks in the decoration input.
- `curvyo-editor-wasm`: `session/conversion_view.rs` and the `take_conversion_notice`,
  `*_conversion_count` exports go; `KeyHint::PathOnly` goes; group hint lines; C: the move
  entry view carries the reference, mark press, hint, box decoration.
- `frontend`: A: `conversionText.ts` and its callers go, tooltip of "Object to path"; B: one
  slider hook, four sliders, `EntryField` for S1/S2, hex focus for S3/S4; C: `MoveEntryChip`
  (Reference control, text, chip placement), marks wiring, tokens.
- `docs`: `design-system.md` rows, `specs/` status lines of the superseded criteria,
  `requirements.md`, technical debt if any.

## Tasks

### Milestone A: skew and stretch convert silently (AC 1 to 16 of Part A)

- [ ] A1. Test first: `ui-core` conversion predicate (`converts_on_stretch`, skew converts every
  primitive); `group_transform::apply` shears a converted primitive; example of AC 2 and AC 7. (AC 2, 3, 7)
- [ ] A2. `conversion.rs`: drop the count types, add `converted_for_handle` (stretch: the
  converting shapes; skew: every primitive). `group_drag`/`group_entry` build `converted` for
  Resize on any handle and Skew. (AC 2, 5, 7, 8)
- [ ] A3. `group_box.rs`/`group_map`: skew handles for every selection, corner drag free (Ctrl
  keeps proportions), no `proportional_corners`; `GroupEntry::for_skew` for every selection. (AC 1, 6, 7, 9)
- [ ] A4. Single object: skew of a primitive converts (`skewed_unchecked` takes the converted
  path, `SkewEntry` for every kind, `begin_handle_drag` builds it); lone polygon/star resize on
  every handle (corner included) converts on a stretch and scales uniformly otherwise, pivot
  at the opposite corner/edge or the centre with Shift; typed size likewise. (AC 1, 6, 9, 10, 12)
- [ ] A5. Delete the notice machinery: `SelectTool::last_conversion` and friends,
  `Session::*_conversion_count`, `take_conversion_notice`, `KeyHint::PathOnly`,
  `KeyEntryRefusal::SkewNeedsPath`, the corner "proportional" hint lines, `resize-corner-uniform`. (AC 11)
- [ ] A6. Frontend: remove `conversionText.ts`, the hint line, readout line, entry note, the
  6 s notice and the "Skew works on paths only" text; tooltip of "Object to path". (AC 11, 13)
- [ ] A7. Persistence and one-commit tests in `editor-wasm` (save/reopen, version advances by
  one commit, a replica sees old or final). (AC 4, 14)
- [ ] A8. Update the superseded tests of `0019` Part H, `0005` criterion 11, `0008` skew gap; docs
  (design-system rows, spec status lines). (AC 11, 13)

### Milestone B: sliders (AC 17 to 32 of Part B)

- [ ] B1. Test first: a pure TypeScript module `sliderGesture.ts` (click vs drag vs
  double-click classification, thresholds 3 px / 8 px, 4 px second click) with `node --test`. (AC 17 to 19, 21, 24)
- [ ] B2. One hook `useSliderGesture` plus the 24 px target wrapper; S1 and S2 on it (native
  input kept or replaced; click end state, thumb click, mixed state); readout becomes `EntryField`
  on double-click and Enter/F2. (AC 17 to 22, 24, 25, 26, 28, 29)
- [ ] B3. S3 and S4 (colour area, hue) on the same hook; double-click focuses and selects the
  hex field of the block. (AC 17 to 20, 23, 24, 26 to 28)
- [ ] B4. Tooltips, aria, `touch-action`, `aria-keyshortcuts`; `design-system.md` "Slider" row. (AC 26, 27, 32)

### Milestone C: typed move reference (AC 33 to 50 of Part C)

- [ ] C1. Test first: `ui-core::reference_point` (nine points of a box, nearest-mark hit rule with
  radius `min(12, 0.24 s)`, ties to the centre), `MoveEntry` with a reference (prefill, resolve,
  rotated and multi-selection examples of AC 40). (AC 35, 37, 38, 40, 45, 47, 48)
- [ ] C2. `MoveEntry::new`/`resolve` take the reference; default Centre; `commit_move_entry`
  takes it; limit checked on all four edges. (AC 37, 38, 45, 49)
- [ ] C3. The reference box during move drags (preview bounds, no second pass) and while the chip
  is open; `render-core` draws box and marks (casing, tokens, layer order, viewport clip). (AC 33, 34, 35, 36, 39, 50)
- [ ] C4. Mark press: a `PressTarget` that exists only while an Absolute chip is open; executes on
  release, 3 px / 8 px rules, Escape, Copy check, hover state and hint. (AC 40, 43, 44, 45, 46, 47)
- [ ] C5. Frontend: Reference control (3 x 3 radiogroup, roving tabindex), chip placement, text
  of the Absolute segment, tokens `--reference-box`, `--reference-mark`; remove the old
  top-left text. (AC 41, 42, 49, 50)
- [ ] C6. Docs: `design-system.md` rows, `0010`/`0019` status lines; requirements R-EDIT-029.

### Closing

- [ ] Z1. Full gate: CLAUDE.md section 7, frontend tsc/oxlint/npm test, boolean budgets, wasm32
  builds of each touched `-core` crate.
- [ ] Z2. PR description draft with the review guide (not opened here; reported to the lead).

## Validation

Unit tests next to the code (ui-core, render-core), acceptance tests in
`curvyo-ui-core/tests/transform_polish.rs` and `curvyo-editor-wasm/tests/transform_polish.rs`
(one per numbered criterion where a pointer is not needed), the existing acceptance suites
updated where a criterion of this slice supersedes them (listed in the PR), `node --test`
for the slider gesture classifier and the shared texts, and the full gate. The look is for the
later UX review (no browser pane in this run).
