# Plan for the font preview

Implementer, 2026-10-23, on `story/font-preview`, stacked on `story/single-line-fonts` (`0076`,
PR #171, not merged yet). Built for Frage 25 to 28 at A and the architect's decisions 1 to 5 of
[`adrs.md`](adrs.md); UX per [`ux-notes.md`](ux-notes.md).

## Affected crates/modules

- `curvyo-text-core`: `preview::preview_path` draws the tile's sample as one line, as it draws
  the rows (decision 2; the wrap of 1 to 3 lines is gone, customer decision 2026-10-23);
  `Face::missing_chars` (decision 3).
- `curvyo-ui-core`: new module `preview_faces` (the faces the preview draws, within a 256 MiB
  byte budget, least recently drawn dropped first, decision 2); new module `preview_sample`
  (the first line of a text, at most 200 characters, decision 5).
- `curvyo-editor-wasm`: `TextBarView` gains `sample_node` and `sample` (decision 5); glue
  `PreviewFacesGlue` (`add`, `has`, `tile` as JSON `{d, box, cap, missing, single_line}`) and its
  binding `WasmPreviewFaces`.
- `frontend`: `CanvasArea` (the canvas region with one optional side pane, decision 1); the
  pane `FontPreview` (header, tile grid, tile); the Preview button in `FontSearchRow`; the
  Fonts tab publishes its listed families to a page store so the pane keeps them on other tabs
  (criterion 26); pure rules in `lib/fontPreview*.ts`; tile faces loaded through the row loader's
  byte read (`fontFaceLoader`), shared by rows and tiles.
- `docs/design-system.md`: section "Font preview (`0077`)" and the Fonts tab search-row amendment
  (UX notes §7).

## Tasks

Milestone 0, docs
- [x] 0. This plan.

Milestone 1, core and glue (test first)
- [x] 1. `Face::missing_chars`: distinct, first-seen, whitespace and controls left out; OpenType
  charmap and SVG `<missing-glyph>`; "Jürgen" in a fixture face without "ü", none in Source
  Sans 3 (AC 18, 25)
- [x] 2. `preview_path` for tiles: the whole sample as one line however long, no wrap and no
  ellipsis (rewritten for the customer decision of 2026-10-23); empty string (AC 11, 13, 15, 25)
- [x] 3. `preview_faces` store: add, get marks use, eviction above the byte budget, the face
  just added is never dropped (AC 17, 23)
- [x] 4. `preview_sample` and `TextBarView.sample_node`/`sample`: none, one, two texts, edit
  mode, a newline, 201 characters (AC 8, 9)
- [x] 5. `PreviewFacesGlue` and `WasmPreviewFaces`; a budget test: 30 tiles of "Anna & Jonas
  2026", within 100 ms in a release-like measure (AC 10, 12, 15, 18, 23)

Milestone 2, frontend
- [x] 6. Pure rules with node tests: stored state (key `curvyo.font-preview`, guarded, defaults
  and range), pane layout (split width, forced full when the drawing would keep under 360px, width deltas), grid (columns
  W = max(240, 5 x size), one-line tile height, rows in view plus one screen),
  tile name and description, missing tooltip (at most 10), subject line, sample follow rule,
  grid keys (AC 3, 6, 8 to 13, 16, 18, 22, 23, 26)
- [x] 7. `CanvasArea`: split and full, `keep_view_origin_for_panel_toggle` before every width
  change, the drawing `inert` and hidden in full view (AC 2 to 5)
- [x] 8. Preview button in the search row (`LayoutGrid`, `aria-pressed`), open and close,
  focus rules (AC 1, 5, 21)
- [x] 9. Header: sample field with Clear, Size slider (16 to 160, default 32, click, drag,
  double-click to type, keys, tooltip "Size: 32 px"), subject line, Full view and Close; one or
  two rows by width (AC 4, 5, 7, 12, 26)
- [x] 10. Tile grid: virtualised by rows, tiles from `preview_tile`, loading placeholder after
  100 ms in view, missing marker, star, chips, current mark, picking, keys (AC 11, 13 to 23)
- [x] 11. Fonts tab publishes the listed families, chosen cell and count; the preview keeps them
  when the tab changes or the panel collapses (AC 16, 26)
- [x] 12. Design-system rows (UX notes §7)

- [x] 14. Draggable divider (customer decision 2026-10-23): split share stored with the view
  state, clamp to one column and a 360px drawing, live drag without re-zoom, double-click reset,
  `separator` with arrow keys; node tests for the clamp, storage and reset (AC 3, 6, 27)

Customer feedback 2026-10-23
- [x] 15. Split default 35 % instead of 30 %: layout rule, node tests, spec criterion 3, UX notes,
  design system (AC 3, 27)
- [x] 16. One line per tile, cut off at the tile's right edge: wrap, reserved lines and ellipsis
  removed from `preview_path`, the glue, the binding and the grid; tile height from the size
  alone; implementer and tester tests rewritten to the new rule (AC 11, 13)
- [x] 17. No flicker while dragging the divider: pointer moves coalesced into one width per
  animation frame (`latestPerFrame`, node test), committed in that frame; `CanvasArea` resizes
  and redraws the canvas in the commit that changes its width (`fitCanvas`), the resize
  observer measures afresh and skips a buffer already applied; a window resize commits the pane
  width in the observer, so pane and drawing change in one frame (AC 27, 23)

Customer reports 2026-10-23 (second round)
- [x] 18. Backspace and Delete in canvas text edit on the Linux desktop: the text sink no longer
  takes `defaultValue` (React wrote it again on every render; WebKitGTK then put the caret at 0,
  so Backspace had nothing before it); the layout effect alone keeps value and caret (AC 9)
- [x] 19. Divider drag without canvas resizes: the drawing takes the width beside the narrowest
  pane once the pressed divider first moves, the pane moves over it, and the drawing gets its
  final width when the drag ends (two resizes per drag, none per frame; keys and double-click one
  each) (AC 2, 27)
- [x] 20. Escape outside the preview closes it once the canvas's Escape ladder had nothing to do
  (`escape-none` stays unhandled; a window listener closes on an unhandled Escape); node test
  (AC 5)

- [x] 21. Tags on tiles (customer decision 2026-10-23): the listing carries every tag and the
  list's tag edits (kept rows), the tile footer holds the Fonts tab's `FontTagLine` (new `inset`
  prop), `useFontTagLine` looks rows up in a given scope (tree or grid), "+N" shows every tag
  over the sample, F2 and Shift+F2 on a tile; node tests for the tag room and the keys (AC 14)

Customer reports 2026-10-23 (third round, desktop with a real GPU)
- [x] 22. No canvas resize by the split at all: the drawing's canvas element (and the horizontal
  ruler's) is wider than its box by the pane's width (`--canvas-overhang`), so it always spans
  the canvas area and only a window resize changes it; the pane lies over it.
  `Session::cover_right` / `WasmSession::cover_canvas_right` give the viewport the visible width
  (fit, centring, placement, off-screen and mark rules), re-applied to a new or opened session;
  the rulers, rail and bars follow the visible box; a press on the pane is the pane's. Rust test
  of the session; node test of the overhang rule (AC 2, 5, 27)
- [x] 23. Divider drag at frame rate: tiles take CSS shares of the grid's width
  (`tileColumnCss`), the grid is memoised and takes a new width during a drag only when the
  column count changes (`sameGridArea`); one drawing per face and sample for every Size and width
  (`TILE_DRAW_CAPS`, cache key without the width); node tests (AC 23, 27)
- [x] 24. Tiles never in a wrong font, drawn ahead: a `--tile-skeleton` bar until the face is
  read; the faces of the listed families read ahead while idle once the Fonts tab lists them, in
  view first and in parallel, then the next ones one at a time (`fontPreviewPrerender`, at most
  96); node tests for the order and the idle queue (AC 17, 23)
- [x] 25. Tags in a row of their own under the name and the star, reserved on every tile; tile
  height, tag room, UX notes, design system, node tests (AC 13, 14)

Milestone 3
- [x] 13. Full gate (`CLAUDE.md` §7) and `tsc`, `npm test`, `oxlint`

## Validation

- Rust unit and integration tests for tasks 1 to 5, on the included fonts and one new SVG fixture
  face without "ü" (`curvyo-text-core/tests/fixtures/svg_fonts/no-umlaut.svg`).
- Budgets: criteria 10 and 12 (30 tiles per keystroke) measured in a Rust test of the glue (the
  wasm work per keystroke); criterion 16 is the Fonts tab's filter of `0075` (already measured
  there) plus one store publish; criterion 23's virtualisation is the node test of the rows in
  view. Frame rate and the 1 s / 2 s first-screen budgets need the app and are left to the
  tester's manual run on the plan's machine.
