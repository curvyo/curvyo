# Tester review of 0050 layers and objects panel (with 0051 amendments)

Branch `test/layers-panel` on `story/layers-and-objects-panel` (c105ca2). Tester: independent tests written from
`specification.md` before the implementation diff was read (only public seam names were looked up).

**Verdict: PASS** for everything buildable and testable without a browser. No acceptance criterion failed.
Expected gaps (0049 not merged): the clone parts of ACs 7, 24, 25, 26, 28, 36, 38, 45 and the status-bar measures of AC 19.
Not verifiable here: the visual and gesture parts that need a browser (see "Not verified").

## Gate (CLAUDE.md section 7), run locally in the worktree
fmt clean; clippy `-D warnings` clean (needed `npm run build` first for `frontend/dist`, which `curvyo-app` embeds);
nextest 6058 passed, 0 failed (incl. 105 new); wasm32 builds of the four core crates ok; `cargo deny` ok; `cargo doc -D warnings` ok.
Release budgets (`layers_budget`, ignored tests): first window 15 ms, jump to end 0.3 ms, Ctrl+A 34 ms, layer hide/show/lock/opacity
57-64 ms, one move 13 ms, 1,000 rows 71 ms, 9,000 rows 102 ms, expand/collapse 2-3 ms: all inside AC 48-52.
`npm test`: 308/308 three times in a row. See finding 3 for the "flaky" run.

## Results per criterion
Tests: `S` = `curvyo-editor-wasm/tests/acceptance_0050_tester.rs`, `D` = `curvyo-document-core/tests/acceptance_0050_tester.rs`,
`U` = `curvyo-ui-core/tests/acceptance_0050_tester.rs`, `F` = `frontend/tests/acceptance_0050_tester.test.ts`.

| AC | Result | Tests |
|---|---|---|
| 1 | PASS | S ac1_the_strip..., U the_strip_has_layers_fourth..., (0051 c2/c45 tests) |
| 2 | PASS | S ac2_the_panel_never_switches..., U layers_is_sticky... |
| 3 | not verified | layout, 520 px canvas, 12 rows: needs a browser; F checks 28 px rows and a window of at least 12 |
| 4 | PASS | S ac4_the_subject_line..., U the_words_of_the_spec |
| 5 | PASS | S ac5_rows_are_depth_first..., ac5_collapsed_containers... |
| 6 | PASS (logic) | F indent/row height/eye+lock buttons/state as shape; visuals not verified |
| 7 | PASS (non-clone) | S ac7_default_names_and_layer_numbering; clone name = gap |
| 8 | PASS | S ac5_collapsed..., ac8_a_collapsed_container_with_further_selected... |
| 9 | PASS (model) | S ac9_...; the two texts exist in `LayersTab.tsx` |
| 10 | PASS | S ac10_* (2) |
| 11 | PASS | S ac11_* (2) |
| 12 | PASS, see finding 2 | S ac12_* (2) |
| 13 | PASS | S ac13_* (2) |
| 14 | PASS | S ac14_* (5 incl. layer lock/hide, entered group) |
| 15 | PASS | S ac15_hovering_a_leaf_row... |
| 16 | PASS | S ac16_* (3), D rename_cleans_the_name... |
| 17 | PASS | S ac17_*, D ac17_names_read_from_a_file..., F escaped markup |
| 18 | PASS | S ac18_... (one step; multi-row one step; undo) |
| 19 | PASS (status bar = gap) | S ac19_* (draw, click, Ctrl+A, marquee, Alt-click, Fit, eyedropper, Pen continue, Node tool) |
| 20 | PASS | S ac20_* (2), ac14 lock cases |
| 21 | PASS | S ac21_a_selected_group_carries_its_locked_members |
| 22 | PASS | S ac22_... |
| 23 | PASS | S ac23_* (rectangle, ellipse, Pen, polygon/star; hidden and locked texts) |
| 24 | PASS (non-clone) | S ac24_the_frame_draws_exactly..., D effective_values_come_from_the_ancestors, D randomised_trees... |
| 25 | gap (0049) | |
| 26 | PASS (non-clone) | S ac26_* (2): one commit per release, Mixed = null value (front end maps it), clamp, caption, locked target |
| 27 | PASS | S ac27_alpha_is_the_product...: fill 51, stroke 64 of 255 |
| 28 | gap (0049) | |
| 29 | PASS | S ac29_* (3), U restacking_plans_agree_with_a_reference_model |
| 30 | PASS | S ac30_* (2: refusals and texts, depth 32/33), D move_rules_of_criterion_30 |
| 31 | PASS | S ac31_..., D a_move_is_one_commit_keeps_ids... |
| 32 | PASS | S ac32_* (3: one step, announcement text, out/in, locked row) |
| 33 | PASS | S ac33_* (4: all four keys, several nodes, layers, ignored without selection/with modifiers/in a field/other tool) |
| 34 | PASS | S ac34_* (2) |
| 35 | PASS | S ac35_* (2) |
| 36 | PASS (non-clone) | S ac36_* (3: notice text, undo restores ids/name/flags/order, locked refusal, next active layer), D delete_layer_refuses... |
| 37 | PASS | S ac37_..., D a_layer_stays_when_its_last_child_goes... |
| 38 | PASS (row); list not verified | F role/aria-level/posinset/setsize/selected/expanded/current; `role="treegrid"` named "Layers and objects" is in source only; roving focus needs a browser |
| 39 | not verified | keyboard map in `LayersTab.tsx`; needs a browser |
| 40 | not verified | focus behaviour needs a browser |
| 41 | PASS (names) | S ac41_..., F; contrast and focus ring not verified |
| 42 | PASS | S ac42_* (2) |
| 43 | PASS (partial) | S ac43_* (Group, Union keep the layer); other commands not individually run |
| 44 | PASS | S ac44_... (labels: new_layer, rename_node, set_node_visible, set_node_locked, set_node_opacity, move_nodes, delete_layer) |
| 45 | PASS (non-clone) | S ac36 undo, ac45_undo_after_hide_lock_opacity_and_move... |
| 46 | PASS | D ac46_* (4: round trip byte-stable, version 12, newer refused with or without layers, older fixtures open without layers or flags, golden `layers_v12.curvyo`) |
| 47 | PASS | D ac47_* (types/range incl. NaN and inf, marker below top level = group, marker on a leaf refused, 10,000 layers, depth 33 refused) |
| 48-52 | PASS | `layers_budget` (release), numbers above; S hostile windows (usize::MAX) |
| 53 | PASS | S ac53_* (19 actions each answer the hint; no commit, JSON, or selection change; works after Escape) |
| 54 | PASS | D ac54_save_and_document_json_carry_hidden_and_locked_nodes... |

