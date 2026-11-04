# ADRs for trace recognition (Part A: Recognise text)

Architect draft 2026-10-30, written against `main` at `a8df2729` for the decided Frage 70 B, 71 A, 72 A
and 73 A. Part B (shapes) gets its own dated note before it is built.

**Aligned 2026-10-31 (customer answers Frage 101 to 106; passages marked "Changed 2026-10-31"):** unsure letters
are guessed and reported (101 C), faces are searched in five tiers with early stop (102 B), nearest face A (103),
entry A (104), readout only (105 A), no canvas marks (106 B). Still no new crate, dependency, trait or
`format_version` change. The dated note in ADR 0011 §2 is still to be added when the build starts.

**No new ADR, no new crate, no new external dependency, no trait, no generic, no `format_version`
change, no new file format.** Recognition is one new module folder in `curvyo-text-core` and one
plan module in `curvyo-ui-core`. Three questions on the operation's meaning go to the customer with
safe defaults (end of this file); none touches the document model or a written format.

## Depends on

- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2, §3 and its note of 2026-10-20:
  `curvyo-text-core → (external only)`, `ui-core → text-core`. No edge changes. Recognition widens
  text-core's responsibility sentence from "string to outlines" to "and outlines back to glyphs";
  when the story is scheduled the architect adds a one-line dated note to 0011 §2 saying so.
- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §1, §2: a text is a
  leaf with `text_string`, `text_font`, `text_size` (cap height in mm) and `text_placement` (an
  `Affine`, so baseline angle needs no angle value); `text_letter_space`, `text_word_space` from
  `0079`. Recognition writes only these existing registers. Faces enter as bytes through
  `FaceStore`; text-core reads no file.
- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) and
  [`0089` adrs.md](../0089-trace-quality-and-colour/adrs.md) §8 and §11: the traced outlines are
  closed `BezPath`s in mm, outer before holes; the determinism rules (basic operations and `sqrt`)
  and the harness pattern (`#[ignore]`, `CURVYO_TRACE_LOCAL`, numbers only, `baseline.json`, +5 %
  guard) are reused as they are. Recognition does not depend on `vectorize-core`: it runs on any
  selected filled paths, traced or not.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one Recognise run is one history
  step (criterion 6).
- (Changed 2026-10-31: the Fonts tab filter no longer picks the faces, tiers do; favourites and the
  Curvyo fonts folder come from the library.) [`0075` adrs.md](../0075-font-library/adrs.md) decisions 1 and 4 and [`0082`
  adrs.md](../0082-glyph-table/adrs.md) decision 1: the Fonts tab's tag filter names the candidate
  families; `Face::glyph_for` and the per-`Face` `OnceLock` cache are the pattern for descriptors.

## Feature-local decisions (2026-10-30)

