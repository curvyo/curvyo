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
