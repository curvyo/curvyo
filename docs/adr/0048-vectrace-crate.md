# ADR 0048: `vectrace`, one trace library crate

**Status:** Proposed (architect, 2026-11-10). These parts were decided by the customer on
2026-11-10 (`specs/0108-trace-library-extraction/plan.md`): the name `vectrace` in the directory
`vectrace/` (Frage 1 B), OCR as a module of the same crate (Frage 2 A), and the licence
MIT OR Apache-2.0, with the weights staying Apache-2.0 (Frage 3 A). This ADR becomes Accepted
when the PR of `story/trace-library-extraction` (slices 1 to 5) is merged. Amends by dated
notes: [ADR 0011](0011-workspace-and-crate-layout.md)
§2/§3/§6 (new crate, two crates removed), [ADR 0023](0023-raster-trace-crate-pipeline-and-worker.md)
§1/§2/§5 (crate, `Affine` in the API, the worker job), [ADR 0028](0028-ocr-text-in-trace.md)
§1/§4 (crate, the `rayon` exception) and [ADR 0006](0006-license.md) (one crate is not AGPL).
Inventory: `specs/0108-trace-library-extraction/inventory.md`.

## Context

The tracer (`curvyo-vectorize-core`, 9.9 k lines), the text reader (`curvyo-ocr-core`, 1.2 k) and
the recognition passes in `curvyo-ui-core/src/trace_*.rs` (7.9 k) are going to be used by a
standalone app and a SaaS. They need to move to their own repository with a `git mv`. The slice-0
inventory measured the edges that block this:

- `Affine`, `Length`, `Point`, `Angle` and the `Shape` family from `document-core`. They are plain
  data, except that `Affine` computes with `mul_add`. `kurbo::Affine` does not, so it is not a
  drop-in replacement.
- `curvyo-geometry-core::boolean` in the mask clip (`trace_region`) and in the composite join
  (`trace_parts`).
- `curvyo-text-core` fonts in the fit (`trace_fit*`).

Options considered:

- **A. One crate, no `curvyo-*` dependency, Curvyo-only work behind one seam (chosen).** The
  boolean clip stays in Curvyo and reaches the job through a `Mask` trait. Font fitting and
  document writing stay in Curvyo as well.
- **B. vectrace depends on `curvyo-geometry-core`.** Lost: that crate pulls in `document-core`,
  and that is the coupling this ADR removes. The repository could not be cut out.
- **C. Move the boolean kernel into vectrace.** Lost: the kernel is about 1,000 lines on
  `i_overlay`, and Curvyo uses it for Boolean, Fracture, Flatten and Offset. Curvyo would then
  depend on the trace library for geometry, which is the wrong direction, or keep a second copy.
- **D. vectrace returns raw results and Curvyo builds the job around them.** Lost: the text job
  filters lines by the mask before it traces, and the counts in the wire are taken after the
  clip. Curvyo would have to repeat all four job paths (mono, colour, text, text and colour).
  That makes the glue larger, the opposite of the plan's goal.
- For the units: **`kurbo::Affine`** lost because a different last bit in `determinant` changes
  the speck threshold, so the goldens would change. **Mirror types of `RectBounds`, `StarFrame`
  and the rest** lost against one flat `Primitive` enum whose fields are the wire's numbers.

## Decision

### 1. Crate and edges

`vectrace/` (crate `vectrace`). Until it is extracted, every `*-core` rule applies to it:
`#![forbid(unsafe_code)]`, it builds for `wasm32-unknown-unknown` in CI, it has no filesystem,
clock, threads or UI, and native and wasm results are bit-identical. Dependencies: `kurbo`,
`thiserror`, `rten` and `rten-tensor` (pinned as in ADR 0028), `sha2`. It has no `curvyo-*`
dependency; dev-dependencies keep `curvyo-image-core` and `curvyo-svg-core` until slice 5 of 0108
removes them. Edges: `ui-core → vectrace`, `editor-wasm → vectrace`. `ui-core` re-exports nothing
from it. `curvyo-vectorize-core` and `curvyo-ocr-core` are removed. `kurbo` is a public dependency,
so a kurbo major version bump is a breaking change of vectrace. Licence `MIT OR Apache-2.0`; the
`LICENSE-*` files come with the extraction.

