# Raster trace: turn an image object into smooth paths with the Trace tool

Status: Draft (2026-10-27; Frage 49 to 60 are open and each has a default built in; needs `adrs.md` from the architect and `ux-notes.md` from the ux-engineer before Ready)
Priority: Must (MVP slice 12, R-VEC-001)
Origin: Customer. Request of 2026-10-14 ("a new tool, settings in the top-middle area, not a property, not the right side; start with Inkscape's functions, check Illustrator and VectorCraft; beat Inkscape: smoother, fewer nodes, fewer wobbly lines, fewer micro paths; bitmap import first"). Decided by the customer on 2026-10-14: the UX mockup of the Trace tool (`ux-mockup.html`) is accepted; 2 to 4 s of compute for a 2000 x 2000 px image is acceptable; the licence is AGPL-3.0-or-later; "VectorCraft" is `storytold/vectorcraft`; tracing should be extensible later (not in the first slice). Everything marked **Proposal** is the product owner's (or the ux-engineer's, from `ux-proposal.md`) and vetoable at the demo.
Depends on: `0058-bitmap-import` (done: image objects, the working image and the pixel-to-millimetre map, criteria 42 and 43), `0050-layers-and-objects-panel`, `0023-groups`, `0054-combine-open-paths` and `0016-boolean-operations` (compound paths with holes), `0020-undo-redo`, `0049`/`0057` not needed.
Inputs: `research.md` (PO, field research and the 8 + 6 questions), `ux-proposal.md` and `ux-mockup.html` (ux-engineer, accepted mockup).

## User value

As a maker with a logo, a scan, a drawing or a photo as a bitmap, I want to select the image, start the Trace tool, see the outline it would make over my picture while I adjust a few settings in millimetres, and press one button to get clean, smooth, light paths, so that I can cut, engrave, plot or edit the shape without redrawing it by hand and without cleaning up a wobbly, node-heavy Inkscape result.

**Field reference (full text in `research.md`).**

- *Inkscape Trace Bitmap.* A docked dialog with Single scan and Multiple scans; potrace underneath (binary bitmap; colour is many threshold passes). Settings in pixels. Known complaints: wobbly curves from staircase input, many micro paths, too many nodes, hairline gaps between colour layers or duplicate edges when stacked.
- *Illustrator Image Trace.* A panel with presets, a live "tracing object" and an Expand step; Noise and Paths in pixels; Abutting and Overlapping layering.
- *CorelDRAW PowerTRACE, Vectorizer.ai, Vector Magic.* Closed benchmarks only; Vectorizer.ai is cloud, so it is never used with customer images (R-SYS-007). Affinity has no automatic trace.
- *VectorCraft (`storytold/vectorcraft`, crate `trace`) and VTracer.* Open source (MIT OR Apache-2.0) tracers in Rust; VectorCraft's pipeline is the one this spec asks for (sub-pixel contours, shared borders, kurbo fit). Their quality is unmeasured; the first task of the build measures them (Frage 49).
- *What is better here.* (1) Settings in millimetres of the placed image, with a pixel hint, not in pixels. (2) Contours are placed at the sub-pixel crossing of the threshold, so edges do not stair-step, and curves are fitted with a stated tolerance, so a result has fewer nodes at the same deviation. (3) The preview is an outline over the dimmed picture at real size, in the bar above the canvas, not a thumbnail in a window. (4) Path and node counts are shown, because they decide how long a job runs on the machine. (5) Presets are named after the job (Logo, Photo, Silhouette for cutting, Engraving). (6) No live object to expand: the result is plain paths and one undo step.

## Terms

- **Image object, working image, pixel map:** as `0058` (Terms and criterion 42): the working image is RGBA8, row-major, at most 16 Mpx and 8192 px on the long side; the pixel map maps pixel coordinates to document millimetres including rotation, skew and flip. The trace reads the working image only, never the file.
- **Foreground:** the pixels the trace turns into shapes. In black and white mode: pixels whose luma is below the **Threshold**. Luma is `0.2126 R + 0.7152 G + 0.0722 B` of the stored sRGB values (0 to 255); transparent pixels are composited over white first (`0058` criterion 7 gives straight alpha).
- **Outline:** one closed boundary of a foreground region (outer) or of a hole in it. **Paths** in the status line means the number of outlines. **Nodes** means anchor points of all outlines.
- **Specks:** foreground regions and holes smaller than the Specks size. The size is the side, in millimetres, of a square of the same area (Frage 51 and `ux-proposal.md` TRACE-UX-UNIT), computed through the pixel map (a rotated or stretched image uses the true area in mm²).
- **Smooth:** 0 to 100; higher means a looser curve fit (fewer nodes, larger deviation). It maps monotonically to the fit tolerance (the mapping is the architect's and the implementer's and is documented in the crate).
- **Trace result:** a plain data value: closed outlines in document millimetres with hole structure, an optional fill colour per outline group, and the counts. It is what the preview draws and what Create paths inserts.
- Lengths in millimetres; computed geometry is compared with the tolerances named in each criterion (pixel distances are measured in pixels of the working image).

