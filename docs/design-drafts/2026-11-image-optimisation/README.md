# 0135 Image optimisation drafts (ux-engineer, 2026-11-19)

Static screens for `specs/0135-image-optimisation/`; real tokens, the 280 px panel with its 256 px
column, PNG beside each HTML. A dashed outline marks a new or changed control. The notes are
`specs/0135-image-optimisation/ux-notes.md`; proposals there are P1 to P27, spec deltas E1 to E14.

- `01-style-tab-preset-and-strength`: the Adjustments group in the selected image's Style tab: a new
  picture (Expert, nothing changed), Photo picked with its tooltip "Photo (detailed) · Dither: Jarvis",
  the Strength slider and field (the one slider), the derived read-only rows and Make editable, the
  stack after Make editable with Levels open, a preset over a manual stack ("Preset in use", bands),
  the reorder drag and an armed trash.
- `02-step-rows-and-bodies`: the twelve step glyphs at 1x and 2x, the add toolbar, and every step's
  body: Curves, Local contrast, Sharpen, Denoise, Contours (off), Resize, Channel mix, Crop, a refused
  Levels value, an unknown step.
- `03-crop-and-before-after`: the Crop bar replacing the Select bar with the crop handles and an entry
  chip; Before and after as a canvas side pane (Split, and Side by side in full view with the operation
  pick); the canvas looks in the Select and Operations views with a removed background.
- `04-ai-steps-and-limits`: slice 3: Run, the not-downloaded line, the one-time consent in place, the
  download and run progress with Cancel, host not reachable, a bad checksum, stored results up to date
  and out of date, the budget line, the working-limit and size refusals.
- `05-clone-grid-and-operation`: a linked clone with the third override row Adjustments (Original's,
  then Own after the first edit), the Image row with nine Dither modes (Halftone fields), the two check
  lines and the one notice, Halftone and Lines at draft scale, the Image test grid with Picture and the
  Strength axis.

`adjust.css` and `adjust.js` build on `../2026-11-raster/kit.css` and `kit.js` (tokens, Lucide paths,
rows, the dither routine) and add the step glyphs, the group's parts, the Levels and Curves graphs and
a draft-scale Photo preset, Halftone and Line screen, so the dots are what the steps give.

No new colour: ink, muted ink, the existing blue of a pressed or selected control, `--field-invalid`
red for refusals, `--destructive-soft` for an armed row, pale orange `--engrave-hatch` for dots, the
checkerboard for "burns nothing", the clone violet of `0049`.
