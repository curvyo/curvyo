# Trace quality and colour: intended geometry, colour selection, multicolour, automatic analysis

Status: Ready (slices A and D done, PR #223; slice C done, PR #226; slice E in progress)
Priority: Must (slices A, D, C, E); Should (slices M, F); Could (Later: G, H)
Origin: Customer, 2026-10-29: he handed over a German proposal document from another chat ("Tracing weiterentwickeln", dated 2026-10-28; the document calls itself a proposal for review, not an order for all stages) with the words "after that we continue with tracing; we now simply bring it to a very, very cool level." The direction, target material, priorities and work packages below are the document's. Slicing, numbers and everything marked **Proposal** are the product owner's and vetoable.
Depends on: `0025-raster-trace` (slices 1 to 3; the Trace tool, bar, presets, harness, ADR 0023). Feeds: `0088-trace-recognition` (text first, glyph matching; shapes later).

## User value

As a maker who turns logos, clipart, line drawings, sketches, AI-generated black and white drawings or my own paper drawings (scanned or photographed) into paths for laser engraving, cutting, plotting or embroidery, I want sharp corners where I meant corners, real straight lines, calm curves, few but sufficient nodes, and a way to pick exactly the colours I want (also pale ones, also several at once, or each colour as its own shape), so that the paths are usable for production without redrawing and without node cleanup.

**Target material.** Primary: graphics and clipart, logos, line drawings, sketches, AI-generated black and white drawings, paper drawings as scan or photo. Secondary: diagrams, general photos (photos matter mainly as a way to capture a paper drawing). Keeping the hand-drawn character and cleaning up a template are both legitimate goals the maker chooses.

**Priorities (in this order).** 1. Keep content and structure (details, holes, thin lines, connections). 2. Hit the intended geometry (corners, straight lines, smooth curves). 3. Fewer nodes at comparable quality. 4. Fast and understandable. Fewer nodes alone is no proof of quality; cleanup never turns missing information silently into confident geometry.

**Field reference (from memory, to be checked before Ready).**

- *Inkscape Trace Bitmap (potrace).* Brightness cutoff, edge detection, colour quantisation, multiple scans stacked or not; potrace already separates corner threshold (`alphamax`) from curve optimisation tolerance (`opttolerance`). Colour scans give hairline gaps or stacked duplicates; no colour pipette; centerline only via the autotrace mode.
- *Illustrator Image Trace.* Paths, Corners and Noise as separate sliders; palette limited/full/custom with swatch library; Abutting or Overlapping; "Ignore white"; live view of Outlines, Outlines with source, Source image.
- *LightBurn Trace Image.* Cutoff and Threshold, "Ignore less than", Smoothness, Optimize, and "Sketch trace" for line art; single colour only; no shared boundaries.
- *Ink/Stitch.* No tracer of its own; relies on Inkscape's, then auto-fill. *VTracer.* Colour mode stacked or cutout, no shared nodes at junctions.
- *What we do better.* (1) Corners are found on the source contour before the fit, so a corner is never rounded first and guessed back. (2) Pale and near-grey colours are selectable by pipette with a mask view that shows what is captured. (3) Multicolour shapes share one fitted boundary with common nodes at triple junctions: no gaps, no overlaps, also after simplification. (4) The deviation is measured, not sampled, in source pixels. (5) Automatic analysis says what it saw and why, and never overrides a manual change. (6) Everything stays local (R-SYS-007).

## Terms

- **Selection method:** how pixels become content: **Brightness** (today's Threshold and Invert), **Remove background** (distance to a background colour), **Pick colours** (similarity to one or more picked swatches).
- **Membership:** a value 0 to 1 per pixel saying how much the pixel belongs to the selection (not a probability). The tracer gets a binary mask from it; sub-pixel use is Later (G).
- **Mask:** the binary image handed to the tracer; the Mask view shows exactly this.
- **Structural corner:** a corner detected on the source contour before smoothing, stable across scales; the fit splits the outline there.
- **Deviation tolerance:** the largest allowed distance, in source pixels, between the simplified outline and the contour before simplification, measured densely both ways (every 0.1 px).
- **Single colour / Multicolour:** single colour unions the selected parts into shared shapes; multicolour gives separate, filled, editable colour shapes.
- **Adjacent (abutting):** neighbouring colour shapes share one fitted boundary. **Stacked:** shapes overlap; Later.
- **[T]:** a target number that is a starting point and is **frozen after slice A** from the measured baseline (criterion 4); the relation stated with it is the criterion. Frozen on 2026-10-29 (see "Frozen targets"); the colour distances 0.02 and 0.03 frozen on 2026-10-30 after trial C1, in the sRGB unit cube (stored bytes / 255, Euclidean).

## Slices

Each slice except A is one PR the customer can try end to end on his own images (`CLAUDE.md` §4). Work package B of the document (UX model, data flow, tolerances, old behaviour) is not a slice: it is the Ready step (`ux-notes.md`, `adrs.md`, an amendment of ADR 0023, not a silent overwrite).

| Slice | Content | Demo |
|---|---|---|
| A | Reference set, metrics, baseline, frozen targets | No PR; first commits of slice D's branch; the PR shows the table |
| D | Geometry: corners before fit, line fit, decoupled simplification | Same bar; corners, straights and node counts visibly better |
| C | Selection methods: Brightness, Remove background, Pick colours; alpha semantics | Method control and pipette in the bar |
| E | Integration: Mask view, bar order, migration, docs | The whole single-colour tool as one |
| M | Multicolour, adjacent shapes with shared boundaries | Single colour / Multicolour, palette |
| F | Automatic analysis (heuristic, no ML) | Profile line, "Analyse again" |
| Later | G: sub-pixel, lighting correction, regional; H: centerline, shape objects, OCR (`0088`) | - |

Order decided (Frage 80 A): A, D, C, E, M, F; centerline stays in H after multicolour (Frage 81 A); C and D are independent; M needs C and D; F comes after the manual methods are good. A trial in D or C may end with "no advantage"; the PR then says so and the slice ships what helped.

## Acceptance criteria

### Slice A: reference set and baseline

1. Given the fixtures in `curvyo-vectorize-core/tests/fixtures/trace/`, then there are at least 3 images per primary group (graphics/clipart, logos, line drawings, sketches, AI black and white drawings, paper scan, paper photo) and at least 1 diagram and 1 photo; each has a sidecar note with origin, rights (Frage 82) and intended use (area engraving, cut contour, line plot). Note (2026-10-29): real images alone are 1 to 3 per group (CC0/public domain); the synthetic fixtures fill the count of 3. The customer's own images (Frage 82 A: "I can provide reference images. Tell me what you need. We make them public or use public images.") are committed as public fixtures: each with a sidecar note "own work" and the licence he states (e.g. CC0); third-party images only if public domain or CC0. Needed from him: 10 to 20 images covering the groups above, each with a one-line intended use, and his word on the licence. Until they arrive the set stays as is; their numbers are then added to `baseline.json` like any fixture (no `local.<hash>` entries needed).
2. Given the synthetic fixtures (rendered from known vectors, the truth kept beside them), then they cover: sharp and round corners; straight lines at 0, 7, 15, 30 and 45 degrees; circles; webs 1 and 2 px wide; parallels 1 px apart; hatching; stippling; open and nearly closed strokes; pale colours; transparency; grey seams; JPEG quality 50; scan noise; paper texture; a shadow; a lighting gradient; small text with Ä, Ö, Ü, ß; each at three resolutions (1x, 0.5x, 0.25x).
3. Given the harness (0025 criterion 27), then it also measures: two-sided boundary deviation (max and mean, every 0.1 px, against the vector truth or the 4x reference); corner position error per annotated corner; straightness (max distance from the true line and segment count per annotated edge); topology (components, holes, connections versus the truth, as lost and extra counts); self-intersections; paths and nodes; thread CPU time native and wasm; peak memory (counting allocator).
4. Given the baseline run (today's pipeline, each 0025 preset), then `baseline.json` holds it, the lead confirms the measuring method, and this spec gets a "Frozen targets" table replacing every [T] by a number. No product code changes in slice A. Done 2026-10-29 (Proposal of the lead, the customer confirms at D's demo); the measuring method of `plan.md` is confirmed.

### Slice D: intended geometry (single colour, Brightness)

5. Given the corner fixtures (squares rotated 0, 15, 30, 45 degrees; a triangle; a five-point star with 36 degree tips), then every annotated sharp corner is a node of the result within **0.5 px** of the true corner at 1x and 0.5x (at 0.25x only reported); 0025 criterion 20 still holds.
6. Given an annotated straight edge longer than 20 px at any angle of criterion 2, then it is at most **2** line segments, deviates at most **0.35 px** from the true line, and has 0 curvature sign flips (no staircase wobble); the deviation and segment bounds apply at 1x and 0.5x, at 0.25x they are only reported.
7. Given round transitions (a rounded rectangle with 10 px radius) and the sketch fixture with a bowed edge (2 px sagitta over 200 px), then neither is straightened or cornered: deviation from the true arc and bow at most **0.5 px**, no structural corner on them (0 false corners).
8. Given the parameter schema (0025 criterion 6) and the bar (Frage 85 B), then input smoothing, deviation tolerance (source px) and corner protection are separate parameters, and the bar shows two controls: **Smooth** (calmness, 0 to 100) and **Tolerance** (px, shown with its unit); **Corners** stays in Advanced; changing only the tolerance leaves every protected corner's position unchanged within 1e-6 px. The Node limit and the Full-size preview are removed (customer, 0025 Frage 74 A, merged); there is no node-budget control and no internal budget.
9. Given the set at Logo / Line art, then the geometric mean of nodes per 100 mm is at most **75 %** of the baseline (19.4 nodes per 100 mm at Logo; straight edges carry about 3 times the needed segments; 85 % only if D's trial shows 75 % is unreachable without losing quality, reported in the PR) while the guard of criterion 9a holds (replaces "no image worse than 5 % on any metric", Proposal of the lead 2026-10-29, the customer confirms at D's demo).
9a. Guard per image (replaces the per-image +5 % on every metric of 0025 criterion 28 for the geometry metrics; the baseline is re-measured with the 8-connected Specks reference first, Frage 96). **Hard:** components, holes and connections lost never exceed the baseline; components or holes that are EXTRA (more than the baseline) are only reported, not a failure (Proposal of the lead 2026-10-29); 0 self-intersections; deviation from the true shape (both directions, where a truth exists) at most baseline + 5 % + 0.05 px. **Max deviation against the pixel border:** real images only, at most max(baseline x 1.05, tolerance + 1.5 px). **Nodes and wobble:** the geometric mean over the set meets the frozen target; per image at most +25 % or the absolute floor (+3 nodes per 100 mm, +2 curvature flips per 100 mm), whichever is larger. **Reported only until slice G:** axis-aligned half-pixel features (0 degree edges at k + 0.5 px, corners at 0.5x), which a binary mask cannot reach (0.5 px per axis, 0.71 px at the corner); everything else stays guarded.
10. Given any fixture, then components, holes and connections equal the baseline's or the truth's (none lost), there are 0 self-intersections (real work, not a guard: today's baseline has up to 470, e.g. the AI drawings at 0.25x, and 34 on the synthetic photograph at 1x), and an outline whose simplification would self-intersect or touch a neighbour falls back to its previous valid contour (test: the 1 px parallels fixture).
11. Given the same shape at different start positions (translated by whole pixels, the image rotated by 90 degrees), then the results agree after mapping within 1e-6 mm with equal node counts; closed outlines are compared cyclically, their start point is free.
12. Given any result, then its max deviation from the contour before simplification, measured densely both ways in pixels of the traced raster (source pixels), is at most the tolerance plus 0.01 px; the same image placed 10 mm and 100 mm wide (or stretched 2:1) gives the same outlines in pixel space. Under a stretched placement the mm bound (tolerance times the largest stretch factor) is reported, not enforced.
13. Given remembered settings from 0025 (one Smooth 0 to 100), then they migrate to the pair Smooth and Tolerance so that Logo / Line art looks the same or better (Frage 85 B: Smooth keeps its meaning as calmness, Tolerance starts at the preset's value); the 0025 ordering (criteria 23, 62 to 64) and the 4 s budget (criterion 25) still hold. The PR reports the greedy reduction against an optimised split (kurbo) per image.
13a. Given D ships its own contour, corner and fit (Frage 93), then native and wasm traces are bit-identical again: 0025 criterion 18 returns to exact equality and the technical-debt entry "Native and wasm traces differ in the last few nodes" is closed in that PR.

### Slice C: selection methods

14. Given the bar, then a **Method** control (Brightness | Remove background | Pick colours) takes the place of Threshold, and its parameters follow it; Brightness gives results identical (1e-9) to slice D's on opaque images; images with partial transparency change by design (criterion 15).
15. Given alpha, then a pixel with alpha 0 is never content in any method, also with Invert and also when its RGB is black; a partly transparent pixel's alpha counts once (fixture: a 50 % alpha black edge gives membership 0.5, not composited and weighted again); the background is not assumed white (Frage 87 covers the change of 0025 criteria 24 and 44).
16. Given Remove background on a logo on a uniform #3366CC background with a pale #FFF2A8 element, then Auto suggests the background from the border pixels within a distance of **0.02** in the colour space chosen by trial C1 (sRGB unit cube, frozen 2026-10-30; Oklab was the candidate), the background pipette replaces it with a click, Colour tolerance 0 to 100 (the method's own field, not the geometry Tolerance in px; Proposal for the name) widens it, and the result holds the logo including the pale element with area error at most **2 %** against the truth (today 56 % on `logo-pale`).
17. Given Pick colours, then a click with the pipette on the image picks a swatch (sampled from the working image), Shift+click adds one (up to 8, **Proposal**), and each swatch can be removed; on white, a pale #FFF2A8 shape and a light grey #D9D9D9 shape are each traced alone with the boundary within **0.75 px** of the truth; the near-greys #D9D9D9 and #DCD6CF (hue alone fails) are separated, and the default Colour tolerance **19** selects at least 99 % of a picked shape with +-6 per channel of noise. *(Proposal 2026-10-30: Pick colours scores each pixel as the best of its own colour, the 3 x 3 channel median and, in flat areas (every channel varies by 16 or less within 5 x 5), the 5 x 5 mean, so noise is taken out without widening the tolerance; a pixel next to a selected one is also tried as a blend between a picked colour and the colour of a neighbour that is not selected, so a grey fringe of cheap resampling does not shrink the outline (criterion 18: the picked red star of the fringe logo is 0.32 px from the truth, 1 path). Measured on the tester's fixture: +-3 noise 100 %, +-6 noise 99.2 %, picking either near-grey takes no solid pixel of the other and no rim of the star at 19; selection of 8 colours on 2000 x 2000 px 0.22 s CPU. Limitation: from tolerance 20 a picked #D9D9D9 pulls in the star's anti-aliased rim (many stray paths) and from 26 a picked #DCD6CF takes the lighter solid #D9D9D9; noise above +-8 in a flat area is not smoothed; a pixel is only tried through its neighbourhood when its colour is within 14 bytes of a mixing segment. Pick colours started at 15 before.)*
18. Given anti-aliased edges, then a picked colour includes the blend pixels toward the background so the outline lies within **0.5 px** of the true edge (not shrunk inward by one pixel).
19. Given two picked colours that touch, then the result is one compound path with no outline along their shared border (masks are united before tracing, not paths per colour).
20. Given Pick colours or Remove background on a 2000 x 2000 px image, then selection adds at most **0.5 s** thread CPU native to the trace; the 4 s budget holds. The method and its tolerance are remembered as the app setting; picked colours belong to the session of the tool (**Proposal**).

### Slice E: integration

21. Given the View control, then it gets a **Mask** view showing exactly the mask handed to the tracer (test: pixel-equal), beside Overlay, Trace and Image; the peek key works in all. The mask is the one after Specks and border shapes are applied, at the traced resolution.
22. Given the bar, then its order follows the document: (1) preset/automatic and View, (2) Method with its parameters, (3) geometry (Smooth, Tolerance; Corners in Advanced), (4) Ignore below, then the expander, then the progress pill at the far right (criterion 22a); nothing moves when a method changes except the method's own parameters.
22a. (Frage 84, customer's own answer; built with the first visible PR, i.e. slice D, and kept by E.) Given the Trace bar, then the progress pill is the finish action and sits at the far right of the bar, right of the Advanced expander; the "Trace" state label at the start of row 1 is removed (the "i" About Trace button stays). Idle, the pill reads as the trace/create button and a click creates the paths (same result as Enter, which keeps working); while running it shows the percentage and an x that aborts; after Stop it shows "Stopped" with Retry; on failure "Failed" with Retry (0025 Frage 79 B behaviour otherwise unchanged). It is always present and does not change position or width class between states more than its own content needs. The widths in `ux-notes.md` §2 are re-measured in the PR. **Customer 2026-10-30 (feedback on #223):** the button is always blue (`--editor-accent` fill, white text) and keeps its shape (75 x 28 px at rest and while running); its label is **"Trace"** (not "Create paths"); while a job runs the percentage and the x show inside the same blue button; a short job never flashes them: the progress shows only after the job has run 400 ms and then stays at least 300 ms; Stopped and Failed keep Retry; Enter still creates.
23. Given the same settings, then the preview equals the created result (same worker, same build); one history step; several selected images use the same method and swatches; a late worker answer for stale settings is dropped.
24. Given the documentation, then the About Trace text (0025 criterion 70), the user page and `docs/design-system.md` describe the methods, the Mask view and the new geometry; every string matches code (0025 criterion 35). The intro reads "Trace turns the selected image into paths. The method decides which parts become shapes. Press Enter or the Trace button to create the paths." (lead 2026-10-30, Proposal; replaces "Dark parts become shapes, light parts stay empty", true only for Brightness).

### Slice M: multicolour (adjacent)

25. Given an output control **Single colour | Multicolour** in the Advanced block (Frage 92 B, not in the main bar; the Colours count and the palette pipette sit next to it), then it is only changed by the maker (never by the analysis of F).
26. Given Multicolour, then the palette comes from an automatic count (2 to 16, default 6 **Proposal**) plus fixed colours by pipette, which are kept; colours closer than **0.03** in the colour space chosen by trial C1 (sRGB unit cube, frozen 2026-10-30) merge; a pale accent covering 0.2 % of the image is kept; on the 3-colour logo fixture with anti-aliasing and JPEG quality 75 the result has exactly 3 colours plus background (no seam colours).
27. Given the segmentation, then every pixel belongs to exactly one region or the background; regions below Ignore below join the neighbour with the longest shared border (ties by palette order), and the count of joined regions is shown.
28. Given two neighbouring regions, then their shared boundary is the same fitted curve in both paths (the same segments reversed, so nodes are equal within 1e-6 mm), and at a triple junction all three share one node, also after simplification; rasterised at 4x, uncovered and doubly covered area are each at most 0.01 % of the image.
29. Given the result, then it is one group "Trace of <name>" with one compound path per colour, filled with that colour, stroke off, largest area at the bottom, ties by colour value (stable). Recolouring a swatch changes fills without a refit (outlines equal within 1e-9). Excluding a colour makes it background; "Merge into" reassigns it to another colour (**Proposal**). The palette is usable without colour perception (hex value and name per swatch, keyboard reachable).
30. Given 8 colours on a 2000 x 2000 px image, then the trace takes under 4.0 s thread CPU native (customer limit 2026-10-14); the wasm time is reported.

### Slice F: automatic analysis

31. Given a new image, then the analysis reports properties with a confidence (colour count, background uniformity, transparency, noise, gradients, share of straight edges, component and hole sizes, stroke widths), an image type (graphic/clipart, logo, line drawing, sketch; scan or paper photo as capture condition), the settings it proposes and a short reason, shown as one line in the bar ("Line drawing · paper background evened"). Text-like regions are only reported.
32. Given a labelled test set kept separate from the tuning set, then the type is correct on at least **80 %**; below a confidence threshold the analysis falls back to Logo / Line art with the automatic Threshold and says so.
33. Given a manual change, then no later analysis changes it; analysis runs again only for a new image or "Analyse again"; it never switches Single colour to Multicolour; in Multicolour it proposes a palette and keeps picked colours. Frage 83 B: on a new image the analysis applies its settings automatically (the line in the bar says what it set and why); once the maker has changed any setting of that image it never applies again until "Analyse again", which redoes it on request. There is no "Suggested/Apply" state. Presets stay the four job presets and the analysis sets only image-dependent values (Frage 86 A).
34. Given a 2000 x 2000 px image, then analysis takes at most **0.5 s** thread CPU native (global on a reduced copy, detail checks on full-resolution crops, at most 3 candidate configurations), and is deterministic.

### All slices

35. Image data never leaves the machine; no model is shipped (0025 Frage 58 A); every number is reproducible by the harness.

## Frozen targets (2026-10-29)

Proposal of the lead, from the slice A baseline (`plan.md` "Baseline results", `baseline.json`); the measuring method in `plan.md` is confirmed; the customer confirms the numbers at D's demo. The guard for every other metric of every run is criterion 9a (replaces "within 5 % (plus 0.001) of `baseline.json`").

| Criterion | Metric | Baseline (Logo, 1x) | Frozen target | Applies at |
|---|---|---|---|---|
| 5 | Corner position error | max 4.1 px, 49 % within 0.5 px | 0.5 px | 1x, 0.5x; 0.25x reported |
| 6 | Segments per straight edge (> 20 px) | mean 6.9, max 13 | 2 | 1x, 0.5x; 0.25x reported |
| 6 | Straight edge deviation | 0.80 px | 0.35 px | 1x, 0.5x; 0.25x reported |
| 6 | Curvature flips per straight edge | 21 (`line-angles`) | 0 | all |
| 7 | Arc and bow deviation, false corners | 0.72 px arc, 1.96 px bow, 2 false corners | 0.5 px, 0 | all |
| 9 | Nodes per 100 mm, geometric mean | 19.4 | 75 % of baseline (about 14.6) | 1x |
| 10 | Self-intersections | up to 470 | 0 | all |
| 16 | Background suggestion distance (sRGB unit cube, trial C1) | none | 0.02 (frozen 2026-10-30) | 1x |
| 16 | Area error, pale element | 56 % (`logo-pale`) | 2 % | 1x |
| 17 | Pale shape boundary | pale shapes lost | 0.75 px | 1x |
| 18 | Anti-aliased edge position | 1.31 px max | 0.5 px | 1x |
| 20, 34 | Selection / analysis CPU, 2000 x 2000 px | whole set 3.0 s native | 0.5 s | native |
| 26 | Palette merge distance (sRGB unit cube, trial C1; #D9D9D9 to #DCD6CF is 0.043) | none | 0.03 (frozen 2026-10-30) | - |
| 32 | Image type accuracy | none | 80 % | held-out set |

## Conflicts with existing customer decisions

| Document asks | Decided before | Handling |
|---|---|---|
| A visible finish action | Enter creates, no button (2026-10-29) | Decided (Frage 84): the progress pill is the button, criterion 22a; Enter stays |
| Decoupled smoothing, tolerance, corners | One Smooth 0 to 100 plus Corners (0025 Frage 76 C) | Decided (Frage 85 B): Smooth and Tolerance in the bar |
| Profiles by image type and automatic settings | Four job presets (0025 Frage 64 A) | Decided (Frage 86 A): presets stay |
| Better geometry for Logo / Line art | 0025 criterion 65: Logo equals the stored candidate | Decided (Frage 87 A): Logo may change |
| Transparent never content, background not white | 0025 criteria 24 and 44: transparent counts as white | Decided (Frage 87 A): transparent is never content |
| Node budget | Node limit removed (0025 Frage 74 A, merged) | Stays removed, no internal budget (criterion 8) |
| Multicolour in scope | Colours postponed (0025 Frage 50 A) | The document lifts it; order in Frage 81 |
| "Remove background" method | "Ignore background" toggle drops border shapes | Decided (Frage 88 A): toggle renamed "Drop border shapes" |

## Out of scope

- Stacked multicolour, gradients beyond chosen steps, not cutting shared edges twice (job planning, later).
- Perspective and skew correction of paper photos (own, possibly manual, function), lighting correction and local thresholds (G trials).
- Gap closing of nearly closed lines; sharpening or Laplace filters; Canny output as a fill mask.
- Machine learning, cloud services, OCR on pixels; regional processing of mixed templates.
- Laser settings, stitches, tool paths (downstream).
- Saving own presets; a plugin or tracer trait (0025 Frage 55 A).

## Later

- **G:** sub-pixel contours (0025 Later, option D of Frage 63), lighting correction, regional processing, edge-preserving denoise compared with the median.
- **H:** centerline tracing (open single strokes for plotting and line engraving; own quality criteria: one stroke per line, junctions, no doubled lines; the `centerline` crate does not build for wasm32, `docs/technical-debt.md`); shape objects (`0088` part B, `0024` slice 3); text (`0088` part A, glyph matching). Recognition does not create duplicate areas.

## Questions for the customer

### Decided by the customer (2026-10-29)

- **Frage 80 (TRQ-ORDER): A.** Slices A, D, C, E, M, F.
- **Frage 81 (TRQ-CENTERLINE): A.** Multicolour before centerline; centerline stays in H (Later).
- **Frage 82 (TRQ-REFIMG): A**, in his words: "I can provide reference images. Tell me what you need. We make them public or use public images." His images are committed as public fixtures (own work, licence stated by him, e.g. CC0); third-party images only if public domain or CC0 (criterion 1).
- **Frage 83 (TRQ-AUTO): B.** The analysis applies its settings to a new image automatically, never after a manual change; "Analyse again" redoes it (criterion 33).
- **Frage 84 (TRQ-FINISH): own answer.** "We use the progress pill with abort x as trace button. Then remove the Trace label at the start of line one. The button will then be on the right side of the expand Advanced button." (criterion 22a).
- **Frage 85 (TRQ-SMOOTH): B.** Two bar controls, Smooth (calmness) and Tolerance (px); Corners in Advanced (criteria 8, 13).
- **Frage 86 (TRQ-PROFILES): A.** The four job presets stay; the analysis sets only image-dependent values and names the image type.
- **Frage 87 (TRQ-CHANGE): A.** Logo / Line art may change when no metric gets worse than +5 % and corners/straights get better; transparent pixels never become content, also with Invert (criteria 9, 15).
- **Frage 89 (TRQ-PICK): A.** In Pick colours and Multicolour the pipette stays armed (Shift+click adds more), Esc ends it; in Remove background it ends after the pick.
- **Frage 90 (TRQ-MULTIPICK): A.** Multicolour with Pick colours: the palette is exactly the picked colours; the Colours count is fixed and greyed out.
- **Frage 91 (TRQ-SMOOTHREADOUT): superseded by Frage 85 B.** Tolerance in px is its own bar control; there is no readout beside Smooth.
- **Frage 92 (TRQ-OUTPUTPOS): B.** Single colour / Multicolour lives in Advanced, not in the main bar (criterion 25).
- **Frage 88 (TRQ-BGNAME): A.** The toggle becomes "Drop border shapes"; "Remove background" is the method.
- **Frage 93 (TRQ-TRACER): A.** If slice D's own contour, corner and fit beat visioncortex on the frozen targets, ship ours; visioncortex stays only as the harness baseline. Decided at D's demo with the per-image table (criterion 13a).

- **Finish button (customer, 2026-10-30, feedback on #223):** always blue, keeps its shape, label "Trace"; progress inside the same button, shown only after 400 ms and then held at least 300 ms (no flash of percent and x on short jobs); Stopped and Failed keep Retry; Enter still creates (criterion 22a).

- **Frage 96 (TRACE-8CONN): A** (customer, 2026-10-29). 8-connected foreground, background stays 4-connected; thin diagonal lines stay one shape (Logo nodes 19.4 to about 13.2 per 100 mm). Amends ADR 0023 §3 (see `adrs.md` §5 note).

### Still open

None.

## UX notes

See `ux-notes.md` (bar order and widths, analysis line, View with Mask, method fields, pipette, palette, Advanced, states, keyboard, strings).

## Links

Requirements: R-VEC-001, R-VEC-002 (combining passes), R-VEC-003, R-VEC-004, R-SYS-007. PR: none.
Related: `0025-raster-trace` (spec, `adrs.md`, `research.md`), ADR 0023, `0088-trace-recognition`, `0024` slice 3, `0086-stick-fonts`.
Sources: the customer's document of 2026-10-29; visioncortex/VTracer docs; `kurbo::simplify_bezpath`; Oklab (bottosson.github.io/posts/oklab).
