# ADRs for 0073 parameter-handle-clone

- `0002` (document model, units and SVG round trip): a copy is an ordinary object made by the
  Duplicate path (`specs/0010-edit-interaction-polish/adrs.md`, decision 2); no new node, register
  or meta key, no `format_version` change.
- `0014` (history, undo and branches): the copy and its new value are one step under one label; an
  undo removes the copy and leaves the original.
- The key is the customer's answer to COPY-KEY (Frage 387, 2026-11-26): **Shift** pressed during the
  drag, Shift's state at the release decides. Considered: Alt (the lasso at the press, window managers
  take Alt+drag) and Ctrl (it snaps on parameter handles).

## Note 2026-11-26: a parameter drag or entry that writes a copy

No new ADR, crate, trait, `Document` method or format change. Everything sits on code that
exists: `Document::batch`, `Document::duplicate_objects`, `param_edit::apply_param` and
`commit_param`, and the copy preview of `LiveEdit.copy` and `DragSwap`.

**1. When the key is read.** Not fixed at the press: the copy flag is read live. `TransformDrag`
gets the live Shift state (`copy_key`) and `shift_lifted`; `TransformDrag::note_copy_key` sets them
from every pointer and key event (through `Session::modifiers_changed`, which the wasm layer calls
with the event's own flags before every pointer event and on every key event, so each redraws, also
with the pointer at rest) and, last, from the pointer-up's own `shiftKey` in
`pointer_up_snapped_with_artwork`, so a lost keyup cannot turn a visible copy into a change. The
preview (`live_edit_snapped`) and the commit both read that one field, so what was drawn in the last
frame is what is written. A Shift held from the press counts only once it was let go
(`shift_lifted`): at the press it chose the one-corner drag of `0013` and keeps that meaning alone
(Proposal). Shift pressed mid-drag and released before the end writes a plain change, no copy.
`classify_press` and the press are unchanged: Shift at a press on a knob starts the knob drag and
does not toggle the selection. A parameter drag reads no other Shift (`shape_knobs.rs`,
`param_edit.rs`); resize, rotate and skew keep their own Shift (`param_copy` is `None` for them).

**2. Preview.** A copy drag returns `LiveEdit { copy: true }` with the resolved object under the
original's id, as a move copy does. `DragSwap` then keeps the original drawn as itself, draws the
copy directly above it and draws no ghost; this is the look Duplicate's result will have.
Decorations: `live_objects_in` substitutes nothing for a copy, so the box and the other knobs stay
on the original, which is what `ux-notes.md` section 3 (V6) asks for; the dragged knob, the Rounded
guide and the readout are drawn at the copy's value (`select_view.rs` `param_copy_knob`; the
" · copy" suffix in `select_readout.rs`).

**3. Commit.** New `pub(crate) fn commit_param_copy(document, handle, result) -> Option<NodeId>` in
`curvyo-ui-core/src/param_edit.rs`: `document.batch(copy_label(handle), ..)` around
`duplicate_objects(&[CopySource { id, anchor_ids: vec![] }], Vec2::ZERO)` (a primitive has no
anchors) and `commit_param` on `result` with the copy's id. The copy lands directly above its
original with the same style and layer, and `clone_origin` gives it the "Duplicate of ..." row of
`0041` (that row comes from the copy's meta and the step match, not from the label). `batch` has
no rollback, so every check runs before the duplicate: `still_current`, and a result that differs
from the start beyond `PARAM_EQUAL_EPSILON` (an unchanged value or a release in the 3 px dead zone
writes nothing, also as a copy). The `Transforming` branch of `pointer_up_snapped_with_artwork`
calls it when `copy` is set and selects the copy (`selection.set`), as `commit_move` does.

**4. Limiting and the notice.** The copy is written from the same `apply_param` result as a plain
drag, so it gets the limited value and the drag's readout ("max") unchanged. The typed entry's
limit notice in `commit_transform_entry_fields` reads `entry.object().id()`, the original; for a
copy it must read the copy. The selection must be the copy before `arc_or_curve_announcement` runs.

**5. Entry box (unchanged by the key).** `ParamEntry::commit_as_copy` and
`SelectTool::commit_param_entry_as_copy` (the "Copy" check in the UI, `ux-notes.md` V3) write the
copy and select it, as `commit_move_entry` does; `commit_entry` is unchanged. The check's state lives only in the DOM chip and
starts unset on every open (Proposal). Public API change: the wasm exports
`commit_transform_entry` and `commit_transform_entry_fields` gain an `as_copy: bool`.

**6. Labels.** One label per parameter family, named like its "Change ..." step:
`copy_with_curve` "Copy with curve", `copy_with_arc` "Copy with arc", `copy_with_corner_radius`
"Copy with corner radius", `copy_with_inner_ratio` "Copy with inner ratio", `copy_with_rounding`
"Copy with rounding". Each goes into `OPERATION_NAMES` (`history_names.rs`) and `EDIT_STEPS`
(`step_scope.rs`); both guard tests fail otherwise.

**7. Scope facts for the PO.** A selection of two or more objects has no parameter knobs
(`begin_handle_press` starts a group press; the group handles carry no `ParamHandle`), so
criterion 6 has no case today; a parameter drag on a multi-selection would be a new feature. The
function takes one result; no batch path is built ahead (`CLAUDE.md` §5). `ParamHandle` has no
point-count ("sides") knob, but has `Rounded` (`0145`); the spec's handle list should match.

**8. Preview modifiers.** The knob preview passes raw Shift (the copy key lives on the drag, not in
the argument) and Ctrl as today; Shift keeps driving resize-about-centre and the hover pivot marker
of the transform handles.

**9. Tests.** In `curvyo-ui-core`: for each family, original unchanged, one new object with the
value directly above it, one step with its label, and one undo leaves only the original; a limited
corner radius and curve on the copy; an unchanged value as a copy adds no step; the copy's first
object-history row is "Duplicate of" the original; `pointer_up` selects the copy; `live_edit` is a
copy exactly while the live Shift state is set: Shift pressed mid-drag shows the copy in that frame,
Shift released shows the change again; Shift pressed mid-drag and released before the end commits
no copy; Shift down at the release commits the copy; the release reads the pointer-up event's
Shift; a Shift held from the press is no copy until let go; the entry with `as_copy` and with a
stale object; `classify_press` with Shift at the press on a knob still returns the knob, and the
selection is not toggled. In `curvyo-editor-wasm`: a drag end to end
(object count, selection, step name, undo); a preview frame with the original as itself, the copy
above it and no ghost; an entry copy whose limit notice shows the copy's value; size and node
handles unchanged under Shift (criterion 14).

**10. Risks.** The batch has no rollback, so a failed setter after the duplicate would
leave a plain copy under a "Copy with ..." label; the checks in 3 and the setters' own validation
must make that unreachable, and a test pins it. A copy of a clone source leaves its linked clones
on the original, and a text on the primitive stays on the original, as Duplicate does today.
