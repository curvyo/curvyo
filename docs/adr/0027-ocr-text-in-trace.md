# ADR 0027: Offline text recognition in Trace (crate, runtime and model weights)

**Status:** Proposed (architect, 2026-11-01). Its `needs-customer` parts (a shipped model, the
weights' licence, the download size, data leaving the machine) are already answered by the
customer on 2026-10-31: Frage 109 B (an OCR model is allowed), 110 A (PaddleOCR mobile ONNX
weights on `rten`, our own pre- and post-processing, no Paddle code), 111 A (default, unanswered:
weights bundled in installer and web app, no download), 112 A (accept the Apache-2.0 weights
without itemised training data, recorded in the licence list), 113 A, 114 B (feature behaviour,
`specs/0091-trace-text-recognition/adrs.md`). Becomes Accepted when the first slice's spike step
(§7) confirms the measured numbers and the customer accepts that PR. Asked by
`specs/0091-trace-text-recognition` (customer Frage 107 A, 108 A). Research and measurements:
`specs/0088-trace-recognition/research-ocr.md`.

## Context

The customer wants text on a raster image recognised during Trace, before and independent of the
outline trace, set as text objects in the nearest font, and left out of the traced paths. The
0088 recogniser (glyph-outline matching, `curvyo-text-core::recognise`) fits fonts well but has to
guess where the text is and what it says from a global trace whose threshold is often wrong for the
text. A text detector and reader answers both. Hard to change later: the crate boundary, a
runtime that must run identically in the browser and the desktop webview, and shipped model data
with its own licence. No file format we write changes.

| Option | Why it lost or won |
|---|---|
| **E. PaddleOCR mobile ONNX weights on `rten` (pure Rust), our own pipeline (chosen, Frage 110 A)** | Reads German incl. ÄÖÜäöüß; weights Apache-2.0, about 10.3 MB; `rten` MIT OR Apache-2.0, builds for wasm32; 0.53 s native (8 threads), 4 to 5 s wasm single-threaded for 2000 x 2000 px; editor wasm +0.93 MB gzip; same text and integer boxes native and wasm, run to run identical (spike). |
| A. `ocrs` (same author as `rten`) with its own models | Easiest API, but no umlauts or ß (ASCII alphabet) and weights CC BY-SA 4.0, outside our allow-list. |
| C. PaddleOCR weights on `tract` | Same output, but editor wasm +3.7 MB gzip, single-threaded natively, needs a fixed recognition width. Kept as the fallback runtime for the same ONNX files. |
| B. Tesseract (C++), D. `ort` (ONNX Runtime, C++) | Native libraries on three OSes, no Rust wasm32 path; the browser would need a second engine (`tesseract.js`, `onnxruntime-web`). Breaks "same code in both builds" (ADR 0023 §5). |
| F. No model: find text regions from traced outlines only (0088 Part A) | No new crate or data, but the global threshold problem stays and the reading is a guess. It stays as the selection-based Recognise text tool. |
| Crate `curvyo-ocr-io` with only pre/post-processing in core | The inference does no I/O; naming it `-io` hides the real rule bend (threads) instead of recording it, and puts pure numeric code outside the wasm check. |
| A module of `vectorize-core` or `text-core` | Both are in the editor wasm already, but each would gain a heavy dependency and a second responsibility; 0088 decision 1 named this exact trigger for a crate of its own. |

## Decision

### 1. Crate and edges

New crate **`curvyo-ocr-core`**, `#![forbid(unsafe_code)]`, builds for `wasm32-unknown-unknown`.
Responsibility: it reads text lines from an RGBA raster with model weights handed in as bytes.
Edges: `curvyo-ocr-core → (external only)`: `rten` and the `rten-*` crates it needs, `sha2`
(workspace). No `document-core`, no `vectorize-core`: input is `(width, height, &[u8])`, output is in
pixel space. `ui-core → ocr-core`; `editor-wasm` reaches it only through `ui-core` (as the tracer,
ADR 0023 §1). `rten` is pinned exactly (`=0.x.y`, the version the step-0 spike confirms) with
`default-features = false` plus ONNX loading; `tract` is the documented fallback for the same files.

### 2. Public API (minimal, no `rten` type)

```rust
pub struct OcrModel { /* parsed detector, recogniser, alphabet */ }
impl OcrModel {
    pub fn load(detector: &[u8], recogniser: &[u8], alphabet: &str) -> Result<Self, OcrError>;
}
pub fn read_lines(model: &OcrModel, width: u32, height: u32, rgba: &[u8],
                  poll: &mut dyn FnMut(OcrProgress) -> ControlFlow<()>) -> Result<Vec<TextLine>, OcrError>;
pub struct TextLine { pub quad: [[f32; 2]; 4], pub text: String, pub confidence: Vec<f32> } // one per char
pub enum OcrError { WrongModel, BadModel(String), Cancelled, Invalid(String) }
```

`load` checks each file's SHA-256 against constants in the crate (§5) before parsing; a mismatch is
`WrongModel`. `poll` is called between stages and between lines, never on a clock. No trait, no
generic, no model registry: one model, one function.

### 3. Pipeline (ours, about 300 lines, no Paddle code)

Detection (DB, the spike's settings as the start: input scaled to 960 px, threshold 0.3,
4-connected components, unclip 1.5) gives word boxes; boxes are grouped into lines with a quad each
(rotated text allowed); recognition (height 48, width buckets instead of the spike's fixed 1024)
with greedy CTC gives the string and a per-character confidence. The exact constants are Proposals
the harness of `0091` tunes.

### 4. Threads: the documented exception to "no threads in `*-core`"

`rten` brings `rayon`. In the product the crate runs only in wasm (the trace worker, ADR 0023 §5),
where `rayon` runs everything on the calling thread; natively it runs only in tests. The crate itself
spawns nothing, keeps no global state and reads no clock. **Exception, scoped:** `rayon` may appear
in the wasm32 dependency tree of `curvyo-ocr-core` and of the crates that depend on it (`ui-core`,
`editor-wasm`) **only through `rten`**; the ADR 0011 §6 ban-list check becomes
"`cargo tree -i rayon` names only `rten` as a dependent". Any other path to `rayon`, or a
thread spawned by our code, still fails CI. `rten` contains `unsafe` (SIMD kernels); like
`visioncortex`'s leaves it is a vetted, pinned dependency, not our `unsafe`.

### 5. Weights: shipped data with a licence record

The detector (PP-OCRv3 mobile, 2.4 MB) and the Latin recogniser (PP-OCRv5 mobile latin, 7.9 MB,
502-symbol alphabet) in the ONNX conversion `monkt/paddleocr-onnx`, unmodified, live in
`frontend/public/models/paddleocr-latin/` with the Apache-2.0 text and a `model.toml` (`name`,
`licence = "Apache-2.0"`, `source_url` for the conversion and upstream PaddleOCR, upstream model
names, `sha256` per file, `training_data = "not itemised upstream; accepted by the customer, Frage
112 A, 2026-10-31"`). This is ADR 0006 §5 data, like fonts (ADR 0020 §4): `cargo deny` does not see
it, so `curvyo-ocr-core/tests/model_record.rs` checks that every file has the record, an allowed
licence and the hash the crate expects. The app's licence list reads `model.toml`. Changing the
weights is a code change (new constants, new harness baseline), never a silent file swap.

**Loading (Proposal, one path):** in both builds the frontend fetches the three files same-origin
on the first trace with Recognise text on and posts them to the trace worker, which calls
`OcrModel::load` once per worker. The desktop installer carries them because Tauri embeds the
frontend's static files in the bundle (Frage 111 A, +10 MB, no download, nothing leaves the
machine). Rejected: reading a Tauri resource in `curvyo-app` and passing it through IPC, a second
loader for the same bytes the webview can already fetch.

### 6. Determinism

Run to run: identical (spike, both runtimes). Native and wasm: same strings and integer boxes on
the spike images, but not promised bit-identical (`rten` picks SIMD kernels per target). As in
ADR 0023 §4 the preview and Create paths both run in the wasm worker, so the user gets what he
previewed; golden files are native. The OCR result is only a region, a mask and a hint (`0091`
adrs.md decisions 2 to 4); the final geometry comes from our tracer and the 0088 matcher under
their existing rules (0089 adrs.md §8, 0088 adrs.md decision 7).

### 7. Step 0 of the first slice (before the code is written)

Build `curvyo-ocr-core` with the pinned `rten` in the workspace and confirm: wasm32 build and the
amended ban-list check pass; `cargo deny` passes; no `getrandom`; editor wasm growth near 0.93 MB
gzip; wasm OCR on the 2000 x 2000 fixture within about 5 s with `+simd128` (and measured without
it). A large miss is reported to the lead with the `tract` option, not worked around.

## Consequences

- One more crate and a heavy pinned dependency with `unsafe` inside; one young maintainer
  (`rten`), with `tract` tested as a fallback runtime for the same weights.
- Installer and web app +10.3 MB of weights; editor wasm +0.93 MB gzip for code the main thread
  loads but does not run. Recorded in `docs/technical-debt.md`, no limit (`CLAUDE.md` §3).
- A scoped exception to "no threads in `*-core`" (§4) and to the ADR 0011 §6 ban list.
- First OCR of an image costs 4 to 5 s in wasm; cached per image for the session (`0091` adrs.md
  decision 1).
- The weights' licence rests on the upstream statement; training data is not itemised (accepted,
  Frage 112 A). Script and decorative faces read worse; a wrong reading is caught by the font fit
  or marked (`0091` decision 5).
- Amends by dated notes: ADR 0011 §2 (new crate) and §6 (ban list), ADR 0023 §5 (the worker job
  gains a text pass and an in-painted raster; `trace()` and `TraceParams` unchanged, `0091`
  decisions 2 and 3), `specs/0088-trace-recognition/adrs.md` decision 1 (its trigger fired).

## Note 2026-11-01: wasm SIMD (lead decision, `0091` slice 1)

The wasm build enables `+simd128` for every `wasm32-unknown-unknown` crate
(`.cargo/config.toml`), so `rten` uses its SIMD kernels in the trace worker. Support checked:
WebKitGTK 2.54.1 (the desktop webview; its `jsc` validates a SIMD module), Chromium 154 (V8),
Firefox since 89 (MDN; 159 here). Measured on the 2000 x 2000 reference poster in wasm (Node,
thread CPU): OCR 5.6 s without SIMD, 3.0 s with it; weights load 0.19 s. Native and wasm give the
same strings, the same integer quads and bit-identical trace results (plain and with the text
pass) on that poster. The step-0 check of §7 is thereby met.
