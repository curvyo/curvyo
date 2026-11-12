# 0111 Laser redesign drafts (ux-engineer, 2026-11-12)

Static screens from the customer's redesign after the PR #333 demo; real tokens, 280px panel body plus its 32px strip, PNG beside each HTML. No spec yet; they supersede `specs/0028-material-test-library/drafts/` (sliders, Judge steps). Dashed outline in a draft = new control.

- `01-select-bar-operations-and-groups`: the five kind buttons (symbol + word) in the Select bar, only for a selection on a laser page; a group selected with one click (one item, ticks act on the whole group); a group entered by double-click: read-only crumb, outside dimmed, one member takes an operation.
- `02-test-grid-tool`: Test grid as third tool of the Offset/Measure card; its bar: kind, test id + "Burn test ID" (on), material, one line per axis (parameter, Min and Max each with a single slider, Count), every-cell values, Fill (Engrave), cell, labels; grid on the canvas; Engrave variant with the material list open.
- `03-operations-tab-test-grid-rows`: a 5 × 5 test as 26 ordinary rows (1 labels Engrave + 25 cells) in the flat list, one cell unfolded; an ordinary operation unfolded in place.
- `04-materials-tab`: list, selected material with per-laser sets (typed or from a test), Tests by id; new material, a set typed by hand, armed trash.
- `05-test-detail`: one-tap judge table, note, photo (file, camera, crop), "Place on <page>" with the same id, not-lasered test on a page.
- `06-fixtures-tab`: fixtures in their own tab, rows with the operation-row anatomy, fields unfolding in place, empty state.

**Decided by the customer (not questions):** "Lasered" is set only by Curvyo's output (0029); no button, an internal test hook covers the id rule. Generated operations are ordinary and hand-editable. After lasering, a change of the grid fields or of its operations by hand gives the grid a new id. Test id: 5 characters [0-9A-Z], random, unique against every id in the material store, deleted tests included.

**Proposals (vetoable):**
- P1 Select bar: the Operations group sits right after the settings switches. Inside an entered group the bar holds only that group.
- P2 Group with differently assigned members: the group counts as one item; the row of an operation holding some members shows the mixed tick and "Group 3 in part" in line 2; a press on it assigns the rest. Members are never named in the tab.
- P3 The test grid moves but has no resize handles; its size follows Cell, Gap, Count.
- P4 Material choice in the tool bar: a dropdown under the bar (the only popup of a bar); "New material" is named in the Materials tab.
- P5 Cell operation names "<column value> · <row value> · <id>", labels "<id> labels"; unfolded rows show a source line (material, test, cell).
- P6 Tabs on a laser page: Materials (`SwatchBook`) and Fixtures (Holder symbol) after Operations.
- P7 Sets per laser: the page's laser first and open, others folded; "+ <kind> set" per missing kind; Engrave sets hold interval and fill.
- P8 Test detail replaces the material body (back button with the material's name); the best cell carries check + accent frame.
- P9 Fixtures: one heading, On the bed moves into the right control cluster, slots unfold one level in.

**Frage 204 (GRID-REGEN): Grid fields change after its operations were edited by hand**
A: the regenerated operations overwrite the hand edits. B: the grid refuses the change until the hand edits are reset. Recommendation A (the grid stays the one source, one undo restores); default A.

**Frage 205 (OP-BUTTON): What does a kind button in the Select bar do?**
A: as the Add buttons today: a new operation per press, the selection assigned to it. B: assign to the page's last operation of that kind, a new one only if none exists (Shift: always new). Recommendation B (ten presses do not make ten Cut rows); default A (accepted behaviour stays).

**Frage 206 (GRID-SIZE): May the test grid be resized on the canvas?**
A: no, it only moves; size comes from the bar (P3). B: resize handles that scale cell size and gap together. Recommendation A (no hidden change of the test's geometry); default A.