## What exists today

Facts from the specs (2026-10-27); the architect confirms in `adrs.md`.

- `0058` is done: an image node, `Import` (rail button, Ctrl+I, drop), the working image and the pixel map (criterion 42) and the guarantee that a traced path lands over the pixels it came from (criterion 43). No Trace tool exists; `0058` built no button called Trace.
- The geometry kernel has compound paths with holes (opposite directions, nonzero fill, `0016`, `0054`), `kurbo` is a workspace dependency (curve fitting, `fit_to_bezpath`), `i_overlay` is the polygon kernel (ADR 0003). No tracing code exists.
- The tool rail has column A with Tools, Group, Path tools and, since the customer change of 2026-10-27, the Import card; the overlay row at 800 x 600 is about 356 px wide with the panel open (`ux-proposal.md` section 3.2, measured for `0035`). `ux-proposal.md` placed Trace in a new "Image tools" card of column A; with the Import card there the room is short, so the placement is re-checked by the ux-engineer (UX notes below).
- Core crates have no file system, network, clock or threads; a platform side runs long work on a background thread (desktop) or a Web Worker (browser) (`CLAUDE.md` §5, §6).

## Slices

The spec is one story in three slices, each a PR the customer can try (one PR per slice, `CLAUDE.md` §4).

1. **Slice 1: black and white trace, end to end.** Select an image, choose the Trace tool, see the preview, adjust Threshold, Smooth and Specks, press Create paths, get one compound path above the hidden image. Includes the tracer core and the quality harness against Inkscape (criteria 1 to 36).
2. **Slice 2: colours, presets, Advanced, several images.** Colours mode (2 to 16, stacked), the four presets, the Advanced block, tracing several selected images (criteria 37 to 50).
3. **Slice 3: large images and comparing.** The View pill and the peek key, reduced-copy preview, Stop, the failure card, the warning over 100,000 nodes (criteria 51 to 58).

## Acceptance criteria

### Slice 1, Part A: the tool and the bar

1. Given the tool rail, then there is a button named **Trace** (tooltip "Trace tool (I)", second line "Turn an image into paths. Enter creates them, Esc cancels.") in a card named "Image tools" of the rail (**Proposal**, `ux-proposal.md` section 1; the ux-engineer re-checks the place now that the Import card exists). It is a tool: it takes the active-tool look, shares the one-active-tool rule, has `aria-pressed`, and its key is **I** (no modifier). Ctrl+I stays Import (`0058`) and is unaffected. The Trace tool is never hidden; it is dimmed for nothing.
2. Given the Trace tool is chosen with **one image selected**, then at once (a) the Select bar is replaced in place by the Trace bar in the overlay row, (b) the picture is dimmed to 35 % (a white wash on the light theme, the canvas colour at 65 % on the dark theme), and (c) the preview of the remembered settings is computed and drawn as a hollow outline (criterion 8). The right panel gets no section, tab or row. Nothing is written to the document.
3. Given the Trace tool is chosen with **nothing selected**, then the picture and canvas are untouched and the bar is one row: "Click an image to trace it." and a text button "Import image..." that does what the rail Import button does (`0058` criterion 1). A click on an image with the tool selects it and starts the preview. With **something else selected** (no image) the bar says "The selection is not an image. Click an image to trace it." and nothing is refused. A locked or hidden image cannot be selected (`0050`) and so is not traced.
4. Given the bar with an image, then it has these groups in this order and wraps by whole groups, the first row never moving (**Proposal**, `ux-proposal.md` section 3; the ux-engineer measures at 800 x 600): **Threshold** (value field 0 to 255, label inside, starts at the image's automatic value and shows it, see criterion 9), **Smooth** (0 to 100, default 60), **Specks** ("Ignore below", mm, 0 to 10, default 0.5, with a muted pixel hint such as "0.50 mm  ~ 2 px" that follows the image's size on the canvas), and the **result row** (status text and the **Create paths** button, 112 px, accent fill). In slice 1 the Mode is black and white only and no Mode control is drawn (a control with one choice is not drawn, `ux-proposal.md` 3.3). The right-hand panel keeps its state.
5. Given the bar's fields, then Up/Down change by 1, Shift+Up/Down by 10 (Specks: 0.1 and 1 mm; Ctrl 0.01 mm), Home/End go to the limits, a typed value commits on Enter, an out-of-range typed value is clamped silently to the range, and Enter in a field commits the value and returns focus to the canvas (the next Enter creates the paths, criterion 12). Changing a value is not a history step. The three values are remembered between uses as an app setting, not in the project file.
6. Given the settings of criteria 4 and 5, then they are described by **one plain data schema** (name, type, unit, range, default, step, group, advanced flag) from which the bar's controls are generated; there is no trait, registry or plugin type for tracers (Frage 55, default A; `CLAUDE.md` §5). Test: a unit test reads the schema and compares it with the defaults and ranges of criteria 4 and 5.

### Slice 1, Part B: the preview