1. **Where it lives: a module folder `recognise/` in `curvyo-text-core`; the document side in a
   `ui-core` plan module.** Matching needs glyph outlines, metrics and the real `layout()` (for the
   final check, decision 6); all of that is in text-core, and text-core is already in the editor
   wasm, so the cost is one module. text-core input and output stay document-free: `&[BezPath]` in
   mm, `kurbo::Affine`, `Mm`, face indices. `curvyo-ui-core::text_recognise_plan` ("what Recognise
   text writes") maps the selection to outlines in document mm, calls text-core, and builds the text
   nodes and the leftover compound path as one transaction.
   Rejected: (B) a new crate `curvyo-recognise-core` needs an ADR and isolates nothing today (one
   consumer, no heavy dependency); **trigger to revisit:** a later `ocrs` decision brings model
   weights and an inference dependency, which belong in their own crate, out of the editor wasm by
   default. (C) all in `ui-core`: ui-core is interaction logic; outline matching is domain geometry.
   (D) `vectorize-core`: needs a new edge to text-core and would tie recognition to traces only.

   **Note 2026-11-01 (architect): the trigger fired.** The customer allowed an OCR model (Frage 109 B,
   110 A: PaddleOCR mobile weights on `rten`, not `ocrs`). Model, weights and inference go into the
   new crate `curvyo-ocr-core` ([ADR 0028](../../docs/adr/0028-ocr-text-in-trace.md), Proposed), used
   by Trace (`specs/0091-trace-text-recognition`). Recognition itself stays here in
   `curvyo-text-core::recognise`; it gains an optional OCR hint (`Recogniser::with_hint`, a
   look-alike table) and `fit_text` for 0091's Frage 114 B. Frage 103 A still holds for this
   selection-based tool; 114 B applies only to the Trace path.

2. **Modules, one sentence each, each under 500 lines.**

   | Module (`curvyo-text-core/src/recognise/`) | One sentence |
   |---|---|
   | `mod.rs` | Runs the recognition stages in a fixed order and returns the result. |
   | `sample.rs` | Samples a Bézier outline into points at a fixed spacing in basic operations. |
   | `blobs.rs` | Nests closed outlines into filled shapes, each an outer outline with its holes. |
   | `lines.rs` | Groups shapes into text lines and gives each line its baseline frame. |
   | `descriptor.rs` | Computes a glyph shape's occupancy descriptor for the shortlist. |
   | `distance.rs` | Measures the two-sided distance between two sampled outlines. |
   | `assemble.rs` | Turns matched glyphs into runs with string, size, placement and spacing. |
   | `candidates.rs` | Builds a line's glyph candidates, runs of 1 to 3 neighbouring shapes in line space. |
   | `shortlist.rs` | Ranks the faces of a tier for a line by descriptor distance and shortlists each candidate's nearest characters. |
   | `verify.rs` | Fits one face to a line's glyph candidates and judges its letters. |
   | `reference.rs` | Holds a face's reference glyphs (outlines and descriptors), built once per face. |
   | `shape.rs` | Compares a placed glyph's shape with its traced candidate beyond their distance. |
   | `size.rs` | Finds a line's size and baseline for one face. |

   (Added at build, 2026-10-31, architect review and tester findings: the last six keep each
   module under 500 lines.)

   **Changed 2026-10-31 (tester findings, Proposal):** a letter is *sure* only when, besides the
   tolerance and the 15 % margin, it has the candidate's pieces and holes, its shape residual
   (the part of the distance a change of weight does not explain) is within 0.8 % of the line's
   height and at most 0.35 of the next character's, and its largest distance is within 12 % of
   the line's height whatever the tolerance; the 50 % and 80 % rules count every glyph of the
   line, those left as paths included; the line is levelled on its baseline (median slope of
   the shapes standing on it) and its direction refined by the median slope of the letters'
   origins; each face is fitted again at the size its letters give; the named deviation is
   measured without the search cap.

   Public surface, minimal:
   ```rust
   // Changed 2026-10-31: `faces` became `tier` (one tier's faces); ui-core calls tier by tier (decision 5)
   pub fn recognise(outlines: &[BezPath], tier: &[&Face], tolerance: Mm) -> Recognition;
   pub struct Recognition { pub runs: Vec<RecognisedRun>, pub left_as_paths: Vec<usize>,
                            pub nearest: Option<NearestFace> }
   pub struct RecognisedRun { pub face: usize, pub text: String, pub size: Mm,
                              pub placement: Affine, pub letter_spacing: Mm, pub word_spacing: Mm,
                              pub max_deviation: Mm,       // over the sure glyphs only
                              pub guessed: Vec<usize>,     // char positions in `text`, ascending
                              pub outlines: Vec<usize> }
   pub struct NearestFace { pub face: usize, pub max_deviation: Mm }
   ```
   `max_deviation` is taken over the sure glyphs, so the notice's "largest deviation" stays within the
   tolerance; guessed glyphs are listed separately.
   `outlines` indices point back into the input, so ui-core knows which subpaths each run replaces
   and which stay. The descriptors are a private `OnceLock` on `Face` (decision 5).

3. **Pipeline.**
   1. *Shapes:* sample every closed subpath (`sample.rs`, spacing 0.05 mm, at most 4,000 points per
      outline; longer outlines are left as paths), nest by even-odd containment into shapes (outer
      plus holes). Open paths are left alone; a closed path without fill is read as filled
      (Changed 2026-10-31, lead, permit over refuse).
   2. *Lines:* the line direction is the principal axis of the shape centres (2 x 2 covariance,
      eigenvector by `sqrt`, a unit vector, no angle); shapes are projected into that frame and
      grouped into lines by vertical overlap. Reading direction is left to right along the axis;
      vertical, upside-down, mirrored and curved text is out of scope and stays paths. One text
      object per line (no multi-line text objects in this slice).
   3. *Glyph candidates:* the shapes of a line sorted along it; a candidate is a run of 1 to 3
      consecutive shapes (i, j, ä, %, :, ;, "). Segmentation is a dynamic programme over the line
      choosing the split with the lowest total distance, so accents need no special rule. Touching
      letters (one shape, several glyphs, e.g. connected scripts) are not split: they stay paths.
   4. *Shortlist (cheap, per face):* each candidate and each face glyph gets a descriptor
      (decision 4); the top 4 characters per candidate per face by descriptor distance go on. Faces
      are ranked by the sum of their best descriptor distances; the best 8 faces go to stage 5.
   5. *Verification (exact, in mm):* per face, the line's em size and baseline come from the medians
      of the shortlisted matches (candidate height over glyph height, candidate bottom minus glyph
      bottom); a 1-D search over ±3 % refines the size, a few translation-only iterations (mean
      residual vector) refine each glyph's position. The distance is decision 4's two-sided one.
   6. *Choice:* the line's face is the face with the most glyphs within tolerance, ties by lower
      summed mean distance, then by candidate order. Per glyph the character with the lowest mean
      distance wins **if** its maximum distance is within the tolerance **and** the margin to the
      next different character is at least 15 % of that character's mean distance (confidence =
      that margin, Proposal constant). Characters whose outlines in the chosen face coincide within
      a quarter of the tolerance (l/I/|, O/0 in some faces) are resolved by their neighbours
      (lower-case run, capitals, digits) and otherwise count as unsure.
   7. *Assembly and final check:* see decision 6.

   **3a. Guessing and tier stop (Changed 2026-10-31, Frage 101 C and 102 B; the constants are Proposals).**
   - A glyph is *sure* under the rule of stage 6 (max distance within tolerance and margin 15 %).
     Otherwise it is *guessed*: the lowest-mean-distance character of the line's face, even beyond tolerance
     or without margin. A line is converted only if at least **50 %** of its glyphs are sure; below that the
     whole line stays paths (so garbage never becomes text) and counts toward `left_as_paths`. A candidate
     that is one merged shape of several glyphs is not split and stays a path.
   - A **tier stops the search** when its best face (stage 6 choice) has at least **80 %** of the line's
     glyphs sure. Lines are decided independently, so one document can use faces from several tiers. If no
     tier reaches 80 %, the best face over all searched tiers wins if it reaches 50 %. The earliest tier
     that reaches 80 % wins even if a later tier would be better (spec criterion 9).
   - Guessed glyphs also feed the spacing medians, but never the face choice.

4. **Descriptor and distance.**
   - Descriptor: even-odd occupancy of a 16 x 16 grid over the candidate's box (cell-centre point in
     polygon, crossing counts in basic operations), 256 bits, plus the box aspect ratio and the hole
     count. Distance: Hamming plus weighted aspect and hole difference. Only for the shortlist;
     never decides a letter.
   - Outline distance: both outlines sampled at 0.05 mm; for every sample the nearest sample of the
     other outline through a uniform grid of `Vec` buckets (no hash map); the **maximum** of both
     directions (Hausdorff) is compared with the tolerance (criterion 2: "everywhere"), the **mean**
     ranks candidates. Not used: `kurbo` `nearest`, `flatten`, `arclen` (they use `cbrt`, `acos`,
     `powf` or adaptive recursion); sampling uses uniform parameter steps from the control polygon
     length.
   - Rejected: (b) Hausdorff against every glyph of every face: 64 faces x 190 characters x 30
     candidates is about 365,000 full comparisons, minutes instead of under a second. (c) Fourier or
     turning-function descriptors: need `sin`/`cos`/`atan2`, break decision 7, and depend on the
     contour's start point. (d) Moment invariants alone: cheap but confuse b/d/p/q-like shapes.

