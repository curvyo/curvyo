# Plan for Snapping

One branch `story/snapping` (cut from `main` at `db776a01`), one PR, opened when the whole
slice can be tried end to end. The five milestones below are commits on that branch, each
leaving the local gate green; none of them is a PR. Spec, UX notes and ADR notes:
`specification.md`, `ux-notes.md` and `adrs.md` in this folder.

Defaults taken (the spec's open questions, customer may change): snapping off, all four
target kinds on, capture 8 px, release 14 px, Ctrl+Alt suspends, block in the Document tab.
The stored settings are document state; the modifier is not stored.

## Affected crates/modules

- `curvyo-document-core`: new `snapping.rs` (value, defaults, lenient read, strict `validate`,
  `Document::snapping` / `set_snapping`); `document.rs` (format version 26, `document.json`
  view, validation call); `lib.rs`; golden `tests/fixtures/snapping_v26.curvyo` and five
  damaged fixtures; `tests/format_*.rs` following `guides_format.rs`.
- `curvyo-ui-core`: new `snapping.rs` (settings, targets, probes, state, resolve, page
  fractions, marker geometry); calls in `select_tool/move_drag.rs`,
  `select_tool/gesture.rs`/`press.rs` (resize), `node_tool.rs`, `shape_tool*` and `pen_tool*`;
  `lib.rs`.
- `curvyo-render-core`: new `snap_marker.rs` (`DrawList` output); `theme.rs` (reuse
  `REFERENCE_MARK`, cross and casing sizes); `lib.rs`.
- `curvyo-editor-wasm`: `session/snapping.rs` (new: read settings, gather targets at the
  press, hold the ephemeral result, settings writes and view record), `session/select.rs`,
  `session/node.rs`, `session/shapes.rs`, `session/pen.rs`, `session/draw.rs`,
  `session/mod.rs`, new `wasm_snapping.rs` bindings, `lib.rs`.
- `frontend`: `SnapToggle.tsx` (Select bar), `SnappingBlock.tsx` and `useSnappingPanel.ts`
  (Document tab), `SelectToolbar.tsx`, `DocumentSection.tsx`, `useEditorSession.ts`,
  `App.tsx`.
- Docs: `specs/README.md` (0060 state, format row 26), `docs/design-system.md` (two new rows:
  "Snapping toggle", "Snapping block"; the marker row), `CURRENT_FORMAT_VERSION` comment.

## Milestone 1: `document-core` (criteria 1, 3 storage part, 10 storage part, 14)

- [ ] 1. `snapping.rs`: `DocumentSnapping` with `DEFAULT` (off, all kinds on, 8/14), the seven
  register keys, lenient `snapping_of_root`, `Document::snapping`, `set_snapping` with one
  commit `set_document_snapping`, equal value writes nothing, release raised to capture on
  read (AC 1, 3, 14).
- [ ] 2. Strict `validate`: present register of the wrong shape or outside 1..=100 px
  (non-finite included) is `Damaged`; release below capture is valid (AC 14).
- [ ] 3. Format version 26 with the comment paragraph, `document.json` gains `snapping`,
  `Document::open` calls `validate`; unit tests for defaults, round trip, per-register writes
  and the raised release (AC 1, 3, 14).
- [ ] 4. Golden `snapping_v26.curvyo` (rect, path, guide, on, objects off, 6/20) and five
  damaged fixtures with a test each; older fixtures open with the defaults and stay
  byte-identical in `storage-io` (AC 14).
- [ ] 5. Two-peer merge: both peers write different settings, both survive; one peer writes
  the same value, no commit (AC 1).
- [ ] 6. `specs/README.md`: the 0060 row to Ready and the format row 26 (provisional).

## Milestone 2: pure engine (criteria 3 to 7, 11, 12, 15)

- [ ] 7. `snapping.rs`: `SnapSettings` (resolved), `SnapTargets` (flat, kind-ordered),
  `SnapProbe`, `SnapState`, `SnapResult`, `resolve` per `adrs.md` decision 4; tolerances in mm,
  no view or document type (AC 3, 4, 6).
- [ ] 8. `SnapTargets::gather`: boxes via `object_bounds` over visible leaves minus the
  dragged ids, guides via `read.guides` and their anchors, nodes via every visible path's
  anchors, page via six fractions of `DocumentSize`; kind switches filter; locked included,
  hidden excluded (AC 5, 13, 15).
- [ ] 9. `PAGE_FRACTIONS` (thirds, not 0.333/0.666) and the line extents for the marker; no
  storage (AC 5, 15).
- [ ] 10. Marker geometry: one line per held axis with the target's extent, one cross at the
  moving point, both-axes form; pure and testable (AC 8, 12).
- [ ] 11. Unit tests: capture exactly at 8, hold at 13, release above 14; release raised to
  capture; nearest wins and the four-kind tie order; node both-axes win; Shift free axis;
  raw-point release (no stick/flicker over 100 steps); hidden excluded, locked included, guide
  clone; page thirds at A4 and after a resize (AC 3 to 7, 11, 13, 15).

## Milestone 3: tools and session (criteria 2, 7, 9, 11, 12)

- [ ] 12. `session/snapping.rs`: read settings at open and after a write, gather the targets at
  the press from `DocumentRead::visible`/`guides`, drop on release/Escape/tool change, hold the
  result for drawing; no document read per move (AC 11).
- [ ] 13. Select move: resolve from `MoveDrag`'s raw position and box features (edges, centre
  lines, nine points), apply the displacement once, keep the existing commit and one step;
  Ctrl copy and Shift lock compose (AC 7, 9, 12).