### 2. What lives where

| vectrace | Curvyo (`ui-core`, adapter) |
|---|---|
| tracer, colour, analysis, schema and presets; `ocr` (reader and line acceptance); text pass and inpaint; shape recognition; the jobs and their wire | `trace_region` (implements `Mask`), `trace_region_raster`, `trace_parts`, `trace_plan`, `trace_fit*`, `trace_words`, `trace_text_notice`, and `trace_adapter` (`Primitive` to `Shape`, `with_primitives`, `keeps_fill`). The settings JSON stays in `editor-wasm` (vectrace has no serde). |

Every string the user sees stays in Curvyo. vectrace returns statuses and reasons as enums or
`&'static str` diagnostics.

### 3. Units

`Mm(f64)` replaces `Length`, and `Angle(f64)` (radians) replaces `Angle`. `PixelMap([f64; 6])`
maps pixel corners to millimetres. Its `compose`, `apply_point`, `apply_vector`, `determinant`,
`inverse` and `is_finite` are copied from `document-core` with `mul_add` unchanged; `to_kurbo()`
is used to map the outlines, as today. `kurbo::Point` and `Vec2` hold coordinates in mm.
`PixelLength` (tolerance in px) keeps its name. A test in Curvyo asserts
`PixelMap == document Affine` bit for bit on fixed matrices.

### 4. The mask seam

```rust
pub trait Mask {
    /// Whether the mask keeps `p` (mm); text lines are kept by their box centre.
    fn keeps(&self, p: Point) -> bool;
    /// `outlines` clipped to the kept area.
    fn clip(&self, outlines: &[BezPath]) -> Clipped;
}
pub struct Clipped { pub outlines: Vec<BezPath>, pub changed: bool, pub kept_whole: usize }
```

