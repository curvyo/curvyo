# Plan for Masked and composite trace, slice 1 (one mask region per trace)

Implementer, 2026-11-04, on `story/masked-trace` (from `main` `d1275c72`). Built against
`specification.md` slice 1 (criteria 1 to 17), `adrs.md` decisions 1 to 6, ADR 0023 §5, ADR 0003,
`ux-notes.md` §1, §2, §7 and §10. Everything marked **(Proposal)** is mine and vetoable at the demo.

## Affected crates/modules

- **`curvyo-ui-core`**
  - `trace_region.rs` (new): the mask region (kernel union of the mask paths at 0.005 mm) and the
    clip of traced outlines to it (box test, point test, `boolean` only for crossing outlines).
  - `trace_region_raster.rs` (new): the region as a pixel mask (scanline, nonzero), the Mask view's
    whitening and the region-only analysis (Threshold, background, analysis report).
  - `trace_transfer.rs`, `trace_text_wire.rs`, `trace_text_colour.rs`: `*_in` variants of the jobs
    that take an optional region; the existing functions delegate with `None` (bit-identical).
  - `trace_text.rs`: `TextRegion::centre` (the line's box centre in mm) for the centre rule.
  - `trace_region.rs` also exposes `mask_operand`, the Boolean commands' collector, for the
    session.
- **`curvyo-editor-wasm`**
  - `session/trace.rs`: the selection is one image plus mask paths; a refusal for text, sketch or
    a second image with mask paths; the region cached per mask; Inside / Outside as tool state.
  - `session/trace_region.rs` (new): the region of the selected mask paths, its JSON for the
    worker, the outside-the-image test and the Overlay's 12 % wash; tests in
    `session/trace_region_tests.rs`.
  - `trace_glue.rs`, `wasm_trace.rs`: the region read from the settings JSON (`"mask"`), the
    region-only defaults (`trace_region_defaults`), the view's mask fields.
- **`frontend`**: Mask item after the View pill, the mode in the session, the region in the
  settings and in the defaults request, the muted rows, the status segment.
- Docs: `docs/adr/0023-...` §5 dated note, `docs/design-system.md` Trace rows,
  `docs/tracing-images.md` "Mask", `docs/technical-debt.md`, spec criterion 2 (lead decision).

## Decisions (Proposal)

- The region crosses the wasm boundary inside the settings JSON (`"mask": {"outside", "region"}`,
  flat numbers in document mm), not as a new slice parameter: no wasm function changes its arity
  (frontend tests call them directly), and the result and mask caches key on the settings string,
  so a new mask or mode invalidates them by construction. Without a mask the settings string is
  unchanged.
- Region-only analysis: the auto Threshold reads the histogram of the region's pixels only; the
  background reads the region's border pixels (pixels of the region next to a pixel outside it or
  the picture's edge), so a mask over the right half of M2 sees the paper; the analysis report
  reads the region's box with the pixels outside made transparent.
- Text objects are exempt from criterion 2 (lead decision): a recognised line is kept whole when
  its box centre lies in the kept area, else it is dropped (its pixels were in-painted).
- A crossing traced shape comes back as a polyline over its whole outline (kernel flattening at
  0.005 mm); tracked under "Boolean results are polylines" in `docs/technical-debt.md`.
- Architect review (2026-11-04): a kernel refusal of the union is its own failure ("0 paths: the
  mask paths could not be combined."), not "outside the image"; a kernel refusal while clipping
  keeps the crossing outlines whole when the region keeps their first point (no exact edge for
  them) instead of dropping them. A linked clone is a mask path only when it resolves to outlines.
- Criterion 11 is checked on a synthetic picture with synthetic OCR lines (the poster tests need
  the OCR weights and are `#[ignore]`d).

## Tasks

1. [x] ui-core tests first: fixtures M1, M2 and the clip cases (criteria 2 to 8, 15).
2. [x] ui-core: `trace_region.rs` clip and `*_in` job variants; no-mask path delegates
   (criteria 1, 2 to 8, 15).
3. [x] ui-core: raster, Mask view whitening, region-only defaults (criteria 9, 10).
4. [x] ui-core: text and shape passes with a region (criterion 11).
5. [x] session: selection with mask paths, refusal row, region, outside-image test, wash, Create
   paths leaves the mask paths (criteria 1, 5, 6, 7, 12, 13, 14, 17).
6. [x] wasm surface and worker: settings `"mask"`, `trace_region_defaults`, view fields
   (criteria 1, 7, 9, 10).
7. [x] frontend: Mask item, mode, muted rows, status segment, keys (criteria 1, 7, 13, 14, 17).
8. [x] criterion 16: CPU time of the clip on 2000 x 2000 px with a 500-node mask (reported).
9. [x] docs: ADR 0023 §5 note, design-system row, tracing-images, technical debt, criterion 2.
10. [ ] full gate; browser files of 0091, 0088 Part B, 0025, 0089 tidy-up.

## Validation

- `cargo nextest run -p curvyo-ui-core -p curvyo-editor-wasm` during work; the existing golden
  JSONs (`curvyo-editor-wasm/tests/fixtures/*_native.json`) stay unchanged.
- New tests: `curvyo-ui-core/tests/acceptance_0094_mask.rs` (M1, M2, clip, raster, text),
  session tests in `curvyo-editor-wasm/src/session/trace_region_tests.rs`, the criterion 16
  harness `curvyo-editor-wasm/tests/acceptance_0094_cpu.rs` (ignored, run in release).
- One existing assertion changed by the spec: `session/trace_tests.rs` expected an image plus a
  rectangle to be "not an image"; with 0094 that is a masked trace, so the case now selects the
  rectangle alone.
- Determinism: the same job twice gives the same numbers (criterion 15).
