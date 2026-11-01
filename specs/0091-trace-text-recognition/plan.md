# Plan for Trace text recognition, slice 1 (one colour, end to end)

Implementer, 2026-11-01, on `story/trace-ocr` (from `main` `9d990ebf`). Built against
`specification.md` slice 1 (criteria 1 to 22), `adrs.md` decisions 1 to 10, ADR 0027 and
`ux-notes.md`. Everything marked **(Proposal)** is mine and vetoable at the demo.

## Affected crates/modules

- **`curvyo-ocr-core`** (new, ADR 0027): `model.rs` (SHA-256 check, parse, alphabet), `detect.rs`
  (DB detector at 960 px, components, oriented boxes), `lines.rs` (word boxes to lines with a quad),
  `read.rs` (crop along the quad, width buckets, greedy CTC with per-character confidence),
  `resize.rs` (bilinear sampling, our own, integer-free but fixed order), `lib.rs` (`read_lines`,
  `poll`). `rten =0.26.0` without default features plus `onnx_format`; `sha2` (workspace).
  Tests: `tests/model_record.rs` (criterion 15), `tests/reference.rs` (the reference set; ignored
  harness numbers).
- **`frontend/public/models/paddleocr-latin/`**: `det.onnx`, `rec.onnx`, `dict.txt`, `LICENSE`
  (Apache-2.0 text), `model.toml`.
- **`curvyo-text-core`** `recognise/`: `confusable.rs` (look-alikes), `Recogniser::with_hint`
  (shortlist limited to the hint and its look-alikes), `fit.rs` (`Recogniser::fit_text`, 114 B).
- **`curvyo-ui-core`**: `trace_text.rs` (accepted lines, per-line Otsu and polarity, region trace,
  mask), `trace_inpaint.rs` (ring median), the text section of the trace wire
  (`trace_transfer.rs`), the job with lines, the mask with left-out pixels; `text_recognise_job.rs`
  gains hinted lines and the nearest face per line; `trace_plan.rs` gains the text objects.
- **`curvyo-editor-wasm`**: `wasm_trace.rs` (`ocr_load`, `ocr_job`, lines on `trace_job` and
  `trace_mask_job`), `session/trace_text.rs` (OCR lines per asset, the font fit stepped like 0088,
  Create paths with texts in one step, the guessed record).
- **`frontend`**: Recognise text toggle in Advanced, the weights fetched same-origin and posted to
  the worker, the OCR job with "Reading text" progress and stop, the fit stepped with face loading,
  the guessed-letters warning, the Mask's grey tone, the status line count.
- Docs: `docs/design-system.md`, `docs/tracing-images.md`, `docs/technical-debt.md`, licence list,
  CI ban-list check (rayon only through rten), `deny.toml` exception for the new crate.

## Tasks

1. [x] `curvyo-ocr-core` with weights, record and record test; native read of the poster
   (criteria 1, 2, 8, 13, 15, 20).
2. [x] text-core: confusables, hint, `fit_text` (criteria 3, 4, 9).
3. [x] ui-core: text pass, in-painting, wire text section, mask grey (criteria 6, 7, 10, 12, 18).
4. [x] ui-core and session: hinted tier search, 114 B, Create paths with texts in one step,
   guessed record and warning (criteria 4, 5, 6, 9, 10, 11, 13).
5. [x] editor-wasm and worker: `ocr_load`, `ocr_job`, lines into `trace_job` (criteria 14, 16, 19).
6. [x] frontend: toggle, OCR cache per image, progress "Reading text", Mask grey, warning
   (criteria 5, 16, 17, 19).
7. [x] Reference set and numbers (criteria 1, 2, 3, 20 to 22).
8. [x] Docs, CI ban list, deny; full gate.

## Validation

Unit tests per module (in-painting, polarity, wire round trip, confusables, hint); an ignored
reference test over the synthetic posters (rendered from included OFL fonts) reporting lines exact,
umlauts, wrong sure letters, times; e2e for the toggle and the Create paths step.

