# Architect review: 0090 grid

Date: 2026-11-01. Branch `story/0090-grid` at `ff48b130`, reviewed read-only against
`specs/0090-grid/adrs.md` and `CLAUDE.md` §5/§6 (no build, no tests run).

**Verdict: APPROVE.** No blocking findings. The non-blocking findings below can land as one
small follow-up commit on this branch or as a `chore/` PR after the merge.

## What was checked and holds

- Core purity: `page_grid.rs`, `grid/*`, `snapping/grid.rs` and `grid_lines.rs` do no I/O, use
  no clock, threads or UI, and add no dependency. `curvyo-render-core` takes its own
  `GridStroke`, so it still does not depend on `curvyo-ui-core` (ADR 0011).
- Format: `CURRENT_FORMAT_VERSION` 29 with its doc paragraph (`document.rs:290-295`).
  Registers, types, defaults and ranges match decision 1. Reads are lenient per field
  (`page_grid.rs:240`). `validate` is strict, deleted pages included, and a container value
  is damage (`page_grid.rs:280`). `validate` is wired next to `snapping::validate`
  (`document.rs:469`). The writer writes only the fields that differ, in one commit labelled
  `set_page_grid` (`page_grid.rs:359`). `add_page` copies the size source's grid inside its
  own commit and writes only non-default fields (`pages.rs:200,217`). Golden and one
  malformed fixture per register type, plus "opening writes nothing" and "older file opens
  with defaults" tests (`tests/grid_format.rs`).
- Units: `Length`, `Angle`, `Vec2` in `PageGrid`. The new `ScreenPx` is used for every grid
  threshold (`grid.rs:12-19`). Named tolerances: `ANGLE_EPS`, `LEVEL_EPS`, `EPS_MM`,
  `FULL_ALPHA_EPS`, `AXIS_EPS_MM`.
- No `unwrap`/`expect` outside tests. `unwrap_or` on the `i32::try_from` calls is safe.
- Function size: the largest new functions, `snap_to_grid` (`grid/snap.rs:160`, about 60
  lines) and `resolve_with_grid` (`snapping/grid.rs:18`, 58 lines), are at the limit, not
  over it.
- The grid appears only in `Session::frame_draw_list`, which feeds only the GPU
  (`wasm_render.rs:82,97`). It is absent from the document preview branch
  (`session/frame.rs:21-33`, criterion 19 of 0042), `draw_list`, export, print, eyedropper
  and hit testing. Grid targets never enter `SnapTargets`.
- Snap hold: `drag.state.clear()` on a version change also clears `SnapState.grid`
  (`session/snapping.rs:261`, `ui-core snapping.rs:351`). Pages are re-read only at the press
  and on a version change.
- Older tester tests were only updated mechanically: tab lists and counts got `"grid"`, the
  format-version asserts went to 29, struct literals got `..DocumentSnapping::DEFAULT`, and
  the malformed v29 fixtures were added to the skip list of
  `acceptance_0017_block1_tester.rs:573`. No assertion was weakened.
- Budget test uses thread CPU time through `tests/cpu_time/` (`tests/grid_budget.rs:8,121,172`).

## Non-blocking findings

1. `curvyo-ui-core/src/grid/snap.rs:379` — §5 explicit tolerance. `along.x == 0.0` decides
   Vertical vs Horizontal, but `axis_aligned` (line 370) accepts any angle up to `ANGLE_EPS`.
   A stored angle in `(0, 1e-12]` (possible from a file, since `normalize_angle` only zeroes
   values near 0 and `TAU`) would draw the vertical marker as a horizontal one. Fix: match on
   the feature/direction index (the first direction of an intersection and the only direction
   of `U` are vertical), or test `along.x.abs() <= ANGLE_EPS`.
