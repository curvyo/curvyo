# Plan for Grid

Status: Built on `story/0090-grid` (2026-11-01), amendment built 2026-11-01; drafted 2026-10-31, updated after the customer's answers to Frage 1 to 6 (grid per page) and Frage 7 to 9 (clip to page, active page only, new page copies the grid).

## Affected crates/modules

- `curvyo-document-core`: new `page_grid.rs` (`PageGrid` registers stored in each page's map in `pages.rs`, defaults, read/write, damage, copy on Add page); `snapping.rs` gets the `snapping_grid` target flag and the `snapping_grid_point` choice; `document.rs` `CURRENT_FORMAT_VERSION` takes the next free number at merge (30 after the rebase onto 0066 revision 4).
- `curvyo-ui-core`: new `grid/` module, pure functions: grid frame (origin, angle), adaptive level and fade (criterion 5), fixed thinning (6), log layout (8), `grid_lines(view, viewport, settings) -> GridLayout` bounded by the viewport; a grid target in `snapping/` that resolves the nearest intersection/line for a probe in grid coordinates (12 to 14) without an index (the grid is computed, not gathered).
- `curvyo-render-core`: grid line drawing under objects, over page backgrounds; major/minor weights.
- `curvyo-editor-wasm`: `session/grid.rs` (active page's grid writer, Shift+click toggle as one history step); `session/snapping.rs` passes the grid settings and current zoom into the resolver; grid snap point choice applied in Select move (`select.rs`), resize, Node drag (`snapping_node.rs`), Pen and shapes (`pen.rs`, `shapes.rs`, `placement.rs`); `wasm_document.rs` bindings.
- `frontend`: `GridSection.tsx` and `useGrid.ts` beside `SnappingSection.tsx`/`useSnapping.ts`; Grid entry in `PanelStrip.tsx`; Grid switch and snap point choice in `SnappingSection.tsx`; grid colour tokens in `docs/design-system.md`.
- No new crate, no new dependency.

## Tasks

1. [x] Page settings: `PageGrid` registers per page, offset relative to the page corner (survives page move and resize), copied on Add page, defaults, normalisation (angle mod 360), refusal of bad values, a current-format golden `grid_v<N>.curvyo`, malformed-register test. Criteria 1, 3 (core part).
2. [x] Snapping settings: `snapping_grid` (default on) and `snapping_grid_point` (default Centre) registers, round trip. Criteria 10, 11 (storage).
3. [x] Grid layout in `curvyo-ui-core`, test-first: level formula (25/5/1/0.2 mm at 20/100/500/2500 %), nesting of levels, fade between 4 and 12 px, fixed thinning below 4 px, rotation, log decades and adaptive log refinement, line count bounded by viewport. Criteria 5 to 8.
4. [x] Rendering in `curvyo-render-core` and the frame path: per page, clipped to the page rectangle, under objects, over backgrounds, never in export/print/eyedropper/hit test; CPU-time test (thread CPU, `tests/cpu_time/`) with 10,000 objects. Criteria 4, 9.
5. [x] Rail entry and panel: Grid entry next to Snapping (click opens, Shift+click toggles, blue when shown), `GridSection` controls for the active page (page name in the header), notices, history labels. Criteria 2, 3.
6. [x] Grid snap target in the resolver: intersections and lines of the shown level of the page grid containing the moving point (none on the pasteboard), grid coordinates for rotation, axis switches in grid axes, tie order after page fractions, hysteresis/Alt/marker reused from `0060`. Criteria 12 to 15.
7. [x] Grid snap point: Centre / Box corners and edges / Nodes for Select moves; dragged point for resize, Node tool, Pen and shapes. Criterion 11.
8. [x] Docs: README row and format table, `docs/requirements.md` R-EDIT-049, design-system grid colour, Snapping spec cross-link.

## Built (2026-11-01)

- Task 1, 2: `curvyo-document-core/src/page_grid.rs`, `snapping.rs`; tests `tests/grid_format.rs` (golden `grid_v30.curvyo`, seven damaged fixtures). `document.json` leaves a default grid and the default grid snapping out, like node registers (Proposal), so older exports stay byte-identical.
- Task 3, 6: `curvyo-ui-core/src/grid/` (`frame`, `level`, `log`, `layout`, `snap`, `tests`) and `snapping/grid.rs` (`resolve_with_grid`, `grid_snap_nodes`).
- Task 4: `curvyo-render-core/src/grid_lines.rs`, `build_page_areas_with_grids`; drawn only from `Session::frame_draw_list`. Budget: `curvyo-editor-wasm/tests/grid_budget.rs` (thread CPU, about 0.35 ms per frame at worst in a debug build).
- Task 5: `curvyo-editor-wasm/src/session/grid.rs`, the Grid tab after Snapping, `frontend/src/hooks/useGrid.ts`, `components/panel/GridSection.tsx`, `PanelRadioGroup.tsx`, Shift toggle in `PanelStrip.tsx`; frontend test `tests/grid.test.ts`.
- Task 7: `session/select.rs` (`move_grid_points`), `snapping_node.rs` (dragged anchor), creation and resize through `resolve_snapping_direction`.
- History label: one label `set_page_grid` ("Change grid") per ADR decision 1; the per-action labels of the UX notes are not built.

## Amendment tasks (2026-11-01)

9. [x] Format 30 after the rebase: goldens and damaged fixtures renamed and regenerated as `*_v30.curvyo`, format-version asserts, README format table (29 is `0066` revision 4).
10. [x] Frame cost: `pages_of_root` reads the grid registers in its one pass over the root map (hidden grids cost no keyed read); grid line ordering by a counting sort; per-level looks computed once per axis; `grid_budget` takes the cheapest of five runs. Criterion 9.
11. [x] Origin corner register replacing the offset, copy on Add page, damage on a malformed value; `Page::zero_point`, `measured`, `placed`. Criterion 16.
12. [x] Grid frame from the corner with axes into the page; rulers count from the active page's corner (`RulerDirection`, `ruler_layout_in`), repaint on `ruler_origin_key`. Criterion 17.
13. [x] Panel renamed Grid & Rulers (tooltip, accessible name "Grid and rulers"), four corner buttons under the page name, origin fields and Reset removed. Criterion 18.
14. [x] Typed move Absolute X/Y measured from the active page's corner (prefills, Enter and mark click). Criterion 19.

## Validation

- Unit tests per task in the crate (core logic test-first).
- Acceptance tests from the spec by the tester (format change → full pass).
- UX review of the rail entry, panel and grid look at several zooms.
- Full gate (`CLAUDE.md` §7) once before the push.
