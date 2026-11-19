# 0123 Raster engraving drafts (ux-engineer, 2026-11-18; Image kind 2026-11-19)

Static screens for `specs/0123-raster-engraving/`; real tokens, the 280px panel, PNG beside each HTML.
A dashed outline in a draft marks a new or changed control. The notes are
`specs/0123-raster-engraving/ux-notes.md`; proposals there are P19 to P46, spec deltas E1 to E13.

**Current (after the customer's feedback on PR #394: a separate Image kind, Dither as a strip):**

- `04-image-operation-row`: the unfolded Image row (Floyd-Steinberg default) with the Dither choice
  strip in two fixed rows right under the set; Threshold chosen by the arrow keys with the mode field as the last row;
  Grey with Lowest power and line 3 "Grey 10–35 %"; a damaged dither value; the Engrave row as `0111`
  built it (hatch only).
- `05-image-kind-and-canvas`: the Select bar's six kind buttons with the Image tooltip and the glyph
  sheet at 1x and 2x; the Operations view with dots, the wrong-kind look (dimmed, dashed, muted chip)
  for an image in Engrave and a Colour-filled shape in Image, a gradient shape hatched in Engrave; the
  two new check lines; a linked clone turned 20° with its own dots and the `0049` marks; the pending
  first build (pale orange frame and chip).
- `06-test-grid-image-kind`: the Test grid bar with the fourth Kind segment Image and the six
  Dither segments (no menu, no Fill), the grid's cells as dots; the bar in the smallest window; the Engrave grid
  as `0112` built it.

`kit.css` and `kit.js` hold the shared tokens, Lucide paths, kind glyphs, row builders and the dither
routine (Floyd-Steinberg, Jarvis, Atkinson, Stucki, Ordered 8 x 8 Bayer, Threshold, Grey) at draft
scale, so the dot patterns are real.

**Superseded (kept for the record):**

- `01-engrave-row-image-settings`: the Image settings in the Engrave row and the Dither pick with its
  inline list are replaced by screen 04 (Image kind, strip). Screen E (Engrave without the group) still
  holds.
- `02-operations-and-toolpath-view`: still holds for the dots, the Toolpath view and Grey, read
  "Engrave 1" as "Image 1" and the Engrave chip on the dots as the Image chip.
- `03-checks-and-test-grid`: screen A holds with "Image 1" in place of "Engrave 1" in the Grey line;
  screens B and C (Fill and the Dither menu on the Engrave grid) are replaced by screen 06.

No new colour: pale orange `--engrave-hatch`, red `--cut`, ink, muted ink, the clone violet of `0049`;
blue is the existing pressed and selected look.
