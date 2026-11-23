# ADRs for the AI tile pool

Status: Accepted by the lead as a Proposal 2026-11-21, the customer may veto at the demo (architect, 2026-11-21). One new ADR, `needs-customer` (Frage 307).

- [ADR 0078](../../docs/adr/0078-ai-tiles-on-a-pool-of-separate-wasm-workers.md): Upscale and AI denoise run their 25 tiles (1 Mpx) on a pool of N AI workers, each the existing editor wasm with its own memory; a coordinator hands out tile indices, worker 0 assembles. No `SharedArrayBuffer`, so no cross-origin isolation, no service worker, no header change on Pages or in Tauri, no nightly, no new crate or dependency, no wasm size change. Estimate ~17 s on the 8-core build host, ~20 to 28 s on 4 cores; bit-identical to one worker. Rejected: shared memory with `wasm-bindgen-rayon` in the editor wasm (A) or in a second AI wasm (B), for their isolation, hosting, nightly and CI cost; native desktop inference (D) deferred.
- Extended, not replaced: [ADR 0075](../../docs/adr/0075-on-device-image-models-runtime-and-crate.md) §2 (tile functions `tile_count`, `upscale_tile`, `denoise_tile`, `assemble`) and §3 (a pool of AI workers). Unchanged: [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) §5 (no COOP/COEP), ADR 0075 §5 (`rayon` only through `rten`), [ADR 0076](../../docs/adr/0076-model-download-host-and-the-one-network-use.md) (the model fetch is CORS already).

## Feature-local decisions

- 2026-11-21: **The spec's shared-memory criteria change if Frage 307 is A** (the default). **For the PO:** "Thread pool" in Words becomes "N web workers, each with its own wasm memory"; criterion 8's shared cancel flag becomes terminate-all (0.2 s today); criteria 10 to 12 (isolation on Pages, headers, webviews) and 15 (COEP audit) drop to one check that no header is needed; criterion 14's `--max-memory` becomes the pool-size bound of ADR 0078 §4 (about 1 GiB beyond today's worker); criterion 17 (atomics build) drops; Fragen 301, 303, 304 and 305 are moot (305 C is this ADR). Criterion 3's cap 8 stays, with the memory bound added. Criterion 5 tightens to bit-identical.
- 2026-11-21: **Upscale x2 halves inside each tile** (ADR 0078 §2): equal to halving the assembled picture because cores are multiples of 896 output px; the x4 tiles never leave the worker. Nothing for the PO.
- 2026-11-21: **Step 0 measures the per-worker memory** before code (ADR 0078 §8); the pool size formula takes it. **For the lead:** the spike branch is `spike/ai-tile-pool`, never merged.

## Open question

**Frage 307 (WASM-THREADS-SHAPE), default A taken, spec reworked as `ai-tile-pool`: How do the AI tools use several cores?**
A. Separate engine copies per core, one per tile (ADR 0078): no change to hosting, headers, the desktop apps or the toolchain; about 150 to 250 MB memory per copy while a step runs (measured 2026-11-21: about 65 MB per copy); ~17 s on 8 cores, ~20 to 28 s on 4 (measured on 8 cores: Upscale 19.8 s, AI denoise 6.9 s).
B. Shared-memory threads inside one engine: similar speed and less memory, but the web demo must be served cross-origin isolated (a service worker on GitHub Pages or a move to another host), the desktop webviews must be verified one by one (WebKitGTK unknown), and a pinned nightly Rust builds a second wasm; Fragen 301, 303, 304 and 305 then apply.
C. A now, B later if a future feature needs isolation anyway.
Recommendation: A. Default if unanswered: A.
