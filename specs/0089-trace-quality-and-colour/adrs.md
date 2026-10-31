# ADRs for Trace quality and colour

Architect, 2026-10-29. Built on the defaults of Frage 80 to 88 (order A, D, C, E, M, F; Frage 82
C plus B; Frage 85 A; Frage 86 A; Frage 87 A; Frage 88 A). Slices A, D and C are decided below;
E, M and F are outlines that get their own dated note before their build.

**No new ADR, no new crate, no new external dependency, no trait, no generic, no `format_version`
change.** Everything stays inside `curvyo-vectorize-core` and the `trace_job` path of ADR 0023. The
changes are recorded as a dated amendment of ADR 0023 (below), the way `0025` recorded slices 2
and 3. One point reverses a customer choice in effect (visioncortex leaves the shipping path if D
wins) and goes to the customer as a question with a safe default.

## Depends on

- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md): this feature amends §3
  (pipeline), §4 (determinism, now reachable bit-identical) and executes §8 (own pipeline) behind the
  unchanged §2 call shape; §5 (worker) gains a stage cache; §1 edges unchanged.
- `specs/0025-raster-trace/adrs.md`: decisions 3 to 6 and both 2026-10-28 notes (presets as data,
  stored `{ preset, edited, values }`, `reduce.rs`, determinism rules for our own code, the
  harness per preset, `area_error_true_pct`).
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §4, §6: RGBA8 straight alpha,
  pixel-corner space, `pixel_map`. Unchanged.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §5, §9 and the compound
  path: multicolour output is one group of filled compound paths, existing kinds. No format change.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): no new crate; `ui-core →
  vectorize-core` stays the only edge.
- [ADR 0005](../../docs/adr/0005-extension-and-plugin-model.md) §4: no tracer seam (0025 Frage 55 A).

## Amendment of ADR 0023 (2026-10-29, Proposed; the lead adds "amended by `specs/0089` adrs.md" to its Status line)

### 1. Module responsibilities (one crate, no engine abstraction)

Each module does one thing; the stages are plain functions over owned data, called in a fixed order
by `trace.rs`. No `Stage` trait, no registry.

