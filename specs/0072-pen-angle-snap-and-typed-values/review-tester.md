# Tester and architect review: 0072 Pen angle snap and typed values

Branch `story/pen-angle-snap-and-typed-values`, 2026-10-19. Combined tester pass and architect
check, restarted after the VM disk hang. The stalled run's
`curvyo-editor-wasm/tests/acceptance_0072_tester.rs` was reviewed and trimmed: its right-press
test duplicated the implementer's `a_right_press_finishes_the_path_like_enter` and was removed;
the frozen-press test was cut to the part the implementer's tests do not cover.

Verdict: **FAIL** (one MUST-FIX in the core, one rule violation). Everything else passes.

## Tester part

### Coverage per criterion

Implementer tests: `curvyo-ui-core/src/pen_constraint.rs`, `pen_entry.rs`, `pen_entry_text.rs`,
`curvyo-editor-wasm/src/session/pen.rs`, `session/pen_entry.rs` (unit tests). Tester tests:
`curvyo-editor-wasm/tests/acceptance_0072_tester.rs` (T) and
`frontend/tests/acceptance_0072_tester.test.ts` (TS).

| # | Test | Result |
|---|---|---|
| 1, 2, 3, 5, 7 | implementer (`pen.rs`, `pen_constraint.rs`) | pass |
| 4 | implementer `ctrl_constrains_the_rubber_band_from_the_last_node` (Ctrl release at rest changes the band); the frame timing and WebKitGTK key-up are GUI: UX review | pass (session part) |
| 7, 31 | T `a_continuation_from_the_first_node_types_and_snaps_from_e` | pass |
| 8 | implementer `ctrl_snaps_the_handle_direction_and_never_moves_the_node` | pass |
| 9 | no test: the session takes no pointer kind, the host passes `ctrlKey \|\| metaKey` for every `pointerType` (`useEditorSession.ts` `pointer_down`). Satisfied by construction | pass (review) |
| 10 | implementer `pen_snap_cost` (release, ignored) and T `the_snap_adds_at_most_a_tenth_of_a_millisecond_per_move` (release, ignored) | see "Benchmark" |
| 11 | T `ctrl_and_tab_keep_their_meaning_outside_the_pen` | pass |
| 12, 14, 19, 19a, 20, 22, 23, 26, 32, 34 | implementer (`session/pen_entry.rs`) | pass |
| 13, 15 to 17, 21, 24, 25, 27, 28, 29 | implementer (`pen_entry.rs`, `pen_entry_text.rs`) | pass |
| 18 | no seam: the Tab cycle is inline in `PenEntryChip.tsx` `onKeyDown`; read, correct (wraps, Shift+Tab back, selects, no edit so no freeze). UX review checks it live | review only |
| 20, 26 | T `a_frozen_press_with_a_zero_length_adds_nothing` | pass |
| 24, 28 | T `degenerate_texts_stay_finite_and_inside_the_limit` ("1e400", "inf", "NaN", "1e308", "1e300" degrees) | pass |
| 26, 28 | T `a_segment_shortened_to_zero_at_the_limit_places_no_node` | **FAIL** |
| 33 | GUI only: UX review | not run |
| 35 | T `every_number_on_screen_has_its_unit` (mm, cm, in; readout with and without Ctrl; emptied fields keep suffixes) | pass |
| 36 | TS `c36 the Pen rail button has no hint line...`, `c36 the shortcut table lists...` | pass |

### Findings

1. **A zero-length segment at the coordinate limit (MUST-FIX).**
   `curvyo-ui-core/src/pen_entry.rs:210-219`: the zero-length refusal runs before the limit
   clamp. When L lies on the limit (or beyond it: the Pen pointer is clamped at 1e9 mm, not
   1e7) and the typed direction points outward, `reach_inside_limit` returns 0, the length is
   cut to 0 and a second node is placed on L, with the notice "Too far away...". Criterion 26
   says this is exactly the case that must not place ("a second node on the same spot would be
   a zero-length segment"). Repro: Pen, click (10000000, 50), Tab, Length 30, Angle 0, Enter:
   two nodes at (10000000, 50). Fix: check `length_mm <= LENGTH_TOLERANCE` again after the
   clamp (return `LengthNotPositive`). Test: `a_segment_shortened_to_zero_at_the_limit_places_no_node`.
2. Untouched chip over a join target: `pen_entry_live_end` uses the target's node centre and
   Enter places a plain node there, while a click there joins. Criterion 19 says "the point a
   click would place"; the point matches, the action does not. Minor, not tested (should-fix or
   accept: Enter in the chip is defined as "adds one node").

### Benchmark (criterion 10)

