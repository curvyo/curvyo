# Plan for library into slots (slice 1: copy into a slot)

Implementer, 2026-11-25. Slice 1 at the defaults: Frage 377 C slice 1 (a copy only), 378 B (Fill
empty slots), 379 A (Fit, Alt keeps the real size), 385 C (a filled slot refuses, armed Replace).
The live placed page (377 C slice 2) is the seam of ADR 0088 decision 7 only: nothing is built
for it.

## Affected crates/modules

- `curvyo-laser-core::fixture_fit`: `FIT_TOLERANCE`, `FitMode`, `SlotFit`, `fit_into_slot`,
  `slot_fit_matrix`; `fixture_slot`: `Slot::holds`.
- `curvyo-document-core::fragment_insert`: `FragmentTarget::Group`, `ImportError::TooDeep`.
- `curvyo-ui-core`: `map_uniform` (beside `flip`), the slot texts (`slot_drop_text`), the three
  step labels in `history_names` and `step_scope::FREE_STEPS`.
- `curvyo-editor-wasm::session::fixtures`: `into_slot` (targets, filled, `put_into_slot`,
  `fill_empty_slots`), `into_slot_cue` (drag cue, ghost draw), `into_slot_view` (the Into slot
  list); `fit.rs` takes `FIT_TOLERANCE`; Layers " · Slot n"; Fixtures tab "filled".
  `wasm_into_slot.rs` for the bindings; `tests/cpu_time/` for the fill budget.
- Frontend: `lib/libraryDrag.tsx` (slot cue, Alt, Shift), `lib/intoSlot.ts`, `IntoSlotList.tsx`,
  the Library tab row, the page preview footer, the browser detail bar, the canvas notice with the
  armed Replace, Layers and Fixtures rows.

## Tasks

- [x] 1. laser-core fit: `fit_into_slot` and `slot_fit_matrix`, table test of the criterion 6
  cases, the tolerance boundary, the turn gain, a zero-width unit, Alt with and without overflow;
  `Slot::holds` (AC 6, 16)
- [x] 2. document-core `FragmentTarget::Group` (wrap into one group named after the source page,
  Came from on the group, copies keep their own), `ImportError::TooDeep` (AC 2, 3, 14)
- [x] 3. ui-core `map_uniform` (scale and turn of a fragment's private copy, strokes and radii
  scaled), the notice and badge texts, history names and step scope (AC 4, 5, 6)
- [x] 4. session: targets, computed "filled", `put_into_slot`, `replace_slot_content`,
  `fill_empty_slots` (one undo step each, selection after, refusal of a filled slot), the drag cue
  (`begin_slot_drop`, `slot_drop_cue`, `end_slot_drop`), the list view `slot_targets_view`, the
  selected target slot (AC 1, 4 to 12, 17)
- [x] 5. golden test in `tests/fixtures/`: a holder with 30 x 30, 60 x 40, circle 25 and a
  damaged slot, a 60 x 40 library page: drop, fill, filled refusal and Replace, turned, circle,
  Alt, undo, save and reopen (AC 14, 16)
- [x] 6. thread-CPU budget: fill of 50 slots with a page of 200 objects within 100 ms
  (`tests/cpu_time/`, `budget_factor`) (AC 8)
- [x] 7. wasm bindings and frontend: drag over a slot (light, fitted ghost, badge with scale, Alt,
  Shift fill, filled refusal), the notice with the armed Replace, the Into slot… inline list in the
  Library tab row, the page preview footer and the browser detail bar, double-click and Enter into
  a selected slot, Layers " · Slot n", Fixtures tab "filled" (AC 1, 7 to 10, 13, 17)
- [x] 8. full gate; spec status stays Ready until the PR (the lead) (AC 15: no network, no I/O)

## Notes from the build (2026-11-26)

- Task 6, ADR 0088 decision 9: the insert path stays; the budgets are 50 x 200 within 500 ms,
  15 x 200 within 150 ms and the Into slot list of 50 filled slots within 50 ms
  (`tests/into_slot_budget.rs`, release). The quadratic "filled" read is fixed: the walk to the
  top-level object reads the placed pages once per call (`top_level_among`).
- Built after the first review: a drop over a slot of another laser page activates it
  (criterion 9), icons and one-page templates as sources (V1; a template is read on the release),
  the content outlines in the fitted ghost (V2, box alone past 2,000 leaves), a filled slot hides
  its name (V16), the holder line "· 1 filled" (V18), the polite status of the drag, and the
  Into slot list pushed into the tab's tree, above the preview footer and the detail bar (V20).
- Not built: the page thumbnail inside the ghost (the content outlines stand for it), the
  `LoaderCircle` while a template downloads on the release.

## Validation

Table-driven unit test of the fit in laser-core; session tests per command (one undo step, undo
restores, selection after); a golden project file for save and reopen; the thread-CPU fill budget;
`tsc`, `vitest` and `oxlint` for the frontend; a manual drag in the browser build by the lead.
