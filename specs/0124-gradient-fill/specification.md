# Gradient fill: a third fill paint that shades any closed shape from one colour to another, with handles on the shape

Status: Draft
Priority: Should
Origin: Customer (decision on Frage 227 C of `0123`, 2026-11-17: "every closed shape gets a third fill paint, Gradient, next to Colour and Pattern; pull it forward as its own slice before `0123`, because it would blow up `0123`"). Everything marked **(Proposal)** is the product owner's and vetoable at the demo. Frage 232 to 236 are open (see "Open customer questions"); each has a default, so the story is Ready for the architect and the UX engineer as written, and a different answer changes only the criteria named under the question.

## User value

As a maker I want to fill a closed shape with a shade from one colour to another, set by two handles on the shape, so that I can draw soft areas, and so that a shaded shape can later be engraved as dots on a laser page (`0123`), without a second program for shading.

**What other tools do.**
- **Inkscape**: Fill and Stroke has a Linear/Radial gradient button; the gradient lives in a shared `<defs>` vector, edited with a Gradient tool (G) whose handles sit on the canvas; stops are added by double-click; every gradient is also an SVG element, so it round-trips.
- **Illustrator and Affinity**: a gradient panel and an annotator bar on the shape with stops, angle and midpoints.
- **LightBurn**: no gradient object; shaded areas come from imported bitmaps.
- **What we do better.** The gradient is a fill paint like Colour and Pattern (`0071`), switched in the same place, kept when switched away from, and edited by two handles on the shape in the Select tool (no second tool), in the shape's own box so it moves, scales, rotates and mirrors with it. It is plain data (no pixels), exports as a real SVG `linearGradient`, and on a laser page it is previewed as grey and is what `0123` reads as lightness.

## Words (shown in the UI)

- **Gradient:** the third value of the Fill paint switch, after None, Solid (shown as Colour) and Pattern. A linear shade between colour stops.
- **Stop:** a colour (8-digit RGBA hex, as every colour) at a position along the gradient line. A gradient has two stops: Start and End (Frage 232).
- **Gradient line:** the line between the two handles; the shade runs along it, perpendicular lines have one colour. Before Start and after End the end colours continue.
- **Handle:** a point on the canvas, at the end of the gradient line, that moves it; shown in the Select tool for a selected shape with a Gradient fill.
- **Angle:** the direction of the gradient line in degrees, 0 pointing right, positive counter-clockwise on the page (as `0111` Scan angle).

One-line meaning (customer rule: the meaning of a new fill before it is built): **Gradient:** "Shades the inside of the shape from one colour to another along a line you can drag."

## Acceptance criteria

"Fill paint", "PaintRow", "Pattern", "seed", "style register" are as in `0071`; "linked clone", "own style" as in `0049` and `0057`; "working image" as in `0058`; "laser page", "Engrave", "leaf" as in `0101` and `0029`. All lengths mm, angles degrees.

### The paint

1. Given a closed shape selected (rectangle, ellipse, polygon or star, path, compound path, linked clone; Frage 234), when the maker opens Properties > Style > Fill, then the Fill paint switch has the choices None, Colour, Pattern, Gradient (`0071` criterion 1 gains one item, symbol-only with a tooltip: "No fill", "Solid fill", "Pattern fill", "Gradient fill"). There is no rail tool and no top bar control for gradients.
2. Given Gradient is chosen on a shape that has none, then the fill starts with Start = the shape's current Colour (alpha kept) and End = white (`#FFFFFFFF`) at Angle 0 across the shape's box, so the shade is visible at once (Proposal), as one undo step "Fill: Gradient". Switching to None, Colour or Pattern keeps the gradient and its numbers; choosing Gradient again brings it back as it was, with no undo needed (the rule of `0071` UX notes).
3. Given a shape with a Gradient fill, then the Style tab shows, where the Colour block is today, a Gradient block: two rows, Start and End, each with a colour swatch, an eyedropper and an 8-digit hex field exactly as the Colour row (`0017`; no new picker), and a Reverse button (symbol `ArrowLeftRight`, tooltip "Reverse") that swaps Start and End colour; a plain number field Angle (-180.0 to 180.0, one decimal, unit inside, "deg"). The inline colour picker of `0017` opens under the row being edited. No sliders, no stop list, no captions beyond the field names (customer lessons).
4. Given the two end positions, then they are two points on the shape's box stored relative to it (x and y as fractions of the box width and height, 0 to 1 inside the box, free beyond it, two decimals shown as percent in the handle tooltip only), defaulting to the middle of the left edge (Start) and the middle of the right edge (End). Changing Angle in the panel turns the gradient line about its midpoint and keeps its length; a typed or dragged handle sets the angle from the line (criterion 6).
5. Given the shade, then colour is interpolated linearly in sRGB, channel by channel, alpha included (premultiplied: Proposal, so a fade to transparent has no grey fringe); positions before Start and after End take the end colour. Test: Start `#000000FF`, End `#FFFFFFFF` over a box 100 wide at Angle 0 reads 50 % grey at x = 50 within 1/255 per channel.

