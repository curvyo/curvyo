# 0123 Raster engraving drafts (ux-engineer, 2026-11-18)

Static screens for `specs/0123-raster-engraving/`; real tokens, 280px panel body plus its 32px strip,
PNG beside each HTML. A dashed outline in a draft marks a new or changed control. The notes are
`specs/0123-raster-engraving/ux-notes.md`; proposals there are P19 to P34, spec deltas E1 to E7.

- `01-engrave-row-image-settings`: the unfolded Engrave row with the Image settings group
  (Floyd-Steinberg default); the Dither pick open as an inline list; Threshold with the widest values;
  Grey with Lowest power in the same place; an Engrave with shapes only (no group, as `0111` built it).
- `02-operations-and-toolpath-view`: a photo and a gradient-filled ellipse in Engrave and a circle in
  Cut, in the Operations view as dots, with the Operations view off (grey on the laser page), in the
  Toolpath view zoomed in (runs, dashed row steps, ring, one arrowhead) and zoomed out (shaded dot
  area), and Grey mode with dot opacity by power.
- `03-checks-and-test-grid`: the new check lines in the Output group; the Engrave test grid bar with
  Fill (Solid / Gradient symbols) and the Dither menu open; the grid's cells grey on the laser page and
  as Ordered dots in the Operations view, finer with the finer Interval.

The dots are computed in the HTML by a small Floyd-Steinberg, Atkinson, Ordered (8 x 8 Bayer) and Grey
routine at draft scale, so the patterns are real. No new colour: pale orange `--engrave-hatch`, red
`--cut`, ink; blue is the existing pressed and selected look.