| Module | One sentence | Slice |
|---|---|---|
| `raster.rs` | Reads the straight-alpha RGBA8 input and validates its size. | kept |
| `colour.rs` (new) | Converts stored sRGB bytes to linear light and Oklab with basic operations. | C |
| `select.rs` (new) | Turns a raster and a selection method into a score field and an alpha field. | C (D keeps today's luma) |
| `filter.rs` | Applies Denoise (median) and Upscale (2x bilinear) to a byte field. | kept |
| `regions.rs` | Labels a mask or label map into 4-connected regions and drops specks and border shapes. | kept |
| `contour.rs` (new) | Walks the crack edges between differently labelled pixels into boundary chains. | D |
| `corners.rs` (new) | Finds structural corners on a boundary chain before any smoothing. | D |
| `fit.rs` (new) | Fits lines and cubics between corners within the deviation tolerance. | D |
| `guard.rs` (new) | Rejects a fitted outline that self-intersects or crosses a neighbour and falls back. | D |
| `params.rs` | Holds the schema, the presets and the mapping of bar values to resolved settings. | kept, grows |
| `trace.rs` | Runs the stages in order, keeps their cache and maps the result to mm. | kept |
| `palette.rs`, `segment.rs` (new) | Builds a palette; assigns every pixel to one palette entry or background. | M |
| `analysis.rs` (new) | Measures image properties and proposes image-dependent settings. | F |

`outline.rs` (visioncortex `Spline`) and `reduce.rs` stay until D's harness shows the new path
wins; then both are deleted (reduce's greedy merge survives only if the measurement in §5 shows it
adds something after `fit.rs`). Modules over ~500 lines (`params.rs` and `reduce.rs` today, tests
included) are split by the implementer as part of D, not justified.

### 2. Data interfaces (public, minimal, no visioncortex type)

The free function keeps its shape; its settings type grows; the result gains diagnostics.

```rust
pub fn trace(raster: Raster<'_>, pixel_map: Affine, params: &TraceParams,
             poll: &mut dyn FnMut(Progress) -> ControlFlow<()>) -> Result<TraceResult, TraceError>;
pub struct Tracer { /* owned stage cache, §6 */ }   // Tracer::trace(&mut self, same arguments)
pub fn auto_threshold(raster: Raster<'_>) -> u8;      // C: opaque-weighted, §3
pub fn suggest_background(raster: Raster<'_>) -> Rgb8; // C: Auto of Remove background
pub fn mask(raster: Raster<'_>, pixel_map: Affine, params: &TraceParams, poll) -> Result<Mask, TraceError>; // E
```

- **Resolved settings = `TraceParams`.** Everything explicit, no "Auto", no preset key: `selection:
  Selection`, `denoise`, `upscale`, `specks`, `drop_border_shapes` (stored key stays
  `ignore_background`, Frage 88 A renames only the label), `geometry: Geometry`. Presets, Smooth and
  Corners resolve to it in `params.rs` (§7); the host stores bar values, never `Geometry`.
- `Selection` = `Brightness { threshold: u8, invert: bool }` | `RemoveBackground { background:
  Rgb8, tolerance: u8 }` | `PickColours { swatches: Vec<Rgb8>, tolerance: u8 }` (1 to 8).
  `TraceParams` becomes `Clone`, not `Copy`. `Rgb8 { r, g, b }` is ours.
- `Geometry { smoothing_px: f64, tolerance_px: f64, corner_cos: f64 }` (`pub(crate)` fields,
  built only by `params.rs`, so no caller can hand in an unmapped value).
- **Selection field** stays internal: a `u8` score plus the alpha byte per pixel. `Mask { width,
  height, bits: Vec<u8> /* 0 or 255 */, pixel_map: Affine }` is the one public view of it (Mask
  view, criterion 21), produced by the same code path the trace uses.
- **Result diagnostics** (`TraceResult.diagnostics`), each field has a reader: `fallbacks: usize`
  (outlines kept at their previous valid contour, status line and criterion 10), `max_deviation_px:
  f64` (criterion 12, harness and Layers tooltip), `joined_regions: usize` (M, criterion 27).
  `Counts` unchanged.
- The analysis report (F) and the multicolour result (M) are outlined in §9 and §10.

### 3. Colour and alpha semantics (C)

- **Alpha is counted once, never composited.** A pixel's content decision uses its straight RGB for
  the method's score and its alpha separately: `membership = α/255 · s`, `s ∈ [0, 1]` from the
  method; the mask is `membership ≥ 0.5`. A 50 % alpha pixel of a fully selected colour has
  membership 0.5 (criterion 15). α = 0 gives 0 whatever the RGB, the method or Invert: **Invert
  acts on `s`, never on α**. There is no "over white" anywhere; ADR 0023 §3 step 1 and `0025`
  criteria 24 and 44 change (Frage 87 A).
- **Background is not white:** `auto_threshold` builds Otsu's histogram from pixels with α ≥ 128
  only; `suggest_background` takes the most frequent quantised Oklab colour of the opaque border
  pixels (ties by value), so a #3366CC frame is found as such.
- **Score field:** Brightness: the integer Rec. 709 luma of the stored RGB (today's formula without
  compositing), cut at the Threshold. Remove background: distance to the background colour, cut at
  the tolerance. Pick colours: per swatch, the **two-colour mixing estimate** (see trial C1), the
  union over swatches is the per-pixel maximum, so touching swatches give one region (criterion 19:
  masks united before tracing, not paths).
- Denoise and Upscale run on the score field and the alpha field before the cut, as today on the
  luma. Median commutes with a monotone cut, and Brightness's score is the luma, so **opaque
  Brightness is bit-identical to D** (criterion 14 holds as "on opaque images"; images with partial
  alpha change by design, criterion 15).

**Trial C1, colour space and mixing model (Oklab is the candidate, the measurement decides).**
Candidates: (a) sRGB Euclidean (baseline), (b) linear-RGB Euclidean, (c) Oklab Euclidean, (d)
Oklab or linear-RGB distance to the *segment* swatch → background (mixing fraction from the
projection, residual as the gate), which keeps anti-aliasing blends at their coverage. Measured on
the slice A colour fixtures: boundary deviation on the AA edges (criterion 18, 0.5 px), area error
(criterion 16), separation margin of #D9D9D9 against #DCD6CF and #FFF2A8 against white at one
tolerance (smallest in-class distance gap over the tolerance spread), and thread CPU for 4 Mpx
native and wasm (criterion 20, 0.5 s). Winner: the simplest candidate that passes 16 to 18 and 20;
Oklab only if (a) and (b) fail the near-grey or pale case. CIELAB is not tried (same cube root, no
gain over Oklab for this). Hue-only selection is rejected up front (fails near-greys).

### 4. Coordinate spaces and tolerances

- **All geometry is fitted in pixel space** of the raster handed in (pixel corners, y down), after
  Upscale in its 2x space with tolerances doubled there, so every tolerance is in **source pixels**.
  Cubic Béziers are affine-invariant: mapping the fitted outline by `pixel_map` is exact, so the
  same image placed 10 mm, 100 mm or stretched 2:1 gives the same pixel outlines (criterion 12).
- **Deviation tolerance is pixels, not mm.** The 2026-10-28 reduction tolerance `(Smooth − 80) ×
  0.01 mm` becomes a pixel value (§7). Under an anisotropic map the mm deviation is at most
  `tolerance_px × σ_max(pixel_map)`; that bound is reported, not enforced. A manufacturing
  tolerance in mm is a different meaning (job side, later) and is not mixed in.
- **Specks stay mm²** through `|det|`; they are a physical size decision, not a fit tolerance.
- The reduced preview copy is traced in its own pixels with the same settings (as today); the
  created paths come from the full-size trace in the same worker (ADR 0023 §5).
- **Measured, not sampled:** `fit.rs` accepts a piece only after a dense two-way check (every 0.1
  px of arc on the candidate to the reference polyline, every reference vertex to the candidate,
  nearest point by Newton projection in basic operations). The reference is the chain after input
  smoothing ("the contour before simplification", criterion 12).

### 5. Geometry pipeline (D): contour, corners before the fit, line fit

Order per boundary chain: crack-edge chain → structural corners on the unsmoothed chain → input
smoothing per span with the corner points fixed → line or curve fit per span → guard.

- **Contour (`contour.rs`).** Our own walk of the crack edges between pixels of different label,
  on our `u32` labels (replaces visioncortex's per-crop walk, its `u16` limit and so `TooDetailed`).
  Saddles (2x2 checkerboard) resolve so foreground stays 4-connected, matching `regions.rs`. Vertices
  are integer pixel corners. A chain is a closed loop in single colour; in M it runs between
  junctions (§10), so D builds the edge table M needs.
- **Corners, trial D1 (`corners.rs`).** Candidates: (a) visioncortex's corner angle (baseline, as
  shipped); (b) multi-scale turn angle on the chain: the angle between the chords to the points at
  arc distance ±s for s = 2, 4, 8 px, a corner is a local maximum sharper than the Corners angle at
  two of three scales; (c) dominant points of a polygon at 0.75 px and the angle between adjacent
  long sides. Corner position: the intersection of the two adjacent total-least-squares lines when
  both sides are straight within tolerance, else the vertex of maximum turn. Measured: position
  error per annotated corner (criterion 5), missed and false corners (false = any on the rounded
  rectangle and the bowed edge, criterion 7), equality under whole-pixel shift and 90° rotation
  (criterion 11), CPU. Expected winner (b); it is the only one stable across scales by construction.
- **Line and curve fit, trial D2 (`fit.rs`).** Per span: a total-least-squares line first (accepted
  if the dense check passes); else (a) recursive split at the point of largest deviation with a
  tangent-constrained cubic per piece (Schneider), G1 at smooth joints, free tangents at corners;
  (b) the same pieces chosen by dynamic programming (fewest pieces under the tolerance, potrace's
  optimal polygon idea) then fitted; (c) `kurbo::simplify_bezpath` **in the harness only** (it uses
  `cbrt`/`acos` in its solvers, so it cannot ship under §8). Plus today's greedy `reduce` as the
  reference. Measured: segments per annotated straight edge and their deviation from the true line,
  curvature sign flips (criterion 6), nodes per 100 mm (criterion 9), max deviation (criterion 12),
  CPU. The PR reports greedy against (b) and (c) per image (criterion 13).
- **Start independence.** Spans between corners are fitted independently, so a chain with corners
  does not depend on its start. A corner-free loop starts at the vertex of largest turn at the
  coarsest scale, ties by the smallest arc index from that vertex set's canonical order; criterion
  11 compares closed outlines cyclically (symmetric ties give congruent results).
- **Guard (`guard.rs`).** After fitting, flattened outlines (0.05 px) are tested for self-intersection
  and for crossing or touching any outline whose box overlaps (sorted sweep, segment tests in basic
  operations). A failing span is refitted once at half the tolerance, then falls back to its
  smoothed polyline (the "previous valid contour", criterion 10); `diagnostics.fallbacks` counts it.
- **Input smoothing.** A fixed binomial kernel along the chain of width from `smoothing_px`, with
  the span's end points (corners, junctions) pinned; 0 means the crack chain's edge midpoints.
- Not built: Sobel/Scharr, Canny, Hough (no consumer in A to C; G's sub-pixel trial may use a
  gradient operator). Gap closing and ideal-shape reconstruction stay out of scope.
- **If D's trials show no advantage**, the shipping path stays visioncortex + `reduce`, the PR says
  so per image (spec, "a trial may end with no advantage"), and only the parameter decoupling of §7
  ships on that path.

### 6. Staged cache in the worker, cancel and late answers

`Tracer` (in `trace.rs`) owns the last image's intermediate results, each with the settings subset
it was built from; a stage reruns only when its key or an earlier stage changed. It is still pure:
no clock, no I/O, the caller owns it. `trace()` is `Tracer::default().trace(..)`.

| Stage | Holds | Key |
|---|---|---|
| S0 pixels | RGBA copy, `width`, `height` | the image's content hash + reduced flag (host-supplied `u64`, compared with the bytes' length) |
| S1 selection | score + alpha after Denoise/Upscale | S0, `selection`, `denoise`, `upscale` |
| S2 regions | cleaned mask/labels | S1, `specks`, `drop_border_shapes`, `|det|` of the map |
| S3 chains | crack chains + corners | S2, `corner_cos` |
| S4 outlines | fitted pixel outlines | S3, `smoothing_px`, `tolerance_px` |

