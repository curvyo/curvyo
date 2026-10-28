# Plan for Raster trace, slice 1 (black and white)

Implementer, 2026-10-27, on `story/raster-trace` (docs commit `b406a220` on top of `main`).
Design: ADR 0023 and `adrs.md` (visioncortex 0.8 with our pre- and post-processing), UX notes
`ux-notes.md`. Slice 1 = criteria 1 to 36 and 46.

## Affected crates/modules

- **`curvyo-vectorize-core` (new, ADR 0023 §1):** `raster` (luma, mask, Otsu), `regions`
  (`u32` scanline union-find labelling, speck filter in mm², hole assignment, the
  65 000-hole guard), `outline` (one visioncortex `Spline::from_image` per outer boundary and per
  hole, `Spline` → `kurbo::BezPath`, near-straight cubics to lines, map to mm), `params` (the
  schema `PARAMS`, `TraceParams`, the Smooth mapping), `trace` (the one pure function, `poll`).
  Golden fixtures in `tests/fixtures/trace/`; the quality harness as an `#[ignore]` test.
- **`curvyo-image-core`:** `reduced_copy` (longest side 1000 px, area-averaged).
- **`curvyo-document-core`:** `Document::insert_drawing_above`: drawing nodes written directly
  above a sibling, in one commit (the existing drawing writer; no new node kind).
- **`curvyo-ui-core`:** `trace_transfer` (the flat `f64` wire of a trace result between worker
  and session), `trace_plan` (one group "Trace of <name>" per image with one black compound path).
- **`curvyo-editor-wasm`:** `Tool::Trace`, key `I`, Escape, press selects, double-click asks for
  Create paths; the trace view (state, images, the last clicked one); the image's pixels and pixel
  map for the worker; the preview overlay (dim + hollow outline); `create_traced_paths` (one step
  "Trace image"); the free function `trace_job` the worker calls.
- **`frontend`:** the trace worker (cached `WebAssembly.Module`, cancel by `terminate()`), the job
  runner (150 ms debounce, reduced copy first, full size after, 400 ms "Tracing...", 1.5 s Stop),
  the Trace bar, the Trace button in the Select bar, the settings as an app setting, the notices.
- **docs:** `docs/design-system.md` rows, the user page "Tracing an image", technical debt.

## Tasks

Milestone 1, the tracer (`curvyo-vectorize-core`):
- [x] 1. Crate, `deny.toml` exception, workspace dependency `visioncortex` 0.8 (AC 17, 29).
- [x] 2. Luma over white, mask, `auto_threshold` (Otsu) (AC 9, 19, 24).
- [x] 3. Labelling, specks in mm², hole filling, hole assignment, `TooDetailed` (AC 11, 16, 19).
- [x] 4. Outlines through visioncortex, assembly, mapping, counts, `poll` and `Cancelled`
      (AC 17, 19, 20, 21, 22).
- [x] 5. `PARAMS` schema, `TraceParams`, Smooth mapping (AC 4, 5, 6).
- [x] 6. Golden fixtures and tests: rectangle, disc, ring with dot, specks, transparent logo,
      monotone Smooth and Threshold, determinism (AC 11, 18, 20 to 24).
- [x] 7. Quality harness `#[ignore]`, synthetic quality set, `baseline.json`, nightly step; the
      2000 x 2000 px timing and the 16 Mpx memory test (AC 25 to 29, 36).

Milestone 2, document, ui-core and editor-wasm:
- [x] 8. `reduced_copy` in image-core (AC 10).
- [x] 9. `insert_drawing_above` in document-core (AC 13, 14, 33).
- [x] 10. `trace_transfer` and `trace_plan` in ui-core (AC 13, 14, 46).
- [x] 11. `Tool::Trace`, key `I`, Escape, press, double-click, the trace view (AC 1, 2, 3, 12).
- [x] 12. Preview overlay: dimmed image, hollow outline over a casing, dropped on leaving (AC 2, 7, 8).
- [x] 13. Create paths: one step "Trace image", groups above images, selection, Select tool
      (AC 12, 13, 14, 30, 31, 46).
- [x] 14. `trace_job` export and the wasm surface (AC 17, 18).

Milestone 3, frontend and docs:
- [x] 15. Worker and job runner (AC 10, 12, 17, 46).
- [x] 16. Trace bar, Select bar button, notices, app setting (AC 1 to 5, 11, 12, 15, 16, 46).
- [x] 17. npm test: the wasm trace against the native goldens within the tolerance of AC 18.
- [x] 18. Docs: design-system rows, user page, notice strings test, technical debt (AC 34, 35).

