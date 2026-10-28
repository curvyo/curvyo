# ADRs for Raster trace

Architect, 2026-10-27, revised the same day to the customer answers (Frage 49 B, 50 A, 52 postponed,
54 A, 61). The spec needs **one new ADR, [0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md)**
(Proposed): the tracer crate `curvyo-vectorize-core` wrapping **`visioncortex`** (the library under
VTracer) with our pre- and post-processing, its one pure function, determinism, the Web Worker job
and the quality harness. **No `format_version` change, one new external dependency tree, no trait,
no generic.** Nothing here is `needs-customer`. A spike on 2026-10-27 found B workable (ADR 0023 §7).

## Depends on

- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) (new): `trace(Raster, pixel_map,
  &TraceParams, poll) -> Result<TraceResult, TraceError>`, `auto_threshold`, `PARAMS`; our luma, mask,
  `u32` region labelling and mm² speck filter, then `visioncortex` per region in spline mode, then
  `Spline` → `kurbo::BezPath` mapped to mm; run-to-run deterministic, native and wasm differ by one
  `acos`; worker in both builds, cancel by terminate; fallback to an own pipeline (C) only after step 0.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2, §3: fills the reserved
  `vectorize-core` slot; `ui-core → vectorize-core`; `editor-wasm` goes through `ui-core`, no new edge.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §6: tracing lives in
  `vectorize-core`, not in the kernel.
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §1, §4, §6: `WorkingImage` (RGBA8,
  straight alpha) and `ImageSnapshot::pixel_map(w, h)`; visioncortex's pixel-corner coordinates are
  the same pixel space, so the map applies unchanged.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3: desktop runs the editor as wasm
  in the webview, so the worker and the wasm time are the shipping path on both builds.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §5, §9 and the compound path of
  `0016`/`0054`: one group per image with one compound path (nonzero, holes wound against).
- [ADR 0005](../../docs/adr/0005-extension-and-plugin-model.md) §4: no image type in the WIT interface now
  (Frage 55 A).
- [ADR 0006](../../docs/adr/0006-license.md) and `deny.toml`: AGPL exception for the new crate;
  `visioncortex` and its tree are MIT/Apache-2.0/BSD-2-Clause; notices kept in the third-party list.

## Feature-local decisions

1. **Crate name `curvyo-vectorize-core`** (criterion 17), the reserved slot of ADR 0011. (Proposal)
2. **Dependency is `visioncortex = "0.8"`, not `vtracer`.** `vtracer` 0.6.5 fails `cargo deny`
   advisories (`atty`, `ansi_term`, `adler` unmaintained via `clap` 2 and `image` 0.23) and takes
   files; its binary mode is a 20-line loop over visioncortex that we own. Transitive: `flo_curves`,
   `roots`, `itertools` 0.8, `smallvec`, `bit-vec` 0.6, `num-traits`. (Proposal)
3. **Our pre-processing:** alpha over white, integer luma, Threshold, `u32` 4-connected labelling,
   foreground and hole specks in mm² through `|det|` of the pixel map. Regions go to visioncortex one
   crop at a time; this avoids its `u16` label panic on noisy images and gives `poll` a boundary per
   region. A single region with 65 000 holes or more returns `TooDetailed` (status: raise Specks).
4. **Our post-processing:** `Spline` → `BezPath` directly (no SVG strings), near-straight cubics
   (control points within 0.01 px of the chord) become lines, map to mm, count nodes.
5. **Settings mapping:** Smooth 0 to 100 → `segment_length` 3.5 to 10 px; corner 60°, splice 45°,
   10 iterations fixed and hidden in slice 1; final constants from step 0, monotone. (Proposal)
6. **Determinism:** run-to-run bit-identical; native and wasm differ only through visioncortex's
   `acos` (spike: 9 678 against 9 680 curves on 2000 x 2000 px, same 593 paths). Preview and Create
   paths share the worker, so the user gets what he saw. Native goldens are exact; the `npm test` wasm
   check uses equal path count, nodes within 1 %, boxes within 0.01 mm. Upstream PR to visioncortex
   (basic-operations `acos`); when released, back to 1e-6 mm. (Proposal)
