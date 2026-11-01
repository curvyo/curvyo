# Plan for Measure and dimension

Implementer, 2026-11-01. Built with DIM-STORE A (ADR 0027 option A), MEAS-RAIL A (second button of the Path-tools card,
key M) and MEAS-KEEP A (Shift), all three confirmed by the customer on 2026-11-01. Order: document model, then ui-core and render-core, then the
session, then the frontend. Items marked **(Proposal)** are the implementer's reading of an open
point, vetoable at the demo.

## Affected crates/modules

- `curvyo-document-core`: `dimension_model.rs` (kind, slots, place, kept, snapshot),
  `dimension_codec.rs` (registers, damage rules), `dimension_read.rs` (pure resolve and measure),
  `dimension_commands.rs` (create, move text and offset, convert, copy remap, translate of kept
  points), `dimension_keep.rs` (the `commit_step` refresh); `ObjectSnapshot::Dimension`;
  `CURRENT_FORMAT_VERSION` 31; goldens in `tests/fixtures/`.
- `curvyo-ui-core`: `sketch_dimension_layout*` renamed to `dimension_layout*`;
  `dimension_text.rs` (the shared value formatter); `measure_tool.rs` (kinds, placement regions,
  picks, Alt phases, readout); every exhaustive `ObjectSnapshot` match gains a `Dimension` arm.
- `curvyo-render-core`: `sketch_annotation_marks.rs` becomes `dimension_marks.rs` with the look
  enum `Driving | Reference | Guide`; artwork skips dimensions.
- `curvyo-editor-wasm`: `session/measure*.rs` (tool, drawing, labels, hit test, Select-tool text
  drag, Guide button convert), wasm glue `wasm_measure.rs`.
- `frontend`: rail entry (key M), readout chip, mode label, dimension labels.

## Tasks

- [x] 1. Model: `DimensionKind`, point and curve slots, `DimensionPlace`, `DimensionKept`,
  `DimensionSnapshot` with its measured state; `ObjectSnapshot::Dimension`; a dimension is a
  guide by kind (`effective_guide`) so every output filter leaves it out (AC 9, 16).
- [x] 2. Codec: registers `dimension_kind`, `dimension_ref_a/b/c`, `dimension_place`,
  `dimension_kept`; open-file damage rules (unknown kind, slot count, malformed slot, non-finite
  value); format 31; goldens `dimension_v31.curvyo` and the four damaged files (AC 16).
- [x] 3. Resolve on read: slots against snapshots (path anchor, segment midpoint, primitive
  centre/corner/vertex/edge midpoint, clone by own id and root anchor ids, circle, arc
  circumcircle within 0.01 mm, rounded corner, free point); linked or unlinked; one pass after
  the carriers (AC 11, 12, 13, 14).
- [x] 4. Commands: create ("Dimension"), move text and offset ("Move dimension"), kept points of
  an unlinked dimension move with Node/translate ("Edit nodes"), copy remap (referents copied:
  remapped; else free points), convert to paths and a text ("Convert to path") (AC 4, 8, 10, 13).
- [x] 5. `commit_step` refresh of `dimension_kept` for linked dimensions whose value or points
  changed beyond 1e-9 mm; tests for Delete, Cut (delete), Boolean, Node delete, join, Break
  apart, Convert to path of a shape, clone unlink, non-uniform scale of a circle; undo relinks;
  thread-CPU-time cost test (AC 11, 12).