5. **Faces, character set and caching.**
   - **Changed 2026-10-31 (Frage 102 B): five tiers, searched in order, stop at the first tier that
     reaches the 80 % rule (3a).** All styles, outline faces only (single-line and SVG faces are
     excluded: a traced stroke is an outline, not a centre line). The Fonts tab filter no longer
     selects the faces.

     | Tier | Faces | Cap per run |
     |---|---|---|
     | 1 | faces already used by text in the document | all (a handful) |
     | 2 | favourites (`0075`) | 64 |
     | 3 | fonts the maker installed (Curvyo fonts folder) | 64 |
     | 4 | included fonts | all (3 today; at most 64) |
     | 5 | all other installed (system) fonts | 64 |

     A face in several tiers is searched once, in its earliest tier. Within a tier, faces are taken in
     library order; when a tier has more faces than its cap, the descriptor first pass (stage 4) only
     looks at the first 64 and the notice says "searched 64 of N system fonts" (tier 5, 3, 2 only).
     The worst case (all tiers fail) is about 64 x 3 + small = about 200 faces' descriptors, see decision 8.
   - **Loading:** ui-core calls `recognise` for tiers 1 to 4 with the bytes already in the session
     `FaceStore`; tier 5 bytes are read and handed over only for the outlines still in `left_as_paths`
     after tiers 1 to 4 (a second `recognise` call on those outlines). System font bytes are therefore
     never loaded when an earlier tier matches. The text object stores the face reference as any text does.
   - Character set: printable ASCII and Latin-1 letters and punctuation (about 190 characters),
     those a face lists (`glyph_for != 0`). Non-Latin is out of scope (spec).
   - Descriptors are computed lazily per face, cached in a `OnceLock` on `Face` (the `0082` pattern),
     lost with the session. Nothing is written to disk: no cache file, no new format. A persistent
     cache is added only if the measured first run misses the budget below.

