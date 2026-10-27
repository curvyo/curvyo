# ADR 0023: Raster trace crate, pipeline and background job

**Status:** Proposed (architect, 2026-10-27). Not `needs-customer` by itself: no document model,
file format, plugin or platform change. It rests on the defaults of `specs/0025-raster-trace`
Frage 49 (C, own pipeline with step 0), Frage 55 (A, no extension seam) and Frage 60 (B, VectorCraft
as idea source only); a different customer answer to any of them reopens this ADR. Becomes Accepted
when step 0 (§7) has numbers and they do not overturn §2. Fills the `curvyo-vectorize-core` slot of
[ADR 0011](0011-workspace-and-crate-layout.md) §2/§3 (ADR 0003 §6). Asked by `specs/0025-raster-trace`.

## Context

The Trace tool turns the working image of an image node (ADR 0017 §4, §6: RGBA8, straight alpha, at
most 16 Mpx, pixel map `placement · scale(1/w, 1/h)`) into closed paths. The customer wants fewer
nodes, less wobble and no micro paths compared with Inkscape (potrace), a live preview while values
change, and 2 to 4 s for a 2000 x 2000 px image. Hard to change later: where the tracer sits in the
crate graph, the shape of its call (it is the seam a plugin tracer would later copy, ADR 0005 §4),
and the rule that it is deterministic, because golden files and the preview-equals-result promise
rest on it. The editor runs as wasm in the webview on desktop too (ADR 0001 §3), so the wasm time is
the shipping time. No Web Worker exists in the frontend today.

| Option | Why it lost or won |
|---|---|
| **C. Own pipeline in `curvyo-vectorize-core` (chosen)** | Sub-pixel contours and a tolerance-bounded fit are the two things that beat potrace; both are a few hundred lines on top of `kurbo`, which is already locked. Ours to tune, wasm-clean, no new dependency. Cost: the most work of the three. |
| A. Port potrace | GPL-2.0-or-later is compatible (ADR 0006), but the module loses dual-licensing and the port keeps potrace's limits: binary input, pixel-border contours. It reproduces what the customer wants beaten. |
| B. VTracer/visioncortex as a dependency | Fast to a demo, MIT OR Apache-2.0; but pixel-border regions need our refit anyway, its fitter and API are not ours, and an extra tree has to pass `cargo deny` and `unsafe` review. Kept as a benchmark (§7). |
| D. Depend on VectorCraft's `trace` crate | Alpha, internal workspace crate bound to `vectorcraft-geom` and `image`, API changing weekly. Idea source and benchmark only. |
| Crate: new name `curvyo-trace-core` | Would need a note superseding a row of ADR 0011 for no gain; the reserved name already covers later centerline and R-VEC-002 multi-pass. |
| Crate: a module of `geometry-core` | Pixels, histograms and region labelling are not the geometric kernel; the kernel's one-sentence responsibility would need an "and". |

## Decision

### 1. Crate and edges

`curvyo-vectorize-core`, `#![forbid(unsafe_code)]`, builds for `wasm32-unknown-unknown` (CI finds
it by the `-core` suffix). Responsibility: it turns an RGBA8 raster and trace settings into closed
outlines in document millimetres. Edges: `vectorize-core → document-core` (`Affine`, `Point`,
`Length`) and `kurbo` (workspace, Apache-2.0 OR MIT); the `geometry-core` edge that ADR 0011 allows is
not used until a stage needs it. It does **not** depend on `image-core`: it takes a borrowed raster,
so no decoder is linked into it. `ui-core → vectorize-core` (the trace job wrapper and the insert
command); `editor-wasm` reaches the tracer only through `ui-core`, so ADR 0011 §3 gains no edge.
`deny.toml` gets the AGPL exception for the crate. Dev-dependencies (harness only, never linked into a
build): `curvyo-image-core` (decode PNG fixtures), `curvyo-svg-core` (read baseline SVGs, §7).

### 2. Public API: one pure function, a schema as data