## Validation

- Golden files (counts, boxes, sample points) per fixture, exact on native; the same fixtures in
  `npm test` through the wasm build with the tolerance of AC 18.
- Session tests in `curvyo-editor-wasm` for the tool, the preview and the one-step Create paths.
- `#[ignore]` tests: quality harness (AC 27, 28), 2000 x 2000 px CPU time (AC 25), 16 Mpx memory
  with a counting allocator (AC 26); numbers in the PR description.
- The Inkscape columns of `baseline.json` are filled by the tester's baseline run
  (`tools/trace-baseline/README.md`); they are information only (customer decision, Frage 62 A).

## Step 0 and budgets (2026-10-27, build host, release)

Decision A (Frage 62, 2026-10-28): visioncortex stays; the Inkscape columns of `baseline.json` are
now filled by the tester's baseline run and only printed as a gap. Criterion 28 is a regression
guard against the stored `candidate` numbers (+5 %, +0.001 absolute). Our run
at the defaults (Smooth 80, Specks 0.5 mm, automatic Threshold) on the synthetic set:

| Image | Nodes /100 mm | Micro | Max dev px | Mean dev px | Area err % | Wobble /100 mm | Closed % |
|---|---|---|---|---|---|---|---|
| 01 flat logo | 11.50 | 0 | 1.50 | 0.20 | 1.12 | 2.65 | 100 |
| 03 text 300 dpi | 124.39 | 0 | 9.46 | 0.21 | 6.15 | 52.15 | 100 |
| 04 small text | 53.47 | 0 | 1.33 | 0.09 | 12.09 | 16.32 | 100 |
| 05 line art scan | 10.28 | 0 | 3.22 | 0.70 | 31.86 | 3.80 | 100 |
| 07 silhouette | 23.52 | 0 | 1.70 | 0.26 | 0.48 | 4.37 | 100 |
| 10 pixel art | 16.00 | 0 | 0.00 | 0.00 | 0.00 | 4.86 | 100 |
| 11 stencil lines | 46.58 | 0 | 1.12 | 0.15 | 7.69 | 9.39 | 100 |
| 12 technical drawing | 12.75 | 0 | 2.00 | 0.15 | 11.11 | 2.43 | 100 |

Misses of the absolute targets: max deviation above 0.75 px on 7 of 8 images (pixel border
smoothing; on the text the 0.5 mm Specks fill small letter holes, and visioncortex closes 1 px
notches), mean above 0.3 px on the line-art scan (paper noise), area error above the target on the
thin-line images (the metric divides the border error by a thin foreground). Micro paths 0 and
100 % closed everywhere. The fallback of ADR 0023 §8 is not triggered (decision A).

Default Smooth (criterion 4): criterion 21 holds from Smooth 30 up (disc: mean 0.33 px at 20,
0.27 at 30, 0.25 at 80, max 0.66 px from 30 up, 7 nodes everywhere). 80 is kept: on the set it
has fewer nodes, less wobble and a lower mean deviation than 30.

Criterion 25, line art 2000 x 2000 px (400 rings, 20 000 noise pixels), defaults: native 0.091 s
thread CPU (733 paths, 5 723 nodes, 18 279 specks); wasm in Node 0.10 to 0.17 s CPU, same counts;
the reduced copy (1000 x 1000 px) 0.024 s native, 0.029 s wasm. Longest stretch between two
`poll` calls 0.015 s. Criterion 26, 16 Mpx line art: 5.3 bytes per pixel besides the picture.

Not built in this slice: the Layers row tooltip that repeats the success notice (criterion 30,
second half); the dimmed image is the image at 35 % strength over the page, not a separate white
or canvas-coloured wash (criterion 2, Proposal).

# Plan for Raster trace, slice 2 (presets and Advanced, black and white)

Implementer, 2026-10-28, on `story/raster-trace-presets` (from `main` `da69beff`). Design: the
dated note of 2026-10-28 in `adrs.md` (presets as data beside `PARAMS`, Custom as stored state,
Denoise = own integer median, Logo with Upscale Off, the 3.8 mm² tolerance, Ignore background =
foreground regions touching the border), `ux-notes.md` sections 11, 12, 16 and 17. Slice 2 =
criteria 37, 40 to 45, 59 to 69 (colours and slice 3 are out).