6. **Assembly: runs, spacing, and the check that makes criterion 2 true by construction.**
   - **Changed 2026-10-31 (Frage 101 C):** per line, sure and guessed glyphs together become one text
     object (a line that passes the 50 % rule); `RecognisedRun.guessed` lists the guessed positions.
     Size = em size x the face's `cap_height` (Size is cap
     height, ADR 0020). Placement = the line frame's unit vector and the first glyph's origin on the
     baseline. Spaces: gaps between the layout's advance and the next glyph that exceed half the
     face's space advance become spaces; `letter_spacing` is the median residual inside words,
     `word_spacing` the median residual at spaces, both rounded to 0.01 mm.
   - **Final check:** the run's string is laid out with the real `layout()` at that face, size,
     spacing and placement, and every laid-out glyph is compared with its traced candidate by the
     same Hausdorff distance. A *sure* glyph over the tolerance after layout (spacing rounding) is
     demoted to guessed (Changed 2026-10-31; before: it split the run and stayed a path). So the sure
     glyphs in the document are what the renderer draws, within tolerance, glyph by glyph; guessed
     ones are reported.
   - **Marking (Changed 2026-10-31, Frage 106 B; Proposal):** no overlay and no stored flag. ui-core
     keeps a session record `guessed: text node id -> positions` (not in the document, not saved, no
     `format_version` change), dropped on the first edit of that text, on a selection change by the
     maker, or on undo. The Glyph table and the Text tool read it for the existing selection highlight.
     The history step (one transaction) contains only document changes.
   - Styling: the text takes the fill and stroke of the path it replaces; the leftover subpaths (lines
     under 50 % sure, unsplittable shapes, skipped outlines) stay one compound path with that style, at the same place in the layer order. Original removed,
     texts inserted, all in one transaction (one undo step).
   - Result line (for the UX notes): "Recognised 12 letters as Source Sans 3 Bold (included
     fonts), largest deviation 0.12 mm; 1 letter guessed: l (9)." (Changed 2026-10-31) With nothing within tolerance: "No text within 0.30 mm;
     nearest face Source Serif 4, 0.84 mm." (criterion 3 names face and deviation).

7. **Determinism.** As `0089` §8: only `+ − × ÷` and `sqrt`; no trigonometry, `powf`, `hypot`,
   `mul_add`, hash maps or threads; fixed iteration and summation order; ties broken by face order,
   then code point, then input index. Same input, same output, native and wasm bit-identical; a
   test runs the synthetic set twice and in the wasm `npm test` check.