```rust
pub struct Raster<'a> { pub width: u32, pub height: u32, pub rgba: &'a [u8] } // straight alpha
pub fn trace(raster: Raster<'_>, pixel_map: Affine, params: &TraceParams,
             poll: &mut dyn FnMut(Progress) -> ControlFlow<()>) -> Result<TraceResult, TraceError>;
pub fn auto_threshold(raster: Raster<'_>) -> u8;           // Otsu on the luma histogram
pub const PARAMS: &[ParamSpec];                             // name, kind, unit, range, default, step, group, advanced
```

`TraceResult` holds the outlines (each a closed `kurbo::BezPath` in document mm, holes wound against
their parent), the per-colour grouping (slice 2), and `Counts { paths, nodes, specks_removed }`.
`TraceError` is `Cancelled` or `Invalid(reason)` (`thiserror`). No trait, no registry, no generic
(CLAUDE.md §5, Frage 55 A). `ParamSpec` is plain `const` data; `editor-wasm` hands it to the frontend
as JSON and the bar is generated from it (criterion 6).

**`poll`** is the only way out of the call: it is called at fixed work boundaries (every 64 rows of a
raster pass, every outline in the fit, every bisection step of the node limit), never on a clock, with
`Progress { done, total }` in work units. `Break` makes `trace` return `Cancelled` at once. That keeps
the core free of threads and time, gives the progress line its value, and bounds the cancel latency by
work, not by wall time (criterion 17 is measured in thread CPU time on native).

### 3. Pipeline (slice 1, black and white)

1. **Luma** in integers: alpha composited over white, `2126 R + 7152 G + 722 B` (Rec. 709 × 10 000,
   criterion D8), one `u16`-scaled value per pixel. Threshold compares integers.
2. **Mask and regions:** foreground 8-connected, background 4-connected (no topological ambiguity at
   diagonal touches); one scanline union-find labelling. Specks: a region whose pixel count × `|det|`
   of the pixel map's linear part is below `specks²` mm² is flipped (foreground speck to background,
   hole speck filled). This is the true area for rotated or skewed images.
3. **Contours:** marching squares on the cleaned mask decides topology (saddles by the connectivity
   rule above); each crossing point sits at the linear interpolation of the **luma** between the two
   pixel centres, clamped to the cell edge. Topology from the mask, position from the field.
4. **Corners** by the turning of the sub-pixel polyline over a short window, compared as a cosine
   threshold (no angle functions at run time), with a minimum run length.
5. **Fit** in pixel space, per run between corners: a straight run within tolerance becomes a line,
   the rest goes through `kurbo::fit_to_bezpath` over a `ParamCurveFit` of the run. Smooth 0 to 100
   maps monotonically to a pixel tolerance (start: 0.05 px to 1.0 px, linear; tuned in step 0 and
   documented in the crate). Fitting in pixels keeps the deviation targets in pixels and is exact for
   any pixel map, because an affine maps a cubic to a cubic.
6. **Assemble:** close every outline, map through `pixel_map` once at the end, order outlines by
   their first scan position, count nodes.

Slice 2 (quantisation, merge into most common neighbour, stacked colours, denoise, upscale, node
limit by bisection over step 5 only) gets a dated note in `specs/0025-raster-trace/adrs.md` before its
build. No new dependency is expected; any one that appears needs that note and `cargo deny`.

### 4. Determinism

