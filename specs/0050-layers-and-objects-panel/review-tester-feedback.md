# Tester review: customer feedback 2026-10-14 on 0050

Verdict: FAIL (one finding; everything else passes).

Gate on `test/layers-panel` (from `origin/story/layers-and-objects-panel`, 67c6f86): fmt and clippy clean on
document-core, ui-core, editor-wasm; nextest 5643 run, 5642 pass, 1 fail (the finding below);
frontend `npm test` 325 pass, `tsc -b` clean (after `build:wasm`, the old bindings were stale).

Tests: `curvyo-editor-wasm/tests/acceptance_0050_feedback.rs` (24), `frontend/tests/acceptance_0050_feedback.test.ts` (7).

| Criterion | Test | Result |
|---|---|---|
| 35 | each drawing tool (Rectangle, Ellipse, Polygon/star, Pen) makes "Initial Layer", one commit, one undo removes both | pass |
| 35 | second object, deselect then draw, undo/redraw/redo: same layer, never a second one | pass |
| 35 | loose objects stay loose, only the new one enters the layer | pass |
| 35 | document with a layer: none made; entered group: new object goes into the group | pass |
| 35 + 0041 c31 | wiping the first object's history drops its drawing step | FAIL |
| 20 | locked press: no notice on any channel; cursor "locked" (Select, Offset), also for a locked ancestor | pass |
| 20 | hidden+locked object: no lock cursor; locked in front of unlocked: press selects the one behind; other tools: no lock | pass |
| 26 | drag previews on the canvas (alpha), writes nothing, release is one step, Escape restores, second release is a no-op | pass |
| 26, 3 | no hint text, no "footer" key in the view, empty-document view | pass |
| 34, 36 | New layer in front of the active layer; Delete layer: one step, notice without "No undo yet.", undo restores | pass |
| 55 | toolbar flags follow the target; page: role=toolbar "Layers", five buttons in order, icon only, aria-disabled and still rendered | pass |
| 56, 57 | Move up/down one step, selection kept, no-op writes nothing; Duplicate (objects: copy in front of the original and selected; layer: copy active) one step, undo | pass |
| 53 | unfinished Pen path blocks New layer, Delete layer, Duplicate, Move up/down, Opacity commit | pass |
| 6, 38 | row order name, opacity readout, Select contents, eye, lock | pass |
| 9, 3 | empty state texts, toolbar present, Opacity slot after the list | pass |

## Finding 1 (spec conflict, needs a decision)

The first drawing step in an empty document now also writes the layer. `object_history_session`
`the_wipe_hides_...` passed only after the implementer changed its start document to one that already
has a layer (`fresh_with_layer`). With the original `fresh()` the test fails: `undo_len` is 3, not 2.
Wiping the history of object A leaves A's drawing step on the undo stack, because that step also
touched the Initial Layer, and 0041 criterion 31 keeps steps that touched other objects. In the most
common case (start the app, draw, wipe) Ctrl+Z then still undoes A's drawing. Repro: new document,
draw two rectangles, select the first, wipe object history, `undo_len` stays 2 (expected 1). Test
`ac35_with_0041_wiping_the_first_object_drops_its_drawing_step_from_the_stack`. Either the Object
history must not count a layer as another object (my reading: a layer is a container), or 0041
criterion 31 gets a sentence. The edit hid the question instead of answering it.

## Judgement of the implementer's edits to earlier tests

- `acceptance_0040_tester` ac41: justified. Criterion 35 says the three objects now sit in one Initial
  Layer; the test still compares the background variants with each other and now asserts 1 layer with 3 children.
- `acceptance_polygon_star_box_refit` ac14: acceptable. The reference document must draw like the session
  or the stored-key comparison sees the layer keys as new. The assertion is the old one; its `||` of three
  alternatives was already lenient before this change.
- wipe test in `object_history_session`: not accepted as a plain fix, see finding 1. The edit is only valid together with a
  decision on 0041 criterion 31.

## Not covered

Tooltip text is only checked through the accessible name: the tooltip content is not in the server render.
The live canvas drag with the real pointer and the lock cursor image were not looked at in a browser.
