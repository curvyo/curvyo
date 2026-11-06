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
