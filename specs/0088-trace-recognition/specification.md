# Trace recognition: text and shapes from traced outlines

Status: Part A Done (PR #234); Part B Ready
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

## Decisions of the lead, 2026-11-03 (Proposal, vetoable at the demo)
Frage 107 A (0091) put shape recognition into Trace. Part B is rewritten accordingly: switch "Recognise shapes" in
Trace Advanced, off by default, not in presets, black and white and Multicolour, tolerance by the 0091 c26
rule with no field, text decided before shapes, the hole rule left to the implementer, no select-bar
variant for now.

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
   used and the result names it, its tier and its largest deviation; the maker can pick another face from the 0062 list. **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no result notice naming the face, its tier and the deviation; the written text is the feedback.**
4. Given a line where no face reaches 50 % sure letters within tolerance (REC-NEAREST A), then the line stays paths,
   nothing changes in the document, and the notice reads "No text within <tol> mm; nearest face <name>, <dev> mm."
   <dev> is the largest per-letter deviation of that face, the one the tolerance judges (lead, 2026-10-31).
   Closed paths are read whether filled or not (lead, 2026-10-31, permit over refuse).
5. Nothing is sent over the network; recognition works with the network disabled.
6. The tolerance is a setting in mm (default 0.3 mm), shown in the options bar; the original paths are restored by one undo step.
7. Given a line where a letter is uncertain (outside tolerance or no clear margin) but at least 50 % of the line's
   letters are sure (REC-PARTIAL C), then the text object contains the best-match letter, and the result notice lists
   each guessed letter with its 1-based position ("2 letters guessed: e (5), l (9)"; more than 5 are cut with "and N more").
   The reported largest deviation covers the sure letters only. No canvas overlay or dashed outline appears; the new
   texts are selected and the Glyph table and Text tool highlight the guessed characters until the first edit of that text. **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no "Recognised N letters as ..." notice; only the guessed letters are a warning, as "2 letters guessed: e (5), l (9).".**
8. Given a document where the line's face is in tier 1 (used in the document), then no later tier is searched and no
   system font file is read (REC-FACES B). Given a face only in the system fonts, then it is found in tier 5 and the
   notice names "system fonts". A face in several tiers counts in its earliest tier.
9. Given two tiers that both reach 80 % sure letters on a line, then the earlier tier wins even if the later one deviates less.
10. The Recognise bar shows a readout of the face sources before a run ("5 face sources", tooltip lists the order);
    it has no selector. After a run the notice names the tier that matched (REC-SCOPE A).

## Part B: shape recognition as part of Trace (Ready)
Meaning (confirmed): a closed traced outline that matches a primitive (rectangle with corner radius, circle or
ellipse, regular polygon, star) within tolerance becomes that primitive, keeping position, size and rotation.
Entry (customer Frage 107 A in 0091, entry changed from "explicit operation on a selection" to "part of Trace"):
the switch **Recognise shapes** in Trace Advanced, next to Recognise text. Everything below is the lead's detailing
of that answer and is marked Proposal where it is a choice.

### Entry and behaviour
- The switch is off by default, like Recognise text (Proposal, by analogy with 116 B a). It is an app setting and
  not part of presets. It works in black and white and in Multicolour; in Multicolour the primitive keeps the
  fill of its colour.
- With the switch on, the preview and Create paths show recognised primitives in place of the traced paths. They
  are ordinary primitives, editable with their handles afterwards. Create paths is one undo step, as today.
- Holes (Proposal, implementer's choice, recorded in adrs.md): a compound outline becomes a compound of
  primitives. A hole that matches no primitive either stays a path inside the compound, or the whole compound
  stays paths.
- Tolerance: default 0.2 mm (Frage 73), applied relative to the picture as in 0091 criterion 26: in picture
  pixels, max(2 px, a share of the shape size), so the result does not depend on the dpi. No tolerance field in
  Advanced and no hint text (Proposal: the fixed rule is enough; add a field only if real traces show the need).
- Text first: with Recognise text also on, text lines are decided first; outlines that became text, and
  letters that stayed paths in a text line, never become shapes.
- The 0088 Part A select-bar tool stays as it is. A select-bar "Recognise shapes" for existing paths is Later.

### Acceptance criteria
Pictures below are rendered at 10 px/mm, so the 2 px floor equals 0.2 mm.
1. Given a traced 200 x 100 mm rectangle with 10 mm corners and Recognise shapes on, when Create paths runs,
   then one rectangle primitive of 200 x 100 mm with corner radius 10 mm (each within 0.2 mm) results instead of the path.
2. Given a traced circle of 50 mm diameter, then a circle primitive of 50 mm (within 0.2 mm) results; a traced
   ellipse 60 x 40 mm gives an ellipse with those radii and the traced rotation (within 0.5 degrees).
3. Given a traced five-point star, outer radius 40 mm, inner/outer ratio 0.5, then a star primitive with 5 points
   and those radii (each within 0.2 mm) results, rotation within 0.5 degrees.
4. Given a traced regular hexagon, then a polygon primitive with 6 sides results.
5. Given a closed outline whose deviation from every candidate exceeds the tolerance, then it stays a traced path,
   unchanged and without a notice. A hand-drawn wobbly circle (radius varying by +-3 % of 25 mm, about 0.75 mm)
   is such a case and stays a path.
6. Given 500 traced closed paths and Recognise shapes on, then the recognition pass finishes in under 2 s
   thread CPU time on the 0025 reference machine. It runs in the trace worker or pool, inside the one Trace progress.
7. Simplest fit: a shape becomes a primitive only when every sampled outline point is within tolerance and the
   primitive is the simplest fit. A circle is a circle, not an ellipse; a square-cornered rectangle is not a
   rounded one; a regular polygon is a polygon, not a star, when both fit equally.
8. Holes are kept: a traced ring (circle with a circle hole) becomes one compound of two circles; a rectangle
   frame becomes a compound of two rectangles. The hole rule for non-matching holes follows the Proposal above.
   **Changed 2026-11-03 by criterion 16 (Frage 119 A):** a ring or frame of constant width is one stroked
   primitive; a compound of two clean outlines remains for bands of varying width.
9. Given the 0025 ring fixture, then the result is 1 compound of 2 circles (**changed 2026-11-03 by criterion 16:**
   one circle with a stroke of the ring's width). Given the 0025 dot grid fixture,
   then the result is 10,000 circles (100 x 100) and no path remains.
10. Multicolour: given a picture of a red circle and a blue rectangle, then the result is a circle primitive with
    the red fill and a rectangle primitive with the blue fill.
11. Off equals today: with Recognise shapes off, the Trace result for every existing trace fixture is
    path-for-path identical to the result before this part (compared node by node).
12. Interplay with text: given a picture with a line of text and a circle, with Recognise text and Recognise
    shapes both on, then the result is one text object and one circle primitive, and no letter (not even
    an "o") became a circle.
13. Determinism: the same picture and settings give the same primitives, with identical parameters, natively and
    in wasm, and on repeated runs.
14. Create paths with recognised primitives is one undo step that restores the state before it.
15. Nothing is sent over the network; recognition works with the network disabled.
16. Bands of constant width (lead Proposal 2026-11-03, customer Frage 119 A): given a traced region with exactly one
    hole whose outer and inner outlines are parallel offsets of one primitive (same kind, centre, rotation and n,
    the sides a constant distance d apart within tolerance), then it becomes that primitive at the centre line
    with a stroke of width d, no fill, the stroke in the region's fill colour (black, or its palette colour). A
    ring is a stroked circle, a rectangle frame a stroked rectangle (corner radius at the centre line), an outline
    star or polygon a stroked star or polygon with the join as traced, miter or bevel (**changed 2026-11-04, lead
    Proposal**: a star or polygon whose traced outer corners are cut flat is a bevelled stroke; the centre line
    is derived from the offsets; a mitred corner sharper than the miter limit of 4 is not a stroke). The stroked
    area matches the traced band's within the tolerance, else the region stays two clean outlines, as does a band
    of varying width (dynamic strokes are R-EDIT-050). Touching outline stars or polygons (lead Proposal
    2026-11-04): a region whose outer outline is no primitive, with several holes that are each the inner outline
    of a stroked star or polygon whose outer outlines together make the region's outer, is one stroked primitive
    per hole; when the region cannot be split so within the tolerance it stays paths. The customer's page: the
    8-point outline star is one stroked star with n = 8, its stroked area within 2 % of the traced band's.

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
- Recognise shapes on existing paths from the select bar (Later; Part A's select-bar text tool is unchanged).
- A tolerance field for shapes in Advanced.
- Live pen-stroke recognition (see Later).
- Font identification from the web, installing missing fonts.

## UX notes
See `ux-notes.md` (Ready, aligned 2026-10-31). Technical decisions: `adrs.md`.

## Questions for the customer
None open. Frage 70 to 73 are decided (2026-10-28), Frage 101 to 106 (2026-10-31), see above.

## Links
Requirements: to be added when scheduled  PR: none
Related: 0024 (slice 3), 0025, 0062, 0074, 0075, 0082
