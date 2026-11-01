# Plan for align and distribute

## Affected crates/modules
- `curvyo-ui-core`: pure align/distribute translations (`align.rs`), history
  names.
- `curvyo-editor-wasm`: `session/align.rs` (units, boxes, key state, commit),
  key mark in the Select decoration, wasm methods in `wasm_select_bar.rs`.
- `curvyo-render-core`: the key object's thicker outline in the Select
  decoration.
- `frontend`: `AlignGroup.tsx`, `useAlign.ts`, `alignText.ts`, Select bar and
  `App` wiring.

## Implementation choices (Proposal)
- Pure planning functions per the architect notes; per-unit translations
  applied with `ObjectSnapshot::translated`, one `transform_objects` commit
  inside one `recording` (one history step, selection kept).
- Reference resolution per press: key unit's box if still selected, else the
  selection union box; one selected unit uses the active page's rectangle.
- Distribute: stable ordering by the axis coordinate (paint order on ties);
  fewer than three units and every-zero plans write nothing.
- Key object: `Session` view state set by a Ctrl-click on an already selected
  unit (press time, so a Ctrl-drag copy still works), cleared by Escape,
  a second Ctrl-click, or leaving the selection; drawn as a twice-as-thick
  dashed outline.
- Gap field: frontend session state in millimetres; empty = keep the ends;
  invalid text and negative values are refused without distributing.

## Tasks
- [x] 1. Tests first: pure translations for all six aligns, both references,
  page reference, both distribute modes with and without a typed gap, ties,
  tolerances, no-ops (`curvyo-ui-core/tests/align.rs`) — criteria 2, 3, 4, 6,
  7, 8, 9.
- [x] 2. `align.rs` planning functions; history names for the eight labels —
  criteria 2, 6, 12.
- [x] 3. Session commands, unit and lock resolution, page reference, key
  state, Ctrl-click and Escape, one-commit writes, wasm methods — criteria
  3, 4, 5, 10, 11, 12, 13.
- [x] 4. Key outline in the Select decoration; frontend groups, glyphs,
  tooltips, gap field, App wiring — criteria 1, 5, 6, 8, 13.
- [ ] 5. Full gate once before the push.

## Page mode (customer, 2026-10-30, after the first demo)

Two more behaviours on the same buttons, no new UI:

- With **nothing selected**, the Align group aligns the **active page's**
  objects onto the page and the Distribute group spreads them evenly across
  it, horizontally and vertically, in both modes (criteria 1, 4 and 6 applied
  with the page as the set *and* the frame). `page_object_ids` picks the set
  (every pickable object whose true box overlaps the page, edges included; a
  group comes whole; hidden and locked objects never move) and
  `distribute_on_frame_translations` plans against the page's extent: equal
  gaps run from the page's start edge to its end edge, equal centres space
  the centres from edge to edge, a typed gap spaces from the start edge and
  may reach past the end.
- Both groups are in the bar in every selection state of the Select tool,
  hidden only when there is nothing to act on; the tooltip's rule line names
  the page (`ALIGN_PAGE_RULES`, `DISTRIBUTE_PAGE_RULES`).
- One history step, same labels, the empty selection stays: nothing new in
  `history_names` and nothing in the format.

## Validation
Core tests precede the implementation. Focused: `cargo nextest -p
curvyo-ui-core -p curvyo-editor-wasm`, `npm test` and `npx tsc -b --noEmit`
in `frontend/`, then the affected-crate clippy pass. The full gate once
before the push. No format change, so no golden files.

## Development evidence
- Pure plan: `curvyo-ui-core/tests/align.rs` 13/13 (worked examples, the
  references, no-ops, tied order, the typed gap).
- Session: `session/align.rs` 9/9 (groups follow selection/tool, one-step
  align and undo, page reference, key via Ctrl-click, Escape, distribute with
  and without a typed gap, a clone moving through its matrix while its
  original stays).
- Focused gates: `cargo fmt` clean; clippy `-D warnings` clean on
  ui-core, render-core and editor-wasm; nextest green: render-core/
  editor-wasm/ui-core 6196/6196, the remaining core crates 3168/3168.
  Frontend: tsc, oxlint exit 0 (existing warnings only), npm test 636/636
  (including the wasm goldens the rebuild repaired). Wasm rebuilt. The
  desktop shell (`curvyo-app`) depends on a built `frontend/dist`, which this
  clone has never produced — unrelated to the story, see the PR description.
