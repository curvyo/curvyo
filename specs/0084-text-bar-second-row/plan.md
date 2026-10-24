# Plan for the Text bar second row and Font jump button

Implementer, 2026-10-24, on `story/text-bar-second-row`, stacked on `story/text-vertical`
(`0079` slice 3, PR #187). Lead decision: the UX notes §0.1 (Glyphs on row 2), §0.2 (Font and
Size 4px apart) and §0.3 (double chevron) are applied to the criteria as Proposal. §0.5 (Tab
order in the Select bar follows what is seen) is applied too, since it follows from criterion 8.

Frontend only: no Rust, no session, no document, no `format_version` change.

## Affected modules

- `frontend/src/lib/textBarRows.ts` (new, "words and the stored state of the Text bar's two
  rows"): storage key `curvyo.text-bar-row2`, guarded read and write, the Font button's
  accessible name and tooltip lines, the expand button's name, the changed-control list and its
  description.
- `frontend/src/components/TextBar.tsx`: Font icon button, row 1 / row 2 split, `ExpandButton`,
  `Row2` container, Font and Size 4px apart, Size notice above row 2.
- `frontend/src/components/SelectToolbar.tsx`: Text controls split (row 1 in place, row 2 at the
  bottom of the pill), shared `Divider` drawn absolutely (aligned left edges).
- `frontend/src/components/StyleGroup.tsx`: unchanged (its notice is already `z-30`).
- `frontend/src/App.tsx`: one row-2 state for both bars, read once at startup.
- `docs/design-system.md`: rule "Bar overflow: expandable second row, never a dropdown icon";
  rows "Font button", "Text bar", "Select bar Text group", "Writing mode and Glyph orientation
  groups", "Select bar layout" (divider).

## Tasks

1. [x] Tests first for `textBarRows.ts`: key, closed by default, read back, unknown value and
   throwing storage mean closed; Font name with face, Mixed and missing; tooltip lines; expand
   names; changed list and description (criteria 2, 3, 11, 13, 14).
2. [x] `textBarRows.ts` until the tests pass.
3. [x] Font jump button: 28 x 28, `Type` / `TriangleAlert`, name from task 2, tooltip lines; the
   press reuses the `0074` command (criteria 1 to 5).
4. [x] Divider drawn absolutely 13px left of its group, 25px flex gap, so wrapped lines start at
   the pill's padding (UX notes §2, Left edge).
5. [x] Text bar: row 1 (Font and Size, Alignment, Style, expand), row 2 (Writing mode, Glyph
   orientation, Glyphs) in its own `role="toolbar"` container, `w-0 min-w-full`, separator, not
   rendered while closed (criteria 6, 7, 10 to 13, 16, 17).
6. [x] Select bar: Text row 1 controls in place in the order of criterion 6, row 2 as a
   `role="group"` line at the bottom of the pill (criteria 8, 12).
7. [x] App: one state for both bars, written on toggle; mouse press returns focus, keys keep it
   (criteria 14, 15).
8. [x] Measure in the browser build at 800 x 600 and 1280 x 800, with stand-ins for Taper
   (120px) and On path (28px) (criteria 18 to 20); screenshots in the scratchpad.
9. [x] Design system rule and rows with the measured line counts (criteria 21 to 23).

## Validation

- `npm test` (node:test) for the pure logic of task 1.
- `tsc -b`, `oxlint`.
- Playwright against `npm run dev`: line counts (bar height / 36), left edges of wrapped lines,
  row 1 control positions before and after the toggle, `aria-expanded`, row 2 absent from the DOM
  while closed, the preference across a reload.

## Open points

- UX notes §0.4 (missing with Mixed, "1 of 3 fonts missing"): built. The session's Text bar view
  counts the subject faces and the missing ones (`faces`, `missing_faces`); `missing` is true when
  any subject face is missing.
- Tester finding c17: notices under bar controls are shifted into the bars' area
  (`data-bar-area`, `useNoticeShift`), so the tool rail never covers them.
- Taper (`0079` slice 2) and On path (`0080`) are not built: their places are defined, the fit is
  measured with stand-ins.
