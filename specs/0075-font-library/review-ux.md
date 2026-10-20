# UX review: 0075 font library (re-review after the tags rework)

ux-engineer, 2026-10-20. Branch `story/font-library` at `8896915` (3 commits on `main`), built
with `npm run build` and run with `vite preview`. Driven live in Chromium (playwright-core) at
1280 x 800 and 800 x 600, against the UX notes and the tags amendment (A1 to A12) in
`specification.md`. The desktop paths (This computer block, the cuttable run, Add font from the
dialog) were checked in the same page with a mocked Tauri host listing 400 system faces.

**Verdict: CHANGES REQUESTED** (two MUST-FIX, both focus loss; one SHOULD-FIX).

## Checked and fine

- **Rows**: every family row is 52 px (name line 28, tag line 24), at both sizes. Chips, "+N" and
  Add tag sit on one line; Add tag is right-aligned 8 px from the edge. Chip order everywhere is
  Cuttable, Script, Sans, Serif, Initials, then own tags A to Z ignoring case ("birthday" sorts
  before "Engraving tests"). "+N" leaves room for itself, and its accessible name is "3 more tags, show
  all"; a press expands the family, which wraps to every tag (Lobster Two with 4 tags: 100 px =
  28 + 3 x 24, the faces start right under the last line, no gap). The row's name says
  "..., tags Script, Kids, Wedding" and the title lists "Tags: ...".
- **Tags**: the x is always shown, `--panel-muted-fg` at rest; on hover only the 18 x 20 zone takes
  `--editor-accent-hover` and the x darkens. On keyboard focus the whole chip has the 2 px
  `--editor-accent` ring. A mouse press on x keeps the focus where it was (canvas stays focused).
  Add tag turns the line into the field; typing "e" completes to "engraving tests" with the rest
  selected, Right takes it, Backspace drops it, "KIDS" adds the existing "Kids". Enter on a new
  name creates the tag and the strip shows it in the next frame. Validation: "All and Favourites
  are not tags." and "Use at most 24 characters." under the field, focus stays in it.
- **Keyboard**: F2 on a family row or on one of its face rows opens the field; Escape puts the
  focus on Add tag, a second Escape on the row. Shift+F2 lands on the last visible chip's x. Left,
  Right, Home, End move between x, "+N" and Add tag. Delete, Backspace, Enter and Space on an x remove
  the tag and the focus moves on to the next chip. Up and Down leave to the neighbouring tree rows.
- **Auto tags**: removing Script from Great Vibes hides it, also after a reload; Great Vibes then
  is not in Script. Adding it back shows the chip with "Set for the included fonts" again (the override
  is cleared). Initials and Cuttable added by hand say "Added by you".
- **Tag strip**: single choice; arrows choose (with a real key hold; Radix moves the focus in a
  timeout); a mouse press returns the focus to the canvas. With 7 own tags the strip stops at 4
  rows (115 px) and scrolls; the chosen cell is scrolled into view. Removing the last use of the
  chosen "Wedding" keeps it chosen and the row in place; after choosing All it is gone.
- **Tags button**: 28 px between the 180 px search field and Add font, pressed look while open. The
  list takes the font list's place, the search field is disabled at 40 %. Built-in rows reserve
  the 52 px; own rows have Rename and Delete. Rename validates ("A tag named Kids exists.", also
  for "Script") and renames the strip cell. Delete opens the inline confirm with Cancel focused.
  F2 and Delete do nothing on built-in rows. Escape closes the list and focuses Tags.
- **Earlier fixes**: the Tags list is one Tab stop (strip, Tags, Add font, list with it open;
  strip, search, Tags, Add font, list with it closed). "checking 13/400" shows in the This
  computer header above "No font in Cuttable yet.", and the Tags list count reads "0 · 75/400".
  No gap under the tag line (measured, above). Add font from the dialog with 3 files shows
  "Adding 3 fonts…".
- **Customer rules**: nothing appears without a press except the notices, the validation chip and
  the confirm; the only tooltips name controls or a tag's source. No hint labels.

## Fixed in this review (folded into the frontend commit)

1. `FontTagLine.tsx`: the add field's validation chip was clipped by the line's
   `overflow-hidden`, so Enter on "Favourites" or a 25-character name did nothing visible. The
   line drops `overflow-hidden` while the field is open.
2. `FontTagLine.tsx`: an expanded row's wrapped line started 2 px higher than a collapsed one
   (chips at 28 instead of 30). `py-0.5` on the wrapped line; 24 px per line still holds.
3. `FontTagLine.tsx`: the completion was drawn in the system selection blue; now
   `--panel-muted-fg` on no fill (amendment A4).
4. `fontLibraryText.ts`: "Its 1 font stay in Curvyo." now reads "Its 1 font stays in Curvyo."
   (test added in `tests/fontLibraryText.test.ts`).

## MUST-FIX

1. `frontend/src/components/panel/FontsTab.tsx:348`: Enter or Space on a focused Add tag, then
   Enter or Escape in the field, drops the focus to `<body>`. `onStartAdd` always passes
   `byKeyboard = false`, so `useFontTagLine.ts:311` keeps the Add tag button as the return
   target, and that button is replaced while the field is shown. Pass the press's `detail === 0`
   through `onStartAdd`, or in `closeAddTag` fall back to `[data-add-tag]` when the kept element
   is no longer connected.
2. `frontend/src/components/panel/FontTagsList.tsx:165`: confirming Delete on the last row of the
   Tags list drops the focus to `<body>` (Escape then no longer closes the list). `focusAt`
   clamps against the `rows` of the render before the delete. Clamp to the new length
   (`Math.min(index, rows.length - 2)`) or focus the row after the next render.

## SHOULD-FIX

1. `frontend/src/hooks/useFontTagLine.ts:345`: Delete on the last visible chip of a row with
   "+N" moves the focus to Add tag, though the next tag (from the overflow) now shows in its
   place. Take the next tag from the family's tag list, not from the line's controls.

## Screens

- `ux-screens/01-rows-strip-add-field-800x600.png`: strip with own tags (3 rows), rows with "+7",
  the add field completing "e" to "engraving tests".
- `ux-screens/02-tags-list-confirm-800x600.png`: Tags list open, search disabled, delete
  confirm for Kids.
- `ux-screens/03-expanded-row-chip-focus-1280x800.png`: desktop, Lobster Two expanded with
  wrapped tags, keyboard focus on the Script chip with its source tooltip.
