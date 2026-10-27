# ADRs for Raster trace

Architect, 2026-10-27, against `main` at `311e6b4e` (format 25, `0058` merged). The spec needs **one
new ADR, [0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md)** (Proposed): the
tracer crate `curvyo-vectorize-core`, its one pure function, the slice 1 pipeline, determinism, the
Web Worker job and the quality harness. **No `format_version` change, no new external dependency, no
trait, no generic.** Built on the defaults of Frage 49 (C with step 0), 55 (A) and 60 (B); another
answer to any of them reopens ADR 0023. Nothing else here is `needs-customer`.

## Depends on

- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) (new): `trace(Raster, pixel_map,
  &TraceParams, poll) -> Result<TraceResult, TraceError>`, `auto_threshold`, `PARAMS` schema as
  `const` data; integer luma, mask topology with luma-interpolated crossings, `kurbo` fit in pixel
  space, map to mm at the end; no angle functions, no hash maps; worker in both builds, cancel by
  terminate; harness with offline baselines, nothing of Inkscape, VTracer or VectorCraft in `Cargo.lock`.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2, §3: fills the reserved
  `vectorize-core` slot; uses only its `→ document-core` edge; `ui-core → vectorize-core` as listed;
  `editor-wasm` goes through `ui-core`, so no new edge.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §6: tracing and the
  later R-VEC-002 multi-pass live in `vectorize-core`, not in the kernel.
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §1, §4, §6: the `WorkingImage`
  (RGBA8, straight alpha) and `ImageSnapshot::pixel_map(w, h)`; the unit-square placement makes the
  reduced copy's map just `pixel_map(w', h')`.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3: desktop runs the editor as wasm
  in the webview, so the worker and the wasm time are the shipping path on both builds.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §5, §9 and the compound path of
  `0016`/`0054`: the result is a group with one compound path (nonzero, holes wound against), one
  commit.
- [ADR 0005](../../docs/adr/0005-extension-and-plugin-model.md) §4: no image type in the WIT interface now
  (Frage 55 A); `trace`'s signature is what one would carry.
- [ADR 0006](../../docs/adr/0006-license.md) and `deny.toml`: AGPL exception entry for the new crate; no
  GPL code; external tracers run as separate programs only.

## Feature-local decisions

1. **Crate name `curvyo-vectorize-core`, not `curvyo-trace-core`** (criterion 17). The slot, its
   responsibility and its edges are in accepted ADR 0011; a new name would need a superseding note
   for nothing. The tool and the function stay "trace". (Proposal)
2. **Disagreements with the specification (to be reworded by the product owner):**
   - "What exists today", bullet 4, and criterion 58: the desktop build does not trace on a native
     background thread; it runs the same wasm in the webview (ADR 0001 §3). Both builds trace in one
     dedicated **Web Worker** (ADR 0023 §5).
   - Criterion 17: the cancel check is `poll: &mut dyn FnMut(Progress) -> ControlFlow<()>`, polled at
     work boundaries, and also carries progress for the 2 px line. In the worker a stale job is stopped
     by `worker.terminate()` (a synchronous wasm call cannot receive a message; a shared flag needs
     cross-origin isolation). The 250 ms bound is a native test of `poll`.
   - Criterion 19: `kurbo`'s `fit_to_bezpath` is not Schneider's algorithm; write "fitted with `kurbo`'s
     curve fit within the tolerance". A Schneider fit is the fallback only if the determinism test
     fails (ADR 0023 §4).
   - Criterion 25: the gate is native, but desktop ships wasm. Keep the native gate (measurable in
     CI); the PR also reports the wasm time in a worker; above 4 s on the 2000 x 2000 px line-art
     fixture it goes to `docs/technical-debt.md` and a native-thread follow-up ADR is proposed.
   - Criterion 26: the measure is a counting global allocator in a native `#[ignore]` test; peak
     intermediate data at most 10 bytes per pixel (luma 2, labels 4, union-find 4), under the 16 of
     the criterion.
   - Criterion 18: the native/wasm comparison runs in `npm test` against the editor wasm build (the
     same golden JSON as the Rust test); no new CI job.
3. **Smooth to tolerance:** fit in pixel space, Smooth 0 to 100 linear to 0.05 to 1.0 px as the start
   value, tuned in step 0; the mapping and the final constants are documented in the crate and stay
   monotone (criterion 23). Specks in mm² go through `|det|` of the pixel map's linear part.
4. **Reduced copy** (criterion 10, longest side 1000 px, area-averaged) is a function of
   `curvyo-image-core` beside its existing box downscale, not of the tracer.
5. **Timers stay in TypeScript:** debounce 150 ms, "Tracing..." at 400 ms, Stop at 1.5 s. The core
   has no clock.
6. **Create paths inserts the worker's full-size result** through an `editor-wasm` call into one
   `ui-core` command (criteria 12, 13); the main thread never traces. The worker keeps the RGBA copy
   per image (transferred once) and re-receives it after a terminate.
7. **Settings are an app setting** stored where the other tool settings are; the bar is generated
   from `PARAMS` (criterion 6). No project field.
8. **Harness and step 0** (criteria 27 to 29): `#[ignore]` test in `curvyo-vectorize-core`, own 4x
   scanline rasteriser in test code, dev-dependencies `curvyo-image-core` and `curvyo-svg-core` only.
   `tools/trace-baseline/` runs Inkscape, `vtracer` and VectorCraft's CLI offline into
   `target/trace-baseline/`; only `tests/fixtures/trace/baseline.json` (numbers) is committed. The
   step 0 table is copied into ADR 0023, which then moves to Accepted or is amended. Fixture images
   are own or CC0, each with an origin note.
9. **Slice 2** (quantisation, denoise, upscale, node limit, stacked colours) gets a dated note here
   before its build; no new dependency is expected.

## Format

None. Group and compound path are existing kinds (criterion 33).

## Questions for the customer

None new. Frage 49, 55 and 60 are in the spec with their defaults; ADR 0023 is built on them.
