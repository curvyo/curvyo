# ADR 0075: On-device image models: `rten` in a new crate `curvyo-imageai-core`, three permissive models, the stored result as the source of truth

**Status:** Proposed (architect, 2026-11-19), `needs-customer` (platform and dependency, model licences: `CLAUDE.md` §3; spec criterion 63). The customer decided the scope (Frage 273 C), the place (274 B, on the device), the download (280 A, on demand from a Curvyo origin behind a CDN, never bundled, ADR 0076) and the training data (281 A, 2026-11-19: models trained on research datasets are accepted, as Frage 112; decision 9). Slice 3 of spec `0135` is not Ready before this ADR, [ADR 0076](0076-model-download-host-and-the-one-network-use.md) and [ADR 0077](0077-derived-image-assets-in-the-project-file.md) are accepted. Spec: [`0135`](../../specs/0135-image-optimisation/specification.md) criteria 47 to 50. Precedent: [ADR 0028](0028-ocr-text-in-trace.md) (PaddleOCR weights on `rten`, now `vectrace::ocr`, ADR 0048).

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
