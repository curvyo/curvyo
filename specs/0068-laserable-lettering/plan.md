# Plan for Weld

Implementer, 2026-10-20, branch `story/weld-v2` from `origin/main` at `6bf6540`. Built under
`adrs.md` of 2026-10-20 (the reworked definition, Frage 15 WELD-DEF and Frage 14 WELD-FILL A). The
rejected branch `story/weld` (PR #167) only lends the pieces `adrs.md` decision 9 keeps. Frage 16
WELD-MINH is built with its default A (window minimum height 640). One PR for the slice, 1 to 3
self-contained commits.

## Affected crates/modules

- **`curvyo-geometry-core`:** `weld.rs` (new: the fold from the top with the string clip, the
  inner-line overlay, `Welded`, `weld`), `weld_lines.rs` (new: the chaining on the grid),
  `boolean_cleanup.rs` (`cleanup_open`, the open form of `remove_near_collinear`),
  `stack_regions.rs` (`GridBox::touches`, the inclusive box test the string clip needs). Fixtures
  (c) to (e) with golden file and budgets in `tests/weld_golden.rs`; (a) and (b) need fonts and
  run through the session in `curvyo-editor-wasm/tests/weld_budget.rs`.
- **`curvyo-document-core`:** `replace.rs` `replace_with_path_at` (place and style from the
  caller; no `name`).
- **`curvyo-ui-core`:** `weld_plan.rs` (operands through groups at any depth, texts one glyph per
  operand, base, line-art style, place, refusals, notice counts), `history_names.rs` (`weld`).
- **`curvyo-editor-wasm`:** `session/weld.rs` (`apply_weld()`, `weld_availability()`),
  `wasm_weld.rs`.
- **`frontend`:** `stackText.ts`/`weldText.ts` (names, rule, note, notices), `BooleanGlyphs.tsx`
  (glyph), `BooleanCommands.tsx` (button 8 with its own predicate), `useRailCommands.ts`,
  `useEditorSession.ts`, `booleanText.ts` (`of === 1` refusal branch).
- **`curvyo-app/tauri.conf.json`:** `minHeight` 600 to 640 (Frage 16 default A).
- **CI:** the `performance-budgets` job and the wasm golden run get `weld_golden`.

## Tasks

- [x] 1. Docs: spec and `adrs.md` replaced, UX notes inserted, criteria 5 and 21 aligned with them,
  status Ready, design-system rows, `specs/README.md` row (Milestone 0).
- [x] 2. Kernel: pin the `ClipRule` semantics, the fold with string clips, hidden test, inner-line
  overlay, chaining, cleanup of open and closed lines, result order (AC 4 to 10, 12, 14 to 16, 25,
  26).
- [x] 3. Property test, 200 random sets, fixed seed, 0.002 mm (AC 14).
- [x] 4. `replace_with_path_at` without `name`, with the style from the caller (AC 17 to 20).
- [x] 5. `plan_weld`: operands, glyphs, open outlines, base style 14 A, place, refusals, counts,
  line length (AC 11, 13, 17, 18, 21, 23, 24).
- [x] 6. Session and wasm: `apply_weld()`, `weld_availability()`, one commit `weld`, selection,
  undo (AC 1, 3, 19, 20, 22).
- [x] 7. Frontend: button 8, glyph, tooltip, notices, refusals (AC 1 to 3, 21, 23, 24).
- [x] 8. Fixtures (a) to (e), goldens native and wasm, budgets in the `performance-budgets` job
  (AC 25, 27).
- [x] 9. Window minimum 640 (Frage 16 default A).
- [x] 10. Full gate, frontend build and tests, wasm size.

## Validation

- One focused test per criterion; the criterion 14 property test over 200 sets with a fixed seed.
- Golden files for fixtures (a) to (e), compared natively and on `wasm32-unknown-unknown` at 1e-6 mm.
- Budgets in release builds in the `performance-budgets` job.
- Frontend unit tests for the sentences; a manual look at the glyph and the rail at 800 x 640.

## Notes

- 2026-10-20: the kernel's dated notes (noding against the known limit, short lines, the fold over
  shapes, `ClipRule`) are in `adrs.md`, "Implementation notes".
- 2026-10-20: `frontend/tests/stackText.test.ts` (0037, implementer-written) checked
  `BOOLEAN_CARD_COMMANDS.slice(5)`; it now checks `slice(5, 7)`, buttons six and seven, as its
  name says.
- 2026-10-20: `sharedRefusalText` gets the `of === 1` branch for every command (UX notes,
  "Refusals").

