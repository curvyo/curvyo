# ADRs for trace text recognition

Architect, 2026-11-01. Feature-local decisions for recognising text during Trace with an offline OCR
model. Customer answers of 2026-10-31: Frage 107 A (switches "Recognise text" and "Recognise shapes"
in Trace Advanced, on by default), 108 A (text first), 109 B (an OCR model is allowed), 110 A
(PaddleOCR mobile ONNX weights on `rten`, our own pre- and post-processing, no Paddle code), 111 A
(default, unanswered: weights bundled, no download), 112 A (accept the weights, record them),
113 A (text on a filled shape: the shape stays whole, the text lies on top), 114 B (no face within
tolerance: set the read text in the nearest face anyway). Research:
[`../0088-trace-recognition/research-ocr.md`](../0088-trace-recognition/research-ocr.md).
Constants marked Proposal are tuned by the harness (decision 8) and vetoable at the demo.

**One new ADR, one new crate, one new external dependency family, no trait, no generic, no
`format_version` change, no new file format we write.**

## Depends on

- [ADR 0027](../../docs/adr/0027-ocr-text-in-trace.md) (new, Proposed): crate `curvyo-ocr-core`,
  `rten`, the shipped weights and their licence record, the scoped `rayon` exception, determinism.
- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) §2, §4, §5 and
  [`0089` adrs.md](../0089-trace-quality-and-colour/adrs.md) §8: `trace()` and `TraceParams` stay
  as they are; this feature adds a text pass before the trace in the same worker job, and hands the
  trace an in-painted raster (dated note added to 0023 §5).
- [`0088` adrs.md](../0088-trace-recognition/adrs.md) decisions 1 to 7: the recogniser
  (`curvyo-text-core::recognise`), the five face tiers, sure and guessed letters, the session record
  of guessed positions and its determinism rules are reused; decision 1's trigger fired (dated note
  added there). Frage 103 A stays for the selection-based Recognise text tool; 114 B applies only
  to the Trace path.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2, §6: `ui-core → ocr-core` and the
  ban-list exception (dated notes added).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): Create paths stays one history step.
- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §1, §4: texts use the
  existing registers; the weights' licence record follows the fonts' pattern.

## Feature-local decisions

1. **Flow, and OCR once per image.** With Recognise text on, the trace worker runs a separate OCR job
   on the full working image the first time an image is traced in a session (`ocr_job`, progress
   per stage and line, cancel by terminate as ADR 0023 §5). Its lines (quad in source px, string,
   per-character confidence) are small and kept in the editor session, keyed by the image's asset
   hash (ADR 0017); not in the document, not saved. Every later preview and Create paths job gets
   the lines as input, so the 4 to 5 s of OCR are paid once per image, not per settings change.
   Preview jobs on the reduced copy scale the quads by the copy's factor. Rejected: OCR inside every
   trace job (seconds per slider move); a cache inside the worker (lost on every terminate, the
   missing stage cache of `docs/technical-debt.md`).

2. **Text pass in the worker: line region, local threshold, region trace (ui-core module
   `trace_text`, "Turns OCR lines into region outlines and the pixels the trace leaves out").**
   A line is *accepted* if its mean character confidence is at least 0.6 and it holds a letter or
   digit (Proposal); other lines are ignored and traced as usual. Per accepted line: the quad's box
   plus 2 px is cropped; Otsu inside the crop (`vectorize-core::auto_threshold`) picks the line's
   own threshold; the polarity is the class that touches the crop border less (white text on a
   green box is foreground); the crop is traced with the current trace settings and that threshold,
   mapped back by the crop's pixel map. The line's text pixels (foreground inside the quad, grown
   by 1 px) form its mask. This removes the global-threshold problem for text only.

3. **Leaving text out of the main trace (113 A): in-painting, not a mask parameter.** Before the
   main trace, every masked pixel takes the per-channel median RGBA of the unmasked pixels on the
   1 px ring around its mask component (integer, fixed order; ties to the lower value). So a text
   on a filled shape leaves the shape whole in both the black and white and the multicolour paths,
   and `trace()` stays unchanged. Done in place on the worker's own copy where the binding allows,
   otherwise one copy (+4 B/px, measured against 0089 §8). Rejected: an exclusion mask in
   `TraceParams` (`TraceParams` is `Copy`, and both pipelines would need a masked labelling step);
   letter-shaped holes (113 B, not chosen).

