# Research for 0025 raster-trace

Author: product-owner. Date: 2026-10-14. Status: input for the customer and the UX engineer; nothing here is accepted.

Customer request (2026-10-14): 0025 needs more thought. A new tool, with its settings in the top-middle area (not a property, not the right side). Start with the vectorisation functions Inkscape has, but check Illustrator, "VectorCraft" and Affinity. Inkscape's results should be beaten: smoother, fewer nodes, fewer wobbly lines, fewer micro paths. Bitmap import comes first (spec 0058).

**Decided by the customer (2026-10-14):**
- Compute time of 2 to 4 s for a 2000 x 2000 px image is acceptable (section 6, CPU budget).
- The UX mockup of the Trace tool is accepted (section 6, UI shape).
- Our licence is AGPL-3.0-or-later ([ADR 0006](../../docs/adr/0006-license.md)). GPL-2.0-or-later code is compatible with it; only GPL-2.0-only is not. Section 4 has the licence matrix.
- The product named "VectorCraft" is the GitHub project `storytold/vectorcraft` (section 3). It is by the same organisation as ArtCraft.
- Tracing should be extensible with other methods (algorithms, an MCP server, machine learning). Not in the first slice; ideas are collected in section 10.

Labels: **Fact** = read in a source named next to it. **From memory** = my knowledge, not re-checked today. **Uncertain** = sources conflict or I found none. **Proposal** = my idea, not a customer requirement.

Fetch problems today: inkscape.org and helpx.adobe.com (main pages) returned HTTP 403, so Inkscape and Illustrator details rest on search snippets, mirrors and memory.

## 1. Inkscape Trace Bitmap