Same input and params give the same result on every run and OS; native and wasm agree within 1e-6 mm
with equal counts (criterion 18). Rules: integer luma and labelling; `Vec` and `BTreeMap` only (no
hash-map order); no `sin`, `cos`, `atan2`, `exp`, `powf` in our code (platform `libm` differs from
wasm's); no clock, locale or threads; f64 basic operations and `sqrt` only (IEEE-exact). `kurbo`'s fit
is the one place we do not control; a test runs the golden fixtures through the wasm build in
`npm test` and compares with the native goldens. If it diverges, the fit is replaced by an own
Schneider fit on basic operations, which is an amendment here, not a new crate.

### 5. Where the job, the debounce and the reduced copy live

- **Both builds run the trace in one dedicated Web Worker** that instantiates the same
  `curvyo-editor-wasm` module (the compiled `WebAssembly.Module` is posted from the main thread, so no
  second download or compile), and calls one export `trace_job(params_json) -> result`. The worker
  holds its copy of the RGBA bytes (transferred once per image, not per job) and the reduced copy.
- **Cancel = terminate.** A worker inside a synchronous wasm call cannot receive messages, and a
  shared flag needs `SharedArrayBuffer`, which needs cross-origin isolation headers we do not serve.
  A stale job is stopped with `worker.terminate()` and a fresh worker is started from the cached
  module. In the worker `poll` posts progress and always continues; `Break` is used by native tests
  and any later native caller.
- **Debounce 150 ms, "Tracing..." after 400 ms, Stop after 1.5 s** are TypeScript timers in the
  frontend. The core has no time.
- **Reduced copy** (longest side 1000 px, area-averaged) is a function of `curvyo-image-core`, which
  already owns the box downscale; the pixel map of the copy is `pixel_map(w', h')` (ADR 0017 §1).
- **Create paths** inserts the worker's last full-size result through an `editor-wasm` call into a
  `ui-core` command: one history step (criterion 13). The main thread never traces; determinism makes
  the inserted result the one previewed.

| Rejected | Why |
|---|---|
| Native thread in `curvyo-app` via a Tauri command | Faster (wasm is about 1.5 to 3x slower), but a second code path and a different number in the browser. Revisit only if the wasm time misses 4 s (§Consequences). |
| A second, small `-wasm` crate for the worker | Smaller instance, but a new crate and build output for a size problem not yet measured. |
| `SharedArrayBuffer` cancel flag | Needs COOP/COEP on the Tauri protocol and on the browser deploy (GitHub Pages cannot set them). |

### 6. Format

None. The result is a group and a compound path, existing kinds; the settings are an app setting.
`format_version` is unchanged (criterion 33).

### 7. Quality harness and step 0

The harness is an `#[ignore]` integration test of `curvyo-vectorize-core` with a rasteriser of its own
(nonzero scanline fill at 4x, test code) computing the metrics of `research.md` §5. Fixtures in
`tests/fixtures/trace/`: synthetic ones (rectangle, disc, ring with dot) are generated by test code;
real images are PNGs with an origin note, own or CC0.

**External tracers never enter `Cargo.lock`.** Inkscape (potrace), VTracer (`vtracer` CLI) and
VectorCraft (its own CLI) are run offline by a script in `tools/trace-baseline/` on the developer's
machine, writing SVGs to an ignored `target/trace-baseline/`. The harness reads those SVGs and writes
`tests/fixtures/trace/baseline.json` with numbers only. No GPL output and no output of another tracer
is committed; running a GPL program is not linking to it. Step 0 is this harness plus those three
baselines, built before the pipeline; its table is copied into this ADR. Reading VectorCraft's MIT OR
Apache-2.0 sources as ideas is fine; copying a file needs Frage 60 = A, a dated amendment listing the
files and the notices kept.

The nightly workflow runs the harness and fails on a regression above 5 % (criterion 36); the PR gate
runs only the fast fixture tests.

## Consequences

- One new crate, no new external dependency, no new `unsafe`, no format change.
- The wasm worker holds a second linear memory: the module's own memory plus the RGBA copy (64 MB at
  16 Mpx) plus at most 10 bytes per pixel of intermediate data (luma 2, labels 4, union-find 4); the
  bound of criterion 26 is checked by a counting allocator in a native test.
- Terminate-to-cancel loses the worker's cached image; it is re-sent (one transfer) after a cancel.
- The 4 s budget is gated on native (criterion 25), but desktop ships wasm. If the wasm time on the
  2000 x 2000 px line-art fixture is above 4 s, the PR records it in `docs/technical-debt.md` and the
  native-thread option above becomes a follow-up ADR.
- A plugin tracer (ADR 0005) would need an image type in the WIT interface; not written now
  (Frage 55 A). `trace`'s signature is the shape that type would carry.
