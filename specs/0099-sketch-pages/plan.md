# Plan for sketch pages

Implementer, 2026-11-06. Branch `story/0099-sketch-pages`. ADR 0043 (Proposed), `adrs.md`.
Frage 157 A and 158 A built (their defaults).

## Affected crates/modules

- **`curvyo-document-core`**: `sketch_page.rs` (`is_fixed_sketch`, a cheap `page_sketch`),
  `sketch_page_commands.rs` (new: `convert_sketch_page`, `place_linked_clone`, the sketch branch
  of `delete_page`), `pages.rs` (`delete_page` calls it), history label guard, golden
  `tests/fixtures/sketch_pages_v37.curvyo`.
- **`curvyo-ui-core`**: `page_membership.rs` (new: the one membership read of ADR 0043 §1);
  `placement_view.rs` and `svg_export.rs` call it instead of their own overlap tests.
- **`curvyo-editor-wasm`**: `session/pages.rs` (frame move, Fit page, page selection use the
  membership read; sketch frame move carries only the sketch; sketch delete and convert),
  `session/context.rs` (the fixed sketch is not pickable), `session/row_drag.rs` (new: the
  page-row drag, `RowDragSource::Sketch`), `session/sketch_use.rs` (new: usage count, Place clone
  on, Select clones of the sketch, Select original onto the sketch page), Layers row click,
  double-click on a clone, `pages_json` (state, clone count), wasm bindings, CPU-time test.
- **`frontend`**: `PagesBlock.tsx` (state line, no frame switch on sketch rows, delete wording,
  drag out of the list), `hooks/usePageRowDrag.ts` (new: ghost position and badge),
  `SketchesPanel.tsx` (Use group, Place clone on list, Convert full width), canvas ghost.

## Tasks

- [x] 1. Plan (this file).
- [x] 2. Core: `is_fixed_sketch`; `page_membership.rs` with tests (rectangle inside a sketch
  frame on no page; sketch across a drawing page's edge only on its sketch page) (AC 1, 4).
- [x] 3. Wasm: frame move, Fit page, nudge scope and press-to-activate through the membership;
  `placement_members` and the SVG export scope through it (AC 1, 2, 5, 18 c).
- [x] 4. Wasm: fixed sketch not pickable, not in marquee, Select all or Layers selection; Layers
  row of the sketch activates the sketch page; Sketch Select never touches drawing objects
  (AC 3, 4).
- [x] 5. Wasm: sketch frame move with `[owner]` and every clone held; no frame-only switch;
  resize keeps the sketch (AC 5, 6).
- [x] 6. Core + wasm: `place_linked_clone` (own placement, top of the active layer, one step
  "Place sketch clone"); row drag begin/move/drop/cancel with badge kinds, target page,
  snapped ghost offset; Place clone on page centre (AC 8, 9, 10, 11).
- [x] 7. Wasm: usage line and count, Select clones of the sketch, Select original and
  double-click onto the sketch page, clone follow test with rotation and scale (AC 7, 12, 13).
- [x] 8. Wasm: page row state, accessible name, clone count in `pages_json` (AC 14, 16).
- [x] 9. Core + wasm: `convert_sketch_page` (kind to drawing, one step, paths selected, undo
  restores) (AC 15).
- [x] 10. Core + wasm: delete sketch page with the `0049` clone repair, one step, undo restores
  clones as clones (AC 16, 17).
- [x] 11. SVG export goldens (a) to (d) and `sketch_pages_v37.curvyo` golden (AC 18, 19).
- [x] 12. CPU-time test: frame drag and ghost move within 16 ms thread CPU (AC 20).
- [x] 13. Frontend: page row, row drag with ghost and badges, Use group, Place clone on,
  Convert, focus to the canvas (AC 8 to 16, 22).
- [x] 14. Browser check end to end; full gate (AC 21: no I/O added).

## Validation

Unit tests in each crate for core logic, written before the code; session tests in
`curvyo-editor-wasm` for the gestures; goldens under `curvyo-document-core/tests/fixtures/` and
`curvyo-ui-core/tests/fixtures/export/`; CPU-time test under `curvyo-editor-wasm/tests/cpu_time/`;
`npm test` for the hook; a browser pass with screenshots.

## Notes for the lead

- "Has geometry" is read as "the solved outline has at least one anchor" (`adrs.md` note 5): a
  lone point or only construction lines draw nothing.
- `place_linked_clone(original, translation)` uses the document's creation parent (the session
  sets it to the active layer) instead of a parent argument (ADR 0043 §6 names one).
