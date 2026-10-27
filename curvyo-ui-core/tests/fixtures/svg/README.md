# SVG import fixtures

Golden files of `specs/0024-svg-import-export` (criteria 74 and 75). Each `name.svg` is an input;
`name.document.json` is the document it opens as (page, display unit, background, the tree with
every leaf in document order; a clone names its original by that number and carries its matrix).
`tests/svg_import_golden.rs` compares them within 1e-9 mm, `tests/svg_import_fixtures.rs` checks
what each file is for, and the hostile sweep of criterion 70 (a) runs over all of them.
`CURVYO_UPDATE_GOLDENS=1 cargo test -p curvyo-ui-core --test svg_import_golden` rewrites the goldens;
a changed golden is a changed behaviour of the spec and is explained in the PR.

**Provenance.** No Inkscape or Illustrator runs on the build machine. Every file here is
**hand-written** in the layout those programs write (attribute order, namespaces, `namedview`,
`sodipodi:` and `inkscape:` data, Illustrator's comment, DOCTYPE and `switch`), from their
documented output, by the implementer on 2026-10-26. The tester replaces the files marked
"Inkscape layout" with files saved by the current stable Inkscape 1.x and records its version and
the steps here (criterion 75); `page_illustrator.svg` is replaced by the customer's own export when
he has one. All files are original work under the project's licence; none is copied from a third
party.

| File | Layout | What it holds |
|---|---|---|
| `layers.svg` | Inkscape 1.3 | Layers Cut (with the sublayer Holes), Engrave (locked, 50 %, moved: `translate`), Reference (hidden), Empty |
| `groups_nested_transforms.svg` | Inkscape 1.3 | Three nested groups with translate, rotate(30), scale(2), a matrix, a path with skewX, leaves at every level |
| `primitives.svg` | Inkscape 1.3 | Rectangle, rounded, rx ≠ ry, rotated; ellipse, circle; 5-point star, hexagon; spiral, arc, pie; line, polyline |
| `text_converted_to_paths.svg` | Inkscape 1.3 | "Boho" as paths (holes in B and o; simplified glyph shapes) |
| `text_live.svg` | Inkscape 1.3 | The same word as live text (slice 2: one text object) |
| `text_inkscape.svg` | **Inkscape 1.4.4** (real) | Cut from `text_ink.svg` (see below): 30 pt line; three centred lines at `line-height:1.25`; right-aligned; Bold, Italic, Light, Bold Italic, a 600 request, Source Serif 4; letter and word spacing; `dx`/`dy`/`rotate` kerns; "Hello world red" with a bold and a smaller red word; `baseline-shift:super`; vertical upright; rotated 30°, stretched 150 %, mirrored; "Nonexistent Sans". `text_inkscape.paths.svg`: Inkscape's Object to Path of it |
| `text_on_path.svg` | **Inkscape 1.4.4** (real) | Cut from `text_ink.svg`: texts on an open stroked path, a stroked circle at `startOffset` 75 % centred, an unpainted rectangle, an unpainted path with `side="right"`, and a path whose text has its own `transform`. `text_on_path.paths.svg`: Inkscape's Object to Path |
| `text_frame.svg` | **Inkscape 1.4.4** (real) | Cut from `text_ink.svg`: an `inline-size` text and a `shape-inside` text, as Inkscape saves them (with its SVG 1.1 lines). `text_frame.paths.svg`: Inkscape's Object to Path |
| `images.svg` | **Inkscape 1.4.4** (real) | A copy of `images_ink.svg`: an RGBA PNG (default meet, `none`, `xMinYMin slice`: the same PNG three times), a JPEG, one rotated 30° by Inkscape, one with `skewX(20)`, a linked image, one in a clipped group |
| `text_flowroot.svg` | Inkscape 0.92 | A centred `flowRoot` of 60 mm with two paragraphs, a bold `flowSpan` and a `flowBreak` |
| `text_illustrator.svg` | Illustrator CC | `<style>` classes, PostScript family names, `tspan` lines with `x` and `y`, tracking, a kerned pair as a positioned `tspan`, a coloured text |
| `text_generators.svg` | hand-written | Per-character `x` lists, `rotate` lists, `dominant-baseline` values, `baseline-shift: super`, `text-transform`, font-size keywords and `em`, generic families only, `xml:space` both ways, `white-space: pre`, a `switch` behind Illustrator's `foreignObject` and one behind a bare `foreignObject` |
| `images_webp_and_others.svg` | hand-written | Data URIs of a WebP, a GIF, a BMP, an SVG, a percent-encoded PNG, damaged base64, the PNG bomb and a truncated PNG of `0058` (only the WebP and the PNG come in) |
| `inkscape_shapes.svg` | **Inkscape 1.4.4** (real, see `SOURCES.md`) | Slice 3 (criterion 125), made from tool numbers by `inkscape_shapes_src/make_inkscape_shapes.sh`: slice, arc, chord; an arc past 180° and one from a negative angle; the arc tool's whole ellipse (saved as `<ellipse>`) and a 359.99° arc; an elliptic arc; an arc rotated 30° and one flipped; a labelled pie; a rectangle with `rx=10 ry=5`; polygons with 3, 5 and 6 sides (one rotated 15°); a 5-point star; stars with `rounded=0.3`, `randomized=0.2`, a twisted one, one rotated 20°, one flipped; a spiral; a spiral and an arc under `skewX(20)`; an arc and a polygon under `scale(2,1)`; a star in a rotated group; a 3D box with a labelled face. `inkscape_shapes.paths.svg`: Inkscape's Object to Path |
| `inkscape_shapes_edited.svg` | hand-written | Slice 3: an arc and a star with a node moved by 0.002 and 0.0005 mm, a twisted star, `arc-type` missing with and without `open`, an 0.92-style arc, the hostile numbers of criterion 124, an arc whose `d` is unusable (skipped) |
| `inkscape_stars_customer.svg` | **Inkscape 1.4.4** (customer) | The customer's two stars (his own drawing, given to the project, project licence; criterion 117's note S3-10): `path17`, 12 points with `r2 > r1` under a uniform `matrix`, copied verbatim; `path1`, 15 points with `r1 > r2`, its numbers verbatim, its centre and `d` rebuilt from them in Inkscape's layout (the original `d` was not sent) |
| `dashed_strokes.svg` | Inkscape 1.3 | Inkscape's dash presets, caps, joins, opacities, a dash offset, miter limit 8, widths in px and mm |
| `arcs.svg` | Inkscape 1.3 | The four flag combinations, rotated ellipse arcs, too-small radii, a zero radius, equal end points |
| `even_odd.svg` | Inkscape 1.3 | Donut, triple ring, pentagram, two crossing circles (even-odd), a nonzero donut, an unfilled even-odd path |
| `colours_and_css.svg` | Inkscape 1.3 | Every colour form, attribute against `style`, classes and an id from `<style>`, inheritance, `currentColor`, an unsupported selector |
| `unsupported.svg` | Inkscape 1.3 | Linear and radial gradient, pattern, clip path, mask, blur, marker, embedded PNG, `foreignObject`, an `inkstitch:` attribute |
| `clones.svg` | Inkscape 1.3 | Alt+D clones moved, rotated 30°, mirrored; a clone of a clone; scaled; of a rectangle; in a rotated group of an original in another group; uses that set a fill; a symbol |
| `page_a4.svg` | Inkscape 1.3 | The default A4 page |
| `page_legacy_0_48.svg` | Inkscape 0.48 | 744.09 × 1052.36 px at 90 per inch, no viewBox |
| `page_namedview.svg` | Inkscape 1.3 | Document units in, page colour at full opacity |
| `page_illustrator.svg` | Illustrator CC | 595.28 × 841.89 px with a viewBox, classes in `<style>` |
| `page_illustrator_entities.svg` | Illustrator CS6 | The `ns_*` entities in the DOCTYPE, `switch` with `requiredExtensions` |
| `page_px_no_viewbox.svg`, `page_inches.svg`, `page_no_size_viewbox.svg`, `page_no_size_no_viewbox.svg`, `page_aspect_meet.svg`, `page_aspect_none.svg`, `page_percent.svg` | hand-written | The page rules of criteria 15 to 18 |
| `path_grammar.svg` | hand-written | Every command absolute and relative, compact numbers and flags, the errors of criterion 23 |
| `empty.svg` | hand-written | An `<svg>` without content |
| `hostile_*.svg`, `hostile_gzip.svgz` | hand-written | Entity bomb, external entity, external references, `use` loop and fan-out (a million copies), 1,000 nested groups, out-of-range numbers, a truncated file, UTF-16, gzip, not XML |

