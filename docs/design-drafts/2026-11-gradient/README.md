# 0124 Gradient fill drafts (ux-engineer, 2026-11-18)

Static screens for `specs/0124-gradient-fill/`; real tokens, 280px panel body plus its 32px strip,
PNG beside each HTML. A dashed outline in a draft marks a new or changed control. The notes are
`specs/0124-gradient-fill/ux-notes.md`; proposals there are P1 to P18, spec deltas D1 to D9.

- `01-style-tab-gradient-block`: the Fill paint switch with four items; the Gradient block just
  chosen (Linear, two stops, picker under the active stop); four stops with the third active; Radial
  with Radius; a mixed selection with the image skip line and "Mixed gradients"; a text (three items,
  Pattern not offered).
- `02-handles-and-entry-boxes`: the default line on a rectangle (covered edge handles hidden, P14);
  an End drag with Shift and its readout; Radial on an ellipse; the end entry box (Angle, swatch,
  hex); the stop entry box with the 0017 picker opened inside it; an open path and a text with gradients.
- `03-laser-page-grey-and-import-notice`: the same objects on a drawing page and on a laser page
  (grey by luminance), the Style tab on the laser page in colour, the SVG import notice.

No new colour: the swatches and handles show the document's stop colours (navy, teal, amber, white of
a sample drawing); the UI uses `--accent`, ink and the existing grounds only. The grey of screen 03 is
an SVG `feColorMatrix` with the weights 0.2126, 0.7152, 0.0722 of criterion 10.