8. **Performance targets (Proposal, measured in thread CPU, reported in the PR; a miss is a
   `docs/technical-debt.md` entry, not a blocker). Changed 2026-10-31 for the tiers.** Native on the
   0025 reference machine: descriptors ≤ 15 ms per face; a 30-glyph line against 64 cached faces
   ≤ 300 ms; first run per tier of 64 faces ≤ 1.3 s; tier 1 or 4 hit (the usual case) ≤ 0.3 s on a
   cold start; worst case, all five tiers searched cold (about 200 faces), ≤ 4 s, reported and
   filed as debt if missed. Later runs use the cached descriptors. The tier stop keeps the common
   case cheap; system fonts are read only when tiers 1 to 4 fail. Runs synchronously in the editor session (the faces
   live there); if the wasm measurement exceeds 1 s, moving it to a worker is a later note, not
   this slice. Wasm size growth is reported, not budgeted.

9. **Reference harness.**
   - text-core unit level (`tests/recognise.rs`): strings laid out with the included faces (Source
     Sans 3, Source Serif 4, Great Vibes) at 5, 10 and 24 mm cap height, turned 0°, 7° and −30°,
     with a deterministic ±0.05 mm wobble; ground truth = string, face, size, placement.
   - Chain level (`curvyo-ui-core/tests/recognise_reference.rs`, `#[ignore]`; ui-core already sees
     text-core and vectorize-core): render to pixels with a test-only even-odd scanline filler at
     300 and 600 dpi, trace with the Logo preset, recognise, score. A hold-out run removes one
     family from the candidates (criterion 3).
   - Metrics into `baseline.json` (keys `synthetic.*`, `local.*`): characters right, **wrong
     characters accepted (must stay 0)**, left as paths, face right, size error mm, max deviation,
     thread CPU. +5 % regression guard as `0089` §11.
   - The customer's images: read from `CURVYO_TRACE_LOCAL` with a local truth file per image (text
     and face name) that never enters the repository; only numbers are committed.

10. **Licence and privacy.** No new dependency, no model, no download (0025 Frage 58 A unaffected).
    Fonts are the maker's: outlines are read in memory for matching, descriptors are never stored
    or sent; the text object references the face as any text does. Nothing leaves the machine
    (criterion 5); a test runs recognition with no network access by construction (text-core has no
    I/O). The harness renders only the included OFL fonts, which the OFL allows.

11. **UI plug-in point.** Recognise text is a `ui-core` command on the selection (closed paths, filled or not; a
    traced group counts), explicit, never automatic (Frage 72 A). The tolerance (mm, default 0.3,
    criterion 6) is an options-bar value. Where the button sits (Trace tool options bar after
    Create paths, and the Path menu, as default) is the ux-engineer's call in `ux-notes.md`.

## Part B: Recognise shapes in Trace (implementer, 2026-11-03, Proposal)

Lead decisions of 2026-11-03 applied: the primitives sit inside the "Trace of" group in the fill
of their path; no new crate, no new dependency, no `format_version` change, no document model
change.

12. **Where it runs.** A pass over the outlines of the one trace, in the same worker job
    (`trace_job`, `trace_text_job`) after the trace and its text section, when the settings say
    `"shapes": true`: `curvyo-ui-core::trace_shapes` (pass and wire), `trace_shape_sample`
    (outline sampled at one picture pixel, area moments), `trace_shape_fit` (circle, ellipse,
    rectangle, rounded rectangle), `trace_shape_star` (regular polygon, star),
    `trace_shape_math` (`atan2`, `sin_cos` from `+ − × ÷` and `sqrt`). It adds a section (mark
    -4) after the text section; the traced numbers before it are untouched, and with the switch
    off there is no section, so the wire is bit-identical to before (criterion 11). The session
    reads the section and puts each recognised outline's primitive outline in place of the traced
    one, so preview and Create paths draw the same thing. No extra progress stage (criterion 6).
13. **Reuse of `0024` slice 3.** Its checks compare an SVG star or ellipse whose parameters the
    file states with its `d`; they cannot find unknown parameters, so the fit is new. Reused: the
    primitive constructors as the import builds them (rect with uniform `CornerRadii`, ellipse
    with `Curve::ELLIPSE_DEFAULT`, polygon and star by `StarFrame` with the first outer corner's
    angle and rotation 0) and `svg_import_match::outline_path` for a primitive's drawn outline.
