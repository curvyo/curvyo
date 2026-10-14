# Plan for Click to create shapes

Branch `story/click-to-create-shapes`, one PR for the whole slice at the end (`CLAUDE.md` §4).
Inputs: `specification.md` (criteria 1 to 36), `adrs.md` (decisions 1 to 9). Built on `main` at
825c2b0, where `0050` (layers) is merged, so the locked/hidden layer refusal
(`Session::refuse_drawing_into_layer`) is wired into the placement start and the place in this
slice (`adrs.md` decision 9). No new crate, dependency, ADR or `format_version` change.

## Affected crates/modules

- `curvyo-document-core`
  - `shapes.rs`: new `Document::create_primitive(&self, shape: Shape, rotation: Angle) -> NodeId`,
    one commit, the kind's existing label, the `rotation` register written only when not zero;
    `create_rect`, `create_ellipse`, `create_polygon`, `create_star` become one-line calls with
    `Angle` zero (decision 2).
  - `primitive_model.rs`: the `PrimitiveSnapshot::rotation` doc comment no longer says "a
    Select-tool rotate drag is the only writer".
- `curvyo-ui-core`
  - `shape_tool_common.rs`: `CreatePreview.centre` renamed `pivot` (decision 1).
  - `rectangle_tool.rs`, `ellipse_tool.rs`, `poly_star_tool.rs`: `pub fn shape_for(a, b,
    modifiers) -> Option<CreatePreview>`; `live_shape` and `pointer_up` call it.
  - new `shape_placement.rs` (state, fields, freeze, field texts, result) and
    `shape_placement/resolve.rs` (typed geometry: box from A in the kept direction or centred,
    Ctrl link, turn about A).
  - `display_unit_text.rs`: `parse_length(text, unit)` with optional `mm`/`cm`/`in` suffix
    (decision 3).
  - `lib.rs`: exports.
- `curvyo-editor-wasm`
  - new `session/placement.rs`: click-or-drag decision, start (mouse, touch, Enter), follow,
    modifiers, second press, place (commit, select, Select tool), cancel, chip view, live region
    texts; layer refusal at start and place.
  - `session/shapes.rs`: `shape_pointer_up` measures press-to-release against the threshold; the
    create-drag readout is hidden during a placement.
  - `session/mod.rs`: `placement` field, `set_touch_input`, `pointer_down` routes a press during a
    placement, `set_tool` cancels a placement.
  - `session/keys.rs`, `session/key_table.rs`: `operation_in_flight` includes a placement;
    `cancel_drag_in_flight` cancels it (Escape step 2); `KeyAction::StartPlacement` for Enter.
  - `session/select.rs`: `modifiers_changed` forwards to an unfrozen placement.
  - `session/draw.rs`: preview with rotation and the pivot marker from `CreatePreview.pivot`.
  - `wasm_shape_tools.rs`: chip view, `placement_edit`, `placement_toggle_diameter`,
    `placement_place`, `cancel_placement`, `set_touch_input`.
  - new `benches`-style `#[ignore]` test for criterion 32 (decision 7).
- `frontend`
  - new `PlacementChip.tsx`; the shared chip surface and error line taken out of
    `MoveEntryChip.tsx` if they are inline there (decision 1, frontend); `Canvas.tsx` mounts the
    chip inside the canvas container, the capture-phase outside-press listener (decision 6), Enter
    routing, the touch flag; `ToolRail.tsx` tooltip line; `ShapeToolbar.tsx` gets the `data-`
    attribute of the active tool's bar.
- Docs: `0003`, `0009`, `0010`, `0011` specs corrected per "Changes to accepted behaviour";
  `specification.md` status and PR link at the end.

## Milestones (commits on the branch, each green)

1. **M1 document-core API** (`feat(curvyo-document-core)`): `create_primitive`. Architect review
   on the branch diff before M2.
2. **M2 ui-core placement** (`feat(curvyo-ui-core)`): `shape_for`, `pivot` rename,
   `parse_length`, `shape_placement` with `resolve`.
3. **M3 session and wasm** (`feat(curvyo-editor-wasm)`): placement in the session, key gate,
   Escape, Enter, cancel, draw, wasm surface, rewritten session tests, benchmark.
4. **M4 frontend and docs** (`feat(frontend)`, `docs`): chip, listeners, tooltips, spec
   corrections.

## Tasks

- [x] 1. M1: `Document::create_primitive(shape, rotation)`, the four `create_*` delegate; tests:
  same keys and label as `create_*` for each kind, rotation register absent at 0 and stored
  normalized otherwise, one commit (fulfils AC 6, 16 b, 18, 35, 36).
- [x] 2. M2: `shape_for` on the three tools, `live_shape`/`pointer_up` through it; `centre` to
  `pivot` (AC 2, 4, 21, 34).
- [x] 3. M2: `parse_length` with unit suffixes; property tests comma = point, "x in" = 25.4 x mm
  (AC 23, 24).