- The key-object gesture (criterion 5) and the gap-field rules are the
  answers' defaults and are flagged Proposal in the spec, vetoable at the
  demo.

## Page reference and grid (2026-11-01)

Customer report (2026-10-31): "distribute on page and align on page don't
work yet". Reproduced in the browser on this branch before any change:
align with nothing selected and with one object worked, but **Distribute was
absent with one object selected and a no-op with two** (the session only
went to the page with an empty selection, and `distribute_group_shown`
needed two selected units), and the page distribute put the outer objects
onto the page edges instead of the n + 1 equal gaps of criterion 16, so two
objects were pushed to the page corners. The history said "Align left"
whether the page was the reference or not.

Choices (Proposal, vetoable at the demo):
- Align and Distribute are in the Select bar in every Select-tool state,
  the empty page included (criterion 15 "stay shown"; this overrides the UX
  line "hidden on an empty page").
- Distribute uses the page with 0, 1 or 2 selected units (criterion 16);
  a typed gap starts the first box that gap from the page's start edge.
- The grid group is shown from two selected units only (criterion 18; the UX
  notes' disabled-with-one and nothing-selected grid lines are not built).
- Reading order: top edge first, then left edge, then paint order, exact
  comparison (criterion 20, "top to bottom, then left to right"). Cells are
  the largest width by the largest height of the boxes.
- Rows/Columns linking and validation are the field's job (`gridText.ts`,
  tested by `npm test`); the session refuses too few cells as well.
- The page cue (UX notes "Canvas cue") reuses the key object's mark.

Tasks:
- [x] 6. Tests first, pure: page distribute with n + 1 equal gaps and n + 1
  equal centre parts, one object centred, typed gap from the page edge; grid
  placement, reading order, too few cells, no-op (`tests/align.rs`) —
  criteria 16, 19, 20, 21.
- [x] 7. `distribute_on_frame_translations` to the n + 1 rule;
  `grid_translations` and `arrange_selection_in_grid`; history names "Align
  left to page" (six), "Distribute horizontally on page" (two), "Arrange in a
  grid" — criteria 16, 17, 20, 21.
- [x] 8. Session tests through the real press path (draw, select, press):
  one object to the page on a page off the origin and on the second of two
  pages, all six aligns; nothing selected on two pages (other page and
  locked objects untouched, empty page no step); distribute 1/2/0+3; grid 6
  objects 2 x 3 gap 5 with undo as one step; page labels — criteria 14 to 17,
  20, 21.
- [x] 9. Session: page mode per count, groups always shown, grid shown from
  two, grid command, page cue; wasm methods — criteria 14 to 18, 21.
- [x] 10. Frontend: Align, Distribute and Grid as Select-bar groups after
  Guide with the bar's dividers, rule lines per mode, Grid button and the
  Rows/Columns/Gap fields (linking, refusals, Enter, Escape, reset on a
  selection change), page cue on hover and focus; `gridText` tests —
  criteria 15 to 19, 21, 22.
- [x] 11. `docs/design-system.md` rows; browser script
  `frontend/tests/browser/align-distribute.cjs` (page align, page distribute,
  grid) with screenshots in `target/0061-browser/`; focused gate.

Evidence (2026-11-01): `curvyo-ui-core/tests/align.rs` 21/21 (n + 1 gaps,
one centred, two in thirds, typed gap from the page edge, grid worked
example, idempotent second press, too few cells); `session/align_tests.rs`
16/16 (all six aligns to the active second page off the origin, nothing
selected on two pages with locked and hidden objects, empty page no step,
distribute 1/2/0+3, grid 2 x 3 gap 5 with undo, page cue); nextest
ui-core 2098, editor-wasm 4175, render-core 409 passed; clippy host and
wasm32 clean; `npm test` 670/670; tsc and oxlint clean; browser script
`frontend/tests/browser/align-distribute.cjs` PASS (screenshots in
`target/0061-browser/`). Open: the UX notes' Grid "disabled with one
object" and "nothing selected arranges the page" lines and the restore of
the grid fields on a press elsewhere are not built (criterion 18 hides the
group below two; the typed values survive a blur so the button can use
them).

