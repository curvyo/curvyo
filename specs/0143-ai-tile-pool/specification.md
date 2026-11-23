# AI tile pool: the image models on a pool of separate wasm workers

Status: Ready (ADR 0078 is accepted by the lead as a Proposal, Frage 307 default A; the customer may veto at the demo)
Priority: Must (it gates slice 3 of `0135`)
Origin: Customer. Frage 286 B (2026-11-21): "build wasm threads first, then ship slice 3 of 0135." Reason: in Chromium on a 1 Mpx picture Upscale (realesr-animevideov3) and AI denoise (ffdnet-color) take 74 s each against the target of 30 s (`0135` criterion 50); Remove background takes 2.1 s. `rten` runs single-threaded in wasm today. The architect's ADR 0078 reaches the same goal without shared memory (Frage 307 A, the default), so the story was renamed from `wasm-threads` to `ai-tile-pool`.

## User value

As a maker who runs Upscale or AI denoise on a photo before engraving I want the step to finish about as fast as my machine allows, with a progress bar and a Cancel, so that I do not wait over a minute per step. The AI steps use several cores of my computer in the browser and in the desktop app, with no setting and no change in how the app is served; on a machine with one or two cores the same steps still run, slower.

## Words

- **Tile pool:** N web workers, each the existing editor wasm with its **own** memory (no `SharedArrayBuffer`), that compute the tiles of one AI step in parallel (ADR 0078). A coordinator hands out tile indices; worker 0 assembles the picture. Only `curvyo-imageai-core` work (ADR 0075) uses it; the editor, trace and every other worker stay as they are.
- **Tile:** one 256 px model input with 16 px overlap and a 224 px core (ADR 0075 §2); a 1 Mpx picture has 25 tiles, about 3 s each on one core.
- **N:** the pool size, decided in criterion 3.
- **Fallback:** N = 1, the path of today (74 s on the reference run).

## Reference run (used by every speed criterion)

One picture, 1 Mpx (1000 x 1000 px, the test photo `tests/fixtures/imageai/photo_1mpx.png`, to be added with the fixture), steps Upscale (realesr-animevideov3) and AI denoise (ffdnet-color), Remove background (u2netp) as the control. Release build, wasm SIMD, model already downloaded and cached. Machine classes, to be named once in the PR: **A** 4 physical cores, **B** 8 physical cores, in Chromium and Firefox and in the desktop webview of the platform at hand. Wall time is reported for the maker's view (it is what "30 s" means); the scaling is also reported as CPU time per worker, because wall time alone hides a pool that spins (`CLAUDE.md` §5: timing tests measure CPU time; the wall-time check here is a manual demo number, not a CI test, and is marked so).

## Acceptance criteria

### Step 0: the spike (before the build)

0. Given a throwaway spike on branch `spike/ai-tile-pool` (never merged), when the 25 tiles of the reference picture run on N = 1, 2, 4, 6 and 8 plain workers in Chromium and Firefox on the build host, then a dated note in ADR 0078 lists per N the wall time, the sum of the worker CPU times and the peak memory per worker `M`. If 6 workers miss 30 s on the build host, the lead is told before any production code is written, with shared-memory threads (ADR 0078 option B: cross-origin isolation, a service worker or a new host, a pinned nightly) as the next option; the miss is not worked around silently.

### Speed and pool

