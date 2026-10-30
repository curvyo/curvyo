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
- **Extended 2026-10-30** (customer: "wenn nichts markiert ist, ist align auf
  page bezug … distribute evenly on page"): with an empty selection the
  commands act on the **active page's pickable objects** (`page_object_ids`:
  every context object whose true box overlaps the page, edges included) —
  align with the page as the reference for all of them, distribute across the
  page's extent (`distribute_on_frame_translations`: equal gaps from the
  page's start edge to its end edge, or the centres from edge to edge; a
  typed gap spaces from the page's start edge). Groups come in whole (a
  group partly on the page moves whole); hidden and locked objects are not
  pickable and never move. Same commit labels, one history step, the
  selection stays empty.
- Ties in distribute order (equal start coordinate) keep the document's paint
  order (stable sort) — deterministic, no selection-order dependence.
- **Amended 2026-10-30** (customer, "distribute funktioniert noch gar
  nicht"): a typed gap distributes from two units — the ends do not define
  the spacing a typed gap needs (the spec's amendment of criterion 7).
  Without a typed gap, three or more. The Gap field takes the "Bar number
  field" look: label outside, value right-aligned with the `mm` suffix inside
  the right edge, so value and unit never overlap, Escape restores the value
  the field had.
