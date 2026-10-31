# Trace recognition: text and shapes from traced outlines

Status: Ready (Part A; Part B is later)
Priority: Could
Origin: Customer idea 2026-10-28 (the split into parts A and B and all tolerances were Proposals, now decided, see below)

## User value
As a maker who traced a logo, a sign or a scanned drawing I want the traced outlines turned back into
what they were (a line of text, a star, a rounded rectangle, a circle) so that I can edit the wording, the
radius or the number of points instead of moving hundreds of nodes, and so that the cut path is clean and short.

Customer wording: "an OCR feature that recognises text and maps it to the nearest font, and object
back-mapping, rebuilding a star object from a pixel star. Might be a castle in the air, but maybe a
practical synergy with the glyph table."

## Decided by the customer (2026-10-28)
- Frage 70 REC-ORDER: B, text first. "Shapes are a gimmick, text recognition is real added value." Text is Part A, shapes are Part B and come later.
- Frage 71 REC-TEXT-ENGINE: A for now: glyph-outline matching against our own fonts, no model. The customer is unsure; the `ocrs` option stays documented below as a possible later decision (it would change 0025 Frage 58).
- Frage 72 REC-MEANING: A. Both operations are explicit, undoable, run on a selection, never automatic, and leave the path alone when unsure.
- Frage 73 REC-TOL: A, adjustable tolerance (default 0.3 mm for text, 0.2 mm for shapes); he restated it for shapes.

## Decided by the customer (2026-10-31)
- Frage 101 REC-PARTIAL = C: uncertain letters are guessed (best match) and marked, not left as paths. Marking uses
  existing UI only, no canvas overlay: the result notice lists the guessed letters with their positions, the new texts
  are selected, and the Glyph table and Text tool highlight them while the session lasts. A line with fewer than
  50 % sure letters stays paths (Proposal guard against turning noise into text).
- Frage 102 REC-FACES = B with the customer's tier order: (1) fonts used in the document, (2) favourites, (3) fonts the
  maker installed (Curvyo fonts folder), (4) included fonts, (5) all other installed system fonts. The search stops at
  the first tier with a match within tolerance. The 80 % per-line threshold for "a match" and the 64-face cap per tier are Proposals.
- Frage 103 REC-NEAREST = A: replace only within tolerance. Otherwise nothing changes and the notice names the nearest face and its deviation.
- Frage 104 REC-ENTRY = A: a "Recognise text" button in the Select bar chooses the Recognise tool, whose bar holds the tolerance and the Recognise button.
- Frage 105 REC-SCOPE = A: a readout of the face sources, no selector; the result names the tier that matched.
- Frage 106 REC-MARK = B: no dashed outline of leftovers; affected letters are selected and reported in words.

## Builds on
- 0025 raster trace: its output paths are the input. Recognition runs on traced paths only, never on pixels.
- 0074 text objects, 0075 font library, 0062 fonts, 0082 glyph table: target of part A and source of glyph outlines.
- 0024 slice 3: recognition of Inkscape stars, polygons and ellipses as primitives. Part B reuses its shape checks and
  primitive constructors; do not write a second set.

## Field notes (Proposal basis)
- Inkscape Trace Bitmap and LightBurn "Trace Image" return only paths. Neither rebuilds primitives or text. Ink/Stitch
  does not either. Nobody in the comparison set does this, so it is a real differentiator.
- Text options, all local-only (data never leaves the machine, CLAUDE.md §3):
  1. Glyph-outline matching against our own installed faces (0075/0082). No model, no new licence. Needs
     segmented glyphs and a known script. Weak on touching letters and unknown fonts. Works well on clean traces. **Chosen for now.**
  2. `ocrs` (pure Rust, MIT/Apache-2.0 code). Needs ONNX-style model weights (a few MB, separate download, licence of
     the weights must be checked before any use). Latin text mainly; accuracy on stylised fonts is modest. **Possible later decision**, only if option 1 proves too weak on real traces.
  3. Tesseract (Apache-2.0, C++ with C bindings, trained data Apache-2.0). Breaks the pure-Rust `*-core` rule and
     wasm32 build; usable only behind a `-io` crate on desktop. Not recommended.
  Licences above are from memory of the projects and must be verified against the repositories before an ADR.
- Conflict: 0025 Frage 58 A decided "the app ships no model". Option 2 needs weights, so it is either an optional
  user-triggered download (a data-leaves-the-machine question for the download itself, none for the data) or off the table.
  Option 1 does not conflict.
- Shapes need no model: fit-and-test. Fit a candidate primitive to the closed path (least-squares circle/ellipse,
  corner detection for rectangles and polygons, radial profile for stars), compute the maximum deviation, accept
  if below the tolerance, otherwise leave the path alone.

## Part A: text recognition (first)
Meaning (confirmed): **Recognise text** replaces a selected group of traced letter outlines by a text
object (0074) in the library font (0075) whose glyphs match best, keeping position, size and baseline angle.

