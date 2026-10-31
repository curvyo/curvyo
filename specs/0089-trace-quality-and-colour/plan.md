# Plan for Trace quality and colour, slices A, D, C, E, M and F

Implementer, 2026-10-29, on `story/trace-geometry` (cut from `main` `faeb6f07`). Built on the
defaults of Frage 80 to 93 (order A, D, C, E, M, F; reference images synthetic plus CC0, the
customer's own images only read locally through `CURVYO_TRACE_LOCAL`, only numbers committed).
Design: `adrs.md` §11 (harness) and §1 to §8 (geometry). Slice A has no PR of its own: its
commits are the first ones of slice D's branch, and D's PR shows the table.

## Slice A: reference set and baseline

### Affected crates/modules

- **`curvyo-vectorize-core`, tests only** (no product code changes, criterion 4):
  - `tests/common/quality/` (was `quality.rs`, split per `adrs.md` §11, each module under 500
    lines): `fixtures.rs` (the 0025 set, unchanged), `metrics.rs` (the 0025 metrics, unchanged),
    `render.rs` (scenes of filled vector paths or implicit shapes to RGBA and the 4x truth),
    `degrade.rs` (capture artefacts: JPEG quality 50, scan noise, paper texture, shadow,
    lighting gradient, grey seams), `scenes.rs` and `scenes_more.rs` (the synthetic reference
    fixtures with their annotations), `shape.rs` (the slice A metrics), `topology.rs` (regions,
    components, holes, connections), `report.rs` (JSON, the +5 % guard, per-group summary),
    `real.rs` (the CC0 fixtures and the local images).
  - `tests/reference.rs`: the `#[ignore]` reference table (all four presets, three
    resolutions), the peak-memory child run, the writer for the wasm timing; one fast
    non-ignored test that checks criteria 1 and 2 on the metadata.
  - `tests/fixtures/trace/reference/`: CC0 and public-domain images, one JSON sidecar each,
    `SOURCES.md`.
  - `tests/fixtures/trace/baseline.json`: new key `reference` (per preset, per fixture and
    resolution).
  - dev-dependency `curvyo-image-core` (PNG and JPEG decoding, already named by ADR 0023 §1).
- **`frontend/scripts/trace-reference-wasm.ts`**: thread CPU time of the wasm trace per
  fixture (`process.threadCpuUsage`), merged into the baseline by the native harness.
- **`.github/workflows/ci.yml`**: the nightly `trace-quality` job runs the reference table in its
  own step with a 15 minute timeout.

### Tasks

- [x] A1. Split `tests/common/quality.rs` into `fixtures` and `metrics` without changing a number
      (0025 baseline still passes) (AC 3, `adrs.md` §11).
- [x] A2. Scene renderer: filled vector paths (even-odd) or implicit shapes, coloured layers over a
      background with straight alpha, 4 x 4 coverage, the 4x truth raster, any scale (AC 2).
- [x] A3. Capture artefacts: JPEG quality 50 (baseline-JPEG DCT quantisation with the standard
      tables and 4:2:0 chroma), scan noise and dust, paper texture, shadow, lighting gradient,
      grey seams, transparency (AC 2).
- [x] A4. The synthetic fixtures, each with group, intended use, what must be preserved, corner,
      straight-edge and smooth-curve annotations, at 1x, 0.5x and 0.25x (AC 1, 2).
- [x] A5. CC0/public-domain real fixtures with sidecars and `SOURCES.md`; the local images of
      `CURVYO_TRACE_LOCAL`, numbers only under `local.<hash>` (AC 1, Frage 82).
- [x] A6. Metrics: two-sided boundary deviation every 0.1 px (vector truth, or the 4x reference
      for implicit scenes), corner position error, straightness (deviation, segments, curvature
      flips) and false corners on smooth curves, topology lost/extra (components, holes,
      connections split or merged), self-intersections, paths and nodes, thread CPU native,
      peak memory per run (counting allocator in a child process) (AC 3).
- [x] A7. Wasm timing: the writer and `frontend/scripts/trace-reference-wasm.ts` (AC 3).
- [x] A8. Baseline run of today's tracer, all four presets, into `baseline.json`; +5 % guard on
      the geometric metrics; nightly step with timeout (AC 3, 4).
- [x] A9. "Baseline results" below with per-group numbers and proposed frozen targets (AC 4).

### Validation

`cargo nextest run -p curvyo-vectorize-core` (the metadata test, the unchanged 0025 tests);
`cargo test --release -p curvyo-vectorize-core --test reference -- --ignored --nocapture`
reproduces every committed number (the guard fails at +5 %); the 0025 `quality_table` still
passes against its committed numbers after the split.

## Slice D: intended geometry

### Affected crates/modules

`curvyo-vectorize-core`: new `contour.rs` (crack-edge walk), `corners.rs` (multi-scale turn,
total-least-squares crossing, blunt-tip runs merged), `smooth.rs` (binomial, ends pinned),
`line.rs` (TLS lines, straight runs), `fit.rs` (Schneider cubics, measured two-way check),
`guard.rs` (crossings and near touches), `outline.rs` (one chain to one outline, replaces the
visioncortex wrapper); `params.rs` (`Geometry`, Smooth → smoothing, new Tolerance px, Corners →
corner cosine); `trace.rs` (stage order, guard loop, `Diagnostics`); `reduce.rs` and the
`visioncortex` dependency removed. `curvyo-ui-core`, `curvyo-editor-wasm`: Tolerance in the
settings JSON, presets and schema. Harness: `shape.rs` ignores zero-length flattening pieces in
the self-intersection count.

### Tasks

- [x] D1. Crack-edge contour walk, saddles 4-connected (AC 10, 11, 13a).
- [x] D2. Trial D1: candidate (b), multi-scale turn (2/4/8 px, two of three), apex peak, side
      lines by TLS, one side short: the corner on the other side's line, blunt tips merged
      (AC 5, 7, 11). Candidates (a) and (c) not built: (b) met criterion 5 at 1x.
- [x] D3. Input smoothing with pinned span ends; small loops capped (AC 8).
- [x] D4. Trial D2 candidate (a): chord line per span, straight runs inside spans, Schneider
      cubics with split, measured check (AC 6, 12). (b) DP and the kurbo comparison not built.
- [x] D5a. Round 4: room by construction (`clearance.rs`): every segment of every walked
      contour measures its distance to the nearest other boundary (row-bucketed, no hash map);
      the freely smoothed contours measure theirs once more; a fit may use half that room
      less the guard's gap, a point whose room collapsed in smoothing (a thin feature closing)
      is pulled back; the Specks band and the per-corner vertex fall-back run inside the guard
      (AC 10, 12, 19 of 0025).
- [x] D5. Guard: per-part fall-back to the contour as walked, then corners at their vertices,
      then the pixel border; outlines a fit shrank below Specks take the pixel border (AC 10).
- [x] D6. Parameter decoupling, Frage 85 B: Smooth → smoothing only, Tolerance (px) its own
      field, Corners → corner cosine (AC 8). Node limit was already gone.
- [ ] D7. Start independence on whole traces (AC 11): corners tested under shift and turn; a whole-trace test is open.
- [ ] D8. Frozen targets as assertions: round 4 meets 9, 6, 5 and 7 at 1x and 10 on all but 3 runs (see "Slice D results").
- [x] D9. Tolerance field in the bar, the pill as the finish button, no label (Frage 84, 85 B);
      docs; goldens regenerated (new tracer); native = wasm exact (AC 13a).

### Slice D results (round 5, 2026-10-30: crossings 1, extras reported, wobble open)

Changes: the guard treats a self-touch closer than 0.03 px as a crossing (two cubics at a kink
crossed under the harness's coarser flattening); the harness's crossing test ignores an end
point on the other segment within 1e-9 px (a shared pixel corner after the round trip through
millimetres); a corner that falls back goes to its inset, never onto a vertex another outline
passes; a fit may use the whole room the smoothed contours leave (the guard catches the rare
pair that both use it); the smoothing clamp of a held point is 3/4 of its room whatever the
Tolerance; shapes under 1.5 Specks areas are fitted at 0.25 px so the Specks band does not
depend on the Tolerance; the band's form for larger shapes is the pixel border simplified
within half a pixel; `report.rs` guards lost topology only (extras reported, 9a); 0025
criterion 63 as superseded is checked in `reference_table` (Silhouette ≤ 0.8 x Logo as the
geometric mean, never more per image).

| Target (Logo / Line art) | Baseline | Round 4 | Round 5 | Met |
|---|---|---|---|---|
| 9 nodes/100 mm geo mean ≤ 75 % (17.3) | 23.09 | 16.66 | 16.40 (71 %) | yes |
| 5 corners ≤ 0.5 px, 1x / 0.5x | 4.11 / 4.68 | 0.14 / 0.58 | 0.14 / 0.58 | 1x yes |
| 6 straight edges | 13 segs, 0.80 px, 21 flips | met | met | yes |
| 7 bow/arc ≤ 0.5, false corners | 1.96, 4 | 0.43 / 0.69 at 0.5x | 0.43 / 0.69 at 0.5x, 0 | 1x yes |
| 10 self-intersections | up to 6 542 | 4 | 1 (Engraving `photo-pencil-sketch`) | nearly |
| guard misses, all presets | - | 346 | 256 (wobble 131, Silhouette nodes 28, Engraving true shape 26) | no |
| Silhouette / Logo nodes (geo), 0025 c63 superseded | - | - | 0.69 (≤ 0.8); per image 22 of 124 above Logo | mean yes |
| pencil sketches `loewe` / `pferd` nodes | 82 / 81 | 97 / 99 | 97 / 98 | yes |
| CPU, whole set native | 3.0 s | 5.2 s | 5.1 s | reported |

Findings. (a) **Wobble** is not a fit matter: on `loewe` the Tolerance 0.5 / 0.75 / 1.0 px gives
96 / 90 / 89 nodes and 57 / 51 / 48 flips per 100 mm, on `precision` 52 / 52 / 52 and 29 / 29 /
29 (limit 20): the flips are inflections of the smoothed contour that any faithful fit keeps;
the baseline's lower counts came with crossings (540 on `loewe`) and shrunk shapes. The +25 %
cap is not reachable without changing the contour; proposal: measure wobble against the true
shape (inflections the truth does not have) where a truth exists, report it elsewhere.
(b) **Silhouette above Logo per image** (`line-webs` 24 vs 16, `diagram-boxes@0.5x` 81 vs 4):
Denoise Low erases 1 px lines and Specks 2 mm removes most of a small diagram; the fragments
that remain are all ends and corners, so the density of what is left rises although the
whole has fewer nodes (`line-webs@1x` 68 vs 56 nodes on 7 vs 9 paths). The rule "never more
per image" cannot hold where the preset changes the mask. (c) **Tolerance inversion on the
tester's line-art fixture** (12 225 / 13 124 / 13 499 at 0.5 / 1 / 2 px): the fits do fall
(4 883 / 4 869 / 4 722 segments); the rise is the Specks band, since a looser fit shrinks more
specks-sized bits (Specks 0.5 mm is 3.6 px² at 96 dpi; 410 / 447 / 458 of 1 513 outlines)
below 0.95 x Specks and each then takes its pixel border; fitting bits under 1.5 Specks areas
at 0.25 px removes the tolerance dependence of the fit but not of the count (11 935 / 12 771 /
12 873 now). A band that does not depend on the fit would need criterion 19 to measure the
pixel area, not the outline's. (d) `sketch-bowed@0.5x` 0.69 px is a half-pixel case: at 0.5x
the bow has 1 px of sagitta over 100 px, one staircase step, and the truth lies 0.5 px from the
contour the tracer is given (0.43 at 1x with two steps); the star tips at 0.5x (0.58) are not:
each side has about 15 midpoints and the crossing of the two side lines lands 0.55 px short of
the tip. (e) Topology lost at 0.25x: +1 connection on 5 runs, `text-umlauts@0.5x` 22 vs 20
components.

### Slice D results (round 4, 2026-10-29: crossings gone, nodes met, wobble and extras open)

Root causes found by measuring (diagnostic dumps per image): (1) the remaining crossings came
from the Specks band swapping a shape to its pixel border *after* the guard, and from corners
placed exactly on a vertex another outline passes; (2) the node and wobble explosion on the
pencil sketches and paper photos came from the guard's last fall-back, the contour as walked
(387 of 2 093 outlines on `loewe` carried 49 000 of 58 000 segments), reached because fits
bulged into 1 px walls and gaps. Fix: `clearance.rs` measures every contour's room (distance
to the nearest other boundary, row-bucketed, no hash map); a fit gets half the room the
smoothed contours leave each other less the guard's gap, a point whose room collapsed in
smoothing is pulled back (thin features never close), the fall-back ladder is: fit within the
walked room, walked contour simplified within half its room, walked contour, corners back to
their vertices (inset), pixel border; the Specks band runs inside the guard and keeps the
walked contour simplified before the pixel border. Tried and dropped (measured worse):
2x smoothing on textured spans (nodes +25 %, more collapses), straight-side tangents at
corners (broke 0.5x thin bars), a fit on the pixel corners for band shapes (conflicts).

| Target (Logo / Line art) | Baseline | Round 3 | Round 4 | Met |
|---|---|---|---|---|
| 9 nodes/100 mm geo mean ≤ 75 % (17.3) | 23.09 | 17.19 | 16.66 (72 %) | yes |
| 5 corners ≤ 0.5 px, 1x / 0.5x | 4.11 / 4.68 | 0.14 / 0.58 | 0.14 / 0.58 (star tip) | 1x yes, 0.5x no |
| 6 straight edges (1x and 0.5x) | 13 segs, 0.80 px, 21 flips | ≤ 2, ≤ 0.35, 0 | ≤ 2, ≤ 0.31, 0 | yes |
| 7 arcs and bow ≤ 0.5 px, no false corner | 1.96 px, 4 | 0.69 (bow 0.5x) | 0.43 at 1x, 0.69 at 0.5x, 0 | 1x yes, 0.5x no |
| 10 self-intersections 0 | up to 6 542 | 2 to 14 on 12 images | 4 in 496 runs (3 images, Photo/Engraving) | nearly |
| guard misses per image (all presets) | - | 429 | 346 (wobble 143, Silhouette nodes 32, extras 65) | no |
| pencil sketches `loewe` / `pferd`, Logo nodes/100 mm | 82 / 81 | 219 / 221 | 97 / 99 (limit 103 / 102) | yes |
| phone photos, Logo nodes | 38 to 84 | up to 107 | 37 to 105 (one 59 vs 58) | nearly |
| CPU, whole set native | 3.0 s | - | 5.2 s (clearance pass) | reported |

Other presets, nodes geo: Photo 42.5 → 31.0 (wobble 8.0 → 11.7), Silhouette 10.8 → 11.1,
Engraving 52.3 → 27.1 (wobble 16.2 → 8.1). Open: (a) wobble per image is the largest miss
(143 runs; the fits reproduce every inflection of the smoothed contour, pixel borders of the
band add staircases; the baseline's lower wobble on sketches came with hundreds of crossings);
(b) Silhouette on thin line art is denser than the baseline because its 1.5 px tolerance is
cut to half the clearance on 2 to 3 px lines, where the baseline crossed itself (its Silhouette
runs on `bird-head`, `book-butterfly`, `scan-bird-head` had 38 to 57 crossings); (c)
components/holes "extra" against the truth are Specks-sized blobs the band keeps as pixel
borders while the baseline's fit shrank them away (criterion 9a names only *lost* as hard);
(d) 0.5x star tip 0.58 px and bow 0.69 px; (e) a few lost connections at 0.25x (+1 each). 0025
tester tests red: c06, c23 (superseded), c64 (superseded by the 0089 note), c63: with Logo at
the geometric minimum on the quality set (ring 3.7, text 6.9 nodes/100 mm) only `blobs`
(10.9) counts, and Silhouette cannot be 40 % lighter than a near-minimal fit (ratio 0.77).

### Slice D results (round 3, 2026-10-29, local, not pushed: frozen targets still miss)

Changes after the architect's analysis: foreground 8-connected (Frage 96 A default), fall-back
per span (half tolerance, simplified smoothed contour, walked contour; then corners just inside
their vertices; then the pixel border), a Specks band (a fit within the tolerance of the Specks
size keeps the shape as its pixel border, so criterion 19 holds), corners only where the
coarsest scale agrees and the next finer scale turns as much, thin-stroke ends not merged into
tips, Photo's smoothing 1.5 px (Smooth now 0.03 px per step), Upscale smooths in its own pixels.

**Baseline re-measured** on the visioncortex tracer of `f7d19dc9` (slice A, rebased) with the
corrected harness: 8-connected Specks reference, zero-length flattening pieces ignored in the
self-intersection count, axis-aligned half-pixel features measured apart ("reported only"),
straight edges measured only on trace running their way, and the customer's 31 images
(`tests/assets/trace-references/`, sidecars in `reference/customer/`). The guard is the one
the PO froze (report.rs `misses`, `target_misses`).

| Target (Logo / Line art) | Baseline | Now | Met |
|---|---|---|---|
| 9 nodes/100 mm geo mean ≤ 75 % | 23.09 (with the customer images) | 17.19 (74.4 %) | yes |
| 5 corners ≤ 0.5 px, geo-corners 1x / 0.5x | 4.11 / 4.68 | 0.14 / 0.58 (star tip) | 1x yes, 0.5x no |
| 6 straight edges (4 clean fixtures, 1x and 0.5x) | 13 segs, 0.80 px, 21 flips | ≤ 2, ≤ 0.35, 0 | yes |
| 7 arcs and bow ≤ 0.5 px, no false corner | 1.96 px, 4 | 0.69 px (bow at 0.5x), 0 | no |
| 10 self-intersections 0 | up to 540 | synthetic 0; 2 to 14 on 12 customer images | no |
| guard per image (all presets) | - | Logo 114, Photo 133, Silhouette 60, Engraving 118 misses | no |

Other presets, nodes geo: Photo 42.5 → 31.2 (wobble 8.0 → 11.7), Silhouette 10.8 → 11.4,
Engraving 52.3 → 28.1. Largest misses: pencil sketches (`loewe`, `pferd`: 2.7x nodes, 4x
wobble), wobble on phone photos, topology extras at 0.25x. 0025 tester tests red: c06, c23
(superseded), c63 "the set has detailed images" (Logo now below 10 nodes/100 mm on every image
of its set; line-art Silhouette 9.4 vs Logo 7.5 per 100 mm).

## Slice C: selection methods

Implementer, 2026-10-30, on `story/trace-selection` (cut from `main` after #223). Design:
`adrs.md` §2 (`Selection`), §3 (alpha semantics, trial C1), §8 (determinism); UX: `ux-notes.md`
§5, §6, §8 (Method, colour group, pipette, Drop border shapes). Customer answers in force:
Frage 87 A (transparent never content), 88 A (Drop border shapes), 89 A (pipette armed in Pick
colours, Shift+click adds, Esc ends; one pick in Remove background), and the finish button
feedback of 2026-10-30 (always blue, "Trace", progress inside, no flicker).

### Affected crates/modules

- `curvyo-vectorize-core`: `params.rs` (`Rgb8`, `Swatches`, `Selection`, the colour tolerance
  in `TraceParams`; `TraceParams` stays `Copy`, swatches are a fixed array of 8); new
  `select.rs` (score and alpha fields per method, the membership cut `α·s ≥ ½`,
  `suggest_background`, `sample_colour`); `raster.rs` (luma of the stored RGB, no compositing;
  Otsu over pixels with α ≥ 128 only); `trace.rs` (`mask` reads the fields of `select.rs`);
  `schema.rs` (the Method choice and Colour tolerance; the toggle's label "Drop border shapes").
- `curvyo-ui-core`: re-exports; `curvyo-editor-wasm`: settings JSON (`method`, `background`,
  `swatches`, `colour_tolerance`), `trace_suggest_background`, the session's Trace pipette
  (armed state, hover sample, picks for the host, the eyedropper cursor).
- `frontend`: Method menu, colour group (background chip/hex/Auto/pipette; swatches/pipette),
  Colour tolerance field, the chip of the Style eyedropper reused, Esc order, "Drop border
  shapes", the finish button (always blue, "Trace", progress inside after 400 ms, kept at least
  300 ms).
- Harness: `Layer::content` (layers that are not content are not truth), `Built::selection`,
  the slice C fixtures, trial C1 (`tests/selection.rs`).

### Tasks

- [x] C1. Trial C1 in test code: masks of sRGB, linear-RGB and Oklab distance and of the
      two-colour mixing model (sRGB, linear, Oklab) traced through Brightness and measured;
      choose the simplest that passes (AC 16, 17, 18, 20).
- [x] C2. Alpha semantics: luma of the stored RGB, membership `α/255 · s`, mask at ½, Invert on
      `s` only, Otsu over opaque pixels (AC 15; 0025 criteria 24 and 44 as changed by Frage 87 A).
- [x] C3. `Selection` and `select.rs`: Brightness (bit-identical on opaque images), Remove
      background (distance to the background colour), Pick colours (mixing model, union as the
      per-pixel maximum, up to 8 swatches), `suggest_background`, `sample_colour` (AC 14, 16 to
      19).
- [x] C4. Slice C fixtures with truth (pale colours on white, transparency, grey fringes,
      coloured paper `logo-blue`, picked-colour logo) in the reference set; acceptance tests per
      criterion; selection CPU at 2000 x 2000 px (AC 16 to 20).
- [x] C5. Settings JSON, `trace_suggest_background`, the session's Trace pipette (AC 17, 20).
- [x] C6. Bar: Method menu, Colour tolerance, colour group, pipette with chip, Esc order, Drop
      border shapes; method and colour tolerance remembered, swatches per session (AC 14, 17,
      20; Frage 88 A, 89 A).
- [x] C7. Finish button (customer 2026-10-30): always blue, "Trace", progress inside after
      400 ms and kept at least 300 ms; Stopped/Failed keep Retry; spec 0089 22a, 0025 and
      `ux-notes.md` reworded.
- [x] C8. Frozen colour distances in the spec's table; results below; full gate; reference and
      quality tables in release; Chromium bar tests incl. 800 x 600.

### Validation

`cargo nextest run -p curvyo-vectorize-core` (selection unit tests, `tests/selection.rs` with
one test per criterion 14 to 20); `reference_table` and `quality_table` in release (the guard
of 9a on every old fixture; the slice C fixtures measured with their method); the bar in
Chromium (`CURVYO_E2E_BROWSER=/usr/bin/chromium`), 800 x 600 included.

### Slice C results (2026-10-30)

**Trial C1** (`tests/trial_c1.rs`, 1x, each candidate's mask traced through Brightness; "in" is
the largest gate distance of a picked colour's blends with the background from 25 % coverage
(Point from 50 %), "out" the smallest of a colour that must stay out (its blends from 50 %);
margin = out / in; deviation in px against the truth at the gate √(in · out)):

| Candidate | Pale + near-grey (17): margin, dev | Black AA edge (18): dev | Grey fringe: dev | Touching picks (19): paths | ms per 4 Mpx |
|---|---|---|---|---|---|
| sRGB distance | 0.2 (fails), 215 | 200 (fails) | 1.01 | 1 | 37 |
| linear distance | 0.2 (fails), 215 | 0.96 | 1.18 | 1 | 436 |
| Oklab distance | 0.2 (fails), 215 | 200 (fails) | 1.18 | 2 (fails) | 1106 |
| **sRGB mixing** | **10.2, 0.39** | **0.34** | 1.19 | **1** | **96** |
| linear mixing | 4.8, 0.41 | 1.75 | 0.19 | 1 | 617 |
| Oklab mixing | 4.3, 0.39 | 0.36 | 0.69 | 1 | 1354 |

Winner: the mixing model in stored sRGB (simplest that passes 16 to 18 and 20; renderers blend
sRGB bytes, so blends lie on the segment). Remove background uses the sRGB distance to the
background. **Frozen colour distances** (sRGB unit cube, bytes / 255): background suggestion
**0.02** (criterion 16; measured 0.000 on `logo-blue`), palette merge **0.03** (criterion 26;
#D9D9D9 to #DCD6CF is 0.043, so they stay apart). Colour tolerance t → gate `0.5 · (t/100)²`;
defaults Pick colours 15 (gate 0.011, between 0.002 in-class and 0.0195 out-of-class),
Remove background 40 (gate 0.08).

**Criteria** (`tests/selection.rs`, `select.rs` unit tests, `trace_glue` and session tests,
Chromium `acceptance_0089_c_browser.test.ts`):

| Criterion | Target | Measured | Met |
|---|---|---|---|
| 14 Brightness on opaque pictures | identical to D | goldens exact; reference table: every changed row is a picture with partial alpha (16 real images, `logo-transparent`) | yes |
| 15 alpha once, transparent never content | membership α·s ≥ ½ | 50 % black → content, 49.8 % not; Invert never takes α = 0; `logo-transparent-colour` 0.31 px, 0.36 % | yes |
| 16 Remove background on #3366CC | suggestion ≤ 0.02, area ≤ 2 % | 0.000; area 0.53 % (Brightness today 308 %), 0.57 px | yes |
| 17 pale and near-grey picked alone | ≤ 0.75 px, separated | #FFF2A8 0.00, #D9D9D9 0.41, #DCD6CF 0.34 px, one shape each | yes |
| 18 anti-aliased edge kept | ≤ 0.5 px | 0.34 px away from corners, area 0.28 % | yes |
| 19 touching picks one shape | 1 path, no inner border | 1 path, 0 samples on the border, 0.00 px | yes |
| 20 selection CPU at 2000 x 2000 px | ≤ 0.5 s native | worst case (a colour per pixel): Remove background 0.012 s, Pick 1/3/8 colours 0.030/0.062/0.256 s; wasm not measured | yes (native) |

**Reference table** (release, all presets, 1x/0.5x/0.25x, without the peak child): 284 guard
misses against 256 on main; every new one is on a picture with partial alpha (changed by design,
criterion 15), and Photo's wobble (geo) improves 11.08 → 9.43 (still above 8.00). The slice C
fixtures at Logo / Line art, 1x: `logo-blue` 0.53 % area, `logo-transparent-colour` 0.36 %,
`logo-pale-picked` 0.17 %, `logo-picked` 0.00 %, `logo-fringe` 2.05 % (the star tips lose
their red-to-grey blends: corner 1.5 px; the mixing model only knows blends toward the
background; a higher Colour tolerance takes them). `quality_table`: the same 14 wobble and
Silhouette misses as on main.

**Finish button** (customer 2026-10-30): blue 75 x 28 px "Trace"; a job at 0 ms delay never
showed its progress; at 480 ms it showed for 315 ms (held), percent and x inside the blue
button; Stopped keeps Retry. Bar at 800 x 600 (Advanced closed): panel open Brightness 252 px,
Remove background 288 px, Pick colours 288 px; closed 108, 108, 144 px; nothing past the edge.

Open: the Mask view (criterion 21) is slice E; Pick colours mixes only toward the border
colour (fringes of a second colour need a higher tolerance); the wasm time of the selection is
not measured separately.

## Slice E: integration

Implementer, 2026-10-30, on `story/trace-integration` (cut from `main` after #226). Design:
`adrs.md` §2 (`mask`, `Mask`), §6 (late answers), §7 (migration); UX: `ux-notes.md` §2 (bar
order), §4 (View with Mask), §12 (About Trace entries). Customer decisions kept: the blue
"Trace" finish button at the far right, no hint texts outside About Trace, the 800 x 600 layout;
Single colour / Multicolour (Frage 92 B) is slice M and not built here.

### Affected crates/modules

- `curvyo-vectorize-core`: `trace.rs` (`Mask`, `mask()`: the cleaned mask of the same code path
  the trace walks, at the traced resolution, with its pixel map).
- `curvyo-ui-core`: `trace_transfer.rs` (`trace_mask_job` for the worker).
- `curvyo-editor-wasm`: `trace_draw.rs` (`TraceViewMode::Mask`, cycle Overlay, Mask, Trace,
  Image; the mask drawn as a picture in place of the shown image), `trace.rs` (the kept mask),
  `image_cache.rs` and `gpu_images.rs` (the mask's working image; nearest-neighbour as soon as a
  mask pixel is larger than a screen pixel), `wasm_trace.rs` (`trace_mask_job`,
  `set_trace_mask`, the `"mask"` view).
- `frontend`: `traceText.ts` (the Mask segment, the About Trace entries, the migration of
  remembered settings), `TraceBar.tsx` (the `Binary` segment), `traceJobs.ts` and
  `traceWorker.ts` (the mask job), `useTraceMask.ts` (new: the mask jobs of the Mask view),
  `useTrace.ts`, `useTraceViewMode.ts`.
- Docs: `docs/tracing-images.md`, `docs/design-system.md`.

### Tasks

- [x] E1. `mask()` in core, test first: pixel-equal with the traced outlines (every mask pixel's
      centre lies inside the outlines exactly when it is content), after Specks and Drop border
      shapes, 2x after Upscale, transparent pixels never content (AC 21).
- [x] E2. Session: the Mask view (fourth view, Shift+\ cycles four, peek shows Image), the mask
      drawn black on white in place of the shown image, half strength when stale, its pixels equal
      to the mask handed in; wasm surface and the worker job (AC 21).
- [x] E3. Frontend: the Mask segment (`Binary`) after Overlay, the mask job whenever the Mask view
      shows (a new view, each preview step in it), dropped when its generation is old (AC 21, 23).
- [x] E4. Migration (Frage 85 B): a remembered 0025 choice without Tolerance takes its preset's
      Tolerance (not Logo's); slice-1 values, unknown keys (Node limit, a stored Ignore background)
      and a missing method read without a change of meaning; node tests per case (AC 13, 22).
- [x] E5. About Trace entries (View with Mask, Mask, Background and Auto, pipette), the user page
      and the design system; every string compared with the code by the existing test (AC 24).
- [x] E6. Chromium: the View pill's four segments in order, Shift+\ through four, the Mask view
      black where the content is and white elsewhere, the bar order (1) Preset and View, (2)
      Method and its fields, (3) Smooth and Tolerance, (4) Ignore below, expander, finish button,
      nothing before the method's field moving between methods; 800 x 600 (AC 21, 22, 22a).
- [x] E7. Full gate; `reference_table` and `quality_table` in release.

### Validation

`cargo nextest run -p curvyo-vectorize-core -p curvyo-editor-wasm` (the mask tests, the session
view tests); `npm test` (migration, strings); Chromium `acceptance_0089_e_browser.test.ts` with
`CURVYO_E2E_BROWSER=/usr/bin/chromium`; the full `CLAUDE.md` §7 gate once.

### Slice E results (2026-10-30)

| Criterion | Test | Result |
|---|---|---|
| 21 Mask view, pixel-equal | `tests/mask.rs` (every mask pixel's centre is inside the traced outlines exactly when it is content, with Specks, Drop border shapes and Upscale), `trace_mask_tests.rs` (the drawn picture equals the bits handed in, black on white, at the image's frame, pixelated, half strength when stale), Chromium `acceptance_0089_e_browser.test.ts` (both blocks black, background white, no outline; Pick colours of the red leaves the black block white; the peek shows the image) | met |
| 21 four views | Shift+\\ Overlay, Mask, Trace, Image (session and Chromium); the 0025 tester cases now expect Mask after Overlay (comment cites criterion 21) | met |
| 22, 22a bar order | Chromium: i, Preset, View, Method, its field, Smooth, Tolerance, Ignore below, expander, Trace button in reading order for every method at 1280 x 800 and 800 x 600 with the panel open and closed; the first five never move between methods; the Trace button at the far right | met |
| 23 late answers | the page's generation check drops a mask or trace of older settings (as before); `adrs.md` slice E note: the §6 stage cache is not built | met (as before) |
| 13, Frage 85 B migration | `traceText.test.ts`: a 0025 choice without Tolerance takes its preset's (Silhouette 1.5 px, Engraving 0.35 px; before: Logo's 0.5 for all), Smooth keeps its number, Node limit and Full-size keys are ignored, a stored Ignore background switches nothing on, missing method is Brightness | met |
| 24 docs | About Trace entries (View with Mask, Mask, Background, Pipette, Pick colours up to 8), `docs/tracing-images.md`, `docs/design-system.md`; the page test compares every string | met |

Gate (2026-10-30): the full `CLAUDE.md` §7 gate green (9,460 tests); `reference_table` and
`quality_table` in release report exactly the misses of the base `80439436` (the trace path is
unchanged; the goldens stay exact).

Bar heights at 800 x 600, Advanced closed (measured): panel open Brightness 252, Remove
background 288, Pick colours 288 px; closed 108, 108, 144 px: unchanged by the four-segment
View pill (it still fits beside Preset). 1280 x 800: open 72, 108, 108; closed 72, 72, 108 px.

The About Trace intro reads "The method decides which parts become shapes" (lead 2026-10-30;
0025 criterion 70 and 0089 criterion 24 updated). Open: the §6 stage cache (worker keeps its
state, queues one job) is not built (no criterion needs it; `docs/technical-debt.md`). The Mask view computes the selection
again in the worker (one extra job per preview step while it shows). No casing around the mask
on a dark document ground (no dark theme yet).

## Slice M: multicolour (adjacent)

Implementer, 2026-10-30, on `story/trace-multicolour` (cut from `main` after #227). Design:
`adrs.md` §10 (palette, label map, shared edge table, each edge fitted once with its junctions
pinned, `ColourTrace` with layers by area, recolour without refit, trial M1), §8 (basic
operations and `sqrt`, memory); UX: `ux-notes.md` §4 (Mask), §7 (Output, Colours, palette strip
and its five actions), §10 (notices), §12 (strings). Customer answers in force: Frage 81 A,
90 A (Pick colours + Multicolour: the palette is exactly the picks, Colours greyed out), 92 B
(Output, Colours and the palette pipette in Advanced).

### Affected crates/modules

- `curvyo-vectorize-core`: new `palette.rs` (flat-pixel median cut in the sRGB unit cube, merge
  closer than 0.03, fixed colours kept), `segment.rs` (label map: nearest colour or two-colour
  blend, exclusions and merges, regions below Ignore below joined), `edges.rs` (crack edges
  between labels, junction vertices, one chain per edge, the loops of each colour as edge ids),
  `colour_trace.rs` (`trace_colours`, `colour_mask`, `ColourParams`, `ColourTrace`,
  `ColourLayer`, `PaletteEntry`; each edge fitted once in its owner's loop, the guard on the edge
  spans, the loops assembled from shared segments); `outline.rs` (`Chain::pinned`: junction
  vertices as fixed corners); `guard.rs` (crossings of open parts); `trace.rs` (`masked`
  shared, `Diagnostics::joined_regions`).
- `curvyo-ui-core`: `trace_transfer.rs` (the colour job and its wire tail: palette, layers,
  joined regions; the colour mask job), `trace_plan.rs` (one filled compound path per colour,
  named by its hex, largest area first, recolours applied).
- `curvyo-editor-wasm`: `trace_glue.rs` (settings `output`, `colours`, `fixed`, `excluded`,
  `merged`), `wasm_trace.rs` (the colour jobs, the palette of the preview, recolours and
  Isolate), `session/trace*.rs` (layers in the preview, coloured Trace view, dimmed others, the
  segmented Mask view, Create paths per colour in one step).
- `frontend`: `traceText.ts` (Output, Colours, palette words, colour names, notices, About
  entries, the settings JSON), `TraceAdvanced.tsx` (Output pill, Colours field, pipette),
  new `TracePalette.tsx` (strip, detail row, Isolate, Recolour, Merge into, Exclude, Remove),
  `useTrace.ts` / new `useTracePalette.ts` (session state of the palette).
- Harness: `tests/colour.rs` (criteria 25 to 30 with label-map fixtures), `degrade.rs`
  (JPEG quality 75), the multicolour rows of the quality report.

### Tasks

- [x] M1. Trial M1 and `palette.rs`, test first: automatic count, fixed colours kept, merge at
      0.03, pale accent at 0.2 % kept, 3-colour logo with AA and JPEG 75 gives 3 colours (AC 26).
- [x] M2. `segment.rs`: every pixel one label or background, blends to the dominant colour,
      exclusions and merges, small regions joined to the neighbour with the longest border, the
      count reported (AC 27, 29).
- [x] M3. `edges.rs` and the shared fit in `colour_trace.rs`: junctions pinned, each edge fitted
      once, neighbours share reversed segments, the guard on edge spans, layers by area (AC 28,
      29).
- [x] M4. Harness: label-map fixtures (shared straight and curved edges, triple junctions,
      enclosed regions, pale accent, near-equal colours, transparent background, AA, JPEG 75
      fringes, gradient in steps); colour assignment, gaps and overlaps at 4x, shared-boundary
      identity, nodes; 8 colours at 2000 x 2000 px CPU (AC 26 to 30).
- [x] M5. ui-core and wasm: the colour job and wire, the plan (one group, one path per colour,
      one step), recolour without refit, the segmented Mask view, Isolate (AC 25, 29).
- [x] M6. Frontend: Output and Colours in Advanced, palette strip and its actions, notices,
      About entries, remembered Output and Colours; Chromium file by file (AC 25, 26, 29).
- [x] M7. Full gate; `reference_table` and `quality_table` in release; results below.

### Validation

`cargo nextest run -p curvyo-vectorize-core` (palette, segment and edge unit tests,
`tests/colour.rs` one test per criterion 26 to 30); `npm test` (words, settings JSON, palette
state); Chromium `acceptance_0089_m_browser.test.ts` file by file; the full `CLAUDE.md` §7 gate
once.

### Slice M results (2026-10-30)

**Trial M1: our palette in the sRGB unit cube; Oklab not needed.** The flat-pixel median cut
(a pixel votes only when its 3 x 3 window is within 12 bytes per channel; an automatic split
only where a box falls into two groups, between-group share of the variance at least 0.8 and the
means at least 0.03 apart) finds every fixture's colours, keeps #D9D9D9 and #DCD6CF apart (0.043)
and merges #606060 and #646260 (0.018); the 3-colour logo with anti-aliasing and JPEG 75 gives
exactly 3. Colours that form no groups (a photo, a gradient) take the default 6 steps (Proposal
for "default 6" of criterion 26). visioncortex is no longer in the workspace (removed in D) and
its colour path uses hash maps; it was not added back for the trial (the ADR's expected outcome:
ours). Blends are labelled by the two-colour model of trial C1: the nearest colour within 0.03,
else the dominant colour of the blend of two of the four nearest (or of one with the background)
when it explains the pixel with half the distance.

**Shared boundaries.** The label map is cut into edges between junction vertices (three or four
labels; a 2-label checkerboard is passed around the label that does not connect, a colour over the
background, the later palette entry over the earlier, as the region labelling connects them).
Each colour's loops are closed chains with their junctions pinned; an edge is fitted once in the
loop of its larger label (`Chain::pinned`, the slice D fit unchanged) and taken reversed by the
other, so neighbours share segments and a triple junction is one node. A fitted span that crosses
goes to the fit at half tolerance and smoothing, then to the walked contour (which never crosses).

**Criteria** (`tests/colour.rs`, unit tests of `palette.rs`, `segment.rs`, `edges.rs`,
`colour_map.rs`; ui-core and session tests; Chromium `acceptance_0089_m_browser.test.ts`):

| Criterion | Target | Measured | Met |
|---|---|---|---|
| 25 Output in Advanced, only the maker changes it | radiogroup in Advanced, app setting | Chromium: no Output in the main bar, "Single colour", "Multicolour" in Advanced; remembered (`traceMulticolour.test.ts`); F does not exist yet | yes |
| 26 palette | 2 to 16, fixed kept, merge 0.03, pale 0.2 % kept, AA + JPEG 75: 3 colours | every fixture's count; pale accent (0.2 %) kept; fixed colour kept exactly and first; count clamped 2 to 16; logo3-aa-jpeg75: 3 | yes |
| 27 one label per pixel, small regions joined and counted | no double cover; joins counted | overlap 0 at 4x; three 3 x 3 dots of a fixed colour joined (`joined_regions` 3), shown as "3 regions joined" under the counts | yes (Proposal: a region joins a colour neighbour first, the background only without one, so the content stays the mask's) |
| 28 shared boundary, triple junction one node, gaps/overlaps <= 0.01 % at 4x | identical reversed segments | 0 bad segment groups, gap 0 %, overlap 0 % on all fixtures, at Logo and at Silhouette settings; the T junction is a node of all three colours | yes |
| 29 one group, one compound path per colour, largest area at the bottom, recolour without refit, Exclude, Merge into, colour-blind use | | ui-core plan and session tests (one step, fills, hex names, outlines unchanged by a recolour); Exclude = background, Merge into = one layer of the target; swatch names hex + colour name + state words, marks are shapes | yes |
| 30 8 colours, 2000 x 2000 px, < 4 s CPU native | 4.0 s | native 0.32 s (`criterion_30_eight_colours_2000_budget`), wasm 0.63 s; the slowest real photo (`customer-photo-fox-2`, 3 Mpx, 11 colours, 40 800 paths) native 2.06 s, wasm 2.50 s (`criterion_30_wasm_manifest` + `trace-reference-wasm.ts` with `CURVYO_TRACE_WASM_DIR`, 2026-10-31) | yes |

**Fixture table** (release, `multicolour_table`, Remove background with the suggested background,
0.1 mm per px):

| Fixture | Colours (want) | Assigned | Gap | Overlap | Shared / bad | Paths | Nodes | Joined |
|---|---|---|---|---|---|---|---|---|
| adjacent-straight | 3 (3) | 1.000 | 0 % | 0 % | 6 / 0 | 3 | 13 | 0 |
| adjacent-curved | 3 (3) | 1.000 | 0 % | 0 % | 20 / 0 | 4 | 29 | 0 |
| enclosed | 3 (3) | 1.000 | 0 % | 0 % | 40 / 0 | 9 | 44 | 0 |
| pale-accent | 4 (4) | 1.000 | 0 % | 0 % | 14 / 0 | 5 | 21 | 2 |
| near-equal | 3 (3) | 1.000 | 0 % | 0 % | 10 / 0 | 4 | 16 | 0 |
| transparent | 3 (3) | 1.000 | 0 % | 0 % | 6 / 0 | 3 | 13 | 0 |
| logo3-aa-jpeg75 | 3 (3) | 1.000 | 0 % | 0 % | 14 / 0 | 3 | 31 | 5 |
| gradient-steps (count 4) | 4 (4) | 1.000 | 0 % | 0 % | 6 / 0 | 4 | 16 | 0 |
| junctions-8 | 8 (8) | 1.000 | 0 % | 0 % | 40 / 0 | 9 | 54 | 93 |

The real images in Multicolour (`multicolour_real_table`): logos and clipart 1 to 6 colours;
photos 6 colours, up to 38 000 paths and 341 000 nodes (2.2 s), the edge spans that fell back
up to 6 200.

**Bar** (Chromium): Single colour heights unchanged from slice E (the Output pill has no visible
label and tight segments so it shares Advanced row 1 with Denoise and Upscale at 1280 x 800).
800 x 600 with the panel open, Multicolour, Advanced open, a swatch selected: the bar is 589 px
high (the UX notes' flagged worst case: it covers most of the canvas; a scrolling palette is the
architect's to propose).

Gate (2026-10-30): the full `CLAUDE.md` §7 gate green (9,604 tests); `reference_table` and
`quality_table` in release report the misses of the base (281 guard lines, the 14 wobble and
Silhouette misses): the single-colour path is unchanged (`fitted` delegates with every span
wanted, the guard's closed mode is the old code, the goldens stay exact). Chromium file by file:
`acceptance_0089_m_browser`, `_e_browser`, `_c_browser`, `_sliceC_tester`, `_sliceE_tester`,
`acceptance_0025_browser` (its Advanced list now has Output after the toggles), `_0025_tester`,
`_slice2_tester`, `_slice3_tester`, `acceptance_0087_browser`, `_0087_tester`: all pass.

**Tester and architect round (2026-10-31).** (1) Auto lost a pale accent and merged two
near-greys when the three shared one box: no two-way split explained 0.8 of its variance. A box
now also splits when a three-way split along its widest channel explains 0.95 (an even spread
gives 0.89, a normal one 0.81) with every group at least the merge distance from the next; a
two-level look-ahead was tried first and gave 11 to 16 colours on customer logos and line art.
(2) Content without a flat pixel (1.3 px anti-aliased rings, `clipart-clock` at 137 px) had an
empty palette: such regions now vote with their ridge pixels (no content neighbour farther from
the background); each ink not explained by a flat colour's blends toward the background (or its
ink beyond them) seeds a line colour, taken as the blend 80 % of its pixels are weaker than.
Real images in Auto against the slice's first build: 37 of 46 unchanged; `customer-bluetenzeit-logo`
2 to 11 colours (watercolour flower, leaves and gold script), `customer-balance-logo` 6 to 9,
`customer-photo-fox-2` 3 to 11, `customer-photo-lion-2` 4 to 6, `customer-dog-photo` and
`customer-precision-logo` 6 to 7; `customer-photo-bird-head` 6 to 2 and `customer-photo-fox-1`/`-3`
6 to 3 (a photo with one three-way split no longer falls back to the default 6 steps; reported for
the demo). (3) Recolour has a hex field (six digits, `#` optional) next to Reset; a recoloured
swatch is named "#NEW (was #OLD) · name". (4) The functions over 60 lines are split; a colour
loop smooths only the spans it owns; the session test for every colour excluded is added.
Memory (counting allocator, `customer-photo-fox-2`): 137.9 B/px besides the picture, tracked in
`docs/technical-debt.md`. `ColourLayer::fixed` stays: the tester's c26 case reads it (a layer of
a fixed colour). Latent: on `clipart-goatee` with a black line colour beside a dark-grey fill,
fitted edges passed a 1 px region within 0.002 mm (the tester's shared-segment check failed);
the line test above keeps that palette from forming, the guard's near-touch rule was not changed.

Open: Mask view shows the segmented palette (the lead's brief) instead of the black union of the
UX notes §4; Isolate dims there. A colour narrower than 3 px inside a region that has flat
pixels takes the nearest colour unless picked with the palette pipette. The upscaled label map is nearest-neighbour
(Upscale gives 2 px steps between colours).

## Slice F: automatic analysis

Implementer, 2026-10-31, on `story/trace-auto` (stacked on `story/trace-multicolour`, PR #230,
customer's leave to stack tonight). Design: `adrs.md` §9 (heuristic, explainable, no ML, the
report with properties and confidences, image type and capture, a proposal of image-dependent
values only, reasons as an enum); UX: `ux-notes.md` §3 (the analysis row and its states, Frage
83 B: no Suggested state), §11, §12. Customer answers in force: Frage 83 B (applied to a new
image automatically, never after a manual change; Analyse again redoes it), Frage 86 A (the four
presets stay; the analysis sets only image-dependent values and names the image type; never
Single colour / Multicolour).

### Affected crates/modules

- `curvyo-vectorize-core`: new `analysis.rs` (the public report and `analyse`), `analysis/`
  submodules `measures.rs` (the properties on a reduced copy and full-resolution crops) and
  `classify.rs` (image type, capture, confidence, the proposal and its reasons); `lib.rs`
  exports.
- `curvyo-ui-core`: `trace_transfer.rs` (`trace_analyse` for the worker).
- `curvyo-editor-wasm`: `wasm_trace.rs` (`trace_analyse` as JSON).
- `frontend`: `traceJobs.ts` / `traceWorker.ts` (the analysis job replaces the threshold job),
  new `traceAnalysis.ts` (the line's words and states, the apply rule), `useTrace.ts` (apply on
  a new image, Custom on an edit, Analyse again), `TraceBar.tsx` (the analysis row),
  `traceText.ts` (About entry).
- Harness: `tests/analysis.rs` (labelled tuning and test split, type accuracy, auto against the
  preset default per group, CPU at 2000 x 2000 px, determinism).

### Tasks

- [x] F1. Exploration (done before this plan): on the synthetic set Denoise never lowers the
      true-shape area error and Remove background only wins where Brightness misses content
      (pale or light colours, a coloured background); a lower Threshold wins on paper photos
      with a shadow. The proposal therefore holds Method, Threshold, background and Colour
      tolerance; Denoise is measured and reported, not set (AC 31, 33).
- [x] F2. `analysis` measures, test first: transparency, background colour and uniformity, noise
      on full-resolution crops, gradients (paper level across the picture), mid-tone share,
      colour count, stroke width, components and holes, straight-edge share, text-like rows;
      each with a confidence (AC 31).
- [x] F3. Classification and proposal, test first: image type and capture with a confidence; low
      confidence falls back to Brightness at the automatic Threshold and says so; pale content
      → Remove background with a tolerance at half the palest colour's distance; a shadow class
      on a paper photo → the lower cut; reasons as an enum; deterministic (AC 31, 32, 34).
- [x] F4. Harness: labelled tuning/test split by image family, type accuracy on the test part
      (>= 80 %), auto against the preset default per group with the truth metrics on the
      synthetic set and visual checks on the real images, misdetections listed, CPU at
      2000 x 2000 px (AC 32, 34).
- [x] F5. ui-core and wasm: `trace_analyse` as JSON (type, capture, confidence, proposal,
      reasons, properties) (AC 31).
- [x] F6. Frontend: the analysis row (Analysing..., Auto, Custom, Low confidence, Failed),
      applied to a new image, Custom after an edit of an analysed value, Analyse again, never
      Output, Preset, Smooth, Corners or picked colours; About entry; Chromium (AC 31, 33).
- [x] F7. Full gate; `reference_table` in release unchanged (the presets' paths are untouched);
      results below.

### Validation

`cargo nextest run -p curvyo-vectorize-core` (unit tests of `analysis`, `tests/analysis.rs`
criteria 31, 32 and 34 in the gate where cheap, the per-group comparison `#[ignore]`d in
release); `npm test` (the line's words, the apply rule); Chromium
`acceptance_0089_f_browser.test.ts` file by file; the full `CLAUDE.md` §7 gate once.

### Slice F results (2026-10-31)

**What the analysis sets.** On a new image (Frage 83 B) the method with its background and Colour
tolerance, and the Threshold; nothing else (Frage 86 A). Denoise was tried first (F1): Low or High
never lowered the true-shape area error of a synthetic fixture and broke 1 to 2 px lines, so it is
measured and reported only. With Pick colours the method stays (the picks are the maker's,
Proposal) and only the Threshold is set.

**Rules** (`analysis/classify.rs`, each a measured property against a stated limit):
- Remove background when flat content lighter than the automatic Threshold and unlike the
  background covers at least 0.2 % of the picture and a tenth of the dark content, on an even,
  quiet background (border at least 90 % background, noise under 0.75) or a transparent one; white
  on a transparent background counts as a cut-out. Colour tolerance: the smallest whose distance
  reaches half the palest tenth's distance from the background.
- The lower cut of a three-class Otsu split when the capture is a photo of paper and the middle
  class is at least 3 % of the picture, 8 px wide and three times the ink's width (a shadow).
- Capture: transparent → digital; noise at least 0.75 → photo of paper (border under 70 %
  background or paper level spread over 0.12) else scan; a border 90 to 99.5 % background
  (sheet edge, dust) → scan; else digital.
- Type, first rule that holds: 3 or more text-like rows → Logo; mid tones at least 0.1 with grain
  → Sketch; strokes at most 5 px or 1.2 % of the short side → Line drawing; 3 or more colours or
  chroma → Graphic; straight runs at least 35 % → Logo; else Graphic. Not digital and 45 % or more
  darker than the Threshold → unclear (confidence 0.3): Brightness at the automatic Threshold.

**Criterion 32** (`criterion_32_type_accuracy_on_the_test_part`, release): labels from the
sidecar groups (graphics and diagram Graphic, logos Logo, line drawings and AI drawings Line
drawing, sketches Sketch; paper scans and photos by their capture; a photo unclear). The split was
fixed before the rules: test part 22 images (the fox, bird-head and ladybug families with their
scans and photos, plus single images per group), tuning part 24. **Test 20 of 22 (91 %)**,
tuning 20 of 24 (83 %). Misses (misdetections): test `clipart-goatee` (Graphic, seen as Line
drawing: an ink drawing), `sketch-portrait` (Sketch, seen as Line drawing: little shading);
tuning `clipart-clock` (Line drawing), `diagram-kho-kho` (unclear: green field, JPEG), 
`customer-dog-lines` (Sketch, seen as Line drawing), `customer-lion-logo` (Logo, seen as
Graphic). Not scored but visible in the line: every photo of paper is typed Sketch (the desk and
shadow are mid tones); the AI logos with soft gradients and clean scans are told apart only by the
border (`customer-elephant`, a digital JPEG, reads "scan"). The synthetic fixtures are not labelled
for the type: their group says which material they emulate, a ring and a star are a logo or a
graphic only by name.

**Auto against the preset default per group** (`auto_is_at_least_the_default_per_group`,
release, synthetic set at three resolutions, true-shape area error summed, lost topology summed):

| Group | Area error default → auto | Lost default → auto | Changed fixtures |
|---|---|---|---|
| graphics | 22.1 → 22.1 | 0 → 0 | none |
| logos | 1969.5 → 420.3 | 80 → 68 | logo-pale 55 → 0.2 %, logo-blue 308 → 0.2 %, logo-transparent-colour 35 → 0.4 %, logo-pale-picked 103 → 31 %, logo-picked 56 → 6 % (each resolution) |
| line-drawings | 356.8 → 356.8 | 7 → 7 | none |
| sketches | 343.5 → 343.5 | 1099 → 1099 | none |
| ai-drawings | 132.2 → 132.2 | 26 → 26 | none |
| paper-scan | 279.2 → 279.2 | 7 → 7 | none |
| paper-photo | 3603.9 → 3178.5 | 3 → 3 | photo-shadow@1x 428 → 2.9 % (0.5x and 0.25x unchanged: the shadow class is under 8 px wide there) |
| diagram, photo | unchanged | unchanged | none |

No group is worse. Real images: the proposal differs from the default on
`customer-balance-logo`, `customer-bluetenzeit-logo` and `customer-precision-logo` only (Remove
background; visually the pale swirl, flower and gear are now content, as their sidecars ask; the
balance swirl's shading fills solid). The customer's paper photos keep the automatic Threshold:
the dark desk regions are darker than the ink, so no global cut separates them (Drop border shapes
does, which the analysis does not touch; reported for the demo).

**Criterion 34**: 2000 x 2000 px photographed sheet 81 ms thread CPU native, the same report
twice; slowest real image `customer-photo-fox-1` 74 ms.

**Line** (Chromium `acceptance_0089_f_browser`): "Graphic · pale colours kept", "Line drawing ·
scan · automatic Threshold", "Graphic · photo of paper · shadow left out", "Custom · Graphic",
"Type unclear · Brightness with automatic Threshold" (Proposal: the UX notes' "Logo / Line art
with automatic Threshold" would name a preset the analysis never sets), "Analysis failed ·
automatic Threshold used", "Analysing..." after 400 ms. Analyse again is `aria-disabled` in Auto
and Type unclear.

**Gate** (2026-10-31): the full `CLAUDE.md` §7 gate green (nextest 9,646 passed). Chromium file
by file: `acceptance_0089_f_browser`, `_e_browser`, `_c_browser` (now picks Brightness after the
analysis took Remove background for its pale fixture), `_m_browser` (closes Advanced before
sampling a block the taller bar covers), `_sliceM_tester`, `acceptance_0025_browser`,
`_0025_tester`, `_slice2_tester`, `acceptance_0087_browser`, `_0087_tester`: pass.
**Tester tests that encode the bar before F (not changed, for the tester):**
`acceptance_0089_sliceC_tester` (800 x 600 row heights: Brightness 252 px is now 288 px, the
analysis row), `acceptance_0089_sliceE_tester` c21 (screen sampling under the taller bar), c23 x2
(focuses the Threshold field of a fixture the analysis now starts in Remove background), c13
(stored Brightness reads back as Remove background because the new image's analysis applies,
Frage 83 B), `acceptance_0025_slice3_tester` c51 (counts the blue Trace button, now 36 px lower,
as outline) and c54 (the failure tooltip, now 36 px lower, covers the History tab for the first
click). Each passes on `story/trace-multicolour`; each is the analysis row's 36 px or Frage 83 B.

Open: the type of a photo of paper is not reliable (desk and shadow read as mid tones: every
customer paper photo is typed Sketch); clean scans are told from digital pictures only by the
border; `Type unclear` and `Analysis failed` lines differ from the UX notes' wording (Proposal,
above); the analysis does not propose Drop border shapes, which is what the customer's paper
photos need (the toggle is not an analysed value, UX notes §3).

## Baseline results

Today's tracer (visioncortex + `reduce`, main `faeb6f07`), automatic Threshold, each preset;
26 synthetic fixtures at 1x, 0.5x and 0.25x plus 15 CC0/public-domain images, 372 runs. Full
numbers: `baseline.json` key `reference` (one line per run); this table is reproduced by
`reference_table`. Distances in pixels of the traced raster; "truth→trace" is outline lost or
misplaced, "trace→truth" outline extra or misplaced; c/x/h = components / connections / holes;
corner and edge columns over the annotated corners and edges (straight edges > 20 px).

### Logo / Line art at 1x (per group; deviations are the mean of the fixtures' max)

| Group | n | Nodes/100 mm | Dev. truth→trace max | Dev. trace→truth max | Corner err. max | Corners ≤ 0.5 px | Straight dev. max | Segments/edge | Flips | Lost c/x/h | Extra c/x/h | Self-int. | CPU native/wasm ms | Peak B/px |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| graphics | 3 | 13.6 | 1.78 | 1.53 | 4.11 | 49 % | 1.00 | 6.5 | 52 | 0/0/0 | 0/0/0 | 0 | 17/27 | 6 |
| logos | 5 | 19.8 | 44.82 | 1.30 | 209.42 | 30 % | 0.85 | 4.4 | 47 | 17/0/5 | 0/1/1 | 0 | 18/21 | 5 |
| line-drawings | 4 | 21.8 | 2.88 | 0.87 | 6.50 | 72 % | 0.92 | 6.1 | 151 | 0/0/0 | 0/1/1 | 0 | 51/62 | 6 |
| sketches | 3 | 15.6 | 64.15 | 1.71 | 0.00 | 100 % | 0.00 | 1.0 | 0 | 400/0/0 | 0/0/0 | 0 | 15/17 | 5 |
| ai-drawings | 3 | 9.0 | 5.73 | 2.29 | 1.41 | 67 % | 0.67 | 5.1 | 8 | 1/0/1 | 0/0/61 | 14 | 51/62 | 6 |
| paper-scan | 3 | 13.9 | 1.83 | 2.33 | 2.24 | 95 % | 1.50 | 2.3 | 8 | 0/0/0 | 0/0/0 | 0 | 27/30 | 6 |
| paper-photo | 3 | 16.3 | 43.72 | 43.15 | 32.53 | 84 % | 1.50 | 3.2 | 28 | 0/0/0 | 18/2/0 | 2 | 28/30 | 7 |
| diagram | 1 | 7.5 | 2.03 | 1.00 | 2.83 | 80 % | 0.76 | 2.1 | 13 | 0/0/0 | 0/0/0 | 0 | 5/6 | 5 |
| photo | 1 | 62.4 | - | - | - | - | - | - | - | - | - | 34 | 4/4 | 6 |

### What the baseline shows

- **Worst groups.** *Paper photo*: the shadow is traced as content (trace→truth 43 px, 18 extra
  components at Logo; 243 at Engraving). *Logos*: the pale shapes are lost (`logo-pale`: #FFF2A8
  and #DCD6CF gone, area error 56 %), umlaut dots and counters fall to Specks (`text-umlauts`: 16
  components, 5 holes lost). *Sketches*: Specks 0.5 mm removes all 400 stipple dots at Logo
  (Engraving keeps them); the 2 px bow over 200 px is straightened (1.96 px off, 2 false corners).
  *AI drawings*: thin white strands break into 61 extra holes; 14 self-intersections at 1x, 470
  at 0.25x. *Line drawings at 0.5x/0.25x*: 33 to 36 thin components lost.
- **Geometry (the slice D target).** Corners: only 49 % (graphics) and 30 % (logos) of the
  annotated corners have a node within 0.5 px, worst 4.1 px (`geo-corners`, star tips and the
  30 degree corner). Straight edges carry 4 to 7 segments on average (max 13 on a 240 px edge of
  `line-angles`), deviate up to 0.80 px from the true line and wobble (151 curvature flips in the
  line drawings). Engraving is far worse here (39 segments per edge, 8585 flips) and 3x the nodes.
- **Self-intersections exist today**: the synthetic photograph 34 (195 at Photo), the real line
  drawings 83, the 0.5x diagram 301. Criterion 10 (0 everywhere) is a real change, not a guard.
- **Cost.** All 372 traces: 3.0 s thread CPU native, 3.6 s wasm (wasm 0.7x to 1.6x native,
  median 1.24x). Peak memory at Logo 5 to 8 B/px at 1x, up to 22 B/px on the smallest 0.25x
  pictures (fixed overhead); with Upscale (Engraving) 20 to 29 B/px per source pixel at 1x,
  because the 2x copy counts four times.
- Geometric mean nodes per 100 mm over all runs: Logo 19.4, Photo 34.9, Silhouette 9.3,
  Engraving 45.7.

### Proposed frozen targets (for the lead and the product owner; Proposal)

| Criterion | [T] in the spec | Baseline (Logo, 1x) | Proposed frozen value |
|---|---|---|---|
| 5 corner position | 0.5 px | max 4.1 px, 48 % within 0.5 px (`geo-corners`) | **0.5 px** at 1x and 0.5x; 0.25x reported only |
| 6 straight edge segments | 2 | mean 6.9, max 13 (`line-angles`) | **2** |
| 6 straight edge deviation | 0.35 px | 0.80 px (`line-angles`), 0.43 px axis-aligned (`geo-round`) | **0.35 px** at 1x and 0.5x |
| 6 curvature flips | 0 | 21 (`line-angles`) | **0** |
| 7 arc and bow deviation | 0.5 px | 0.72 px (`geo-round`), 1.96 px bow (`sketch-bowed`) | **0.5 px**, 0 false corners |
| 9 nodes against baseline | 85 % | 19.4 nodes/100 mm (geometric mean) | **75 %** (straight edges alone carry 3x the needed segments); 85 % if D's trial says so |
| 16 background suggestion | 0.02 | none (method new) | 0.02 in the C1 space, frozen after trial C1 |
| 16 area error | 2 % | `logo-pale` 56 % today (Brightness) | **2 %** |
| 17 pale boundary | 0.75 px | pale shapes lost today | **0.75 px** |
| 18 anti-aliased edge | 0.5 px | black edges 1.31 px max incl. corners (`logo-shapes`) | **0.5 px** |
| 20, 34 selection / analysis CPU | 0.5 s | whole set 3.0 s native | **0.5 s** at 2000 x 2000 px |
| 26 palette merge | 0.03 | none | 0.03 in the C1 space, frozen after trial C1 |
| 32 analysis type accuracy | 80 % | none | **80 %** |

Unchanged as guards: every other metric of every run no more than 5 % (plus 0.001) worse than
`baseline.json` (`reference_table`, nightly).

### Measuring method (for the lead to confirm, criterion 4)

- Truth: the drawn vectors (exact) for 21 fixtures; for implicit scenes (strokes, text, AI
  shapes) the border of the 4x truth raster, whose staircase adds up to 0.18 px.
- Deviation: samples every 0.1 px both ways, nearest distance to the flattened other side
  (1e-3 px). Corners: distance to the nearest node. Edges: trace samples within a window of half
  the edge's clearance (at most 3 px) and outside a margin of 10 % of the edge (2 to 10 px) at
  each end; an edge with less than half its length covered counts as lost.
- Topology on the 4x rasters: foreground 8-connected, background 4-connected; lost or extra = no
  overlap at all; a connection needs 1 px² of overlap. Self-intersections: proper crossings of the
  outlines flattened to 0.05 px.
- Time: thread CPU (native `clock_gettime`, wasm `process.threadCpuUsage` in Node via
  `frontend/scripts/trace-reference-wasm.ts`); peak memory: the `cap` allocator's high-water
  mark in a child process per run. Neither is guarded (machine noise).
- JPEG quality 50 is simulated by baseline-JPEG DCT quantisation with the standard tables and
  4:2:0 chroma (no encoder dependency); the entropy coding it leaves out is lossless.

### Nightly guard against the accepted state (2026-10-30)

The nightly `trace quality harness` failed on the misses the customer accepted (criterion 9a,
Frage 98 A, 99 A): they are measured against the pre-0089 `candidate` in `baseline.json`, which
stays the frozen reference. The nightly must fail on regressions only, so the accepted state is
committed next to it and the harness compares against that:

- `tests/fixtures/trace/accepted.json`: for every `<run>/<image>` (`candidate/...` and
  `<preset>/<image>`) and metric (`node_density`, `wobble`) that is beyond the 9a limit of the
  baseline, the accepted value. `quality_table` prints these as `accepted miss`; it fails on a
  miss that is not listed, on a listed one that is more than 5 % worse than its accepted value,
  and on any micro path or open outline (those have no accepted state).
- `tests/fixtures/trace/accepted_relations.json`: the preset relations of criteria 62 to 64 that
  miss, as text without the measured numbers. `preset_relations` fails on a relation miss that is
  not listed; the numbers behind them are guarded per image by `quality_table`.
- `tests/fixtures/trace/accepted_reference.json`: the same for `reference_table`, by
  `<preset> <case>: <metric>` (the per-run guard, the frozen targets, the geometric means and
  Silhouette against Logo), with the accepted value; fails on an unlisted miss or one over
  accepted + 5 %. Keys of images that are not committed are harmless.
- An improvement never fails. Accepting a new state is a deliberate act in a PR:
  `CURVYO_UPDATE_ACCEPTED=1 cargo test --release -p curvyo-vectorize-core --test quality -- --ignored --test-threads=1 quality_table preset_relations`
  rewrites both files from the current misses; the diff is the review.
