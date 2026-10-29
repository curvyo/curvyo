# Trace quality and colour: intended geometry, colour selection, multicolour, automatic analysis

Status: Draft
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
- **[T]:** a target number that is a starting point and is **frozen after slice A** from the measured baseline (criterion 4); the relation stated with it is the criterion.

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

Order by Frage 80 (recommended A, D, C, E, M, F); C and D are independent; M needs C and D; F comes after the manual methods are good. A trial in D or C may end with "no advantage"; the PR then says so and the slice ships what helped.

## Acceptance criteria

### Slice A: reference set and baseline

1. Given the fixtures in `curvyo-vectorize-core/tests/fixtures/trace/`, then there are at least 3 images per primary group (graphics/clipart, logos, line drawings, sketches, AI black and white drawings, paper scan, paper photo) and at least 1 diagram and 1 photo; each has a sidecar note with origin, rights (Frage 82) and intended use (area engraving, cut contour, line plot).
2. Given the synthetic fixtures (rendered from known vectors, the truth kept beside them), then they cover: sharp and round corners; straight lines at 0, 7, 15, 30 and 45 degrees; circles; webs 1 and 2 px wide; parallels 1 px apart; hatching; stippling; open and nearly closed strokes; pale colours; transparency; grey seams; JPEG quality 50; scan noise; paper texture; a shadow; a lighting gradient; small text with Ä, Ö, Ü, ß; each at three resolutions (1x, 0.5x, 0.25x).
3. Given the harness (0025 criterion 27), then it also measures: two-sided boundary deviation (max and mean, every 0.1 px, against the vector truth or the 4x reference); corner position error per annotated corner; straightness (max distance from the true line and segment count per annotated edge); topology (components, holes, connections versus the truth, as lost and extra counts); self-intersections; paths and nodes; thread CPU time native and wasm; peak memory (counting allocator).
4. Given the baseline run (today's pipeline, each 0025 preset), then `baseline.json` holds it, the lead confirms the measuring method, and this spec gets a "Frozen targets" table replacing every [T] by a number. No product code changes in slice A.

### Slice D: intended geometry (single colour, Brightness)

5. Given the corner fixtures (squares rotated 0, 15, 30, 45 degrees; a triangle; a five-point star with 36 degree tips), then every annotated sharp corner is a node of the result within **0.5 px [T]** of the true corner; 0025 criterion 20 still holds.
6. Given an annotated straight edge longer than 20 px at any angle of criterion 2, then it is at most **2 [T]** line segments, deviates at most **0.35 px [T]** from the true line, and has 0 curvature sign flips (no staircase wobble).
7. Given round transitions (a rounded rectangle with 10 px radius) and the sketch fixture with a bowed edge (2 px sagitta over 200 px), then neither is straightened or cornered: deviation from the true arc and bow at most **0.5 px [T]**, no structural corner on them.
8. Given the parameter schema (0025 criterion 6), then input smoothing, deviation tolerance (source px) and corner protection are separate parameters; changing only the tolerance leaves every protected corner's position unchanged within 1e-6 px. There is no node-budget control (customer, 0025 Frage 74 A); an internal budget, if any, is not shown.
9. Given the set at Logo / Line art, then the geometric mean of nodes per 100 mm is at most **85 % [T]** of the baseline while each image's max deviation is no more than its baseline plus 0.05 px; no image is worse than its baseline by more than 5 % on any metric (guard of 0025 criterion 28).
10. Given any fixture, then components, holes and connections equal the baseline's or the truth's (none lost), there are 0 self-intersections, and an outline whose simplification would self-intersect or touch a neighbour falls back to its previous valid contour (test: the 1 px parallels fixture).
11. Given the same shape at different start positions (translated by whole pixels, the image rotated by 90 degrees), then the results agree after mapping within 1e-6 mm with equal node counts (closed outlines do not depend on their start point).
12. Given any result, then its max deviation from the contour before simplification, measured densely both ways in source pixels, is at most the tolerance plus 0.01 px; the same image placed 10 mm and 100 mm wide (or stretched 2:1) gives the same outlines in pixel space.
13. Given remembered settings from 0025, then they migrate as Frage 85 decides; the 0025 ordering (criteria 23, 62 to 64) and the 4 s budget (criterion 25) still hold. The PR reports the greedy reduction against an optimised split (kurbo) per image.

### Slice C: selection methods

14. Given the bar, then a **Method** control (Brightness | Remove background | Pick colours) takes the place of Threshold, and its parameters follow it; Brightness gives results identical (1e-9) to slice D's.
15. Given alpha, then a pixel with alpha 0 is never content in any method, also with Invert and also when its RGB is black; a partly transparent pixel's alpha counts once (fixture: a 50 % alpha black edge gives membership 0.5, not composited and weighted again); the background is not assumed white (Frage 87 covers the change of 0025 criteria 24 and 44).
16. Given Remove background on a logo on a uniform #3366CC background with a pale #FFF2A8 element, then Auto suggests the background from the border pixels within Oklab distance **0.02 [T]**, the background pipette replaces it with a click, Tolerance 0 to 100 widens it, and the result holds the logo including the pale element with area error at most **2 % [T]** against the truth.
17. Given Pick colours, then a click with the pipette on the image picks a swatch (sampled from the working image), Shift+click adds one (up to 8, **Proposal**), and each swatch can be removed; on white, a pale #FFF2A8 shape and a light grey #D9D9D9 shape are each traced alone with the boundary within **0.75 px [T]** of the truth; #D9D9D9 and #DCD6CF (near-greys, hue alone fails) are separated.
18. Given anti-aliased edges, then a picked colour includes the blend pixels toward the background so the outline lies within **0.5 px [T]** of the true edge (not shrunk inward by one pixel).
19. Given two picked colours that touch, then the result is one compound path with no outline along their shared border (masks are united before tracing, not paths per colour).
20. Given Pick colours or Remove background on a 2000 x 2000 px image, then selection adds at most **0.5 s [T]** thread CPU native to the trace; the 4 s budget holds. The method and its tolerance are remembered as the app setting; picked colours belong to the session of the tool (**Proposal**).

### Slice E: integration

21. Given the View control, then it gets a **Mask** view showing exactly the mask handed to the tracer (test: pixel-equal), beside Overlay, Trace and Image; the peek key works in all.
22. Given the bar, then its order follows the document: (1) preset/automatic and View, (2) Method with its parameters, (3) geometry (Smooth; Corners in Advanced), (4) Ignore below, slot, expander; nothing moves when a method changes except the method's own parameters; there is no visible finish button unless Frage 84 says otherwise.
23. Given the same settings, then the preview equals the created result (same worker, same build); one history step; several selected images use the same method and swatches; a late worker answer for stale settings is dropped.
24. Given the documentation, then the About Trace text (0025 criterion 70), the user page and `docs/design-system.md` describe the methods, the Mask view and the new geometry; every string matches code (0025 criterion 35).

### Slice M: multicolour (adjacent)

25. Given an output control **Single colour | Multicolour**, then it is only changed by the maker (never by the analysis of F).
26. Given Multicolour, then the palette comes from an automatic count (2 to 16, default 6 **Proposal**) plus fixed colours by pipette, which are kept; colours closer than **0.03 [T]** in Oklab merge; a pale accent covering 0.2 % of the image is kept; on the 3-colour logo fixture with anti-aliasing and JPEG quality 75 the result has exactly 3 colours plus background (no seam colours).
27. Given the segmentation, then every pixel belongs to exactly one region or the background; regions below Ignore below join the neighbour with the longest shared border (ties by palette order), and the count of joined regions is shown.
28. Given two neighbouring regions, then their shared boundary is the same fitted curve in both paths (nodes equal within 1e-6 mm, reversed), and at a triple junction all three share one node, also after simplification; rasterised at 4x, uncovered and doubly covered area are each at most 0.01 % of the image.
29. Given the result, then it is one group "Trace of <name>" with one compound path per colour, filled with that colour, stroke off, largest area at the bottom, ties by colour value (stable). Recolouring a swatch changes fills without a refit (outlines equal within 1e-9). Excluding a colour makes it background; "Merge into" reassigns it to another colour (**Proposal**). The palette is usable without colour perception (hex value and name per swatch, keyboard reachable).
30. Given 8 colours on a 2000 x 2000 px image, then the trace takes under 4.0 s thread CPU native (customer limit 2026-10-14); the wasm time is reported.

### Slice F: automatic analysis

31. Given a new image, then the analysis reports properties with a confidence (colour count, background uniformity, transparency, noise, gradients, share of straight edges, component and hole sizes, stroke widths), an image type (graphic/clipart, logo, line drawing, sketch; scan or paper photo as capture condition), the settings it proposes and a short reason, shown as one line in the bar ("Line drawing · paper background evened"). Text-like regions are only reported.
32. Given a labelled test set kept separate from the tuning set, then the type is correct on at least **80 % [T]**; below a confidence threshold the analysis falls back to Logo / Line art with the automatic Threshold and says so.
33. Given a manual change, then no later analysis changes it; analysis runs again only for a new image or "Analyse again"; it never switches Single colour to Multicolour; in Multicolour it proposes a palette and keeps picked colours. Whether it applies settings by itself is Frage 83.
34. Given a 2000 x 2000 px image, then analysis takes at most **0.5 s [T]** thread CPU native (global on a reduced copy, detail checks on full-resolution crops, at most 3 candidate configurations), and is deterministic.

### All slices

35. Image data never leaves the machine; no model is shipped (0025 Frage 58 A); every number is reproducible by the harness.

## Conflicts with existing customer decisions

| Document asks | Decided before | Handling |
|---|---|---|
| A visible finish action | Enter creates, no button (2026-10-29) | Frage 84, default: keep Enter |
| Decoupled smoothing, tolerance, corners | One Smooth 0 to 100 plus Corners (0025 Frage 76 C) | Frage 85 |
| Profiles by image type and automatic settings | Four job presets (0025 Frage 64 A) | Frage 86 |
| Better geometry for Logo / Line art | 0025 criterion 65: Logo equals the stored candidate | Frage 87 |
| Transparent never content, background not white | 0025 criteria 24 and 44: transparent counts as white | Frage 87 |
| Node budget | Node limit removed (0025 Frage 74 A) | Kept removed; internal only (criterion 8) |
| Multicolour in scope | Colours postponed (0025 Frage 50 A) | The document lifts it; order in Frage 81 |
| "Remove background" method | "Ignore background" toggle drops border shapes | Frage 88 |

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

Frage 80 (TRQ-ORDER): Order of the slices
A: A, D, C, E, M, F (geometry first: every existing trace improves without new UI).
B: A, C, D, E, M, F (colour selection first).
C: A, M first (multicolour as the headline).
Recommendation: A. Default: A.

Frage 81 (TRQ-CENTERLINE): Centerline against multicolour
A: As the document: M, then F; centerline in H (later).
B: Centerline right after E, before M (line drawings, sketches and AI drawings are primary material; plotters and line engraving need single strokes).
C: Centerline after M, before F.
Recommendation: B. Default: A (the document's order).

Frage 82 (TRQ-REFIMG): Your reference images and their rights
A: You send 10 to 20 of your own images (own drawings, own AI generations, scans, photos); they are committed as fixtures with a note "own work, may be used for tests".
B: You send them, they stay local on the build host; only the numbers are committed.
C: Only synthetic and public-domain/CC0 images.
Recommendation: A for your own works, B for anything from third parties. Default: C plus B for whatever arrives.

Frage 83 (TRQ-AUTO): May the analysis change settings by itself?
A: It only proposes; one click applies.
B: It applies its settings to a new image automatically, never after a manual change; "Analyse again" redoes it.
C: No analysis.
Recommendation: B. Default: A.

Frage 84 (TRQ-FINISH): Visible finish action
A: Keep Enter only (your decision of 2026-10-29).
B: A visible "Create paths" button again.
C: Enter only, plus the operation in a menu.
Recommendation: A. Default: A.

Frage 85 (TRQ-SMOOTH): Smooth after the decoupling
A: Keep one Smooth in the bar, mapped to tolerance and smoothing together; Corners stays in Advanced; old values map so Logo looks the same or better.
B: Two bar controls, Smooth (calmness) and Tolerance (px).
C: Replace Smooth by a new control; remembered values reset to the preset.
Recommendation: A. Default: A.

Frage 86 (TRQ-PROFILES): Presets and image types
A: Keep the four job presets; the analysis sets only image-dependent values (method, threshold, denoise) and names the image type in its line.
B: Replace presets by image-type profiles plus a separate job choice.
C: Two menus, image type and job.
Recommendation: A. Default: A.

Frage 87 (TRQ-CHANGE): Accepted results may change
A: Logo / Line art may change when no metric gets worse than +5 % and corners/straights get better; transparent pixels never become content, also with Invert.
B: Logo stays exactly as today; the new geometry goes into the other presets only.
C: The new geometry only behind an Advanced switch.
Recommendation: A. Default: A (each slice is a story PR you accept at the demo).

Frage 88 (TRQ-BGNAME): Two "background" controls
A: The toggle becomes "Drop border shapes"; "Remove background" is the method.
B: The toggle is dropped; the method replaces it.
C: Keep both names.
Recommendation: A. Default: A.

## UX notes

(filled in by ux-engineer before Ready: bar states standard, colour picking, multicolour with palette, automatic, custom, Advanced, computing, failure, low confidence; narrow windows, keyboard, pipette.)

## Links

Requirements: R-VEC-001, R-VEC-002 (combining passes), R-VEC-003, R-VEC-004, R-SYS-007. PR: none.
Related: `0025-raster-trace` (spec, `adrs.md`, `research.md`), ADR 0023, `0088-trace-recognition`, `0024` slice 3, `0086-stick-fonts`.
Sources: the customer's document of 2026-10-29; visioncortex/VTracer docs; `kurbo::simplify_bezpath`; Oklab (bottosson.github.io/posts/oklab).