Also: S hostile_row_ids_and_arguments_are_refused_without_a_panic (bad ids, NaN/inf/huge opacity, absurd windows).

## Judgement of the implementer's edits to earlier tester tests
1. **Format-version pins** (`0023` doc-core ac27, `groups_format`): from `== 11` to "writer declares CURRENT_FORMAT_VERSION, newer = CURRENT+1 refused, CURRENT >= 11".
   Correct. 0050 AC 46 makes the version a moving number ("next free"); the intent of the old pins (a writer that declares its version and a reader that refuses newer) is kept.
   The old "11" fact is still guarded by the v11 fixtures opening. The `layers*` skip in "earlier builds open unchanged" is correct (those files are not earlier builds).
   `0047` part A: `== 11` to `== 12` is correct but brittle: it breaks at the next bump. Prefer `== CURRENT_FORMAT_VERSION` or `>= 11`. Not a defect today.
   `0017`: adding `layers_opacity_damaged_v12.curvyo` to the list of fixtures that are refused by design is correct (D ac47 proves it is refused for the right reason).
2. **`0044` c11 and edit-polish ac55** (Home/End/PageUp/PageDown dropped from the "ignored" lists): correct, AC 33 binds them in the Select tool with a selection, and the old tests ran with a selection.
   Coverage is not lost: S ac33_the_keys_are_ignored_without_a_selection_with_modifiers_in_a_field_or_in_other_tools keeps the negative cases (no selection, Ctrl/Alt/Shift, focused field, other tools).
3. **`0043` / `0051`** (five tabs, Layers fourth: File, Document, Style, Layers, History; `from_name("layers")` valid): correct against 0051 criterion 2 and 45 (tabs found by name).
   Hostile-name lists switched `"layers"` to `"Layers"`, which stays invalid: not weakened. `classify_press` extra argument in four ui-core tests is a mechanical signature change.

## Findings
1. LOW (spec silent, behaviour chosen): multi-selection restack by one place. `reorder_plans` moves a non-contiguous selection as a block, and does nothing if any selected node is already at the edge.
   Repro: U `restacking_plans_agree_with_a_reference_model` generator, e.g. 7 siblings, picked 0,1,5,6, `Reorder::Backward`: result unchanged although 5 and 6 could move.
   Front/Back and single or contiguous selections follow the reference model. Customer may want per-node stepping (decide at demo).
2. LOW (AC 12 wording): "the first selected row" is the first in selection order, not list order, and is evaluated when the list is read.
   Repro (S, removed from the tests): layers L1 (a) and L2 (b, group g); click a; collapse L1 by hand; Ctrl+A. The first selected unit is still a, so nothing re-opens; both layers show the "contains selection" mark but the front-most selected row (in L2) is hidden in a collapsed layer. Ctrl+A from an empty selection opens the right layer.
3. INFO (test environment, not product): the "flaky 4 failures" of `npm test` is an out-of-memory crash of node in the five test files that start a Vite SSR server
   (`acceptance_0051_tester`, `fileTab`, `panelStrip`, `tooltipProvider`, and my `acceptance_0050_tester`) when the shell has `ulimit -v 8000000`.
   Reproduced: 4-5 failures in 3 of 3 runs with the limit (with or without cargo running), 308/308 in 3 of 3 runs without it. Run `npm test` without `ulimit -v`.
4. INFO: Delete on a focused object row deletes the current selection (AC 37 wording), not the focused row. With arrow keys the focus moves without selecting, so Delete can remove an unrelated selection. Ctrl+Z restores it. Consider selecting on focus or deleting the row.

## Not verified (needs the browser pass by the ux-engineer)
AC 3 widths and 12 rows at 800 x 600, AC 6 visuals (indent, glyphs, inherited dashed look), AC 29 pointer drag gesture (4 px threshold, insertion line, edge scroll, Escape), AC 32 Alt+Left/Right delivery per webview,
AC 38-40 roving focus and keyboard map, AC 41 contrast and focus ring, and AC 8's 6 px mark.
