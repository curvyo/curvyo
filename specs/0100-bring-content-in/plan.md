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

- [x] 1. Plan (this file).
- [x] 2. Core read: primitive, text, image and group origins; `CloneSnapshot::picture` (the drawn
  leaves mapped, opacity counted through G, hidden below G, guides and dimensions left out,
  G's own flag ignored); chains through picture roots (AC 6, 7, 9, 22 c).
- [x] 3. Core: containment cycles orphan (SCC, order independent); `would_cycle` and its refusal
  in the Layers move and tiling apply (AC 12).
- [x] 4. Core: Linked clone accepts groups, texts and images (AC 8); `place_linked_clones` (one
  step, own placement, top of the active layer, depth-8 refusal) (AC 1 b, c, e, 6).
- [x] 5. Core: Unlink and the delete repair as copies for picture and primitive roots (AC 10, 11).
- [x] 6. Core: own placement held back for a group original moved whole; a member edit flows
  (AC 7 f).
- [x] 7. Core: `place_page`, placement transforms (translate, rotate, transform) with the
  off-page refusal, duplicate and copy; delete drawing page removes its placements (AC 13, 17,
  18, 19, 24).
- [x] 8. Core: format 38, golden `bring_content_in_v38.curvyo`, the format-37 refusal (AC 21).
- [x] 9. Render and ui-core: draw, bound, hit and mark picture clones; SVG export of picture
  clones and placements as `<g>`, goldens (AC 9, 20).
- [x] 10. Wasm: `clone_drop.rs` with `drop_mode` and its table test; the cloning release, the
  ghost, the badge answer; Alt move and Ctrl copy kept (AC 1 to 6).
- [x] 11. Wasm: `RowDragSource::DrawingPage`, its ghost and badges, `place_page` on drop; Place
  page by button (AC 13 to 15).
- [x] 12. Wasm: placed pages in the selection: hit by frame, box and handles, move, typed values,
  arrows, align, distribute, duplicate, delete, Group refusal, the Style block (AC 16 to 19).
- [x] 13. Wasm: CPU-time tests of criterion 22 (a) and (b); core timing test of (c) (AC 22).
- [x] 14. Drag answer contract: Rust test and `dragAnswer.ts` with its test on one fixture.
- [x] 15. Frontend: badge, Place page list, placed page block, Linked clone names and tooltips.
- [x] 16. Core + wasm + frontend: Place on… (inline list of laser pages, `place_linked_clones`
  at the same position relative to the page's top-left, clamped inside, one step) (AC 25).
- [x] 17. Browser check end to end; full gate (AC 23: no I/O added).

## Validation

Unit tests in each crate written before the code; session tests in `curvyo-editor-wasm` for the
gestures; goldens under `curvyo-document-core/tests/fixtures/` and
`curvyo-svg-core/tests/fixtures/export/`; CPU-time tests under
`curvyo-editor-wasm/tests/cpu_time/`; `npm test` for the decoder; a browser pass with
screenshots.

## Notes for the lead

- **(Proposal) Read shape.** `CloneSnapshot` keeps `resolved` (empty for a picture root) and
  carries the picture in `CloneChain::picture`; `ObjectSnapshot::picture()` and
  `drawn_path() == None` route every consumer. Same meaning as ADR 0045 §3's `CloneDrawing`,
  without touching the ~260 uses of `resolved`.
- **(Proposal) Unlink in place.** A picture or primitive root clone becomes its root's copy in
  the clone's own node (same id, the 0049 rule), so followers and undo need nothing new.
- **(Proposal) One drop badge enum.** `DropBadge::Clone` ("Linked clone") serves the sketch row
  and the object drag; added `Move`, `Copy`, `PlacedPage`, `OnlyOnLaserPage`.
- **(Proposal) Placed pages in the one selection.** Each reads as a frame object (image
  shaped, the unit square onto the source frame under its matrix; `Document::object` and the
  root context list it), so the Select tool's box, handles, typed values, Flip, arrows, align,
  distribute, Ctrl copy, Duplicate and Delete work on it; `translate_objects`,
  `transform_objects`, `rotate_object`, `duplicate_objects` and `delete_objects` hand a
  placement id to the placement commands; `group_objects` refuses it. Session state of the drop:
  `clone_drops: CloneDropState`.
- SVG export rounds every written matrix entry to 1e-9 (no float noise); goldens updated.
- Place on… list opens inline under its button in the Select bar.
