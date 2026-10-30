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