### Acceptance criteria (draft, glyph matching only)
1. Given letters traced from a rendered line "Curvyo 2026" in an installed face at 24 mm cap height, when
   Recognise text runs, then one text object with the content "Curvyo 2026" and that face results.
2. The text object's glyph outlines deviate from the traced outlines by at most the tolerance (default 0.3 mm) everywhere (checked by comparison).
3. Given text in a face we do not have, when a face within tolerance exists in some tier, then the nearest such face is
   used and the result names it, its tier and its largest deviation; the maker can pick another face from the 0062 list.
4. Given a line where no face reaches 50 % sure letters within tolerance (REC-NEAREST A), then the line stays paths,
   nothing changes in the document, and the notice reads "No text within <tol> mm; nearest face <name>, <dev> mm."
5. Nothing is sent over the network; recognition works with the network disabled.
6. The tolerance is a setting in mm (default 0.3 mm), shown in the options bar; the original paths are restored by one undo step.
7. Given a line where a letter is uncertain (outside tolerance or no clear margin) but at least 50 % of the line's
   letters are sure (REC-PARTIAL C), then the text object contains the best-match letter, and the result notice lists
   each guessed letter with its 1-based position ("2 letters guessed: e (5), l (9)"; more than 5 are cut with "and N more").
   The reported largest deviation covers the sure letters only. No canvas overlay or dashed outline appears; the new
   texts are selected and the Glyph table and Text tool highlight the guessed characters until the first edit of that text.
8. Given a document where the line's face is in tier 1 (used in the document), then no later tier is searched and no
   system font file is read (REC-FACES B). Given a face only in the system fonts, then it is found in tier 5 and the
   notice names "system fonts". A face in several tiers counts in its earliest tier.
9. Given two tiers that both reach 80 % sure letters on a line, then the earlier tier wins even if the later one deviates less.
10. The Recognise bar shows a readout of the face sources before a run ("5 face sources", tooltip lists the order);
    it has no selector. After a run the notice names the tier that matched (REC-SCOPE A).

## Part B: shape recognition on traced outlines (later)
Meaning (confirmed): **Recognise shapes** replaces a selected closed traced path by the primitive
(rectangle with corner radius, ellipse or circle, regular polygon, star) it matches within a tolerance, keeping position, size and rotation.

### Acceptance criteria (draft)
1. Given a traced 200 x 100 mm rectangle with 10 mm corners, when Recognise shapes runs with tolerance 0.2 mm,
   then one rectangle primitive of 200 x 100 mm with radius 10 mm (each within 0.2 mm) replaces the path.
2. Given a traced circle of 50 mm diameter, then a circle primitive of 50 mm (within 0.2 mm) results; a traced
   ellipse 60 x 40 mm gives an ellipse with those radii and the traced rotation (within 0.5 degrees).
3. Given a traced five-point star, outer radius 40 mm, inner/outer ratio 0.5, then a star primitive with 5 points
   and those radii (each within 0.2 mm) results, rotation within 0.5 degrees.
4. Given a traced regular hexagon, then a polygon primitive with 6 sides results.
5. Given a path whose deviation from every candidate exceeds the tolerance, then it stays unchanged and the
   result says so in one line.
6. Given a selection of 500 traced paths, then recognition finishes in under 2 s on the 0025 reference machine
   (thread CPU time) and is deterministic: same input, same primitives.
7. The tolerance is a setting in mm (default 0.2 mm), shown in the options bar; the original path is restored by one undo step.
8. Hole structure is kept: a recognised ring (circle with circle hole) stays a compound with both primitives.

## Later
- Gesture-based shape recognition (customer, 2026-10-28): the customer compares Part B with VectorCraft's shape
  builder, which he finds practical on a tablet. There the drawing stroke itself (the gesture, not only the finished
  snapshot) is available, so shape recognition may later also work on live pen strokes (draw a rough circle, get a
  circle primitive), not only on traced outlines. Needs its own story, the pen-input path and a decision; not part of this spec's criteria.
- `ocrs` model-based text recognition if glyph matching proves too weak (see field notes; changes 0025 Frage 58).

## Out of scope
- Recognition on pixels (OCR of photos, scans, handwriting); only traced vector paths.
- Cloud OCR, accounts, telemetry, any remote service.
- Bundling a model in the app (0025 Frage 58 A stands unless a later decision changes it).
- Non-Latin scripts, math, tables, document layout analysis.
- Recognising Inkscape-only shapes (arcs, spirals) beyond what 0024 slice 3 covers.
- Automatic recognition during tracing; these are explicit operations on a selection.
- Live pen-stroke recognition (see Later).
- Font identification from the web, installing missing fonts.

## UX notes
See `ux-notes.md` (Ready, aligned 2026-10-31). Technical decisions: `adrs.md`.

## Questions for the customer
None open. Frage 70 to 73 are decided (2026-10-28), Frage 101 to 106 (2026-10-31), see above.

## Links
Requirements: to be added when scheduled  PR: none
Related: 0024 (slice 3), 0025, 0062, 0074, 0075, 0082