14. **Fit and simplest fit (criterion 7).** Candidates in order, the first within tolerance wins:
    circle (centroid, mean radius), ellipse (second area moments: a solid ellipse has a²/4 and
    b²/4, axis from the moments), rectangle with square corners, rounded rectangle (direction from
    the length-weighted fourfold direction of the outline; each side the mean of the pieces within
    10° of it, refitted on the straight parts; radius from the area w·h − (4 − π)·r²), regular
    polygon, star (count 3 to 24 from the strongest harmonic of the radius over the angle; *changed
    2026-11-03 after the customer's 8-point star*: every side is folded onto the first corner's
    wedge and all sides are fitted as one line, a polygon's perpendicular to the wedge's middle;
    the turn is the one, within 2° of the harmonic's, where the folded sides lie best on one line;
    a second fit leaves out the tip the tracer cuts (2 x tolerance over the tangent of the half
    tip angle) and the valley; up to 1 % of a turn of steps back around the centroid is allowed
    at traced corners). Before, each side had its own line through its middle 60 % and the corners
    were where neighbouring lines crossed: a sharp tip far from the fitted middles turned a small
    line error into a radius error of 2 to 5 px, over the 2 px floor (7 of 392 stars of the sweep
    `curvyo-ui-core/tests/trace_shape_sweep.rs` failed, e.g. 4 points ratio 0.4 at 22.5°, radius
    2.8 px short; now 0 of 392, polygons 0 of 56 before and after). Accepted when every sampled point is within the tolerance of the primitive
    and the areas differ by at most the tolerance times the perimeter (no sliver along a curve).
15. **Tolerance share (lead: implementer's choice).** max(2 picture pixels, **1 % of the square
    root of the outline's area**). In pixels, so it does not depend on the dpi; the floor is the
    spec's 0.2 mm at 10 px/mm. Measured on the 10 px/mm fixtures (`curvyo-ui-core/tests/trace_shapes.rs`),
    largest deviation of the traced outline from the found primitive: circle 50 mm 0.033 mm (tolerance
    0.44 mm), ellipse 60 x 40 mm 0.039, rounded rectangle 200 x 100 mm 0.039 (1.41), hexagon 0.107
    (0.48), star 0.057 (0.49), ring 0.044 / 0.023, rectangles 0.000. The wobbly circle (±3 % of
    25 mm, irregular): best circle 0.78 mm, best ellipse 0.80 mm against 0.44, so it stays a path.
    Found parameters: rectangle 200.000 x 100.000, corner 9.94 (spec 10 ± 0.2); circle 25.004;
    ellipse 29.99 x 20.03 at 24.96° (25°); star outer 40.02, inner 20.00, turn off by 0.01°;
    hexagon 30.001. 0.5 % would leave a large scanned shape little room; 2 % would accept the
    wobbly circle (0.89 mm).
16. **Holes (lead: implementer's choice).** A region is an outer outline (winding as the
    largest outline of its colour) and the holes whose smallest enclosing outer it is. **The outer
    decides:** an outer that is no primitive leaves its region as traced paths, holes included
    (a letter's counter never becomes a polygon). A recognised outer without holes becomes a
    primitive node. A recognised outer with holes becomes its own compound path of the region,
    each recognised outline as its primitive's clean outline and each other hole as traced
    (criterion 8: a ring is one compound of two circles, a frame one of two rectangles). The
    document model has no compound of primitives; making the ring two editable circles with a hole
    would be a model change (architect and customer), so it is not done here.
    *Changed 2026-11-03 after the customer's page (an 8-point star drawn as a 2 to 3 px outline came
    out filled):* a primitive's outline always winds one way, so a hole replaced by its
    primitive's outline wound like the outer and the nonzero fill closed it (the ring of criterion
    8 had the same fault; the tests counted outlines, not fill). Each primitive outline now takes
    the winding of the traced outline it replaces, and a guard keeps a region's traced paths when
    its fill would change by more than its outlines' own accepted area changes add up to
    (`trace_shapes::with_primitives`, `keeps_fill`), so a hole is never lost. The customer's page
    (`curvyo-ui-core/tests/fixtures/trace_shapes/customer-page.png`): ellipse, rounded rectangle and
    15-point star become primitives, the outline star one compound of two clean star outlines,
    each region filling what it fills with the switch off within 2 %. The wire is unchanged.
