# ADR 0078: AI tiles on a pool of separate wasm workers, no shared memory and no cross-origin isolation

**Status:** Proposed (architect, 2026-11-21), `needs-customer` (platform: how the wasm runs in parallel and whether the web app and the desktop webview change their hosting headers; `CLAUDE.md` §3). Customer decision so far: Frage 286 B (threads first, then slice 3 of `0135`). Open: **Frage 307** (`specs/0143-ai-tile-pool/adrs.md`), default A = this ADR's decision. Spec: [`0143`](../../specs/0143-ai-tile-pool/specification.md). Extends [ADR 0075](0075-on-device-image-models-runtime-and-crate.md) §2 (tile functions) and §3 (a pool of AI workers); keeps [ADR 0023](0023-raster-trace-crate-pipeline-and-worker.md) §5 ("no COOP/COEP") and ADR 0075 §5 (`rayon` only through `rten`) as they are.

**In plain words (for the customer).** Curvyo cuts the picture into 25 squares anyway. Instead of teaching the AI engine to share one memory between threads, which would force the web demo and the desktop app to change how they are served, Curvyo starts several independent copies of the engine, one per processor core, and gives each its own squares. The result is the same picture, to the last pixel, as with one copy. Nothing about hosting, GitHub Pages, the desktop webviews or the toolchain changes; the cost is memory (estimated 150 to 250 MB per copy while a step runs; measured 2026-11-21: about 65 MB per copy, at most about 540 MB more for the whole tab on 8 cores). Measured on an 8-core machine: Upscale about 20 s and AI denoise about 7 s on a 1 megapixel picture, instead of about 80 s and 27 s with one copy.

## Context

