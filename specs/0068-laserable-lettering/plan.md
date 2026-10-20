# Plan for Weld (laserable lettering)

Stacked on `story/text-objects` (`0074`, PR #165): Weld converts texts with its conversion.
Branch `story/weld`.

## Affected crates/modules

- `curvyo-geometry-core`: `boolean.rs` (`union_region`, shared by Union and Weld),
  `offset_polyline.rs` (the `Paths64` offset helper `offset_region`), new `weld.rs` (union,
  gap closing, piece count) and `stroke_band.rs` (stroke band of open outlines).
- `curvyo-document-core`: `replace.rs`, `Document::replace_with_path_at` (place, style and name
  from different nodes; `replace_with_path` delegates to it).
- `curvyo-ui-core`: new `weld_plan.rs` (operands, contributions, plan, refusals) and
  `weld_gap.rs` (range of the Gap); `history_names.rs` ("weld" → "Weld").
- `curvyo-editor-wasm`: new `session/weld.rs`, `wasm_weld.rs`.
- Frontend: `components/WeldGroup.tsx` (button, glyph, Gap field, notices), `hooks/useWeld.ts`,
  `lib/weldText.ts`, the Select bar and `App.tsx`.
- Docs: this folder, `docs/design-system.md` rows, the specs index. CI: the weld tests join the
  `performance-budgets` job and the wasm golden run.

## Tasks

- [x] 0. Docs: spec Ready with the UX amendments, `adrs.md` with the build notes, UX notes in
      the spec, design-system rows (AC 1, 2, 5, Terms)
- [x] 1. `union_region` in `boolean.rs`; Union calls it (AC 30)
- [x] 2. `stroke_band.rs`: the band of open outlines with width, join, cap, miter limit 4 (AC 9)
- [x] 3. `weld.rs`: contributions, union, closing by g/2 out and in plus `∪ U`, cleanup,
      canonical form, pieces (AC 4 to 8, 14 to 18, 24, 29)
- [x] 4. `Document::replace_with_path_at`: place of a whole group, style of the lowest leaf,
      name, one commit (AC 19, 20, 22, 25, 26)
- [x] 5. `weld_plan.rs` and `weld_gap.rs`: leaves of groups, texts by the `0074` conversion,
      contributions per outline, stacking order, notice counts, refusals (AC 2, 10 to 13, 20,
      23, 27, 28)
- [x] 6. Session command and wasm surface (AC 1, 3, 19, 21, 22, 27)
- [x] 7. Frontend: Weld group in the Select bar, Gap field with storage, notices, busy state
      (AC 1 to 3, 10, 23, 27, 28)
- [x] 8. Golden and budget tests (AC 29, 31, 32), CI lines
- [x] 9. Full gate, frontend build and tests, wasm size

## Validation

- Kernel unit tests in `weld.rs`, one per criterion 4 to 9, 15 (property, 200 random sets, fixed
  seed), 16, 17, 28.
- Goldens: `curvyo-geometry-core/tests/weld_golden.rs` (AC 31 (c), (d); native and
  `wasm32-unknown-unknown`, AC 29) and `curvyo-editor-wasm/tests/weld_budget.rs` (AC 31 (a),
  (b), 32, Great Vibes from `frontend/public/fonts`). Budgets asserted in release.
- Session tests in `session/weld_tests.rs` (AC 1, 12, 19, 21, 22, 25, 27); plan test in
  `weld_plan.rs` (AC 13, 20, 29); document test `tests/replace_with_path_at.rs`.
- Frontend: `tests/weldText.test.ts` (AC 2, 10, 23, 27, 28).
- Not measured here (no browser in the build session): the Select bar rows of UX notes §8 and
  the width check of the Gap field; left for the UX review.

## Notes

- 2026-10-20: the spec number stays 0068 (the next free number is 0075).
