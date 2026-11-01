# Trace text recognition: read the text on the raster first

Status: Ready
Priority: Should
Origin: Customer (2026-10-31); numbers marked Proposal are the product owner's

## User value
As a maker who traces a logo, a sign or a scanned drawing I want the text in the picture read
before the outline trace and set as editable text in the nearest font, and left out of the traced
paths, so that I get real, editable wording and a clean cut path without raising the trace
threshold until the rest of the picture is ruined.

Customer wording (2026-10-31): tracing text needs a high threshold that hurts the rest, so text is
recognised on the raster first: "OCR, find positions, match the typeface glyph by glyph, narrow the
choice until something fits, then exclude the recognised text from the rest of the trace."

Field notes: Inkscape Trace Bitmap and LightBurn Trace Image return only paths; Ink/Stitch does not
trace. None of them reads text from the raster and none sets it in a font. Better here: one
threshold per text line, text kept out of the main trace, fonts matched by our own glyph matcher
(0088), everything offline.

## Terms
- **Text pass**: the step of Trace that runs before the outline trace when "Recognise text" is on.
- **Line**: a run of text that the OCR found, with a position, a string and a confidence per letter.
- **Accepted line**: a line with mean letter confidence of at least 0.6 and at least one letter or
  digit (Proposal, adrs.md decision 2) that also passes the garbage filter (Frage 115 B / 117 B):
  with a sure letter, accepted; without one, at least 2 characters, most of them letters or digits,
  line confidence at or above the threshold, not one short pattern repeated. Other lines are traced
  as paths as usual.
- **Sure / guessed letter**: as in 0088. Sure: the font's glyph matches within the tolerance.
  Guessed: best match, outside tolerance or without a clear margin.
- **Tolerance**: the Recognise text tolerance of 0088 (default 0.3 mm).
- **Left out**: the pixels of an accepted line are filled from their surroundings before the main
  trace, so the traced paths contain no letter shapes and a shape under the text stays whole.

## Decided by the customer (2026-10-31)
- Frage 107 A: switch "Recognise text" in Trace Advanced (default changed to off by 116 B a, 2026-11-01). The switch "Recognise shapes"
  is not drawn in this story: a control that cannot be honoured is not drawn. It arrives with
  0088 Part B.
- Frage 108 A: text first, shapes later.
- Frage 109 B: an OCR model is allowed. Frage 110 A: PaddleOCR mobile weights (ONNX) run by `rten`,
  our own pre- and post-processing (ADR 0027).
- Frage 111 A (default applied, unanswered): the weights are bundled in the installer and the web app;
  no download, nothing leaves the machine. Cost: about 10.3 MB.
- Frage 112 A: the Apache-2.0 weights are accepted without itemised training data; recorded in the
  licence list.
- Frage 113 A: text on a filled shape: the shape stays whole, the text lies on top.
- Frage 114 B: no font within tolerance: the read text is set in the nearest font anyway, the letters
  marked as guessed.