**Real Inkscape files (slice 2).** `text_inkscape`, `text_on_path`, `text_frame` and `images`
are not hand-written: they come unchanged from the tester's Inkscape 1.4.4 files in
`curvyo-svg-core/tests/fixtures/svg_real/inkscape_text/` (made by `make_slice2.sh`, provenance in
`../../../../curvyo-svg-core/tests/fixtures/svg_real/SOURCES.md`). `cut_inkscape_fixtures.py`
(`python3 -I cut_inkscape_fixtures.py`) keeps whole elements of `text_ink.svg` and of its
`text_ink.paths.svg` (Inkscape's own Path > Object to Path) for each fixture and its
`name.paths.svg`, and copies `images_ink.svg`; rerun it after `make_slice2.sh`. The other
slice-2 fixtures are hand-written on 2026-10-27 in the layout named in the table (the
`inkscape:version` of `text_flowroot.svg` is part of the 0.92 layout, not a claim that 0.92 wrote
it), with the picture bytes of
`curvyo-image-core/tests/fixtures/images/`. All texts use the included faces, so the goldens need
no system font. The golden test skips the `name.paths.svg` files.

The large hostile inputs (a 256-byte entity referenced 200,000 times, a 100,000-number dash list, a
60 MiB file) are generated by `curvyo-svg-core/tests/hostile.rs` instead of being stored; the
hostile texts of slice 2 (2,000,000 characters, 100,000 texts, 10,000 font names, 10,000 spread
positions) by `tests/svg_import_slice2_fixtures.rs`.