2. `curvyo-ui-core/src/lib.rs:247-250`, `grid.rs:67-69`, `snapping.rs:338` — §5 no
   speculative surface. `snap_to_grid`, `snap_to_grid_along`, `GridCandidate`, `GridFeature`
   and `HeldGrid` have no user outside `curvyo-ui-core`, which only calls `resolve_with_grid`.
   `ScreenPx` is re-exported at the crate root (`lib.rs:481`) with no outside user either. Fix:
   make them `pub(crate)`, and make `SnapState.grid` `pub(crate)` like `curve`. Only
   `grid_layout`/`GridLayout`/`GridLine`, `resolve_with_grid`, `grid_snap_nodes` and
   `GridSnapInput` stay public. This also removes the public bare-`f64` grid coordinates in
   `GridFeature`.
3. `curvyo-ui-core/src/snapping/grid.rs:161-188` — §5 one responsibility per module. The
   module doc says "combination of grid snapping with the indexed resolver", but the module
   also builds the Nodes probe points (`grid_snap_nodes`). Fix: move `grid_snap_nodes` next to
   `collection::moving` in `snapping/collection.rs`, which already decides which objects move.
4. Module doc comments with "and" (§5, one sentence without "and"):
   `curvyo-ui-core/src/grid.rs:1` ("layout and snap targets"), `grid/frame.rs:1` ("origin and
   rotation, and the map"), `grid/log.rs:1-3`, `curvyo-editor-wasm/src/session/grid.rs:1`
   ("commands and record"). Fix: one sentence each, for example `grid.rs` "Page grid geometry
   for one view."; `frame.rs` "The map between document and grid coordinates of one page.";
   `session/grid.rs` "The Grid panel's host API for the active page's grid."
5. `curvyo-editor-wasm/src/session/grid.rs:115-116` — `PageGridError::NotFound` becomes the
   notice key `"number"`. The active page always exists, so this cannot happen today, but the
   mapping says something false. Fix: map `NotFound` to `Ok(false)` (nothing written, no
   notice) or mark it with `// invariant: the active page exists`.
6. `curvyo-editor-wasm/src/session/snapping_node.rs:48-58` — the grid probe is "the moving
   anchor nearest the pointer", a guess at the pressed anchor. When two selected anchors lie
   within the press offset, the wrong one snaps. Fix: when the node drag starts, keep the
   pressed anchor's identity (the node tool already knows it) and use only that anchor.
7. `curvyo-document-core/src/document.rs:295` — "for this SID build" is unexplained jargon in
   a format doc. Fix: "older files open with the defaults; old-file compatibility waived
   (customer, 2026-10-30)".
8. `curvyo-ui-core/src/grid/tests.rs` is 513 lines (§5, ~500). It is a test module, so this is
   acceptable. Name it in the PR, or split the snap tests into `grid/snap_tests.rs`.

## Accepted deviations (architect records them in `adrs.md`)

- One history label "Change grid" for every grid edit: this matches decision 1.
- A default grid and default grid-snapping values are left out of `document.json`
  (`pages.rs:26`, `snapping.rs:164-169`). This is accepted: `document.json` is a derived
  export, the `.curvyo` Loro registers are the source of truth, and older goldens'
  `document.json` stay byte-identical. `adrs.md` decision 1 still says "so `document.json`
  carries it" and gets a dated note.
- Snapping pixel zones stay bare `f64`. A debt entry exists (`docs/technical-debt.md`, last
  section).
- `HeldGrid` has no `level` field. Instead, `retain` checks again each frame that the held
  line is still drawn at full opacity (`grid/snap.rs:232-241`). The release behaviour is the
  same and nothing goes stale. Gets a dated note in `adrs.md` decision 5.
- Fixed mode fades between 12 and 4 px (`grid/level.rs:97-131`), following spec criterion 6.
  `adrs.md` decision 4 still says "alpha 1 while s ≥ 4 px" and gets a dated note.
- Angles of 90°, 180° and 270° count as rotated (atomic competition, curve marker). The
  result is correct because the spec only asks for document axes at 0°. No change.

`adrs.md` is not edited now, because the tester is committing in this checkout. The four
dated notes above go in after the tester's push.
