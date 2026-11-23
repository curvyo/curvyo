# Plan for the AI tile pool

**Status:** Done (PR #447, 2026-11-23, merged with `0135` slice 3).

Built on `story/image-ai-tools` (stacked on `0135` slice 3, one PR, lead 2026-11-21).

## Affected crates/modules

- `curvyo-imageai-core::tiles`: `tile_count`, `TileOutput`, `assemble`; `run`: `upscale_tile`,
  `denoise_tile`, `upscale` and `denoise` as a loop over them (ADR 0078 §2).
- `curvyo-editor-wasm`: `ai_glue` and `wasm_image_ai` gain the worker's tile calls (input set once
  per Run, one tile, the assembler's tiles and `assemble`); the session's Running line says
  "Running on one core" when the pool has one worker.
- `curvyo-ui-core::image_adjust_ai`: the running line with the one-core words.
- `frontend`: `lib/aiPool.ts` (coordinator: pool size, tile hand-out, assembly on worker 0,
  progress throttle, Cancel, worker drop), `lib/aiJobs.ts` (`AiRunner` runs the tiled kinds on the
  pool), `workers/aiWorker.ts` (the tile messages), `lib/aiSteps.ts` (the model file for the
  extra workers, the one-core call).

## Tasks

- [x] T0. Step 0 spike outside the repository, measured, dated note in ADR 0078 (AC 0).
- [x] T1 (core, test first). `tile_count`, `upscale_tile`, `denoise_tile`, `assemble` with
      `AiError::Invalid` for a missing, doubled or wrongly sized tile; `upscale`/`denoise` loop
      over them; x2 halves inside each tile; golden: the pre-change output of a 3 x 3 blur
      stand-in is pinned by hash, shuffled tiles assemble to it (AC 5, 13, 17).
- [x] T2 (wasm). Tile calls in `ai_glue`/`wasm_image_ai`; native test that tile by tile plus
      assemble equals `run_model` (AC 5, 17).
- [x] T3 (ui-core, editor). "Running on one core" in the step row's running line when the Run
      has one worker (AC 10).
- [x] T4 (frontend, test first). `aiPool.ts`: `poolSize(cores, inputBytes)` =
      `min(cores - 1, 8, floor(1 GiB / (64 MiB + input)))`, at least 1; dynamic tile hand-out;
      tiles forwarded to worker 0, `assemble` there; extra workers terminated at the end; a worker
      that fails to start or errors is dropped (its tile handed out again); progress at most
      10/s; Cancel terminates all. Fake-worker tests: N = 1 and N = 3 give the same tile set, any
      finishing order, Cancel, drop, throttle (AC 3, 5, 6, 7, 8, 9, 10, 14).
- [x] T5 (frontend). `AiRunner`/`AiSteps` wiring: the pool for Upscale and AI denoise, Remove
      background on worker 0 as today, worker 0 ended after 60 s idle (AC 1, 6, 7, 8, 16).
- [x] T6 (docs). ADR 0075 §2/§3 acceptance notes, ADR index, measurements on the build in this
      plan (AC 1, 2, 8, 14, 18, 19).

## Measured on the build (2026-11-21, manual, `CLAUDE.md` §5)

Build host, class B: 8 logical cores (`nproc`), Intel Xeon SierraForest, 19 GB RAM; headless
Chromium (`/usr/bin/chromium`, Playwright), the whole app on the Vite dev server with the release
wasm, the real pinned models served by a Playwright route in place of the model host (no code
override), a synthetic 1000 x 1000 picture dropped on a laser page, one AI step per page, the
model cached. N set by overriding `navigator.hardwareConcurrency` (cores 2, 3, 5, 8 give N 1, 2,
4, 7). Wall: Run pressed to the progress row gone (result stored). CPU: the sum of the
`DedicatedWorker` threads' time over the Run (`/proc`). Memory: the renderer's RSS peak above its
level before the Run.

| Step | N | Wall | Worker CPU | CPU vs N = 1 | RSS peak above before |
|---|---|---|---|---|---|
| Upscale x2 | 1 | 78.8 s | 77.8 s | 1.00 | 36 MB |
| Upscale x2 | 2 | 47.1 s | 90.2 s | 1.16 | 187 MB |
| Upscale x2 | 4 | 27.8 s | 95.9 s | 1.23 | 238 MB |
| Upscale x2 | 7 | **19.8 s** | 111.6 s | **1.43** | 450 to 542 MB |
| AI denoise | 1 | 26.8 s | 26.2 s | 1.00 | 144 MB |
| AI denoise | 2 | 15.6 s | 28.7 s | 1.10 | 71 MB |
| AI denoise | 4 | 8.9 s | 28.9 s | 1.10 | 200 MB |
| AI denoise | 7 | **6.9 s** | 31.6 s | **1.21** | 346 MB |
| Remove background (control) | 1 | 2.3 s | 1.8 s | | 26 MB |

Criterion 1 (class B): Upscale 19.8 s and AI denoise 6.9 s, under 30 s; Remove background 2.3 s,
under 3 s. Criterion 2: the CPU sum at N = 7 is 1.43 and 1.21 times N = 1. Criterion 8: Cancel
4 s into an Upscale on 7 workers: the row is idle 0.45 s after the click, no worker thread left.
Criterion 10: N = 1 shows "Running on one core · n %". Criterion 13: N = 1 takes 78.8 s for
Upscale (74 s before, the same tile code; within run-to-run noise) and 26.8 s for AI denoise.
Criterion 14: the tab peaks at most 542 MB above its level before the Run. Class A (4 cores),
Firefox and the desktop webviews (criteria 1, 12) are not measured here: no such machine or
browser on the build host.

## Validation

Native core and wasm-glue tests with tiny test nets; node tests of the coordinator with fake
workers and a fake clock; a manual Chromium run of the whole app on the build host for criteria
1, 2, 8 and 14 (wall time is a manual number, `CLAUDE.md` §5).

## Deviations (Proposal)

- Step 0 ran as a page and driver outside the repository, not on a `spike/ai-tile-pool` branch
  (one checkout, one branch); Chromium only, Firefox is not installed on the build host.
- The worker's progress keeps the whole-percent callback of `0135` (tiles done of total as a
  percent, the bar unchanged); the pool size and "one core" reach the session through an optional
  `extras` argument, so the `0135` tester's fakes stay as they are.
- The 74 s of AI denoise in the `0135` run is 27 s on one worker here, with the same tile code;
  the earlier number is not reproduced.
- `M` in criterion 3 is 64 MiB measured plus the input picture's bytes each worker holds (so a
  16 Mpx AI denoise gets fewer workers than a 1 Mpx one).