- A frame drag of a sketch page started on a drawing page keeps the drawing tools during the drag
  and activates the sketch page on release; before, the workbench switched mid-drag and the
  release never ended the drag.
- The trace mask pick still refuses a press on a page's sketch, although that sketch is no
  longer pickable (`0025` criterion 38).

## Revision 2026-11-19, slice 1: sketch instances (criteria 23, 24, 26 to 31; slice 2 handles not built)

Implementer, 2026-11-25. Branch `story/sketch-instances`. Decisions A1 to A11 of `adrs.md`; Frage 372
at its default A (the fallback is saved). `format_version` 62.

### Affected crates/modules

- **`curvyo-sketch-core`**: `instance_values.rs` (new: `InstanceValues`, `Sketch::with_values`,
  `Sketch::exposed_value`, the exposable relations).
- **`curvyo-document-core`**: `instance_value.rs` + `instance_codec.rs` (new: `InstanceValue`, the
  one-key `{"mm"}`/`{"deg"}` encoding, the three registers, damage rules), `sketch_instance.rs`
  (new: solve once per value set, content-keyed cache), `clone_read.rs` (effective sets, per-set
  refinement), `object_read.rs`, `clone_model.rs` (`own_params`, `no_solution`),
  `instance_commands.rs` (new: expose, limits, change, reset, own, fallback), the follower rule in
  Convert, Delete and Unlink, `document.rs` (format 62), goldens
  `sketch_instances_v62.curvyo` and `sketch_instances_damaged_v62.curvyo`.
- **`curvyo-extension-core`**: `ParamValue::to_stored`/`from_stored` call the document-core codec for
  `mm` and `deg`.
- **`curvyo-ui-core`**: refactor commit first (`instance.rs` view code to `instance_view.rs`);
  `InstanceSource`, `instance_sketch.rs` (rows, clamp, plan, commits), `svg_export.rs` `as_use`
  (`!own_params`), golden `export_sketch_instance.svg`.
- **`curvyo-editor-wasm`**: `session/generators.rs` preview slot takes an `InstanceSource`, new
  `session/sketch_instance.rs` (bindings for rows and commits, no-solution status), CPU test
  `tests/sketch_instances_cpu_time.rs`.
- **`frontend`**: Expose switch (Parameters tab, Sketches rows, entry box), exposed mark in the
  dimension pill, the `Parameters` row in `CloneBlock`'s `extraRow` with `ValueField`s, no-solution
  state line and dashed outline; `docs/design-system.md` rows.

### Tasks

- [x] 1. Refactor, behaviour-preserving: `instance.rs` view code moves to `instance_view.rs`.
- [x] 2. Sketch-core `with_values` and the exposed value read, tests first (AC 23, 26).
- [x] 3. Document-core value type, codec, registers, damage rules, format 62 and goldens (AC 30).
- [x] 4. Read pass: effective sets over chains, per-set solve with the content-keyed cache, the
  fallback rule, `own_params` and `no_solution` on the chain (AC 26, 27, 28, 31).
- [x] 5. Commands: Expose, Stop exposing (drops clone values), limits and name, change, reset,
  own, the fallback write (AC 23, 24, 27, 28).
- [x] 6. Follower rule in Convert, Delete, Unlink; SVG `as_use` (AC 15, 16, 17, 29).
- [x] 7. ui-core `InstanceSource` and the sketch arm: rows, clamp notice, plan (AC 24).
- [x] 8. Session: preview slot, bindings, status bar, CPU test (AC 24, 26, 31).
- [x] 9. Frontend: Expose, Parameters row, no-solution look; design-system rows (AC 23, 24, 26).

