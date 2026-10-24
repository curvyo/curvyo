# Plan for the inline delete confirm

## Affected crates/modules

No Rust change. Frontend only:

- `frontend/src/lib/deleteArm.ts` (new): the pure arm/disarm state machine (guard 400 ms,
  timeout 4 s, one armed target) and the armed and announced texts.
- `frontend/src/hooks/useDeleteArm.ts` (new): holds the one armed target of the Fonts tab, the
  4 s timer, the window listeners (pointer press elsewhere, Escape, focus leaving the zone) and
  the polite status text.
- `frontend/src/components/panel/FontTreeRows.tsx`: trash on Curvyo family rows, armed look,
  row notice slot.
- `frontend/src/components/panel/FontTagStrip.tsx`: the cell trash arms instead of the Inline
  confirm; armed look.
- `frontend/src/components/panel/FontsTab.tsx`, `frontend/src/lib/fontTreeKeys.ts`,
  `frontend/src/lib/fontRows.ts`: no Remove row; Delete arms, Delete/Enter on the armed row
  removes; disarm on list change; one live region.
- `frontend/src/hooks/useFontLibrary.ts`, `frontend/src/components/panel/FontTabNotice.tsx`:
  the TTC notice anchored at the family row.
- Deleted: `frontend/src/components/panel/FontRemoveRow.tsx`, `removePrompt`,
  `deleteTagPrompt`, the `remove` row kind.
- `frontend/src/index.css`: token `--destructive-soft`.
- Docs: `docs/design-system.md` (Inline delete confirm, token, Fonts tab rows, shortcuts),
  `specs/0075-font-library/specification.md` (criteria 5, 8, 25 wording).

## Tasks

- [x] 1. Failing node:test for the state machine and texts, then `deleteArm.ts` (AC 3, 4, 5,
  6, 11, 12, 16)
- [x] 2. `useDeleteArm` hook: timer, pointer/Escape/focus disarm, status text (AC 3, 5, 6, 11,
  13, 16)
- [x] 3. Family row trash and armed look; remove the Remove row and `FontRemoveRow.tsx`;
  keyboard Delete/Enter in the tree; focus after removal; list-change disarm (AC 1-9)
- [x] 4. Tag strip trash arms; armed cell look; keyboard Delete/Enter; focus to All after a
  keyboard delete (AC 10-15)
- [x] 5. TTC notice anchored under the family row (AC 8)
- [x] 6. Token `--destructive-soft`, 120 ms transition, none under reduced motion (AC 16)
- [x] 7. Design-system rows and the 0075 criteria wording (AC 17); other Inline confirm uses
  untouched (AC 18)

## Validation

- `npm test` (the state machine and texts), `tsc -b`, `oxlint`.
- Browser check with Playwright (Chromium): add a font to the Curvyo block, arm, wait 4 s
  (disarms), arm, press again (removed); the same for a tag cell; screenshot of an armed row.
