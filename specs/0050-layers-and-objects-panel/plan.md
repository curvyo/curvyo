# Plan for Layers and objects panel

Branch `story/layers-and-objects-panel`, one PR for the whole slice at the end (`CLAUDE.md` §4).
Inputs: `specification.md` (criteria 1 to 54), `adrs.md` (decisions 1 to 15), ADR 0015, and the
amendments of `0051` (criteria 2 and 13 there replace criteria 1 and 4 here: the Layers tab is
the fourth button of the vertical strip, 28 x 28 px, 16 px glyph; the subject line lives in the
24 px header row; Tab order "collapse control, tablist, body").

State of `main` at 6841fb6 that this plan builds on: `0020` (undo), `0023` (groups), `0041`,
`0043`, `0044` (object read cache), `0051` (vertical strip) are merged. **`0049` (linked clone) is
not merged**, so there is no clone node yet; see "Clone criteria" below. `CURRENT_FORMAT_VERSION`
is 11; this slice makes it **12**.

## Affected crates/modules

- `curvyo-document-core`
  - new `node_registers.rs`: keys `name`, `visible`, `locked`, `opacity`, `layer`; read with
    defaults, name cleaning (trim, 128 chars, control and bidi characters dropped), validation
    of an opened file (wrong type, opacity not finite or outside 0..=1, `layer` on a non-group).
  - new `node_read.rs`: the one pre-order pass of node reads (id, parent, depth, kind
    layer/group/leaf with its `ObjectSnapshot`, own registers, effective visible, locked and
    opacity, the nearest hiding and locking ancestor), O(n) with values carried down.
  - new `node_edit.rs`: `rename_node`, `set_node_visible`, `set_node_locked`,
    `set_node_opacity` (many ids, one commit each).
  - new `layers.rs`: `new_layer`, `delete_layer` (one tree delete of the layer node), the
    "Layer N" name.
  - new `node_move.rs`: `move_nodes(plan)` (one commit of tree moves, rules checked again
    before the first write).
  - `group_tree.rs`: `children(None)` / `child_groups(None)` see through layers (the one place
    that makes layers transparent, decision 4); a `layer` marker below the top level reads as a
    plain group; layer-aware depth check.
  - `groups.rs`: `remove_emptied_groups` skips layers (decision 2).
  - `document.rs`: format version 12 and its doc comment; `export_json` writes layers and the
    registers (`adrs.md`, the note of 2026-10-12 on decision 11).
  - `tests/fixtures/`: `layers_v12.curvyo`, `layers_opacity_damaged_v12.curvyo`,
    `layers_marker_in_group_v12.curvyo`.
- `curvyo-render-core`: `artwork.rs` multiplies each paint's alpha by the leaf's effective
  opacity (decision 10); the `0023` dimming multiplies on top.
