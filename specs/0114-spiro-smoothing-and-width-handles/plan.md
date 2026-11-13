# Implementation plan — smoother Spiro and width handles

## Affected crates and modules

- `curvyo-geometry-core`: `spiro_solver.rs` (runs between corners, coincident controls), `spiro_solver/clothoid.rs` (the fixed G2 iteration), `spiro.rs` (`solve_spiro`, `spiro_cubics`); since the rework of 2026-11-12 `spiro_solver/newton.rs` (G4), `solve_spiro_near`, `SpiroShape::loops`, `SPIRO_MAX_*` and `SpiroError::{Far, NoConvergence}` are gone, and `curvyo-ui-core` loses `solve_spiro_nodes_near`.
- `curvyo-editor-wasm`: `session/spiro_cache.rs` (a memo of the solve), `session/spiro_hold.rs` (removed 2026-11-12), `session/width_edit.rs` with `width_edit/display.rs` (moved out) and `width_edit/wheel.rs` (new), `session/select.rs` (`DoubleClickHint::WidthEntry`), `wasm_pen.rs`.
- `curvyo-render-core`: `width_handles.rs` (diamond look, `WIDTH_DIAMOND_PX`).
- Frontend: `WidthEditToggle.tsx` (replaces `WidthDoneButton.tsx`), `WidthPointEntry.tsx`, `SelectToolbar.tsx`, `NodeToolbar.tsx`, `StrokeSection.tsx`, `useEditorSession.ts` (wheel, double-click), `App.tsx`, `Canvas.tsx`.

## Tasks

- [x] 1. Measure (criterion 2): `spiro_cache/measure.rs` walks the Pen's live point (circle r15 and r40, doubling back, a raster; 0.5 and 6 mm steps) and reports stall share, lag, largest move between events and shown-versus-reopen. Finding: the G4 curve itself loops or fails over large regions (a four-point wave with its last point 30 mm up loops in its first segment); the policy (180° loop rule for moves, refusal of any other branch, 1 mm/event creep) stalled on top.
- [x] 2. G2 Spiro (criterion 1), test-first: `spiro_g2_reference_0114.rs` (circle, closed circle, continuity, end arcs, interpolation). The G4 reference fixtures and the six-point G4 reference test are removed (superseded); `spiro_near_0110.rs` rewritten for G2.
- [x] 3. (Superseded by item 14: the G4 fallback is removed.) Following (criteria 2, 3): loop = 270° sweep, solve from scratch for moved controls when near, snap of at most 9.5 mm to the solve from scratch, G4 fallback capped at 8 segments for the budget. Regression test `the_live_spiro_point_rarely_stays_behind_and_never_jumps`; `pen_modes_0110` fold test now follows past the old fold.
- [x] 9. Tester review fixes (criteria 2, 3, 5, 7), see "Tester review 2026-11-12" below.
- [x] 4. Edit width in the Select and Node bars (criterion 4), Style panel keeps Profile and Point.
- [x] 5. Diamonds 16 px, white fill, dark ring, white casing; hit radius 11 px (criterion 5).
- [x] 6. Double-click typed width (criterion 6): `DoubleClickHint::WidthEntry`, `WidthPointEntry.tsx`; test `a_double_click_on_a_point_opens_the_typed_width_without_a_step`.
- [x] 7. Wheel (criterion 7): `width_wheel`, `end_width_wheel`, `width_point_under`; the host ends a burst after 500 ms; tests in `width_edit/wheel_tests.rs`.
- [x] 8. Browser: `frontend/tests/browser/pen-modes.cjs` at 1440 × 1000 and 800 × 600 (toggle in the bar, no Spiro notice, typed width, wheel without zoom and one undo, Done).

## Measurements (release, `print_spiro_follow_figures`, 0.5 mm steps; `origin/main` → this branch, same harness)