### Editing on the canvas

6. Given a shape with a Gradient fill selected with the Select tool, when the maker looks at the canvas, then two handles are drawn on the gradient line, joined by a thin line, each a round dot filled with its stop colour and ringed in the selection colour (no new colour; the existing handle look of `0009`), with the Start handle marked by a small square so the direction is readable without colour. Dragging a handle moves that end, the angle and length follow; Shift locks the angle to the product's angle stops (15 and 22.5 degrees, horizontal and vertical included, as `0072`); Alt drags both ends together (moves the line); Escape while dragging restores the previous state. One drag is one undo step "Edit gradient". The handles never hide the shape's own handles: when both lie within 8 px the gradient handle loses the click (Proposal), and the gradient handles appear only while the Fill switch is Gradient.
7. Given a handle, when the maker clicks it, then the Style tab focuses the Start or End colour row (keyboard route: Tab order Fill switch, Start row, End row, Reverse, Angle); a colour typed there previews on the canvas and commits as one step "Edit gradient".
8. Given a selection of several shapes with Gradients, then the panel shows the fields common to all and "Mixed" where they differ (as `0071` UX note), and the handles are drawn for the first selected shape only (Proposal); typing a colour or Angle sets it on all in one step. A Reverse is applied to each shape's own gradient.

### Where it shows

9. Given the editor, then the gradient is drawn in the WebGL path of `curvyo-render-core` as a shaded fill clipped to the shape's outline with holes (even-odd or nonzero as the shape's fill rule), at every zoom, with no banding visible at 8 bits and with the stroke drawn over it; hit-testing of the interior (click to select) works wherever the gradient has alpha above 0 at the point, as for a Colour fill with alpha (Proposal).
10. Given a drawing page, then the gradient is shown in its colours. Given a laser page (`0098`), then the same gradient is shown as grey: each pixel's colour is converted to its luminance (0.2126 R + 0.7152 G + 0.0722 B on the gamma-encoded values, over white; the weights `0123` criterion 6 uses), because on a laser page colour does not output (`0101` criterion 24) and the grey is what the laser engraves in `0123`. The Fill block still shows the real colours; only the canvas and the thumbnails of a laser page are grey (Proposal: a notice-free change of view, no new setting).
11. Given PNG export, then the gradient is rendered into the PNG at the export resolution, with the same colours as the canvas on a drawing page (grey is a view only: PNG always uses colour).
12. Given a laser job (`0029`), then a Gradient fill changes nothing in it: the fill paint never counts (`0101` criterion 24); the shape keeps its vector hatch exactly as a Colour-filled shape. Engraving a shape as dots by its gradient is `0123` and not this story.
13. Given SVG export (`0024`), then a shape with a Gradient fill is written with `fill="url(#gradient-n)"` and one `linearGradient` in `defs` with `gradientUnits="userSpaceOnUse"`, `x1 y1 x2 y2` in the exported coordinates (the box fractions resolved against the shape's box, then the shape's transform applied as `gradientTransform` only where a rotation/skew exists), two `stop` elements (`offset`, `stop-color`, `stop-opacity`); ids are stable and numbered in document order so two exports are byte-identical. A shape that also has a stroke keeps it. Test: golden files `curvyo-svg-core/tests/fixtures/export/gradient_linear.svg`, `gradient_ellipse_rotated.svg`, `gradient_compound.svg`; each read back by the reader in the test and by a second, independent parser of the tester's making.
14. Given SVG import (`0024`, `docs/svg-import.md`), then a `fill` that refers to a `linearGradient` (directly, by `href` chain up to 16 links, in `userSpaceOnUse` and `objectBoundingBox` units, with `gradientTransform` of translation, scale and rotation) becomes a Gradient fill whose end points are fractions of the shape's box; with exactly two stops it is read fully; with more stops and a `radialGradient` the **middle colour** is used as today (`curvyo-svg-core/src/paint.rs`, `gradient_middle`) and the import notice counts them ("2 gradients were simplified to a colour."); `spreadMethod` other than `pad` is read as `pad`. Goldens: `curvyo-svg-core/tests/fixtures/svg_real/gradient_*.svg` (from Inkscape and Illustrator, one of each of the cases above), checked by tests. Frage 236.

