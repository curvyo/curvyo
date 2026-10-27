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
- [ ] 1. Crate, `deny.toml` exception, workspace dependency `visioncortex` 0.8 (AC 17, 29).
- [ ] 2. Luma over white, mask, `auto_threshold` (Otsu) (AC 9, 19, 24).
- [ ] 3. Labelling, specks in mm², hole filling, hole assignment, `TooDetailed` (AC 11, 16, 19).
- [ ] 4. Outlines through visioncortex, assembly, mapping, counts, `poll` and `Cancelled`
      (AC 17, 19, 20, 21, 22).
- [ ] 5. `PARAMS` schema, `TraceParams`, Smooth mapping (AC 4, 5, 6).
- [ ] 6. Golden fixtures and tests: rectangle, disc, ring with dot, specks, transparent logo,
      monotone Smooth and Threshold, determinism (AC 11, 18, 20 to 24).
- [ ] 7. Quality harness `#[ignore]`, synthetic quality set, `baseline.json`, nightly step; the
      2000 x 2000 px timing and the 16 Mpx memory test (AC 25 to 29, 36).

Milestone 2, document, ui-core and editor-wasm:
- [ ] 8. `reduced_copy` in image-core (AC 10).
- [ ] 9. `insert_drawing_above` in document-core (AC 13, 14, 33).
- [ ] 10. `trace_transfer` and `trace_plan` in ui-core (AC 13, 14, 46).
- [ ] 11. `Tool::Trace`, key `I`, Escape, press, double-click, the trace view (AC 1, 2, 3, 12).
- [ ] 12. Preview overlay: dimmed image, hollow outline over a casing, dropped on leaving (AC 2, 7, 8).
- [ ] 13. Create paths: one step "Trace image", groups above images, selection, Select tool
      (AC 12, 13, 14, 30, 31, 46).
- [ ] 14. `trace_job` export and the wasm surface (AC 17, 18).

Milestone 3, frontend and docs:
- [ ] 15. Worker and job runner (AC 10, 12, 17, 46).
- [ ] 16. Trace bar, Select bar button, notices, app setting (AC 1 to 5, 11, 12, 15, 16, 46).
- [ ] 17. npm test: the wasm trace against the native goldens within the tolerance of AC 18.
- [ ] 18. Docs: design-system rows, user page, notice strings test, technical debt (AC 34, 35).

## Validation

- Golden files (counts, boxes, sample points) per fixture, exact on native; the same fixtures in
  `npm test` through the wasm build with the tolerance of AC 18.
- Session tests in `curvyo-editor-wasm` for the tool, the preview and the one-step Create paths.
- `#[ignore]` tests: quality harness (AC 27, 28), 2000 x 2000 px CPU time (AC 25), 16 Mpx memory
  with a counting allocator (AC 26); numbers in the PR description.
- Inkscape is not installed on the build host: the baseline columns stay empty until it is run
  offline (`tools/trace-baseline/README.md`).
