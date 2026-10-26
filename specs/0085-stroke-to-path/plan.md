# Plan for Stroke to path

Implementer, 2026-10-26, branch `story/stroke-to-path` from `origin/main` at `cf0865c`. Built under
`adrs.md` of 2026-10-26 (customer answers Frage 45 A, 46 A, 47 A). One slice, 3 to 4 commits; no
push, no PR in this run.

## Affected crates/modules

- **`curvyo-geometry-core`:** `stroke_outline.rs` (new, public `stroke_outline`, `StrokeShape`,
  `StrokeOutlineError`), `stroke_loops.rs` (raw loops: sides, joins, caps, dots), `stroke_dash.rs`
  (dash intervals on exact arc length, the semantics of `render-core`'s `dash_walk.rs`),
  `stroke_markers.rs` (placements and shapes of `marker_place.rs`/`markers.rs`). `offset_pieces.rs`
  (open segment lists, `pub(crate)` corner/arc), `offset_rebuild.rs` (the fill rule as a
  parameter), `offset.rs` (`OffsetOutlines` crate constructor). Tests `tests/stroke_outline.rs`,
  golden `tests/stroke_to_path_golden.rs` + `tests/fixtures/stroke_to_path/cases.golden`.
- **`curvyo-document-core`:** `stroke_to_path_write.rs` (new): one commit `stroke_to_path` with
  clone unlinks, text conversion, fill parts and outlines.
- **`curvyo-ui-core`:** `stroke_to_path_plan.rs` (new): subjects, skips and counts, refusals,
  kernel call, fresh anchor ids; `history_names.rs` (`stroke_to_path`).
- **`curvyo-editor-wasm`:** `session/stroke_to_path.rs`, `wasm_stroke_to_path.rs` (binding),
  select bar state; parity test `tests/stroke_to_path_parity.rs` (render-core vs kernel), budget
  test `tests/stroke_to_path_budget.rs`.
- **`frontend`:** `lib/strokeToPathText.ts` (sentences), `components/StrokeToPathButton.tsx`,
  `SelectToolbar.tsx` (Convert group), `useEditorSession.ts`/`App.tsx` (wiring, Ctrl+Alt+C).
- **Docs:** `docs/design-system.md` (Select bar row, "Hidden, not disabled" fifth exception,
  shortcut row, Select bar layout row counts), `specs/README.md` row.
- **CI:** `performance-budgets` and the wasm golden run get the new tests.

## Tasks

- [x] 1. Docs commit: spec Ready with the answers, technical-debt entry "Dash and marker walk exist
  twice" (adrs.md decision 4).
- [x] 2. Kernel, tests first: open line with each cap (AC 4), closed square with each join (AC 5),
  sharp corner bevel (AC 6), circle (AC 7), dashes (AC 8), figure eight union (AC 9), markers
  (AC 10), node form (AC 12), empty on the grid (AC 27), out of range (AC 26).
- [x] 3. Golden file native + wasm32 and the budgets of AC 29 (a), (d) (AC 28, 29).
- [x] 4. Document write: one commit, fill parts, outline above, Fill-off replacement keeping the
  id, primitives stay primitives, clone unlinks, texts converted, undo (AC 14 to 17, 19, 21, 23).
- [x] 5. Plan: subjects through groups, texts, clones, skips (guides, no stroke, hidden, locked),
  counts, refusals (AC 13 to 18, 22, 25 to 27), history name (AC 21).
- [x] 6. Session + wasm: command, availability, selection after, refusal marks (AC 1, 2, 20, 22,
  24 via an offset of the result).
- [x] 7. Parity test render-core vs kernel at 20 px/mm (AC 11), edges 0.01 mm, dash ends and
  markers 0.05 mm; budgets (b), (c) in the session budget test (AC 29).
- [x] 8. Frontend: button in the Convert group, dimmed rule and tooltip, notice, Ctrl+Alt+C (not
  with AltGr, not while typing), focus rules (AC 1 to 3, 22).
- [x] 9. Docs: design-system rows, measure the Select bar rows at 800 x 600.
- [x] 10. Full gate, frontend build and tests, browser check with screenshots.

## Notes from the build

- `offset_rebuild::rebuild` takes the fill rule (`Positive` for Offset, `NonZero` for the stroke);
  a crossing at a shallow angle may lie up to four tolerances from its ring vertex (the chords of
  both curves slide along each other), so such rings map instead of falling back. The offset
  golden file is unchanged.
- A miter past the limit is a bevel for a stroke (as `lyon` draws it) and stays clipped for an
  offset (`RawStyle::clip_long_miters`).
- The parity test showed dash ends drifting along long curves: `render-core` now measures dashed
  arc length at 0.001 mm at most (adrs.md decision 4, "the kernel is the reference"). The parity
  test renders before and after through `render-core` in `curvyo-editor-wasm`, so no new crate
  edge is needed. The 0.5 % share is checked on the criterion 11 shapes; the thin dashed curve
  with markers and the long dashed path (1,200 nodes, under the canvas's 2,000 dashes per object;
  the canvas draws more dashes solid) are held to the 1 px edge rule.
- More than 100,000 dashes on one outline stroke it solid (a guard against crafted patterns).

## Validation

- One focused test per criterion in the crate that owns it (kernel geometry in
  `curvyo-geometry-core/tests/stroke_outline.rs`, the write in document-core unit tests, plan and
  session in editor-wasm session tests).
- AC 11 by rasterising both the lyon stroke (`render-core` draw list) and the filled outline at
  20 px/mm and counting differing pixels.
- Golden file compared natively and in Node on wasm32 at 1e-6 mm.
- Browser: a dashed line with Round cap and join before and after the press.