### Live model and operations

15. Given a Gradient shape, when the maker moves, scales, rotates, mirrors, skews or edits its nodes, then the gradient follows: it belongs to the shape's box in its own coordinate frame, so the picture inside the shape is unchanged by a transform and a node edit that changes the box re-fits the gradient to the new box (the end points are fractions, so they stay on the same relative place) in the same frame, with no extra step. A resize drag previews the gradient live.
16. Given copy and paste, duplicate, and Paste style (the style copy of `0017`/`0071`), then the copy has the same Gradient (colours, fractions, Angle); Paste style onto another shape sets that shape's fill to the Gradient with the same fractions in the target's box. Editing the copy does not change the original.
17. Given a linked clone (`0049`) of a Gradient shape, then it shows the gradient of its original, in the clone's frame, and follows edits of the original; with own style (`0057`) it carries its own gradient or colour like any style; the Spread colours action of `0057` leaves gradients alone and says so in its existing skip line (Proposal).
18. Given undo and redo, then every edit of a Gradient (paint switch, colour, Angle, handle drag, Reverse) is one step with its name ("Fill: Gradient", "Edit gradient", "Reverse gradient") and the previous state returns exactly; no gradient edit is applied twice or lost with a History branch (`0042`).
19. Given Object to path, Boolean operations, Combine, Split, Break apart, Offset and Fracture, then a result that is a closed shape keeps the Gradient of the first (bottom) input where `0016` keeps its fill colour, and the gradient fractions are re-based to the result's box (Proposal; same rule as the Colour); Flatten and Weld keep each piece's own. Given a group, then it has no fill of its own and nothing changes (`0023`).
20. Given an open path or a stroke, then the Gradient item is offered on an open path as Pattern is (`0071` criterion 23: kept, nothing drawn, no error), and no stroke ever has a gradient (out of scope).
21. Given a text object (`0074`), then it has no Gradient fill in this slice (Frage 234); the Gradient item is not in the tree for a selection that holds only text, and a mixed selection shows the existing skip line "1 text cannot have a gradient and was not changed." (the sentence of `0071`).

### Model, format, performance