| Walk | Stall share | Largest lag | Largest move between events |
|---|---|---|---|
| Circle r 15 mm round the last node | 67 % → 7 % | 27 → 7 mm | 2.3 → 9.1 mm |
| Circle r 40 mm round the middle node | 68 % → 53 % | 80 → 80 mm | 1.6 → 2.2 mm |
| Doubling back past the path | 60 % → 14 % | 39 → 34 mm | 1.9 → 8.9 mm (excluding the closing node) |
| Raster over the plane | 71 % → 26 % | live point lost → 52 mm | 11.5 → 9.4 mm |
| Hairpin (three controls on a line) | 100 % → 100 % | — | — |

The largest move between two events goes up on the circling and doubling walks: that is the snap of at most 9.5 mm that ends a hold (before, the point crept 1 mm per event and the curve barely moved, but stayed behind for most of the walk). The remaining stalls are loops (r 40: the point behind the first node needs a segment turning more than 270°) and the line behind a neighbour (SPIRO-WINDING). Three controls almost on one line give a huge circle in G2 (as in Inkscape); every fourth point then loops.

Budgets (release, thread CPU time): held Pen hover worst 2.1 to 2.2 ms over three runs on both `origin/main` and this branch (test limit 4 ms); `frame_start_budget_for_one_remote_edit_in_1000_objects` passes; `pen_target_budget` passes in the debug gate.

The tester's frame distance (`acceptance_0110_stability_tester.rs` `distance`) sampled triangle vertices only; a long flat span of the G2 curve read as an 11 mm move although the curve moved under 4 mm (same frames: 84 → 78 → 102 triangles). It now samples triangle edges every 1 mm (marked in the file, needs the tester's re-check).

## Tester review 2026-11-12 (three failures in `acceptance_0114_tester.rs`, architect findings)

- **What is shown is what a reopen draws (criterion 2).** A moving Spiro point now only ever shows its controls solved from scratch (`spiro_cache/entry.rs`, `Entry::following`); the near-solution following is gone, and so is the follow of stored paths after an undo or nudge. A live point added after a click had no jump bound at all (the hairpin's 585 mm move): it is measured like a move now. The curve may move at most 9.5 mm per pointer event both ways (4× the pointer's move + 2 mm when the pointer itself jumps farther, Proposal); the hold's three halvings use the solver's first try only (`quick_shape`, the same shape whenever it finds one), which keeps a held hover within the budget.
- **F1 click while held (lead, Proposal).** A Pen click places the control where it is shown, not at the raw pointer.
- **F3 (lead).** A width point is hit within 11 px, no longer the larger of 11 px and the 16 px point tolerance.
- **F4 (lead, Proposal).** Over a width point every wheel event, Ctrl or Meta and a pinch too, sets its width and never zooms or pans.
- Architect: `Entry` and its rules moved to `spiro_cache/entry.rs`; `// invariant:` comment on the G4 seed test; `measure.rs` uses `SpiroShape::loops`; tests for a G4-fallback Node drag (commit and reopen draw what was shown), the eight- against nine-segment fallback and an SVG export golden (`tests/spiro_0114.rs`).

Measurements after the review (release, `print_spiro_follow_figures`, 0.5 mm steps; before = this branch at `973fb450`):