### Validation

Unit tests per crate before the code; goldens under `curvyo-document-core/tests/fixtures/` and
`curvyo-ui-core/tests/fixtures/`; CPU-time test under `curvyo-editor-wasm/tests/`; the 0141 tests
as the safety net of the refactor.

### Notes for the lead

- Measured (release, `cargo test --release -p curvyo-editor-wasm --test sketch_instances_cpu_time`):
  a field-drag step with the 200-clone workload 7.6 ms worst of 40 steps (budget 16 ms), the open
  with its first frame (200 solves) 265 ms, the release commit 10 ms with 0 further solves. One
  solve of the 100-entity sketch costs about 1.3 ms, under the 4 ms of A4, so `SolveMode::Edit` is
  used and no `SolveMode::Values` was added.
- The status bar line "Linked clone, Page 2, no solution" of criterion 26 is not built: the app has
  no status bar for a selection today; the Parameters row's state line carries the message.
- Slice 2 (canvas handles, criterion 25) is not built.

## Revision 2026-11-19, slice 2: canvas handles and linked values (criteria 24, 25, 26, 31 to 37)

Implementer, 2026-11-25. Branch `story/sketch-instance-handles`. Decisions A12 to A16 of `adrs.md`;
Fragen 373 B and 374 B. `format_version` 64.

### Affected crates/modules

- **`curvyo-ui-core`**: refactor commit first: `curvyo-editor-wasm/src/session/sketch/dimension_geometry.rs`
  moves unchanged to `sketch_dimension_geometry.rs` (A12). Then `instance_handles.rs` (`Knob` with
  `String` key and label, `KnobMotion::{Along, Around}`, `dragged`), new
  `instance_sketch_handles.rs` (where each exposed value of a sketch clone puts its knob),
  `instance_sketch.rs` (`only_this`, coupled plan and change through `instance_links`, the pill and
  marks from committed state, group letters and partners per row), `history_names.rs`,
  `step_scope.rs` (the two link steps).
- **`curvyo-document-core`**: new `instance_links.rs` (`linked_change`, `linked_reset`), `KeySet` in
  `instance_value.rs`, `link` on `exposed_params` entries and the clone register
  `instance_unlinked` (`instance_codec.rs`), groups resolved in `sketch_instance::exposed_of`,
  `link_exposed` / `unlink_exposed` and the Stop exposing rule (`instance_commands.rs`),
  `CloneChain::{unlinked, fallback_values}` copied by `RawLeaf::of_clone` (A15a),
  `Document::instance_solution`, `Document::clone_previews` on the document's solve cache, Unlink and
  Convert drop `instance_unlinked`, format 64, goldens `sketch_instance_links_v64.curvyo` and
  `sketch_instance_links_damaged_v64.curvyo`.
- **`curvyo-editor-wasm`**: new `session/sketch_instance_handles.rs` (memo, press, drag, release,
  Escape, readout, draw list of the sketch-clone knobs), the shared knob rules of
  `session/generators/handles.rs` as functions over `&[Knob]`, the pointer router (generator knob,
  sketch knob, Select), `only_this` on the preview and commit bindings, Link and Unlink bindings and
  the Link list in the sketch view JSON; CPU test `tests/sketch_instances_cpu_time.rs` gains the
  knob drag and the coupled knob drag.
- **`frontend`**: the Link line and list on exposed rows (Parameters and Sketches tabs), the group
  letter in the dimension pill, the link mark in the clone fields, Shift = only this at the gesture
  start on grouped fields with "only this", PageUp / PageDown, the knob drag chip glyph;
  `docs/design-system.md` rows (UX notes 12.8), the debt entry of A15a removed from
  `docs/technical-debt.md`.

### Tasks

