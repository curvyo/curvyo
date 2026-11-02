# 0071 Pattern fill: acceptance report (tester)

Date: 2026-11-02. Branch `story/0071-pattern-fill` at `608cf210` plus the tester commits.
Verdict: **FAIL** (one criterion not met as written, one performance defect below the hard limit;
everything else passes).

Tests written from the specification, ADR 0029 and the UX notes before the implementation was read:

- `curvyo-geometry-core/tests/acceptance_0071_tester.rs` (32 tests and 2 proptests): every generator
  in both modes and three margins on an L-shaped region with a round and a square hole, generator
  semantics, Lines/Shapes, seeds, limits, degenerate input.
- `curvyo-geometry-core/tests/acceptance_0071_bits_tester.rs` and
  `tests/fixtures/pattern/acceptance_0071_bits.golden`: a digest of every coordinate's `f64` bits
  for 40 cases; run natively (debug and release) and on `wasm32-unknown-unknown` with
  `wasm-bindgen-test-runner`: identical.
- `curvyo-document-core/tests/acceptance_0071_tester.rs` (21 tests): format 32, golden, older and
  newer files, damaged registers, seeds, copies, clones, history labels.
- `curvyo-editor-wasm/tests/acceptance_0071_tester.rs` (18 tests): the session as the host drives it.
- `frontend/tests/browser/pattern-fill-tester.cjs` (22 checks, 20 pass) plus the implementer's
  `pattern-fill.cjs` (ok, 137 paths in the export). Screenshots: `target/0071-browser-tester/`.

## Criteria

| # | Test | Result |
|---|---|---|
| 1 | session `c1_pattern_is_a_fill_kind…`; browser "criterion 1", "no rail tool" | PASS |
| 2 | session `c2_*` (preview, cancel, typed, one step); browser drag, Escape, release, undo | PASS |
| 3 | doc `c3_switching_to_colour…`; session `c3_c22_…`; browser "criterion 3" | PASS |
| 4 | doc `c4_the_picker_lists…`; session `c4_every_generator_tile…`; browser 10 tiles | PASS |
| 5 | geo `c5_hatch_…`, `c5_crosshatch_…`, `c5_grid_…` | PASS (see finding 3) |
| 6 | geo `c6_dots_are_round…staggered` | PASS |
| 7 | geo `c7_honeycomb_cells_are_regular_hexagons…` | PASS |
| 8 | geo `c8_brick_rows_shift_by_half_a_brick`; doc `c8_brick_wall_parameters…` | PASS |
| 9 | geo `c9_leaves_never_overlap…` (4 seeds x 3 densities), `c9_leaf_size…` | PASS |
| 10 | geo `c10_sea_waves_are_concentric_arcs…` | PASS |
| 11 | geo `c11_waves_with_amplitude_zero_equal_hatch` (exact), `c11_waves_are_sines…` | PASS |
| 12 | geo `c12_interpolation_first_and_last…`, `…concentric_half_ellipses`, `…holes_and_margin` | PASS |
| 13 | geo `c13_c14_c15_every_generator…`, `c13_nothing_lies_inside_either_hole`, proptest | PASS |
| 14 | geo `c14_hatch_lines_end_on_the_inset_boundary`, the above, proptest | PASS |
| 15 | geo `c13_c14_c15_…`, `prop_pieces_stay_inside_the_inset_region` | PASS |
| 16 | geo `c16_lines_mode…`, `c16_shapes_mode_hatch_strips…`; session `c16_lines_draw…stroke_off` | PASS |
| 17 | geo `c17_*`, bits golden native = wasm; doc `c17_*`; session `c17_*` | PASS |
| 18 | geo `c18_*`; session `c18_*` (note, refusal, drag, shape edit); browser | PASS (see finding 2) |
| 19 | session `c19_*` (canvas equals a fresh read after node drag, move, undo); browser resize | PASS |
| 20 | doc `c20_a_copy_has_the_same_pattern_and_seed…`; browser copy/paste seed | PASS |
| 21 | doc `c21_a_linked_clone_shows…follows_edits` | PASS |
| 22 | doc `c22_every_pattern_edit_is_one_step…`; session `c3_c22_…` | PASS |
| 23 | geo `c23_an_open_outline…`; session `c23_an_open_path…` | PASS |
| 24 | session `c24_c25_convert…`; browser "criterion 24" Object menu / context menu | **FAIL** (finding 1) |
| 25 | session `c24_c25_convert…` (anchors and exported `d` strings identical) | PASS |
| 26 | session `c26_svg_export_has_paths…`; browser export | PASS for SVG; PNG not testable (no PNG export yet, debt) |
| 27 | not testable: no laser or plotter jobs exist (ADR decision 7, `docs/technical-debt.md`) | n/a |
| 28 | all of the above in one branch; demo journey in the browser | PASS except 24 |