1. Given the reference run on class B in Chromium, then Upscale and AI denoise each finish in 30 s or less, and Remove background in 3 s or less (it must not get slower than 2.1 s plus 50 %). On class A the target is 40 s or less (**Proposal**: the 30 s of `0135` assumed 4 to 8 cores; the PR states the measured values, never a rounded-down number). A miss on class A is handled by Frage 306 (default A).
2. Given the reference run, then the report lists for pool sizes 1, 2, 4 and the machine's N the wall time and the sum of the worker CPU times; the CPU-time sum at N is at most 1.5 times that of N = 1 (a pool that merely spins fails this). The Rust part follows the `curvyo-editor-wasm/tests/cpu_time/` rules; the browser wall time is a documented manual run.
3. Given a machine with C logical cores (`navigator.hardwareConcurrency`) and a measured peak memory `M` of one AI worker during a tile (step 0), then `N = min(C - 1, 8, floor(1 GiB / M))`, at least 1 (**Proposal**: one core stays for the page; 8 is the cap of the measured scaling, Frage 302 A decided; the memory term keeps the pool small). N is read when a tiled step runs; the pool is created then, never at app start (no extra worker and no extra memory for a maker who never uses an AI tool). A worker that fails to start is dropped and the step goes on with the rest.
4. Given an AI step is running, then the page stays responsive: a pointer move over the canvas is handled within 100 ms (manual check; the workers are not the main thread, and the main thread never waits for them with a blocking call).
5. Given the same picture, model and parameters, then the result is **pixel-identical** whatever N is (N = 1, 3, the maximum, tiles finishing in any order): a native test in `curvyo-imageai-core` runs the tiles in shuffled order through `assemble` and compares with `upscale` and `denoise` on a stand-in model, and a coordinator test with fake workers shows N = 1 and N = 3 give the same tile set. The stored PNG stays the source of truth (ADR 0077).
6. Given a step ends, then the extra pool workers are terminated and their memory is released; worker 0 stays as today and is terminated after 60 s idle (**Proposal**). The next step builds the pool again (instantiation from the cached module plus model parse, estimated under 0.5 s). An idle tab holds one worker, as today.

### Progress and Cancel