- [x] 4. M2: `resolve.rs` tests first from the examples of AC 16 (a, b, c), 17, 18, 19; then the
  functions (AC 16 to 19).
- [x] 5. M2: `ShapePlacement`: following, field texts from the live shape (AC 12), freeze on first
  edit keeping direction, Shift, Ctrl link and polygon angle (AC 14), linked W/H (AC 17), r/⌀
  toggle (AC 20), result and `FieldError` with "Enter a number", "Must be more than 0", "Too
  large" (AC 21, 23), keyboard start with untouched size refused (AC 26).
- [x] 6. M3: click-or-drag decision in `shape_pointer_up` with `drag_threshold` (3 px; 8 px with
  touch) (AC 1, 31).
- [x] 7. M3: follow, modifiers live, second press commit, press near A ignored, frozen press places
  anywhere, one commit, select, Select tool, entered group or active layer (AC 2 to 6, 22).
- [x] 8. M3: Escape step 2, tool switch cancel, `cancel_placement`, gate in `operation_in_flight`,
  Enter `StartPlacement`, blur keeps the placement and drops modifiers (AC 7, 8, 26, 27).
- [x] 9. M3: draw: preview turned by the placement's rotation, pivot marker at A, no outline within
  3 px of A, readout hidden; chip view (labels, texts, placeholder, link, hint, error, group name),
  live-region texts (AC 2, 11, 12, 15, 28, 29, 30).
- [x] 10. M3: polygon/star bar changes during a placement reach the preview (AC 25); layer refusal
  at start and place (UX notes 5).
- [x] 11. M3: rewrite the session tests that press and release closer than 3 px (decision 8), list
  below; the `#[ignore]` release benchmark (AC 32).
- [x] 12. M4: `PlacementChip.tsx` with shared surface, place rule (outward, flip, clamp; frozen
  anchor), Tab cycle, Space r/⌀, Enter, Escape, focus back to the canvas, outside-press and
  right-press cancel, tool-bar exception, touch focus rule, rail tooltip line (AC 7, 9, 10, 11, 13,
  20, 25, 29, 31, 33).
- [x] 13. M4: spec corrections in `0003`, `0009`, `0010`, `0011`; design-system rows checked
  (AC 34, "Changes to accepted behaviour").
- [ ] 14. Full gate (`CLAUDE.md` §7), real-window frame check noted for the lead.

## Tests at risk (decision 8)

The full suite right after the click-threshold change (2026-10-14) failed 5 tests; each is
rewritten, none deleted. The click now starts a placement: the test asserts that, Escape cancels
it, and the old checks (nothing written, tool and selection unchanged) stay.

- `session/shapes.rs::a_press_without_movement_creates_nothing_and_keeps_the_selection`
- `tests/acceptance_unified_editing_pr2.rs::ac26_a_press_without_movement_creates_nothing_and_leaves_selection_empty_and_tool_alone`
- `tests/acceptance_unified_editing_pr2.rs::ac26_a_press_without_movement_with_nothing_selected_keeps_it_that_way`
- `tests/acceptance_unified_editing_pr2.rs::ac25_a_double_click_in_a_creation_tool_creates_nothing_and_changes_nothing`
- `tests/acceptance_polygon_star_box_refit.rs::edge_tiny_and_huge_radius_keep_the_box_direction`:
  its `scene` helper drags radii of 0.05 and 0.6 mm (under 3 px); it now types radius and angle
  into the placement that click starts, which gives the same frame.

`fix_select_created_shape.rs`, `acceptance_0005_reverify.rs` and `shape_creation_from_center.rs`
did not fail.

## Deviations from `adrs.md` (implementer notes, 2026-10-14)

- Decision 6: Escape cancels the placement in `Session::escape` right after
  `cancel_drag_in_flight`, not inside it, because `cancel_drag_in_flight` also runs for a lost
  pointer and a window blur, which must not cancel (criterion 7). Same step 2, same outcome.
- Decision 1, `Kept`: it records the pointer, the modifiers and the live shape at the freeze; the
  kept sizes, direction, Shift, Ctrl link and polygon angle are read from them.
  `ShapePlacement::result` also takes the bar's `PolyStarSettings` (criterion 25).
- Decision 7: the benchmark follows the repo's budget tests (asserted in release, printed in
  debug, run in the `boolean-budgets` CI job) instead of `#[ignore]`.
- `ShapePlacement`'s error type is `PlacementError`: ui-core already has a `FieldError`.

## Validation

- Unit tests first for `create_primitive`, `parse_length`, `resolve` and `ShapePlacement`, with
  the spec's examples as given; property tests for `parse_length` (decision 3).
- Session tests for every criterion that Rust decides (click threshold, gate, Escape, Enter,
  commit, cancel, layer refusal).
- `#[ignore]` native release benchmark (decision 7): placement preview frame of a 1024-point
  star at most 1.1 times the create-drag frame; one `result` under 1 ms.
- Manual check in the real window (`npm run tauri dev`) for the chip place, focus and frame rate.