## Results (2026-11-01)

- Reference set (`session::trace_text_tests`, release): poster 8 of 8 lines exact, noisy poster
  8 of 8, rotated line exact and within 1 degree; 0 wrong sure letters; the Liberation Mono line
  is set in the nearest face, all guessed (114 B); 5 whole shapes, no letter left as a path.
- OCR native 0.6 to 0.9 s; wasm (no SIMD, Node) load 0.18 s, OCR 5.6 s, text pass with trace
  0.29 s, peak linear memory 178 MB; editor wasm +0.84 MB gzip; font fit 4.6 to 6.3 s native.
- Deviations (Proposal, recorded in `docs/technical-debt.md`): the font fit runs at Create paths;
  the preview draws the lines' own outlines where the texts will be. The detector reads at most
  960 px; the reader pads lines to 1024 px (better than tight buckets on the reference set).

## Round 2 (2026-11-01)

- Font fit: with an OCR hint 3 faces per tier and 3 characters per candidate go on to the exact
  distance (were 8 and 4), letters reuse the glyphs scaled for their trials, and a hinted line
  with fewer than half its letters sure skips the direction refits. Poster fit 1.4 to 1.5 s thread
  CPU (was 4.6 to 12 s), same accuracy. The fit now runs after each full-size preview, so the
  Trace view shows the texts in their faces; Create paths reuses it.
- Wasm SIMD on (ADR 0027 note): OCR 3.0 s in wasm (was 5.6 s); native and wasm bit-identical on
  the poster.

## Round 3 (2026-11-01)

- Renumbered from 0090 (main's `grid`) to 0091.
- The font fit runs in the trace worker (`curvyo-ui-core::trace_fit`, `curvyo-editor-wasm`
  `trace_fit_glue`): the session writes the request (lines and the tiers' faces by file), the
  worker asks for the faces it needs, the page reads them and hands them in, the worker returns
  the texts with their glyphs; the session keeps them for the preview and Create paths. The OCR
  hands the full picture back to the page for the trace.
- Main thread on the poster in Chromium (dev server, from opening Trace to the fitted preview):
  2 to 3 tasks over 50 ms, longest 95 to 112 ms, 181 to 217 ms in all; with Recognise text off
  2 tasks, longest 93 ms, 146 ms. The CPU profile shows no OCR or fit on the main thread; the long
  tasks are React development rendering and the existing reduced copy of the picture.

## Round 4 (2026-11-01, after the tester's FAIL)

- [x] c9: the crop of a line is moved to its place, then mapped (`pixel_map.compose(translation)`;
  `compose` applies its right operand first). Unit test in `trace_text.rs`; the native golden of
  the tester (`acceptance_0091_tester_native.json`) re-blessed.
- [x] c12/c18 (lead decision 1): the fit sets a line in the nearest face only when some face gave
  it a sure letter and it has more than one letter; otherwise it stays traced shapes. The session
  keeps the strings of such lines per picture; the lines handed to the next trace no longer
  accept them, and the host traces once more, so the paths equal a trace without text (dot grid:
  80,000 nodes again, ring: no text). Test `a_line_without_a_sure_letter_stays_traced_shapes`.
- [x] Lead decision 3: the Recognise text switch is not drawn with Multicolour.
- [x] c19: two races between Stop and the read: a stop while the weights load (no worker job to
  cancel yet) let the old read cache its lines, and the old read's `finally` cleared the "Reading
  text" label of the Retry's read. The read checks `stale` after each await and only the newest
  read clears the label. 15 of 15 runs pass.
- [x] `0025` c54: a worker that dies during the read fails the trace (Failed, Retry reads again)
  instead of tracing without text.
- [x] UX notes §5: the warning and the failed notice live in `curvyo-ui-core::trace_text_notice`.