- `curvyo-ui-core`
  - new `layer_rows.rs`: `layer_rows(read, collapsed, selection)` flattening and
    `rows_window(first, count)`; default names, the opacity readout, "contains the selection",
    posinset/setsize, inherited-state cause.
  - new `layer_move.rs`: `plan_move(read, moved, target) -> Result<MovePlan, MoveRefusal>`
    for drag, Alt+arrows and the canvas z-order keys, with the refusal texts.
  - new `layer_text.rs`: accessible names, tooltips, notices ("Deleted layer Cut and 12
    objects.", "Layer Cut is locked. ...", "Moved Rectangle 3f9a1c to position 2 of 5"),
    Opacity caption.
  - `panel_tabs.rs`: `PanelTab::Layers` (sticky like History), `shortcut_layers`.
  - `units.rs`, `object_bounds.rs` (`content_bounds` skips hidden), `colour_pick.rs`,
    `pen_target.rs`, `hit_test_object.rs`: take effective flags from the read, write no walker.
  - `history_names.rs`: `rename_node`, `set_node_visible`, `set_node_locked`,
    `set_node_opacity`, `move_nodes`, `new_layer`, `delete_layer`.
- `curvyo-editor-wasm`
  - `session/object_cache.rs`: holds the node read; derives the drawn leaves (effectively
    visible, with opacity) and the pickable leaves (visible and unlocked).
  - `session/context.rs`: tools see pickable leaves of the context; render sees drawn leaves;
    the selection invariant (drop hidden or locked, then the `0023` context fallback) runs once
    per document version before the frame.
  - new `session/layers.rs`: row window, row press (plain, Ctrl, Shift, layer row, Select
    contents), active layer and panel target (view state), expand/collapse state, toggles,
    rename, opacity, move, New/Delete layer, the one Pen-unfinished gate for all panel
    writes and selects (decision 9).
  - `session/keys.rs`, `key_table.rs`: Shift+Ctrl+L, Home/End/Page Up/Page Down in the Select
    tool (criterion 33), list keys forwarded from the panel.
  - `session/shapes.rs`, `session/pen.rs`: refuse drawing into a hidden or locked active
    layer (criterion 23); new objects go to the front of the active layer via
    `set_draw_parent`.
  - new `wasm_layers.rs`: the wasm surface of `session/layers.rs`.
- `frontend`
  - new `components/panel/LayersTab.tsx`, `LayerRow.tsx`, `useLayersList.ts`
    (virtualised window, roving focus, treegrid), `useRowDrag.ts` (pointer-event drag),
    `lib/layersText.ts`; `PanelStrip.tsx` and `PropertiesPanel.tsx` get the fourth tab.
- `docs/design-system.md`: only if a number of the Layers rows changes during the build.

## Clone criteria (`0049` not merged): not built

These parts are **not built in this slice; they go to whichever of `0049` and `0050` merges
second** (lead decision, 2026-10-12):

- AC 7: the default name of a clone ("Clone of <name of its direct original>").
- AC 24: a clone handed over by the read as the path it shows.
- AC 25: the clone's flags independent of a hidden or locked origin.
- AC 26: clones left out of what the Opacity field writes and of "Mixed", and the caption for
  a clone-only target.
- AC 28: a clone's opacity read from its root original.
- AC 36: the repair of clones outside a deleted layer.
- AC 38: the Select original cell of a clone row.
- AC 45: clones re-linked after the undo of Delete layer.

The read pass has no clone kind yet. The opacity rule is written so that a clone only has to
supply its root's register.

## Tasks

### M1: document model and file format (`curvyo-document-core`)

- [x] 1. Failing tests, then registers and their codec: read with defaults, write with name
      cleaning, open-file validation, `layer` below top level reads as group (AC 16, 17, 46, 47)
- [x] 2. Node read pass with effective visible, locked, opacity and the causing ancestor; O(n)
      test over a deep and a wide tree (AC 22, 24, 27)
- [x] 3. Setters `rename_node`, `set_node_visible`, `set_node_locked`, `set_node_opacity`:
      many ids, one commit, one register per node; locked node refuses opacity (AC 16, 18, 20,
      26, 50)
- [x] 4. `new_layer` (front of the active layer or of the top level, "Layer N"), `delete_layer`
      (one tree delete, refuses a locked layer), `remove_emptied_groups` skips layers (AC 7, 34,
      36, 37)
- [x] 5. `move_nodes(plan)` with the rules checked again; ids, registers, geometry kept
      (AC 30, 31)
- [x] 6. Transparent top level in `children`/`child_groups`; layer-aware depth (AC 42, 47)
- [x] 7. Undo: Delete layer revives the subtree with ids; hide, lock, opacity, move restore
      field by field (AC 45)
- [x] 8. Format 12, `export_json` with layers and registers, golden files, older file opens
      unchanged and is not rewritten, newer file refused, 10,000 layers open, deeper than 32
      refused (AC 46, 47, 54)
- [x] 9. Operation names in `history_names.rs` so the `0020` guard passes (AC 44)

### M2: canvas consumers (`curvyo-render-core`, `curvyo-ui-core`, `curvyo-editor-wasm`)

- [x] 10. Per-paint alpha times effective opacity in `artwork.rs`, test with the numbers of
      criterion 27 (AC 27)
- [x] 11. Object cache holds the node read; drawn and pickable lists; render draws exactly the
      effectively visible leaves in stacking order (AC 24)
- [x] 12. Hit test, marquee, lasso, Alt-click cycle, Ctrl+A, eyedropper, Fit to content,
      status measures, Node tool, Pen continue and connect read the flags; one table test
      lists every consumer (AC 19, 20, 42)
- [x] 13. Selection invariant after every version change, with the `0023` context fallback;
      group selection carries locked members (AC 14, 21, 43)
- [x] 14. Active layer (view state): draw parent, set by selecting an object, front-most layer
      at Open; drawing into a hidden or locked active layer refused with the notice (AC 23, 35)
- [x] 15. Canvas z-order keys Home, End, Page Up, Page Down via `plan_move` (AC 33)

### M3: panel logic (`curvyo-ui-core`)

- [x] 16. `layer_rows` and `rows_window`: depth-first front-most first, collapse, default
      names, readout, "contains the selection", posinset/setsize, row count (AC 5, 6, 7, 8, 9,
      12, 22, 48, 52)
- [x] 17. `plan_move` and `MoveRefusal` texts; Alt+Up/Down/Home/End/Left/Right targets
      (AC 29, 30, 32)
- [x] 18. Texts: accessible names, tooltips, notices, Opacity caption and "Mixed"; markup and
      bidi shown as plain text (AC 17, 26, 32, 36, 41)
- [x] 19. `PanelTab::Layers`, sticky, Shift+Ctrl+L, Shift+Ctrl+F leaves it (AC 1, 2; `0051`
      AC 2, 13, 14)

### M4: session and wasm surface (`curvyo-editor-wasm`)

- [ ] 20. Row press: plain (enters the parent group when needed), Ctrl, Shift range among
      siblings, other-parent fallback, layer row (active layer, panel target, clear
      selection), Select contents, hidden/locked row refuses (AC 10, 11, 13, 14)
- [ ] 21. Selection to rows in the same frame: expand ancestors of the first selected row
      only, scroll target unless the change came from the list (AC 12, 49)
- [ ] 22. Eye, lock, rename, Opacity field, New layer, Delete layer, Delete on a row, moves:
      one step each, multi-row toggles in one step (AC 16, 18, 20, 26, 34, 36, 37, 44)
- [ ] 23. Hover or focus on a row draws the `0014` hover box (AC 15)
- [ ] 24. One Pen-unfinished gate for every panel write or select, table test over the list of
      criterion 53 (AC 53)

### M5: the tab (`frontend`)

- [ ] 25. Fourth strip button, header row word and subject line "40 objects" (AC 1, 4 as
      amended by `0051`)
- [ ] 26. Tab body: Opacity header (52 px reserved), list as the one scroll area, footer with
      fixed slots; 12 rows at 800 x 600 (AC 3, 9, 26)
- [ ] 27. Virtualised treegrid row component: glyphs, eye, lock, readout, row action, states,
      collapse mark, inherited look, `aria-*`, coarse-pointer sizes (AC 6, 8, 22, 38, 41, 48)
- [ ] 28. Keyboard: roving focus, arrows, Home/End/Page, Enter/Space, Ctrl+Space,
      Shift+Up/Down, F2, Delete, Alt+arrows with `preventDefault`, Escape to canvas (AC 32,
      38, 39, 40)
- [ ] 29. Rename field: F2/double-click, Enter/blur commits, Escape cancels (AC 16, 34)
- [ ] 30. Pointer drag: 4 px threshold, insertion line, container outline, edge scroll,
      600 ms expand, refusal cursor and tooltip, count badge, Escape cancels, long press on
      coarse pointer (AC 29, 30, 51)

### M6: performance, docs, gate, PR

- [ ] 31. Release-build benchmarks in `curvyo-editor-wasm/tests` (the pattern of
      `draw_list_cache_budget.rs`): first window within 100 ms, at most 60 rows, Ctrl+A
      within 100 ms, layer toggle within 150 ms, moves of 1 / 1,000 / 10,000 rows, expand and
      collapse of 10,000 children (AC 48 to 52)
- [ ] 32. Criterion 54 test naming `export_loro_snapshot` and `export_json` (AC 54)
- [ ] 33. Spec status line, design-system numbers if changed, `/no-slop` on new UI text; full
      gate (§7); PR with "What changes in the demo" and the Review guide

## Decisions taken in this plan (reversible in this PR)

- **`document.json` form.** Recorded in `adrs.md` as the note of 2026-10-12 that supersedes
  decision 11: `0023`'s `{"shape": "group", "id", "objects"}` stays, a layer is
  `"shape": "layer"`, and `name`, `visible`, `locked` and `opacity` are optional keys.
- **The move rules live in document-core.** `Document::check_move` holds the rules of
  criterion 30 once; `Document::move_nodes` runs it before the first write, and ui-core's
  `plan_move` only turns a gesture into a `MovePlan` and asks `check_move`, so the rules are not
  written twice.
- **`parent_group` treats a layer as the top level.** The Select tool's context, entering and
  units read `parent_group`, `ancestor_groups` and `child_groups(None)`; these now see through
  layers (criterion 42). `Document::parent_node` gives the real parent for the panel.
- **Where the effective flags live.** On the node read in document-core; the session cache
  derives two lists from it once per version, so no consumer filters on its own.
- **Status-bar measures (criterion 19).** The status bar shows the cursor, the zoom and the
  document size; no measure of objects exists there yet, so nothing is filtered.
- **Where the flags reach the tools (M2).** The session's document read gives three lists: every
  leaf (what commands act on, so a selected group carries its hidden and locked members,
  criterion 21), the visible leaves (eyedropper; Fit to content reads the same filter in
  `content_bounds`) and the drawn leaves (faded per paint, `curvyo-render-core`
  `node_opacity.rs`). `Units` carries the set of hidden or locked nodes: a press, the marquee, the
  lasso, the Alt-click cycle, Ctrl+A, the Node tool and the Pen hit only what is not in it, and the
  selection drops every object of the context that is in it.
- **Alt+Left/Right.** `preventDefault` on keydown; checked on WebKitGTK and the browser build
  here; WebView2 and WKWebView are left to the ux-engineer's check and the nightly runs
  (UX notes: fall back to Shift+Alt on a platform that cannot).

## Validation

- Unit tests first for every core function (registers, read pass, setters, moves, rows,
  `plan_move`, texts); golden files for format 12 (`CLAUDE.md` §5).
- Undo tests for each step label against the real history (criteria 44, 45).
- A consumer table test that fails when a new canvas consumer reads unfiltered objects
  (criterion 19).
- Frame test through the read and the draw list (criterion 24) and the alpha numbers of
  criterion 27.
- Release-build timing tests on the CI runner for criteria 48 to 52, with a 2x allowance noted
  for the browser build.
- Manual run of the app (`cargo tauri dev` in `curvyo-app`, README) at 800 x 600: 12 rows,
  drag, keyboard, screen-reader names via the accessibility tree.

## Milestones (commits on this branch, each green)

1. M1 document model, registers, format 12, layer commands, moves (tasks 1 to 9).
2. M2 rendering and canvas consumers read the flags (tasks 10 to 15).
3. M3 panel logic in ui-core (tasks 16 to 19).
4. M4 session and wasm surface (tasks 20 to 24).
5. M5 the Layers tab in the frontend (tasks 25 to 30).
6. M6 performance tests, docs, full gate, one PR (tasks 31 to 33).

The architect may review M1 on the branch diff (document model and format); the UX review and the
full tester pass run once on the finished slice.
