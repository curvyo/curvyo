# ADR 0023: Raster trace crate, pipeline and background job

**Status:** Accepted (customer, 2026-10-28, Frage 62 option A: keep visioncortex, loosen the absolute
quality targets; the fallback of §8 is not triggered). Proposed by the architect 2026-10-27, revised
the same day to the customer's Frage 49 = B.
Not `needs-customer` by itself: no document model, file format, plugin or platform change. It rests
on the customer answers of `specs/0025-raster-trace` (Frage 49 B "VTracer for now, if it works",
Frage 50 A black and white only, Frage 55 A no extension seam, Frage 60 B VectorCraft as idea source
only). Becomes Accepted when the Inkscape baseline is in, step 0 (§7) does not trigger the fallback (§8) and the customer decides. Fills
the `curvyo-vectorize-core` slot of [ADR 0011](0011-workspace-and-crate-layout.md) §2/§3 (ADR 0003 §6).

## Context

The Trace tool turns the working image of an image node (ADR 0017 §4, §6: RGBA8, straight alpha, at
most 16 Mpx, pixel map `placement · scale(1/w, 1/h)`) into closed paths. The customer wants fewer
nodes, less wobble and no micro paths compared with Inkscape (potrace), a live preview, and 2 to 4 s
for a 2000 x 2000 px image. He chose VTracer/visioncortex as a dependency "for now, if it works", with
our pre- and post-processing. Hard to change later: where the tracer sits in the crate graph, the
shape of its call (the seam a plugin tracer would later copy, ADR 0005 §4), and what "deterministic"
promises, because golden files and the preview-equals-result promise rest on it. The editor runs as
wasm in the webview on desktop too (ADR 0001 §3), so the wasm time is the shipping time.

| Option | Why it lost or won |
|---|---|
| **B. `visioncortex` (the library under VTracer) as a dependency (chosen, customer)** | MIT OR Apache-2.0, pure Rust, no `unsafe` of its own, builds for wasm32, takes an in-memory bit mask, passes `cargo deny`. Its binary path is fast and good on the spike fixtures (§7). Cost: pixel-border outlines (no sub-pixel position), a `u16` label limit, one `acos` that breaks bit-equality between native and wasm (§4). |
| B'. The `vtracer` crate itself | Fails `cargo deny` advisories (`clap` 2 pulls unmaintained `atty`, `ansi_term`; `image` 0.23 pulls unmaintained `adler`), plus `rayon` and `fastrand`, and its entry point takes a file or a `ColorImage`. Its binary mode is a 20-line loop over visioncortex; we write that loop ourselves. |
| C. Own pipeline (sub-pixel contours, `kurbo` fit) | The previous decision of this ADR; better in theory, the most work. Now the fallback (§8). |
| A. Port potrace | GPL-2.0-or-later; reproduces what the customer wants beaten. |
| D. VectorCraft's `trace` crate | Alpha, internal workspace crate bound to `image`, API changing weekly. Benchmark only. |
| Crate `curvyo-trace-core` or a module of `geometry-core` | A new name needs a note superseding ADR 0011 for no gain; pixels and labelling are not the geometric kernel. |

## Decision

### 1. Crate and edges

`curvyo-vectorize-core`, `#![forbid(unsafe_code)]`, builds for `wasm32-unknown-unknown`.
Responsibility: it turns an RGBA8 raster and trace settings into closed outlines in document
millimetres. Edges: `→ document-core` (`Affine`, `Point`, `Length`), `kurbo` (workspace) and
**`visioncortex = "0.8"`** with `default-features = false` (transitive, per `Cargo.lock`: `bit-vec`
0.6, `flo_curves` and `num-traits`; `flo_curves` and `bit-vec` have a few `unsafe` blocks, ordinary
for crates.io leaves). It does **not** depend on `image-core` or
`image`. `ui-core → vectorize-core`; `editor-wasm` reaches the tracer only through `ui-core`.
`deny.toml` gets the AGPL exception for the crate. `cargo deny` stays green; it may add duplicate
*warnings* (`bit-vec`), no error. Dev-dependencies (harness only):
`curvyo-image-core`, `curvyo-svg-core`.

### 2. Public API: one pure function, a schema as data (unchanged)