- [x] 1. Refactor, behaviour-preserving: `dimension_geometry.rs` moves to ui-core (AC 25 prerequisite).
- [x] 2. `linked_change` / `linked_reset`, tests first: targets, intersection clamp with the held
  bound, empty intersection, only this marks only a changed grouped value, Back of one and of all
  (AC 33, 34).
- [x] 3. Codec: `link` on entries, kept through a limit edit; `instance_unlinked` list register and
  its damage rule; groups resolved at read (kind of the first member, a group of one is none)
  (AC 32, 37).
- [x] 4. Commands: Link (new group, join, merge) and Unlink (a group of one dissolves, marks dropped);
  Stop exposing leaves the group in the same step; the clone writes its marks with its values; a
  deleted value leaves its group by read (AC 32, 35, 36).
- [x] 5. `CloneChain::{unlinked, fallback_values}`, `of_clone` copies them, `clone_previews` on the
  document cache, `instance_solution`; Unlink and Convert drop the marks (AC 26, 29, 36, A15a).
- [x] 6. Format 64, doc paragraph, goldens incl. damaged, residue test, tests moved 63 to 64 (AC 37).
- [x] 7. ui-core: `Knob` motions and `dragged`; sketch knob anchors and rates (50 % clone, rotated,
  mirrored angle, bare-name rule, diameter x2) (AC 25).
- [x] 8. ui-core sketch arm: `only_this`, coupled clamp notice naming the member whose limit held,
  pill and marks from committed state during a gesture, group letter and partners per row (AC 24,
  32 to 34, 36).
- [x] 9. Session and wasm: sketch knobs (72 px, collisions, Ctrl/Cmd snap, Shift at the press, partner
  knobs live, readout, Escape, one step on release), Link and Unlink bindings, `only_this`
  arguments; CPU test of a knob drag and a coupled knob drag with 200 clones (AC 25, 31, 33, 34, 37).
- [x] 10. Frontend: Link line and list, group letter on the pill, link marks in the fields, Shift
  rule, PageUp / PageDown; design-system rows; debt entry removed (AC 24, 32 to 36).

### Validation

Unit tests per crate before the code (`instance_links.rs`, codec, the knob anchors); goldens under
`curvyo-document-core/tests/fixtures/`; the CPU-time test under `curvyo-editor-wasm/tests/`; the
generator knob tests of `0141` as the safety net of the `Knob` change; frontend unit tests for the
decoders and the Shift rule.

### Notes for the lead

- Measured (release, `cargo test --release -p curvyo-editor-wasm --test sketch_instances_cpu_time
  -- --nocapture`, 200 clones of the 100-entity sketch): a field-drag step 7.7 ms worst, a coupled
  knob-drag step 7.9 ms worst (budget 16 ms), open with the first frame 259 ms, the field release
  10 ms and the knob release 16 ms, both with 0 further solves. A knob step solves at most once for
  the whole group; a step whose set equals another clone's reuses that solve.
- Built after the first review: the numeric entry chip on a knob double-click (one field with the
  link mark, Enter commits coupled, Escape and a blur write nothing), the `--dim-guide` extension
  lines and arrowheads of the knob's dimension and its partners' on hover and drag, the follower
  border on partner fields during a coupled field drag. The link commands live in
  `instance_link_commands.rs`. The focus moves after Link and Unlink (UX notes 12.9.1; `fix/sketch-link-focus`). Not built:
  the status-bar line of criterion 26 (not built by the criterion).
- (Proposal) `linked_change` takes the clone's inherited values besides its own (A14 lists only the
  own), so "the value changes" of criterion 34 compares with what the clone shows up a chain.
- (Proposal) The dimension pill's accessible name adds ", linked in group A" (UX notes ask for
  ", linked with Radius 2"; the pill JSON carries the letter only).
- (Proposal) The 72 px rule is the generator knobs' rule (the shorter side of the box), shared: a
  clone of a single straight line never shows knobs.
- Format: `origin/story/machine-focus` (commit f6a55072, laser-core record format 8) also claims
  document format 62; whichever merges second renumbers.
