# Inline delete confirm: a trash can that asks by turning red

Status: Done (PR #182; Ready 2026-10-23, adrs.md and UX notes done)
Priority: Should
Origin: Customer. Request of 2026-10-23 (translated): "Remove from Curvyo fonts should simply be a trash can in the font row. I want the delete dialog gone. Instead (and we do this for tag delete too): the row turns a pleasant red and you have to press the trash can again." Timeout, guard time, disarm rules and keyboard keys are the product owner's and marked **(Proposal)**; they are safe defaults, vetoable at the demo.
Depends on: `0075-font-library` (Curvyo fonts block, family row, tag strip, Remove from Curvyo fonts criteria 25 and 26, tag delete criterion 8, UX amendment A7 and A8), `0045` and `0020` (the design-system row "Inline confirm", which stays for its other uses).
Changes accepted behaviour of `0075`: criteria 5, 8 and 25 (the Inline confirm and the Remove row are replaced, see "Changes to `0075`").

## User value

As a maker who keeps a few bought fonts in the Curvyo folder and a handful of own tags, I want to remove a font or a tag with the trash can right in its row, pressed twice, so that I do it in two presses at the same spot without a sentence and two buttons appearing, and still cannot remove something by one stray click.

**Field reference.** Most desktop tools ask with a modal dialog (Inkscape's font collection manager, LightBurn's font manager). Mobile lists use a two-step in place: swipe, then a red Delete in the same row (iOS, Android). Curvyo does the in-place two-step on the desktop: the same button is pressed twice, the row shows the armed state in red, and it falls back by itself after a few seconds. No dialog, no extra block, no text to read.

## Terms

- **Trash:** an icon button with Lucide `Trash2`.
- **Armed:** the state after the first press: the row (or strip cell) has the soft red ground and the trash its confirm look. Only one trash in the app is armed at a time.
- **Disarm:** going back from armed to the normal look without removing anything.
- **Soft red ground, confirm look:** design-system tokens the ux-engineer names (for example `--destructive-soft` for the ground; the confirm look = trash button filled `--destructive` with a white glyph).

## Acceptance criteria

### Part A: Remove from Curvyo fonts

1. Given the Fonts tab, then every family row in the **Curvyo fonts** block has a trash on its name line (place and size in the UX notes), always visible (no hover reveal, like the star), accessible name "Remove Lobster from Curvyo fonts", tooltip "Remove from Curvyo fonts". Family rows of the Included, This computer and Missing blocks have no trash (absent, not greyed). The same holds in the browser build for its Curvyo block.
2. Given an expanded Curvyo font, then there is no "Remove from Curvyo fonts" button row any more, and no press or key in the Fonts tab opens the sentence-and-two-buttons Inline confirm for removing a font.
3. Given a press on a font's trash, then the row is armed: the whole family row (name line and tag line, all wrapped tag lines when expanded) gets the soft red ground and the trash the confirm look; its accessible name and tooltip become "Press again to remove Lobster from Curvyo fonts." plus, when texts in the open project use the family from this block, " 2 texts here use it." (the count rule of `0075` criterion 25); a polite status message says the same for screen readers. Nothing is removed and no file is touched. The press does not pick the font, does not expand or collapse the row, and with the mouse the keyboard focus stays where it was (as the star).
4. Given an armed font row, when its trash is pressed again at least 400 ms after the arming press, then the family is removed exactly as `0075` criteria 25 and 26 say (its files deleted from the Curvyo folder, the TTC rule, star and tags kept, texts follow the font matching). A second press within 400 ms of arming is ignored and the row stays armed (a double-click never removes) **(Proposal: the 400 ms of the Inline confirm)**.
5. Given an armed row, then it disarms, with nothing removed and no notice, on any of these: a pointer press anywhere other than its trash (in the panel, on the canvas, on a bar); Escape; the keyboard focus leaving the row; 4 s (± 0.2 s) after the arming press without a second press; the list changing under it (search text typed, another strip cell chosen, the row collapsed or expanded, another tab shown, the panel collapsed). After disarming the row looks exactly as before arming. **(Proposal: 4 s and the list)**
6. Given an armed trash, when another trash is pressed (another font row or a tag cell of Part B), then the first disarms and the second arms; at most one trash is armed in the app.
7. Given the keyboard focus on a Curvyo font's family row (or any face row of that family, as `0075` UX §6 lets Delete act there today), when Delete is pressed, then that family row arms as by criterion 3 and the focus stays where it was; Delete or Enter pressed while it is armed removes it as by criterion 4 (400 ms guard included). Enter on a row that is not armed picks the font as today. Delete on a row of another block does nothing. Up, Down or any key that moves the focus off the family disarms (criterion 5). **(Proposal: Delete arms; "press again" is Delete or Enter)**
8. Given a font removed by the second press, then with the keyboard the focus goes to the next family row, else the previous one (as today); with the mouse the focus stays where it was. Notices are those of today: "Lobster is in a file with other fonts and was kept." for the TTC case (anchored at the row's trash instead of the old Remove row), nothing on a plain removal.
9. Given a font removed, then, as today, no history step is written and Ctrl+Z does not bring the file back (`0075` criterion 26); Add font… brings it back with its tags and star.

### Part B: Delete tag in the tag strip

10. Given an own tag's strip cell, then its trash (today's 20 x 20 `Trash2` beside the pencil, `0075` UX A7) is always visible, accessible name "Delete tag Wedding". Built-in cells, All and Favourites have no trash (as today).
11. Given a press on a tag's trash, then the cell is armed: the whole cell gets the soft red ground and the trash the confirm look; accessible name and tooltip "Press again to delete tag Wedding. Its 12 fonts stay in Curvyo." ("Press again to delete tag Wedding." when no font carries it); the polite status message says the same. The press does not choose the cell and nothing is deleted. The Inline confirm block across the strip (`0075` UX A7) is gone.
12. Given an armed tag cell, when its trash is pressed again at least 400 ms after arming, then the tag is deleted as `0075` criterion 8 says (removed from every family, listed or not, never a font file); if it was the chosen cell, the strip chooses All at once (`0075` criterion 6) and, when the delete was done by keyboard, the focus goes to the All cell.
13. Given an armed tag cell, then criterion 5 and 6 apply to it; a press on its pencil disarms and starts Rename as today.
14. Given the keyboard focus on an own tag's strip cell, when Delete is pressed, then the cell arms (focus stays on it); Delete or Enter while armed deletes as by criterion 12; arrow keys move and choose as today and disarm. On built-in cells, All and Favourites Delete does nothing.
15. Given a tag deleted, then, as today, no history step is written and no notice shows.

### Part C: one pattern

16. Given Part A and Part B, then both use the same armed look (the same ground token, the same confirm look of the trash), the same 400 ms guard, the same 4 s timeout and the same disarm rules; the armed state is shown by the filled trash (a shape change) as well as by colour. Contrast: row or cell text on the soft red ground at least 4.5:1, the white glyph on the filled trash at least 4.5:1, the filled trash against the soft red ground at least 3:1, in the light and the dark theme (if both exist when built). With `prefers-reduced-motion` the change is instant; otherwise any transition takes at most 150 ms.
17. Given `docs/design-system.md` after the build, then it has a row **"Inline delete confirm"** describing this pattern (trash, armed look, tokens, guard, timeout, disarm rules, keys) and naming its two uses (Remove from Curvyo fonts, Delete tag); the "Inline confirm" row no longer names them as uses, and the keyboard shortcuts table reads "Remove a Curvyo font: `Delete` on its family row, then `Delete` or `Enter`" and lists the tag delete the same way.
18. Given the other uses of the Inline confirm (Wipe history, Wipe object history, Delete format, Delete group of formats, the unsaved-changes check of `0051`), then they behave exactly as before. The x on a tag chip of a family row still removes that tag from that family at once, with no arming (`0075` UX A5).

## Changes to `0075` (applied by this story's PR)

- Criterion 5: "Its bin, or Delete while the cell has the focus, replaces the cell by an inline confirmation" reads "arms its trash (`0081`)".
- Criterion 8: the confirmation is the inline delete confirm of `0081`; its wording moves into the armed trash's accessible name and tooltip.
- Criterion 25: the Remove row is replaced by the trash on the family row; Delete on the family row arms it (`0081` criterion 7).
- UX notes §3 item 2 (Remove row), A7 row "Delete", A11 row "Remove row" and the shortcuts row "Remove a Curvyo font" are superseded by this spec.

## Out of scope

- Replacing the Inline confirm anywhere else (Wipe history, formats and format groups, unsaved changes) or adding a confirm to actions that have none (Delete layer, the tag chip x).
- Undo for removing a font or deleting a tag (neither is a document change; unchanged).
- Moving removed font files to the operating system's trash (`0075` out of scope).
- A trash on face rows, on families of other blocks, or for removing system or included fonts.
- Deleting several fonts or tags at once.
- A setting for the timeout or for switching the second press off.

## UX notes

In [ux-notes.md](ux-notes.md): the trash's place on the family name line and whether rows of other blocks reserve its slot, the soft red ground token and the confirm look in both themes, the status message wording for screen readers, where the TTC notice anchors, the design-system row "Inline delete confirm".

## Links

Requirements: R-FONT-006 (own fonts), R-FONT-002 (tags)
Depends on: `specs/0075-font-library/`
ADRs: none expected (frontend only; no document, file-format or crate change); `adrs.md` confirms.
PR: #182.