- [x] 6. ui-core: rename layout to `dimension_layout*`; shared value formatter (3 decimals
  trimmed for Vector; Sketch keeps its panel precision **(Proposal)**, as "at most the panel's
  precision" of 0066 is accepted behaviour); layout input from a `DimensionSnapshot` (AC 7, 8).
- [x] 7. ui-core `measure_tool`: the eight placement regions (pure, unit-tested), kind from
  picks, Angle sector, Radius/Diameter Alt cycle with the phase rules, Ctrl angle stops,
  selection-first eligibility, readout lines (length, angle, Δx, Δy) (AC 1, 3, 4, 5, 6).
- [x] 8. render-core: one look enum `Driving | Reference | Guide`, Guide solid green; dashed
  4/3 for unlinked; artwork skips dimensions (AC 7, 12).
- [x] 9. Session: Measure tool (key M, gating as R), picks with feature snapping and Alt,
  measurement pinned until the next press / Escape / tool change / undo / edit, Shift live swap,
  placement click, Enter on an eligible selection; dimensions drawn every frame from the drawn
  (preview) objects so they follow drags live; Select-tool hit and text drag; Guide button
  pressed on a dimension converts it (AC 1-8, 10, 11, 15).
- [x] 10. Layers row "Dimension 12.5 mm" (+ "not linked"), announcer text, Delete (AC 17).
- [x] 11. Frontend: rail button (Path-tools card renamed "More tools"), tooltip "Measure tool
  (M)" / "(Esc, M)", readout chip, mode label, dimension labels with the Unlink glyph, token
  `--dim-guide` (AC 1, 2, 4, 7, 12).
- [x] 12. Browser script `frontend/tests/browser/measure.cjs` with screenshots in
  `target/0070-browser/`; full gate.

## Proposals and readings

- Angle from two segments: the vertex slot is the shared anchor when the segments share one,
  else a free point at their intersection; the first and second slots are the far ends of the
  segments. The sector (which of the four angles) is stored in `dimension_place`. **(Proposal)**
- `dimension_place` holds four numbers: offset (mm), text fraction along the line, leader angle
  (radius/diameter, radians) and the angle sector (0 to 3). **(Proposal)**
- Measure picks resolve features in the tool itself (nodes, segment midpoints, shape centres,
  corners and vertices) instead of extending every `0060` snap target with a source; the grid,
  page and guide lines of `0060` are not dimension anchors (free points). **(Proposal; adrs.md
  decision 2 reversible)** These feature picks follow the document's snapping setting (lead
  decision on tester defect D2, 2026-11-01): with snapping on a point feature captures within
  8 px; with it off only a press right on the feature (1 px) names it, so nothing snaps to a
  centre or any other feature; Alt held at the pick inverts the setting. A curve or segment
  under the pointer is a hit, not a snap, and is picked either way.
- SVG export, G-code and a job preview do not exist yet; criterion 9 is covered by the one
  output filter (`NodeRead::is_output_node`) treating a dimension as a guide, tested in core.

## Follow-up of 2026-11-01 (built after the first report)

- [x] 13. Node tool on an unlinked dimension: its kept points show as nodes and move, one step
  "Edit nodes"; the value stays (AC 13).
- [x] 14. Hover band (4px `--segment-hover`) along the segment, arc or circle a press would
  pick; the Lock badge while Ctrl locks the direction (UX notes §5, §6).
- [x] 15. The polite live region says the value on a pin ("50 millimetres, 0 degrees") and on a
  placement ("Dimension 50 millimetres") (UX notes §3).
- [x] 16. A selected dimension has no selection box, group box or handles (UX notes §7).
- [x] 17. The Layers row and the announcer name the value in the display unit (AC 17).
- [x] 18. Session CPU test: a Select drag of 200 measured paths with 200 dimensions within one
  60 Hz frame or 1.5 times the frame without dimensions (**Proposal** for "at the frame rate";
  measured 2.1 ms against 1.1 ms in release), `curvyo-editor-wasm/tests/dimension_frame_budget.rs`
  (AC 11).

Superseded assertions: three tester tests asserted that M gives "Select an object first" outside
Select-with-a-selection; `0070` criterion 2 makes M the Measure tool there. The M part of
`acceptance_0019_tester` ac37, `acceptance_edit_polish_pr3_tester` c59 and
`edit_polish_typed_skew_move` was removed with a note; the session tests cover the new rule.

## Review follow-up of 2026-11-01 (architecture review and acceptance report)

- [x] 19. A linked dimension moved or transformed without all its referents writes nothing
  (`DimensionSnapshot::stays_when_moved`, used by `translate_objects` and `transform_objects`);
  an unlinked one, or one moved with its referents, moves its free slots and kept points
  (review 1).
- [x] 20. A copied radius or diameter whose curve was not copied gets a `free_circle` curve slot
  at the paste offset (`["free_circle", x, y, r > 0]`, damage when the radius is not above
  zero); copied free points move with the copy too (review 2, option a).
- [x] 21. Convert to path lays the selected dimensions out from the document at that moment;
  pointer hits keep reading the last frame (review 3).
- [x] 22. One arrowhead shape (`curvyo_render_core::dimension_arrow_outline`) and one guide
  colour (`curvyo_render_core::GUIDE`) for the canvas and the conversion; the conversion is split
  into paths, styles and text (review 4).
- [x] 23. Newtypes: `CIRCLE_TOLERANCE`, `arc_circle`, `differs_from`, `MeasurePick::Curve`,
  `measure_pick`, `length_text`, `dimension_text`, `dimension_speech`, `radius_readout` (review 5).
