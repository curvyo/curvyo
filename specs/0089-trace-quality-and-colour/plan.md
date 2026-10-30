# Plan for Trace quality and colour, slices A and D

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
