# ADRs for 0073 parameter-handle-clone

- `0002` (document model, units and SVG round trip): a copy is an ordinary object made by the
  Duplicate path (`specs/0010-edit-interaction-polish/adrs.md`, decision 2); no new node, register
  or meta key, no `format_version` change.
- `0014` (history, undo and branches): the copy and its new value are one step under one label; an
  undo removes the copy and leaves the original.
- The key is the customer's open question COPY-KEY (Frage 387). Default A: Alt pressed DURING the
  drag makes a copy and Alt's state at the RELEASE decides; Alt at the press still arms the lasso
  and nothing existing changes. B: Alt at the press (needs the lasso carve-out). C: Ctrl. The note
  below is written for A; section 8 says what B and C change. It decides no key.

## Note 2026-11-26: a parameter drag or entry that writes a copy

No new ADR, crate, trait, `Document` method or format change. Everything sits on code that
exists: `Document::batch`, `Document::duplicate_objects`, `param_edit::apply_param` and
`commit_param`, and the copy preview of `LiveEdit.copy` and `DragSwap`.

**1. When the key is read (Proposal, default A).** Not fixed at the press: the copy flag is read
live. `TransformDrag` gets one field, the live Alt state (`alt: bool`). It is updated by every
pointer move and by Alt keydown and keyup during the drag (each redraws, also with the pointer at
rest), and by the pointer-up event's own `altKey` flag at the release, so a lost keyup cannot turn a
visible copy into a change. The preview (`live_edit_snapped`) and the commit
(`pointer_up_snapped_with_artwork`) both read that one field, so what was drawn in the last frame is
what is written (one rule, as the "one resolving function per gesture" of `TransformDrag`). Alt is
plumbed as an argument next to Shift and Ctrl; today `select_pointer_up` takes only those two and
reads Alt from `held.alt`, which may be stale. Alt held at the press still arms the lasso
(`press.rs` `arms_lasso`), so the press, `classify_press` and the cursor are unchanged. Alt
pressed mid-drag and released before the end writes a plain change, no copy. This follows the
customer's rule "state of the frame decides what is drawn, state at the release what is written"
(`ux-notes.md` sections 1, 2). Consequence for the spec: criterion 1 reads "Alt held at the release,
pressed at any time during the drag" (K5); for the PO through the lead.

**2. Preview.** A copy drag returns `LiveEdit { copy: true }` with the resolved object under the
original's id, as a move copy does. `DragSwap` then keeps the original drawn as itself, draws the
copy directly above it and draws no ghost; this is the look Duplicate's result will have.
Decorations: `live_objects_in` substitutes nothing for a copy, so the box and the other knobs stay
on the original, which is what `ux-notes.md` section 3 (V6) asks for; the dragged knob and its
readout are drawn at the copy's value (the " · copy" suffix is a frontend/readout text), so the code
change is limited to the dragged knob's position in `live_objects_in`.

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

**5. Entry box (unchanged by the key).** `ParamEntry::commit` and `SelectTool::commit_entry` take an `as_copy` flag (labelled "Copy" in the
UI, `ux-notes.md` V3) and
hand back the copy's id; the session selects it. The check's state lives only in the DOM chip and
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

**8. What only one key changes.**

| Part | A (default): Alt during the drag | B: Alt at the press | C: Ctrl |
|---|---|---|---|
| Copy flag | live (`TransformDrag.alt`), release decides | `copy` fixed at the press in `begin_handle_press` | live like A, Ctrl's flag |
| `classify_press` | unchanged (Alt press = Lasso, also on a knob) | parameter-knob hit before the Alt lasso clause | none (handles already come before the Ctrl marquee clause) |
| Hover cursor (`select_gesture.rs`, `arms_lasso`) | unchanged | knob over lasso | none |
| Accepted behaviour | nothing changes (`0009` criterion 35, `0014` criteria 16 and 17 stay) | Alt on a knob no longer starts a lasso: a customer decision, covered by COPY-KEY | none |
| Snap during a copy | Ctrl still snaps; the preview passes raw Ctrl and raw Alt | as A, after the press | Ctrl is the copy flag and can no longer snap: the snaps lose their key, the replacement key is the customer's call |
| Spec texts to amend | `0073` criterion 1 only | `0009` criterion 35, `0014` criterion 16 | `0021` criteria 7, 23 and 41, `0145` §3 |

**Bug warning (preview):** `select_view.rs` passes `ctrl && !alt` to the preview today. For a knob
drag it must pass raw Ctrl and raw Alt; otherwise the Alt+Ctrl curve or arc drag previews an
unsnapped value and commits a snapped one.

**9. Tests.** In `curvyo-ui-core`: for each family, original unchanged, one new object with the
value directly above it, one step with its label, and one undo leaves only the original; a limited
corner radius and curve on the copy; an unchanged value as a copy adds no step; the copy's first
object-history row is "Duplicate of" the original; `pointer_up` selects the copy; `live_edit` is a
copy exactly while the live Alt state is set: Alt pressed mid-drag shows the copy in that frame,
Alt released shows the change again; Alt pressed mid-drag and released before the end commits no
copy (one changed original, no new object); Alt pressed mid-drag and down at the release commits
the copy; the release reads the pointer-up event's Alt, not a stale `held.alt`; the entry with
`as_copy` and with a stale object; `classify_press` with Alt at the press still returns Lasso on a
knob, a resize handle, an object and empty canvas. In `curvyo-editor-wasm`: a drag end to end
(object count, selection, step name, undo); a preview frame with the original as itself, the copy
above it and no ghost; an entry copy whose limit notice shows the copy's value; size and node
handles unchanged under Alt (criterion 7); an Alt+Ctrl curve drag whose preview value equals the
committed value.

**10. Risks.** Alt+drag is taken by some Linux window managers in the desktop app at the press;
under A only a mid-drag Alt is needed, so the press is never affected. Alt keydown/keyup during a
knob drag are `preventDefault`ed against the webview menubar (`ux-notes.md` section 2). The batch has no rollback, so a failed setter after the duplicate would
leave a plain copy under a "Copy with ..." label; the checks in 3 and the setters' own validation
must make that unreachable, and a test pins it. A copy of a clone source leaves its linked clones
on the original, and a text on the primitive stays on the original, as Duplicate does today.
