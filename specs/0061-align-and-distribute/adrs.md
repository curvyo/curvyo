# Architecture: align and distribute

Date: 2026-10-30. No format change, no new crate, no new dependency
(criterion 12 and the architect notes of the spec stand). Implementation
choices marked **Proposal** are reversible in this story.

## Decisions used

- The command is the pure function the architect notes ask for:
  `(boxes, reference rect, command) -> translations` (align) and
  `(boxes, axis, mode, typed gap) -> translations` (distribute) in
  `curvyo-ui-core` (`align.rs`), applied as **one commit** through
  `Document::transform_objects` with `ObjectSnapshot::translated` per unit
  (per-object translations; a shared `translate_objects` offset cannot express
  an alignment). `write_stroke_width` is `false`: a move never rescales.
- Units are `curvyo_ui_core::Units::selected`: a group is one unit
  (criterion 2), a linked clone moves through its own placement matrix via
  `translated` (criterion 11). One `Session::recording` per press, so the
  selection snapshot and one labelled history step come free (criterion 12).
- Boxes are the true bounding box `object_outline_bounds` unions per unit —
  the same function the selection box and `0056`'s TBB use (criterion 9,
  stroke width not included). Texts use the same TBB as every object; their
  drawn selection box agrees because the commands work in the document axes.
- The reference (criterion 3, 4): the key object's box when the key unit is
  still selected, else the selection box (the union box, the group box of
  `0019`); with exactly one selected unit the active page's rectangle
  (`Session::active_page`). The reference is resolved per press, never stored.
- The key object is view state on `Session` (`Option<NodeId>`, the unit's id),
  never in the document, never undoable — like the selection (architect
  notes). A key that is no longer selected is dropped (`sync_select_units`)
  and falls back to the selection box.
- Criterion 10's locked rule is the accepted `0050` behaviour: a locked object
  is never picked or selected directly (`Units::pickable`, criterion 14), and
  a selected group moves as one whole with its locked descendants (criterion 2
  "moved whole", `0050` criterion 21). No extra per-leaf lock logic.
- History labels: `align_left`, `align_centre_h`, `align_right`, `align_top`,
  `align_centre_v`, `align_bottom`, `distribute_h`, `distribute_v` in
  `history_names.rs` (criterion 12's "Align left", "Distribute horizontally";
  the distribute name is per axis, not per mode — **Proposal**).
- The wasm surface stays strings and scalars in `wasm_select_bar.rs`:
  `align_shown()`, `distribute_shown()`, `align(code)`, `distribute(code,
  Option<f64>)` (ADR 0001 §5). The typed gap reaches the command as
  millimetres; `None` is the empty field ("keep the ends").
- Ties in distribute order (equal start coordinate) keep the document's paint
  order (stable sort) — deterministic, no selection-order dependence.