22. Given the model, then a Gradient fill is style data of the shape: the style registers `fill_paint` value `gradient` and `fill_gradient` (two colours as 8-digit hex, two points as fractions, nothing else); no pixel, texture or generated vector is stored; the Colour and the Pattern registers stay untouched when Gradient is chosen. A damaged value (unknown colour, fraction not a number) is refused on read like `0071`'s keys: the shape is shown with no fill, the file opens, and the damage is counted in the existing notice. The architect decides the register names, the codec and whether a `format_version` bump is needed (Proposal: yes, one bump, the number taken at merge, "next free at merge" in the README table; an older build would draw no fill where the gradient is). Golden project files: `gradient_fill_vNN.curvyo` (one of every closed kind with a Gradient, a linked clone, an own-style clone) and `gradient_fill_damaged_vNN.curvyo` (an unknown colour, a fraction `abc`). Curvyo is not deployed, so there is no migration.
23. Given the data of `0007` and `0017` criterion 53 (gradient data in old files, ignored since `0017`), then it stays ignored: this slice does not read it, and an old file opens as it does today (Proposal; old files are not a concern).
24. Given the renderer, performance, and thread CPU time (`curvyo-editor-wasm/tests/cpu_time/`; the architect confirms the limits; Proposal): (a) dragging a handle on a shape with a Gradient redraws within 16 ms per pointer move at 1,000 gradient shapes on the page; (b) a page with 1,000 gradient shapes draws within the frame budget of today's page of 1,000 Colour shapes plus 2 ms; (c) the Style tab's Gradient block appears within 16 ms of choosing Gradient; (d) opening a project with 10,000 gradient shapes takes no longer than 10,000 Colour shapes plus 5 %.
25. Given crates, then the gradient types, the colour interpolation and the SVG mapping live in `*-core` crates (`curvyo-document-core`, `curvyo-render-core`, `curvyo-svg-core`) with no filesystem, network, clock, thread or UI and `#![forbid(unsafe_code)]`; this slice adds no crate and no dependency (Proposal; the architect confirms), no `unsafe`, and no trait without a second implementation. Colours are the existing colour newtype; fractions and angles are newtypes, no bare `f64`.
26. Given the reversal of an accepted decision, then the repository states it: `docs/requirements.md` R-EDIT-006 reads "none, solid colour, pattern (`0071`) or gradient (`0124`)", `docs/design-system.md` gets a "Gradient (`0124`)" row (the UX engineer writes it), and a case-insensitive search for `gradient` in the Rust crates is no longer expected to find only the legacy path (`0017` criterion 52 is replaced by this story's tests).

## Changes to accepted specs

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0007` / `0017` | "Gradients removed completely", 49 to 52 (no gradient anywhere, no gradient fill kind in the model, no code that draws or hit-tests one), 53 (old gradient data ignored) and "Out of scope: gradients in any form, ever, until the customer asks for them; then as a new design, not as a restoration" | no gradient in the product | **Reversed by the customer on 2026-11-17 (Frage 227 C of `0123`):** a new design (a Fill paint with two colour stops and handles on the shape), not a restoration; the old stop model, thumbs, panel bar, radial gradient and `0007`'s format data are not brought back; 53 stays (old data ignored, 23) |
| `0071` | criterion 1 (fill kind None, Colour, Pattern) | three | four: plus Gradient (1) |
| `0024` | SVG export of fills; SVG import of paint servers | Colour and Pattern only; a gradient fill becomes its middle colour | a two-stop `linearGradient` round-trips (13, 14); other gradients keep the middle colour |
| `docs/requirements.md` | R-EDIT-006 | "none or solid color; gradients removed" | adds Gradient (26) |
| `0123` | criteria 23 to 30 and Frage 227 | the gradient rectangle lives in `0123` | moved here and generalised; `0123` depends on this story |

## Out of scope

- Multi-stop gradients, a stop list, midpoints, and radial, conic, diamond and mesh gradients, unless Frage 232 and 233 say otherwise.
- A gradient on a stroke, on a text object, on an open path's (non-existent) fill, on a group, on the document background, or on the canvas background.
- A gradient tool in the tool rail, a gradient panel of its own, saved gradient presets or a library of gradients, colour palettes (`color-management`).
- Dithering, engraving a gradient, the laser power ramp, the Engrave test grid: all in `0123`.
- A gradient in the plotter, CNC or embroidery outputs.
- Reading the legacy gradient data of `0007` (23).
- Non-linear interpolation (in linear light, in Oklab, easing).
- Animating a gradient, a gradient follow-the-path or any other generator.

## Open customer questions

Frage 232 to 236, in the format of `CLAUDE.md` §1; also in the report to the lead.

**Frage 232 (GRAD-STOPS): How many colours can a gradient have?**
- A: Two. A Start and an End colour, a Reverse button, two handles. The simplest control; nothing to add or remove.
- B: Two to eight stops. Double-click on the gradient line adds a stop, a stop dot is dragged and deleted with Delete, a stop list sits under the colour rows.
- C: Two now, more stops in a later slice (same as A for this slice, the stored data is written so that more stops can be added without a new format).
- Recommendation: C. You get a first usable gradient quickly; stops are one more control and one more set of cases (editing, SVG import of more than two), and a laser ramp needs only two.
- Default if unanswered: C.

**Frage 233 (GRAD-KIND): Linear only, or radial too?**
- A: Linear only (the gradient line with two handles).
- B: Linear and radial (a switch Linear / Radial in the Gradient block; radial has a centre handle and a radius handle).
- Recommendation: A. Radial is a second shape of data, rendering, SVG import and handles; it can follow as its own slice when you need a glow.
- Default if unanswered: A.

**Frage 234 (GRAD-SCOPE): Which objects can have a Gradient?**
- A: Every closed shape: rectangle, ellipse, polygon and star, path, compound path, and a linked clone of these. Not text, not images, not groups.
- B: A plus text objects (a gradient over the glyphs as one shade across the text's box).
- C: Only primitives (rectangle, ellipse, polygon and star), not paths.
- Recommendation: A. It is what you asked for ("every closed shape"). Text has its own fill rules (`0074`, per letter after Object to path) and can follow.
- Default if unanswered: A.

**Frage 235 (GRAD-HANDLES): How is the gradient line edited?**
- A: Two handles on the shape in the Select tool (drag, Shift for angle stops, Alt moves both), plus the colour rows and an Angle field in the Style tab. As described in criteria 3, 4, 6 and 7.
- B: Only the Style tab: colours, Angle and a Length field; no handles on the canvas.
- C: A: plus a small gradient bar (a coloured line with the two stop dots) in the Style tab as a second way to drag the ends.
- Recommendation: A. The shade is a place on the shape, so the handle is where you look; the panel has the numbers. A bar in the panel is the removed `0007` look.
- Default if unanswered: A.

**Frage 236 (GRAD-SVG): What does SVG import do with gradients?**
- A: A two-stop `linearGradient` becomes a Gradient fill; other gradients (more stops, radial) keep today's rule (their middle colour) and the import notice counts them. Export writes a real `linearGradient` (criteria 13, 14).
- B: Export writes a `linearGradient`, import keeps today's rule (middle colour) for every gradient; no import change in this slice.
- C: Neither; a Gradient fill exports as its middle colour like the import does (no SVG change).
- Recommendation: A. It is the one place the gradient shows outside Curvyo, and Inkscape drawings arrive with the shade they had.
- Default if unanswered: A.

## UX notes

Not written. The UX engineer owns them before Ready: the Fill paint switch with four items and the `GradientPaintIcon` (16px, 1.5px stroke, `currentColor`: a 12px square with a diagonal fade), the Gradient block in the Style tab (rows, Reverse, Angle, heights so no section moves when the block comes and goes), the canvas handles (dot filled with the stop colour, Start marker, hit areas, priority against the shape's own handles), the grey view on a laser page, the History names, the skip line for text, the keyboard route, the row in `docs/design-system.md`. Customer lessons that bind them: symbol-only buttons with a tooltip, units inside the fields, plain fields and no sliders, a tool-specific bar and no new tool, no invented colours (the swatches show the stop colours and nothing else), Hidden not disabled, Curvyo is not deployed so there are no migrations. The pattern to follow is `specs/0071-pattern-fill/ux-notes.md` (§1 Fill paint group, §2 block heights, the "kept when switched away" rule).

## Links

Requirements: R-EDIT-006 (amended, 26)
Parent: none; follows `0071` (the fill paint), reverses part of `0007` and `0017`
Depends on: `0071` (fill paint switch and the kept-data rule), `0049` and `0057` (clones), `0017` (colour rows and picker), `0009` (handles), `0072` (angle stops), `0024` (SVG)
Next: `0123` (engrave a Gradient shape as dots; depends on this story), later slices for multi-stop (Frage 232), radial (Frage 233) and text (Frage 234)
Changes: `specs/0007-stroke-and-fill-styling/`, `specs/0017-style-panel-rework/`, `specs/0071-pattern-fill/`, `specs/0024-svg-import-export/`
Format: the Gradient fill register (criterion 22; architect decides the bump, no migration). Goldens: criteria 13, 14, 22.
PR: -