On the reference picture (1 Mpx, spec `0143`) Upscale (`realesr-animevideov3`) and AI denoise (`ffdnet-color`) take 74 s each in Chromium on one wasm thread (8-core build host, `specs/0135` `plan.md`, 2026-11-21); the target is 30 s (`0135` criterion 50). Remove background takes 2.2 s and is not tiled. Both slow steps run the model on **25 independent tiles** (256 px input, 16 px overlap, 224 px core; each output pixel comes from exactly one tile's core, no blending: `curvyo-imageai-core::tiles`), about 3.0 s per tile. Cancel terminates the AI worker (0.18 s).

`rten` 0.26 threads through `rayon` (`rten/src/threading.rs`): its own pool is built with `std::thread`, which wasm32-unknown-unknown refuses, so it falls back to running every operator on the calling thread; `rayon-core` 1.13 does the same for the global pool (`registry.rs`, "unsupported" falls back to the current thread). Threads inside one instance therefore need a wasm build with `atomics` (shared memory, only by `-Z build-std` on nightly) and a `rayon` pool whose threads are Web Workers (`wasm-bindgen-rayon`). Shared memory needs a `SharedArrayBuffer`, which the browser only gives a page that is cross-origin isolated (COOP `same-origin` plus COEP `require-corp` or `credentialless`).

The repository already runs a pool of separate instances: OCR reads text lines on up to four extra trace workers, each its own wasm instance with its own memory (`frontend/src/lib/ocrPool.ts`, `0091`), on every target including the desktop webviews.

## Options

| Option | Speed on 1 Mpx (estimate, build host 8 cores; 4-core laptop) | Why it won or lost |
|---|---|---|
| A. Shared memory in the editor wasm: the whole module built with `atomics` on a pinned nightly, `wasm-bindgen-rayon` as `rten`'s pool | ~15 s; ~25 s (operator-level split of each convolution, about 60 % efficient) | Needs isolation everywhere (below). The editor itself would need shared memory, so a page that is not isolated cannot start the editor at all: a second, non-atomic build is needed anyway as the fallback. Two builds of the largest artefact, nightly on the main product path. |
| B. Shared memory in a second, AI-only wasm (`curvyo-imageai-wasm`, new crate) with `atomics` and `wasm-bindgen-rayon` | as A | Same isolation and nightly cost, smaller blast radius. A second copy of `rten` in the download (estimate +2 to 3 MB uncompressed, +0.8 to 1 MB gzip), a new crate, a new dependency (`wasm-bindgen-rayon`, MIT OR Apache-2.0), a second wasm-bindgen output with its own worker bootstrap in Vite. Lower memory than C (one copy of the weights and buffers). The only option that would also speed up an untiled model (Remove background, already 2.2 s). |
| **C. Tile per instance (chosen):** N AI workers, each the existing editor wasm with its own memory, a coordinator hands out tile indices, one worker assembles | **~17 s** (6 workers, 5 rounds of ~3.0 s, +15 % for shared memory bandwidth); **~20 to 28 s** (4 physical cores, 6 workers with SMT) | No `SharedArrayBuffer`, so **no cross-origin isolation, no header, no service worker, no nightly, no new crate, no new dependency, no wasm size change**. Tiles are independent, so it scales almost linearly and the result is bit-identical to one worker by construction. Costs memory per worker and a coarse grain (25 tiles: 25/6 leaves a short last round). |
| D. Native inference in the desktop app (`curvyo-imageai-core` in a Tauri command, `rayon` on real threads) | desktop only: native single thread is already 9.5 s and 2.5 s, threaded about 2 s and 1 s | Fastest by far, but a second run path (progress, Cancel, model cache) and nothing for the browser, which needs C anyway. Deferred: can sit beside C later (spec `0143` out of scope). |
| E. ONNX Runtime Web with WebGPU | unknown, GPU-dependent | A second runtime (~10 MB wasm, inference in TypeScript), WebGPU missing in parts of WebKitGTK (ADR 0075 option B). Out of scope (spec `0143`). |

**What isolation (A, B) would cost**, audited 2026-11-21 (`frontend/index.html`, `frontend/src`, `curvyo-app/tauri.conf.json`, `.github/workflows/pages.yml`, `docs/deploy-demo.md`):

- **GitHub Pages cannot set response headers.** The demo would need either `coi-serviceworker` (a service worker that re-serves every response with COOP/COEP: the first visit reloads once, private windows that block service workers stay single-threaded, and the app gains its first service worker, which also intercepts every fetch and complicates updates) or a move of the demo to a host with a headers file (Cloudflare Pages, Netlify): a hosting decision. The dev and preview servers would need the headers in `vite.config.ts`.
- **What would break under COEP / COOP:** today nothing measurable: page, scripts, wasm, workers, fonts, OCR weights are same-origin; the sync relay, own-devices sync, key backup and the connector all use `WebSocket`, which COEP does not touch; pictures are `blob:`/`data:`; `window.open` in `platform/host.ts` already uses `noopener`. The model fetch (ADR 0076) is a `cors` fetch and passes with its CORS answer (plus `Cross-Origin-Resource-Policy: cross-origin` as a safety net). **Future cost:** every later cross-origin `<img>`, `<script>`, iframe or no-cors resource (a remote asset library's thumbnails, ADR 0007; a font CDN; embedding the demo in `curvyo.org`) needs CORP or CORS; COOP `same-origin` breaks popup-based sign-in flows (accounts, payment) that rely on `window.opener`. `credentialless` eases images but is not in Safari or WKWebView.
- **Desktop webviews:** Tauri 2 can add COOP/COEP to its asset responses (`app.security.headers`). WebView2 (Chromium) should then grant `SharedArrayBuffer`; WKWebView supports it in Safari 15.2 and later, but isolation of the custom `tauri://` scheme is not verified; WebKitGTK's support depends on version and build and is unverified. Each needs a manual run, and a webview that fails stays single-threaded at 74 s.
- **Toolchain and CI:** a second pinned nightly with `rust-src` beside `rust-toolchain.toml`'s `1.99.1`, `-Z build-std=std,panic_abort` with `-C target-feature=+atomics,+bulk-memory,+mutable-globals`, a second wasm build in `ci.yml`, `pages.yml` and `wasm-size.yml` (estimate +3 to 6 minutes per run, std rebuilt per profile), Renovate bumps of the nightly, and the risk of a nightly regression blocking the demo.

C needs none of this. Its one real cost is memory, bounded below.

## Decision

1. **Option C.** Upscale and AI denoise run their tiles on a pool of N AI workers. Each worker is a dedicated Web Worker instantiating the same `curvyo-editor-wasm` module (posted `WebAssembly.Module`, ADR 0023 §5) with its own memory, the model loaded once per worker (2.5 and 3.4 MB of weights). No `SharedArrayBuffer`, no `atomics` build, no COOP/COEP, no service worker, no change to `pages.yml`, `vite.config.ts` or `tauri.conf.json`. `rten` stays single-threaded per instance (ADR 0075 §5 unchanged). Remove background stays one run on one worker.
2. **Core API** (`curvyo-imageai-core`, extends ADR 0075 §2; no trait, no `rten` type public):
   ```rust
   pub fn tile_count(width: u32, height: u32) -> u32;
   pub struct TileOutput { pub index: u32, pub width: u32, pub height: u32, pub rgba: Vec<u8> } // the tile's core, output pixels
   pub fn upscale_tile(m: &Model, image: &WorkingImage, factor: Factor, index: u32) -> Result<TileOutput, AiError>;
   pub fn denoise_tile(m: &Model, image: &WorkingImage, index: u32) -> Result<TileOutput, AiError>;
   pub fn assemble(width: u32, height: u32, tiles: Vec<TileOutput>) -> Result<WorkingImage, AiError>; // output size; each index once
   ```
   `upscale` and `denoise` become a loop over these plus `assemble`, so one worker and N workers run the same code. Upscale x2 halves inside `upscale_tile` (each core is a multiple of 896 output px, so the 2 x 2 boxes never cross a core and the result equals halving the assembled x4 picture; a test checks it); the x4 tiles never leave the worker, which cuts the transfer and the assembler's memory by four. `assemble` refuses a missing, doubled or wrongly sized tile with `AiError::Invalid`.
3. **Coordinator** (TypeScript, beside `ocrPool.ts`, no React, node-tested): posts the input picture once per Run to each worker, hands the next tile index to whichever worker is idle (dynamic, so a slow core does not hold the run), transfers each `TileOutput`'s buffer to the assembling worker (worker 0, today's AI worker), and posts progress as tiles done of `tile_count`, at most 10 times a second. The order of finishing never matters: `assemble` places by index. The result is then stored exactly as today (ADR 0077).
4. **Pool size** (Proposal, within Frage 302): `N = min(cores - 1, 8, floor(1 GiB / M))`, at least 1, where `cores` is `navigator.hardwareConcurrency` and `M` is the measured peak memory of one AI worker during a tile (step 0 below; expected 150 to 250 MB, so N is 4 to 6 on most machines). The pool is created on the first tiled Run, never at app start. A worker that fails to start (instantiation or memory refused) is dropped and the Run goes on with the rest; with none left beyond worker 0 the Run is today's single-worker path. The extra workers are terminated when the Run ends (Proposal; spec criterion 6's 60 s idle timer applies to worker 0).
5. **Cancel** terminates every pool worker, as today's one (ADR 0023 §5 pattern): expected about 0.2 s, no partial result stored. The next Run builds the pool again (instantiation from the cached module plus model parse, estimate under 0.5 s in parallel).
6. **Determinism.** Each tile is computed by the same single-threaded engine in the same binary on the same machine, so the result is **bit-identical** for any N (ADR 0028 §6). Tests: native `curvyo-imageai-core` (stand-in model) that tiles in shuffled order through `assemble` equal `upscale`/`denoise`; frontend coordinator test with fake workers that N = 1 and N = 3 give the same tile set.
7. **Fallback.** None needed by the platform: dedicated workers exist on every target (browser, WebView2, WKWebView, WebKitGTK; the OCR pool runs there today). N = 1 (one or two logical cores, or a memory refusal) is the 74 s path of today, with the "Running on one core" line of spec `0143` criterion 10.
8. **Step 0 of the story** (before the code, a throwaway spike on `spike/ai-tile-pool`, never merged): run the 25 tiles on N = 1, 2, 4, 6, 8 plain workers in Chromium and Firefox on the build host; record wall time, summed worker CPU time and the peak memory per worker `M` as a dated note here. If 6 workers on the build host miss 30 s, report to the lead with B as the next option (its cost is listed above); not worked around silently.
9. **Shared memory stays out** until a later ADR needs it. If isolation is ever served for another reason, B can be added inside the AI workers without changing the API of decision 2.