- [ ] 14. Select resize: moving edges and the dragged reference point snap, fixed edges do not
  (AC 7).
- [ ] 15. Node tool: anchors use the both-axes scan against all enabled kinds; handles use a
  nodes-only target list (AC 7).
- [ ] 16. Shape creation drags (Rectangle, Ellipse, Polygon/Star) and Pen placement: the raw
  point of the press and every move resolve before the tool sees the point; the existing size
  readout shows the snapped value (AC 7).
- [ ] 17. Ctrl+Alt suspends from the event's modifiers in the same frame; no state written
  (AC 9).
- [ ] 18. Settings writes: `wasm_snapping.rs` methods for the switch, the four target flags
  and the two fields, all through `set_snapping`; a view record for the panel; re-read after a
  peer change (AC 1, 2, 10).
- [ ] 19. `session/draw.rs` and `session/snapping.rs`: append the marker after the previews and
  the reference box, before the selection box and handles; gone on release/Escape (AC 8, 12).
- [ ] 20. Session tests: a move snaps to a box edge, a guide and a page third; a node snaps
  both axes; a handle ignores a box; Ctrl+Alt leaves the same position as no snapping; one
  history step; no document read during the move (AC 3 to 9, 11, 12).

## Milestone 4: renderer (criterion 8)

- [ ] 21. `snap_marker.rs`: 1 px lines with 1 px white casing, 12 x 12 px cross with casing,
  constant screen width, clipped to the viewport; token reuse and no fade (AC 8).
- [ ] 22. Layer order in `draw.rs` and a test that the marker sits above the reference box and
  below the selection box/handles (AC 8).

## Milestone 5: frontend and docs (criteria 1, 2, 10)

- [ ] 23. `SnapToggle.tsx`: 28 x 28 px Lucide `Magnet`, pressed state, tooltip, appended last
  group of `SelectToolbar`; mouse press returns focus to the canvas (AC 2).
- [ ] 24. `SnappingBlock.tsx` + `useSnappingPanel.ts`: title row, 2 x 2 target toggles, Capture
  and Release `TextField`s with px suffix, ranges and validation, raising Release with Capture,
  in `DocumentSection` after Fit to content and before Background (AC 10).
- [ ] 25. `useEditorSession.ts` wiring: the switch reads the document, the panel fields read
  the session view; refresh on a peer change; no `localStorage` (AC 1, 2, 10).
- [ ] 26. Docs: design-system rows, README, spec status; frontend lint, typecheck, tests.

## Validation

Core logic test first. The document format gets a golden file, five damaged fixtures and the
older fixtures opening byte-identical. The engine is tested in document millimetres at two
scales (zone conversion is the caller's, so both a 1x and a 4x view are covered). The renderer
is tested as a draw list. The session tests cover the tool call sites end to end without a
browser. Full gate of `CLAUDE.md` section 7 plus `wasm32` builds of every touched core crate.
The frontend is type-checked, linted and unit-tested; the bar's row count at 800 x 600 and the
panel's scroll are measured in the UX review, since this environment has no browser.