7. **Unchanged from the first version:** Web Worker in both builds, cancel by `terminate()`; timers
   (150 ms, 400 ms, 1.5 s) in TypeScript; reduced copy in `curvyo-image-core`; settings are an app
   setting; Create paths over several images is one `ui-core` command, one history step; source images
   stay (Frage 54 A); harness `#[ignore]` with offline Inkscape/VectorCraft baselines, numbers only.
8. **Fallback C** (own pipeline) only if step 0 misses criterion 28 or 26 after tuning; the PR reports
   per image and the lead asks the customer. The public API does not change.
9. **Slice 2** (presets, denoise, upscale, node limit) gets a dated note here before its build.
   Colours are postponed (Frage 50 A); visioncortex's colour path (which uses hash maps) is not used.

## Spec criteria that must change (for the product owner)

- **18:** "native and wasm builds agree within 1e-6 mm" → run-to-run identical on each build; native
  vs wasm: equal path count, node count within 1 %, bounding boxes within 0.01 mm, until the upstream
  `acos` fix is released (decision 6).
- **16 / 26:** add the one exception: a single region with 65 000 or more holes after speck removal
  gets the status "Too much detail: raise Specks." instead of a trace (decision 3); never a crash.
- **29:** the shipped dependency is `visioncortex` (the library under VTracer), not the `vtracer` crate;
  step 0 runs it as the candidate, Inkscape and VectorCraft as baselines.
- **19 / 21:** outlines follow the thresholded pixel border, smoothed (no sub-pixel position from
  anti-aliasing); criterion 21's mean 0.3 px holds at segment length 10 (0.20 px), not at 4 (0.34 px),
  so the default Smooth must sit at the upper end or 21 is relaxed to 0.35 px mean. (Proposal: keep 21,
  pick the default.)
- **17:** `poll` acts between our passes (every 64 rows) and before each region; one visioncortex call
  (at most about 0.1 s native at 2000 x 2000 px) is not interruptible. The 250 ms bound holds.

## Format

None. Group and compound path are existing kinds (criterion 33).

## Questions for the customer

None. Frage 49 B is built as asked; the fallback C is put to the customer only if step 0 fails.

## Dated notes