- [x] 24. The golden covers every slot form the codec writes: corner, vertex, edge middle, a
  circle under `radius`, an arc and a free circle; the free-circle damage rule is checked in
  memory, without a new damaged fixture (review 6).
- [x] 25. The selection-first and angle picks moved to `curvyo-ui-core/src/measure_tool.rs`
  with unit tests; `session/measure.rs` is under 500 lines (review 7).
- [x] 26. Shift is read at the second pick for every kind, the second segment of an angle
  included (tester D1); feature picks respect the snapping setting (tester D2, see above).

### UI review 2026-11-01 (`ux-notes.md` "UI review (2026-11-01)")

- [x] 27. Layers row of a dimension without its own name shows the value only ("⌀ 26.458 mm",
  "90° · not linked"; `LayerRow::text`); name, accessible name and `title` keep "Dimension …"
  (item 1, criterion 17).
- [x] 28. Row glyph Lucide `Ruler` 16px (item 2).
- [x] 29. Rust decides what a selection of dimensions only offers
  (`curvyo_ui_core::only_dimensions`): no Stroke or Fill (`StylePanelView::sections_shown`), no
  Linked clone, no Stroke to path (`stroke_to_path_shown`), no scale switches; a dimension never
  counts for Radius (item 3, criterion 9).
- [x] 30. Item 4 reproduced in Chromium: a press within 400 ms of a selecting click on the text
  is the second press of a double-click and is withheld (0 px moved, 2 of 2); after 500 ms the
  drag works (80 px, 2 of 2). A rectangle behaves the same (double-click edit hint), so this is
  the app-wide double-click rule, not a 0070 bug: dropped per the review's own rule; a
  "press that drags is not a double-click" change would touch every tool (note for the lead).
- [x] 31. Design-system rows "Measure tool", "Measure readout", "Vector dimension look" and
  "Tool rail budget with Measure" (the Shortcuts row existed) (item 5).
- [x] 32. The first anchor's dot stays under the Shift preview (`measure_open_anchor`) (item 6,
  criterion 4).

### Customer feedback on PR #243 (2026-11-01)

Reproduced first in Chromium (scratch script, then `frontend/tests/browser/measure.cjs`): a click
on a rectangle side, an ellipse axis end or its rim measured from a free point; a right click
started a measurement; the canvas cursor was the arrow and the chip "Measure" showed on
activation.

- [x] 33. Primitives (criteria 3, 6): a rectangle side, a polygon or star side is a segment pick
  between its `ShapePoint` corners or vertices (`measure_pick::edge_near`); corners, vertices and
  centres stay point picks, a rounded corner stays a radius, a circle keeps radius and diameter.
  **(Proposal)** A whole ellipse that is not a circle offers its centre and its four axis ends
  (`ShapeFeature::Vertex(0..4)`, east, south, west, north, resolved by `shape_point` from its
  outline; no format change, the codec already writes `vertex`); two of them give an aligned
  distance; its rim between them is no pick (no radius kind for ellipses); a circle shows no axis
  ends so its rim keeps the radius. A curved polygon or star side is no segment pick.
  Tests: `curvyo-ui-core` `measure_pick::shape_tests`, `dimension_read_tests`
  `an_ellipse_vertex_is_an_axis_end`, session `measure::feedback_tests` (through
  `pointer_down`/`pointer_up`).
- [x] 34. A secondary press and release in the Measure tool reach no tool (`useEditorSession`,
  as the creation tools): no measure, no pick, no dimension (browser check).
- [x] 35. In the Select tool a handle of the drawn selection box under the pointer wins over a
  dimension's text and lines (`Session::selection_handle_under_pointer` in
  `dimension_pointer_down`); test `a_resize_handle_wins_over_a_dimension_on_it` and the browser
  check.
- [x] 36. Measure shows the plain crosshair of the Rectangle tool (`Canvas.tsx`) and no chip
  until the first pick (`measure_mode_chip`); tester assertions c2 and c3 in
  `acceptance_0070_tester.rs` and `measure-tester.cjs` follow the customer's change (to be
  re-verified by the tester).

## Validation

- Unit tests per module (test first), goldens and damaged fixtures, native/wasm32 build of
  `dimension_read`.
- Session tests: history length unchanged after a measurement, one step per placement, live
  follow, unlink on delete and relink on undo, convert and round trip.
- Browser script with screenshots; full gate of `CLAUDE.md` §7.
