# ADR 0075: On-device image models: `rten` in a new crate `curvyo-imageai-core`, three permissive models, the stored result as the source of truth

**Status:** Accepted (PR #447, 2026-11-23; the customer accepted it with the PR). The customer decided the scope (Frage 273 C), the place (274 B, on the device), the download (280 A, on demand from a Curvyo origin behind a CDN, never bundled, ADR 0076) and the training data (281 A, 2026-11-19: models trained on research datasets are accepted, as Frage 112; decision 9). Slice 3 of spec `0135` is not Ready before this ADR, [ADR 0076](0076-model-download-host-and-the-one-network-use.md) and [ADR 0077](0077-derived-image-assets-in-the-project-file.md) are accepted. Spec: [`0135`](../../specs/0135-image-optimisation/specification.md) criteria 47 to 50. Precedent: [ADR 0028](0028-ocr-text-in-trace.md) (PaddleOCR weights on `rten`, now `vectrace::ocr`, ADR 0048).

**In plain words (for the customer).** The three AI tools run inside Curvyo on your computer, with the same engine that already reads text in Trace. Each run's result is saved in the project as a picture, so the burn never depends on the AI again: another computer without the model burns exactly the same dots.

## Context

Remove background, Upscale and AI denoise need a neural-network runtime that runs in the browser build and the desktop webview alike, on the CPU (WebGPU only as an accelerator, criterion 50), in a worker. Model output is not bit-identical across CPUs, but the dots must be (spec criteria 17, 48): so the runtime's result is stored once and the job reads only that.

## Options

**Runtime**

| Option | Why it won or lost |
|---|---|
| **A. `rten` 0.26 (MIT OR Apache-2.0, pure Rust, wasm32 with SIMD) (chosen)** | Already in the lock file and in the editor wasm through `vectrace` (ADR 0028), so the runtime adds about nothing to the wasm; same code native and wasm; ONNX loading; run-to-run identical (ADR 0028 §6). |
| B. ONNX Runtime Web (`onnxruntime-web`, MIT, C++ to wasm, npm) | Broadest operator coverage and a WebGPU backend, but about 10 MB more wasm, a second engine beside `rten`, inference in TypeScript the customer does not review. **Kept as the fallback** if the step-0 times (decision 7) miss by a wide margin. |
| C. `tract` (MIT OR Apache-2.0) | ADR 0028's measured fallback: +3.7 MB gzip wasm, single-threaded natively. Second choice for the same ONNX files. |
| D. `candle` (MIT OR Apache-2.0) | Its ONNX support is partial; each model would be rewritten as Rust code, a port per model. |
| E. WebGPU-only (`wonnx`, `burn` with `wgpu`) | WebGPU is not required by criterion 50 and is missing in parts of WebKitGTK; `wonnx` is unmaintained; `burn` brings a large tree. |

**Crate**

| Option | Why it won or lost |
|---|---|
| **A. New crate `curvyo-imageai-core`: weights and pixels in as bytes, pixels out (chosen)** | Inference does no I/O, clock, network or UI: it is pure compute, which is what `*-core` means (ADR 0011, `CLAUDE.md` §6), and it stays on the wasm32 check. A crate of its own keeps the runtime **off the job path**: `curvyo-laser-core` and `curvyo-image-core` must not depend on it (CI: `cargo tree -i curvyo-imageai-core` names only `ui-core` and `editor-wasm`), which is what spec criterion 50's "a `*-core` crate never contains the runtime" protects. |
| B. A platform crate (`-io`, `curvyo-app`) | The browser build has no Rust platform crate besides the wasm; `curvyo-app` is native only, so the browser would need a second engine. Naming pure compute `-io` hides the real rule bend (threads, decision 5) instead of recording it (ADR 0028's argument). |
| C. A module of `curvyo-image-core` or `ui-core` | image-core is on the job path (laser-core depends on it); ui-core would gain a second responsibility. |
| D. A module of `vectrace` | An external trace library (ADR 0048); image adjustment is not tracing. |

**Models** (licences checked 2026-11-19 against each repository's licence via the GitHub API; sizes are the published PyTorch weights; the weights carry the repository's licence, none states a separate one)

| Step | Chosen | Licence, size | Alternatives and why not first |
|---|---|---|---|
| Remove background | **U²-Netp** (`xuebinqin/U-2-Net`, `u2netp.pth`), 320 x 320 input, soft saliency mask | Apache-2.0, 4.7 MB | IS-Net general (`xuebinqin/DIS`, Apache-2.0, 176 MB: over the 50 MB target; better edges); MODNet (Apache-2.0, 25 MB: portraits only). Rejected: BRIA RMBG 1.4 and 2.0 (non-commercial), BiRefNet (MIT, but about 180 MB and heavy). |
| Upscale | **Real-ESRGAN `realesr-general-x4v3`** (`xinntao/Real-ESRGAN`, compact network), x4; x2 by our integer 2 x 2 box of each x4 tile | BSD-3-Clause, 4.9 MB (4,885,111 bytes) | `RealESRGAN_x4plus` (BSD-3-Clause, 67 MB, slower, sharper). Rejected: SwinIR (Apache-2.0, far over the time target on CPU). |
| AI denoise | **SCUNet `scunet_color_real_psnr`** (`cszn/SCUNet`, Apache-2.0; weights in `cszn/KAIR` releases, MIT): real noise and JPEG blocks | Apache-2.0 / MIT, 72 MB (71,982,841 bytes) | FFDNet colour (`cszn/KAIR`, MIT, 3.4 MB): fast, but trained on Gaussian noise only, no JPEG blocks; the fallback if SCUNet misses the time target. Rejected: DRUNet (MIT, 131 MB), NAFNet (no licence asserted on the repository). |

All three need conversion from PyTorch to ONNX; that conversion is ours (decision 4).

## Decision

1. **Crate** `curvyo-imageai-core`, `#![forbid(unsafe_code)]`, wasm32. One sentence: "runs the three image models on a picture with the weights handed in as bytes". Edges: `→ curvyo-image-core` (the `WorkingImage` and grey plane types), `→ rten` (exact pin of the version `vectrace` uses, `default-features = false` plus ONNX loading; one copy in the tree, checked by `cargo tree -d`), `→ sha2` (workspace). Dependents: `curvyo-ui-core` (session commands) and `curvyo-editor-wasm` (the worker entry). **No new third-party crate**: `rten` and its tree, `sha2` and `rayon` are already locked and allowed by `cargo deny`.
2. **API** (no `rten` type public, no trait, no registry):
   ```rust
   pub enum ModelKind { Background, Upscale, Denoise }
   pub struct ModelFile { pub kind: ModelKind, pub name: &'static str, pub version: &'static str,
                          pub sha256: [u8; 32], pub bytes: u64, pub licence: &'static str }
   pub const MODELS: [ModelFile; 3];                        // the pinned list, decision 4
   pub fn verify(file: &ModelFile, bytes: &[u8]) -> Result<(), AiError>;   // size and SHA-256
   pub struct Model { /* parsed graph */ }
   impl Model { pub fn load(file: &ModelFile, bytes: &[u8]) -> Result<Self, AiError>; } // verify, then parse
   pub fn remove_background(m: &Model, image: &WorkingImage, poll: &mut Poll) -> Result<GreyPlane, AiError>;
   pub fn upscale(m: &Model, image: &WorkingImage, factor: Factor, poll: &mut Poll) -> Result<WorkingImage, AiError>;
   pub fn denoise(m: &Model, image: &WorkingImage, poll: &mut Poll) -> Result<WorkingImage, AiError>;
   pub enum AiError { WrongModel, BadModel(String), TooLarge, Cancelled, Invalid(String) }
   ```
   `Poll` is called between tiles, never on a clock (progress and Cancel, criterion 47). Upscale and Denoise run in fixed 256 px tiles with 16 px overlap in a fixed order (run-to-run identical); the background mask is the raw 320 x 320 output, resized later by the integer step of ADR 0077 §3. `TooLarge` is checked before any work against the working limit of ADR 0017 §4 (spec criteria 47, 66).
3. **Where it runs.** A dedicated worker instance of the editor wasm (the ADR 0023 §5 pattern), so a Run never blocks the page or a trace; the model is loaded once per worker and kept while the worker lives. The worker returns the result pixels; the session encodes the PNG and writes it (ADR 0077). Only the Run button starts inference; editing never does (criterion 47).
4. **Pinned models and conversion.** `curvyo-imageai-core/models.toml` records per model: name, version, upstream repository and file, upstream file SHA-256, licence, copyright line, training data note (decision 9), our ONNX file's SHA-256 and size, opset, and its URL on the model host (ADR 0076 §2). A maintainer converts once with `tools/models/export_onnx.py` (pinned PyTorch, opset 17, static input shapes, weights inline, no external data, no custom operators) and the file is published on the model host with the upstream licence beside it (ADR 0076 §2, §9). No model file is ever shipped in the repository, the installer, the web app or the wasm, and none is loaded before a Run (Frage 280 A). The constants in `MODELS` are generated from `models.toml` and checked by `tests/model_record.rs` (licence in the allow list Apache-2.0, MIT, BSD-2/3-Clause, CC0; hash present; size under the targets of criterion 50). A new model or version is a code change and an app release (spec criterion 64).
5. **Threads.** The ADR 0028 §4 exception extends to this crate: `rayon` may appear in its wasm32 tree only through `rten`; our code spawns nothing, keeps no global state, reads no clock. The ADR 0011 §6 check stays "`cargo tree -i rayon` names only `rten`".
6. **Security.** Bytes are parsed only after `verify` matches the pinned SHA-256, so the only ONNX ever parsed is a file we published; a mismatch deletes the cached file and is never run (criterion 64). ONNX carries no code: no PyTorch pickle (`.pth`) is ever read by the app, custom operator domains and external-data references are refused, the file size is capped at the pinned size. `rten` contains `unsafe` in its SIMD kernels (a vetted, pinned dependency, as in ADR 0028 §4). Output sizes are bounded before allocation.
7. **Step 0 of slice 3** (before the code): convert the three models, load them on the pinned `rten`, and measure on the build host for a 1 Mpx input in wasm with SIMD: targets (criterion 50) Remove background under 5 s, Upscale and AI denoise under 30 s. Record the numbers and hashes in a dated note here. A miss is reported to the lead with the options: the smaller fallback model (FFDNet, `realesr-animevideov3`), `tract`, or ONNX Runtime Web with WebGPU; not worked around silently.
8. **Determinism.** Inference is run-to-run identical but not promised identical across targets. It does not need to be: the stored PNG is the source of truth (ADR 0077 §1), core and job tests use stand-in PNGs (spec criterion 54), and the real models run only in an opt-in test that needs the model files.
9. **Training data (Frage 281 A).** The weights' licences are the repositories' (table above); their training sets are public research collections whose own terms are research use or not itemised: U²-Net on DUTS-TR; Real-ESRGAN on DIV2K, Flickr2K and OutdoorSceneTraining (DIV2K's terms: academic research only, copyright with the original owners); SCUNet on DIV2K, Flickr2K and further research sets. Step 0 confirms each list from the paper and README and writes it into `models.toml`'s training data note, which the About licence list shows (ADR 0076 §9). Accepted by the customer as for OCR (Frage 112): the risk is that a dataset owner's claim reaches the weights; the answer is the takedown path of ADR 0076 §10, and stored results stay in the projects. A model whose repository states a non-commercial or research-only licence for the weights themselves stays excluded (criterion 50).

## Consequences

- One more crate; the runtime is the one already shipped, so wasm growth is our pre- and post-processing only (stated in the PR).
- The model files are not in the repository, the installer or the wasm (ADR 0076); the app's licence list reads `models.toml`.
- A young runtime maintainer (`rten`), as in ADR 0028; ONNX Runtime Web and `tract` stay documented fallbacks for the same files.
- The weights' licences rest on the repositories' statements; their training sets are research datasets, accepted (Frage 281 A, decision 9) and named in the About licence list.
- Amends by notes on acceptance: ADR 0011 §2 (new crate) and §6 (the `rayon` check), ADR 0028 §4 (the exception covers this crate).

## Note 2026-11-21: step 0 measured (implementer; Proposal, for the customer's acceptance)

The three converted files run on `rten` 0.26 with every operator supported; their output differs
from ONNX Runtime 1.32 by at most 1 grey level. On a 1 Mpx picture (`specs/0135` `plan.md`,
"Step 0 measured"): natively on one thread 0.3 s, 16.3 s and 25.8 s; in Chromium through the AI
worker with wasm SIMD (`+simd128`, already on for every wasm build) **2.2 s (Remove background),
143 s (Upscale), 470 s (AI denoise)**, peak 1.3 GB in the worker's process; Cancel 0.2 s; the page
stays responsive. Remove background meets decision 7's target; Upscale misses it 5x and AI denoise
16x.

Lead decision (2026-11-21): the two slow models are rejected (kept in the table above as measured
options) and the decision-7 fallbacks ship: **`realesr-animevideov3`** (BSD-3-Clause, 2.5 MB ONNX)
for Upscale and **`ffdnet-color`** (MIT, KAIR, 3.4 MB ONNX, the noise level fixed at 15/255 in the
file; Gaussian noise only, it does not remove JPEG blocks) for AI denoise. Both run every operator
on `rten` 0.26, 1 grey level from ONNX Runtime; on 1 Mpx natively 9.5 s and 2.5 s, **in Chromium
74 s and 74 s**, peak about 1 GB, Cancel 0.2 s. A 128 px tile would not help (more overlap).

Proposal (next step, each its own ADR): the fallbacks miss the 30 s target 2.5x on one wasm thread.
The cheapest gain is **`rten` on wasm threads**: `rten` already parallelises natively through
`rayon`; in wasm that needs `SharedArrayBuffer`, i.e. the web app and the desktop webview served
cross-origin isolated (`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy:
require-corp`, the model host's files with `Cross-Origin-Resource-Policy: cross-origin`), a wasm
build with atomics (`build-std`, nightly or a target feature set) and a thread pool in the worker
(`wasm-bindgen-rayon`); on 4 to 8 cores this should reach the target. The alternative is ONNX
Runtime Web with WebGPU (a second runtime, about 10 MB of wasm, inference in TypeScript, WebGPU
not everywhere in WebKitGTK).

## Note 2026-11-21: tiles on a pool (implementer, ADR 0078 accepted as a Proposal)

§2 gains the tile functions of ADR 0078 §2 (`tile_count`, `upscale_tile`, `denoise_tile`,
`assemble`, `TileOutput`); `upscale` and `denoise` are a loop over them, pinned by hash to the
output before the change. §3: Upscale and AI denoise run their tiles on a pool of AI workers
(worker 0 plus `min(cores - 1, 8, 1 GiB / M) - 1` extras, made per Run), each the editor wasm with
its own memory; Remove background stays one run on worker 0. Measured in `specs/0143` `plan.md`.

## Note 2026-11-23: what slice 3 built (architect review; for the customer's acceptance with the PR)

**In plain words (for the customer).** Two of the three AI models changed after measuring them in
the browser: the first choices for Upscale and AI denoise took 2.5 and 8 minutes on a 1 megapixel
picture. Upscale now uses the smaller Real-ESRGAN model trained on anime video frames
(`realesr-animevideov3`, BSD-3-Clause) and AI denoise uses FFDNet (`ffdnet-color`, MIT), which
removes grain-like noise but **not JPEG blocks**. Remove background is unchanged (U²-Netp). The
three files together are about 10.5 MB instead of about 82 MB. On an 8-core machine Upscale
takes about 20 s and AI denoise about 7 s (ADR 0078); on a one- or two-core machine about 80 s
and 27 s. Both new models were trained on research data, like the first ones (Frage 281 A).

What changes against the text above:
- Models table and decision 9: Upscale is `realesr-animevideov3` (upstream release `v0.2.5.0`,
  training data: anime video frames with synthetic degradations), AI denoise is `ffdnet-color`
  (`cszn/KAIR` release `v1.0`, noise level fixed at 15/255 inside the ONNX file; training data
  BSD400, ImageNet validation and Waterloo Exploration with Gaussian noise). `models.toml` holds
  the pins, upstream hashes and training data notes.
- Decision 1: the only dependent built is `curvyo-editor-wasm` (the session commands live in the
  editor session, not in `curvyo-ui-core`); the CI job `imageai-boundary` allows both. Direct
  edges are `rten` and `rten-tensor` (both `=0.26.0`, already locked through `vectrace`), no new
  third-party crate. When `vectrace` moves to another `rten`, both pins move in the same change.
- Decision 2: the progress callback is `&mut dyn FnMut(done, total) -> bool`, not a `Poll` type;
  `AiError::TooLarge` carries the result size; the tile functions are ADR 0078 §2.
- Decision 4: `models.toml` and `src/models.rs` are kept equal by hand and checked by
  `tests/model_record.rs` (no generator); the conversion is `tools/models/export_onnx.py`, the
  maintainer's steps `docs/runbooks/ai-models.md`.
