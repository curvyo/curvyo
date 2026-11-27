# Plan for slice 1 (nothing armed by itself)

Branch `story/codes-paint` (one PR with `0144` slice 4). Criteria 1 to 7 of `specification.md`; criterion 5 is the
lead's decision of 2026-11-27 **(Proposal)**: an armed generator **stays armed** after a placement.

## What the code did (verified before the change)
`GeneratorsState.kind` kept the last choice and `generator_kind()` fell back to Box, so entering the tool armed
the last kind; a placement ended with `set_tool(Tool::Select)`; Escape left the tool at once; the bar was a Radix
radio group that cannot be emptied.

## Affected
- `curvyo-editor-wasm`: `session/generators.rs` (`generator_kind` empty for none, `toggle_generator_kind`,
  `disarm_generator`, no return to Select), `session/mod.rs` (`set_tool` clears the kind on entry),
  `session/keys.rs` (Escape: release first), `generators/source.rs` (Replace outline arms after the switch),
  `wasm_generators.rs`.
- `frontend`: `GeneratorsBar.tsx` (toggle buttons with `aria-pressed`), `useGenerators.ts`, `Canvas.tsx` (crosshair
  only while armed).

## Tasks
- [x] 1. Entering the tool arms nothing, a click and Enter place nothing (1)
- [x] 2. A press arms, the same press releases, another press moves the arming (2, 3, 6)
- [x] 3. Escape releases first, the second leaves to Select (4)
- [x] 4. An armed generator stays armed after a placement (5)
- [x] 5. Tests: `curvyo-editor-wasm/tests/generators_arming.rs`; existing tests that placed without arming or expected
  Select after a placement are adapted
- [x] 6. Full gate once into `gate-paint.txt` (with `0144` slice 4)

## Not built
Hint line and the per-segment tooltips of criteria 2 and 6 (only the pressed state, `aria-pressed`, the glyph names
and the Loading tooltip), the Shift rule (dropped by the decision), the "Select one closed shape first." hint.
