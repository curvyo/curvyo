# Research for 0025 raster-trace

Author: product-owner. Date: 2026-10-14. Status: input for the customer and the UX engineer; nothing here is accepted.

Customer request (2026-10-14): 0025 needs more thought. A new tool, with its settings in the top-middle area (not a property, not the right side). Start with the vectorisation functions Inkscape has, but check Illustrator, "VectorCraft" and Affinity. Inkscape's results should be beaten: smoother, fewer nodes, fewer wobbly lines, fewer micro paths. Bitmap import comes first (spec 0058).

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

**Which "VectorCraft" did the customer mean?** I could not find a Corel product with that name. Candidates found (searches today):
1. A Figma plugin "VectorCraft" (AI credits, licence key). [Gumroad](https://merahburam.gumroad.com/l/vectorcraft-starter-package).
2. An open-source vector editor "VectorCraft" in pure Rust, which also runs in WebAssembly, with an image-tracing feature. [SourceForge mirror](https://sourceforge.net/projects/vectorcraft.mirror/), [FileCR listing](https://filecr.com/windows/vectorcraft/) (snippet cut off; Uncertain).
3. Products with similar jobs that the customer may have meant: **CorelDRAW PowerTRACE** (Outline, Centerline, Quick Trace, preview, an AI-assisted trace in the 2025 suite, a redesigned workflow in the 2026 update) ([Corel tutorial](https://learn.corel.com/tutorials/convert-images-to-vector-files-quickly-and-easily-with-powertrace/), [Corel AI trace](https://www.coreldraw.com/en/pages/items/1600148.html), [itechguides review](https://www.itechguides.com/?p=978757)); **Vectorizer.ai** and **Vector Magic**.

**What I took:** PowerTRACE (a mature, maker-relevant trace with centerline), Vectorizer.ai and Vector Magic as quality benchmarks. Candidate 2 (the Rust editor) might be worth a look for code ideas, but I did not review its code or licence. **Frage 1 asks the customer to name the product.**

**Vectorizer.ai** ([site](https://vectorizer.ai/)). Vendor claims only: sub-pixel boundary placement from anti-aliasing values, neighbouring shapes kept aligned (gap-free), fits circles, ellipses, rounded rectangles and stars as true shapes, arcs plus quadratic and cubic Béziers, corner control, palette size, symmetry, SVG/EPS/DXF/PDF output, 3 MP and 30 MB limit, pay per use. The service is closed and cloud-based; it is a benchmark, not something we can use (data leaves the machine, CLAUDE.md §3).

**Vector Magic** ([Capterra](https://www.capterra.com/p/178297/Vector-Magic/)). Auto-detects settings, user adjusts image type, detail and colours; desktop and online; about 5 USD per month (Capterra, Uncertain, conflicting trial info). Reviewers use it for laser cutting. Closed.

**Affinity.** Most sources say Affinity Designer 2 has **no automatic trace**; tracing is manual with the Pen tool, and DrawPlus AutoTrace was not carried over ([Logos By Nick](https://logosbynick.com/vector-image-tracing-with-affinity-designer/), [Xquissive](https://www.xquissive.com/how-to-trace-an-image-with-affinity/)). One guide mentions a "Vectorize Bitmap" option; I found no official source (Uncertain). I did not reach the official Affinity help; the Canva-era Affinity app (2025/2026) is not checked. Conclusion for us: Affinity gives no pattern to copy. Treat as "no".

**CorelDRAW PowerTRACE** (taken as the third reference). Modes: Outline (filled shapes), Centerline (strokes for line art, which matters for plotters and engraving), Quick Trace; live preview before/after with colour count and node/object count shown. Fact (Corel tutorials); the node/object count display is from memory.

**What we do better (Proposal).** Show node and path counts live; sizes in mm instead of pixels; cutting presets; a stacked and a gap-free (cutout) result as a choice; trace happens in a tool next to the canvas.

## 4. Libraries and algorithms for a core crate

Hard rules: `*-core` builds for wasm32, no I/O, no threads, no `unsafe` (CLAUDE.md §5, §6). Licence is AGPL-3.0-or-later ([ADR 0006](../../docs/adr/0006-license.md)), which can include GPL code, but the allow-list in the ADR and `cargo deny` still decide per dependency.

| Option | Licence | wasm | Notes |
|---|---|---|---|
| **potrace** (C) | GPL-2.0-or-later (Fact, see section 1) | not as C | Quality reference. "or later" allows GPL-3, so it is AGPL-compatible. We cannot link C into a core crate. |
| Rust or JS ports of potrace | GPL-style (Fact for the Python `potracer` and the npm `potrace`, [licenses.dev](https://licenses.dev/npm/potrace/2.1.8), [PyPI](https://pypi.org/project/potracer/0.0.1)) | possible | I found **no Rust crate port** in today's search (Uncertain; check crates.io). A port written by us is a derivative work of GPL code. |
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

**CPU budget (Proposal; not measured, estimates from memory of VTracer and potrace performance; Uncertain).**
- 2000 x 2000 px (4 MP), B/W, no upscale: target at most 0.5 s on a 2020 laptop, single thread. Marching squares and fitting are linear; the fit dominates and scales with node count.
- Colour, 8 colours: target at most 2 s. Clustering is the cost.
- Upscale 2x of 4 MP is 16 MP of work: target at most 3 s, so it is off by default above 1 MP.
- Wasm is typically 1.5 to 3 times slower than native; threads are not available in the core. The browser needs a Web Worker (platform side).
- **Live preview needs debounce and a worker.** On the desktop, run the trace on a background thread (platform crate, not core), debounce 150 ms after the last slider move, cancel stale jobs, and preview on a downscaled copy (longest side 1000 px) while a slider is dragged; run the full-resolution trace on release and on Apply. The core API takes a cancel check callback or works in chunks, so a stale job stops. Accepting a cancel parameter is an architect question.

## 7. Licensing and dependency risks (for the architect)

1. **Potrace is GPL-2.0-or-later.** AGPL-3.0-or-later (ADR 0006) can include it, but a port makes our tracer a derivative work and the "or later" check must pass `cargo deny`. We also lose the option to relicense or dual-license the core (ADR 0006 mentions the customer as sole copyright holder). Recommend not porting potrace; write our own pipeline (Candidate B/C).
2. **VTracer/visioncortex is MIT or Apache-2.0**: compatible with the ADR 0006 allow-list. Check: transitive dependencies, `unsafe` use (core crates forbid it only in our own code; `cargo deny` plus the ADR rule on dependencies decide), wasm32 build in CI, crate maintenance (last release seen 2025-10-17, Uncertain), API stability.
3. **A new crate needs an ADR** (CLAUDE.md §5). `trace-core` is a new crate. Alternative: a module in an existing geometry crate. Architect decides.
4. **Image decoding is not core** (0058 owns it). Tracing takes a plain pixel buffer.
5. **Inkscape/potrace output as test baseline:** running a GPL tool offline to produce comparison numbers is fine; do not commit its output as fixtures without checking (GPL output is generally not covered by the tool's licence, but this is a legal question, Uncertain).
6. **Vectorizer.ai, Vector Magic:** closed, cloud. Never used as dependency; only as manual benchmark with our own images. Uploading customer images to them breaks the "data leaves the machine" rule.
7. **Photos and copyright:** the tool traces any image. A one-line note in the help is enough; no checking (Proposal).
8. **Determinism:** the same input and settings must give the same output (no floating-point differences between host and wasm). Needed for golden-file tests (CLAUDE.md §5). Risk: wasm and host floating-point rounding; compare with a tolerance.

## 8. Candidate approaches

**A. Port potrace.** Best known quality baseline, same engine as Inkscape. Needs a GPL port, binary only; no colour or sub-pixel; this is what the customer is "not 100 % satisfied" with. Cost: medium. Licence risk: high. Does not beat Inkscape. **Not recommended.**

**B. Adopt VTracer/visioncortex as a dependency, add pre- and post-processing.** Fastest to a working colour trace (days to a first demo). MIT/Apache. Already stacked and cutout, speckle filter, `simplify`. Weakness: its curve fitting is its own and its output is not smoother by nature; pixel-based region boundaries, so wobble must be fixed by us in a post-pass (re-fit with kurbo, merge). Cost: low to medium. Risk: depends on a third-party crate's maturity and `unsafe`. Quality unknown until measured (Uncertain).

**C. Own pipeline: sub-pixel contours, shared borders, kurbo Schneider fit, mm-based tolerances, node budget.** Best chance of beating Inkscape on every metric in section 5 and the closest to the customer's words (smoother, fewer nodes, no micro paths). Fully ours, AGPL, wasm-clean, no new dependency beyond kurbo. Cost: highest (B/W first, colour with shared borders second). Risk: effort and tuning time.

**Recommendation (Proposal): C in two steps, with B as a measured benchmark and fallback.** Step 1 (the slice for 0025): B/W and colour-count trace with the Trace tool, top-middle settings, presets, preview, node counter, and the metric harness from section 5 against Inkscape. Build the contour and fit stages ourselves. Step 2 (later): shared-border gap-free colour layers, centerline, primitive recognition. Run VTracer once through the metric harness (as a dev tool, not a shipped dependency) to see whether it already beats Inkscape; if it does, the architect can reconsider B for the colour path. This keeps the MVP honest: one machine family, one clean B/W silhouette flow first.

## 9. Questions for the customer

**Frage 1 (VEC-NAME): Which product is "VectorCraft"?**
Options: (a) Corel's tool (CorelDRAW PowerTRACE, which I could not find under that name); (b) the AI Figma plugin or the Rust editor of that name; (c) Vectorizer.ai or Vector Magic; (d) you do not remember.
Recommendation: (a)/(c), taken as quality benchmarks. Default: I use PowerTRACE, Vectorizer.ai and Vector Magic as references; Affinity has no trace.

**Frage 2 (VEC-APPR): Which approach?**
Options: A (port potrace), B (VTracer as dependency), C (own pipeline), C with B as benchmark.
Recommendation: C with B as measured benchmark. Default: that one; the architect writes the ADR for the new crate.

**Frage 3 (VEC-SCOPE): What is in the first slice of 0025?**
Options: (a) B/W only plus presets; (b) B/W plus colour count (R-VEC-001 as written); (c) also gap-free colour layers and centerline.
Recommendation: (b), with gap-free layers and centerline in a follow-up. Default: (b).

**Frage 4 (VEC-UI): Settings in the top-middle bar of a new Trace tool, with an Advanced expander?**
Options: (a) as proposed in section 6; (b) a floating panel over the canvas; (c) both bar and live counter only.
Recommendation: (a), the UX engineer details it. Default: (a) and I ask `ux-engineer` for a proposal before Ready.

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