- Customer answers of 2026-11-01 (Frage 115 B, 116 B, 117 B): "a wrong font is better than paths;
  paths can still be made."
  - 115 B / 117 B: a line the OCR reads becomes text in the nearest font, letters outside tolerance
    marked guessed, even with no sure letter. This supersedes lead decision 1 of 2026-11-01 ("needs a
    sure letter"). Garbage filter (Proposal, lead): a line without any sure letter needs at least 2
    characters, most of them letters or digits, an OCR line confidence at or above a threshold
    (Proposal: 0.8, tuned on the fixtures and recorded in adrs.md), and must not be one short pattern
    repeated (the 0025 ring "O" and the dot grid "TOTOTO" stay shapes).
  - 116 B: everything below is in this slice; PR #244 stays the slice.
    a. Recognise text is OFF by default (criterion 17).
    b. One progress 0 to 100 % over the whole job (reading, fit, trace); the trace runs once, lines
       are decided before the main trace (criterion 25).
    c. Font-fit tolerance scales with the picture (criterion 26).
    d. Single-line (stroke) fonts are matched by centre line (criterion 27).
    e. Speed idea (Proposal, customer idea): candidate regions from a coarse trace (criterion 28).
- Lead decisions (2026-11-01, after the slice 1 tester pass):
  1. (Superseded by Frage 115 B / 117 B, see above.) Formerly: a line becomes a text object only if
     it has at least one sure letter. Reason then: the 0025 ring was read as "O" at 0.80 and a dot
     grid lost nodes; the garbage filter now covers this.
  2. "The app's licence list" (Frage 112) means `docs/third-party-assets.md` plus `model.toml`; there is
     no in-app licence page and none is added in this story. (Proposal, lead 2026-11-01)
  3. The switch "Recognise text" is not drawn while Multicolour is on in slice 1; slice 2 brings it.
     (Proposal, lead 2026-11-01)
  4. Speed targets 20 and 21 were measured on the main thread; the work runs in the trace worker. The
     targets stay as stated but are reported, not blocking (CLAUDE.md §3); the PR reports the measured
     numbers. (Proposal, lead 2026-11-01)
- Rules: no success notices (design-system.md, Interaction conventions); no hint texts; the Trace bar
  stays simple, the details live in Advanced.

## Slices
1. **Slice 1, one colour, end to end (Must for this story).** Black and white Trace of one image:
   OCR once per image, per-line threshold, text objects in the nearest font (0088 tiers), text left
   out of the main trace, switch in Advanced, offline, licence record, one undo step.
2. **Slice 2, multicolour.** The same in the multicolour trace: text takes the nearest palette colour
   of its pixels; text on coloured shapes stays on top of whole shapes.
3. **Later, own story:** Recognise shapes (0088 Part B) with its switch.

Slice 1 is the MVP cut. Each slice is tried by the customer on his own images.

## Acceptance criteria

Reference set (committed fixtures, rendered from OFL fonts): the 2000 x 2000 px poster of
`research-ocr.md` (8 text lines, including "Müller & Söhne" and a line with ÄÖÜäöüß, text on a
filled shape, white text on a coloured box), its noisy variant, and one rotated line (15 degrees).

Slice 1
1. Given the clean poster and Recognise text on, when Create paths runs, then at least 7 of the 8
   lines become text objects whose content equals the rendered string exactly (Proposal; the spike
   read 8 of 8 with one lost Ä).
2. Given the line "Müller & Söhne" and a line with ÄÖÜäöüß, then ä, ö, ü, ß, Ä, Ö, Ü are in the
   text where the source has them (checked per character on the reference set; the ocrs engine
   failed here, which is why it was rejected).
3. Given the reference set at the default tolerance, then the number of **sure** letters that are
   wrong is 0. A wrong letter may only appear as guessed. Checked by the harness, number in
   `baseline.json` (`ocr.wrong_sure`).
4. Given a line in a font we do not have, when no font within tolerance has 50 % sure letters (0088
   tiers) but the line has at least one sure letter in the nearest font, then the read text is set
   in the nearest font anyway (114 B), its other letters are guessed, and the Glyph table and Text
   tool highlight it as in 0088 until the first edit. Also a line with no sure letter at all becomes
   text in the nearest font, all letters guessed (115 B / 117 B), if it passes the garbage filter
   (Terms, "Accepted line"). Given a line that fails the filter (one character, mostly symbols,
   confidence under the threshold, or a repeated short pattern), then it stays traced paths.
5. Given at least one guessed letter, then one warning names them as in 0088 ("2 letters guessed:
   e (5), l (9)."). Given no guessed letter, then no notice appears (no success notice).
6. Given the poster with a text line on a filled shape, then the trace has the shape whole (no
   letter-shaped holes, 113 A) and the text object lies above the traced paths.
7. Given the poster and a global threshold at which the small text would fill in or break up, then
   the text still comes out right because each accepted line uses its own threshold, and the rest of
   the trace is the same as at that global threshold with the text pixels filled in (compared with
   Recognise text off: all paths outside the text boxes are identical).
8. Given the rotated line (15 degrees), then it becomes a text object with the rotation within 1
   degree.
9. Each text object's position, size and baseline match the traced letter outlines of its line within
   the tolerance (default 0.3 mm) for the sure letters, checked by comparing outlines as 0088
   criterion 2.
10. Given the black and white trace, then each text object's fill is black. Given white text on a
    dark box in black and white mode, the text is the foreground colour (the polarity is chosen per
    line), and the object is filled black.
11. Given Create paths with several accepted lines, then the result is one history step: one undo
    removes the "Trace of" group and all text objects, and the image is as before.
12. Given Recognise text off, then the Trace result is path-for-path identical to the result before
    this story, and no weight file is fetched.
13. Given the same image and settings run twice, then the text objects (content, face, position,
    size, guessed marks) and the traced paths are identical (determinism, ADR 0027 §6).
14. Given the desktop app and the web app with the network disabled, then recognition works and no
    network request is made (checked in the browser's network log and with the desktop app offline).
    The weight files come from the app's own files.
15. Given the weights, then `frontend/public/models/paddleocr-latin/model.toml` records name,
    licence (Apache-2.0), source URLs, sha256 per file and the training-data statement, a test fails
    if a file lacks its record or its hash differs, and `docs/third-party-assets.md` lists the
    weights with the same licence and source. There is no in-app licence page in this story.
16. Given a second Trace of the same image after changing only a Trace setting, then the OCR is not
    run again (same cached lines; the preview updates without the OCR time).
17. Given the switch "Recognise text" in Trace Advanced, then it is off by default (116 B a), is kept as an app
    setting like the other Trace settings, is not stored in the document, and no "Recognise shapes"
    switch and no hint text appears. While Multicolour is on (slice 1) the switch is not drawn.
18. Given a picture with no text (a plain shape), a plain ring, or a dot grid, then the trace result
    is identical to Recognise text off and no notice appears: the ring (read as "O") and the dot grid
    (read as "TOTOTO") fail the garbage filter and stay shapes, with Recognise text on.
19. Given a cancel during OCR, then the Trace dialog returns to its state before, with no document
    change.

25. Given Recognise text on, when Create paths or a preview runs, then the progress in the Trace
    button is one value from 0 to 100 % over the whole job (reading, font fit, trace), never
    running backwards, and the main trace runs exactly once: the lines are decided before it, so
    there is no second trace without the dropped lines (checked by a trace-call counter of 1 per
    run in a test).
26. Given the font fit, then its tolerance is measured in picture pixels relative to the line's
    x-height, not a fixed 0.3 mm. Given the customer's screenshot
    `curvyo-ocr-core/tests/fixtures/customer/font-manager-screenshot.png` (Curvyo font manager:
    large lines in Anak Gedong, EMS Allure, EMS Readability (single-line), Great Vibes; small UI
    labels, 13 px at 96 dpi, in a sans), then every line the OCR reads becomes a text object, and
    the Anak Gedong and EMS Allure lines are set in those fonts with sure letters. Small UI labels
    may get sure letters; none is required.
27. Given a single-line (stroke) font, then the fit compares the traced strokes' centre line
    (skeleton) with the font's stroke, not outline against outline. Given the EMS Readability line
    of the screenshot, then it is set in EMS Readability.
28. Given the OCR, then the PR reports the measured detection time and recognition time on the
    reference poster and the screenshot (Proposal, customer idea). If detection dominates, text
    candidate regions are found from a coarse trace (letter-sized shapes in rows, generous margin
    toward false positives) and only those crops go to the OCR, with the full-picture detection
    kept where candidates are unclear. Whichever path is kept must not lose a line that the full
    detection finds on the reference posters and the screenshot (checked by a test comparing the
    line sets). If detection does not dominate, nothing is built and the numbers are the result.

Speed and memory (Proposals; thread CPU time in the wasm worker, 2000 x 2000 px; reported, not
blocking: a miss is a `docs/technical-debt.md` entry, never a blocker; the PR reports the measured
numbers. Targets 20 and 21 were first measured on the main thread, the work now runs in the trace
worker.)
20. OCR of the reference poster takes at most 5 s once per image (research: 4 to 5 s wasm, 0.53 s
    native).
21. The text pass per preview takes at most 0.3 s for 10 lines; loading the weights at most 0.2 s per
    worker.
22. Editor wasm growth and peak memory of the OCR job are measured and reported in the PR (research:
    +0.93 MB gzip).

Slice 2
23. Given the multicolour trace of the poster, then each text object's fill is the nearest palette
    colour of its text pixels (Proposal, adrs.md decision 6), and white text on a coloured box is
    white text above the whole box.
24. Slice 1 criteria 3, 6, 11, 12 and 13 hold in the multicolour trace.

## Out of scope
- Recognise shapes and its switch (0088 Part B).
- Non-Latin scripts, vertical text, handwriting; script and decorative faces may read worse and are
  caught by the font fit or marked as guessed.
- Cloud OCR, accounts, telemetry, any download of weights.
- Choosing or installing OCR models; a model selector; a language selector.
- Editing recognised lines before Create paths (the text objects are edited afterwards with the
  Text tool).
- Overlay or preview markers for found lines.
- Result or success notices; hint texts.
- A new file format or `format_version` change.
- Changing the selection-based Recognise text tool of 0088 (Frage 103 A stays there; 114 B applies
  only to Trace).

## Open questions for the customer
None. Frage 111 is answered by default (A applied: weights bundled).

## UX notes
See [`ux-notes.md`](ux-notes.md).

## Links
Requirements: to be added when scheduled  PR: none
Related: 0023 (trace worker), 0025, 0088 (recogniser, tiers, guessed letters), 0089 (trace quality),
ADR 0027, `specs/0091-trace-text-recognition/adrs.md`, `specs/0088-trace-recognition/research-ocr.md`