7. Given the preview, then it is an overlay layer, never a document node; it is dropped when the maker leaves the tool (another tool, Escape, selecting a non-image) and it does not appear in the History, in Layers, in a saved file or in any output.
8. Given the preview outline, then it is drawn 1.5 px screen-space wide in the "new" blue of the design system (`--preview-new`) over a 1 px white casing, holes outlined too, no fills, at the real position over the picture; pan and zoom redraw a cached copy culled to the viewport and never trace again.
9. Given a first use on an image, then Threshold starts at the **automatic value** (Otsu's method on the luma histogram of the working image, rounded to a whole number) and the field shows that number; for a picture of one grey level the value is 128. Test: a 100 x 100 px picture with the left half luma 40 and the right half luma 200 gives a value between 41 and 199.
10. Given a change of any value, then the preview is recomputed after **150 ms** without a further change (debounce); while a value is being dragged it is computed on a reduced copy of the working image (longest side 1000 px, area-averaged) and on release at full size; a newer value cancels a stale job. While a job runs the previous outline stays; after 400 ms the status shows "Tracing..." with a 2 px progress line; the status text is announced politely once per settled result, not per tick. The first preview after choosing the tool uses the reduced copy, the full-size one follows.
11. Given a settled result, then the status reads "7 paths, 412 nodes, 0.4 s" (tabular figures; the time is the compute time of the last full trace), plus a muted line "38 specks ignored" when specks were dropped. Test: a fixture with 3 foreground regions, one hole and 5 one-pixel specks at Specks 0.5 mm shows the exact counts.

### Slice 1, Part C: creating the paths

12. Given an image selected in the Trace tool, then the paths are created by (a) the **Create paths** button (tooltip "Create paths (Enter)"), (b) Enter with focus on the canvas or on a bar button (Ctrl+Enter from inside a field), or (c) a double-click on the image. **Escape** stops a running job (first press) or leaves the tool and drops the preview (second press); there is no Cancel button and no dialog, because nothing was changed.
13. Given Create paths, then **one history step "Trace image"** (`0020`) does all of this: it inserts **one group** named "Trace of <image name>" (cleaned name, 128 characters) **directly above the image** in the same layer or group; inside it, in black and white mode, **one compound path** with the holes as outlines of opposite direction (`0016`, `0054`), filled black (#000000, opacity 1), stroke off (**Proposal**); it **hides the source image** (visible = false, not deleted, not locked; Frage 54, default B); it selects the new group and switches to the **Select tool**. A success notice (Action notice next to the rail, `role="status"`, 3 s) reads "Traced: 7 paths, 412 nodes. Source image hidden." Ctrl+Z removes the group and shows the image again; Ctrl+Y brings both back. Test: one history step, the group's position directly above the image, the image's visible flag, the tool afterwards.
14. Given an image that is inside a group or has an opacity, a rotation, a skew or a flip, then the group is inserted in the same parent and the paths sit on the picture pixel for pixel through the pixel map of `0058` criterion 42: for a 4 x 2 px fixture imported into a 40 x 20 mm frame, rotated 90 degrees and flipped horizontally, every outline point equals the point obtained by mapping its pixel position through the hand-computed affine within 1e-6 mm. The image's own opacity is not copied to the paths.
15. Given a result with **0 paths** at the current settings, then the status reads "0 paths. Try moving Threshold toward lighter." (or "darker"; the direction is computed from the histogram), Create paths is `aria-disabled` and the status says why. Given a result that is **one path covering the whole image** (a full black page), then the status reads "1 path covering the whole image. Check Threshold." and Create paths works (permit over refuse).
16. Given the tool, then nothing in it is refused on size, format or unsuitable content: an image of any size within the working limits of `0058` traces; the status says what will happen.

### Slice 1, Part D: the tracer

17. Given the tracer, then it lives in a new `*-core` crate (the architect decides name and ADR; `curvyo-trace-core` proposed) as **one pure function** `trace(image, pixel_map, params, cancel) -> result`: no file system, network, clock, threads or `unsafe`; builds for `wasm32-unknown-unknown`; takes a cancel check (a callback that is polled) so a stale job stops. Test: wasm build; after the cancel check first returns true the function returns `Cancelled` after at most 250 ms of thread CPU time on the 2000 x 2000 px fixture.
18. Given the same image and settings, then the result is the same on every run and every operating system of the same CPU family; native and wasm builds agree within 1e-6 mm (compare with a tolerance, golden files in `tests/fixtures/trace/` hold the counts, bounding boxes and a handful of points). No result depends on hash-map order, the clock, the locale or thread scheduling.
19. Given the pipeline, then (the architect fixes the algorithms) contours are extracted at the **sub-pixel** position where the luma field crosses the Threshold (linear interpolation between pixel centres), not along pixel borders; corners are detected and kept sharp; the runs between corners are fitted with cubic Beziers within the tolerance given by Smooth (kurbo, Schneider-style); straight runs become lines; all outlines are closed; foreground specks and hole specks below the Specks size are removed (a foreground speck becomes background, a hole speck is filled), and the result has no outline with an area below the Specks size when Specks is above 0.
20. Given the black-on-white rectangle fixture (200 x 100 px, a black rectangle at x 50 to 150, y 25 to 75, placed at 96 dpi, so 0.264583 mm per pixel), then at the defaults the result is **one closed outline of at most 8 nodes** whose corners are within 0.25 px of (50, 25), (150, 25), (150, 75), (50, 75) in pixel space (13.229, 6.615), (39.688, 6.615), (39.688, 19.844), (13.229, 19.844) mm within 0.07 mm.
21. Given the anti-aliased disc fixture (a black disc of radius 100 px on a 500 x 500 px white picture, edges anti-aliased), then at the defaults the result is one closed outline of **at most 16 nodes** and every point of the outline is within 0.75 px of the true circle, the mean distance at most 0.3 px.
22. Given the "ring with a dot" fixture (an outer ring, a hole, a dot inside the hole), then the result has three outlines, the hole in the opposite direction of the ring, the dot in the direction of the ring, and the filled area of the compound path (nonzero) equals the foreground area of the source within 1 %.
23. Given Smooth, then a higher value never produces more nodes: on each fixture of the quality set, nodes at Smooth 80 are no more than at Smooth 60, and those at 60 no more than at 20 (monotone). Given Threshold, then raising it never removes foreground pixels from the result: the filled area at Threshold t+10 is at least that at t within 0.5 %.
24. Given a transparent PNG (alpha 0 around a black logo), then transparent pixels count as white (background) and the result is the logo outline; given a grey scan with uneven lighting the result is whatever the Threshold gives (no hidden adaptive step in slice 1).
25. Given the budgets (customer decision of 2026-10-14: 2 to 4 s is acceptable for a 2000 x 2000 px image), then a full-size black and white trace of a 2000 x 2000 px working image of the line-art fixture takes **under 4.0 s of thread CPU time** (native release, `#[ignore]` benchmark on the build host, as `0056` criterion 27); the measured time is reported in the PR, and the wasm times are reported, not gated. The target (not gated) for black and white is 0.5 s. Preview timings (reduced copy) are reported.
26. Given an image of 16 Mpx (the working limit), then the trace completes with a memory use of at most the working image plus 16 bytes per pixel of intermediate data (reported in the PR; the architect sets the measure) and never aborts the tab; the long run is stoppable through the cancel check.

### Slice 1, Part E: quality against Inkscape

27. Given the **quality harness** (a test binary in the tracer crate, no UI; the fixtures in `tests/fixtures/trace/`), then it computes for each image of the black and white test set the metrics of `research.md` section 5 and compares them with a stored baseline of Inkscape's Trace Bitmap (potrace, default settings and best manual settings), kept as a JSON file of numbers only (no Inkscape output is committed, `research.md` section 7 item 5). Metrics: node density (nodes per 100 mm of path), micro paths (outlines below 0.25 mm²), max and mean deviation from the thresholded reference rendered at 4x, area error (XOR as % of foreground), wobble (curvature sign flips per 100 mm), closedness.
28. Given the black and white test set (images 1, 3, 4, 5, 7, 10, 11, 12 of `research.md` section 5: flat logo, black text at 300 dpi, small low-resolution text, line art scan, silhouette with fine holes, pixel art, thin-line stencil, technical drawing; plus the 3 to 5 images the customer sends, Frage 53), then at the **default settings** the targets are (**Proposal**, Frage 53 default A): node density at most 60 % of Inkscape's default on each image and in the mean; 0 outlines below 0.25 mm²; maximum deviation at most 0.75 px (1 px for the photo-like images), mean at most 0.3 px; area error below 1 % for logos and text and below 3 % for the rest; wobble at most 50 % of Inkscape's; 100 % closed outlines. A miss is reported by the harness per image and metric; the PR description lists the table. Fixtures that are photographs or customer files carry a note on their origin; none is copyright-sensitive.
29. Given the first task of the build ("step 0", Frage 49), then VectorCraft's `trace` and VTracer are run through the same harness as dev tools (not shipped dependencies) and their numbers are stored beside Inkscape's; the architect's ADR states which approach is built from them. If code of either is reused it is MIT OR Apache-2.0, notices are kept, and the ADR lists the files.

### Slice 1, Part F: other specs, documents

30. Given the Layers tab (`0050`), then the group "Trace of <name>" appears as one row with the group glyph and the compound path inside; the hidden image shows the eye-off state, so the maker can show it again there; the Layers row tooltip repeats the success notice.
31. Given the Select tool after Create paths, then the new group moves, scales, rotates and edits like any group; the result is independent of the image (moving the image later does not move it).
32. Given the machine slices (`0026` to `0029`), then the result is ordinary paths and takes part in every output like them; this spec writes no output code and no role.
33. Given the project file, then no `format_version` change is made: the result is a group and a compound path (existing kinds); the Trace settings are an app setting outside the project.
34. Given the trace, then nothing is sent anywhere: the tracer is a pure function and the tool makes no network request (R-SYS-007). A one-line note in the user documentation says that the maker is responsible for the right to trace an image (`research.md` section 7 item 7).
35. Given the work is done, then `docs/design-system.md` has the rows for the rail button and key, the Trace bar, the preview look, the notices of this slice and the new shortcut `I` (in the shortcut table), and the user documentation has a page "Tracing an image" with the settings, the presets (slice 2) and the units; every notice quoted here is the same string in code and in the page (a test compares them).
36. Given the quality set, the nightly job runs the harness and reports the table; a regression of any metric by more than 5 % against the committed run fails the nightly job (not the PR gate).

### Slice 2, Part G: colours, presets, Advanced, several images

37. Given the bar in slice 2, then a **Mode** pill "B/W | Colours" (104 px) joins row 1 and a **Preset** menu button (132 px, shows the preset name or "Custom") precedes it; **Threshold** (B/W) and **Colours** (2 to 16, default 8) share one slot of the same width, so switching Mode moves nothing else.
38. Given Colours mode with N colours, then the working image is reduced to N colours by quantisation that does not create colours from anti-aliasing blends (the pixels between two regions join the nearer region), regions are despeckled by merging a region below the Specks size into its **most common neighbour** (no holes appear), contours are traced once per colour and the result is **stacked** (Frage 52, default: stacked in this slice, "no gaps" in a follow-up): paths are ordered from the largest total area (bottom) to the smallest, and each colour's shape is extended to cover the regions above it, so no gap can appear. Each colour group is one compound path filled with the quantised colour (sRGB, rounded to whole values), stroke off, in the group "Trace of <name>".
39. Given a Colours trace of the 3-colour logo fixture, when the result is rasterised over white at 4x, then every pixel except those within 1 px of a colour boundary has the quantised colour of the source pixel, and **no pixel is uncovered** (0 gaps). Test: count uncovered pixels.
40. Given the four presets, then the Preset menu (`role="menu"`, arrows, Enter, Escape) lists them with a second line each and sets every value including Advanced ones (**Proposal**, `ux-proposal.md` 3.4): **Logo / Line art** (B/W, Smooth 60, Specks 0.5 mm, Upscale Auto, Corners 70; "Sharp corners, clean curves."), **Photo** (Colours 8, Smooth 70, Specks 1 mm, Denoise Low, Node limit 20,000; "Many shapes. Photos trace roughly."), **Silhouette for cutting** (B/W, Smooth 75, Specks 1 mm, Ignore background, Corners 50; "Closed, simple, few nodes."), **Engraving** (B/W, Smooth 30, Specks 0.1 mm, Corners 50; "Keeps fine lines."). Editing any value turns the button into "Custom"; picking the preset again restores it. With Photo chosen a muted line reads "Photos trace into many shapes. Fewer colours and a larger Specks value give lighter results." The Silhouette value for Specks is a fixed 1 mm until a machine profile can give a kerf.
41. Given the **Advanced** button (always present in row 3, never hidden or dimmed, `aria-expanded`), then it opens a block below the result row inside the same pill, in two columns, so the rows above do not move; open or closed is remembered. Contents: **Denoise** (Off | Low | High: a median/bilateral filter before thresholding), **Upscale** (Auto | Off: 2x before tracing; Auto means images under 1 Mpx), **Corners** (0 to 100: how readily a bend becomes a sharp corner), **Node limit** (number field, empty = none; raises the fit tolerance until the result fits), **Invert** (trace the light parts), **Ignore background** (drop the lightest region), **Full-size preview** (off by default).
42. Given Denoise Low on a JPEG fixture with ringing around a black logo (noise amplitude 20 grey levels), then the result has no outline outside the logo's outline and its max deviation is at most 1 px. Given Upscale Auto on the 120 x 60 px low-resolution text fixture, then the outline is traced from a 2x copy and the deviation to the 4x reference is lower than without Upscale (test compares both).
43. Given Node limit N and a result that would exceed N nodes, then the tracer raises the tolerance (bisection) until the result has at most N nodes or the tolerance reaches a ceiling; the status then reads "Node limit reached: 20,000 nodes, tolerance raised." (if the ceiling stops it, "Node limit could not be met: 31,400 nodes."). A result at or under N is untouched.
44. Given Invert, then foreground is luma above the Threshold; given Ignore background, then the region of the picture border with the lightest colour is dropped from the result. Test: a black logo on white with Invert gives the white background shape with the logo as a hole.
45. Given the Colours or Advanced controls, then they are described by the same schema as in criterion 6 (a test compares the preset table with the schema).
46. Given several images selected (**Proposal**, `ux-proposal.md` section 8; TRACE-UX-MULTI default A), then the bar says "Tracing 1 of 3 images" and previews the image clicked last; **Create paths** traces all selected images with the same settings in **one history step "Trace image"** (one group per image, each directly above its image); the success notice names the totals ("Traced 3 images: 21 paths, 1,204 nodes.").
47. Given a colour result, then the status line and notice give the counts of all colour groups and the time; "paths" is the number of outlines of all colours.
48. Given the Layers tab, then a colour result is one group per image with one compound path per colour, named by the colour's hex value ("#1a2b3c"), the largest area at the bottom.
49. Given the quality harness, then the colour set (images 2, 6, 9 of `research.md` section 5 plus the customer's images) is run in Colours mode at 3, 8 and 16 colours with the metrics that apply (node density, micro paths, deviation to the quantised reference, gaps and overlaps along shared borders in stacked mode as the count of uncovered pixels, which must be 0), against Inkscape's multiple-scan baseline; the targets of criterion 28 apply except that wobble and area error are reported, not gated, for photos.
50. Given budgets, then Colours mode with 8 colours on a 2000 x 2000 px working image takes under 4.0 s of thread CPU time (customer limit; the research target of 2 s is reported, not gated); Upscale of a 4 Mpx image (16 Mpx of work) takes under 4.0 s too and is off by default above 1 Mpx.

### Slice 3, Part H: large images, comparing, failure

51. Given the **View** pill in row 1 (three icon segments with tooltips: Overlay (default), Trace, Image), then Overlay shows the dimmed picture and the outline, Trace shows the result as it would be created with its fills and the image hidden, Image shows the original at full strength without outline; **hold `\`** shows the original while held and returns; **Shift+`\`** cycles the three views; zoom and pan work as usual in every view. No split view in this story.
52. Given an image over **4 Mpx**, then the preview is always the reduced copy (longest side 1000 px) and the status adds "Preview from a reduced copy"; Create paths traces at full size with the progress line and a **Stop** button; the document is untouched until the trace finishes. The working image of `0058` is at most 16 Mpx, so the 25 Mpx half-size rule of `ux-proposal.md` section 5 is not needed (**Proposal**: dropped).
53. Given a job that runs longer than **1.5 s**, then the status row adds a text button **Stop**; after Stop (or Escape) the old outline is drawn at 50 % and a muted line reads "Preview is out of date"; changing a value or pressing Enter traces again.
54. Given a trace that fails (out of memory, internal error), then a message card under the bar (`--field-invalid`, `role="alert"`) reads "Trace failed: <reason>. Your drawing is unchanged." with a **Retry** button; the bar values stay and the document is exactly as before.
55. Given a result of more than **100,000 nodes**, then the status reads "412,000 nodes. Slow to edit; raise Smooth or Specks." and Create paths still works. Over **20,000 nodes** the result is inserted as one group in one step and the Layers tab shows it as one row.
56. Given a pan or zoom with a preview of 100,000 nodes, then the redraw takes under 16 ms of frame time on the reference GPU of the renderer tests (cached, culled outline).
57. Given the keyboard route, then Tab enters the bar from the canvas in this order: Preset, Mode, View, Threshold or Colours, Smooth, Specks, Advanced, the Advanced controls when open, Create paths; pills and the View control move with arrows; the Preset menu opens with Enter or Down. Hit targets are at least 28 x 28 px, focus ring 2 px `--editor-accent`, the stale state is stated in words, the progress line has `role="progressbar"` and `aria-busy` on the bar while a job runs.
58. Given the browser build, then the trace runs in a Web Worker and the desktop build on a background thread; the bar stays responsive during a trace (no input handler blocks for more than 50 ms in the timing test of the editor harness; measured as thread CPU time of the main thread in the test).

## Out of scope

- **Extension points:** a tracer trait, registry, plugin type, WIT image type, an MCP tracer, a local or remote ML step, a cloud service (Frage 55 to 60 defaults: nothing built; `research.md` section 10 stays a design note). Nothing leaves the machine.
- **Gap-free "cutout" layering (shared borders), centerline strokes and primitive recognition** (circles, rectangles from contours): follow-up slices. Stacked is the only layering in this story (Frage 52).
- **Saving own presets**, a machine link for the Silhouette preset's feature size (`0026`), tone-step (grey level) mode, edge detection mode.
- **A live "tracing object"** tied to the image (Illustrator's Expand model): the result is plain paths.
- **Tracing a crop or selection** of an image, cropping, brightness or contrast editing of the picture.
- **A split-view compare** (a draggable divider): the View pill and the peek key are the comparison.
- **Raster engrave** of the image (the laser line, `0058` criterion 41).
- **Machine learning, AI upscaling, background removal, OCR** (to text).
- **Tracing groups or paths** (already vector) and **tracing a locked or hidden image** (it cannot be selected, `0050`).
- **Porting potrace** (GPL-2.0-or-later, compatible but not chosen: `research.md` section 8, approach A).
- **Editing the Trace result live** after Create paths (it is ordinary paths; there is no re-trace).

## Decided by the product owner (Proposal, vetoable at the demo)

- **D1. Three slices, B/W first.** R-VEC-001 asks for threshold and colour-count settings (Frage 50 default B); the thinnest end-to-end slice is black and white, which already covers laser, plotter and vinyl silhouettes. Colours, presets and Advanced follow in slice 2.
- **D2. The result is a group above the image, the image is hidden, the tool returns to Select** (UX proposal sections 6.3, TRACE-UX-SOURCE A, TRACE-UX-AFTER A). The group wrapper in black and white keeps slices 1 and 2 consistent.
- **D3. One labelled button "Create paths" plus Enter and double-click; Escape cancels; no Apply/Cancel pair** (TRACE-UX-COMMIT A). A trace has no handle to release; the customer's rule "operations have a visible trigger" needs the button.
- **D4. The preview starts when the tool is chosen** (TRACE-UX-AUTOPREVIEW A), on a reduced copy first.
- **D5. Specks is the side of an equal-area square in mm, with a pixel hint** (TRACE-UX-UNIT A, Frage 51 C): a length compares directly to a feature or a kerf.
- **D6. Black fill, no stroke, for the B/W result** (cut jobs decide by role later, `0027`).
- **D7. Key I, an "Image tools" card** (TRACE-UX-KEY A, TRACE-UX-RAIL A), re-checked by the ux-engineer because the Import card now follows the Path tools card.
- **D8. Luma by Rec. 709 weights on stored sRGB values, no colour management** (`0058` D10).
- **D9. Quality targets are acceptance gates of the harness, not of the PR gate**: a miss is reported per image and metric. The nightly job reports regressions (criterion 36).
- **D10. The trace reads the working image (at most 16 Mpx), not the original file** (`0058` criterion 42); the original stays in the asset.
- **D11. Dropped from the UX proposal:** the 25 Mpx half-size rule (the working image is at most 16 Mpx, criterion 52); a locked-image row (a locked image cannot be selected).

## Decided by the customer

- **2026-10-14:** the UX mockup of the Trace tool (settings in the top-middle bar, preview, Create paths) is accepted; 2 to 4 s of compute for a 2000 x 2000 px image is acceptable (criterion 25); the licence is AGPL-3.0-or-later, so GPL-2.0-or-later code is compatible and only GPL-2.0-only is not; "VectorCraft" is `storytold/vectorcraft`; tracing is to be extensible later, not in the first slice.
- **VEC-NAME (research Frage 1):** answered, see above.
- **VEC-UI (research Frage 4):** answered, the mockup is accepted.
- **2026-10-27:** bitmap import is done (`0058`) and the build order puts `0025` first: 0025, 0059, 0060, 0061.

## Open questions (defaults built in)

Numbering continues the repo's: Frage 48 was the last used. Each is the research's question with the research's recommendation as default; the spec is built on the default and changes only if the customer answers otherwise.

**Frage 49 (VEC-APPR): Which approach builds the tracer?** (research Frage 2)
- A: Port potrace (GPL-2.0-or-later, licence compatible; quality is the limit: binary input, no sub-pixel, no shared borders).
- B: Take VTracer/visioncortex as a dependency (MIT OR Apache-2.0) and add pre- and post-processing.
- C: Our own pipeline (sub-pixel contours, kurbo fit, mm tolerances, node limit), with a "step 0" that first measures VectorCraft's `trace` and VTracer in the harness and may fork a few files of VectorCraft (MIT OR Apache-2.0).
- Recommendation: C with step 0. Default: C with step 0 (criterion 29); the architect writes the ADR for the new crate or the vendored code.

**Frage 50 (VEC-SCOPE): What is in the first slice of 0025?** (research Frage 3)
- A: Black and white only plus presets.
- B: Black and white plus colour count (R-VEC-001 as written).
- C: Also gap-free colour layers and centerline.
- Recommendation: B, delivered in the slices above (B/W in slice 1, colours in slice 2), gap-free layers and centerline in a follow-up. Default: B.

**Frage 51 (VEC-UNITS): Units for the despeckle and tolerance settings?** (research Frage 5)
- A: mm, using the image's physical size.
- B: Pixels, like Inkscape and Illustrator.
- C: mm with a pixel hint next to it.
- Recommendation: C. Default: C (`0058` gives a size for every image, so no dpi question arises).

**Frage 52 (VEC-GAPS): Colour layers: gap-free cutout or stacked?** (research Frage 6)
- A: Cutout (shared edges; best for cutting and vinyl).
- B: Stacked (best for editing and plotter colours).
- C: The maker chooses.
- Recommendation: C, with stacked first because it is simpler and cutout in the follow-up. Default: stacked only in this story (C later); the Layering control is not drawn until the second option exists.

**Frage 53 (VEC-TARGET): Are the quality targets acceptable?** (research Frage 7)
- A: As proposed (node density at most 60 % of Inkscape's, no outlines below 0.25 mm², at most 0.75 px deviation, criterion 28).
- B: Stricter.
- C: Only your visual comparison.
- Recommendation: A plus your visual check on 3 to 5 of your own images that you dislike in Inkscape, sent as test fixtures (nothing copyright-sensitive). Default: A.

**Frage 54 (VEC-SOURCE): What happens to the source image after Create paths?** (research Frage 8; the same as TRACE-UX-SOURCE)
- A: It stays visible, the paths sit on top.
- B: The image is hidden; the eye in the Layers tab shows it again.
- C: The maker chooses with a switch in the bar.
- Recommendation: B. Default: B (criterion 13).

**Frage 55 (EXT-SEAM): How much of the extension seam do we build?** (research 10.7 Frage 1)
- A: Nothing; the tracer is one function with a parameter schema (which the bar needs anyway).
- B: Also write the WIT image type into the ADR 0005 interface now.
- C: A tracer trait now, with the built-in tracer as the only implementation.
- Recommendation: A (C breaks `CLAUDE.md` §5, B belongs to the plugin-host story). Default: A (criterion 6).

**Frage 56 (EXT-FIRST): Which extension would you want first?** (research 10.7 Frage 2)
- A: A local ML pre-step (denoise/upscale or background removal).
- B: An MCP tracer.
- C: A plugin with your own algorithm.
- D: VTracer as an alternate backend.
- E: None yet.
- Recommendation: D as a benchmark first, then C once the plugin host exists. Default: E; no story is written.

**Frage 57 (EXT-PRIVACY): May a remote tracer (MCP or cloud ML) exist at all?** (research 10.7 Frage 3)
- A: Never in the product, only local.
- B: Yes, per extension with a consent dialog before the first upload and a visible mark.
- C: Yes, with one global setting.
- Recommendation: B, if remote ever comes. Default: A until you decide; no story proposes remote (criterion 34).

**Frage 58 (EXT-ML): Extension only, or may the product ship its own model?** (research 10.7 Frage 4)
- A: Extension only; the app ships no model.
- B: The app ships one small model (background removal or classification), licence permitting.
- C: Decide later per model.
- Recommendation: C; when it comes, denoise/upscale and background removal first. Default: A for now.

**Frage 59 (EXT-DETERM): May ML steps make results non-reproducible?** (research 10.7 Frage 5)
- A: Yes, marked non-deterministic; the result (paths) is stored, never re-run.
- B: No, only deterministic steps are allowed.
- C: Deterministic by default; ML allowed with an explicit mark.
- Recommendation: C. Default: C (the built-in tracer is deterministic, criterion 18).

**Frage 60 (EXT-VCRAFT): May we fork parts of VectorCraft's tracer (MIT OR Apache-2.0) after measuring?** (research 10.7 Frage 6)
- A: Yes, after step 0 and an ADR, with notices kept.
- B: Idea source and benchmark only; we write everything ourselves.
- C: Depend on it as a crate.
- Recommendation: A only if it beats Inkscape in the harness, otherwise B; C is not recommended (alpha, internal types, weekly API changes). Default: B until step 0 has numbers.

## Notes for the architect (`adrs.md`; the product owner does not decide these)

- **New crate and ADR** for the tracer (`*-core`, wasm32, `#![forbid(unsafe_code)]`), its public function, the cancel mechanism and the Smooth-to-tolerance mapping; whether the quality harness lives in the same crate (dev tool) and how it runs the external tools.
- **Dependency licences to confirm with `cargo deny`:** `kurbo` (already in the workspace, Apache-2.0 OR MIT); any code or crate taken from VectorCraft `trace` (stated MIT OR Apache-2.0, workspace `Cargo.toml`; internal alpha crate tied to `vectorcraft-geom` and `image`; check crates.io, transitive dependencies, `unsafe`); VTracer/visioncortex (stated MIT OR Apache-2.0, some bundled files MIT/BSD-2-Clause; used as a dev-tool benchmark only unless Frage 49 changes); a median/bilateral filter or quantiser crate if one is wanted instead of own code (permissive only). A potrace port would be a derivative of GPL-2.0-or-later code and needs an allow-list decision under ADR 0006 and loses dual-licensing for that module; it is not planned. Inkscape and potrace run offline for baseline numbers only; none of their output is shipped or committed.
- **Background execution:** desktop thread versus browser Web Worker, the cancel check, and the `trace` call on a reduced copy.
- **Determinism** of floating point between native and wasm within 1e-6 mm (criterion 18).
- **Open ADR 0005 item:** whether a future plugin tracer needs an image type (nothing is built now, Frage 55).

## UX notes

`ux-proposal.md` and `ux-mockup.html` (accepted by the customer 2026-10-14) are the base. The ux-engineer writes `ux-notes.md` before Ready and covers: the rail placement now that the Import card follows Path tools (criterion 1), the bar's rows at 800 x 600, the Trace bar when the properties panel is collapsed, the dark-theme dim wash, the notices of this spec, and the rows for `docs/design-system.md`.

## Links

Requirements: R-VEC-001 (Must), R-VEC-003 (node reduction at a stated deviation, Should), R-VEC-004 (live preview, now Should, mockup accepted), R-VEC-002 (combining passes, Should; not in this story); R-EDIT-037 (the image); R-SYS-007 (nothing leaves the machine).
Related specs: `0058-bitmap-import` (image object, working image, map), `0050`, `0023`, `0016`, `0054`, `0020`, `0038` (the Offset tool as model for a tool with a bar), `0027`/`0029` (outputs), `0059` to `0061` (the next in the build order).
Sources: `research.md` (with its sources), `ux-proposal.md`.
PR: none yet.