Sources: [Inkscape manual, Tracing an Image](https://inkscape-manuals.readthedocs.io/en/latest/tracing-an-image.html) (snippet only), [Logos By Nick guide](https://logosbynick.com/trace-bitmap-in-inkscape/), [Inkscape issue 1057 on the 1.0 autotrace options](https://gitlab.com/inkscape/inbox/-/issues/1057). Accessed 2026-10-14.

**Engine.** Potrace (Peter Selinger), GPL-2.0-or-later ([SUSE package listing](https://packagehub.suse.com/packages/potrace/), [Alpine](https://pkgs.alpinelinux.org/package/v3.21/community/x86/potrace)). Fact. Since 1.0 the dialog also offers autotrace-based modes (centerline, colour). Fact (issue 1057); what each does is Uncertain.

**Dialog structure.** Two tabs: Single scan (one path, one colour) and Multiple scans (several stacked paths). Fact.

| Mode | What it does | Source |
|---|---|---|
| Brightness cutoff | Pixels brighter than a threshold (0 to 1) are background. | Fact (guide); range from memory |
| Edge detection | Canny-style edge map, then a threshold. Gives outlines of edges, not filled areas. | From memory |
| Colour quantization | Reduces to N colours, traces one colour as foreground. | From memory |
| Brightness steps (multiple) | N scans at evenly spaced brightness levels. | Fact |
| Colours (multiple) | N quantised colours, one path each. | From memory |
| Grays (multiple) | N grey levels. | From memory |

**Options.** Fact unless noted.
- Scans: number of layers.
- Smooth: Gaussian blur before tracing.
- Stack scans: layers stacked, so there are no gaps between them (otherwise tiled). Fact.
- Remove background: drops the lightest layer. Fact.
- Suppress speckles: drop patches smaller than N pixels (potrace `turdsize`). Fact for the name; mapping from memory.
- Smooth corners: potrace `alphamax`, 0 gives polygons, about 1.33 gives all curves. From memory.
- Optimise paths: potrace `opttolerance`, merges curve segments within a tolerance. From memory.
- Live preview with a "Update" button; result replaces or sits above the image. From memory.

**Known weaknesses (the customer's complaint).** These are partly from memory and from general community experience, not from one citable source:
- Potrace works on a binary bitmap. Colour is done by many threshold passes, so every layer is traced separately. Edges that should be shared are traced twice with slightly different wobble.
- Without Stack, neighbouring colour layers leave hairline gaps; with Stack, layers hide each other and cutting jobs get duplicate paths. Uncertain on the exact UX; the effect is well known.
- Staircase input (low-res, JPEG noise) turns into wobbly curves, because potrace fits to the pixel edges and has no sub-pixel position.
- Many tiny paths on photos, because the speckle filter is one number per run and not relative to image size.
- Node count is higher than a human would draw: potrace is optimal in polygon segment count, but curve joins at near-straight places stay.
- Inkscape's own docs say it is not meant to reproduce a photo exactly. Fact ([Inkscape forum reply via search](https://alpha.inkscape.org/vectors/www.inkscapeforum.com/viewtopic4090.html?t=18003)).

## 2. Illustrator Image Trace

Sources: [Adobe: Image Trace panel options](https://helpx.adobe.com/lv/illustrator/using/tracing-artwork-live-trace-or.html), [Adobe: Image tracing presets](https://helpx.adobe.com/vn_vi/illustrator/using/image-trace-presets.html), [Webucator](https://www.webucator.com/article/how-to-use-image-trace-in-adobe-illustrator/), [Illustrator How](https://illustratorhow.com/image-trace/). Accessed 2026-10-14.

- **Flow.** Place the image, select it, open the Image Trace panel (or the Control bar), tick Preview, pick a preset or tweak, then **Expand** to get editable paths. Until expanded the result is a live "tracing object" tied to the image. Fact.
- **Presets.** High Fidelity Photo, Low Fidelity Photo, 3 Colors, 6 Colors, 16 Colors, Shades of Gray, Black and White Logo, Sketched Art, Silhouettes, Line Art, Technical Drawing. Fact for most; the exact list differs by version (Uncertain). High Fidelity Photo creates very many paths and is known for large files and cleanup. Fact.
- **Modes.** Black and White, Grayscale, Color (with palette: automatic, limited, full tone, from a swatch library). Fact for modes; palette options from memory.
- **Threshold** (B&W mode, 0 to 255, default 128). Fact.
- **Advanced:**
  - **Paths**: how tightly paths follow the pixels. Sources disagree on the direction (Adobe Illustrator page says low = looser, Animate pages say the opposite). Uncertain; check in the app.
  - **Corners**: higher gives more corner points. Fact.
  - **Noise**: ignore areas smaller than N pixels; Adobe suggests 20 to 50 for high-res and 1 to 10 for low-res images. Fact.
  - **Method**: Abutting (cutout, neighbours share an exact edge) or Overlapping (stacked, each path slightly overlaps the next). Fact.
  - **Fills/Strokes**, **Snap curves to lines**, **Ignore white**, "Create" options. From memory.
- **What is good:** one panel, live preview, presets named after the task, Expand step, Abutting method for gap-free colour. **What is weak for makers:** no sizes in mm, Noise is in pixels, it is a Creative Cloud subscription, the panel is not near the canvas.

## 3. "VectorCraft" and Affinity

### 3.1 VectorCraft (storytold/vectorcraft)

The customer named [github.com/storytold/vectorcraft](https://github.com/storytold/vectorcraft) ("VectorCraft, in front of ArtCraft"). Read 2026-10-14: the repo page, the root `Cargo.toml`, `crates/trace/Cargo.toml`, `crates/trace/src/lib.rs` and the directory listings. I did **not** read the algorithm files (`contour.rs`, `fit.rs`, `subpixel.rs`, ...), run the code, or measure output quality. Everything below is Fact from those files unless marked.

**ArtCraft.** Yes, the same organisation (`storytold`) has `artcraft`: "an intentional crafting engine for artists, designers, and filmmakers", Rust, about 27.8k stars (org page). The VectorCraft README says it is "built by the ArtCraft team". The org also hosts a family of Rust "clean-room reimplementation" apps (wordcraft, photocraft, pdfcraft, filmcraft, designcraft, figcraft, deckcraft, effectcraft, craft-libs), Apache-2.0 where the licence was shown. The ArtCraft licence was not shown on the org page (Uncertain).

**What it is.** An open-source vector editor, "a clean-room reimplementation of Adobe Illustrator, rebuilt in pure Rust". Native on macOS, Windows, Linux and FreeBSD, and in the browser via WebAssembly (`apps/vectorcraft-web`, Trunk). egui-based UI. Status: alpha, version 0.11.0, 2,839 commits, 8.9k stars, 4.2k forks, 263 open issues, status date 2026-10-10 (very active). Self-assessed parity "about 88 % of Illustrator 2026's menu items", about 65 % "ready for real work" (the README says these scores are self-assessed). It has an MCP server (`vectorcraft-cli mcp`, stdio) and a JSON control channel, so agents can draw and export. No built-in ML or generative AI. It also has a `pathops` crate (curve booleans), `plugins`, `format`, `svg`, `eps` crates. It overlaps with Curvyo's drawing side, so it is also a competitor to watch, not only a source for tracing.

**Licence.** `MIT OR Apache-2.0` (root `Cargo.toml`, README: dual-licensed "at your option"). Copyright "(c) 2026 ArtCraft Team and the VectorCraft contributors". Bundled assets have their own licences (ASSETS.md, not read). The ArtCraft name and logos are trademarks and excluded. Both licences are on the ADR 0006 allow-list, so code can be used if we keep notices.

**The tracer: crate `crates/trace` (`vectorcraft-trace`).** Doc comment: "A clean-room tracer built from the published ideas behind bitmap tracers."
- Dependencies: `vectorcraft-geom` (their own), **kurbo** (same as ours), `image`, `serde`, `thiserror`. No C code, no potrace, no ML. Workspace lint `unsafe_code = "deny"`. Whether the crate builds for wasm32: the app does, so likely yes (Uncertain for the crate alone). `image` is a decoding dependency; for us decoding is not core (0058).
- Interface: input `Raster` (RGBA8); `trace(raster, TraceParams) -> TraceResult` and `trace_within`, which takes an anchor limit and returns `TooComplex` when exceeded. This is the "node budget" idea from section 4 (item 9), as a hard stop and not a refit loop (Uncertain which).
- Modes: Black and White (threshold, default 128), Grayscale (1-D k-means, 2 to 256 grays), Color (Limited, Full Tone, Automatic, Document Library palettes), Logo (flat-colour logos).
- Layer methods: Abutting and Overlapping. These are Illustrator's names; the tracer mirrors Illustrator's Image Trace panel (Paths, Corners, Noise, Method, Snap Curves To Lines, Create fills/strokes), plus presets (Default, Auto Color, photo, N colours, Shades of Gray, line art, Outline, Flat Logo).
- Pipeline (crate docs): quantise (anti-aliasing blends do not create palette colours) -> merge same-colour components smaller than `noise` pixels into their most common neighbour -> follow pixel-crack boundaries into closed loops (outer clockwise, holes counter-clockwise) -> move cracks to **sub-pixel** edge positions -> cut outlines at junctions and fit piece by piece, **so abutting regions share one curve along their edge**.
- Corners (0 to 100) map to a turn angle of 35 degrees (at 100) to 90 degrees (at 0), with corners the pixels rounded off restored. Paths (0 to 100) set the curve tolerance from about 0.08 px to 0.78 px. Straight runs become lines, round outlines true circles, the rest "as few cubic Béziers as tolerance allows".
- Extras: centerline strokes (`centerline.rs`), recognition of circle, ellipse, rectangle and regular polygon as exact shapes (`shapes.rs`), linear gradients as one path (`gradient.rs`), auto grouping, snapping lines to level/upright within 2.5 degrees, a `mosaic` module (not documented).
- Tests: a `tests/quality/` harness (`measure.rs`, `inputs.rs`, `raster.rs`). Contents not read. The README gives no tracing quality numbers, and I found no comparison with Inkscape or Vectorizer.ai (Uncertain). The only quality claims are rendering speed and parity, which are not about tracing.

**What this means for us.**
1. **It is nearly the pipeline of approach C** (section 8): sub-pixel contours, shared borders, kurbo fitting, corner restoration, primitive recognition, centerline. It validates that design, in the same language and with the same curve library.
2. **Reusable, as an idea (yes).** The stage order, the junction-cut-then-fit trick for shared edges, "anti-aliasing blends do not count as palette colours", merging specks into their most common neighbour instead of deleting them (no holes), the Corners/Paths parameter mapping, the TooComplex anchor limit.
3. **Reusable, as code (possible, with care).** MIT OR Apache-2.0 is compatible with AGPL-3.0-or-later; we may copy or depend on it if notices are kept. Practical limits: it is an internal workspace crate tied to their `vectorcraft-geom` types and `image` (our units are newtypes per CLAUDE.md §5 and decoding is outside core), so a drop-in dependency is unlikely; adaption means a fork of a few files. I do not know if it is published on crates.io (Uncertain; check). A new crate or vendored code needs an ADR.
4. **Benchmark (yes, first).** Build it as a dev tool and run it through our metric harness (section 5) next to Inkscape and VTracer. Its tracing quality is unproven until we measure it. At 2,839 commits in an alpha product that describes itself as 65 % ready, I would not assume it is good; I also do not know how much of it was written by hand or generated (Uncertain; not a reason to reject, but a reason to measure).
5. **What we should not do.** Do not copy its feature parity goals ("Illustrator clone": Illustrator names, Illustrator presets). Our mm-based, cutting-oriented presets and units are the difference. Do not take its code without reading and testing it; do not depend on an alpha, fast-moving, 0.x internal crate whose API will change weekly; do not use the "VectorCraft"/"ArtCraft" names or logos (trademarks). Do not treat its MCP server as part of our tracer (see section 10).
6. **The earlier candidates for the name** (a Figma plugin "VectorCraft" with AI credits, [Gumroad](https://merahburam.gumroad.com/l/vectorcraft-starter-package), and the Rust editor) are now resolved: the customer meant the Rust project. The Figma plugin is irrelevant.

### 3.2 Other references

I keep **CorelDRAW PowerTRACE** (Outline, Centerline, Quick Trace, preview, AI-assisted trace in the 2025 suite, redesigned workflow in 2026; [Corel tutorial](https://learn.corel.com/tutorials/convert-images-to-vector-files-quickly-and-easily-with-powertrace/), [Corel AI trace](https://www.coreldraw.com/en/pages/items/1600148.html), [itechguides review](https://www.itechguides.com/?p=978757)), **Vectorizer.ai** and **Vector Magic** as quality benchmarks.

**Vectorizer.ai** ([site](https://vectorizer.ai/)). Vendor claims only: sub-pixel boundary placement from anti-aliasing values, neighbouring shapes kept aligned (gap-free), fits circles, ellipses, rounded rectangles and stars as true shapes, arcs plus quadratic and cubic Béziers, corner control, palette size, symmetry, SVG/EPS/DXF/PDF output, 3 MP and 30 MB limit, pay per use. The service is closed and cloud-based; it is a benchmark, not something we can use (data leaves the machine, CLAUDE.md §3).

**Vector Magic** ([Capterra](https://www.capterra.com/p/178297/Vector-Magic/)). Auto-detects settings, user adjusts image type, detail and colours; desktop and online; about 5 USD per month (Capterra, Uncertain, conflicting trial info). Reviewers use it for laser cutting. Closed.

**Affinity.** Most sources say Affinity Designer 2 has **no automatic trace**; tracing is manual with the Pen tool, and DrawPlus AutoTrace was not carried over ([Logos By Nick](https://logosbynick.com/vector-image-tracing-with-affinity-designer/), [Xquissive](https://www.xquissive.com/how-to-trace-an-image-with-affinity/)). One guide mentions a "Vectorize Bitmap" option; I found no official source (Uncertain). I did not reach the official Affinity help; the Canva-era Affinity app (2025/2026) is not checked. Conclusion for us: Affinity gives no pattern to copy. Treat as "no".

**CorelDRAW PowerTRACE** (taken as the third reference). Modes: Outline (filled shapes), Centerline (strokes for line art, which matters for plotters and engraving), Quick Trace; live preview before/after with colour count and node/object count shown. Fact (Corel tutorials); the node/object count display is from memory.

### 3.3 Comparison

| | Inkscape | Illustrator | PowerTRACE | Vectorizer.ai | VectorCraft (trace crate) | Affinity |
|---|---|---|---|---|---|---|
| Engine | potrace (+ autotrace) | closed | closed | closed, cloud | own, Rust (Fact) | none |
| Licence | GPL-2.0-or-later | closed | closed | closed | MIT OR Apache-2.0 | closed |
| Colour | stacked threshold passes | palette modes | yes | yes | palettes, Abutting/Overlapping | - |
| Sub-pixel edges | no | unknown | unknown | claimed | yes (docs) | - |
| Shared edges, gap-free | no (Stack option) | Abutting | unknown | claimed | yes (docs) | - |
| Centerline | autotrace mode | no (strokes option) | yes | no | yes (docs) | - |
| Primitives (circle, rect) | no | no | no | claimed | yes (docs) | - |
| Size units | px | px | px | px | px | - |
| Runs offline | yes | yes | yes | no | yes | - |
| Quality measured by us | no | no | no | no | no | - |

"Fact (docs)" for VectorCraft means stated in its crate docs, not measured.

**What we do better (Proposal).** Show node and path counts live; sizes in mm instead of pixels; cutting presets; a stacked and a gap-free (cutout) result as a choice; trace happens in a tool next to the canvas.

## 4. Libraries and algorithms for a core crate

Hard rules: `*-core` builds for wasm32, no I/O, no threads, no `unsafe` (CLAUDE.md §5, §6). Licence is AGPL-3.0-or-later ([ADR 0006](../../docs/adr/0006-license.md)). GPL-2.0-or-later and GPL-3.0 code can be combined with it; GPL-2.0-only cannot. The ADR's `cargo deny` allow-list currently names only permissive licences (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, ...), so a GPL-2.0-or-later dependency would need an allow-list change (a decision, not a waiver, per ADR 0006 item 2). That is a process step, not an incompatibility.

### 4.1 Licence matrix

| Candidate | SPDX identifier | Compatible with AGPL-3.0-or-later? | Note |
|---|---|---|---|
| potrace (Selinger) | GPL-2.0-or-later (Fact, section 1) | **Yes** | "or later" lets the combined work be under GPL-3.0 / AGPL-3.0. Verify the header in the version we port; some old files may say otherwise (needs check). |
| Python `potracer`, npm `potrace` (ports) | GPL-2.0-or-later or GPL-2.0-only? Not verified ("GPL-style" only) | **Needs check** | Compatible only if the port says "or later". Check each before reading it. |
| VTracer | MIT OR Apache-2.0 (Fact, section 4 table) | Yes | Allow-listed. |
| visioncortex | MIT OR Apache-2.0 (Fact; some bundled files MIT/BSD-2-Clause) | Yes | Allow-listed. |
| VectorCraft `trace` | MIT OR Apache-2.0 (Fact, root `Cargo.toml`) | Yes | Allow-listed; keep notices; names and logos excluded. |
| kurbo | Apache-2.0 OR MIT (in workspace) | Yes | Allow-listed. |
| autotrace | GPL-2.0-or-later or LGPL-2.1-or-later (From memory) | **Needs check** | Wording per version; only the "or later" variants work. Not planned for use. |
| ImageTracer (JS) | Unlicense (From memory) | Yes if so (needs check) | Reading aid only. |
| Inkscape (as offline baseline tool) | GPL-2.0-or-later (Fact, via potrace) / GPL-3.0 (From memory) | Not linked, no issue | Only run to measure; nothing shipped. |
| Vectorizer.ai, Vector Magic | proprietary | No (not usable) | Manual benchmark only. |
| ML models (section 10) | varies: Apache-2.0, MIT, CC-BY-NC, OpenRAIL, "research only" | **Needs check per model** | Model weights are not code; non-commercial or use-restricted licences conflict with "everything else is deliberately free" (ADR 0006). |

Rule for the matrix: only the identifier counts, not the project's name. "GPL-2.0-only" and "GPL-2.0" without "or later" are a **no**.

### 4.2 Options

| Option | Licence | wasm | Notes |
|---|---|---|---|
| **potrace** (C) | GPL-2.0-or-later (Fact, see section 1) | not as C | Quality reference. License is compatible. The blocker for the C code is technical: no C in a `*-core` crate (wasm32, no `unsafe`), so only a Rust port is an option. |
| Rust or JS ports of potrace | GPL-style (Fact for the Python `potracer` and the npm `potrace`, [licenses.dev](https://licenses.dev/npm/potrace/2.1.8), [PyPI](https://pypi.org/project/potracer/0.0.1)); exact identifier not checked | possible | I found **no Rust crate port** in today's search (Uncertain; check crates.io). A port is a derivative work of potrace: it must stay GPL-2.0-or-later-compatible, keep Selinger's copyright notices, and carry the source-offer obligations of the AGPL as a whole (which we already accept). The real costs are those obligations and that the algorithm's limits (binary input, no sub-pixel) stay. |
| **VectorCraft `trace`** | MIT OR Apache-2.0 (Fact, section 3.1) | the app builds for wasm; crate alone Uncertain | Rust, kurbo-based, sub-pixel, shared edges, colour, centerline, shapes. Internal alpha crate; quality unmeasured. Idea source and benchmark first, code reuse possible by fork of a few files. |
| **VTracer / visioncortex** | MIT or Apache-2.0 (Fact: [Debian review of visioncortex](https://dfsg-new-queue.debian.org/reviews/rust-visioncortex), [vtracer](https://dfsg-new-queue.debian.org/reviews/rust-vtracer); some bundled files under MIT/BSD-2-Clause) | yes, it ships a wasm webapp ([docs.rs](https://docs.rs/vtracer-webapp)) | Rust, colour, stacked or cutout hierarchy, speckle filter, corner/splice/length thresholds, `simplify`. Pipeline: colour clustering, region tree, path walking, spline fit. Readme: [VTracer](https://github.com/visioncortex/vtracer). Unknown: whether `visioncortex` has `unsafe` or std-only I/O deps; the crate version I saw was 0.8.x (Uncertain). |
| autotrace | GPL-2 or LGPL (From memory, Uncertain) | C | Discontinued per [AlternativeTo](https://alternativeto.net/software/vtracer/?p=2). Centerline mode is its unique feature. |
| ImageTracer (JS) | Unlicense (From memory, Uncertain) | JS only | Simple, low quality; useful only as a reading aid. |
| **kurbo** (in workspace) | Apache-2.0 / MIT | yes | Has `fit_to_bezpath`, `fit_to_bezpath_opt` (Schneider-style fit with error tolerance), simplification, `BezPath` offsets. Fact that it is in `Cargo.toml` (0.13). Features of its fitter are From memory. |
| Own code | ours | yes | Marching squares with linear interpolation, Schneider fit via kurbo, corner detection, despeckle. Small and well documented. |

**Quality techniques (from memory unless a source is named; all are standard):**
1. **Pre-filter**: median or bilateral filter to remove JPEG noise and dither without softening edges. A Gaussian blur (Inkscape's "Smooth") softens edges and shifts contours.
2. **Upscale before tracing** (2x with bicubic or Lanczos) for low-res input. This gives the contour finder more sub-pixel information and removes staircases. Costs 4x memory.
3. **Sub-pixel contour extraction**: marching squares on the grey or alpha field, with linear interpolation of the crossing point. The contour is at the true threshold crossing and not at the pixel border. This is the single biggest cure for wobble (Vectorizer.ai makes a similar claim, vendor claim).
4. **Corner detection** on the contour (turning angle over a window, with a minimum run length). Split into runs between corners.
5. **Curve fit with error bound**: Schneider's algorithm (Graphics Gems, 1990) on each run, tolerance in physical units (for example 0.05 mm). Fewer nodes come from raising the tolerance; wobble drops when the input is sub-pixel and smoothed along the contour first.
6. **Merge adjacent beziers** that are near-collinear or G1 within tolerance (a post-pass).
7. **Despeckle by area**, in mm² and not pixels, plus dropping holes below the same area. Also drop paths whose bounding box is below a minimum feature size (useful for lasers: a 0.1 mm island cannot be cut).
8. **Shared borders for colour layers**: build a planar region partition (region adjacency graph), trace each boundary once, and reuse it for both neighbours. This gives gap-free Abutting output without overlap. Stacked output instead paints bottom to top with each layer extended under the next. VTracer offers both (Fact, readme).
9. **Node budget**: if the result exceeds N nodes, raise the tolerance and refit (bisect) until it fits. Proposal.
10. **Primitive recognition**: replace near-circle or near-rectangle contours with true shapes (Vectorizer.ai claim). Could, not for MVP. Proposal.

## 5. Quality metrics (Proposal)

Compare our tracer with Inkscape (potrace, default settings and the best manual settings) on a fixed test set. The reference is the **binarised or quantised input**, rendered at 4x resolution.

| Metric | How | Target (Proposal) |
|---|---|---|
| Node density | nodes per 100 mm of path length (or per 1000 px of perimeter) | at most 60 % of Inkscape's on the same image |
| Micro paths | number of paths with area below 0.25 mm² (or 4 px² at the original scale) | 0 at default settings |
| Max deviation | Hausdorff distance, output contour to the reference contour, in px | at most 0.75 px at default (1 px at most for photo) |
| Mean deviation | mean distance as above | at most 0.3 px |
| Area error | XOR area of rasterised output versus reference, as % of foreground | under 1 % for logo, under 3 % for photo |
| Smoothness | sum of abs(curvature change) along the path, or the count of tiny curvature sign flips per 100 mm ("wobble count") | at most 50 % of Inkscape's |
| Gaps and overlaps (colour) | rasterise the union of layers on a white background; count pixels that are uncovered or covered twice along shared borders | 0 gaps in Abutting mode |
| Closed-ness | all paths closed (or marked open for centerline) | 100 % |
| Time | wall clock, 2000 x 2000 px, host and wasm | see section 6 |

The metrics run in a test binary in the tracer crate (no UI), on fixtures in `tests/fixtures/`. Inkscape numbers are measured once with `inkscape --actions` or the CLI and stored as a JSON baseline (GPL tool only runs offline for comparison; none of its output is shipped).

**Test set (about 10 images, Proposal):**
1. Flat logo, 3 colours, clean edges (PNG).
2. Logo with gradient and drop shadow.
3. Black text, several sizes, serif and sans, 300 dpi scan.
4. Small text, low resolution (screenshot, 72 dpi) to test upscaling.
5. Line art scan (ink drawing, paper texture, slight skew).
6. Pencil sketch (light grey strokes on grey paper, uneven lighting).
7. Silhouette for laser cutting: black shape with holes and fine details (animal, filigree), JPEG with ringing.
8. Photo, portrait, for engraving (Floyd or threshold effect).
9. Photo, landscape, for a colour trace (8 colours).
10. Pixel art / low-colour icon, hard edges.
11. Engraving stencil with thin lines (0.3 mm at 300 dpi) to test connectivity.
12. Technical drawing with straight lines, circles and arcs (tests corner detection and primitive recognition).

## 6. Recommended architecture direction (Proposal; the architect decides)

**Pipeline stages**, each a pure function in a `*-core` crate:
1. Input: decoded RGBA buffer plus pixel size in mm (from 0058; see Frage 5 for missing dpi).
2. Pre-process: alpha handling, optional crop to selection, optional denoise, optional 2x upscale.
3. Classify: threshold (single), quantise to N colours (colour), or edge/centerline (line art).
4. Cluster and despeckle: connected regions, area filter in mm².
5. Contour: sub-pixel marching squares, shared borders for adjacent regions.
6. Fit: corner detection plus Schneider fit via kurbo, tolerance in mm, node budget.
7. Assemble: compound paths with holes (even-odd or non-zero as the document model needs), layering Stacked or Abutting, ordered, with fill colour.
8. Result: new paths in the document; the original image stays.

**Parameters that deserve a UI control (Proposal):** Mode (Black and white, Colours, Outline only); Threshold (B/W) or Colours (2 to 16); Detail / smoothness (one slider that maps to fit tolerance and corner threshold); Ignore specks smaller than (mm², with px hint); Layering (Stacked / No gaps). Behind an "Advanced" expander: denoise, upscale, corner threshold, node budget, invert, remove background, tolerance in mm.
**Defaults, not controls:** contour method, curve fitter, merge rules, ordering.

**Presets for makers (Proposal):**

| Preset | Mode | Notes |
|---|---|---|
| Logo / Line art | B/W or 2 to 8 colours | Strong corner detection, upscale low-res, tight tolerance, specks 0.5 mm². |
| Photo | 6 to 12 colours, or tone steps | Denoise on, larger specks, node budget on. Warning that photos do not trace well. |
| Silhouette for cutting | B/W | Closed, simple, no islands below the cut feature size (needs 0058's mm size), smooth, low node count, outline holes kept. |
| Engraving | B/W or grey steps | Keeps fine detail, no despeckle of thin lines, optional centerline later. |

**UI shape (the UX engineer proposes the details):** A new **Trace tool** in the toolbox. When active with an image selected, the top-middle context area (the place the customer named) shows the preset menu, mode, the two or three sliders, a live preview toggle, a node/path counter, **Apply** and **Cancel**. No Properties panel entry. Preview draws as an overlay, the document changes only on Apply (one undo step). The UX engineer should also propose: what happens to the source image (keep, hide, lock), and what the preview looks like on a dark theme.

**Decided (customer, 2026-10-14):** the UX mockup of the Trace tool is accepted, and 2 to 4 s of compute for a 2000 x 2000 px image is acceptable. The targets below are therefore comfortable for the full-resolution run; the live-preview rules still apply (a 2 to 4 s preview while dragging a slider would feel stuck).

**CPU budget (Proposal; not measured, estimates from memory of VTracer and potrace performance; Uncertain).**
- 2000 x 2000 px (4 MP), B/W, no upscale: target at most 0.5 s on a 2020 laptop, single thread. Marching squares and fitting are linear; the fit dominates and scales with node count.
- Colour, 8 colours: target at most 2 s. Clustering is the cost.
- Upscale 2x of 4 MP is 16 MP of work: target at most 3 s, so it is off by default above 1 MP.
- Wasm is typically 1.5 to 3 times slower than native; threads are not available in the core. The browser needs a Web Worker (platform side).
- **Live preview needs debounce and a worker.** On the desktop, run the trace on a background thread (platform crate, not core), debounce 150 ms after the last slider move, cancel stale jobs, and preview on a downscaled copy (longest side 1000 px) while a slider is dragged; run the full-resolution trace on release and on Apply. The core API takes a cancel check callback or works in chunks, so a stale job stops. Accepting a cancel parameter is an architect question.

## 7. Licensing and dependency risks (for the architect)

1. **Potrace is GPL-2.0-or-later, which is compatible with AGPL-3.0-or-later** (ADR 0006); licence is not a blocker for a port. What a port costs: (a) the ported part is a derivative work of Selinger's code and must stay under GPL-compatible terms; (b) ADR 0006 keeps dual-licensing open because the customer is sole copyright holder, and a potrace-derived module cannot be relicensed without Selinger's agreement, so that option is lost for that module (the rest of the code stays ours); (c) `cargo deny` and the ADR 0006 allow-list have to be extended for a vendored GPL source (or the code lives in our crate, where it is covered by our own licence and needs only notices); (d) the algorithm limits remain (binary input, no sub-pixel). Section 8 weighs these.
2. **VTracer/visioncortex is MIT or Apache-2.0**: compatible with the ADR 0006 allow-list. Check: transitive dependencies, `unsafe` use (core crates forbid it only in our own code; `cargo deny` plus the ADR rule on dependencies decide), wasm32 build in CI, crate maintenance (last release seen 2025-10-17, Uncertain), API stability.
3. **A new crate needs an ADR** (CLAUDE.md §5). `trace-core` is a new crate. Alternative: a module in an existing geometry crate. Architect decides.
4. **Image decoding is not core** (0058 owns it). Tracing takes a plain pixel buffer.
5. **Inkscape/potrace output as test baseline:** running a GPL tool offline to produce comparison numbers is fine; do not commit its output as fixtures without checking (GPL output is generally not covered by the tool's licence, but this is a legal question, Uncertain).
6. **Vectorizer.ai, Vector Magic:** closed, cloud. Never used as dependency; only as manual benchmark with our own images. Uploading customer images to them breaks the "data leaves the machine" rule.
7. **Photos and copyright:** the tool traces any image. A one-line note in the help is enough; no checking (Proposal).
8. **Determinism:** the same input and settings must give the same output (no floating-point differences between host and wasm). Needed for golden-file tests (CLAUDE.md §5). Risk: wasm and host floating-point rounding; compare with a tolerance.

## 8. Candidate approaches

**A. Port potrace.** Best known quality baseline, same engine as Inkscape. Binary only; no colour or sub-pixel; this is what the customer is "not 100 % satisfied" with. Cost: medium. Licence risk: **low** (GPL-2.0-or-later is compatible; costs are derivative-work notices and losing dual-licensing for that module). Quality risk: high, because it reproduces Inkscape's behaviour. Does not beat Inkscape by itself. Still **not recommended as the base**; reasonable as a reference for the optimal-polygon and corner-smoothing ideas (`alphamax`, `opttolerance`), which we can study and re-implement.

**B. Adopt VTracer/visioncortex as a dependency, add pre- and post-processing.** Fastest to a working colour trace (days to a first demo). MIT/Apache. Already stacked and cutout, speckle filter, `simplify`. Weakness: its curve fitting is its own and its output is not smoother by nature; pixel-based region boundaries, so wobble must be fixed by us in a post-pass (re-fit with kurbo, merge). Cost: low to medium. Risk: depends on a third-party crate's maturity and `unsafe`. Quality unknown until measured (Uncertain).

**C. Own pipeline: sub-pixel contours, shared borders, kurbo Schneider fit, mm-based tolerances, node budget.** Best chance of beating Inkscape on every metric in section 5 and the closest to the customer's words (smoother, fewer nodes, no micro paths). Fully ours, AGPL, wasm-clean, no new dependency beyond kurbo. Cost: highest (B/W first, colour with shared borders second). Risk: effort and tuning time.

**Recommendation (Proposal): C in two steps, with B as a measured benchmark and fallback.** Step 1 (the slice for 0025): B/W and colour-count trace with the Trace tool, top-middle settings, presets, preview, node counter, and the metric harness from section 5 against Inkscape. Build the contour and fit stages ourselves. Step 2 (later): shared-border gap-free colour layers, centerline, primitive recognition. Run VTracer once through the metric harness (as a dev tool, not a shipped dependency) to see whether it already beats Inkscape; if it does, the architect can reconsider B for the colour path. This keeps the MVP honest: one machine family, one clean B/W silhouette flow first.

**Re-check after the licence correction and the VectorCraft finding (2026-10-14).** Does the recommendation change? **No, but its reasons shift and the first task changes.**
- Licence: an earlier draft of this document treated a potrace port as a licence risk. That was wrong: GPL-2.0-or-later is compatible, so A is a legitimate choice. A still loses on quality (binary input, no sub-pixel, no shared borders), not on licence. Reason for not choosing A: changed. Result: unchanged.
- B (VTracer): unchanged, MIT/Apache, benchmark and fallback.
- VectorCraft `trace` is a new data point: an independent Rust implementation that has already chosen the same architecture as C (sub-pixel, junction-cut shared edges, kurbo). That lowers the design risk of C. It does not remove the work: its quality is unmeasured, and it is tied to their types. New step 0 before building: run VectorCraft and VTracer through the metric harness (section 5) on the 12 test images, with Inkscape as baseline. Then decide: (i) C from scratch using their stage order as a checklist, (ii) fork selected files of VectorCraft `trace` (MIT/Apache, notices kept) if it measures well, (iii) B for colour. The step costs one to two days of harness work that C needs anyway.
- Honest caveat: if step 0 shows VectorCraft beating Inkscape on the targets, "own pipeline from scratch" gets harder to justify, and (ii) becomes the likely path. That is a change to the how, not to the approach (sub-pixel, shared borders, kurbo, mm tolerances).

## 9. Questions for the customer

**Frage 1 (VEC-NAME): answered.** "VectorCraft" is `storytold/vectorcraft` (section 3.1). PowerTRACE, Vectorizer.ai and Vector Magic stay as benchmarks; Affinity has no trace.

**Frage 2 (VEC-APPR): Which approach?** (revised 2026-10-14 after the licence correction and VectorCraft)
Options: A (port potrace; licence is no blocker, quality is), B (VTracer as dependency), C (own pipeline), C with a step 0 that measures VectorCraft `trace` and VTracer first and may fork parts of VectorCraft (MIT/Apache).
Recommendation: C with step 0. Default: that one; the architect writes the ADR for the new crate (or for vendored code).

**Frage 3 (VEC-SCOPE): What is in the first slice of 0025?**
Options: (a) B/W only plus presets; (b) B/W plus colour count (R-VEC-001 as written); (c) also gap-free colour layers and centerline.
Recommendation: (b), with gap-free layers and centerline in a follow-up. Default: (b).

**Frage 4 (VEC-UI): answered.** The UX mockup of the Trace tool (settings in the top-middle bar) is accepted (customer, 2026-10-14).

**Frage 5 (VEC-UNITS): Units for the despeckle and tolerance settings?**
Options: (a) mm, using the image's physical size (needs dpi from 0058; ask when missing); (b) pixels like Inkscape and Illustrator; (c) mm with a pixel hint next to it.
Recommendation: (c). Default: (c), assuming 0058 provides a size for every image.

**Frage 6 (VEC-GAPS): Colour layers: gap-free cutout or stacked?**
Options: (a) cutout (shared edges, best for cutting and vinyl); (b) stacked (best for editing, plotter colours); (c) the maker chooses.
Recommendation: (c), default stacked in the first slice because it is simpler, cutout in the follow-up. Default: that.

**Frage 7 (VEC-TARGET): Quality targets from section 5 acceptable?**
Options: (a) as proposed (60 % of Inkscape's nodes, no paths below 0.25 mm², at most 0.75 px deviation); (b) stricter; (c) a visual comparison by you only.
Recommendation: (a) plus your visual check on 5 of your own images. Default: (a); please send 3 to 5 images you dislike in Inkscape, as test fixtures (no copyright-sensitive ones).

**Frage 8 (VEC-SOURCE): After Apply, what happens to the source image?**
Options: (a) it stays, the traced paths sit on top; (b) the image is hidden; (c) the maker chooses in the bar.
Recommendation: (b), with a visible hide/show toggle in the Layers panel. Default: (b).

## 10. Extension points (collected, not in the first slice)

Customer, 2026-10-14 (translated): extensions have not been discussed yet and are left out for now, but ideas are collected here, because tracing should be extensible with other methods. Really cool algorithms could be developed, or an MCP connected. AI, or at least machine learning, could help a lot.

Everything here is a **Proposal** unless marked. Nothing in this section is an acceptance criterion of 0025.

### 10.1 What the seam could look like

A tracer backend is a pure function with a stable shape, matching ADR 0005 item 4 ("paths in, paths out, plus a declared parameter set"):

- **In:** an RGBA8 image, the pixel-to-mm map (size of one pixel in mm, origin, orientation; from 0058), optional selection mask or crop, the parameter values, a cancel check.
- **Out:** vector outlines: closed or open paths in mm with a fill colour or stroke, hole structure, layer order and the method (stacked/abutting), plus a small report (node count, path count, warnings).
- **Parameters:** a typed schema (name, type, unit, range, default, step, group, "advanced" flag, preset values). The top bar of the Trace tool renders it, as ADR 0005 item 8 requires ("plugins declare parameters; the application renders the controls"). So the built-in tracer uses the same schema as an extension would; this also keeps the accepted mockup valid if a second backend appears.
- **Capabilities:** a backend declares what it supports (colour, centerline, needs network, deterministic yes/no), so the tool can hide controls that do not apply.
- **Preview:** the same call on a downscaled image; results are compared by the metric harness (section 5), whichever backend produced them.

### 10.2 Who could plug in

| Source | Fit | Constraints |
|---|---|---|
| Built-in algorithms | The first tracer, and later variants (centerline, logo mode, VTracer-style colour). | Core crate, wasm32, deterministic. |
| Plugin per [ADR 0005](../../docs/adr/0005-extension-and-plugin-model.md) | Wasm component, Rust SDK, paths in/out, schema rendered by the app. Natural fit for algorithm experiments by the customer. | Sandbox has no network or filesystem at MVP; ADR 0005 is `0.x` and the host ships with the first machine family. A plugin receives an image, which the ADR's "document subset in, document subset out" wording does not yet cover. The WIT interface needs an image type (architect). |
| MCP server as remote tracer | A server (local or remote) that takes an image and returns an SVG or paths. The tool would be an MCP client. | Needs network access, which plugins do not have (ADR 0005 item 7: connectors are not plugins). Data leaves the machine when remote (see 10.4). VectorCraft ships an MCP server the other way round (agents drive the editor); that is a different direction from "tracer as a service". |
| Local ML model | A model run on the machine (ONNX via a Rust runtime, or a wasm-friendly runtime) as a pre-processing or tracing step. | Model size (tens to hundreds of MB), distribution, GPU or CPU time, browser limits, model licence (see 10.5). |
| External command (ADR 0005 item 6) | Call a foreign tool that reads a file and writes SVG (potrace CLI, Inkscape, Vectorizer client). | A separate "interop" story; not a tracer plugin. Offline-capable. |

### 10.3 Where ML or AI would help most (ranked by value for makers, Proposal)

1. **Denoise and upscale before tracing** (JPEG ringing, low-res logos, scans). A learned upscaler removes staircase better than bicubic. Fits as a pre-processing stage, output is still an image, so the deterministic tracer remains in charge. Smallest risk, clearest gain.
2. **Foreground/background segmentation for silhouettes.** A user-facing "Remove background" before a laser silhouette; photos of a pet or a tool traced to a clean outline. High value for cutting. The model output (mask) feeds a normal threshold trace.
3. **Classification: line art vs logo vs photo vs pencil sketch.** A small classifier picks a preset or sets the initial parameters (Illustrator and Vector Magic auto-detect settings). Cheap and small; a rule-based heuristic (colour count, edge histogram) comes first and may be enough.
4. **Node-budget-aware curve fitting.** A search or learned fit that minimises nodes under a deviation bound. This is classic optimisation first (section 4, items 5, 6 and 9), ML only if that falls short.
5. **Shape recognition into primitives** (circles, rectangles, rounded rectangles, text). Primitives are classic geometry (VectorCraft's `shapes.rs` does it without ML). Text recognition to real text (OCR plus font match) is a bigger, separate feature (font classification is already a core concern per CLAUDE.md §6).
6. **End-to-end neural vectorisation** (image to paths). Research quality varies, output is hard to bound by a tolerance, usually not reproducible. Lowest priority for makers who need cut-ready geometry.

Rule for any ML stage: the final geometry still comes through the tolerance-bounded, mm-based fitting stage, so node count, deviation and closed-ness stay measurable by the section 5 metrics.

### 10.4 Privacy and offline

- **Default is offline.** Every built-in step and every default extension works without network.
- **Image leaves the machine = customer rule: ask first** (CLAUDE.md §3, data leaving the machine). A remote MCP tracer or a cloud ML service needs an explicit, per-extension consent shown before the first upload, and the fact that a remote step ran shown in the result. Not a global "allow all".
- A local MCP server on the same machine is not "leaving the machine", but the app should say where it runs; a remote URL looks like a local one to a user.
- No telemetry about traced images, ever, without a separate customer decision.
- Browser build: a remote call is possible there, but the same consent applies.

### 10.5 Reproducibility, licensing, performance

- **Determinism.** The built-in tracer must give the same output for the same input on host and wasm (section 7 item 8; compare with a tolerance). ML output may differ between runtimes, GPUs and model versions. Proposal: an ML or remote step is marked "non-deterministic"; the document stores the *result* (paths), never the recipe to re-run it, so a project opens identically later; golden tests use fixed model files and a tolerance, or test only the contract (schema, units, closed paths) and not the geometry.
- **Model licences.** Weights are not code. Check each: Apache-2.0 and MIT are fine; CC-BY-NC, "research only" and use-restricted (OpenRAIL-style) licences conflict with ADR 0006's "no paid, everything else deliberately free" spirit and with redistribution. Training data origin is a second question (Uncertain, per model). Shipping a model inside the app needs its own licence record, like fonts and assets in ADR 0006 item 5.
- **Performance.** Customer-confirmed budget: 2 to 4 s for 2000 x 2000 px is fine. An ML pre-step eats into that: a small CPU model on 4 MP may take seconds alone (Uncertain, not measured). Extensions run on a background worker with cancel; a backend declares an expected cost class so the UI can show progress instead of a frozen bar. The live preview uses the built-in tracer only unless the extension says it is fast enough.
- **Browser.** A wasm plugin has no GPU and no threads; a large model is unrealistic there. Extensions may be desktop-only or export-only, as with machine drivers (CLAUDE.md §6).

### 10.6 The trait rule (CLAUDE.md §5: no trait without a second implementation)

The first slice has exactly one tracer. So **no trait, registry or plugin type for tracers is written in 0025.** The seam is a design note: the tracer is one pure function `trace(image, pixel_map, params, cancel) -> result`, with a parameter schema as plain data that the top bar renders, which is already needed for the UI. When a second implementation exists, the trait (or enum) is extracted by a refactoring in the same PR as that second implementation.

Candidate second implementations, in order of likelihood:
1. **VTracer** (MIT/Apache), as an alternate backend if step 0 shows it wins on colour; first used as a benchmark in the metric harness (dev tool, no shipped dependency).
2. **A second built-in mode** with a different algorithm (centerline, logo mode), which would count if it has its own entry point.
3. **A test double** for the Trace tool UI (a fake tracer that returns a fixed result), which the rule also allows if actually used by UI tests. Only if the UI tests need it.
4. The first real *extension*: a plugin (ADR 0005 host) or an MCP client, after the host story exists.

### 10.7 Questions for the customer (extension points)

**Frage 1 (EXT-SEAM): How much of the extension seam do we build in 0025?**
Options: (a) nothing; this section is a design note, the tracer is one function with a parameter schema, which the UI needs anyway; (b) also write the WIT image type into the ADR 0005 interface now; (c) a tracer trait now, with the built-in tracer as the only implementation.
Recommendation: (a). (c) breaks the rule in CLAUDE.md §5 and (b) belongs to the plugin-host story. Default: (a).

**Frage 2 (EXT-FIRST): Which extension would you want first?**
Options: (a) a local ML pre-step (denoise/upscale or background removal); (b) an MCP tracer; (c) a plugin with your own algorithm; (d) VTracer as alternate backend; (e) none yet.
Recommendation: (d) as benchmark first, then (c) once the plugin host exists, because it needs no network or model. Default: (e); the order above in the research stays, no story is written.

**Frage 3 (EXT-PRIVACY): May a remote tracer (MCP or cloud ML) exist at all?**
Options: (a) never in the product, only local; (b) yes, per extension with a consent dialog before the first upload and a visible mark; (c) yes, with a single global setting.
Recommendation: (b), if remote ever comes. Default: (a) until you decide; no story proposes remote.

**Frage 4 (EXT-ML): Should ML be an extension only, or may the product ship its own model?**
Options: (a) extension only; the app ships no model; (b) the app ships one small model for background removal or classification, licence permitting; (c) decide later per model.
Recommendation: (c), and when it comes, 10.3 items 1 and 2 first. Default: (a) for now.

**Frage 5 (EXT-DETERM): May ML steps make results non-reproducible?**
Options: (a) yes, marked non-deterministic, the result (paths) is stored, never re-run; (b) no, only deterministic steps are allowed; (c) deterministic by default, ML allowed with an explicit mark.
Recommendation: (c). Default: (c).

**Frage 6 (EXT-VCRAFT): May we fork parts of VectorCraft's tracer (MIT/Apache) after measuring?**
Options: (a) yes, after step 0 and an ADR, with notices kept; (b) idea source and benchmark only, we write everything ourselves; (c) depend on it as a crate.
Recommendation: (a) only if it beats Inkscape in the harness; otherwise (b). (c) not recommended: alpha, internal types, weekly API changes. Default: (b) until step 0 has numbers.