```rust
pub struct Raster<'a> { pub width: u32, pub height: u32, pub rgba: &'a [u8] } // straight alpha
pub fn trace(raster: Raster<'_>, pixel_map: Affine, params: &TraceParams,
             poll: &mut dyn FnMut(Progress) -> ControlFlow<()>) -> Result<TraceResult, TraceError>;
pub fn auto_threshold(raster: Raster<'_>) -> u8;           // Otsu on the luma histogram
pub const PARAMS: &[ParamSpec];                             // name, kind, unit, range, default, step, group, advanced
```

`TraceResult`: closed `kurbo::BezPath` outlines in document mm (holes wound against their parent),
`Counts { paths, nodes, specks_removed }`. `TraceError`: `Cancelled`, `Invalid(reason)`,
`TooDetailed` (§3 step 3). No visioncortex type appears in the public API, so the fallback (§8)
changes no caller. No trait, no registry, no generic.

**`poll`** is called at fixed work boundaries (every 64 rows of our passes, before every region handed
to visioncortex), never on a clock. `Break` returns `Cancelled`. One visioncortex call is not
interruptible; the largest (a full-page region at 2000 x 2000 px) is under 0.1 s native.

### 3. Pipeline (slice 1, black and white)

Ours before, visioncortex in the middle, ours after:

1. **Luma and mask** (ours): alpha composited over white, integer Rec. 709 luma, foreground =
   luma below the Threshold. A `Vec<bool>`, one byte per pixel.
2. **Regions and specks** (ours): one scanline union-find labelling with `u32` labels, 4-connected
   like visioncortex's `to_clusters(false)` so both agree on what a region is. A foreground region or
   a hole whose pixel count × `|det|` of the pixel map is below Specks² mm² is flipped. VTracer has no
   hole filter and counts specks in pixels; ours is in mm² (criterion: Specks).
3. **Guard** (ours): visioncortex labels with `u16` and calls `panic!("overflow")` at 65 535 clusters.
   We never call its `to_clusters` on the whole image; we hand it one cropped region at a time, where
   it only labels that region's holes. A region with 65 000 holes or more after speck removal returns
   `TooDetailed` (status: raise Specks) instead of a worker crash; recorded in `docs/technical-debt.md`.
4. **Trace** (visioncortex): per region, `clusters::Cluster::image_to_compound_path(offset, &crop,
   PathSimplifyMode::Spline, corner, segment_length, max_iterations, splice)`. It walks the pixel
   border, smooths it and fits cubics (`flo_curves`). Output: `CompoundPathElement::Spline` per
   outline, outer clockwise, holes counter-clockwise, in pixel-corner coordinates.
5. **Assemble** (ours): each `Spline` (start point, then control-point triples) becomes a
   `kurbo::BezPath` (`move_to`, `curve_to`, `close_path`) directly; no SVG string is produced or
   parsed. A cubic whose control points lie on its chord within 0.01 px becomes a line. Map through
   `pixel_map` once, order by first scan position, count nodes.

**Settings mapping (Proposal, tuned in step 0):** Smooth 0 to 100 → `segment_length` 3.5 to 10 px,
monotone; `corner_threshold` 60°, `splice_threshold` 45°, `max_iterations` 10 fixed in slice 1 (not
shown). Colour mode, `color_clusters` and VTracer's hierarchical stacking are not used (Frage 50 A).

### 4. Determinism

- **Same build, same input, same settings: bit-identical result on every run** (spike: two runs,
  equal hash). The binary path of visioncortex has no hash map (only its colour path does), no
  threads, no clock, no random numbers; ours keeps the rules (`Vec`/`BTreeMap`, integer luma).
- **Native and wasm are not bit-identical.** visioncortex measures corner and splice angles with
  `f64::acos`; native uses the platform `libm`, wasm Rust's `libm`. On the 2000 x 2000 px spike image
  both gave 593 paths but 9 678 against 9 680 curves. Replacing that one `acos` with basic operations
  made native and wasm bit-identical, so it is the only source.
- **Consequence (Proposal):** the preview and Create paths run in the same wasm worker, so the user
  always gets what he previewed. Golden files are native and exact; the wasm comparison in
  `npm test` checks the same fixtures for equal path count, node count within 1 % and bounding boxes
  within 0.01 mm. We send visioncortex an upstream PR replacing `acos` by a basic-operations version;
  when it is released, the wasm check returns to 1e-6 mm with equal counts.
- Rejected: a patched copy under `[patch.crates-io]` (a fork to maintain for a ±0.02 % node
  difference); our own `acos` around it (not possible, it is internal).