4. **Font fit with the OCR string as a hint (text-core, two small additions in `recognise/`).**
   The worker returns, per accepted line, the region outlines and the string. The editor session
   (where the faces live, 0088 decision 8) runs the recogniser on each line's region outlines,
   tier by tier as 0088 decision 5, with `Recogniser::with_hint(outlines, tolerance, text)`: the
   line's character set is the hint's characters plus their look-alikes from one constant table
   `recognise/confusable.rs` ("Lists the characters OCR confuses with each character": l I 1 | !,
   O 0 o Q D, ß B, A Ä À Á Â, a ä, o ö, u ü, U Ü, S 5, Z 2, g 9 q, c e, rn m and the like). Size,
   baseline, spacing, sure and guessed letters and the tolerance check are 0088's, unchanged; the
   matcher decides, OCR only narrows. Without a hint the recogniser behaves as today.

5. **No face within tolerance (114 B) and how it is marked.** If no tier gives the line 50 % sure
   letters, the session sets the OCR string in the nearest face over all searched tiers with
   `recognise::fit_text(line, face, text)` (size and placement fitted as in `size.rs`, every
   position guessed). The included fonts always exist, so there is always a nearest face. Marking
   uses 0088's existing means only (Frage 106 B: no canvas overlay, no stored flag): all its
   positions go into the session record `guessed`, so the Glyph table and the Text tool highlight
   the whole text while the session lasts. There is no success notice (customer rule of
   2026-10-31); only the 0088 "letters guessed" warning names the line set in the nearest font
   (wording in the UX notes).

6. **What Create paths writes (extends `trace_plan`).** Per image: the "Trace of" group as today,
   plus one text object per accepted line, inserted above the traced paths (113 A), all in one
   history step. A text takes the fill of its line's foreground: black in a black and white trace,
   the nearest palette colour of its masked pixels in a multicolour trace (Proposal). Region
   outlines of an accepted line that the recogniser left as paths (unsplittable shapes) are added
   to the trace's compound path, since the main trace no longer covers those pixels. The preview
   shows the same result (ADR 0023 §4: preview equals result).

7. **Switches (107 A, 108 A).** "Recognise text" in Trace Advanced, on by default, an app setting
   like the other trace settings (not in the document). "Recognise shapes" appears with Part B of
   `0088` (shapes come later, 108 A); until then it is not shown (Proposal; the product owner may
   prefer a disabled switch). Turning text off skips decisions 1 to 6 and traces exactly as today.

8. **Determinism and harness.** The text pass (crop, Otsu, polarity, in-painting) follows 0089 §8
   (basic operations, integers, fixed order, no hash maps, no threads of our own). OCR follows
   ADR 0027 §6. Harness as 0089 §11 and 0088 decision 9: `#[ignore]`, `CURVYO_TRACE_LOCAL`, numbers
   only into `baseline.json` (`ocr.*`: lines found, characters right, wrong characters accepted,
   lines set in nearest face, thread CPU), the 2000 x 2000 research poster and its noisy variant as
   committed synthetic fixtures (rendered from OFL fonts), +5 % guard.

9. **Performance and memory (Proposal; reported in the PR, a miss is a debt entry, never a
   blocker).** Thread CPU, wasm worker: OCR of 2000 x 2000 px ≤ 5 s once per image; text pass per
   preview ≤ 0.3 s for 10 lines; weights load per worker ≤ 0.2 s. Peak memory of the OCR job
   measured by the counting allocator and recorded.

10. **Modules (one sentence each, each under 500 lines).**

    | Module | One sentence |
    |---|---|
    | `curvyo-ocr-core/src/model.rs` | Loads and checks the detector, the recogniser and the alphabet. |
    | `curvyo-ocr-core/src/detect.rs` | Finds word boxes in a raster with the detector. |
    | `curvyo-ocr-core/src/lines.rs` | Groups word boxes into text lines with one quad each. |
    | `curvyo-ocr-core/src/read.rs` | Reads one line's string and character confidences with the recogniser. |
    | `curvyo-ui-core/src/trace_text.rs` | Turns OCR lines into region outlines and the pixels the trace leaves out. |
    | `curvyo-ui-core/src/trace_inpaint.rs` | Fills masked pixels from the ring around them. |
    | `curvyo-text-core/src/recognise/confusable.rs` | Lists the characters OCR confuses with each character. |

    `editor-wasm` gets `ocr_load` and `ocr_job` in `wasm_trace.rs` and the lines input on
    `trace_job`; no logic of its own.

## Not decided here

- Part B (Recognise shapes): its own dated note before it is built (0088 decision list).
- Non-Latin scripts and vertical text: out of scope; a second recognition model would be a new
  licence record and a customer question.
- A mobile v5 detector: not measured; swapping it is a weights change under ADR 0027 §5.