17. **Place and fill.** Per colour (one for black and white): the colour's leftover compound path
    first, then its primitives and holed compounds in outline order, all in the "Trace of" group
    and in the colour's fill (recolours apply). Names are the primitives' own. One step, as before.
18. **Determinism (criterion 13).** Fixed order, no hash maps, no platform trigonometry; the
    shape sections of the golden pictures are recorded natively
    (`curvyo-editor-wasm/tests/fixtures/trace_shapes_native.json`) and the editor wasm gives them
    bit for bit (`frontend/tests/traceWasm.test.ts`).
19. **Time (criterion 6).** Thread CPU, debug build: 500 traced paths 0.033 s; the reference
    poster with Recognise text on (5 outlines left after the text) 0.002 s.
20. **Layout (lead 2026-11-03).** With the fourth toggle in its own row the Advanced block grew
    from about 222 px to 273 px at 800 x 600 with the panel open (the row 124 + 8 + Denoise 185 px
    no longer fit 304 px). The toggles now share the Output row, one flex row at 4 px gaps (173 +
    4 + 124 = 301 px): 237 px panel open, 165 px closed (`0089` tidy-up 22.3 within its 40 px).
    Tab order: Output, Colours, pipette, the four toggles, then strip, Denoise, Upscale, Corners.
21. **Bands of constant width (criterion 16; lead Proposal, customer Frage 119 A, 2026-11-03).**
    `curvyo-ui-core/src/trace_shape_band.rs` ("The centre-line primitive of a traced band of
    constant width"). A region with exactly one hole, both recognised as the same kind, is a band
    when they are parallel offsets of one primitive within the outer's tolerance: circles and
    ellipses with one centre (and turn) and both radii smaller by the same d; rectangles with one
    centre and turn, equal borders d, the centre line's corner radius the outer's less d/2 (a
    square outer is a mitred square centre line) and the inner's the centre line's less d/2 or
    square; polygons of one n, centre and turn, the apothems d apart; stars of one n, centre and
    turn whose sides are parallel within the tolerance over a side's length, the centre-line star
    the one whose sides lie halfway between (a mitred stroke keeps the corner angles and changes the
    ratio), and no tip or valley sharper than the renderer's miter limit of 4. The area guard of
    decision 16 for the stroke: the area the mitred stroke covers (offset areas: Steiner for
    ellipses, exact for rectangles, polygons and stars) must be the traced band's within the
    tolerance times both outlines' length. Create paths writes the centre-line primitive with a
    stroke of width d in the colour's fill, mitred, butt, solid, no fill; the preview still fills
    the two clean outlines (the same pixels). The wire: each region's shapes are followed by the
    band width (0: none) and, for a band, the centre-line primitive's numbers; every recorded "on"
    wire changes by that number (`trace_shapes_native.json`, `acceptance_0088_partb_native.json`
    re-blessed: circle, colour, cut, dots, ellipse, frame, grid500, hexagon, mixed, rect, ring,
    ring25, rounded, rounded_turned, star, wobbly_small; pictures and "off" wires unchanged). A
    band is counted as one shape in the status line. The customer's page: the outline star is a
    star of 8 points with a 0.998 mm stroke, stroked area 368.9 mm² against the traced 365.3 mm²
    (+1.0 %). Bands of varying width stay two clean outlines (R-EDIT-050, dynamic strokes).

## Not decided here

- Part B (shapes): decided in decisions 12 to 19 above; nothing moves to `geometry-core`. Tablet
  gestures (Later) need their own story.
- `ocrs`: a later customer decision; it would be a new crate and an ADR (decision 1 trigger).

## Customer decisions (2026-10-31, replaces the open questions)

- **REC-PARTIAL = C**: unsure letters are guessed and reported (decisions 3a, 6). The 50 % line rule is
  a Proposal guard against turning noise into text.
- **REC-FACES = B with the customer's tier order**: document, favourites, installed, included, system
  (decision 5; stop at the first tier that matches).
- **REC-NEAREST = A**: replace only within tolerance; otherwise nothing changes and the notice names the
  nearest face and its deviation.
