# ADRs for "Weld"

Architect, 2026-10-20, for the reworked specification (customer: Frage 15 WELD-DEF as written,
Frage 14 WELD-FILL A). This file replaces the `adrs.md` of PR #167.

Weld is Flatten's fold from the top (0037). Each step does not cut the operand's region by
the running union. It clips the operand's **boundary** by that union, as an open string, with
`i_overlay`'s string clip. Then one more string clip against the final union splits the kept
lines into the union boundary (closed outlines) and the inner lines (open outlines). Our own
graph walk on the grid chains them. **No new ADR, no new crate, no new dependency, no
`Cargo.toml` change, no `format_version` bump.** The result is an ordinary path or a compound
path with open outlines (0054). No trait, no generic.

## Depends on

- [ADR 0003 §1, §3, §8](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  flattened polygons, `i_overlay` `=9.0.1` in its `i64` engine on the 0.001 mm grid, polyline
  results, no backend trait. The string clip (`i_overlay::string`) is part of the same pinned
  crate and the same integer engine, so it is not a second boolean algorithm (criterion 26).
- [ADR 0002 §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): the
  result takes the lowest selected object's place and is written in one commit.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one step, label `weld`.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md): a linked
  clone contributes the resolved path it shows, read in world coordinates.
- [ADR 0020 §1, §5](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) and
  [`specs/0074-text-objects/adrs.md`](../0074-text-objects/adrs.md): a text enters as its glyphs
  (`texts_refusal`, `texts_as_paths` of `ui-core/src/text_convert.rs`). A missing face is
  refused.
- [`specs/0016-boolean-operations/adrs.md`](../0016-boolean-operations/adrs.md):
  `operand_regions` (checks, `grid_paths`, `normalize`), `run`, `cleanup`,
  `canonical_millimetres`, refusal codes with offenders, `MAX_COORDINATE_MM`, and fixtures (a)
  to (i).
- [`specs/0037-fracture-and-flatten/adrs.md`](../0037-fracture-and-flatten/adrs.md): the fold
  from the top with its running union and `GridBox` test, `has_area`, and the
  `performance-budgets` job.
- [`specs/0054-combine-open-paths/adrs.md`](../0054-combine-open-paths/adrs.md): an open
  outline's region is closed by a straight line. A compound path may hold open outlines.

## Feature-local decisions

1. **2026-10-20: which lines are weld lines (criterion 14).** `O_k` is operand k's normalized
   region, `A_k` is the union of the operands above it, and `V_k = O_k − A_k`. A point lies on
   some `∂V_k` exactly when it lies on `∂O_k` for some k **strictly outside** `A_k` (not inside,
   not on its boundary). A point on `∂O_k ∩ ∂A_k` also lies on the boundary of a higher operand.
   The topmost operand whose boundary holds the point holds it strictly outside its own `A`.
   So **every weld point is emitted once, by the topmost operand whose boundary holds it.**
   - Identical squares: only the upper one is emitted.
   - Squares that share an edge: the shared edge comes from the upper one.
   - A trim edge comes from the upper shape. The lower shape's outline under it is clipped
     away.
   - Lines inside one operand's own region are not on `∂O_k` (criterion 10), because `∂O_k` is
     the boundary of the **normalized** region, not the raw contours.

   Rejected options:
   - Extract the edges of the `Visible::Trimmed` outlines and merge coincident ones. Every
     `V_k` comes from a separate overlay that rounds its crossings separately. A shared edge
     then shows up as two segments up to a few grid units apart, which needs a tolerance merge
     (noding with snapping). That is the robustness surface 0037 refused to own.
   - Our own planar arrangement (DCEL), rejected for the same reason.
   - One overlay of all `V_k`: it cancels opposite coincident edges (shared edges vanish), and
     with two fill colours it cannot tell n regions apart.

2. **2026-10-20: the algorithm (`curvyo-geometry-core/src/weld.rs`, rewritten).**
   1. `regions = operand_regions(operands, tolerance)?`, which gives the same checks and
      refusals as 0016.
   2. Fold from the top, as `visible_regions`, keeping `above` and its `GridBox`. For operand k:
      - If its box meets `above`'s box, its region's contours are added with
        `StringOverlay::add_string_contour` (closed strings). `above` is the shape. They are
        clipped with `FillRule::NonZero` (as `run`) and
        `ClipRule { invert: true, boundary_included: false }`, which keeps the parts strictly
        outside.
      - Otherwise the contours are kept whole.
      - Only `above`'s contours whose box meets the operand's box go into the shape. A contour
        whose box misses the operand's box has winding 0 there, so the result is exact.
      - Then `above := run(Union, above, O_k)`.
   3. **Hidden:** if an operand's kept strings are empty, test
      `has_area(run(Difference, O_k, above_before))`. If that is false, the operand is
      reported hidden. The difference only runs in that rare case. A disc that fills a ring's
      hole keeps no line but is still visible.
   4. `U = above` after the last step.
   5. **Inner lines:** one more string overlay. The shape is `U`, the strings are all kept
      pieces, and the clip is `ClipRule { invert: false, boundary_included: false }`, which
      keeps the parts strictly inside `U`. The parts on `∂U` are dropped, because `∂U` is
      emitted from `U` itself.
   6. **Closed outlines:** `canonical_millimetres(&cleanup(&U))`, exactly as Union does
      (criterion 15: outer, holes, islands, 0016 criterion 41 winding).
   7. Chain the inner lines (decision 3).

   The implementer's first tests pin the `ClipRule` semantics on identical squares, squares
   with a shared edge, and criterion 4. The library doc says that `boundary_included`
   "toggles" with `invert`. If the meaning turns out reversed, the flags flip. The design does
   not change.

   Cost:
   - n unions and up to n + 1 string clips, against Flatten's 2n overlays.
   - No cleanup between steps (0037 drift rule).

3. **2026-10-20: chaining on the grid (criteria 14, 15).** This is our own walk, not
   `i_overlay`'s string chaining, which runs straight through junctions.
   - Every inner piece is split into grid segments. Each is stored undirected as `(min, max)`
     in a `BTreeSet`, so **exactly coincident segments are kept once**. That is the safety net
     for the rare double emission that rounding can cause.
   - Degree per grid point comes from a `BTreeMap<Point64, Vec<segment>>`. A **junction** is a
     point of degree other than 2. An inner line that ends on `∂U` has degree 1 there, so it
     ends there.
   - **Open outlines:** start at each junction in `(x, y)` order. Leave it along each unvisited
     segment, in the order of the far point `(x, y)`, and walk until the next junction. A chain
     is then stored from its smaller endpoint `(x, y)` to the larger one. Criterion 4 gives
     `(10, 20), (10, 10), (20, 10)`. A chain whose two ends are the same junction (a loop that
     touches another line at one point) stays open with equal end nodes.
   - **Closed inner outlines:** the segments left over form cycles of degree-2 points. Each
     starts at its smallest point and is wound as a shape, with positive area like an outer
     outline (criterion 5).
   - **Order in the result:** the closed outlines of `U` (canonical), then the closed inner
     outlines by first node, then the open outlines by `(first node, second node)`. Same
     stacking, same input order, same result, whatever the selection order (criterion 25).
   - **Nodes (criterion 16):** `boolean_cleanup.rs` gets a crate-private `cleanup_open(path)`.
     It is steps 1 and 2 of `remove_near_collinear`, with both endpoints as fixed anchors,
     factored out so the closed and open forms share it. It never moves an endpoint, so
     junctions stay joined. It runs no repair union, because a line has no region. Closed inner
     outlines use `remove_near_collinear` without the union. An open outline that is shorter
     than the kernel tolerance (0.01 mm) after cleanup is dropped: it cannot meet 0016
     criterion 24a. Points go to millimetres with `to_millimetres`, not
     `canonical_millimetres`, which rotates closed outlines.
   - **Known limit:** a lower edge that runs within one grid unit of an upper edge, but not
     exactly on it, can survive as a near-parallel double. This happens only where a shared
     non-axis-parallel edge was split by a snapped crossing. Axis-parallel edges never tilt.
     The criterion 14 property test (200 sets, fixed seed) decides. If it fails, the fix is in
     the fold, not a new algorithm: drop a kept piece whose every point lies within one unit
     of `∂above` (the `EdgeIndex` of `region_contact.rs` finds the near edges). Record it as a
     dated note here.

4. **2026-10-20: curves.** Every outline is flattened at the kernel tolerance, the untouched
   ones included, as in Union. Weld writes one new object, so Flatten's `Untouched`
   pass-through does not apply. Refit stays the 0016 follow-up.

5. **2026-10-20: kernel API.** It is exported from `curvyo-geometry-core`.

   ```rust
   pub struct Welded { pub outlines: Vec<(Vec<Point>, bool)>, pub hidden: Vec<usize> }
   pub fn weld(operands: &[&[Outline<'_>]], tolerance: Tolerance) -> Result<Welded, BooleanError>
   ```

   - `bool` is "closed".
   - `hidden` holds operand indices, bottom first.
   - The errors are 0016's `BooleanError`. An empty `U` gives `EmptyResult`.
   - `StackError` is not used, because Weld has no piece limit.

6. **2026-10-20: operands, style and the write (`curvyo-ui-core/src/weld_plan.rs`, kept and
   trimmed).**
   - The selection is expanded with `leaves_of_unit`, at any group depth, with clones resolved
     in world coordinates.
   - `texts_refusal` runs first (criterion 23). Then `texts_as_paths` converts the texts, each
     glyph as its own operand in 0074 order.
   - Operands are sorted by stacking order.
   - Operands with no area (`outline_has_area` false for all their outlines) are dropped and
     counted for the 0054 sentences. Only when none is left is Weld refused with 0016's "no
     area" sentence (criterion 24).
   - The base is the lowest leaf. Style is a copy of it with **Fill off**. If the base has
     Stroke off, Stroke is switched on in the base's fill colour at the default stroke width of
     a new path (criterion 18, 14 A).
   - The notice counts are: objects, all-texts, hidden (from `Welded::hidden`), styles
     differed, and Fill switched off.
   - The write is `Document::replace_with_path_at(operands, style_from, place, outlines,
     label)` from the old branch. `place` is the lowest selected object, which may be a group
     that is emptied and removed in the same commit (criterion 17).
   - `replace_with_path_at` **loses its `name` parameter**: the new spec names no result
     (criterion 19), and a parameter with no caller is dead code.
   - Label `weld`, which `history_names.rs` maps to "Weld".

7. **2026-10-20: the Boolean card, button 8.** The button goes in `BooleanCommands.tsx` after
   Flatten, with its glyph in `BooleanGlyphs.tsx`. Its availability differs from the other
   seven buttons: Select tool and **at least one** selected object. That is one predicate next
   to the card's existing one, not a parameter on it. `session/weld.rs` keeps
   `apply_weld()` **without an argument**, and `wasm_weld.rs` follows. The column B overflow at
   800 x 600 is the ux-engineer's layout, with no architecture impact.

8. **2026-10-20: budgets and thread.** Synchronous on the UI thread, like Flatten, with no busy
   state and no new limit.
   - Fixtures (a) to (e) of criterion 27 go into `tests/fixtures/weld/`, with golden files
     compared natively and in the wasm build at 1e-6 mm.
   - The budget tests join the `performance-budgets` job, replacing `weld_golden` and
     `weld_budget` of PR #167.
   - (e), 2,000 squares, is the 0037 chain, at about 2,000 unions and 2,000 clips. If (b) or
     (e) misses on CI, the implementer logs it in `docs/technical-debt.md` and the slice ships,
     as Flatten did.

9. **2026-10-20: what PR #167 (`story/weld`) keeps and what goes.** Rebuild `story/weld` from
   `origin/main` (main has 0074 rebased). Cherry-pick what is kept and edit it.

   | Kept (edited) | Removed |
   |---|---|
   | `document-core/src/replace.rs` `replace_with_path_at` and its test (without `name`) | `geometry-core/src/stroke_band.rs` and the `IntStrokeOffset` use |
   | `ui-core/src/weld_plan.rs`: operand collection, texts, base, place, refusals, notice counts | `weld.rs` gap closing (`gap_units`, `close_gaps`), `WeldOperand` (`banded`, `band`), the `pieces` count and the `Weld` struct |
   | `editor-wasm` `session/weld.rs`, `wasm_weld.rs`, `WeldOutcome` (without `pieces`) | the `union_region` refactor in `boolean.rs` and the `Paths64` helper split in `offset_polyline.rs` (both go back to main's form) |
   | frontend `useWeld.ts`, `weldText.ts` (new sentences) | `ui-core/src/weld_gap.rs`, `clamp_weld_gap`, `WELD_GAP_DEFAULT` |
   | `history_names.rs` entry | `WeldGroup.tsx`, the Select-bar wiring (`SelectToolbar.tsx`; `FlipGroup.tsx` and `GuideButton.tsx` changes made only for the group), the `curvyo.weld-gap-mm` key (never shipped, so no migration) |
   | | the dash sentence, all old goldens and acceptance tests (they encode gap closing) |

10. **2026-10-20: the Cuttable check of 0075 no longer uses Weld.** Weld has no gap and no piece
    count any more. `curvyo-ui-core/src/cuttable.rs` counts the pieces of the 0016 **Union**
    of the word's glyphs, with no gap:
    - one operand per glyph (one operand with all contours would cancel a hole of one letter
      against another letter);
    - glyphs with no area are skipped;
    - `boolean(BooleanOp::Union, ..)` at `BOOLEAN_TOLERANCE`;
    - pieces = outlines of even depth in `outline_nesting` (an island counts);
    - no glyph with area, or any refusal, gives 0.

    `CUTTABLE_GAP_MM` is deleted, and `CUTTABLE_CHECK_VERSION` becomes 2 so the caches are
    rebuilt.

    Rejected: a `closed_union_pieces(operands, gap)` in `geometry-core`, or the same code in
    `cuttable.rs`.
    - It would keep the morphological closing, the integer offsetter on `Paths64`, and its arc
      rules (about 150 lines and a crate-private helper split) alive for one check.
    - `cuttable.rs` cannot hold it anyway, because the grid helpers are crate-private to
      `geometry-core`.
    - The gap also makes the check optimistic: letters 0.05 mm apart are two pieces once cut.

    0075 no longer depends on 0068. `story/font-library` drops the two Weld commits
    (`git rebase --onto origin/main 193a519`).

## Flagged to the PO (no customer question)

1. **0075 wording.** "Weld it by 0068 at Gap 0.1 mm" (Part C), criterion 19 ("equals the
   piece count Weld's notice gives"), criterion 13 ("like Weld's Gap"), the `Depends on` line,
   and the chip tooltip "Detected: welds into 1 piece" need new wording. Proposal: "Union (0016)
   of the word, pieces counted"; criterion 19 becomes "equals the number of pieces Union makes
   of the converted text `cuttable` at 20 mm"; the tooltip becomes "Detected: one piece".
   0075's `adrs.md` decision 5 follows decision 10 above.
2. **Criterion 14 distances.** Cleanup keeps every input vertex within one grid unit per round
   (0016 `boolean_cleanup.rs`), and the reference boundary in the test is the flattened grid
   region, not the exact curve. The property test therefore measures against the grid regions
   of `operand_regions`, with 0.001 mm plus one unit of cleanup (0.002 mm). The overlap rule
   stays at 0.001 mm.

## Implementation notes (implementer)

- **2026-10-20, `ClipRule` (decision 2).** In `i_overlay` 9.0.1 "strictly outside" is
  `ClipRule { invert: true, boundary_included: true }`: with `invert`, "included" puts the boundary
  into the area that is clipped away. The flags flip as decision 2 foresaw; "strictly inside" is
  `{ invert: false, boundary_included: false }` as planned. Pinned by criterion 4's test.
- **2026-10-20, the known limit (decision 3).** The property test failed on it, so the fix went in,
  as decision 3 says, before the clip and without a new algorithm: a string gets every vertex of
  the shape that lies within one grid unit of a segment's interior inserted into that segment
  (`weld.rs` `noded`), both in the fold and in the inner-line clip. Each vertex is checked against
  the rest of the segment from the vertex inserted before it, which follows the drift that
  repeated unions leave (up to 4 units from the original line). A string moves by at most one unit
  per vertex. With it the 200 seeded sets pass; a run of 3,000 sets leaves 4 sets with a
  near-double under 0.02 mm long, either a genuine needle tip thinner than 0.001 mm or a rounding
  remnant.
- **2026-10-20, short open lines (decision 3).** Dropping every open line shorter than 0.01 mm
  dropped real visible boundary (a 0.008 mm trim edge where a corner just pokes out from under a
  shape) and broke criterion 14's second distance. Such a line is now dropped only when every point
  of it lies within two grid units of the union boundary (a rounding remnant); otherwise it stays.
  A 2-node line of at least 0.001 mm meets `0016` criterion 24.
- **2026-10-20, the fold keeps shapes.** The running union is kept as its connected shapes (an
  outer outline with its holes) with their boxes, and a step overlays only the shapes whose box
  touches the operand's; the others cannot meet it. Same overlay, same result; fixture (b) went
  from 3.6 s to 0.5 s in a release build.
- **2026-10-20, `GridBox::touches`.** The fold's box test is inclusive: a shared edge must be
  clipped, so `meets` (more than an edge) of 0037 is not used here.
- **2026-10-20, the write (decision 6).** `replace_with_path_at(operands, style, place, outlines,
  label)` takes the style itself instead of `style_from`: the plan builds the line-art style from
  the lowest leaf, so `curvyo-document-core` holds no Weld rule. No `name`.
- **2026-10-20, line art (criterion 18).** With Stroke off, the stroke takes the fill colour and the
  default width (0.25 mm); its stored opacity, dash, join and cap stay as they were.
- **2026-10-20, operands.** The base is the lowest selected leaf, with or without area. An operand
  whose normalized region is empty although an outline has area (two outlines cancelling) is left
  out and the rest welded again. The notice's length is formatted in Rust (`format_status_length`
  and the unit symbol), like the status bar.
- **2026-10-20, decision 10 not built here.** `cuttable.rs` lives on `story/font-library`
  (PR #168), which this branch does not touch; main has no Weld dependency to remove.
- **2026-10-20, Frage 16 WELD-MINH.** Confirmed by the customer as A: `minHeight` 640 in
  `curvyo-app/tauri.conf.json`, and the design-system row "Window minimum" (the rows "Boolean
  toolbox" and "Tool rail architecture" quote the 640 px figure).
- **2026-10-20, doubled lines after review (decision 3 fallback).** The tester found doubles one
  grid unit apart up to 14.5 mm long (12 of 3,000 sets): the fold and the final union round the
  same crossing to different grid points. Now, in `weld_lines.rs`: the inner pieces are noded
  once more against the cleaned union boundary; a grid segment that one boundary edge holds
  within two units (both ends and the midpoint) is dropped at any length (`EdgeIndex::hugs`); of
  two inner segments that hold each other so, the longer stays; a short open line (under
  0.01 mm) along the boundary or another open line is dropped. The tester's 3,000-set scan at
  0.002 mm finds 0 sets; the implementer's stricter scan at 0.001 mm finds 1 of 3,000, a needle
  tip under 0.001 mm wide in the union's own outline (Union gives the same).
