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
  its axes); otherwise it becomes its exact outline. Inkscape's own shapes are described under
  "Inkscape shapes". Inkscape's node types are kept.
- **Structure.** `g` and `a` as groups (32 levels), `switch` as its first child that SVG draws
  (no required extension, only SVG 1.1 features, English or no `systemLanguage`, not a
  `foreignObject`), nested `svg` as a group, Inkscape layers with their names, hidden and locked state and
  opacity. Names come from `inkscape:label`.
- **`use`.** A linked clone where the relation to its original is a move, rotation or flip and it
  looks the same; otherwise an ordinary copy (also of symbols).
- **Style.** Presentation attributes, the `style` attribute, and `<style>` rules with element,
  `.class`, `#id` and `*` selectors. Fill and stroke colours in every CSS form, `currentColor`,
  fill and stroke opacity and the object's opacity, stroke width, caps, joins, dashes. A gradient
  becomes its middle colour, a pattern mid grey. An even-odd fill is re-wound so it looks the
  same where the outlines do not touch or cross.
- **Transforms** of every kind are applied to the geometry.

## Text

Live text (`text`, `tspan`, `a` inside text, `textPath`, Inkscape's flowed text) becomes
editable **text objects**; it is never turned into paths on import.

- **Font.** The `font-family` list is read in order against the fonts of the Fonts tab: Curvyo
  fonts first, then the included fonts, then the fonts of the computer (desktop). An entry
  matches a family name (ignoring case), or one face by its full name ("Myriad Pro Bold") or
  PostScript name ("MyriadPro-Bold", as Illustrator writes it). In a family, the face is chosen by
  italic, then width (`font-stretch`), then the nearest weight (`font-weight`, `bolder`,
  `lighter`); a missing weight takes the nearest face, nothing is made bold or slanted. The
  generic names map to the included fonts: `sans-serif`, `system-ui`, `monospace` and `fantasy`
  to Source Sans 3, `serif` to Source Serif 4, `cursive` to Great Vibes.
- **Font missing.** When every named family of the list is unknown, the text keeps the first
  name and the style asked for ("Lobster Bold"), is drawn in Source Sans 3 and shows the "Font
  missing" tag. Installing the font and reopening the project uses it. In the browser, a file
  that names fonts of the computer comes in this way.
- **Size.** Size (the height of the capital H) is the font size in millimetres times the face's
  H height; a uniform scale of the text's transform goes into the size, the rest (rotation,
  mirror, stretch, skew) stays its transform.
- **Lines and positions.** Inkscape's lines, newlines under `white-space: pre`, and absolute
  positions that start a new line become lines; `line-height` becomes Line spacing. Every other
  position (`x`, `y`, `dx`, `dy` lists, Illustrator's positioned `tspan`s), each turned character
  (`rotate`), `dominant-baseline`, `alignment-baseline` and `baseline-shift` become kerns and
  turns of single characters, so every letter lands where the file draws it.
- **Spacing and alignment.** `letter-spacing` and `word-spacing` become Letter and Word spacing
  (beyond their range the rest is kerns); `textLength` widens or narrows the letter spacing so
  the text is that long (the glyphs keep their shape); `text-anchor` becomes Left, Centre or Right where the
  lines share one anchor, else the text is set left with kerns. `writing-mode` and
  `text-orientation` give vertical text. `text-transform` is applied to the stored string.
- **Several objects.** One text object has one face, size and colour. A text with a bold word,
  a coloured letter or a smaller line, a word placed far from the rest (beyond the kern range) or
  more than 10,000 characters becomes several text objects in one group, together looking as the
  file draws them.
- **Text on path.** A `textPath` on a path or shape becomes a text on that object (Position from
  `startOffset` and `text-anchor`; with `side="right"` the text sits below the object, counted
  from its end, where the file draws it). A carrier with fill or stroke stays drawn; one with neither,
  or a hidden one, becomes a guide; one in `defs` or in another coordinate system is added as a
  new guide path below the text. A `textPath` to anything else is a straight text.
- **Flowed text.** Inkscape's `flowRoot`, `shape-inside` and `inline-size` texts become plain
  lines broken at the frame's width; the frame is not kept.

## Images

An `image` whose `href` is a `data:` URI (base64 or percent-encoded) becomes an **image object**
when its bytes are a PNG, JPEG or WebP (by content, not by the stated type), with the size and
place the file gives it (`x`, `y`, `width`, `height`, `preserveAspectRatio`; `slice` shows the
whole picture at the covering scale), and the image's transform. The same picture twice is stored
once. An embedded picture may be up to the 50 MiB file limit (the 16 MiB attribute limit does not
apply to it); the limits of picture import (`docs/importing-images.md`) apply to its contents.
A linked image (a file path or address), another picture type, or a damaged or cut-off picture is
left out.

## Inkscape shapes

Inkscape saves every shape tool object as a path that also carries the tool's numbers. The import
makes the Curvyo primitive from those numbers only when its outline and the drawn path lie within
0.001 mm of each other (the same subpaths, each open or closed alike, and every point of one
within 0.001 mm of the other, both ways; ellipses and arcs are measured against the true curve);
otherwise, and when a number is missing, unreadable or out of range, the object is an ordinary
path from its drawn data, without a message. A node moved by hand or by another program therefore gives a path.

- **Arcs** (the ellipse tool with angles) become an ellipse with an arc: Inkscape's slice is a
  pie, arc an arc, chord a chord; a file without `arc-type` (Inkscape 0.92) is an arc when it is
  open, else a pie. An arc without angles, or one that sweeps the whole turn, is a whole ellipse.
  A move, rotation, uniform scale or mirror keeps the arc, a stretch along its axes too; a rotated
  or skewed stretch gives a path. An arc is named "Arc".
- **Polygons and stars** (the star tool) become a polygon or a star when they are not rounded,
  not randomized and, for a star, not twisted (the inner points halfway between the outer ones).
  A stretch gives a path.
- **Stay paths:** rectangles whose corner radii differ in x and y (Curvyo's corners are circular),
  rounded or randomized stars and polygons, twisted stars.
- **Spirals** are one open path named "Spiral".
- **3D boxes** are a group named "3D box" of their faces, each a path with its own style named
  "Face 1" to "Face 6"; the perspective is not kept.

A name given in Inkscape (`inkscape:label`) always wins.

## Ignored

Underline, overline and strike-through, small caps and other `font-variant`s, OpenType feature
and variation settings, `font-kerning`, `direction` and `unicode-bidi`, stretched glyphs of
`lengthAdjust`, `textPath` `method` and `spacing`, SVG and embedded fonts; linked images; clip
paths, masks, filters, SVG markers, `foreignObject`, scripts and animation, `stroke-dashoffset`,
miter limits other than 4, `paint-order`, blend modes, CSS rules with combinators, attribute
selectors or pseudo-classes, `@` rules, metadata, guides and grids, attributes of other programs
(for example `inkstitch:`), and every reference to another file or web address: nothing is ever
fetched.

## Limits

At most 500,000 elements and 5,000,000 path points are read (the rest of a larger file is left
out); elements nested deeper than 256 levels refuse the file; `use` copies stop at 16 levels and
100,000 copies; a coordinate beyond 10 km skips its element. A file holds at most 1,000,000 text
characters; position lists are read up to 10,000 numbers; a font name is cut at 128 characters.
