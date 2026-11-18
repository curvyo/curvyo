# Performance pass: idle CPU, a draw only when something changed, and a view that stays glued to its frame

Status: Draft (plan for the customer's review, 2026-11-18)
Priority: Should
Origin: Customer (2026-11-18): about 8.5 % of one core and 2 % of 64 GB while idle; scroll, zoom and pan feel laggy, the white of the page trails the frame.

## User value

As a maker I want Curvyo to cost no CPU while I look at a drawing and to follow the mouse wheel and
the drag without the page trailing behind its frame, on a laptop as on a workstation.

## Findings (read from the code on 2026-11-18, to be confirmed by the measurements of phase 0)

1. **No idle state.** `useEditorSession.ts` `renderLoop` runs `requestAnimationFrame` forever and every
   frame calls `session.render()` (draw list rebuilt in Rust, pictures, wgpu submit), plus about ten
   wasm queries (`view_origin_*`, `sync_frame_start`, `take_*_notice`). Nothing skips a frame whose
   document, view and overlays did not change. This is the likeliest source of the 8.5 % at rest. The
   fixture layer of `0113` adds a rebuild per frame (`docs/technical-debt.md`).
2. **Chrome polls on its own frame.** `PageFrames.tsx` runs a second `requestAnimationFrame` loop that
   calls `pages_json()` (serialises all pages to JSON every frame), compares strings and re-renders
   React. Rulers, clone tags and the sketch chips follow the view in the same way. A DOM frame that
   updates one frame after the wgpu canvas is the trailing edge the customer sees on pan and zoom.
3. **Wheel and pointer events are handled one by one**, each calling into wasm and then syncing
   panels (`syncEntry`), instead of one coalesced update per frame (`lib/latestPerFrame.ts` exists and
   is used elsewhere).

## Criteria (budgets are proposals, to be fixed after phase 0)

1. Idle: a still document with no pointer input costs under 1 % of one core, measured as process CPU
   over 30 s in the Playwright run, and submits no frame.
2. Pan, zoom and wheel on a page of 5 000 nodes: every frame at most 16 ms of main-thread time, and
   the page frame, rulers and objects are always drawn for the same view in the same frame (no
   frame in which the frame outline and the artwork disagree by a pixel).
3. No behaviour change: all existing criteria and goldens stay green; pictures of the golden frames
   are identical.
4. Each phase lands with its measured before/after in the PR description.
