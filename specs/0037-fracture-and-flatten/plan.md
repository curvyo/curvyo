# Plan for Fracture and Flatten

Stacked on `story/pen-angle-snap-and-typed-values` (0072, PR #159).

## Affected crates/modules

- `curvyo-geometry-core`: `boolean.rs` (the operand checks and grid regions shared with the new
  commands), `boolean_cleanup.rs` (sliver test reachable), new `stack_regions.rs` (what both
  commands share: `StackError`, boxes, "has area", cutting a cell into connected pieces), new
  `fracture.rs` (`fracture_cells`), new `region_contact.rs` (a cell against a region without an
  overlay), new `visible_region.rs` (`visible_regions`).
- `curvyo-document-core`: `replace.rs`, `replace_with_pieces` accepts removed objects with no piece
  at all (Flatten that only hides objects), as `adrs.md` says ("0037 adds only the case removed
  object with no piece").
- `curvyo-ui-core`: new `stack_plan.rs` (stack, refusals, plan, the 5,000 limit), `fracture.rs`
  and `flatten_command.rs`;
  `boolean.rs` shares its operand helpers crate-wide.
- `curvyo-editor-wasm`: new `session/fracture.rs` (both commands, one write each) and
  `wasm_fracture.rs`.
- `frontend`: buttons six and seven of the Boolean card, glyphs, tooltips, notices.
- CI: the fixture budgets run in the `performance-budgets` job.

## Tasks

- [x] 1. Spec: UX re-check of 2026-10-19 applied, stale lines dropped.
- [x] 2. Kernel tests first, then `visible_regions` (AC 3, 10, 11, 15, 16).
- [x] 3. Kernel tests first, then `fracture_cells` with the piece cut (AC 3, 6, 7, 8, 15, 16).
- [x] 4. Fixture budgets (a), (b), (c) and the area property (AC 7, 17).
- [x] 5. `replace_with_pieces` with no piece for a removed object (AC 11).
- [x] 6. `ui-core` plans: operands by stacking order, refusals of criterion 2 (groups are the
  session's), no overlap, more than 5,000 pieces, piece order, kept objects, hidden count, stroke
  flag (AC 2, 3, 4, 6, 8, 10, 11, 13, 16).
- [x] 7. Session commands and wasm surface: one commit `fracture` / `flatten`, selection
  afterwards, refusal outline (AC 2, 5, 9, 12, 14).
- [x] 8. Frontend: two buttons in the Boolean card, glyphs, tooltips, notices (AC 1, 5, 9, 11, 13).
- [x] 9. Full gate.

## Validation

Kernel unit tests for the examples of criteria 7, 10 and 11; an integration test with the
generated fixtures (a) and (b) and the `0016` fixture files (c), timed in release in the
`performance-budgets` job; a seeded random-squares check that the pieces tile the union and
the visible regions tile the union. Session tests for the commit, the selection and the notices'
counts. Frontend unit tests for the sentences.

## Notes

- **2026-10-19: one `StackError` for both kernel functions**, wrapping `BooleanError` (operand
  checks shared with `boolean`) plus `TooManyPieces`. The fold stops as soon as it holds more
  cells than the caller's limit, so a stack whose cells explode is refused (criterion 4) and
  does not hang. Flatten never returns `TooManyPieces`.
- **2026-10-19: Flatten folds from the top with the running union of `adrs.md`**: visible =
  O − U, then U := U ∪ O; a box test against U is the shortcut for "untouched". (The first build
  subtracted every operand above whose box met, which the review of 2026-10-19 measured as
  quadratic on nested shapes: 500 concentric circles 11.5 s, now 0.1 s.)
- **2026-10-19: Fracture's fold skips the overlay for a cell whose boundary does not meet the new
  operand's** (`region_contact.rs`: exact integer segment tests through a strip index, then one
  winding test per outline tells "apart" from "inside"), and takes the rest of the new operand
  as O minus the running union. 500 concentric circles: 0.65 s with the largest at the bottom,
  1.0 s with it on top (4.2 s before); the fold is still quadratic in the number of nested
  shapes, linear for side-by-side ones. Both cases run in the release budget test.
- **2026-10-19: "untouched" for Flatten** is "nothing above overlaps it in area"
  (criterion 10: the top shape of an overlapping pair is untouched); for Fracture it is "overlaps
  no other operand" (criterion 6).
- **2026-10-19: the 5,000 limit counts the new objects** (pieces for Fracture, trimmed objects for
  Flatten), not the untouched ones that stay.
- **2026-10-19: notice counts.** Fracture: "N objects became M pieces." with N the operands that
  took part (removed) and M the pieces. Flatten: "N objects became M." with N all operands and
  M = N minus the hidden ones.
- **2026-10-19: open outlines** are closed with a straight line and counted, as for the Boolean
  operations (`0054` criterion 24, status line of the spec); criterion 2's open-path refusal no
  longer applies.