## Affected crates/modules

- **`curvyo-vectorize-core`:** `params` (`PRESETS`, `Preset`, the Advanced entries of `PARAMS`,
  `ParamKind::Choice`/`Toggle`, `TraceParams` grows Corners, Upscale, Denoise, Node limit, Invert,
  Ignore background; Corners → corner angle); new `filter` (median 3 x 3 / 5 x 5 on the integer
  luma, 2x integer bilinear upscale); `trace` (the stages in order: luma, Denoise, Upscale, mask,
  Invert, Ignore background with the specks, outlines, Node limit by bisection, the outcome in
  `TraceResult`). Harness: every preset, `candidate_profiles`, the photograph fixture, the
  `preset_relations` test.
- **`curvyo-ui-core`:** `trace_job` takes `&TraceParams`; the wire carries the Node limit outcome;
  re-export `TRACE_PRESETS`.
- **`curvyo-editor-wasm`:** `trace_job` reads the settings as JSON; `trace_presets()` JSON; the
  wash over a traced image (criterion 68 b); the ids of the created groups for the Layers tooltip
  (criterion 68 a).
- **`frontend`:** Preset menu, Advanced block, the stored `{ preset, values }` app setting, the
  Node limit status texts, the Layers tooltip of a trace group.
- **docs:** user page table of presets, `docs/design-system.md` rows.

## Tasks

Milestone 4, the tracer (`curvyo-vectorize-core`), tests first:
- [x] 19. `PRESETS` table and the Advanced schema entries; tests compare the table with criteria
      59 and 60 and with the schema (AC 40, 45, 59, 60).
- [x] 20. Median Denoise and 2x Upscale on the luma, integer only (AC 41, 42).
- [x] 21. Corners, Invert, Ignore background, Node limit in `trace` (AC 41, 43, 44).
- [x] 22. Harness: every preset, the photograph fixture, `candidate_profiles` in `baseline.json`,
      Logo equals `candidate` (AC 61, 65); `preset_relations` (AC 62, 63, 64); preset values tuned.
- [x] 22a. Node reduction above Smooth 80 (`reduce.rs`, `adrs.md` 2026-10-28 second note point 1),
      the area error against the drawn shape (`area_error_true_pct`, point 2), the reworded
      relations; the reduction bound on the set as the `--lib` test (AC 60, 62, 63, 64, 65).
- [x] 23. Denoise and Upscale fixtures of criterion 42.

Milestone 5, ui-core, editor-wasm and frontend:
- [x] 24. `trace_job` with the full settings and the Node limit outcome on the wire; presets JSON
      (AC 43, 45, 60).
- [x] 25. Preset menu, Custom, remembered `{ preset, edited, values }`, Advanced block (AC 37, 40,
      41, 59, 66).
- [x] 26. Slice-1 leftovers: Layers tooltip, the wash (AC 68).
- [x] 27. Docs: user page presets table, design-system rows, notice strings (AC 69).

## Validation

- Unit tests in `params`, `filter` and `trace` (preset table, median, upscale, Invert, Ignore
  background, Node limit), the golden files unchanged (AC 65), the `#[ignore]` harness
  `quality_table` and `preset_relations` in release (AC 61 to 64).
- `npm test`: the preset table and the stored setting in `traceText`, the wasm trace with
  presets.

## Step 0 of slice 2 (2026-10-28, build host, release): blocked

`preset_relations` (criteria 62 to 64) on the set at the automatic Threshold. Logo / Line art
equals the stored `candidate` (criterion 65 holds, goldens unchanged). The relations cannot be
met by tuning the five preset values within the current mapping (Smooth → segment length,
Corners → corner angle, splice 45° fixed); measured node ratio to Logo and maximum deviation:

| Image | S 100/2.0/70/Off/Low nodes | E 80/0.1/70/Auto/Off nodes | E max dev px (Logo) | E area err % (Logo) |
|---|---|---|---|---|
| 01 flat logo | 1.12 | 2.35 | 1.22 (1.50) | 0.9 (1.1) |
| 03 text 300 dpi | 0.79 | 1.73 | 1.76 (9.46) | 6.4 (6.2) |
| 04 small text | 0.72 | 1.37 | 1.12 (1.33) | 23.9 (12.1) |
| 05 line art | 0.97 | 3.87 | 1.83 (3.21) | 14.0 (31.9) |
| 07 silhouette | 0.89 | 2.34 | 1.13 (1.70) | 0.4 (0.5) |
| 10 pixel art | 1.64 (1.00 with Denoise Off) | 1.00 | 0.00 (0.00) | 0.0 (0.0) |
| 11 stencil | 2.25 (0.95 with Denoise Off) | 2.37 | 0.98 (1.12) | 8.1 (7.7) |
| 12 technical | 1.51 (0.94 with Denoise Off) | 2.74 | 1.10 (2.00) | 27.3 (11.1) |