Format: version 32, golden `pattern_v32.curvyo` opens and round-trips, files of 29 to 31 open,
33 is refused, damaged `fill_paint`/`fill_pattern` registers (unknown generator or mode, wrong
types, NaN/inf, below minimum, negative margin, seed fractional/negative/above u32, Wall not
below its cell) are refused as damaged; an absent parameter reads as its default.

## Findings

1. **Criterion 24 not met as written.** Spec: Convert to paths in the Object menu and the context
   menu. Built: a "Convert to paths" button in the Pattern block only. The Select bar shows "Stroke
   to path | Linked clone" without it, and a right click on a patterned shape opens no menu. UX notes
   §14.1 records this as a lead decision pending the product owner. Repro: browser script, check
   "criterion 24". Needs the spec changed or the entries added.
2. **The hard limit counts pieces, not work: Interpolation waves freeze far below 500,000.**
   Release build, 100 mm circle: 1,000 lines 2.1 s, 2,000 lines 6.4 s, 4,000 lines 21 s; 100,000
   lines on a 100 mm square ran over 9 min (killed). The field allows typing up to 500 per UX
   notes, the stored maximum is 1,000,000. Generation is synchronous, so the tab hangs. Repro:
   `pattern_pieces_of` on `circle(50, 50, 50)` with `InterpolationWaves { lines: 4000 }`. Waves
   with wavelength 0.05 on A4 give 2.5 M anchors in 2.3 s. The plan's CPU table measured Dots only.
3. **Crossing line sets are emitted as staircase polylines** (Grid, Crosshatch): one piece runs
   along a horizontal line, turns at a crossing and continues on a vertical one (Grid on 100 x 80:
   47-node pieces from (0, 0) to (100, 80)). The drawn geometry is the same, but Convert to paths
   gives zig-zag paths instead of lines and Lines mode plots them in that order. Not stated by the
   spec; flag for the customer.
4. Low: in Shapes mode the strips of Grid, Crosshatch and Sea waves overlap each other (49 overlapping
   pairs for Grid on 30 x 30) instead of being one outline; a laser cuts every crossing twice.
5. Low: leaves near the region edge that are not cut are still flattened to polylines (ADR §3 says
   pieces wholly inside keep exact curves); deviation 0.0075 mm, inside the kernel tolerance.
6. Note: `PATTERN_NOTE_PIECES`/`PATTERN_MAX_PIECES` stay 50,000/500,000 (plan: confirmed by
   measurement); the spec still calls them first estimates and must be updated before Done.

## Gate (CLAUDE.md §7, logs in `target/0071-gate/`)

fmt, clippy (workspace, all targets), nextest per crate (all 12, 0 failures; editor-wasm 4,383
tests), wasm32 build and clippy of every core crate and the editor, `cargo deny`, rustdoc with
`-D warnings`, frontend `build:wasm`, `tsc`, `npm test`, `oxlint` (warnings only, none new from
0071): all green. Extra: the pattern goldens and the bits golden on `wasm32-unknown-unknown`.