### 5. Where the job, the debounce and the reduced copy live (unchanged)

Both builds trace in one dedicated Web Worker that instantiates the same `curvyo-editor-wasm` module
(posted `WebAssembly.Module`) and calls `trace_job`. Cancel = `worker.terminate()` and a fresh worker
from the cached module (a synchronous wasm call cannot receive a message; a `SharedArrayBuffer` flag
needs COOP/COEP we do not serve). Debounce 150 ms, "Tracing..." after 400 ms and Stop after 1.5 s are
TypeScript timers. The reduced copy (longest side 1000 px, area-averaged) is a function of
`curvyo-image-core`. Create paths inserts the worker's full-size results (one per selected image,
Frage 61) through `editor-wasm` into one `ui-core` command, one history step; the source images stay
(Frage 54 A). Rejected as before: a native Tauri thread (second code path, different numbers in the
browser), a second small `-wasm` crate, a `SharedArrayBuffer` flag.

### 6. Format

None. Group and compound path are existing kinds; the settings are an app setting.

### 7. Spike result and step 0

Spike of 2026-10-27 (throwaway workspace member, reverted; nothing committed):

| Check | Result |
|---|---|
| `cargo deny check` with `visioncortex` 0.8.10 | ok (licences, advisories, sources); 2 new duplicate warnings |
| `cargo deny check` with `vtracer` 0.6.5 | **fails**: `atty`, `ansi_term`, `adler` unmaintained |
| `wasm32-unknown-unknown` build | ok, no imports needed; spike module 66 KB release, before `wasm-opt` |
| Input | in-memory `BinaryImage` built from our mask; no file I/O, no `image` decoding |
| Run-to-run / native vs wasm | identical / differs by `acos` only (§4) |
| 2000 x 2000 px, 400 shapes + 20 000 noise pixels, 12 859 regions | 0.11 s thread CPU native, 0.19 s in Node (wasm); budget 4 s |
| Rectangle fixture (crit. 20) | 1 outline, 4 nodes, corners exact at (50, 25) … (50, 75) |
| AA disc r = 100 px (crit. 21) | 7 nodes; deviation max 0.66 px, mean 0.20 px at segment 10; mean 0.34 px at segment 4 |
| Ring with dot (crit. 22) | outer and hole wound opposite; the dot is its own region (step 2) |
| Cancel / progress | none in the library; our `poll` between regions (§2) |

Step 0 (first build task) is the harness of the previous version of this section, unchanged: an
`#[ignore]` integration test with its own 4x scanline rasteriser computing the metrics of
`research.md` §5; Inkscape and VectorCraft run offline by `tools/trace-baseline/` into an ignored
`target/trace-baseline/`, only `tests/fixtures/trace/baseline.json` (numbers) is committed. Our
visioncortex-based trace is now the candidate, not a baseline. The nightly job fails on a
regression above 5 %.

**Step 0 result (2026-10-27, build host, release), copied from `specs/0025-raster-trace/plan.md`:**

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

### 8. Fallback to C

If, after tuning §3's mapping, step 0 shows a miss of the criterion 28 targets on the black and white
set that our pre/post-processing cannot fix, or memory above criterion 26 that cannot be brought
down, the PR says so per image and the lead asks the customer whether to build the own pipeline C
(sub-pixel contours, `kurbo` fit, the earlier text of this ADR in git history). The public API (§2)
does not change, so the fallback is a crate-internal swap plus dropping the dependency.

## Consequences

- One new crate, one new external dependency tree (`visioncortex` and 3 small permissive crates), no new `unsafe` in
  our code, no format change.
- Outlines follow the thresholded pixel border, smoothed; there is no sub-pixel position from the
  anti-aliasing. The disc numbers above show this is within the criteria at the default.
- Memory: our mask (`Vec<bool>`, 1 B/px) and labels (`u32` + union-find, 8 B/px) plus visioncortex's per-region
  copies (a bit image, a `u16` label image and 8 B per hole pixel of the region being traced); worst
  case about 18 B/px for a full-page region, measured by the counting allocator of criterion 26; above
  16 B/px it goes to `docs/technical-debt.md` (the criterion says so; no self-made limit).
- Native and wasm differ in the last few nodes until the upstream `acos` fix lands (§4).
- A plugin tracer (ADR 0005) would need an image type in the WIT interface; not written now.