7. Given an AI step running on the pool, then the step row shows a determinate progress bar (tiles done of tiles total, ADR 0075's tiling) and a Cancel, as `0135` criterion 65 says; progress is posted at most 10 times a second.
8. Given Cancel during a step, then every pool worker is terminated and the step row is back to its idle state within 1 s on class B (terminate-all, about 0.2 s today). No partial result is stored and the document is unchanged.
9. Given a step is cancelled or the tab closes during a step, then no worker stays behind (the browser's task manager shows the tab's memory back at its level before the step, within 50 MB).

### Fallback and platforms

10. Given a machine where N is 1 (one or two logical cores, or workers that fail to start or are refused memory), then the step runs on worker 0 as today, shows "Running on one core" once in the step row (wording by UX), and finishes without a hang or a crash.
11. Given the web app and the desktop app, then no hosting change is needed and none is made: no COOP/COEP header, no service worker, no change to `pages.yml`, `vite.config.ts` or `curvyo-app/tauri.conf.json`; the page stays not cross-origin isolated (`crossOriginIsolated === false`) and the speed-up works all the same. Check: the speed criteria 1 and 2 are measured on the unchanged demo build and on the desktop app.
12. Given the desktop app, then the pool runs on each platform's webview: WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux). One manual run per platform, with the number and webview version in the PR (dedicated workers exist everywhere; the OCR pool already runs there). A platform that cannot be tried is listed in the PR as not verified.
13. Given N = 1 and N > 1 on the same input, then the results are pixel-identical (criterion 5) and N = 1 takes the time of today (74 s on the reference run), never longer than before this story.
14. Given a 1 Mpx Upscale step on the reference run with N workers, then the tab memory peaks at no more than 1 GiB beyond today's single worker (about 150 to 250 MB per copy; the pool-size bound of criterion 3 enforces it), measured in step 0 and again on the build. A step that cannot get memory drops workers (criterion 3) and in the end falls back to criterion 10, never a crashed tab.

### No regression

15. Given the build, then no new request, origin, header or network use appears: the model fetch (ADR 0076) is unchanged, the extra workers load the same-origin wasm module and the already cached weights, and the Tauri CSP keeps `worker-src 'self'` as it is today (and `blob:` only where the OCR pool already needs it). A grep in the PR shows no new `fetch`, `WebSocket` or origin.
16. Given the existing features, then they behave as before: the full `npm test`, the editor, trace (`0023`), OCR, PDF and SVG export, clipboard and drag and drop of pictures work, and the idle CPU of `0126` does not rise (no extra worker runs with no step; checked with the task manager). The tester's regression list is `npm test` plus the manual smoke list in the PR.

### Build and CI

17. Given the build, then there is no new crate, no new dependency, no atomics build, no nightly toolchain and no second wasm: the editor wasm gains only the tile functions of ADR 0078 §2 (`tile_count`, `upscale_tile`, `denoise_tile`, `assemble`), `upscale` and `denoise` become a loop over them, and the wasm size stays within the `wasm-size.yml` budget. `cargo deny`, the gate of `CLAUDE.md` §7 and the `wasm32-unknown-unknown` check for every `*-core` crate stay green. `assemble` refuses a missing, doubled or wrongly sized tile with `AiError::Invalid` (test).
18. Given the PR, then the description's review guide lists the central Rust files (`curvyo-imageai-core::tiles` and the wasm entry) and the coordinator beside `ocrPool.ts`, "no dependency change" or each change with licence and reason, the measurements of criteria 0, 1, 2, 8 and 14 with the machine class, and the browser and webview versions of criterion 12.
19. Given the docs, then ADR 0075 §2 and §3 carry the acceptance note ADR 0078 names, `docs/requirements.md` or the ADR index is updated, and `docs/technical-debt.md` gets an entry if a measured class stays above its target (shared-memory threads, ADR 0078 option B, as the known next step).

## Out of scope

- Shared-memory threads (`SharedArrayBuffer`, `wasm-bindgen-rayon`, cross-origin isolation): only if the step 0 fallback applies or a later feature needs isolation anyway (ADR 0078 decision 9).
- A WebGPU runtime for the models (a later story; it could replace the pool and is a different bet).
- Native desktop inference outside the webview (`rten` as a native thread pool in the Tauri backend would be faster still; a second path to maintain, not this story).
- Threads for any other crate or step: the editor, geometry, trace, laser job, sync and export stay single-threaded. Remove background stays one run on one worker (2.1 s, under its target).
- Smaller or quantised models, a different model for Upscale, a lower Upscale size (the models are fixed by ADR 0075; if the target is missed, that is a follow-up decision, Frage 306).
- Mask painting and new AI tools (`0135` slice 4), the download flow itself (ADR 0076), a progress bar for non-AI steps.
- A setting for the number of workers (**Proposal**: no setting; the pool size is decided in criterion 3).

## Open questions

Answered or decided by default; none is open.

- **Frage 286 B (answered, 2026-11-21):** build the speed-up first, then ship slice 3 of `0135`. This story is the result.
- **Frage 307 (WASM-THREADS-SHAPE), default A taken by the lead, the customer may veto at the demo:** the AI tools use several cores through separate engine copies, one per tile pool worker (ADR 0078), not shared-memory threads. B (shared memory) is the step 0 fallback.
- **Frage 302 (WASM-THREADS-POOL-CAP), decided A:** at most `min(cores - 1, 8)` workers, plus the memory bound of criterion 3.
- **Frage 306 (WASM-THREADS-MISS), decided A:** if class A misses its target after this story, slice 3 ships with the measured time shown in the step row ("about 45 s on this computer") and a follow-up is opened.
- **Moot with ADR 0078:** Frage 301 (isolation on GitHub Pages), 303 (`require-corp` or `credentialless`), 304 (one wasm or two) and 305 (nightly Rust) only apply if shared memory is chosen later.

## UX notes

See `ux-notes.md`: a one-paragraph note. There is no new UI except progress and Cancel, which stay as `0135` defines them, plus the "Running on one core" line.

## Links

Requirements: R-MFG-LASER-001 (via `0135`)
Parent and gate: `specs/0135-image-optimisation/` slice 3 (ADRs 0075 to 0077; criterion 50 sets the 30 s target)
Decision: [ADR 0078](../../docs/adr/0078-ai-tiles-on-a-pool-of-separate-wasm-workers.md), `adrs.md`
Related: ADR 0023 (the trace worker; no COOP/COEP stays), ADR 0075 §2 and §3, ADR 0076 (model host)
PR: -