- **Silhouette (criterion 63, at most 0.6 x Logo's nodes per 100 mm):** best 0.72 to 1.0 per
  image (Smooth 100, Corners 70 to 100). Lower Corners adds nodes; visioncortex places one cubic
  per 45° of turn and per inflection, so Smooth (segment length) moves nodes by about 10 %; a
  splice of 90° or 135° does not lower the ratio and raises the deviation.
- **Engraving (criterion 62):** Upscale Auto gives the 25 % lower mean maximum deviation (1.14
  against 2.54 px) and the ordering of criterion 64, but the area error against the 1x thresholded
  reference rises on 03, 04, 11, 12 and the node density exceeds 2.5 x on 05 and 12. Upscale Off
  keeps area and nodes but the mean maximum deviation stays 2.56 px.
- **Photo (criterion 63):** met by tuning: Denoise Off, Upscale Off gives 706 outlines against
  Logo's 155 on the photograph (4.6 x); Denoise Low gives 159.
- Slice-1 crash found on the way and fixed: a noisy photograph made visioncortex panic
  ("STUCK") on regions that touch only diagonally across another region's pocket; every crop now
  fills the pockets its shape encloses, as visioncortex's own clusters do (goldens unchanged).

## Step 0 of slice 2, resolved (2026-10-28, build host, release)

After the architect's second note: the node reduction (Smooth above 80, 0.2 mm at 100) and the area
error against the drawn 4 x 4 samples. `preset_relations` and `quality_table` pass; Logo / Line art
equals `candidate` (goldens unchanged). Final values are criterion 60's.

| Image | nodes L / E / S | density E / L | density S / L | max dev px L / E / S | area err true % L / E |
|---|---|---|---|---|---|
| 01 flat logo | 91 / 214 / 32 | 2.34 | 0.36 | 1.50 / 1.22 / 1.84 | 0.88 / 0.67 |
| 03 text 300 dpi | 706 / 1222 / 170 | 1.72 | 0.29 | 9.46 / 1.76 / 58.83 | 5.87 / 6.07 |
| 04 small text | 95 / 130 / 54 | 1.41 | 0.83 | 1.33 / 1.12 / 9.94 | 25.33 / 18.99 |
| 05 line art scan | 119 / 460 / 68 | 3.83 | 0.57 | 3.21 / 1.83 / 3.44 | 30.89 / 12.27 |
| 07 silhouette | 70 / 164 / 25 | 2.33 | 0.37 | 1.70 / 1.13 / 55.86 | 0.39 / 0.29 |
| 10 pixel art | 168 / 168 / 152 | 1.00 | 0.94 | 0.00 / 0.00 / 2.02 | 0.00 / 0.00 |
| 11 stencil | 833 / 1978 / 483 | 2.38 | 0.59 | 1.12 / 0.98 / 3.91 | 15.46 / 14.45 |
| 12 technical | 273 / 748 / 71 | 2.73 | 0.26 | 2.00 / 1.10 / 3.00 | 33.46 / 9.60 |

- Engraving: density 2.07 x Logo as the geometric mean (worst 3.83 on 05); mean max deviation
  1.14 against 2.54 px (55 % lower); mean area error against the drawn shape 7.8 against 14.0 %
  (44 % lower); 03 is 0.20 points over Logo (within 0.5); letter holes on 03: 14 of 14 (Logo 11).
- Silhouette: density 0.47 x Logo as the geometric mean (worst 0.94 on 10), never more outlines
  than Logo, no outline under 3.8 mm², 100 % closed; within 0.2 mm (+0.001) of the trace before
  the reduction on every image (`silhouette_stays_within_the_smooth_tolerance_on_the_set`).
- Photo on the photograph: 706 outlines against Logo's 155 (4.6 x), max deviation 2.84 px.

Run: `cargo test --release -p curvyo-vectorize-core --test quality -- --ignored --nocapture` and
`cargo test --release -p curvyo-vectorize-core --lib -- --ignored`.