- Node tests in `frontend/tests/fontPreview*.test.ts` for every pure rule.
- Manual: `cd frontend && npm run tauri dev` (desktop) and `npm run dev` (browser).

## Notes for the lead and the architect (2026-10-23)

- `WasmPreviewFaces::preview_tile(id, text)` returns the whole line; the tile JSON carries `cap`
  for the scale (1 face em = size / cap px).
- In full view the drawing keeps the full canvas-area width under the pane (hidden and `inert`,
  never 0 px), so only opening and closing split view change the drawing's width.
- Customer decision 2026-10-23: one line per tile, clipped (replaces the reserved 1 to 3 lines,
  the fixed line pitch and the ellipsis of the first build). Default size 32px (lead decision,
  Proposal).
- Divider flicker (customer feedback 2026-10-23): each pointer move committed a new pane width
  in a React task; the canvas element took its new CSS width there, but its backing store was
  resized and redrawn only by the resize observer at the next rendering step, after the render
  loop had drawn into the old buffer, and every move also redrew all visible tiles at the new
  width. Not reproduced in headless Chromium or WebKitGTK under Xvfb (no blank frame in 482
  grabs); the fix removes the gap rather than a reproduced symptom.
- Divider flicker, second report (desktop, real GPU): the canvas no longer resizes while the
  divider moves. When the drag starts moving, the drawing widens once to the width beside the
  narrowest pane (the added part is under the pane, so nothing on screen moves) and the pane is
  laid over it; every place the divider can uncover is already drawn. On release the drawing
  takes its final width in one resize and redraw. The vertical bars at the drawing's right edge
  sit under the pane during the drag and come back on release. Superseded by task 22: the
  customer still saw one flicker on grabbing and one on releasing, exactly those two resizes.
- Third round, measured in Chromium 1500 x 900 (production build, SwiftShader GL, 7 included
  families, 121 divider moves): JS per drag frame p95 2.6 ms before, 0.8 ms after; renderer main
  thread busy 1,234 ms before, 1,058 ms after (React work 633 to 213 ms); preview tile draws
  during the drag 56 to 63 before (a new width every few px), 0 after; canvas resizes per drag 2
  before, 0 after. Headless frame rate is bound by SwiftShader's GPU thread (busy the whole
  span in both builds), and the canvas now spans the whole area, so headless frame intervals do
  not show the gain; the desktop with a real GPU is the judge.
- Tiles, same setup: the first visible tile drawn 300 to 340 ms after the Preview press before
  (the sample shown in the UI font for about 180 ms in between); after, with the Fonts tab open
  for a moment, every visible tile is drawn in the first frame the pane shows (62 to 98 ms after
  the press, the pane's own mount). Where the time went: face bytes 50 to 210 ms per file (dev
  server; the host read on the desktop), the 100 ms in-view delay, then add and draw 1.5 to 2 ms
  per face; a tile from a loaded face costs about 0.5 ms (a keystroke on 7 tiles 3 to 4 ms of
  wasm), so batching tiles per frame was not needed.