| Walk | Stall share | Largest move between events | Shown ≠ reopen |
|---|---|---|---|
| Circle r 15 mm round the last node | 7.4 % → 7.4 % | 9.1 → 9.1 mm | 0 → 0 |
| Circle r 40 mm round the middle node | 53.3 % → 53.3 % | 2.2 → 1.0 mm | 0 → 0 |
| Doubling back past the path | 14.4 % → 5.6 % | 8.9 → 8.9 mm | 0 → 0 |
| Raster over the plane | 26.2 % → 24.0 % | 9.4 → 9.5 mm | 0 → 0 |
| Hairpin, circle r 20 mm | 100 % → 98.3 % | 79 → 4.7 mm (tester's walk: 586 mm) | 143 → 0 |
| Hairpin, raster | 80 % → 99.9 % | 4.2 → 0.0 mm | 142 → 0 |

The hairpin's three controls already loop (a 300 mm circle), and nearly every fourth point makes a new loop or no curve, so its live point mostly stays at the last control; the tester's walk has no stall limit there. Held Pen hover (`spiro_cpu_budget.rs`, release): worst about 2.7 ms (limit 4 ms).

Not changed: a run of more than eight segments without any G2 curve still tries the G4 curve (the ADR says up to eight); a ten-control tight zigzag has neither and is drawn as its control polygon, as before.

## Tester review 2026-11-12, second round (similarity of the controls)

`the_curve_follows_a_similarity_of_its_controls` and `a_turned_solvable_curve_still_solves` (`acceptance_0114_tester.rs`) found that a turned Spiro could fail to solve or land on another branch. Cause: `solve` scaled each run by the larger side of its axis-aligned bounding box, which changes by up to 41 % when the controls turn; the residuals of curvature and its derivatives scale with that unit, so the absolute thresholds (residual, runaway, damping by energy) let a hard curve converge at some angles only, and the wound last resort picked a different seed's answer. Now every run is solved about its centroid in units of the root mean square distance of its controls from it (`unit_scale`), which a turn, move or scale does not change. Bends and seeds were already relative to the chords. The two tests pass, also with 3000 proptest cases (release).

Measurements (release, `print_spiro_follow_figures`, 0.5 mm steps; before = `8a1e2473`):

| Walk | Stall share | Largest move between events | Shown ≠ reopen |
|---|---|---|---|
| Circle r 15 mm round the last node | 7.4 % → 7.4 % | 9.1 → 9.1 mm | 0 → 0 |
| Circle r 40 mm round the middle node | 53.3 % → 53.3 % | 1.0 → 1.0 mm | 0 → 0 |
| Doubling back past the path | 5.6 % → 5.6 % | 8.9 → 8.9 mm | 0 → 0 |
| Raster over the plane | 24.0 % → 24.1 % | 9.5 → 9.5 mm | 0 → 0 |
| Hairpin, circle r 20 mm | 98.3 % → 98.3 % | 4.7 → 4.7 mm | 0 → 0 |
| Hairpin, raster | 99.9 % → 99.9 % | 0.0 → 0.0 mm | 0 → 0 |

Budgets (release, thread CPU time, three runs): held Pen hover worst 2.43-2.49 ms → 1.39-1.50 ms (limit 4 ms); `frame_start_budget_for_one_remote_edit_in_1000_objects` 3.63-3.87 ms → 3.46-3.74 ms.

## UX review 2026-11-12

- [x] 10. **A held Spiro end rejoins** (criterion 2). Circling the last of three points in about 7 mm steps froze the end after 4 of 27 events (browser, `pen-modes.cjs`). A held point now steps toward the pointer by at most 9.5 mm per event when the whole way is refused (then halvings of that step), and where the straight step would loop it tries steps turned 35° and 70° off the way that still come nearer (`spiro_hold.rs`, `REJOIN_MM`, `SIDE_TURNS`, both Proposals). The curve rules are unchanged: no new loop, no move above the jump limit, shown = reopen. The capped step also applies to a held Node drag. Tests: `a_held_spiro_point_rejoins_a_pointer_circling_fast` (`measure.rs`, new walk "wave circle r30"), the browser circling in `pen-modes.cjs` (desktop 4 → 15 of 27 events at the pointer, 800 × 600 20 of 27).
- [x] 11. Typed width below the diamond (above without room), kept inside the canvas, flipped left at the right edge; "Must be 0 or more." on one line under the field; a wheel readout "Width 2.5 mm" above the diamond, gone 800 ms after the last notch (`placeWidthChip`, `WidthWheelReadout`, `Session::width_point_screen`).
- [x] 12. Edit width: 14 px text, bordered like Sharp and Smooth, on its row's line; the ghost diamond keeps its full ring (fill at half opacity).

Measurements (release, `print_spiro_follow_figures`; before = `d9353a67`, "plain run" = longest run of events behind a pointer whose own curve is fine):

| Walk | Step | Stall share | Plain run | Largest move between events |
|---|---|---|---|---|
| Circle r 15 mm round the last node | 0.5 mm | 7.4 % → 4.6 % | 6 → 0 | 9.1 → 8.3 mm |
| Circle r 30 mm round the last node (new) | 6.7 mm | 41.1 % → 19.2 % | 10 → 2 | 20.1 → 21.9 mm (limit 4 × 6.7 + 2) |
| Circle r 30 mm round the last node (new) | 0.5 mm | 64.3 % → 49.2 % | 269 → 149 | 9.5 → 9.5 mm |
| Circle r 40 mm round the middle node | 0.5 mm | 53.3 % → 53.3 % | 120 → 120 | 1.0 → 9.3 mm |
| Doubling back past the path | 0.5 mm | 5.6 % → 4.6 % | 0 → 0 | 8.9 → 5.4 mm |
| Raster over the plane | 0.5 mm | 24.1 % → 23.4 % | 29 → 29 | 9.5 → 9.5 mm |
| Hairpin, circle r 20 mm | 0.5 mm | 98.3 % → 46.0 % | 89 → 0 | 4.7 → 9.5 mm |
| Hairpin, raster | 0.5 mm | 99.9 % → 69.8 % | 84 → 47 | 0.0 → 9.1 mm |

Shown ≠ reopen stays 0 on every walk. What stays: at slow steps the r 30 end waits at the winding fold behind the last point (the only curves toward the pointer differ from the shown one by more than 20 mm, so rejoining there needs a jump above 9.5 mm or a looped curve; SPIRO-WINDING A), and r 40 waits behind the first node as before.

Budgets (release, thread CPU time, three runs): held Pen hover worst (24-point wave, r 12 mm) 1.55 → 2.6-2.9 ms (limit 4 ms; the side steps cost up to four more first-try solves); `frame_start_budget_for_one_remote_edit_in_1000_objects` 3.64-3.78 ms.

## Customer decisions (2026-11-12)

SPIRO-G2: A. Every Spiro, also those saved since 2026-11-11, is drawn as the G2 curve; no format change. This is an explicit exception to `0110` criterion 8 (see the specification). B (flag per path and format bump) and C (back to G4 with looser hold rules) were rejected.

SPIRO-WINDING: A. As implemented: behind the neighbour the point waits until the curve on the other side is within 9.5 mm, then the curve turns over. B (store each point's winding, format change) was not chosen.

## Functions over 60 lines (justified, CLAUDE.md section 5)

- `spiro_cubics` (62, `spiro.rs`): the checks of shape and tolerance, the straight segments and the polygon fallback share the `ends` and `straight` closures; split if it grows.

## Decide at go (Proposals)

- The 270° sweep cap and the four-turn bend cap of an iterate, coincident controls as one point, the control polygon for a shape that cannot be approximated, 16 px diamonds, 0.5/0.1 mm wheel step, 500 ms burst, the Done label on the toggle. (The 9.5 mm snap, the 270° loop limit of moves and the G4 fallback are gone with the rework of 2026-11-12.)

## Rework 2026-11-12: the Spiro as a pure function of its controls (customer, after trying PR #349)

The customer found the held point "horrible": far from the pointer, slow, stepping back in. Inkscape rebuilds the curve from the points on every pointer move and draws the last of at most ten Newton steps.

- [x] 13. Measure first (criterion 2): `spiro_cache/measure.rs` now reports stalls, the largest curve move between events, the share of events where the curve moves more than 4× the pointer's step + 2 mm ("not smooth"), events drawn as the control polygon, shown ≠ reopen and the CPU time of hover + frame; run on the old head for "before".
- [x] 14. G2 or G4 (criterion 1): both with the fixed procedure (from zero, ten full steps, wrapped angles and turns, last valid iterate). Converged (accurate residual < 1e-6): random open sets of 3 to 8 controls G2 91.0 % / G4 59.2 %; random closed 92.6 % / 45.5 %; wave with the pointer on a 2 mm grid 99.9 % / 83.2 %; hairpin with the pointer on the grid 99.7 % / 37.1 %. Without a loop as well: 58.2 / 40.7, 64.6 / 32.2, 76.9 / 63.6, 9.0 / 14.7 %. G2 stays; the G4 fallback, `newton.rs` and the near solve are removed.
- [x] 15. The solve (`clothoid.rs::solve_g2`, criteria 1 and 2): from the control polygon, at most ten full Newton steps, each knot's angle error reduced to one turn and each segment's turn to two after every step, stop at a squared step below 1e-12, the last iterate is the curve. First run without caps: the last arc of a wave swept up to 359° (a 5.9 m circle with the pointer just behind the last control), the hairpin raster had a 141 km curve and one event took 7 s. Proposal: a step whose iterate sweeps more than 270° or bends more than four turns is cut by bisection (40 halvings) to its longest drawable part (a halving instead put 66 to 728 events per walk on the control polygon). Coincident neighbours are one point; a shape whose segment cannot be approximated is drawn as its polygon. Tests: `hard_controls_give_a_finite_curve_through_them`, `coincident_controls_are_one_point`, proptest `any_finite_controls_give_a_finite_curve` (`spiro_g2_reference_0114.rs`); the tester's `the_curve_follows_a_similarity_of_its_controls` passes with 3000 cases (release; with 12 bisections one case was 0.0209 mm off, the regression seed is kept) and `a_turned_solvable_curve_still_solves`.
- [x] 16. Session (criterion 2): `spiro_hold.rs`, `Entry::following`, `quick_shape`, `jump_limit`, the `Held`/`refused`/`retired` state, `try_preview`, `place_preview`, `node_spiro_pointer`, `spiro_hold_pointer` and the Pen's place and close gates are removed; the Pen preview, a placed point, a Node drag and a stored path all draw `Entry::fresh` of their controls. `SPIRO_UNSOLVED_NOTICE` stays only for a Spiro without a curve (too few points, several subpaths: only an outside file), reworded.
- [x] 17. Tests rewritten to the new contract (for the tester's re-check): `acceptance_0114_tester.rs` c2 (no stalls; at most one turnover per round of the last node and two when doubling back, each ≤ 15 mm; turnovers above 10 mm at ≤ 1 % of the events of every walk; the amended-distance re-check on the worst pair the exact measure saw); `acceptance_0110_stability_tester.rs` (circling and doubling back ≤ 15 mm, the turnover; Node drags onto and round the neighbour move more than 10 mm at no more than two events farther than 2 mm from the neighbour (measured 0 and 2; within 2 mm of it the segment vanishes, up to 54 mm), the far drag keeps 10 mm); `spiro_0114.rs` (the zigzag is a finite G2 curve, its Node drag stores and reopens what it showed, the export golden's zigzag path changed, the wave and ring paths are byte-identical); `spiro_near_0110.rs` deleted (near solve gone); the cache tests now pin "the preview is the curve of its controls wherever the point came from" and "an unconverged curve can be moved"; `svg_export` exports a Spiro on one spot; the trace mask, trace parts, text carrier and ui-core mask tests that used a Spiro on one spot as "no curve" use one 1e300 mm out instead. A straight segment is one cubic with its handles at the thirds, as before.
- [x] 18. Budgets (criterion 3): `spiro_cpu_budget.rs` "a live Spiro rebuild per pointer event fits two milliseconds" (hover + frame, waves of 3, 8 and 20 controls and the hairpin).
- [x] 19. Browser (`frontend/tests/browser/pen-modes.cjs`): circling the last of three points in 48 steps, the end is at the pointer at 59 of 59 events, desktop and 800 × 600 (before: 15 of 27 and 20 of 27 with the hold).

Measurements (release, thread CPU time, `print_spiro_follow_figures`; before = `cffce8a3`, the hold):

| Walk | Step | Stalls | Largest move | Not smooth | Time max / p95 |
|---|---|---|---|---|---|
| Circle r 15 round the last node | 0.5 mm | 4.6 % → 0 | 8.3 → 12.3 mm | 0.9 → 0.5 % | 2.27 / 1.29 → 0.32 / 0.21 ms |
| Circle r 15 | 6.7 mm | 5.5 % → 0 | 11.1 → 12.6 mm | 0 → 2.7 % | 1.88 / 1.32 → 0.22 / 0.21 ms |
| Circle r 40 round the middle node | 0.5 mm | 53.3 % → 0 | 9.3 → 75.9 mm | 1.2 → 0.2 % | 2.25 / 1.48 → 0.29 / 0.22 ms |
| Circle r 40 | 6.7 mm | 53.4 % → 0 | 14.3 → 76.6 mm | 0 → 1.4 % | 1.63 / 1.42 → 0.22 / 0.22 ms |
| Doubling back | 0.5 mm | 4.6 % → 0 | 5.4 → 12.5 mm | 0.4 → 0.7 % | 1.89 / 0.35 → 0.29 / 0.22 ms |
| Doubling back | 6.7 mm | 4.2 % → 0 | 9.1 → 13.4 mm | 0 → 0 % | 1.20 / 0.26 → 0.27 / 0.22 ms |
| Circle r 30 round the last node | 0.5 mm | 49.2 % → 0 | 9.5 → 32.7 mm | 1.3 → 0.3 % | 1.78 / 1.42 → 0.28 / 0.23 ms |
| Circle r 30 | 6.7 mm | 19.2 % → 0 | 21.9 → 34.5 mm | 0 → 2.7 % | 1.54 / 1.45 → 0.25 / 0.23 ms |
| Raster | 0.5 mm | 23.4 % → 0 | 9.5 → 44.6 mm | 0.6 → 0.1 % | 2.08 / 1.55 → 0.30 / 0.22 ms |
| Raster | 6.7 mm | 23.2 % → 0 | 14.4 → 42.2 mm | 0 → 1.4 % | 2.34 / 1.60 → 0.27 / 0.23 ms |
| Hairpin circle r 20 | 0.5 mm | 46.0 % → 0 | 9.5 → 37.4 mm | 2.1 → 0.7 % | 1.79 / 1.06 → 0.87 / 0.31 ms |
| Hairpin circle r 20 | 6.7 mm | 45.2 % → 0 | 9.5 → 35.4 mm | 2.7 → 1.4 % | 1.29 / 1.19 → 0.35 / 0.29 ms |
| Hairpin raster | 0.5 mm | 69.8 % → 0 | 9.1 → 42.5 mm | 0.3 → 0.6 % | 23.08 / 1.85 → 0.71 / 0.31 ms |
| Hairpin raster | 6.7 mm | 68.6 % → 0 | 26.6 → 37.2 mm | 0 → 2.1 % | 3.90 / 1.82 → 0.39 / 0.35 ms |

Waves circled 12 mm from the last control (time max / p95): 3 controls 1.37 / 1.18 → 0.20 / 0.18 ms, 8 controls 4.40 / 2.93 → 0.26 / 0.25 ms, 20 controls 9.87 / 2.19 → 0.75 / 0.39 ms. Polygon frames 0 and shown ≠ reopen 0 on every walk, before and after. `frame_start_budget_for_one_remote_edit_in_1000_objects` 3.59 ms.

What could not be made smooth: the largest moves are turnovers, not lag. Where the pointer crosses the line behind a neighbour (circle r 15 and doubling back: 12 to 13 mm, about the distance to that neighbour), behind the first node of the wave (circle r 40: 76 mm, the whole curve turns over) and over the hairpin's nearly straight run, the curve flips to the other side between two events; a curve that is a function of the controls must flip somewhere on such a walk (SPIRO-WINDING A). A tight zigzag without a G2 curve is drawn as the last iterate, which loops more than the G4 curve of the first `0114` build.