- 2026-10-28 (revised the same day to the four presets of criteria 40 and 59 to 69): **slices 2
  and 3: presets, Denoise, the harness per preset**. Within ADR 0023: same crate, same one function,
  no new dependency, no trait, no format change; the settings stay an app setting.
  1. **Presets are data, not code.** `pub const PRESETS: &[Preset]` in
     `curvyo-vectorize-core/src/params.rs`, next to `PARAMS`, in menu order: Logo / Line art, Photo,
     Silhouette for cutting/carving, Engraving. `Preset { key, label, detail, smooth, specks_mm,
     corners, upscale, denoise }`; `key` is a stable string (`"logo"`, `"photo"`, `"silhouette"`,
     `"engraving"`) because the app setting stores it. A preset never holds Threshold. `ui-core`
     re-exports it as `TRACE_PRESETS`, `editor-wasm` hands it to the page as JSON beside
     `TRACE_PARAMS`, so the menu, the values and the user page table come from one table (crit. 45
     and 60 compare it with the schema and the menu). Logo / Line art holds exactly today's
     `DEFAULT_*` values (Upscale **Off**, point 6), and the first-use defaults read from it.
  2. **"Custom" is stored state, not a value match.** No two presets are equal now, but crit. 66
     stores "preset name or Custom with its values" anyway, and matching would need a float tolerance
     on Specks. The app setting holds `{ preset: key | null, values }`; any edit sets `preset` to
     `null` ("Custom"), a pick sets both. An unknown key (a preset removed later) reads as Logo / Line
     art. No matching function in core.
  3. **`PARAMS` grows the Advanced entries** (`advanced: true`): Corners, Upscale, Denoise, Node limit,
     Invert, Ignore background; `ParamKind` gains `Choice(&'static [&'static str])` and `Toggle`.
     `TraceParams` gains the same fields. Full-size preview is a page setting, not a tracer value.
     Every Advanced control of crit. 41 can be honoured:
     - **Corners** 0 to 100 → visioncortex `corner_threshold` = 200° − 2 × Corners, clamped to 10° to
       180° (monotone; **Corners 70 = 60°, today's fixed value**). Splice and iterations stay fixed.
     - **Denoise** (point 4).
     - **Upscale** 2x: integer bilinear on the luma, before the mask; the pixel map gets `scale(0.5)`,
       so Specks (mm² through `|det|`) and the output in mm are unchanged. Auto = working images under
       1 Mpx. Mask and labels take 4x memory for those images only (crit. 26 counts it).
     - **Node limit**: re-run with `segment_length` raised past Smooth 100's 10 px by bisection up to a
       ceiling of 20 px, at most 6 runs, `poll` between runs; the status names the outcome (crit. 43).
     - **Invert** flips the mask. **Ignore background** drops every foreground region that touches the
       image border (decided: in black and white this is "the lightest region of the border" of crit.
       41 and 44 once Invert has made the light background foreground; without Invert the light
       background is never traced).
  4. **Denoise (Photo and Silhouette use Low)** is our own step 1b of ADR 0023 §3, after the luma and
     before Upscale and the Threshold: a **median** of the integer luma, 3 x 3 px for Low, 5 x 5 px for
     High, with a sliding histogram per row, `poll` every 64 rows, one extra byte per pixel. Integer
     only, so run-to-run and native/wasm stay identical up to the existing `acos` (ADR 0023 §4). Pixels,
     not mm: it removes sensor and JPEG noise, which lives at pixel scale; sizes in mm stay with Specks.
     Rejected: bilateral (floating point, two more parameters, slower; for black and white the median
     keeps edges as well), `imageproc` (pulls `image` and `rayon` into a core crate that must not
     depend on `image`, ADR 0023 §1). Removing features narrower than a cutter is not a denoise; it
     waits for the tool diameter of `0026` (spec Later).
  5. **The harness runs every preset.** `quality_table` traces each image once per entry of `PRESETS`
     (4 x 8 traces plus the photograph fixture of crit. 61, seconds in release) and writes
     `candidate_profiles.<key>.<image>` into `tests/fixtures/trace/baseline.json`; `candidate` stays and
     must equal `candidate_profiles.logo` within 1e-9 (crit. 65). Micro paths count below each preset's
     own Specks² (crit. 61); the +5 % regression guard applies per preset. A second `#[ignore]` test,
     `preset_relations`, asserts crit. 62 to 64 on the same numbers (one helper computes the run once);
     Photo is checked on the photograph only and stays out of the ordering of crit. 64. The
     photograph is generated or public domain with its origin in the fixture note. The nightly job's
     command is unchanged; the tester runs it in release before the PR, as in slice 1.
  6. **Findings resolved (decided; the spec text follows).**
     - **Logo / Line art has Upscale Off.** Every image of the quality set is under 1 Mpx (600 x 400
       down to 120 x 60), so Auto would change all of them and break crit. 65. Photo and Engraving keep
       Auto, Silhouette keeps Off.
     - **Crit. 63's 4 mm² test has a tolerance:** Specks removes regions below 4 mm² by pixel area
       before smoothing, and a smoothed outline just above the limit can measure slightly less. An
       outline or hole counts as a miss below **0.95 × 4 mm² = 3.8 mm²**; the same factor applies to
       Photo's (0.1 mm)² and to the per-preset micro-path count of crit. 61.
     - **Ignore background** = foreground regions touching the image border (point 3).
- 2026-10-28 (second note, after step 0 of slice 2 was blocked on criteria 62 to 64): **node
  reduction for Silhouette, the area reference for Engraving, three open points**. Within ADR
  0023: visioncortex stays, same one function, no new dependency, no trait, no format change.
  Throwaway spike on the branch (reverted, nothing committed), numbers from `preset_relations` in
  release.
  1. **Own node reduction after visioncortex** (R-VEC-003, customer-accepted Should). visioncortex
     places one cubic per 45° of turn and per inflection whatever the segment length, so fewer
     nodes need a step after it, not other parameters. New private module
     `curvyo-vectorize-core/src/reduce.rs`, `pub(crate) fn reduce(path: &BezPath, tolerance_px: f64,
     corner_cos: f64) -> BezPath`, called in `trace_regions` right after each `trace_crop` (so the
     existing poll before each region covers it, the Node limit bisection counts reduced nodes and
     `drop_small_outlines` still removes an outline the reduction shrank below Specks).
     - **Algorithm:** the outline's segments (lines and cubics) in pixel coordinates; a joint whose
       turn is below the Corners angle (the same `200° − 2 × Corners` visioncortex gets) may be
       merged, a sharper one never. Greedy from a corner (or the first anchor of a corner-free
       outline): extend a run while one cubic fits it; the fit keeps the run's end points and end
       tangent directions (so joints with the neighbours stay smooth) and solves the two handle
       lengths by least squares on 8 samples per original segment, chord-length parameters, at most
       4 reparameterisation passes; accepted when the sampled distance both ways (originals to the
       candidate, candidate to the originals) is within the tolerance; a run has at most 48
       segments. Lines take part (their direction is the tangent).
     - **Tolerance comes from Smooth, no new control (Proposal):** Smooth up to 80 → none; Smooth
       81 to 100 → `(Smooth − 80) × 0.01 mm`, so **0.2 mm at Smooth 100**; in pixels
       `tolerance_mm / sqrt(|det|)` of the map (Upscale's 0.5 included). Logo / Line art (Smooth 80)
       and every golden stay bit-identical (crit. 65); crit. 23 stays monotone. Rejected: a hidden
       preset field (Custom would carry a value nobody can see or edit); a new Advanced "Simplify"
       control (UI scope the customer has not asked for; if he wants it with R-VEC-003's Inkscape
       comparison, the Smooth mapping becomes its default).
     - **Determinism (ADR 0023 §4):** only `+ − × ÷` and `sqrt`; no `f64::hypot` (libm) and so no
       kurbo `hypot()`, `nearest`, `arclen` or `ParamCurveFit` (`solve_cubic` uses `cbrt`/`acos`);
       lengths are `(x*x + y*y).sqrt()`; the turn test is `dot ≥ corner_cos × |u| × |v|`, with
       `corner_cos` from the integer Corners through a fixed polynomial in basic operations, never
       `f64::cos`. Run-to-run and native/wasm identical apart from visioncortex's own `acos`.
     - **Cost:** 2000 x 2000 px line art 0.09 → 0.16 s thread CPU native, the 1000 x 1000 px
       reduced copy 0.02 → 0.09 s; longest poll gap unchanged (0.012 s).
     - **Lives in `vectorize-core`, not `geometry-core`:** it is the only caller. It moves to the
       kernel when a second caller arrives (the curve refit after booleans of `0016`, or a Simplify
       path tool for the rest of R-VEC-003); the move is one module and no public API.
     - **Measured** (Silhouette Smooth 100, Specks 2.0, Corners 70, Upscale Off, Denoise Low, with
       the reduction): Silhouette / Logo nodes per 100 mm 01 0.40, 03 0.29, 04 0.83, 05 0.60,
       07 0.37, 10 0.96, 11 0.59, 12 0.27; **geometric mean 0.49**; outlines never more than Logo's,
       no outline under 3.8 mm², 100 % closed, ordering of crit. 64 holds on every image. 04 (72 dpi
       text, 0.2 mm < 1 px) and 10 (pixel art: every node is a grid corner, Logo is already exact)
       cannot reach 0.6 without dropping corners; hence the mean in the wording below.
  2. **Engraving: the area error is measured against the drawn shape, not the 1x staircase.** The
     set is synthetic: `render` draws each image with 4 x 4 samples per pixel. The harness keeps
     those samples (before the noise of 05 and 07) as the image's `truth_4x` and adds the metric
     `area_error_true_pct`: XOR of the outlines' 4x raster with `truth_4x`, % of the true
     foreground. The 1x staircase is itself far from the drawing on thin lines (Logo against the
     truth: 25.3 % on 04, 33.5 % on 12), so it punished the outline that is closer to the drawing.
     `area_error_pct` (1x) stays as it is, so the slice-1 numbers, `candidate` and crit. 65 do not
     change; the new metric is stored beside it (`null` for the photograph, which has no truth).
     Max deviation stays against the pixel border (Engraving passes it). Rejected: coverage from
     the grey levels (05 and 07 would carry their noise into the reference); the 2x staircase the
     tracer saw (circular: it rewards whatever Upscale does).
     - **Engraving gets no reduction** (Smooth 80): it promises accuracy, not few nodes. Its node
       cap becomes a mean plus a ceiling, as for Silhouette. Measured (Engraving Smooth 80, Specks
       0.1, Corners 70, Upscale Auto, Denoise Off): mean max deviation 1.14 against 2.54 px
       (55 % lower), no larger than Logo's on every image; area error against the truth E / L:
       01 0.67/0.88, 03 6.07/5.87, 04 18.99/25.33, 05 12.27/30.89, 07 0.29/0.39, 10 0/0,
       11 14.45/15.46, 12 9.60/33.46 (mean 7.8 against 14.0 %); density ratio E / L 2.34, 1.72,
       1.41, 3.83, 2.33, 1.00, 2.38, 2.73, **geometric mean 2.06**; letter holes 14 of 14 (Logo 11).
  3. **Wording for the product owner (criteria 60, 62 to 64):**
     - **60:** Silhouette Smooth 100, Specks 2.0 mm, Corners 70, Upscale Off, Denoise Low;
       Engraving Smooth 80, Specks 0.1 mm, Corners 70, Upscale Auto, Denoise Off; Photo Denoise
       Off, Upscale Off (implementer's step 0: 4.6 x Logo's outlines); add to the Smooth row: "above
       80, Smooth also merges nodes while the outline stays within (Smooth − 80) × 0.01 mm of the
       trace (0.2 mm at 100)".
     - **62:** "Engraving (finer than Logo / Line art, per image of the set): the maximum deviation
       is no larger (plus 0.001 px); the area error against the drawn shape (the 4 x 4 samples per
       pixel the synthetic image was drawn from) is no larger plus 0.5 percentage points, and its
       mean over the set is at least 25 % lower; the mean over the set of the maximum deviation is
       at least 25 % lower; nodes per 100 mm are at most 4 times Logo's on every image and at most
       2.5 times as the geometric mean over the set; closedness 100 %. Test: the letter holes
       (unchanged)." Expected: 0.20 points over on 03 (within 0.5), area mean 44 % lower, deviation
       55 % lower, density 3.83 x worst, 2.06 x mean.
     - **63, Silhouette:** "nodes per 100 mm, over the images with at least 10 per 100 mm at
       Logo / Line art: at most 0.6 times Logo's as the geometric mean, and never more than Logo's;
       outlines no more than Logo's; no outline or hole under 3.8 mm²; Denoise Low; closedness
       100 %; every outline lies within the Smooth tolerance (0.2 mm) of the trace before the
       reduction (both ways, sampled every 0.1 px, plus 0.001 mm); the maximum deviation from the
       thresholded reference is reported, without a bound (Specks 2 mm fills letter holes, 59 px on
       03, which is the purpose)." Expected 0.49 mean, 0.96 worst. `SILHOUETTE_MAX_DEVIATION_PX`
       goes; the reduction bound is a unit test on `reduce` plus the harness check. Photo
       unchanged.
     - **64:** unchanged (holds with the numbers above).
  4. **Open point 1, `ADVANCED_PARAMS`:** accepted as built. `PARAMS` stays the three bar controls
     (crit. 6 test unchanged), `ADVANCED_PARAMS` the six Advanced entries in the menu order; the
     list is the grouping, so no `advanced` flag on `ParamSpec`. Point 3 of the first note is
     amended accordingly.
  5. **Open point 3, Invert:** foreground with Invert is luma **at or above** the Threshold, the
     exact complement of the normal mask (every pixel is in exactly one of the two). Crit. 44 and
     the doc comment say "at or above".
  6. **Open point 5, Custom:** stored state wins over the ux-notes' computed label. The app setting
     is `{ preset: key, edited: bool, values }` (amends point 2 of the first note): a pick sets
     `preset` and `edited = false`, any edit sets `edited = true`; the button shows "Custom" when
     `edited`, and the menu focuses `preset`, which is the "preset the values came from" of the
     ux-notes. Editing back to a preset's values stays Custom; picking the preset again clears it.
     No value comparison anywhere. The ux-engineer updates the "Custom" paragraph of `ux-notes.md`.

### Note of 2026-10-28 (lead, Proposal): Node limit second stage

Raising the segment length to its 20 px ceiling does not always reach a Node limit the maker can expect (blobs fixture: 68 nodes, ceiling 55, limit 54). When the ceiling still has too many nodes, the tracer runs a second bisection of at most 6 runs on the node-reduction tolerance of `reduce.rs`, up to 2 px; only then the status says the limit could not be met. Both stages use the existing poll points; worst case 12 runs instead of 6.