## Consequences

- No change to hosting, headers, Pages, the Tauri config, the toolchain, CI, `cargo deny` or the wasm size; the web demo and every webview get the speed-up the same day.
- Memory: up to N copies of the engine while a step runs, bounded to about 1 GiB beyond today's worker by decision 4 (spec `0143` criterion 14's 1.5 GB tab peak is checked in step 0). An idle tab holds one worker, as today.
- Grain: 25 tiles per 1 Mpx; a picture with few tiles (under 0.25 Mpx) gains little, which is fine since it is already fast. Remove background is not sped up (2.2 s, under its target).
- Speed per step is estimated at ~17 s on the build host and ~20 to 28 s on a 4-core laptop; a 2-core machine stays around 40 to 50 s (spec Frage 306 covers a miss).
- Spec `0143` (renamed `ai-tile-pool`) is reworked for this ADR: terminate-all Cancel (8), one-worker fallback (10), no hosting change (11, 15), memory bound (14), no atomics build (17), step 0 spike (0); listed in `specs/0143-ai-tile-pool/adrs.md`.
- Amends by notes on acceptance: ADR 0075 §2 (tile functions) and §3 (a pool of AI workers).

## Note 2026-11-21: step 0 measured (implementer)

Build host: 8 logical cores (`nproc` 8; Intel Xeon SierraForest, 1 thread per core, 8 cores, 1
socket), 19 GB RAM, headless Chromium (`/usr/bin/chromium`), the release editor wasm with SIMD
(unchanged), the real pinned files `realesr-animevideov3` and `ffdnet-color`, one synthetic
1000 x 1000 picture cut into its 25 tiles. N plain dedicated workers each instantiate the posted
module with its own memory and load the model; the page hands the next tile index to the next idle
worker; worker 0 receives every tile (transferred). Wall time includes the pool start (0.05 to
0.11 s). CPU time is the sum of the `DedicatedWorker` threads' `utime + stime` from `/proc`;
memory is the renderer's RSS peak above its level before the step, divided by N, and the wasm
memory of each worker. The spike was a page and a driver outside the repository (nothing
committed, no `spike/` branch: one checkout, `CLAUDE.md` §4). Firefox is not installed on the
build host: not measured.

