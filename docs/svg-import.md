# SVG import: what is read and what is ignored

Open… reads an `.svg` file as a new, untitled project; the Import button of the tool rail (Ctrl+I)
adds one as a new layer in the middle of the view, or at the point where it is dropped on the
canvas (`specs/0024-svg-import-export`, `specs/0058-bitmap-import`). Both read the same subset.
What is not in it is left out without a message; when nothing at all could be imported, one
notice says so. Five kinds of file are refused: not an SVG, XML entities beyond plain text
constants, larger than 50 MiB, not UTF-8, and compressed `.svgz`.

## Read

- **Page.** `width` and `height` (px, pt, pc, mm, cm, in, em, ex; a plain number is px) and the
  `viewBox` with `preserveAspectRatio`. Without a viewBox one user unit is one pixel (1/96 inch);
  without sizes the viewBox gives the page; with neither, the drawing's extent. Files from Adobe
  Illustrator use 1/72 inch per pixel, files from Inkscape 0.91 and earlier without a viewBox
  1/90 inch. Inkscape's document units (mm, cm, in) and page colour become the display unit and
  the background. Import uses these rules only to scale the file to millimetres.
- **Shapes.** `path` (every command of SVG 1.1 and 2, arcs as cubic curves of at most 45°),
  `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`. A rectangle, circle or ellipse stays
  a primitive when its transform is a move, rotation, uniform scale or mirror (or a stretch along
  its axes); otherwise it becomes its exact outline. An Inkscape star or polygon stays one while
  its drawn path still matches its star data. Inkscape's node types are kept.
- **Structure.** `g` and `a` as groups (32 levels), `switch` as its first child that needs no
  extension, nested `svg` as a group, Inkscape layers with their names, hidden and locked state and
  opacity. Names come from `inkscape:label`.
- **`use`.** A linked clone where the relation to its original is a move, rotation or flip and it
  looks the same; otherwise an ordinary copy (also of symbols).
- **Style.** Presentation attributes, the `style` attribute, and `<style>` rules with element,
  `.class`, `#id` and `*` selectors. Fill and stroke colours in every CSS form, `currentColor`,
  fill and stroke opacity and the object's opacity, stroke width, caps, joins, dashes. A gradient
  becomes its middle colour, a pattern mid grey. An even-odd fill is re-wound so it looks the
  same where the outlines do not touch or cross.
- **Transforms** of every kind are applied to the geometry.

## Ignored

Text (`text`, `tspan`, `textPath`, `flowRoot`; text converted to paths is read), images, clip
paths, masks, filters, SVG markers, `foreignObject`, scripts and animation, `stroke-dashoffset`,
miter limits other than 4, `paint-order`, blend modes, CSS rules with combinators, attribute
selectors or pseudo-classes, `@` rules, metadata, guides and grids, attributes of other programs
(for example `inkstitch:`), and every reference to another file or web address: nothing is ever
fetched.

## Limits

At most 500,000 elements and 5,000,000 path points are read (the rest of a larger file is left
out); elements nested deeper than 256 levels refuse the file; `use` copies stop at 16 levels and
100,000 copies; a coordinate beyond 10 km skips its element.
