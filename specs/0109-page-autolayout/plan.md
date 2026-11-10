# Plan for page auto-layout and ownership by centre

Implementer, 2026-11-10. Builds `specification.md` criteria 1 to 41 per ADR 0051.

## Affected crates/modules

- `curvyo-document-core`
  - `src/page_layout.rs` (new): the pure layout over page frames, `reflow`, `push_apart`,
    `close_gap`, the page gap and the new-row fallback (ADR 0051 §2, §3, §3a).
  - `src/page_moves.rs` (new): `PageContents` and the one write of a layout (moved frames plus
    the translation of each moved page's owned objects) inside the triggering batch.
  - `src/pages.rs`, `src/document_size.rs`, `src/manufacturing.rs`,
    `src/sketch_page_commands.rs`: `resize_page`, `fit_page_to_content`,
    `set_manufacturing_profile`, `move_page`, `delete_page` take the caller's `PageContents` and
    run the layout in their own step; Add page falls back to a new row (criterion 29).
- `curvyo-ui-core`
  - `src/page_membership.rs`: `owner_by_centre` and `page_contents` (top-level unit, outline-box
    centre, output over drawing, otherwise ambiguous stays); `Reach::Inside` goes.
- `curvyo-editor-wasm`
  - `src/session/pages.rs`, `src/session/page_drag.rs`, `src/session/manufacturing/machine.rs`:
    read the contents before each size command and delete and at drag press; count moved pages
    for the status region.
- `frontend`: the Document section's sr-only status region gets "n pages moved." (ux-notes §7).

## Tasks

1. [x] `page_layout.rs` test-first: edge deltas, extents, chains, shift, cap, fallback
   (criteria 1 to 11, 13, 32, 33); unit tests for each named example and proptests for no new
   overlap, the changed page never moving, determinism and the round trip (criteria 5, 10 to 12).
2. [x] `push_apart` test-first: one page, a chain, between two, a close drop, a pair overlapping
   at press, the fallback (criteria 22 to 24); proptest no new overlap.
3. [x] `close_gap` test-first: three A4 pages (criterion 35, 239.7 mm), the grid of criterion 39,
   a column, a dragged distance kept, an overlapping page, cap and fallback (criteria 38 to 40).
4. [x] Add page new-row fallback when the spot is closer than a page's gap (criterion 29).
5. [x] Commands write the layout in one batch under the existing label, carrying contents
   (criteria 14, 26, 27, 28, 30, 32, 41); placements and fixtures follow the frame write only;
   clones held back (criterion 19); document-core tests with undo restoring everything.
6. [x] `owner_by_centre` and `page_contents` in ui-core (criteria 15 to 20); remove `Reach::Inside`.
7. [x] Session: contents before resize, fit, profile pick and delete; drag carries by centre at
   press and pushes on release (criteria 21 to 25, 36, 37); moved-page count for the status
   region (ux-notes §7).
8. [x] Frontend status region (ux-notes §7), one Chromium e2e of the customer's case
   (criteria 1, 14, 15, 17, 26, 30, 35).
9. [x] Update the tests that change by design; 200-page timing before and after; a CPU-time
   check for one resize, one push and one delete on 200 pages (criterion 34).
10. [x] Full gate.

## Validation

- Unit tests and proptests in `page_layout.rs` (pure, deterministic, 1e-9 mm tolerance).
- Document-core integration tests for the commands: one step, one undo restores frames,
  objects, own items and placements; nothing new written (`format_version` 39, criterion 28).
- Ui-core tests for ownership by centre.
- Editor-wasm tests for drag carry and push, delete close, status count, timing.
- One Chromium e2e for the customer's Page 1 plus Laser page case.

## Notes for the lead and the architect

- ADR 0051 §2 claims the exact round trip for any size change and its inverse when the first
  step capped no pull and sent no page to the new row. The property test found two counterexamples
  for resizes that grow or shrink both sides: a page beyond a corner (criterion 8) moves on both
  axes and leaves the other edge's extent, and a page pulled in along one edge can come to face
  the other axis's extent on the way back. For turns (criterion 5: one side grows, the other
  shrinks) the round trip holds and is proptested; the general claim is not asserted.
- Criterion 32 says a laser profile pick keeps the top-left corner; the code keeps the copy's
  origin corner (ADR 0042 §6), so a profile with a right or bottom origin also moves the pages
  beside its left or top edge. The reflow follows whichever corner the command keeps.

