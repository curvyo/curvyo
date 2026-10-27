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
- Inkscape is not installed on the build host: the baseline columns stay empty until it is run
  offline (`tools/trace-baseline/README.md`).

## Step 0 and budgets (2026-10-27, build host, release)

Inkscape is not installed on the build host, so the baseline columns of `baseline.json` are empty
and the relative targets of criterion 28 (node density at most 60 %, wobble at most 50 % of
Inkscape's) are not evaluated yet; `tools/trace-baseline/README.md` says how to add them. Our run
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
100 % closed everywhere. Whether this triggers the fallback of ADR 0023 §8 needs the Inkscape
numbers on the same set (the targets are relative to them where they are relative).

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
