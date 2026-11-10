# 0101 Laser operations: UX notes

ux-engineer, 2026-11-10 (rework after the customer's review of PR #305, 2026-11-10). Checked against `specification.md` (criteria 1 to 30, with the customer answers of 2026-11-10 in its header and in criteria 3, 4, 18), `specs/0081-inline-delete-confirm/ux-notes.md` (the armed trash: arming, timings, status region, keys), `specs/0050-layers-and-objects-panel` and `specs/0106-move-versus-edit/ux-notes.md` (the Layers rename and drag rules, placed page and inner rows, the refusal notice), `specs/0063-manufacturing-workbenches/concept-output-pages.md`, and `docs/design-system.md` ("Interaction conventions": "Armed action", "Inline delete confirm", "List row", "Canvas notice slot", "Hidden, not disabled", "No popups in the properties panel"; the tokens `--toolbar-bg`, `--toolbar-icon`, `--panel-muted-fg`, `--lane-line`, `--selection-casing`, `--destructive`, `--destructive-soft`; "Drag ghost", "Drop badge", "Casing over artwork"). Customer rules applied: no operating tips or hint labels (2026-10-16), no notice after a success (2026-10-31), hidden not disabled, no popups in the panel, no cut or engrave settings in this step (2026-11-10). Contrast values were computed with the WCAG 2.x formula on the light tokens. Choices marked (Proposal) are vetoable at the demo; section 16 lists them.

Terminology: "operation" and "assignment" as in the spec. The colour of an operation no longer exists. The look has two colours only: **red** for lines to be cut and **pale orange** for hatched areas. There are no `--op-*` tokens, no colour radio group and no swatches anywhere in this feature.

## 1. Principle

The Operations tab is the job read top to bottom: what runs, in which order, and what it is made of. Kind is shape, never colour: a maker reads Cut, Score, Engrave, Engrave + Outline and Cut-out apart by its symbol and its line or hatch, and the two colours only say "this is cut" (red) and "this is engraved" (hatch). Everything that changes the job is a press in the list or on the canvas with the selection; every change is one undo step. Nothing opens a dialog or a popup; the tab's column holds every control.

## 2. The Operations tab layout (criteria 1, 3, 4, 5, 17, 18)

Top to bottom, the tab's column is 244px wide (panel 280px less 12px padding each side, as every tab).

1. **Add group** (28px row). `role="toolbar"` named "Add operation", five icon buttons, 28 x 28px (36 coarse), 4px apart, glyph only (the kind glyphs of section 3). Tooltip and accessible name "Add Cut", "Add Score", "Add Engrave", "Add Engrave + Outline", "Add Cut-out". With a selection on the canvas the description (not the name) reads "Assigns the 3 selected items." Tab stop: one; arrows move, Enter or Space presses. Unchanged from the 2026-11-09 notes.
2. **Cuts last** (28px row, 8px below). Label "Cuts last" 13px `--toolbar-icon`, left; `PanelSwitch` at the right end (`role="switch"`, `aria-checked`, name "Cuts last", no on or off text). Default on for a new laser page. One Tab stop. No tooltip.
3. **List** (flex, the tab's one scroll area, minimum 120px; section 4).
4. **Summary** (below the list, criterion 17; section 7).
5. **View switch** (28px, at the bottom of the tab, Proposal). A segmented control `role="radiogroup"` named "View", segments "Operations" (default) and "Drawn", 13px, one Tab stop, arrows select. Tooltips and names "View: Operations" and "View: Drawn". Session state, not saved (criterion 18).

Empty list: one line "No operations on this page." (14px `--panel-muted-fg`, no glyph, not focusable) in the list area. The summary then reads "Nothing runs. Assign items to an operation." (criterion 17).

## 3. The five kinds (criteria 18, 19 to 23)

Each kind has one **symbol**: a 16 x 16px glyph on a 16-unit grid, 1.5px stroke unless the table says otherwise, drawn in `currentColor`. The same glyph is the row's kind glyph (section 4.2) and the mark on the canvas (section 13.3). Only Cut is red; every other glyph is ink (`--toolbar-icon`).

| Kind | Symbol (16 x 16) | Symbol colour | Canvas line or area, View: Operations (criterion 18) | Section |
|---|---|---|---|---|
| Cut | square outline, 1.5px solid, inset 2px | `--cut` (red) | solid red line, 1 device pixel, on each outline of the item | Cutting |
| Cut-out | square outline, 1.5px dashed 4 on 2 off, inset 2px | `--cut` (red) | dashed red line, 2px, dash 6 on 3 off, on the outer contour of each item only (holes and inner detail do not run) | Cutting |
| Score | one horizontal dotted line through the middle, round dots, 1.5px, dot every 3px | `--toolbar-icon` (ink) | dotted ink line, 1.5px, round dots every 3px, on each outline | Marking |
| Engrave | square with three 45 degree hatch lines inside, 1px ink, no outline | `--toolbar-icon` (ink) | the closed areas of the leaf hatched in `--engrave-hatch` (pale orange), no outline | Marking |
| Engrave + Outline | the Engrave hatch plus a 1.5px ink square outline | `--toolbar-icon` (ink) | the same hatch, plus a 1px solid ink outline | Marking |

- **Line style tells Cut, Cut-out and Score apart; colour only tells "cut" from "not cut".** Cut is solid, Cut-out dashed and only on the outer contour, Score dotted and ink. Cut and Cut-out are both red because both cut (criterion 18).
- **Score is ink, not red** (criterion 18 left this to the notes). Red is for lines to be cut, and a score is not cut. (Proposal, vetoable.)
- **Cut-out needs dashes in criterion 18.** Criterion 18 now says "a red 2 px line"; a solid 2px line next to a 1px solid Cut is hard to tell apart on a canvas at 100 %. The dash (6 on 3) is the difference the customer asked for. The spec line must be changed by the product owner from "a red 2 px line" to "a red 2 px dashed line (6 on 3)". (Proposal, vetoable.)
- **Score glyph changed** from the 0101 notes of 2026-11-09 (dashed square) to a dotted line, so it does not match Cut-out's dashed square at 16px. Cut-out's glyph loses the hole, which at 16px did not read.
- The canvas looks of section 13 use the same line styles; a glyph and the canvas line of a kind never disagree.

## 4. The operation list (criteria 3, 4, 5, 6, 8)

### 4.1 Structure and sections

- With Cuts last on: two `role="group"`s, named "Marking" and "Cutting", each with a header row (24px, 11px semibold `--panel-muted-fg`, a 1px `--divider` line above it except the first; the header word only). Marking lists Engrave, Engrave + Outline, Score in list order; Cutting lists Cut, Cut-out.
- Empty section: its header stays, with a 24px empty drop slot under it. No placeholder text. The slot is the drop target for a kind dragged into that section.
- With Cuts last off: no groups, no header rows, no section line. One flat list.
- Switching Cuts last on or off: no notice (the moved rows are the feedback), no animation. Screen readers hear the switch state only.

### 4.2 Row (40px; 48px with a coarse pointer)

One `listitem` per operation. Two lines in a 40px row. Line 1 (top 2px to 26px) holds the name and the buttons; line 2 (26px to 40px) holds the number and the kind line.

**Line 1**, left to right, centred on the 24px button line:

| Slot | Width | Content | Notes |
|---|---|---|---|
| Glyph | 16px | the kind symbol of section 3 | `aria-hidden`; while items are selected, the tick takes this slot (section 6.2) |
| Gap | 4px | | |
| Name | flexible (about 80px) | 13px `--toolbar-icon`, one line, ellipsis; tooltip with the whole name | double-click or F2 renames in place (section 4.4) |
| Show | 24px | `Eye` on, `EyeOff` off, `aria-pressed` | section 5 |
| Output | 24px | `Power` on, `PowerOff` off, `aria-pressed` | section 5 |
| Trash | 24px | `Trash2` 16px, `--panel-muted-fg`; armed look in section 4.5 | next to Output, as the customer asked |
| Group gap | 8px | | separates the state switches from the order arrows |
| Up | 24px | `ChevronUp` 16px | section 4.6; hidden at the ends (slot kept) |
| Down | 24px | `ChevronDown` 16px | section 4.6; hidden at the ends (slot kept) |

**Line 2** (11px, full width, `--panel-muted-fg`): the number, " · ", the kind word, " · " and the item count "4 items" (count "1 item" singular), or "Output off" in place of the count when off. Example: "2 · Score · 6 items". The number is the digit route of section 9 (1 to 9 for the first nine rows, then 10, 11 ...). Line 2 has no buttons, so it runs under the full row width.

- Row ground: none. Hover: `--editor-accent-hover` (List row). Selected row (section 4.4): `--value-fill` ground plus a 3px `--accent` bar at the left edge. Focus: 2px `--editor-accent` ring inside the row.
- Show or Output off: the glyph changes shape (Eye to EyeOff, Power to PowerOff), the name goes to `--panel-muted-fg`, and line 2 says "Output off" where it applies. Muted ink plus a word, never opacity alone.
- Unknown operation (criterion 26): glyph `CircleHelp` 16px `--panel-muted-fg`, name kept, line 2 "<n> · Unknown operation". Show and Output are not in the tree; Trash stays (it is the only way to remove an unknown operation); Up and Down stay as for any row.
- The name is 80px at 13px: about 11 characters. Long default names ("Engrave + Outline 12") are truncated; the tooltip and the accessible name carry the whole name. This is the price of five buttons on a 244px column (section 16, decision 5).
- Coarse pointer: buttons 28px, 4px apart, row 48px. The name takes the loss; no other change.

### 4.3 Accessible names and states

- The list: `role="list"` named "Operations". Rows: `listitem`, with `aria-posinset` and `aria-setsize` counted over the whole list (sections included). Section groups: named "Marking" and "Cutting".
- Row name (accessible name of the row's content): "<name>, <kind>, <n> items", plus ", Output off" where it applies, e.g. "Cut 1, Cut, 4 items". The buttons are read by their own names (below), not by the row name.
- Show: name "Show <name>", `aria-pressed`. Output: name "Output <name>", `aria-pressed`. Constant label plus pressed state (`icon-context`, `aria-labels`).
- Trash: name "Delete <name>", tooltip "Delete". Armed: name "Press again to delete <name>", tooltip "Press again to delete" (section 4.5).
- Up: name "Move <name> up", tooltip "Move up". Down: name "Move <name> down", tooltip "Move down".
- Glyph: `aria-hidden`; the kind is in line 2 and in the row name.
- Drag has no handle of its own (no grip). Its keyboard and button routes are Alt+arrows and the Up and Down buttons (WCAG 2.2 dragging alternative).

### 4.4 Selection and rename

- **Select:** a press on a row (glyph, name or line 2) selects it: the `--value-fill` ground and the 3px bar (List row, `aria-selected` on the row). One row is selected at a time. A selection of a row does not change the canvas selection (Proposal; the operation's items are selected by its summary count, section 7, and by the Layers route of section 6.4).
- **Enter or Space** on a row selects it, as a press does. There are no details any more; there is nothing else for Enter to open.
- **Rename in place (criterion 3):** a double-click on the name label, or F2 on the selected row, turns the row's line 1 into a 13px text field: white ground, 1px `--editor-accent` border, 4px radius, from the glyph's place to the row's right edge. The Show, Output, Trash, Up and Down buttons leave the tree while the field is open (hidden, not disabled), so the field has the full width; they return on commit or cancel. Enter commits; leaving the field (a press elsewhere, a Tab out) commits; Escape cancels and the label returns; focus stays on the row. 128 characters. Empty name reverts. Names need not be unique (criterion 3). One undo step "Rename operation".
- Touch: a double-tap on the label sends a double-click and renames; F2 has no touch route, and none is needed for touch.
- The rename field has the name "Name of <name>" and is the focus while open. Digits and letters are text while the field has focus; the digit route of section 9 is off then.
- Renaming a row with an armed trash: the arming ends (focus leaves the row's button; the field takes focus).

### 4.5 Delete: the armed trash (criterion 4, `0081`)

The trash is the armed button of `0081` (the family and tag trash of the Fonts tab), with the same timings and rules. The operation's delete stays undoable (one undo step); the customer chose the armed look anyway (section 16, decision 8).

- **Idle:** `Trash2` 16px, `--panel-muted-fg`, 24 x 24 button; hover `--editor-accent-hover` ground; always visible in every row, including unknown operations.
- **First press arms:** the whole row takes `--destructive-soft` (both lines, inside the row's 5px radius; hover ground suppressed while armed); the trash becomes a 22 x 22 square, 5px radius, filled `--destructive`, white `Trash2` 16px. Text, Show, Output and the arrows keep their colours (`0081` section 1 measured them). No width or height changes. The accessible name and tooltip switch to the armed text.
- **Second press deletes**, if it is at least 400 ms after arming. A press within 400 ms of arming is ignored, no feedback. The operation is removed with its memberships (criterion 4); items no other operation holds become Not assigned. The row disappears; focus goes to the next row, else the previous one, else the first Add button (section 9).
- **Disarm** without notice: after 4 s from the arming press (no countdown, no progress), on Escape (consumed: a second Escape does what it does today), on any press elsewhere, when focus leaves the row, when the list changes (reorder, a Cuts last switch, a row added or removed, expand or collapse of a placed page). A second press after a disarm arms again.
- **One armed trash in the app at a time** (criterion 4): arming another disarms the first.
- **Keys:** `Delete` on the focused row arms its trash (focus stays on the row). `Delete` or `Enter` on the armed row removes after the guard. Left and Right move to the trash inside the row and Enter or Space press it (section 9). Arrow keys move focus to another row and disarm.
- **Status region:** the shared polite `role="status"` region of the tab (sr-only) says "Press again to delete Cut 1. Its 4 items become Not assigned." on arming, "Cut 1 deleted." on removal, and is cleared on disarm. No visible notice on removal: an undoable delete after a success stays silent (2026-10-31); the undo label is the record.
- Motion: arm and disarm change the row ground and the trash square over 120 ms ease-out; instant under `prefers-reduced-motion`. No countdown, no shake.
- Contrast of the armed row: `--toolbar-icon` on `--destructive-soft` 8.2:1, `--panel-muted-fg` 4.95:1 (`0081` section 1). The ground is the colour cue; the filled square is the state a colour-blind maker reads.

### 4.6 Order: arrows and drag (criteria 5, 6)

**Arrows.** Up and Down buttons per row (section 4.2). Up is removed from the tree on the first row of the list and on the first row of the Cutting section; Down on the last row of the list and on the last row of the Marking section (with Cuts last on). The slot stays, so the buttons of the other rows do not move. Pressing an arrow moves the row one place and is one undo step "Move operation". Focus follows the row: it stays on the same arrow; if that arrow leaves the tree at the new place, focus goes to the other arrow, else to the row's name (never to `body`). The row stays selected. Arrows never cross the section line; a Cut or Cut-out never gets a Down or Up that would cross it.

**Drag.** Press the glyph or the name and move. The drag starts after 4px (a click never reorders). A 2px `--accent` insertion line shows where the row would land (with Cuts last on, only inside its section). The source row stays in place with a 1.5px dashed `--drag-ghost` outline and no ground; a ghost row at 60% follows the pointer, vertically only, clamped to the list (the 0100 Drag ghost rule). A press on the Show, Output, Trash or arrow buttons never starts a drag.
- Cut or Cut-out dragged above the section line: no insertion line, the ghost snaps back on release, no text (criterion 5). `cursor: not-allowed` over the refused position.
- Escape during the drag cancels; no step.
- Release: one undo step "Move operation". Undo returns the order exactly.
- Drag works while a selection exists too. The tick in the glyph's slot is a press target of its own and never starts a drag (section 6.2).

**Keyboard reorder:** Alt+Up and Alt+Down move one place inside the section (or anywhere with Cuts last off); Alt+Home and Alt+End to the first and last place of the section; refused at the section line (no step, no text).

### 4.7 Cuts last switch states

- On: the two sections and the section line (the group header's top border). Drag, arrows and Alt+arrows work inside a section.
- Off: one list, no section line, no headers. Drag, arrows and Alt+arrows work anywhere.
- Each switch is one undo step: "Cuts last on", "Cuts last off" (criterion 6). Off moves nothing.

## 5. Show and Output (criteria 15, 16)

- **Show** (`Eye`, view only): off hides the operation's leaves on the output page's canvas. Items with a Show-off operation are not drawn unless another operation with Show on holds them (criterion 15). It never changes what runs.
- **Output** (`Power`, what runs): off means nothing of that operation runs (criterion 16). Output on with Show off runs, and the summary counts it under "items run while hidden".
- Each press is one undo step, labelled by the change it makes: "Hide Cut 1" or "Show Cut 1" for Show; "Turn off Output of Cut 1" or "Turn on Output of Cut 1" for Output.
- No notice on either. The output page changes at once; the summary changes in the same frame.
- Pointer targets 24 x 24px (the WCAG minimum), 4px apart on the row; Output and Trash are neighbours: an unintended press on Trash only arms it (section 4.5), so the neighbour is safe.

## 6. Assigning an operation (criteria 2, 8, 9, 10, 11)

### 6.1 The add group with a selection

With items selected on the output page, pressing an Add button creates the operation (criterion 2) and assigns the selection, one undo step "Assign to <name>". The new row is selected (section 4.4). Focus stays on the Add button, so a second press adds the next operation (Cut, Cut: "Cut 1" and "Cut 2", as the naming rule gives).

### 6.2 Ticks in the list rows (criterion 8)

- No separate block. While items that can take an operation are selected on the output page, every operation row shows a tick in the glyph's slot (the glyph hides; the row width does not change). Without a selection the row shows its glyph.
- Tick: `role="checkbox"` 16px (`.cb` look, 1.5px border), name "<name>", `aria-checked` true, false, or `mixed` when only some of the selected items are in the operation. Tooltip "Assign to <name>" or "Remove from <name>", by the press it will do. Description "<n> of <m> selected items in it".
- Press rule: a tick press on an unticked or mixed row adds all selected items (one step "Assign to <name>"); on a ticked row it removes all of them (one step "Remove from <name>"). A mixed row becomes fully ticked on press, never cleared directly.
- The tick is a sibling control of the row: Left and Right reach it from the row (section 9), Space or Enter presses it. Enter or Space on the row itself selects the row (section 4.4).
- Items that cannot take an operation (criterion 10) are not counted in the ticks and do not change them.
- The list stays the tab's one scroll area, windowed as in section 12.

### 6.3 Assignment by drag from the canvas

- A Select drag that starts on a selected item and is released over an operation row of the Operations tab assigns the selection; it does not move the objects. Cursor badge (the `Drop badge` look: pill 20px, `--toolbar-bg`, 12px glyph and word): the kind symbol and "Assign to <name>". Over a refused place (a section that does not take the kind, or another panel): `Ban` and "Not here", `not-allowed`. Escape cancels, no step.
- The row under the pointer gets the drop ground `--value-fill` with a 1.5px `--accent` inset outline.
- Risk noted: a canvas drag that ends over the panel must never move the objects. The release over a row is the assignment; the release on the canvas is the move. Test both (review checklist).

### 6.4 Assignment in Layers (Frage 170 default A)

- In the Layers tree of the laser page, selecting rows (press, Shift-press, Ctrl-press, arrows and Shift+arrows, `0050`) selects the canvas items; the Operations tab then assigns them as in 6.1 to 6.3. This is the route for own items, clones and placed pages.
- **Inner rows** (criterion 11, lead decision 2026-11-09): a press on an inner row selects the inner object, so it can take an assignment. This amends `0106` section 5 (row added to the spec's "Changes to accepted specs"). The inner object stays read-only for edits: its box is dashed with no handles and no Select bar; moves, Delete, arrows, Ctrl+D and the edit tools on it are refused with the `0106` refusal notice ("Not editable here: on Coaster."). The Operations tab ticks and kind buttons act on it.
- **Clone rows and clone member rows** (criterion 11, Frage 167 B): a clone row expands like a placed page row into its members as read-only rows in the original's layer and group order. A member row is an inner row. Its assignment is the path key (the clone, then the member), so it applies to that member in that clone only. The clone row takes an assignment as a whole.
- **Chips** (criterion 18, change to `0106`): placed page rows, clone rows, inner rows and clone member rows show their assignments as chips: a row of up to three chips, 20px high, 1px `--lane-line` border, 4px apart, each with the kind symbol at 12px (ink, Cut red) and the operation name at 11px; a fourth and later chip is "+2" as text. Chips are static: no press, no tab stop. Placed page rows and clone rows: chips on line 2, row height 44px. Inner and clone member rows: chips at 26px row height, 60% text as `0106`. Own items show no chips (their assignment is on the canvas and in the Operations tab).
- Accessible name of a placed page row carries the chips: "Placed page of Coaster, 50 objects, read-only, Cut 1, Engrave 2". Clone row: "Clone of <name>, <n> objects, read-only, <chips>". Clone member row: "<name>, in clone, read-only, <chips>".

### 6.5 Guides, dimensions and fixtures (criterion 10)

- A selection of mixed targets and non-targets assigns the targets and shows the notice "2 guides not assigned. Guides never run." (the text of the 2026-11-09 notes, criterion 10). Canvas notice slot, `role="status"`, 5s above 70 characters. It is a partial result, so it is kept despite "no notice after a success" (the 0100 clone rule).
- A selection of only guides, dimensions or fixtures: nothing is created (no operation, no step); the refusal look (`role="alert"`, 8s) shows "Guides never run." when guides are in the selection (Proposal). A selection of only dimensions is refused silently.

## 7. The summary (criterion 17)

- Under the list, 12px, `--toolbar-icon`, line height 16px, tabular figures. Lines:
  - "Runs <N> operations, <M> items." (M counts each item once; "1 operation", "1 item" singular).
  - Only when above zero, each as its own line: "<K> items not assigned.", "<J> items run while hidden.", "<H> items sit in operations with Output off."
- Each count is a text button named by its whole line: pressing it selects those items on the canvas and in the Layers tree. The line is the button's text, underlined on hover and focus (`--editor-accent`), no icon, a 24px row.
- Empty output: "Nothing runs. Assign items to an operation." as the only line, no button.
- The summary updates in the same frame as any change. Screen readers: `role="status"` with `aria-atomic="true"`, so the whole phrase is read per change.
- Hidden items in "run while hidden": the press selects them even though they are not drawn (Proposal; overrides the Layers rule "a hidden row refuses a press" for this route only). The Layers rows keep their hidden glyph, so the selection is explained by the row state.

## 8. Unknown operations (criterion 26) and deleted objects (criterion 27, Frage 169 A)

- **Unknown operation:** section 4.2 (glyph, line 2, Trash only).
- **Deleted objects** show nothing on the laser page: no row, no count, no notice, no symbol. Their assignments go with them in the same step (criterion 27). Undo of the deletion brings the object back with its assignments, in the one undo step of the deletion.
- Nothing here is a warning state: the customer chose the silent drop (Frage 169 A).

## 9. Keyboard routes

| Action | Route | Notes |
|---|---|---|
| Enter the tab's controls | Tab: add group, then Cuts last, then the list (one stop), then the summary buttons, the View switch | the tab order matches the visual order |
| Press an Add button | Arrows move in the add group (toolbar), Enter or Space presses | one Tab stop |
| Cuts last | Space or Enter | |
| Move between rows | Up and Down (section headers skipped), Home and End, Page Up and Page Down (5 rows); no wrap | roving focus, one Tab stop, `0050` keys |
| Select a row | Enter or Space on the row | section 4.4 |
| Rename | F2 on the selected row | section 4.4; the double-click is the mouse route |
| Commit or cancel a rename | Enter commits; Escape cancels and focus stays on the row | leaving the field commits |
| Show, Output, Trash, Up, Down | Left and Right move among the row's controls in order: tick (only while a selection exists), Show, Output, Trash, Up, Down; Space or Enter presses the focused one | `List row` rule; Up and Down (the keys) still move between rows |
| Arm and delete | `Delete` on the focused row arms its trash; `Delete` or `Enter` on the armed row removes | section 4.5; focus stays on the row until removal |
| Disarm | Escape (consumed); Up, Down, Home, End, Page keys, Tab, F2 leave the row and disarm | a press on another control of the row also disarms |
| Reorder | Alt+Up and Alt+Down, Alt+Home and Alt+End | section 4.6; refused at the section line, no step |
| Move with the arrow buttons | Left and Right to the arrow, Space or Enter | focus follows the row |
| Assign the selection to operation n | digit 1 to 9 on the canvas, Select tool, with items selected, no Ctrl, Cmd or Alt, no drag, no open rename field | Proposal, criterion 8; gated like a letter |
| Digit semantics | If every selected item is in operation n, the press removes all; otherwise it adds all. One undo step "Assign to <name>" or "Remove from <name>" | same as the tick press |
| Digit with no operation n | nothing; no step, no notice | |
| Count button in the summary | Tab to it, Space or Enter selects those items | |
| View switch | Tab to it, arrows select | |
| Leave the list | Escape returns focus to the canvas (the first press in the list); a press on the canvas leaves the list too | |
| Undo | Ctrl+Z, as built | every step of section 11 is one undo |

Notes:
- No letter key is bound to a kind: C, S, E and the other letters are tool letters (`S` is the Select tool, `E` the Ellipse tool). The kinds are reached by Tab and arrows in the add group, and the operations by the digit route. The number on line 2 shows the digit, so the route is visible without a tip.
- Every drag has a keyboard route: reorder by Alt+arrows and the arrow buttons, assignment by digits and ticks (WCAG 2.2 dragging alternative).
- Digits and letters are text while the rename field has focus.
- Clone rows expand and collapse with the keys of placed page rows (`0106`); member rows follow the inner row keys.

## 10. Accessible names (complete list)

| Element | Name | State or description |
|---|---|---|
| Add group | "Add operation" (toolbar) | |
| Add buttons | "Add Cut", "Add Score", "Add Engrave", "Add Engrave + Outline", "Add Cut-out" | description "Assigns the <n> selected items." with a selection |
| Cuts last | "Cuts last" (switch) | `aria-checked` |
| List | "Operations" | |
| Section groups | "Marking", "Cutting" | |
| Row | "<name>, <kind>, <n> items" (plus ", Output off") | `aria-posinset`, `aria-setsize`, `aria-selected` on the selected row |
| Show | "Show <name>" | `aria-pressed` |
| Output | "Output <name>" | `aria-pressed` |
| Delete (trash) | "Delete <name>" | armed: "Press again to delete <name>" (tooltip "Press again to delete") |
| Move up, move down | "Move <name> up", "Move <name> down" | hidden at the ends, not disabled |
| Rename field | "Name of <name>" | open only while renaming |
| Tick (with a selection) | "<name>" (checkbox), tooltip "Assign to <name>" | `aria-checked` true, false or mixed; description "<n> of <m> selected items in it" |
| Unknown operation | "<name>, unknown operation" | |
| Summary | "Operations summary" (status region, `aria-atomic`) | each count button named by its line |
| View switch | "View" (radiogroup) | "Operations" and "Drawn", `aria-checked` |
| Placed page row | "Placed page of <name>, <n> objects, read-only, <chips>" | section 6.4 |
| Inner row | "<name>, in placed page, read-only, <chips>" | `aria-readonly="true"` |
| Clone row | "Clone of <name>, <n> objects, read-only, <chips>" | `aria-expanded` |
| Clone member row | "<name>, in clone, read-only, <chips>" | `aria-readonly="true"` |

Live regions: the summary (`role="status"`, `aria-atomic`), the tab's sr-only status region (arming, removal, section 4.5), the notices of sections 6.5 and 11 (canvas notice slot, `role="status"` or `role="alert"`). Added or removed rows are announced as "<name> added" or "<name> removed" through the polite status region (Proposal; not a visible notice).

## 11. Undo steps (one per gesture)

"Add operation", "Assign to <name>", "Remove from <name>", "Rename operation", "Delete operation", "Move operation" (drag, arrows and Alt+arrows), "Cuts last on", "Cuts last off", "Show <name>" / "Hide <name>", "Turn on Output of <name>" / "Turn off Output of <name>". The undo label names the change; no step for View, for a focus move, for a row selection or for a canvas selection. A deletion is one step with its own label; undo brings back the assignments it held (criterion 27).

## 12. Empty, error and large states

- **No laser page operations and no items:** the list shows "No operations on this page.", the summary "Nothing runs. Assign items to an operation.", the Layers tree its own empty line "Nothing on this page yet." (`0106`).
- **Operations with no items:** the rows show "0 items" on line 2; the summary "Runs 2 operations, 0 items." (plural by count) and no further line.
- **All items not assigned:** the summary's "<K> items not assigned." line is the only extra line; the canvas draws every leaf in the Not assigned look (section 13.2).
- **Last operation deleted:** the list shows "No operations on this page."; focus goes to the first Add button.
- **Extension off** (criterion 28): the tab is absent. Nothing is shown in its place.
- **Selection of mixed or non-target items:** section 6.5.
- **Refusals:** section 4.6 (a Cut above the section line: snap back, no text), section 6.5 (guides), section 6.3 (refused drop: `Ban` "Not here"), and the Layers refusal of `0106` for moves of inner objects.
- **Large documents (criterion 29):** the list is windowed (fixed row height, at most 60 row elements in the DOM, the focused and the selected row kept when they scroll out, scrollbar exact from the first frame). 50 operations and 1,000 items: the first rows paint at once; the ticks sit in the same windowed rows; no second scroll area. The summary is computed once per change, not per frame. Each pointer move of a drag updates the insertion line only (criterion 29 d).
- **Budget note, the symbols on the canvas (section 13.4):** a cap of 500 symbol chips per frame, collisions skipped, selected items first. Symbols never change what is hit or what runs; the summary carries the count of every item that is skipped.

## 13. Canvas look: two colours, symbols, budget

### 13.1 Tokens (light theme)

| Token | Value | Use | Contrast (measured 2026-11-10) |
|---|---|---|---|
| `--cut` (new) | `#C8102E` | Cut and Cut-out lines (the Cut symbol and Cut-out symbol) | 4.81:1 on `--canvas-bg` `#E8E8EB`; 2.98:1 on `--pasteboard-bg` `#B8B8BE` (bounded by the white casing, 5.88:1 against the red, section 13.2); 4.30:1 on `--toolbar-bg` (chip glyph) |
| `--engrave-hatch` (new) | `#C46A1C` | Hatch lines of Engrave and Engrave + Outline, 1.5px, 45 degrees, 6px period, screen-space | 3.17:1 on `--canvas-bg` (criterion 18, 3:1 met); 1.96:1 on the pasteboard, inside closed areas only |
| `--toolbar-icon` (existing) | `#3A3A3F` | Score line, Engrave + Outline outline, every ink symbol | 9.25:1 on `--canvas-bg`; 8.27:1 on `--toolbar-bg` |
| `--lane-line` (existing) | `#7B7B7F` | Not assigned outline (dashed) and the chip border | 3.45:1 on `--canvas-bg` |
| `--panel-muted-fg` (existing) | `#5A5A5F` | Trash idle, Output-off symbol, Unknown glyph | 5.61:1 on `--canvas-bg` |
| `--selection-casing` (existing) | `#FFFFFF` | White casing 1px each side under every operation line | |

- **Pale by coverage, not by hue.** Criterion 18 asks for red and pale orange, each at least 3:1 against the page ground. A pale orange such as `#F2A65A` is 1.65:1 on the canvas and cannot pass; 3:1 needs a value near `#C46A1C` or darker. The hatch therefore uses the 3:1 colour and reads as pale because it covers about a quarter of the area (1.5px lines every 6px). This is the reconciliation the spec's "pale orange" needs. (Proposal, decision 1.)
- Dark theme: the app is light only today; the names are ready, no dark values are proposed here.
- The tokens are added to `docs/design-system.md` at acceptance, with the rows "Operation symbols (`0101`)" and "Operation look (`0101`)"; the `--op-*` tokens never enter it.

### 13.2 The canvas look (View: Operations)

- **Draw order:** each leaf in its operation look, operations in list order (the later one on top). Lines are drawn over their white casing: 1px `--selection-casing` on each side of every operation line, on every ground (the casing rule of "Casing over artwork"), so the line never depends on the ground under it.
- **Cut:** solid `--cut` line, 1 device pixel, on each outline of the leaf.
- **Cut-out:** dashed `--cut` line, 2px, dash 6 on 3 off, on the outer contour of the union of the item's leaves only (criterion 23; holes and inner detail do not run).
- **Score:** dotted `--toolbar-icon` line, 1.5px, round dots every 3px, on each outline.
- **Engrave:** the closed areas of the leaf (holes included, even-odd) hatched in `--engrave-hatch`. No outline.
- **Engrave + Outline:** the same hatch, and a 1px solid `--toolbar-icon` outline.
- **Not assigned** (criterion 14): dimmed, a 1px dashed outline (dash 4 on 3 off) in `--lane-line`, no hatch, no symbol.
- **Output off** (Proposal, decision 4): drawn in the Not assigned look (it does not run), its symbols drawn in `--panel-muted-fg`.
- **Show off** (criterion 15): not drawn, no symbol.
- **Several operations on one leaf:** each look in list order; the hatch and the lines of later operations stay on top.
- The look never changes a layout or a position (criterion 18).
- **View: Drawn:** the drawing's own style of every item and no symbols (criterion 18).

### 13.3 Symbol placement (View: Operations only)

- **One symbol per kind the object is in** (criterion 18, Proposal: two operations of one kind give one symbol). A symbol is a 22 x 22px chip: `--toolbar-bg` ground, 1px `--lane-line` border, 4px radius, the 16px symbol inside at 2px inset, centred. Chips are 4px apart, in the kind order Cut, Cut-out, Score, Engrave, Engrave + Outline, from left to right.
- **Anchor:** the chip row starts 10px to the right of the object's bounding box top-right corner, its bottom edge 6px above the box's top edge (screen-space, constant at every zoom). The corner resize handle, the right edge handle and the rotate handle (above the top-centre) stay clear: the row never sits under a handle. If the row would leave the viewport on the right, it mirrors to the top-left corner (ends 10px left of it).
- **Chips never cover a handle or a reference box.** A chip whose 22px box would touch a handle's hit area is not drawn (for a multi-selection, the group box's handles win).
- **Not in the way of the hatching:** chips sit outside the object's box, so they never lie over the hatch of the object itself. They may lie over the hatch or lines of a neighbour; the chip's ground is opaque, so it reads on any fill.
- **Hit testing:** chips are `pointer-events: none`. A press on a chip is a press on the canvas beneath it.
- **Glyph colour:** Cut and Cut-out chips in `--cut` (4.30:1 on the chip ground); every other chip in `--toolbar-icon` (8.27:1); the Output-off symbols in `--panel-muted-fg` (about 5:1). The chip carries no colour of its own.
- The symbols carry no name for the screen reader (the canvas is not in the accessibility tree). The kinds are read from the Layers chips and the summary.

### 13.4 Budget: many items

- **Scope:** only leaves whose box meets the viewport get chips. Leaves hidden by Show get none.
- **Order:** selected items first, then the rest in z order (top first).
- **Cap:** at most 500 chips per frame. Past the cap, the remaining chips are not drawn; the summary still counts every item.
- **Collisions:** a chip that would overlap a chip already drawn (at this zoom, items close together) is not drawn. At small zoom the canvas shows the chips of the items that are apart and the selected ones; the rest are counted in the summary.
- **Cost:** one pass over the visible leaves per frame; the hatch is one clipped pattern fill per leaf, no per-line geometry. Thread CPU budget as criterion 29 (the frame work of `curvyo-editor-wasm`); the implementer measures it with a test under `curvyo-editor-wasm/tests/cpu_time/`. (Proposal: the 500 cap and the collision skip are the first numbers to measure.)
- **No setting:** there is no switch to hide chips (no speculative option). If the customer wants one, it is a later story.

## 14. Review checklist for the implementer

- Add group: five buttons, toolbar keys, glyph and tooltip names, no popup; a selection changes only the description.
- Cuts last: switch row at the top, two groups with headers, no "runs last" text, empty section keeps a 24px drop slot; off removes the section line and moves nothing.
- Rows: 40px (48 coarse); line 1 glyph, name, Show, Output, Trash, Up, Down; line 2 digit, kind, count or "Output off"; no colour, no swatch, no grip.
- Trash: idle 24 x 24, armed row ground and 22 x 22 filled square, 400 ms guard, 4 s timeout, Escape consumed, one armed at a time, status region, no notice on removal.
- Arrows: hidden at the list ends and at the section line, slot kept; focus follows the row; one undo step per press.
- Drag: starts after 4px on glyph or name only; insertion line; ghost; a refused Cut above the line snaps back with no text; Escape cancels; one step.
- Rename: double-click and F2 open the in-place field; the buttons leave the tree while open; Enter, leaving and Escape as section 4.4; "Rename operation" as one step.
- Show and Output: each one step, undo labels as section 11, no notice, summary updated in the same frame.
- Ticks in the rows: shown only with a selection, mixed state, press rules, Left and Right order, description with the count.
- Canvas drag onto a row: Drop badge, row drop ground, no move of the objects when released over the panel.
- Layers: chips with kind symbol and name on placed page, clone and inner rows; inner row press selects the inner object, read-only for edits; no chips on own items.
- Canvas look: Cut solid red 1 device px; Cut-out dashed red 2px outer only; Score dotted ink; Engrave hatch in `--engrave-hatch`; Engrave + Outline hatch plus ink outline; casing under every line; Not assigned dashed; Output off as Not assigned.
- Symbols: one chip per kind, kind order, anchor and mirror rule, no handle overlap, pointer-events none, cap 500, collision skip, View: Drawn shows none.
- Contrast: `--cut` 4.81:1 and `--engrave-hatch` 3.17:1 on `--canvas-bg`, casing on the pasteboard, `--toolbar-icon` and `--panel-muted-fg` on the panel, chip glyph on `--toolbar-bg`.
- Large: 50 operations, 1,000 items, windowed rows, at most 60 DOM rows, ticks without an extra scroll area.
- Screen reader: list and group names, row names with kind and count, pressed states, button names, "Press again to delete" phrasing, the rename field name, the summary as a whole phrase.
- Touch: 40px rows (48 coarse), buttons 28px on the coarse pointer, double-tap renames; no hover-only control.

## 15. Keyboard and focus (summary)

Focus after a press in the add group: stays on that button. After a delete: the next row. After a rename: the same row. After a row move by arrow: the same arrow, else the other arrow, else the row. After a drag: the row. Focus is never dropped to the document body. Escape in the list returns focus to the canvas (first press), the second Escape does what it does on the canvas.

## 16. Decisions and open points for the lead (vetoable at the demo)

1. **Colours:** red `--cut` `#C8102E` and hatch `--engrave-hatch` `#C46A1C` (new tokens, section 13.1). "Pale" is reached by coverage (1.5px lines every 6px), since 3:1 on the canvas needs the darker orange. A lighter hatch fails criterion 18.
2. **Score is ink and dotted, not red** (section 3). Red stays for cut lines only.
3. **Cut-out is dashed** (6 on 3, 2px, outer contour). Criterion 18 must change from "a red 2 px line" to "a red 2 px dashed line"; the product owner edits the spec.
4. **Output off is drawn as Not assigned, its symbols muted** (section 13.2).
5. **Row layout:** five buttons on line 1 leave the name about 80px; the grip is gone, so drag starts on glyph or name; the digit moved to line 2; rows are 40px. The name is truncated more than before. The alternative (a wider column) is not open.
6. **A row press selects the row only, with no canvas effect**; there are no details any more. Enter and Space select the row. The items of an operation are selected by the summary count or by the Layers route (Proposal).
7. **Rename hides the row's buttons while the field is open**, so the field gets the full row width; touch renames by double-tap.
8. **The trash is armed although Delete is undoable.** `docs/design-system.md` ("Interaction conventions", "Inline delete confirm" and "Armed action") says a reversible action never asks. The customer asked for the armed trash of `0081` on 2026-11-10, so it is built that way; the rule of the design system is the one to weigh against it. Decide whether the operation's delete should instead be direct with its undo (the Layers rule of `0050` criterion 36).
9. **Symbols: one chip per kind**, 22px chips at the top-right corner, cap 500 per frame, collisions skipped, View: Operations only (sections 13.3, 13.4). Chips at a small zoom can be missing for some items; the summary counts them.
10. **Layers chips show the kind symbol** (12px) instead of the colour square, on placed, clone and inner rows (section 6.4).

## Links

Spec: `specs/0101-laser-operations/specification.md` (criteria 3, 4, 5, 14, 18 and the header of 2026-11-10). Previous version of these notes: git history of this file (2026-11-09). Armed trash: `specs/0081-inline-delete-confirm/ux-notes.md`. Layers rename and drag: `specs/0050-layers-and-objects-panel/` (through the design system row "Layers variant"). Placed page and inner rows: `specs/0106-move-versus-edit/ux-notes.md`. Concept: `specs/0063-manufacturing-workbenches/concept-output-pages.md`. Design system rows to be added at acceptance: "Operation symbols (`0101`)", "Operation look (`0101`)", tokens `--cut`, `--engrave-hatch`. Customer rules: no operating tips, no success notices, no popups in the panel, no cut or engrave settings in this step. PR: #305 (reviewed 2026-11-10; the rework follows it).