The mm mapping runs on every call (cheap). Multicolour palette edits (§10) invalidate from S2;
recolouring invalidates nothing. `editor-wasm` keeps one `Tracer` per worker instance; the page sends
the pixels only when the S0 key changes. A settings change while a job runs **no longer terminates
the worker** (that would drop the cache): the page keeps at most one queued job (the latest), every
answer carries the settings generation, and an answer for an older generation is dropped (criterion
23). `terminate()` stays for Stop and unmount only; the cache is then rebuilt from S0.

### 7. Decoupled Smooth, presets and migration (Frage 85 A)

- Internally four concerns: input smoothing (`smoothing_px`), deviation tolerance (`tolerance_px`),
  corner protection (`corner_cos`), node budget. **No node budget, visible or internal**, is built
  (none needed by any criterion; YAGNI).
- The bar keeps **one Smooth 0 to 100**; `params.rs` maps it monotonically to both `smoothing_px` and
  `tolerance_px` (constants frozen from D's harness so Logo / Line art meets criterion 9; Proposal
  shape: tolerance 0.25 px at 0 to about 1.5 px at 100, smoothing 0 up to Smooth 30, then rising).
  **Corners** (Advanced) maps only to `corner_cos` (today's polynomial). Changing Smooth never moves a
  protected corner (corners are found before smoothing and pinned, criterion 8).
- **Migration: none of the stored numbers.** A stored Smooth keeps its number and is read through the new
  mapping; the four presets keep their keys and numbers; `readChoice` is unchanged. Acceptance is
  measured (criteria 9, 13), not a value rewrite. Missing `method` in a stored setting reads as
  Brightness; method and tolerance are added fields of the app setting; swatches are not stored
  (session of the tool, criterion 20).
  *Note 2026-10-30: the spec and the code do migrate stored settings (a stored setting is read into the
  new fields, the added `method` and `tolerance` get their defaults); this ADR meant that no stored
  number is rewritten. The spec and the code are authoritative.*
- **Node limit:** still in `params.rs`, `trace.rs` and `TraceAdvanced.tsx` on main, while the spec
  says it was removed (0025 Frage 74 A). If Frage 74 A stands, D deletes it (both bisection stages,
  `NodeLimitOutcome`, the field) because it is tied to visioncortex's segment length.

### 8. Determinism (ADR 0023 §4) and memory

- New code uses only `+ − × ÷` and `sqrt`; no `cbrt`, `powf`, `acos`, `atan2`, `hypot`, `sin`/`cos`,
  no `mul_add`, no hash maps, no threads, fixed summation order. Oklab's cube root is Newton on a
  bit-pattern initial guess; sRGB decoding is a committed 256-entry table checked against `powf` by
  a unit test.
- **Consequence:** once visioncortex's `Spline` leaves the shipping path, native and wasm become
  bit-identical again; the `npm test` wasm check returns to exact equality and the technical-debt
  entry "Native and wasm traces differ in the last few nodes" is closed in D's PR.
- **Memory:** steady state per pixel in the worker: RGBA 4 B, score + alpha 2 B, mask 1 B, labels
  4 B, plus chains proportional to boundary length; transient: median histogram row, union-find
  4 B. About 11 B/px, under ADR 0023's 18 B/px worst case. Measured by the counting allocator
  (0025 criterion 26); above 16 B/px it is recorded in `docs/technical-debt.md`, no self-made limit.

### 9. Automatic analysis (F, outline)

`pub fn analyse(raster, poll) -> Result<AnalysisReport, TraceError>` in `analysis.rs`, run in the
worker on S0. `AnalysisReport { properties: Vec<Property { kind, value, confidence }>, image_type:
ImageType, capture: Option<Capture>, confidence: f64, proposal: Proposal, reasons: Vec<Reason> }`.
`Proposal` holds only image-dependent values (selection, Threshold, Denoise; Frage 86 A), never
the preset, Single/Multicolour or Smooth. `Reason` is an enum; the sentence lives in the frontend
strings. Text-like regions: a count only, until `0088` needs boxes. Global measures on a reduced
copy, detail checks on full-resolution crops, at most 3 candidate configurations traced through the
same stages, scored with the truth-free subset of the harness metrics. Applying is the host's choice
(Frage 83, default A: propose only). Gets its own dated note before its build.

### 10. Multicolour result contract and shared boundaries (M, outline)

- **Internal:** palette (`palette.rs`, deterministic median cut in the trial-C1 colour space, merge
  closer than the frozen distance, no colour pruned for small area) → label map (`segment.rs`, one
  `u32` per pixel, 0 = background, nearest palette entry; seams then fall to Ignore below, joined to
  the neighbour with the longest shared border, ties by palette order) → `contour.rs` edge table:
  every maximal crack chain with one (left, right) label pair, ending at **junction vertices** (pixel
  corners where three or four labels meet, or a saddle) → each chain is corner-detected and **fitted
  once** with its junctions pinned → each region's outlines are assembled from edge ids with their
  orientation. Neighbours therefore share the same `BezPath` segments reversed, and triple junctions
  are one node by construction, also after simplification (criterion 28). The guard runs on the edge
  table; a fallback replaces one edge in both neighbours.
- **Public:** `trace_colours(raster, map, &ColourParams, poll) -> Result<ColourTrace, TraceError>`,
  `ColourTrace { layers: Vec<ColourLayer>, counts, diagnostics }`, `ColourLayer { sample: Rgb8,
  fixed: bool, outlines: Vec<BezPath> }`, layers ordered by pixel area descending, ties by `sample`
  (integer areas, so stable). The label map never leaves the crate. `ui-core` maps a layer to one
  compound path with fill = `sample`, stroke off, in one group. Recolour changes the fill only (no
  refit); Exclude and Merge into change label assignment (S2 onward).
- Trial M1: visioncortex's colour clustering against our palette + label map. visioncortex's
  colour path uses hash maps, so it can only win if it is run-to-run deterministic; expected
  outcome: ours. Stacked mode later needs no contract change.

### 11. Reference-set harness (A)

- Extend `tests/common/quality.rs` (split into `fixtures`, `metrics`, `report` modules, each under
  500 lines); no new crate, no new tool. Synthetic fixtures stay generated in code from vectors, the
  vector truth and the 4x truth kept beside them; annotations (corners, straight edges, topology,
  intended use) are data next to the generator. Real fixtures (CC0 or public domain only) are
  committed under `tests/fixtures/trace/reference/` with a JSON sidecar (origin, licence, use);
  JPEG and PNG fixtures decode through `curvyo-image-core` as a dev-dependency (ADR 0023 §1 already
  names it).
- **Rights (Frage 82 default C + B):** synthetic and CC0 only in the repository; the customer's own
  images are read from a local directory named by `CURVYO_TRACE_LOCAL` by the same `#[ignore]` test
  and only their numbers are committed (`baseline.json`, keys `local.<id>`, no pixels, no file
  names beyond the id).
- New metrics as in criterion 3: two-sided deviation every 0.1 px, corner error, straightness and
  segment count, topology lost/extra, self-intersections, thread CPU native (wasm through the existing
  0025 timing path), peak memory via `cap`. `baseline.json` gains the slice A columns per preset; the
  +5 % regression guard covers them.

### Dated amendment note (2026-10-29, Proposal of the lead; customer confirms at D's demo, Frage 96)

§5 "Saddles ... foreground stays 4-connected, matching `regions.rs`" and ADR 0023 §3 (4-connected,
chosen to match visioncortex, now removed) are amended: foreground is 8-connected, background stays
4-connected (Specks, regions and the contour walk agree). Reason: thin diagonal lines stay one shape
(Logo about 13.2 nodes per 100 mm instead of 19.4). The baseline is re-measured with an 8-connected
Specks reference before comparing. The guard is redefined in spec criterion 9a; axis-aligned
half-pixel features are reported only until slice G.

### Dated implementation note, slice C (2026-10-30, implementer; for the architect's review)

- **Trial C1 result: sRGB.** The two-colour mixing model in the stored sRGB bytes (Euclidean,
  bytes / 255) passes criteria 16 to 18 and 20 with the widest margins; point distances (sRGB,
  linear, Oklab) fail the near-grey pair (#DCD6CF blends within the gate of #D9D9D9's blends);
  mixing in linear RGB and Oklab pass with margins 4.8 and 4.3 against 10.2 at 6 to 14 times
  the cost (renderers blend in sRGB, so blends lie on the sRGB segment). `colour.rs` is
  therefore not built (no table, no cube root); `select.rs` alone does the selection. Measured
  table: `plan.md` "Slice C results".
- **`TraceParams` stays `Copy`:** `Selection::PickColours` holds `Swatches`, a fixed array of
  8 with a length, instead of a `Vec`; `threshold` and `invert` stay fields of `TraceParams`
  (the Threshold is read by Brightness only; Invert flips the score of every method, which the
  UX notes ask for in Remove background). Every existing caller and test kept compiling.
- **Automatic Threshold:** §3 says Otsu over the pixels with α ≥ 128 only. Measured on the
  customer's transparent line drawings this fails: with only ink opaque, Otsu splits the ink
  itself (`customer-turtle-shell`: Threshold 51, 2 % of the ink content, 15 → 559 nodes per
  100 mm). Built instead: a pixel with α < 128 counts in the top level (the class no Threshold
  makes content, whatever its stored colour), the others by the luma of their stored RGB;
  nothing is composited. Opaque pictures are unchanged.
- **Public additions:** `selected(raster, params) -> Vec<bool>` (the mask before Denoise and
  Upscale; read by the harness, the seed of slice E's `mask`), `suggest_background`,
  `sample_colour`, `Rgb8`, `Swatches`, `Selection`, `METHOD_PARAMS`, the two default colour
  tolerances. Colour tolerance t maps to the gate `0.5 · (t/100)²` in the sRGB unit cube.
- Pick colours mixes toward the picture's suggested background (the border colour), computed
  inside the trace from the same raster; a transparent border mixes nothing (the alpha carries
  the edges).

### Dated implementation note, slice E (2026-10-30, implementer; for the architect's review)

- **`mask()` as in §2**, made by the same private function the trace walks (`masked` in
  `trace.rs`), so the Mask view cannot drift from the tracer; `Mask { width, height, bits,
  pixel_map }` with bits 0 or 255. The worker runs it as its own job (`trace_mask_job`) only
  while the Mask view shows, before each preview step on the same pixels and settings; the
  selection and the regions are therefore computed twice in that view.
- **§6 stage cache and the non-terminating worker are not built.** No slice E criterion needs
  them: criterion 23's "a late worker answer for stale settings is dropped" holds through the
  page's generation check and `terminate()` on a settings change, as before. A synchronous wasm
  call cannot be interrupted, so without the cache a queued job would wait for a stale one to
  finish; the two belong together and are left for a later slice (Proposal; the lead decides).
- The mask is drawn as a picture of the editor's image pipeline under a fresh name per mask
  (no document asset); `FrameImages` carries a "pixelated" flag so a mask pixel is drawn
  nearest-neighbour from one screen pixel per mask pixel (pictures keep the 8 px rule).

### Dated implementation note, slice M (2026-10-30, implementer; for the architect's review)

- **§10 as built.** `palette.rs` (flat-pixel median cut in the sRGB unit cube, the C1 space;
  merge below 0.03; fixed colours kept exactly), `segment.rs` (label map, joins), `edges.rs`
  (edge table and colour loops), `colour_map.rs` (mask to label map), `shared_fit.rs` (one fit
  per edge), `colour_trace.rs` (public). Public additions: `trace_colours`, `colour_mask`,
  `ColourParams { trace, colours, fixed, excluded, merged }`, `ColourTrace { palette, layers,
  counts, foreground_pixels, diagnostics }`, `ColourLayer { sample, fixed, pixels, outlines }`,
  `PaletteEntry`, `ColourMask` (RGBA, for the Mask view), `Diagnostics::joined_regions`,
  `MIN_COLOURS`, `MAX_COLOURS`, `DEFAULT_COLOURS`. The label map stays inside the crate.
  `colour_mask` returns the segmented palette as RGBA (the lead's brief), not a binary union.
- **Trial M1:** ours in sRGB passes criterion 26 on every fixture (incl. AA + JPEG 75 and the
  near-greys); Oklab not built; visioncortex was not re-added (removed in D, hash maps).
- **Shared fit:** each colour loop is a `Chain` with its junction vertices pinned as corners at
  the vertex (`Chain::pinned`); the edge is fitted in the loop of its larger label with the slice
  D contour and fit (`Bound::Free`, no clearance room: neighbours share the edge, so the room of
  §5 would be 0) and reversed by the other loop. The guard checks crossings of the fitted spans
  only (shared junctions may touch, `guard::crossings`); a crossing span goes to the fit at half
  smoothing and tolerance, then to the walked midpoint contour, which cannot cross another edge.
- **Connectivity:** regions and the edge walk agree: same label 4-connected, diagonal where the
  other two pixels differ from it; a 2-label checkerboard connects the colour over the background
  and the later palette entry over the earlier, and the edge passes around the other's pixels.
- **ui-core/wasm:** the colour job appends the palette, the layers and the joined count after the
  outlines (`read_trace_wire` reads both forms); `TraceTarget::colours` gives one path per colour
  in `plan_trace` (existing node kinds, no format change); recolours and Isolate are session state
  (fills only, no job).
- **No new crate, dependency, trait or format change.** Memory: the label map adds 4 B/px and the
  edge vertices are proportional to boundary length.

## Feature-local decisions

1. No new ADR: the swap of the boundary fit is ADR 0023 §8's own path with the §2 shape kept, and
   colour selection is a pre-processing step of the same function. (Proposal)
2. No new external dependency (Oklab, median cut and the fit are ours, about 1 000 lines); any later
   candidate is checked for licence, wasm32, determinism and size; OpenCV is reference only.
3. OCR and shape recognition (`0088`) derive their own copies from the original raster, not from the
   mask or the label map.

## Spec criteria that must change (for the product owner)

- **14:** "identical (1e-9) on opaque images"; partly transparent images change per 15.
- **8:** the Node limit exists on main; confirm Frage 74 A and add "the Node limit is removed".
- **11:** closed outlines compare cyclically, start point free.
- **12:** tolerance in pixels of the traced raster; the mm bound under stretch is reported.
- **16, 26:** "Oklab distance" becomes "distance in the colour space chosen by trial C1".
- **21:** the Mask view shows the mask after Specks and border shapes, at the traced resolution.
- **28:** "within 1e-6 mm" holds exactly (shared segments); keep the wording.
- `0025` crit. 18: back to exact native/wasm equality when D ships the own fit.

## Questions for the customer (for the lead to number)

Frage (TRQ-TRACER): visioncortex in the shipping path
A: If D's own contour, corner and fit beat visioncortex on the frozen targets, ship ours;
visioncortex stays only as the harness baseline (dev-dependency).
B: Keep visioncortex shipping; ours only where it is better (two paths).
C: Keep visioncortex; tune around it only.
Recommendation: A (one path, native = wasm, no `u16` limit, M needs the edge table anyway).
Default: A, shown at D's demo with the per-image table.

> **Architect note, slice C (2026-10-30).** §3 "Otsu over α ≥ 128 only" is replaced: a pixel with α < 128 counts in the top luma level (the class no Threshold makes content, also under Invert, since Invert never touches α); the others count by the luma of their stored RGB; nothing is composited. Reason: this matches the cut `α·s ≥ ½` exactly, and opaque-only Otsu split the ink of transparent line drawings (`customer-turtle-shell` 15 → 559 nodes/100 mm). Opaque images are unchanged. C1 result accepted: two-colour mixing in sRGB bytes, `colour.rs` not built; Oklab comes back only if trial M1 needs it for the palette. `TraceParams` stays `Copy` (`Swatches` = fixed array of 8); `threshold`/`invert` stay top-level because Invert acts on every method.