## Review and acceptance rework (2026-11-01)

Architecture review changes 1-7 (`review-architecture.md`) and acceptance
findings F1, F3 (`acceptance-report.md`).

Choices (Proposal, vetoable at the demo):
- F1 reading order: boxes taken by their top edges; a box joins the current
  line when its top edge lies at or above the middle of the line's vertical
  extent so far; each line left to right by centre. A top-aligned row of
  different heights reads left to right.
- F3 (lead decision): the grid's corner is the selection box's top-left; when
  the boxes already form the grid with the same values (their union corner
  lies inside the first cells by the centring insets), nothing moves. So a
  second Grid press with unchanged values is a no-op.
- `grid_translations` keeps its gap as `f64`: the tester's acceptance tests
  call it with plain numbers. The public commit API takes `Length`.

Tasks:
- [x] 12. Failing test first: a unit without leaves in `objects` shifted the
  later units' translations (`tests/align_commit.rs`); `unit_boxes` returns
  units and boxes index for index (`BoxedUnits`) — review 2, criterion 2.
- [x] 13. Split `align.rs` into `align_plan.rs` and `align_commit.rs`;
  `MOVE_TOLERANCE` as `Tolerance`; gaps as `Length` — review 1, 5.
- [x] 14. History labels as `*_COMMIT_LABEL` constants in `align_labels.rs`
  with `AlignCommand::commit_label` and `distribute_commit_label`; no label
  parameter on the commit functions — review 3, criteria 12, 17.
- [x] 15. `AlignReference`, `align_reference`, `distribute_reference` in
  `curvyo-ui-core`; `align_selection` without the page branch — review 4,
  criteria 3, 14 to 16.
- [x] 16. Unknown wasm command codes run nothing
  (`align_command_from_code`, `distribute_from_code`) — review 6.
- [x] 17. Session tests split into `align_tests.rs`, `distribute_tests.rs`,
  `grid_tests.rs` with `align_test_support.rs` — review 7.
- [x] 18. F1 and F3 in `grid_arrange.rs` with tests in `tests/align.rs` —
  criteria 20, 21.

Evidence: nextest ui-core 2108 passed (+ F1's ignored tester test passes
with `--run-ignored`), editor-wasm 4193 passed; clippy `-D warnings` host and
wasm32 clean; fmt, cargo doc clean; frontend build:wasm, tsc, `npm test`
670 pass, oxlint exit 0; browser `align-distribute.cjs` and
`align-distribute-journey.cjs` PASS.

## UI review (2026-11-01) changes

- [x] 19. `--field-derived-fg` (`#767680`, 4.49:1 on white; dark `#9A9AA2`) for
  the derived Rows/Columns placeholders (`BarField` `derivedPlaceholder`) —
  review item 1, criterion 19.
- [x] 20. Distribute glyphs redrawn as 1.5 px outline bars with dashed guides
  — review item 2, criterion 6.
- [x] 21. Narrow bar: groups wrap whole; Distribute and Grid wrap their fields
  below the buttons only when one alone is wider than the bar (700 px window,
  bar 224 px), so no field leaves the bar — review item 3.
- [x] 22. Grid fields 44/44/72 px; Linked clone wraps with the Convert group
  (`ConvertAndLinkedClone`), never alone on a trailing row. With a rectangle
  selected the 1440 px bar stays three rows (Radius group), the third holds
  Convert and Linked clone — review item 4.
- Item 6: `GRID_NOT_ENOUGH_CELLS` still has callers (`gridPlan` with both
  counts typed, the session's `too-few-cells` outcome), unreachable from the
  UI; kept per the lead's rule (delete only without a caller).
- Item 7: key object verified in the browser with three filled rectangles:
  Ctrl-click on a selected one (off the selection-centre handle) draws the
  thicker accent outline, Align left edges then keeps it and moves the others
  to its left edge. A Ctrl-click on the centre handle hits the handle, not the
  object (likely the reviewer's miss).

Evidence: fmt clean; frontend tsc, `npm test` 670 pass, oxlint exit 0;
browser `align-distribute.cjs` and `align-distribute-journey.cjs` PASS; bar
measured at 700/1100/1440 px with no control outside the bar; screenshots in
`target/0061-browser/ux-fixed/`.
