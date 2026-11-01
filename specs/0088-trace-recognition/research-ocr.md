# Research: offline OCR on the raster before tracing

Architect research spike, 2026-10-31. Docs only; nothing in the workspace was built or changed.
Asked by the customer on 2026-10-31: recognise text on the raster image before and independent of
the trace, fit fonts glyph by glyph (the 0088 Part A matcher verifies), place text objects and
exclude them from the trace (Frage 107 A, 108 A, 109 B: an offline model may be investigated; no
data leaves the machine, `CLAUDE.md` §3).

**Answer in one paragraph.** Yes, OCR runs fully offline in pure Rust, in the desktop build and in
the browser build, from the same crate. The best fit measured here is **PaddleOCR PP-OCRv5 models
(Apache-2.0) run by `rten` (MIT OR Apache-2.0, pure Rust)**: it read every line of the test poster
including ä, ö, ü, ß, Ä, Ö, Ü, needs about 10 MB of weights, takes 0.5 s on the desktop (8 threads)
and 4 to 5 s single-threaded in wasm for a 2000 x 2000 image, and gave the same text and boxes
native and in wasm. `ocrs` (same author as `rten`) is the easiest crate but its model cannot read
umlauts and its weights are CC BY-SA 4.0. Tesseract works offline but is C++, has no Rust wasm
path and complicates packaging. A new crate and ADR 0027 are needed (new crate, new dependency,
shipped weights).

## 1. Spike: what was measured

Throwaway projects in the session scratchpad (`ocr-spike`, `ocr-spike-tract`, `ocr-spike-wasm`,
`ocr-spike-tract-wasm`, `ocr-spike-rtenp-wasm`), toolchain 1.99.0, release builds, wasm with
`+simd128` run in Node 26. Machine: the lead's VM, 8 cores. Test images: one 2000 x 2000 poster
rendered with ImageMagick (Liberation Sans/Serif/Mono, FreeSerif Bold Italic; black text, white
text on a green rectangle, a 20 degree rotated word, a 24 px line, three filled shapes) and a
"scan" variant (blur 1.5 px, Gaussian noise, JPEG quality 60).

Our own Paddle pipeline in the spike is minimal: DB detection at 960 x 960, threshold 0.3,
4-connected components, axis-aligned boxes with DB "unclip" 1.5, recognition at height 48 padded
grey to width 1024, greedy CTC. No angle classifier, no batching.

| | ocrs 0.13.1 (own models) | Paddle via tract 0.23.9 | Paddle via rten 0.26 |
|---|---|---|---|
| Clean poster, 8 lines of text | 7 of 8 lines close; all umlauts wrong ("B?CKEREI", "Miiller & S?hne", "Gr?Re", "??? R"); 6 false lines in the shapes | 8 of 8; only "BACKEREI" lost the Ä (large bold caps) | identical to tract |
| Noisy poster | same umlaut errors, no false lines | 8 of 8, one line lost spaces ("ÄÖüäöüß") | identical to tract |
| 20 degree rotated "TILT" | read | read | read |
| Native time, 2000 x 2000 | 1.15 s wall (load 20 ms, detect 0.79 s, recognise 0.24 s); 1.33 s CPU | 1.35 s wall, 1.3 s CPU, single thread (optimise 0.2 s, detect 0.36 s, recognise 0.74 s) | **0.53 s wall**, 2.0 s CPU (detect 0.12 s, recognise 0.33 s) |
| wasm time (single thread, SIMD) | 3.0 s CPU | 3.4 s CPU | 4.1 to 5.0 s CPU |
| Native vs wasm output | identical text and integer boxes | identical | identical |
| Run to run | identical | identical | identical |
| wasm module (raw / gzip -9) | 4.85 MB / 1.03 MB | 21.1 MB / 3.7 MB | 4.45 MB / 0.93 MB |
| Weights (raw) | 2.5 + 9.7 = 12.2 MB | 2.4 + 7.9 = 10.3 MB | 10.3 MB |

Recognition time in the Paddle rows is dominated by the fixed 1024 width; width buckets and
batching are the obvious first optimisation. "Identical native vs wasm" covers the printed
string and integer pixel boxes on two images, not bit-identical floats: `rten` and `tract` pick
different SIMD kernels per target, so a character near a decision boundary can still differ
between builds (see §4).

