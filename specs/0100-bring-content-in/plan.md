# Plan for bring content in

Implementer, 2026-11-09. Branch `story/0100-bring-content-in`. ADR 0045 (Proposed), `adrs.md`.
Frage 163 answered B by the customer (criterion 25: Place on… in the Select bar).

## Affected crates/modules

- **`curvyo-document-core`**: `clone_read.rs` (picture roots, the containment SCC orphan rule),
  `clone_picture.rs` (new: the drawn subtree of a group, text or image root and its map per
  clone), `clone_model.rs` (`CloneSnapshot::picture`), `object_read.rs` / `node_read.rs` (the
  tree structure the read needs), `clone_cycle.rs` (new: `would_cycle`), `node_move.rs` and
  `tiling_*` (the refusal), `clones.rs` (wider origins, `place_linked_clones`), `clone_unlink.rs`
  (new: Unlink and the removal repair as a copy for picture and primitive roots),
  `clone_transform.rs` (a group origin held back when its whole subtree moves),
  `placement_commands.rs` (new: `place_page`, placement transforms with the off-page refusal,
  duplicate), `pages.rs` (delete a drawing page removes its placements), `document.rs`
  (`format_version` 38), goldens.
- **`curvyo-render-core`**: picture leaves drawn through the per-kind code; faded per leaf.
- **`curvyo-ui-core`**: bounds, hit, oriented box, marks of picture clones; SVG export `<g>` for
  picture clones and placements; clone panel names and rows by root kind; placed page objects.
- **`curvyo-editor-wasm`**: `session/clone_drop.rs` (new: `CloneDrop`, the pure `drop_mode`),
  the release path of the Select move, `row_drag.rs` (`RowDragSource::DrawingPage`), placements
  in the one selection (the `selected_placement` field goes), Place page, Group refusal,
  CPU-time tests, `tests/drag_answer_contract.rs`.
- **`frontend`**: `lib/dragAnswer.ts` (+ fixture + test), canvas Drop badge for the object drag,
  `usePageRowDrag.ts` without `as Answer`, Place page list, placed page block, clone tooltips.

## Tasks

- [ ] 1. Plan (this file).
- [ ] 2. Core read: primitive, text, image and group origins; `CloneSnapshot::picture` (the drawn
  leaves mapped, opacity counted through G, hidden below G, guides and dimensions left out,
  G's own flag ignored); chains through picture roots (AC 6, 7, 9, 22 c).
- [ ] 3. Core: containment cycles orphan (SCC, order independent); `would_cycle` and its refusal
  in the Layers move and tiling apply (AC 12).
- [ ] 4. Core: Linked clone accepts groups, texts and images (AC 8); `place_linked_clones` (one
  step, own placement, top of the active layer, depth-8 refusal) (AC 1 b, c, e, 6).
- [ ] 5. Core: Unlink and the delete repair as copies for picture and primitive roots (AC 10, 11).
- [ ] 6. Core: own placement held back for a group original moved whole; a member edit flows
  (AC 7 f).
- [ ] 7. Core: `place_page`, placement transforms (translate, rotate, transform) with the
  off-page refusal, duplicate and copy; delete drawing page removes its placements (AC 13, 17,
  18, 19, 24).
- [ ] 8. Core: format 38, golden `bring_content_in_v38.curvyo`, the format-37 refusal (AC 21).
- [ ] 9. Render and ui-core: draw, bound, hit and mark picture clones; SVG export of picture
  clones and placements as `<g>`, goldens (AC 9, 20).
- [ ] 10. Wasm: `clone_drop.rs` with `drop_mode` and its table test; the cloning release, the
  ghost, the badge answer; Alt move and Ctrl copy kept (AC 1 to 6).
- [ ] 11. Wasm: `RowDragSource::DrawingPage`, its ghost and badges, `place_page` on drop; Place
  page by button (AC 13 to 15).
- [ ] 12. Wasm: placed pages in the selection: hit by frame, box and handles, move, typed values,
  arrows, align, distribute, duplicate, delete, Group refusal, the Style block (AC 16 to 19).
- [ ] 13. Wasm: CPU-time tests of criterion 22 (a) and (b); core timing test of (c) (AC 22).
- [ ] 14. Drag answer contract: Rust test and `dragAnswer.ts` with its test on one fixture.
- [ ] 15. Frontend: badge, Place page list, placed page block, Linked clone names and tooltips.
- [ ] 16. Core + wasm + frontend: Place on… (inline list of laser pages, `place_linked_clones`
  at the same position relative to the page's top-left, clamped inside, one step) (AC 25).
- [ ] 17. Browser check end to end; full gate (AC 23: no I/O added).

## Validation

Unit tests in each crate written before the code; session tests in `curvyo-editor-wasm` for the
gestures; goldens under `curvyo-document-core/tests/fixtures/` and
`curvyo-svg-core/tests/fixtures/export/`; CPU-time tests under
`curvyo-editor-wasm/tests/cpu_time/`; `npm test` for the decoder; a browser pass with
screenshots.

## Notes for the lead
