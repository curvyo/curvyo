# Plan for Parameter handle clone (Shift during a knob drag makes a copy)

Implementer, 2026-11-26, branch `story/parameter-handle-clone`. Built under `specification.md` (criteria 1 to
18, Frage 387 answered by the customer 2026-11-26: Shift), `adrs.md` (note 2026-11-26) and `ux-notes.md`
(sections 2 to 6, 11). The key was switched from Alt (default A) to Shift during the build. No new crate, trait, `Document` method, dependency or `format_version` change. Own
choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-ui-core`:** `param_edit.rs` (`commit_param_copy`, `copy_label`), `transform_drag.rs` (live `copy_key` and
  `shift_lifted` on `TransformDrag`, `note_copy_key`, `param_copy`), `select_tool.rs` and `select_tool/param_copy.rs` (release
  of a copy drag, `set_copy_key`, `param_copy_in_flight`), `select_tool/preview.rs` (`LiveEdit.copy` for a knob copy), `select_tool/entry.rs`
  (`commit_param_entry_as_copy`), `param_entry.rs` (`commit_as_copy`), `param_copy_tests.rs`, `history_names.rs` and `step_scope.rs`
  (five labels).
- **`curvyo-editor-wasm`:** `session/select.rs` (Shift forwarded to the drag), `session/select_view.rs` (the dragged knob
  and the Rounded guide at the copy's value),
  `session/select_readout.rs` (" · copy"), `session/move_indicators.rs` (Copy badge),
  `session/transform_entry.rs` and `wasm_select_tool.rs` (`as_copy`, selection and limit notice on the copy).
- **`frontend`:** `TransformEntryChip.tsx` (Copy check), `useEditorSession.ts` (`as_copy`),
  `lib/paramEntryCopy.ts` (which chips have the check).
- **Docs:** `docs/design-system.md` (ux-notes section 11 rows), the 0073 spec, adrs and UX notes for
  Shift, `specs/README.md` row, spec status.

## Tasks

- [x] 1. Test first (ui-core): `commit_param_copy` for each family (curve, arc, corner radius linked and one
  corner, inner ratio, rounded): original unchanged, one new object with the value directly above it, one step
  with its label, one undo leaves the original only; the limited curve and corner radius on the copy; the
  copy's first object-history row is "Duplicate of" (3, 4, 8, 9, 10, 12, 13).
- [x] 2. `param_edit.rs`: `commit_param_copy` = `Document::batch(copy_label)` around
  `duplicate_objects(.., Vec2::ZERO)` and `commit_param` on the result re-targeted to the copy's id; labels in
  `OPERATION_NAMES` and `EDIT_STEPS` (4, 12).
- [x] 3. Test first (ui-core, select tool): `live_edit` is a copy exactly while the live Shift is set; Shift
  pressed mid-drag and released before the end commits a plain change; Shift down at the release commits the
  copy and selects it; the release reads its own Shift; a Shift held from the press is no copy until let go;
  under the dead zone nothing; Escape nothing; a resize under Shift unchanged; Shift at the press on a knob
  starts the knob drag and does not toggle the selection (1, 2, 4, 5, 6, 9, 14).
- [x] 4. `TransformDrag.copy_key` and `shift_lifted`, `SelectTool::set_copy_key`, preview and release (1, 3
  to 7, 9).
- [x] 5. Test first (ui-core, entry): `ParamEntry::commit_as_copy` with a typed value, a limited value, an
  untouched Enter (a copy at the current value), an invalid text and a stale object (no copy) (11, 16).
- [x] 6. Session (wasm): `modifiers_changed` forwards Shift to the drag; the dragged knob and the Rounded
  guide read the live copy; readout suffix
  " · copy"; Copy badge; entry with `as_copy` selects the copy before the announcement and limit notice (1, 5,
  7, 11). Session tests: drag end to end with undo, mid-drag Shift let go, Ctrl alone (no badge), preview frame (box on
  the original, knob at the copy's value, badge, readout), entry copy with its limit notice.
- [x] 7. Frontend: Copy check in the five parameter chips (last row, unset on every open, Tab loop field, Copy,
  field; Enter from both); `commit_transform_entry(.., as_copy)` (11, 17, 18).
- [x] 8. Design-system rows (ux-notes section 11), docs for Shift, spec status, gate.

## Validation

Unit tests in `curvyo-ui-core` (`param_edit.rs`, `param_entry.rs`, `select_tool` tests), session tests in
`curvyo-editor-wasm/src/session/param_copy_tests.rs`, a node test of `entryHasCopyCheck`, clippy and nextest of the
two crates, the frontend checks. The look (badge placement, chip row) is the UX review's.

## Own choices (Proposal)

- The live Shift reaches the drag through `Session::modifiers_changed`, which every pointer event and every key
  event already calls with the event's own flags (the pointer-up's `shiftKey` included), instead of a new
  argument of the preview functions: the signatures of `live_edit_snapped` and `select_pointer_up` stay.
- ui-core keeps `SelectTool::commit_entry` and adds `commit_param_entry_as_copy` (selects the copy, as
  `commit_move_entry` does) instead of a flag on `commit_entry`; the session and wasm exports take `as_copy`.
- An untouched Enter with Copy set duplicates under the "Copy with ..." label and writes no parameter.
- A Shift held from the press is no copy until it is let go and pressed again: at the press it chose the
  one-corner drag of `0013`, and a Shift+drag of a corner must keep writing that corner.
- The Copy check starts unset because the chip remounts on every open (no stored state).