## 2. Options

### A. `ocrs` + `rten` (pure Rust)
- Code: MIT OR Apache-2.0 ([crate](https://github.com/robertknight/ocrs/blob/main/ocrs/Cargo.toml)); its whole tree passes our `deny.toml` list (checked with `cargo metadata` in the spike).
- Weights: [ocrs-models LICENSE](https://github.com/robertknight/ocrs-models/blob/main/LICENSE.md): trained only on Google's HierText, so the models are **CC BY-SA 4.0** with an attribution line. Not in our allow-list; share-alike on weights is a new licence question for ADR 0006 §5.
- Languages: default alphabet is ASCII only ([lib.rs](https://github.com/robertknight/ocrs/blob/main/ocrs/src/lib.rs), `DEFAULT_ALPHABET`); README: "currently only the Latin alphabet", early preview, "expect more errors than commercial OCR engines" ([README](https://github.com/robertknight/ocrs)). Measured: no umlauts, no ß.
- wasm: first-class (the crate exports a wasm API; built and run in the spike). Maintained: 0.13.1 on 2026-09-13, commits this week ([CHANGELOG](https://github.com/robertknight/ocrs/blob/main/CHANGELOG.md)).
- Verdict: good engine, wrong model for German. Its value for us is `rten`.

### B. Tesseract via bindings (C++)
- Code Apache-2.0; 5.5.4 released 2026-10-28. Weights [tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) Apache-2.0: `deu` 1.5 MB, `eng` 4.1 MB (`tessdata_best`: 8.6 / 15.4 MB). German is first class.
- Rust: `leptess` 0.14 (last release 2023), `tesseract` 0.15.2 (2025), `tesseract-rs` 0.4 (2026-07, can build Tesseract and Leptonica from source; Windows needs MSVC, CMake, minutes of build) ([tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs), [lib.rs](https://lib.rs/crates/tesseract-rs)). Bindings are MIT.
- Packaging: a C++ library plus Leptonica (and its image libraries) on three desktop OSes, or a static build in CI. No `wasm32-unknown-unknown` path from Rust; the browser would need `tesseract.js` (Emscripten) beside our wasm, a second pipeline.
- Quality on graphics: Tesseract's layout analysis expects documents; logos, text on shapes and rotated words need our own region finding first. Not measured here (no C++ build in this spike).
- Verdict: rejected. Breaks "core builds for wasm32" for the feature, adds native packaging on three OSes, and the browser would get a different engine.

### C. ONNX models via `tract` (pure Rust)
- `tract-onnx` 0.23.9, MIT OR Apache-2.0, released 2026-10-15. Ran PaddleOCR det and rec after fixing the detection input to a concrete shape (symbolic shapes failed in analysis; dynamic recognition width was 6x slower, so a fixed width is needed).
- Single-threaded natively; wasm module 21 MB raw / 3.7 MB gzip; pulls `getrandom` (needs the `wasm_js` backend in wasm) through `rand_distr`.
- Verdict: works, but four times the wasm size of `rten`, slower natively, more setup. Kept as the fallback runtime if `rten` stalls.

### D. ONNX models via `ort` (native ONNX Runtime)
- `ort` 2.0.0-rc.15 (MIT OR Apache-2.0) wraps Microsoft's C++ ONNX Runtime, downloaded as a prebuilt binary at build time by default. Used by `oar-ocr` 0.10 (Apache-2.0, PaddleOCR pipelines) ([docs.rs](https://docs.rs/crate/oar-ocr)).
- Fastest on desktop, but native library on three OSes, no Rust wasm path (the browser would need `onnxruntime-web`), and still a release candidate.
- Verdict: rejected for the same reasons as B.

### E. PaddleOCR models via `rten` (pure Rust) — recommended
- `rten` 0.26/0.27 (MIT OR Apache-2.0) loads ONNX directly (`onnx_format` feature, since 0.24) and ran both Paddle models with no change; output identical to tract.
- Models: PP-OCRv3 mobile detection (2.4 MB) and PP-OCRv5 mobile **latin** recognition (7.9 MB, 32 languages incl. German, dictionary of 502 symbols with ÄÖÜäöüß) from the ONNX conversion [monkt/paddleocr-onnx](https://huggingface.co/monkt/paddleocr-onnx) (Apache-2.0 card). The v5 detection model there is 88 MB (server size); a mobile v5 detector would need its own conversion and check (not measured).
- Licence: upstream [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) is Apache-2.0 and the conversions restate Apache-2.0 for the weights; Paddle does not itemise the training data ([PP-OCRv5 paper](https://arxiv.org/pdf/2603.24373); an open question on model licences in [discussion #18405](https://github.com/PaddlePaddle/PaddleOCR/discussions/18405) has no answer). Weights are data, invisible to `cargo deny`, so they need their own licence record (ADR 0006 §5), like fonts.
- Pure-Rust Paddle pipelines exist (`pure-onnx-ocr` 0.3.2 on tract, Apache-2.0, [docs.rs](https://docs.rs/crate/pure-onnx-ocr/latest)); we would write our own ~300 lines instead (pre/post-processing is small and we need the boxes and per-character confidences in our own units).
- Verdict: recommended. German works, weights are permissive, smallest wasm, fastest native, same code in both builds.

### Others looked at
- `candle` (MIT OR Apache-2.0, pure Rust, wasm-capable) with TrOCR: a transformer recogniser of several hundred MB and no detector; too large for a feature of this size.
- EasyOCR ports (`sceptre`, tract backend): CRAFT detector about 80 MB; no advantage over Paddle.

## 3. Proposed architecture

```text
frontend / curvyo-app      reads the weight files (bundled resource or same-origin static asset): I/O
        | bytes (once per session, into the trace worker)
curvyo-editor-wasm         trace worker: ocr -> region trace -> recognise -> masked trace
        |
curvyo-ocr-core   (new)    pure: weights bytes + RGBA raster -> text lines (quad in px, string, per-char confidence)
curvyo-vectorize-core      trace(raster, pixel_map, params, poll) + an exclusion mask in params
curvyo-text-core           recognise(outlines, tier, tolerance, hint) (0088 Part A matcher)
curvyo-ui-core             trace plan: builds text nodes + remaining paths as one history step
```

1. **Where inference runs.** In a new `curvyo-ocr-core`: weights come in as bytes, the raster as
   the same RGBA view the tracer gets, out come lines in pixel space. No file, network, clock or
   UI. It runs in the existing trace Web Worker (ADR 0023), so desktop and browser share one
   path. The loader is I/O and lives outside: Tauri reads the bundled file in `curvyo-app`; the
   browser fetches a same-origin static asset once (the model comes in; the image never leaves).
   One point the ADR must settle: `rten` brings `rayon` (a thread pool on native; none in wasm).
   In the product it runs only in wasm, where it is single-threaded; on native it runs only in
   the crate's tests. Options for ADR 0027: accept that as the documented exception to "no
   threads" in `*-core`, or name the crate `curvyo-ocr-io` and keep only pre/post-processing in
   core. Recommendation: `-core` with the exception written down, because the crate does no
   I/O and the threads are an internal detail of the dependency.
2. **Finding text regions.** The detector gives word boxes; we group them into lines (as ocrs
   does) and keep a quad per line, possibly rotated. Recognition gives the string and a
   confidence per character from the CTC probabilities.
3. **Glyph fit (reuse of 0088).** Rejected: rasterising candidate glyphs and comparing pixels;
   that needs a rasteriser and a new image distance in text-core, duplicates the matcher and is
   only as precise as the pixel grid. Chosen: **trace each OCR line region separately** with
   `vectorize-core`, with a threshold chosen inside the region (the global threshold problem the
   customer named disappears, because the high threshold applies only there), then run the
   existing `recognise()` on those outlines. The OCR string becomes a **hint**: the shortlist
   (stage 4) is limited to the OCR characters plus their known confusables (l/I/1, O/0, ß/B),
   which makes matching faster and steadier; the font fit, size, baseline and tolerance check
   stay exactly as in Part A and decide. Sub-pixel accuracy comes from the trace's curve fit, as
   today. The text-core change is one optional parameter, no new module.
4. **Masking the trace.** Matched glyph regions are excluded from the main trace: `TraceParams`
   gains an exclusion mask (pixels of the matched glyphs, grown by 1 px), and masked pixels take
   the class of the surrounding region before labelling, so a text on a filled shape leaves the
   shape whole and the text object sits on top. Whether the shape should instead keep
   letter-shaped holes is a customer decision (Frage 113). Lines that were not replaced are not
   masked; their region trace outlines replace the global trace there.
5. **Fallback when no face fits.** As Frage 103 A: replace only within tolerance. The line stays
   as paths from the region trace (better than the global trace), and the notice gives the OCR
   string and the nearest face with its deviation. Placing the OCR string in the nearest face
   anyway is offered as Frage 114 B.
6. **Determinism.** OCR output is only a hint and a region; final geometry comes from our own
   trace and matcher, whose determinism rules (0089 §8) are unchanged. A character that differs
   between native and wasm near a confidence boundary can at most change which shortlist entries
   are tried; the tolerance check still decides. The harness pattern of 0089 (`#[ignore]`,
   numbers only, baseline JSON) is reused for OCR accuracy on our fixtures.
7. **Weights and format.** Weights are not part of any file we write; no `format_version`
   change. They get a licence record next to them (like fonts, ADR 0006 §5) and a SHA-256 check
   on load.

**New crate and ADR: yes.** `CLAUDE.md` §5 requires an ADR for a new crate; this also adds an
external dependency family (`rten`, `rten-tensor`, `rten-imageproc` or our own box code) and
shipped model weights. ADR 0027 "Offline text detection crate, runtime and model weights"
(`needs-customer` for shipping the model and the weights' licence), amending 0011's crate table
and 0023's trace parameters (exclusion mask). The 0088 `adrs.md` trigger in decision 1 ("a later
`ocrs` decision brings model weights and an inference dependency, which belong in their own
crate") fires. The editor wasm grows by about 0.9 MB gzip for `rten`; recorded in
`docs/technical-debt.md`, not a limit (`CLAUDE.md` §3). Weights load lazily on the first trace
with "Recognise text" on.

## 4. Risks
- Paddle's training data is not itemised; the weights' Apache-2.0 rests on the upstream
  repository licence and third-party conversions (Frage 112).
- Large bold capitals lost one umlaut (BÄCKEREI); the matcher's confusable list must include
  A/Ä etc., so the font fit can still pick the right glyph.
- Script and decorative faces read worse than in this test; OCR only proposes, and lines without
  a fit stay paths, so the failure mode is "no text object", not wrong text.
- `rten` is one maintainer's project (active, monthly releases); `tract` is the tested fallback
  runtime for the same ONNX files.

## 5. Questions for the customer

**Frage 110 (OCR-MODELL): Which recognition model ships?**
- A: PaddleOCR mobile models (Apache-2.0, about 10 MB, reads German incl. umlauts), run by `rten`.
- B: `ocrs` models (CC BY-SA 4.0, 12 MB, no umlauts or ß).
- C: no model; text regions are guessed from the traced outlines only (0088 Part A as built).
- Recommendation: A. Default if unanswered: A.

**Frage 111 (OCR-AUSLIEFERUNG): How do the weights reach the user?**
- A: in the desktop installer (+10 MB) and as a static file of the web app, loaded on the first trace with "Recognise text".
- B: downloaded from our server on first use in both builds, then cached (installer stays small; needs network once).
- C: optional add-on the user installs.
- Recommendation: A (works offline from the first start). Default: A.

**Frage 112 (OCR-LIZENZ): Accept weights whose training data is not itemised?**
- A: yes, Apache-2.0 as stated upstream, recorded next to the files with source and checksum.
- B: no; only weights with documented training data (today that means `ocrs`, CC BY-SA, no umlauts).
- Recommendation: A. Default: A.

**Frage 113 (TEXT-AUF-FLAECHE): Text recognised on a filled shape (white "SALE" on a green box).**
- A: the box is traced whole; the text object lies on top.
- B: the box gets letter-shaped holes and the text object lies in them.
- Recommendation: A (the text stays editable without leaving holes). Default: A.

**Frage 114 (OCR-FALLBACK): OCR read a line but no face fits within tolerance.**
- A: the line stays paths (from the region trace); the notice names the read text and the nearest face (as Frage 103 A).
- B: place the read text as a text object in the nearest face anyway.
- Recommendation: A. Default: A.