One production implementation (`TraceRegion` in Curvyo, on the boolean kernel) and one test
double that is actually used (a box mask in vectrace's job tests), so `CLAUDE.md` §5 holds. The job
recomputes `paths` and `nodes` after a clip that changed something, as `clip_trace` does today.

### 5. Public API (types and signatures; doc comments in the code)

```rust
// input
pub struct Raster<'a> { pub width: u32, pub height: u32, pub rgba: &'a [u8] } // straight alpha
pub struct Rgb8 { pub r: u8, pub g: u8, pub b: u8 }
pub struct TraceParams { /* as today, specks: Mm */ }
pub struct ColourParams { /* as today */ }
pub const PARAMS: &[ParamSpec]; pub const ADVANCED_PARAMS: &[ParamSpec];
pub const METHOD_PARAMS: &[ParamSpec]; pub const PRESETS: &[Preset];   // schema as data

// one call per task, cancellable through poll (fixed work boundaries, never a clock)
pub type Poll<'a> = &'a mut dyn FnMut(Progress) -> ControlFlow<()>;
pub fn trace(r: Raster<'_>, map: PixelMap, p: &TraceParams, poll: Poll<'_>) -> Result<TraceResult, TraceError>;
pub fn trace_colours(r: Raster<'_>, map: PixelMap, p: &ColourParams, poll: Poll<'_>) -> Result<ColourTrace, TraceError>;
pub fn colour_palette(r: Raster<'_>, map: PixelMap, p: &ColourParams, poll: Poll<'_>) -> Result<Vec<PaletteEntry>, TraceError>;
pub fn analyse(r: Raster<'_>) -> Result<AnalysisReport, TraceError>;
pub fn auto_threshold(r: Raster<'_>) -> u8;
pub fn suggest_background(r: Raster<'_>) -> Option<Rgb8>;
pub fn sample_colour(r: Raster<'_>, x: u32, y: u32) -> Option<Rgb8>;
pub fn selected(r: Raster<'_>, p: &TraceParams) -> Vec<bool>;
pub enum TraceError { Cancelled, Invalid(&'static str), TooDetailed }

pub mod ocr {
    pub struct OcrModel; // load(detector: &[u8], recogniser: &[u8], alphabet: &str) -> Result<Self, OcrError>
    pub fn read_lines(m: &OcrModel, w: u32, h: u32, rgba: &[u8],
                      poll: &mut dyn FnMut(OcrProgress) -> ControlFlow<()>) -> Result<Vec<TextLine>, OcrError>;
    pub fn crop_lines(/* as today */); pub fn read_crop(/* as today */);  // the worker pool
    pub struct OcrLine { /* quad, text, confidence, positions */ }       // accepted(), plausible(), scaled()
    pub fn read_text(m: &OcrModel, r: Raster<'_>, poll: /* as read_lines */) -> Result<Vec<OcrLine>, OcrError>;
    pub enum OcrError { WrongModel, BadModel(String), Cancelled, Invalid(String) }
}

// shapes: plain data, fields as on the wire
pub enum Primitive {
    Rect { origin: Point, width: Mm, height: Mm, corner_radius: Mm },
    Ellipse { centre: Point, rx: Mm, ry: Mm },
    Polygon { centre: Point, radius: Mm, angle: Angle, points: u32 },
    Star { centre: Point, radius: Mm, angle: Angle, points: u32, inner_ratio: f64 },
}
pub const POINTS: RangeInclusive<u32> = 3..=1024;          // == document-core PointCount, tested
pub enum Join { Miter, Round, Bevel }
pub struct Band { pub primitive: Primitive, pub rotation: Angle, pub width: Mm, pub join: Join }
pub struct ShapeRegion { pub outlines: Vec<usize>, pub shapes: Vec<Option<(Primitive, Angle)>>,
                         pub band: Option<Band>, pub bands: Vec<Band> }

// the worker's job: one entry point, the wire as its result
pub enum Mode<'a> { Mono(&'a TraceParams), Colours(&'a ColourParams) }
pub struct Job<'a> { pub raster: Raster<'a>, pub map: PixelMap, pub mode: Mode<'a>,
                     pub lines: &'a [OcrLine], pub mask: Option<&'a dyn Mask>, pub shapes: bool }
pub fn run(job: &Job<'_>, on_percent: &mut dyn FnMut(u8)) -> Vec<f64>;
pub fn text_regions(job: &Job<'_>) -> Vec<f64>;            // the text section only

pub mod wire {
    pub const HEADER: usize = 7;
    pub enum Status { Traced, TooDetailed, Invalid }
    pub fn read(wire: &[f64]) -> Option<Output>;            // no with_primitives step (Curvyo's)
}
pub struct Output { /* today's TraceWire: status, counts, outlines, palette, layers, joined,
                      unclipped, texts, shapes: Vec<ShapeRegion> */ }
```

Renames compared with today: `TraceWire` becomes `Output`, `TraceShapeRegion` becomes
`ShapeRegion`, `TRACE_WIRE_HEADER` becomes `wire::HEADER`, `trace_add_shapes` goes into
`Job::shapes`, and the ten `trace_*_job_{with,in}` functions become `run`. A native caller gets
plain data from `wire::read(&run(..))`; the round trip is exact and cheap next to the trace. The
job reports progress but cannot be cancelled: the worker stops it with `terminate()` (ADR 0023 §5).
Native callers that need to cancel use the low-level calls and their `poll`.

### 6. The wire

The flat `f64` list of ADR 0023 §5 and its sections (header, outlines, unclipped, text, shape,
colour tail) becomes vectrace's documented result format, unchanged byte for byte. It is a
transport between two halves of one build, not a stored format: it has no version field, and any
change to it is a breaking change of vectrace. No Curvyo file format changes; `format_version` is
unchanged.

### 7. Threads

The scoped exception of ADR 0028 §4 moves to vectrace unchanged: in the wasm32 tree `rayon` may
appear only through `rten`. Tracer-only users still compile `rten`; a cargo feature for it is not
added until a second user asks for one (`CLAUDE.md` §5).

## Consequences

- The glue is one adapter module in `ui-core` (`Primitive` to `Shape`, `with_primitives`,
  `impl Mask`, `Rgb8` to `Color`) and plain forwarding in `editor-wasm`.
- About 3,100 lines of `trace_*` stay in `ui-core` for good reasons: the boolean kernel, fonts and
  the document model. The plan's slice-4 target of 1,500 lines is not reachable without breaking
  options B or C above.
- The code that moves is relicensed from AGPL-3.0-or-later to MIT OR Apache-2.0 (the customer's
  decision as copyright holder). Curvyo stays AGPL; depending on a permissive crate is allowed.
  `deny.toml` drops two AGPL exceptions.
- The name has no suffix, against `CLAUDE.md` §6. The lead amends the suffix line and the §7 wasm32
  list.
- Before extraction (slice 5): replace the dev-dependencies `curvyo-image-core` (check the
  reference tables, since decoding may change pixels) and `curvyo-svg-core`, and remove the OCR
  test's path into `frontend/public/models/`.
- Not solved: a standalone wasm package with its own JSON settings (it needs serde, or the
  settings as numbers). That is decided when the package is built.

## Note 2026-11-10 (slice 3b, names in the code)

The §5 listing leaves the module paths open. In the code: `Job`, `Mode`, `run` and `text_regions`
are in `vectrace::job`, `Mask` and `Clipped` in `vectrace::mask`, the shape types and `POINTS` in
`vectrace::shapes`, `Output` at the crate root, and `Status`, `Palette`, `Layer`, `Text`, `Texts`,
`HEADER` and `read` in `vectrace::wire`. In `vectrace::ocr`, `read_text`, `crop_text` and
`read_crop_text` take `OcrLine` (the accepted line, `f64`); `read_lines`, `crop_lines` and
`read_crop` keep their `f32` `TextLine` form because the reader's own tests use it, and
`read_text` keeps today's `(model, rgba, width, height, on_percent)` arguments. Beyond the list,
three items stay public with a reason: `node_count` (Curvyo recomputes `Output::nodes` after joining
parts), `shapes::valid_ratio` (a Curvyo test pins its `InnerRatio` to it) and
`shapes::add_shapes` (two CPU-time tests time the shape pass alone; the worker sets `Job::shapes`).

## Note 2026-11-10 (architect review of slice 5, API against §5)

Accepted deviations from §5, kept as they are:

- `ocr::read_text` keeps `(model, rgba, width, height, on_percent)` instead of a `Raster` and a
  `poll` (the worker and its pool call it so; one entry point, no second form).
- `node_count`, `shapes::valid_ratio` and `shapes::add_shapes` stay public for the reasons in the
  note above.
- The pixel coordinates of `ocr::TextLine`, `LineCrop` (`f32`) and `OcrLine` (`f64`) stay plain
  numbers, as in ADR 0028 §2; they are source pixels, not lengths.
- The schema (`PARAMS`, `PRESETS`, `Selection::LABELS`, `Denoise::LABELS`, `Upscale::LABELS`)
  keeps its English labels as data. §2 "every string the user sees stays in Curvyo" means
  statuses, notices and reasons; the schema's labels are the exception §5 already lists.
- Further `pub` items beyond the §5 listing are the types and limits its signatures reach
  (`Counts`, `Diagnostics`, `Progress`, `Phase`, `Selection`, `Swatches`, `Denoise`, `Upscale`,
  `ColourLayer`, the analysis report types, `wire::{Palette, Layer, Text, Texts}`, the `MAX_*` and
  `DEFAULT_*` constants, the OCR model hashes and `LINE_HEIGHT`). They are part of the API.
- The `PixelMap` bit test uses constants frozen from `document-core`'s `Affine`; vectrace has no `curvyo-*` crate, not even as a dev-dependency (done 2026-11-11).

Done before the PR (review findings, `CLAUDE.md` §5 units): re-export `Diagnostics`
(its fields are public on `TraceResult` and `ColourTrace`); make `wire::Text::pixel` and `positions`
`Mm`, `Preset::tolerance_px`/`specks_mm` and `DEFAULT_TOLERANCE_PX`/`DEFAULT_SPECKS_MM`
`PixelLength` and `Mm`, and `add_shapes` take a `PixelMap`.

Status: this ADR becomes Accepted when the PR of `story/trace-library-extraction` is merged (it
carries slices 1 to 5); the later move to an own repository only adds a note.
