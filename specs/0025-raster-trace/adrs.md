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