- Implementer `pen_snap_cost` (`cargo test --release -p curvyo-editor-wasm --lib -- --ignored
  pen_snap_cost`, `timeout 600`): **did not finish** within 600 s (exit 124). Its setup draws
  the 5000 paths by Pen clicks and `finish_pen`, one history commit each, and that setup
  dominates; the measured loop never ran. Should-fix: build the paths in the document, as the
  tester benchmark does. Its assertion (< 2 ms per move) also does not measure the criterion's
  "adds no more than 0.1 ms".
- Tester `the_snap_adds_at_most_a_tenth_of_a_millisecond_per_move` (release, best of five
  alternating runs, hover plus one `draw_list` per move, 5000 open paths): plain 2.416 ms,
  Ctrl 2.390 ms per move. The snap adds nothing measurable: **pass**. (The 2.4 ms includes the
  full frame of 5000 paths; criterion 10's 2 ms of `0034` is the hover alone.)

### `acceptance_0016_pr2_tester` rotate cost

`rotate_commit_cost_grows_roughly_linearly_with_the_number_of_outlines`, run three times with
load average about 1: passed 3 of 3 (0.18 to 0.19 s). Unchanged. It also passed inside the full
`nextest` run.

## Architect part

| Area | Verdict |
|---|---|
| Core purity (`curvyo-ui-core`: no I/O, clock, threads; deps only the two core crates; wasm32 build) | pass |
| Module responsibilities: `pen_constraint`, `pen_entry`, `pen_entry_text`, `session/pen_entry`, `wasm_pen_entry` each one sentence, no "and" in the responsibility | pass |
| No trait, no generic, no new crate, no dependency, no format change | pass |
| `unwrap`/`expect` outside tests | pass (none) |
| Functions over ~60 lines | pass: `PenEntry::resolve` is 60 with its doc; `key_table::decide` grew 71 to 72 (existing, one match arm) |
| `session/mod.rs` (1391 lines, +13) | accept: the 13 lines are wiring (field, two initialisers, `set_tool` cancel, press routing, handle point at release) that has no other home; the split is the existing debt entry "`Session` is one module past the size limit" in `docs/technical-debt.md`. Should-fix there: add the 0072 line count to that entry |
| Units as types | **FAIL**: `PenEntryPlaced::degrees: f64` and `PenEntryNotice::AngleNotANumber(f64)` are public bare-`f64` angles in a core crate where `Angle` exists and `shape_placement` uses it (`curvyo-ui-core/src/pen_entry.rs:64`, `:83`; private `Polar::degrees` at `:44`) |
| Tolerances | pass: zero length uses `LENGTH_TOLERANCE` (`GEOMETRIC_TOLERANCE_MM`); readout uses the drag threshold. Nit: `constrain_segment` guards with `!length.is_normal()` instead of a `Tolerance` (`pen_constraint.rs:16`), harmless |
| Deviation 1: drag readout via wasm `live_readout` (`wasm_api.rs:294-301`) instead of a draw-list text | accept: criterion 8 names the create-drag readout, which is that DOM route; two lines, no new per-frame message |
| Deviation 2: separate `pen_entry_open` in `KeyState` | accept: `entry_open` also covers Select and Offset chips; Enter needs the Pen-specific arm so a stray Enter cannot finish the path. Explicit and tested (`the_open_chip_gates_letters_and_enter`) |

### MUST-FIX

1. `curvyo-ui-core/src/pen_entry.rs:210-219`: refuse a length that the limit clamp cuts to the
   tolerance or below (finding 1).
2. `curvyo-ui-core/src/pen_entry.rs:64, 83` (and `:44`): angles as `Angle`, not bare `f64`
   (`CLAUDE.md` §5 "Units are types"); `format_degrees`/`decimal_text` take the degrees at the
   text edge.

### Should-fix

- Finding 2 (Enter over a join target), or record it as accepted in `adrs.md`.
- `docs/technical-debt.md`: note `session/mod.rs` at 1391 lines after 0072.
- `pen_snap_cost` (`session/pen.rs:417`): build the 5000 paths in the document and measure the snap's added cost; today it does not finish in 600 s.

## Gate

| Step | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo nextest run --workspace` | 7665 run, 7664 pass, **1 fail**: `acceptance_0072_tester::a_segment_shortened_to_zero_at_the_limit_places_no_node` (MUST-FIX 1) |
| `cargo build --target wasm32-unknown-unknown` (document, geometry, render, ui core) | pass |
| `cargo deny check` | pass |
| `cargo doc --workspace --no-deps` with `-D warnings` | pass |
| `npm test` (frontend) | 398 pass |
