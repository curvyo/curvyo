# Tester review: 0052 click to create shapes

Verdict: PASS. Branch `test/click-create` on top of `story/click-to-create-shapes` (c3b2a95).

## Tests

`curvyo-editor-wasm/tests/acceptance_0052_tester.rs`: 57 tests through `Session`'s public API, written from
`specification.md` before the implementation was read (only the public signatures of `Session` were looked at).

| Criteria | Covered by |
|---|---|
| 1, 31 | click versus drag (0.5 mm and 1 mm at 100 %), touch 8 px, two-tap creation |
| 2, 12 | preview appears only beyond 3 px, field texts of the live shape, circle versus ellipse placeholder |
| 3 | Ctrl, Shift with the pointer still and held at the first click |
| 4, 5, 22 | second press equals a create-drag, press on an object, press near A ignored, frozen press anywhere |
| 6, 35, 36 | one history step, Select tool, selection, undo, round trip, cancel writes nothing |
| 7, 8, 9 | Escape twice, tool switch, lost pointer, blur, 18 gated chords, leaving the viewport |
| 14 to 21 | every worked example of the spec (16 a-c, 17, 18, 19, 20, 21) |
| 23, 24, 25 | invalid text keeps the placement, unit suffixes, cm document, degree sign, bar changes |
| 26, 27 | Enter at the pointer, at the viewport centre, untouched size refused |
| 28, 30 | pivot marker at A, all live-region sentences |
| 34 | Pen, Select, Node start no placement; Escape in a drag |

Not testable in Rust, left to the frontend suite and the ux-engineer: 10, 11 (chip place), 13 (Tab cycle), 29, 33
(layout at 800 x 600). Not covered by a test of mine: the locked-layer refusal (`0050`) and the entered group
(`0023`); the implementer's own session tests cover them.

## Gate

fmt, clippy (host and wasm32 for all five core crates and editor-wasm), nextest (6632 passed), `cargo deny`,
`cargo doc -D warnings`, the release budget jobs of `ci.yml` (placement frame 0.59 of the create-drag frame,
budget 1.1; one keystroke 520 ns, budget 1 ms): all green. Frontend: `build:wasm`, `tsc -b`, `oxlint` (the 5
warnings are in code older than this branch), `npm run build`, `npm test` three times: 356 of 356 each time.
Not run: the `boolean-wasm-golden` job (needs `wasm-bindgen-cli`; the slice does not touch the boolean kernel).

## Rewrites of earlier tests (adrs.md decision 8)

All four are acceptable: each still asserts every old property (nothing written, tool kept, selection kept, no
hover decoration) and adds "a placement runs, Escape cancels it". None is weakened.

- `session/shapes.rs::a_press_without_movement_creates_nothing_and_keeps_the_selection`: fine.
- `acceptance_unified_editing_pr2.rs` ac26 (both): fine. The moved `pointer_hover(far)` now runs after the
  cancel, so the "no decoration" checks still see an idle tool.
- ac25 double-click: fine, but the new comment is wrong. `dbl` withholds the second press, so criterion 5 (press
  near A ignored) is not what runs; what is checked is that a double-click makes no hint chip and no shape. My
  `ac5_...` test covers the real second press. Comment only, no action needed.
- `scene` helper in `acceptance_polygon_star_box_refit.rs`: typing radius and angle gives the same frame. Nit: the
  `if s.placement().is_some()` guard lets the helper pass whichever way the 0.05 mm drag goes; an `else` branch
  asserting a drag happened would pin the 3 px threshold. Not blocking.

## Deviations listed in plan.md

- Escape in `Session::escape`, not in `cancel_drag_in_flight`: correct. `cancel_drag_in_flight` also runs from
  `pointer_cancelled` (blur, lost pointer), and criterion 7 forbids cancelling there. Tested both ways.
- `result` takes the polygon/star bar settings: correct and needed for criterion 25 (bar changes reach the result
  while the typed values stay).
- `PlacementError`: fine, `FieldError` already exists in ui-core.
- A right press in a creation tool does nothing (`useEditorSession.ts`): fine. The cancel on a right press during a
  placement is the capture listener in `usePlacement.ts`, which runs first. Without it a right press used to reach
  `pointer_down`; not forwarding it is the safer behaviour.

## Findings (none blocking)

1. Frontend double-click suppression (`DOUBLE_CLICK_PX` = 5, 400 ms) swallows a second click 3 to 5 px from A made
   within 400 ms: it neither places nor is ignored by the session. The result is a sliver anyway. Low.
2. An unfrozen Enter/second click with a zero-height live box places a 30 x 0 rectangle, as a create-drag does
   (`0011` criterion 12). Criterion 23 refuses a zero size only for a kept or typed value, so this follows the spec.