| Step | N | Wall | Worker CPU sum | CPU vs N = 1 | Mean tile | Per worker (RSS / wasm) | Terminate all |
|---|---|---|---|---|---|---|---|
| Upscale | 1 | 80.8 s | 80.5 s | 1.00 | 3.23 s | 73 / 43 MB | 0.0 ms |
| Upscale | 2 | 48.1 s | 92.8 s | 1.15 | 3.72 s | 63 / 43 MB | 0.0 ms |
| Upscale | 4 | 24.9 s | 92.2 s | 1.15 | 3.69 s | 63 / 43 MB | 0.1 ms |
| Upscale | 6 | **19.1 s** | 101.6 s | 1.26 | 4.07 s | 62 / 43 MB | 0.2 ms |
| Upscale | 8 | 18.2 s | 126.2 s | 1.57 | 5.24 s | 62 / 43 MB | 0.2 ms |
| AI denoise | 1 | 27.0 s | 26.9 s | 1.00 | 1.08 s | 54 / 24 MB | 0.1 ms |
| AI denoise | 2 | 15.3 s | 29.5 s | 1.10 | 1.18 s | 50 / 24 MB | 0.1 ms |
| AI denoise | 4 | 8.2 s | 29.9 s | 1.11 | 1.19 s | 49 / 24 MB | 0.0 ms |
| AI denoise | 6 | **5.9 s** | 31.3 s | 1.16 | 1.25 s | 48 / 24 MB | 0.2 ms |
| AI denoise | 8 | 5.2 s | 35.2 s | 1.31 | 1.45 s | 47 / 24 MB | 0.1 ms |

Six workers meet the 30 s target on the build host (Upscale 19.1 s, AI denoise 5.9 s), so option C
goes ahead and B stays out. `M` is about 64 MB per worker for the engine and a tile (far below the
150 to 250 MB estimated), so the memory term of decision 4 only binds for large pictures: the
build takes `M = 64 MiB` plus the input picture each worker holds. N = 8 already costs 1.57 times
the CPU of one worker for Upscale (memory bandwidth), above criterion 2's 1.5; this machine's N is
`min(8 - 1, 8, ...) = 7`. AI denoise on one worker takes 27 s here, against the 74 s of the
slice-3 run through the whole app; the tile compute is the same, so the difference is outside the
model (to be read again on the build, criterion 13).
