# 0111 Laser operation tools: UX notes

ux-engineer, 2026-11-12. Written from `specification.md` (criteria 1 to 27, Fragen 212 and 213 with their defaults A), the accepted drafts `docs/design-drafts/2026-11-laser-redesign/` (README P1, P2, P10, P11; screens 01 and 03: "die screens sehen gut aus"), the customer decisions of 2026-11-12 (Fragen 197 A, 198, 205 B, the operation-only settings, the motorised Z axis), `docs/design-system.md` (rows "Select bar layout", "Entered group", "Link corners toggle", "Kind chip places", "Action notice"; "Interaction conventions": "No operating tips", "No notice after a success", "Hidden, not disabled", "No popups in the properties panel", "List row", "Armed action", "Canvas notice slot") and the built notes `specs/0101-laser-operations/ux-notes.md`, `specs/0028-material-test-library/ux-notes.md` (section 8, the settings block), `specs/0106-move-versus-edit/ux-notes.md`. Customer rules applied: no invented colours, symbols plus words (here: symbols with tooltip and accessible name, as the customer asked for the bar), no tips, no success notice, no popups in the panel, hidden not disabled, plain fields and no sliders. Items marked (Proposal) are vetoable at the demo; section 13 lists them as T1 to T12.

Terminology: "kind button" is one of the five Select-bar buttons, "unfolded row" the one operation row opened in place, "lower part" the operation-only settings under the hairline. "Ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from (code) | Change in this slice |
|---|---|---|
| Kind glyphs | `OperationGlyph` (`frontend/src/components/panel/OperationGlyphs.tsx`, `0101` section 3) | none; now also in the Select bar |
| Select bar groups, toggles, tooltips | `SelectToolbar.tsx` (Radix `Toggle` and `Tooltip`, 400 ms, `GROUP_CLASS`, the 1px 20px dividers) | a sixth group "Operation" after the settings switches |
| Assignment | `useOperations`, `useMembership` (the tick and digit routes of `0101` 6.2 and section 9) | the reuse rule of criterion 3, the group rule of criterion 9 |
| Operation row | `OperationRowView` (`OperationRow.tsx`), `operationRows.ts`, `useDeleteArm` | press unfolds; no other change to line 1 or line 2 |
| Numbers block | `OperationSettingsBlock.tsx`, `useOperationSettings`, `operationSettingsRows.ts`, `operationSettingsView.ts`, `MaterialPick.tsx` | moves from below the list into the unfolded row; Interval and the lower part added |
| Windowing | `useScrollWindow`, `operationWindow.ts` | one window item may be tall (the unfolded row) |
| Entered-group crumb | `OpenGroups.tsx` (row "Entered group") | root crumb is the page name; "Read-only" segment |
| Page label | `PageFrames.tsx` with `KindChip` (row "Kind chip places") | the Operations view toggle after the chip |
| Notices | `CanvasNotice.tsx`, `useActionNotice` | texts of section 10 |

## 1. Principle

What runs is said where the work is: the maker selects on the canvas and presses one of five symbols beside the selection. A second press of the same symbol keeps filling the operation the page already has, so a job of three operations stays three rows. A group is one thing until the maker opens it; inside, nothing can be moved, only given an operation. The Operations tab is the list of what runs and nothing else; its numbers unfold under the one row they belong to. Nothing opens a dialog or a popup.

## 2. Tokens and symbols (no new colour)

| Use | Token | Contrast |
|---|---|---|
| Cut and Cut-out glyphs | `--cut` | 4.30:1 on `--toolbar-bg` |
| Every other glyph, labels, field text | `--toolbar-icon` | 8.27:1 on `--toolbar-bg` |
| Line 2, units, the source line, muted Z note | `--panel-muted-fg` | 5.0:1 |
| Pressed icon toggle (Operations view, Crosshatch, Bidirectional fill) | `--toolbar-icon-active-bg` fill, `--toolbar-icon-active-fg` glyph (the "Link corners toggle" look) | 4.5:1 white on `#2F6FEE` |
| Hover of bar and panel buttons | `--editor-accent-hover` | |
| Focus | 2px `--editor-accent` ring, 1px `--toolbar-bg` offset | |
| Selected and unfolded row | `--value-fill` ground, 3px `--accent` bar at the left; the bar continues down the unfolded body | as `0101` |
| Hairline between upper and lower part | 1px `--toolbar-icon` at 25 % (the divider of "Select bar layout"; `0101` named `--divider`, which `index.css` does not have) | non-text, informational |
| Entered group: outside at 35 % | `--context-dim` (row "Entered group") | |
| Entered group outline | 1.5px `--accent` with `--selection-casing` | 3.3:1 on `--canvas-bg` |
| Invalid field | 2px `--field-invalid`, `aria-invalid`, the validation chip | 4.8:1 |

Symbols (Lucide names at 16px, 1.5px absolute stroke, `currentColor`, unless the cell says custom):

| Control | Symbol |
|---|---|
| Cut, Score, Engrave, Engrave + Outline, Cut-out | custom, `OperationGlyph` (unchanged) |
| Operations view toggle | `ListTree` (the Operations tab's glyph in `PanelStrip.tsx`), 14px in a 20px button |
| Crosshatch | `Hash` |
| Bidirectional fill | `ArrowRightLeft` (two stacked arrows) |
| Read-only segment of the crumb | `Lock` 12px |
| Differs, Reset to material, Not found | `Diff`, `RotateCcw`, `Link2Off` (unchanged from `0028`) |
| Source line of a generated row (`0112`) | `SwatchBook` 12px |

## 3. Kind buttons in the Select bar (criteria 1 to 8)

### 3.1 Place and look

- A `role="group"` named "Operation" in the Select bar, directly after the settings group ("Scale stroke width", "Scale corner radius") and before Flip (P1, T1). Five `button`s, 28 x 28px (36 coarse), 4px apart, glyph only, in the order Cut, Score, Engrave, Engrave + Outline, Cut-out. Divider rules of "Select bar layout" (the 1px 20px divider 13px left of the next group).
- Shown only when a laser page is active and every selected object is in context (criterion 1). Absent otherwise: no selection, a drawing, sketch or "Laser (off)" page, an out-of-context object. Hidden, never disabled.
- The group (156px) wraps as a whole, by the rules of "Select bar layout": on a wide bar it sits on row 1 after the switches; in the smallest window (800 x 640, panel open, a 356px bar row, which the two switches fill) it starts row 2 and Flip follows it. Nothing inside the group breaks; the switches keep their place.
- Not a toggle: the buttons have no pressed state (the assignment is shown by the Operations tab ticks and the canvas look). Hover `--editor-accent-hover`; focus ring as the bar's.

### 3.2 Names, tooltip, keys

- Accessible name and tooltip line 1: the kind word, "Cut", "Score", "Engrave", "Engrave + Outline", "Cut-out" (criterion 2; the draft's "Engrave + outline" is spelled as `0101`).
- No tooltip line 2: Shift+press creating a new operation is a shortcut without tip text (criterion 2, M1).
- Accessible description (not the name): "Assigns to <name>" when a last operation of that kind exists, else "Creates <Kind> <N>", so a screen-reader user hears what a press does. Updated with the list.
- Keys: each button is its own Tab stop in bar order, as every Select-bar button (criterion 2). Enter or Space presses; Shift+Enter or Shift+Space is the Shift press. No letter key and no digit is bound to a kind (`0101` section 9: the letters are tool letters; the digits assign to operation n).
- Focus stays on the pressed button.

### 3.3 Press rules

| Situation | Result | Undo step |
|---|---|---|
| A last operation of the kind exists (lowest in the list, not generated by a test grid, not Unknown) | the whole selection is added to it; no row is created; that row is selected (not unfolded) and scrolled into view | "Assign to <name>" |
| None exists, or Shift held | "<Kind> <N>" is created at the end of its section, Show on, Output on, the selection assigned; the row selected and scrolled into view | "Assign to <name>" |
| Every selected object is already in that operation | nothing; no step, no notice | none |
| Selection mixes guides, dimensions or fixtures with work | the work is assigned; Action notice (section 10) | "Assign to <name>" |
| Only guides, dimensions or fixtures | nothing created; no step | none |

- "Selected" in the tab means the row ground of `0101` 4.4; a selected row that was unfolded stays unfolded; a different row selected by a press folds the unfolded one (one at a time, section 6).
- The Operations tab need not be open; nothing switches the panel's tab (T2).

## 4. Groups (criteria 9 to 14)

### 4.1 A group selected by one click

- As built: the group's box and handles, the Select bar with its groups. The kind buttons assign the group (criterion 9): one step "Assign to <name>".
- The Operations tab counts the group as one item. Tick per row (`0101` 6.2): checked when every leaf of the group runs in the operation, mixed (dash) when some do, unchecked when none.
- A mixed row adds to line 2, after the count: " · Group 3 in part" (the group's name; semibold `--toolbar-icon`, the rest of line 2 stays muted, as screen 01). Ellipsis cuts the end of line 2; the tooltip of line 2 and the row's accessible name carry it: "Engrave 1, Engrave, 2 items, Group 3 in part".
- Press on a mixed tick adds the rest ("Assign to <name>"); on a checked tick removes the group and its members from that operation ("Remove from <name>"). Members are never named in the tab (criterion 10).
- With several objects selected and a group among them, line 2 names the group only while exactly one group is in part; with two or more: " · 2 groups in part" (T3).

### 4.2 The entered group (read-only)

- **Enter:** double-click on a group on a laser page, or Enter with exactly one group selected and the canvas focused (the keyboard route of row "Entered group"). Not a history step.
- **Canvas:** everything outside at 35 % (`--context-dim`), outside objects inert (no hover box, not hit, not marqueed, as the row). The group's outline: 1.5px solid `--accent` with casing. No group handles. The pressed member: the 1px dashed `--accent` box, no handles, no knobs, no centre move.
- **Refused inside:** drag (the pointer press selects, the move does nothing, no marquee from a member), arrow nudge, Delete, Ctrl+D, Cut, Paste, the transform and edit keys (S, R, K, M, the Node tool's double-click). Silent: the read-only look is the cue (T4); no notice per refused key, as the placed page of `0106` refuses only its moves with a notice. A drag on empty space inside the group box draws a marquee over the members (Proposal of criterion 12).
- **Crumb** (the "Open groups" pill of `OpenGroups.tsx` in the canvas notice slot): "<page name>" (text button, makes the page the context, i.e. leaves) · `ChevronRight` 12px · "<group name>" (semibold, `aria-current="location"`) · then, muted, `Lock` 12px and "Read-only", then muted "Esc: leave". Words exactly: "Coasters › Group 3 · Read-only · Esc: leave" (criterion 11; the "·" separators are 12px muted, `aria-hidden`). The root crumb is the page name, not "Document" (change to the row, section 14 M3). Nested: one crumb per level, the `...` collapse of the row above three levels.
- **Select bar inside:** only the Operation group (P1, T1). The settings switches, Flip, Guide, Align, Distribute, Grid, kind groups, Object to path and Linked clone leave the tree (hidden, not disabled). With no member selected the bar is empty and not shown.
- **Operations tab inside:** the ticks speak for the selected member alone (criterion 12); the tab header's subject line reads "C in Group 3" (screen 01 B).
- **Leave:** Escape, a press on an object or on empty canvas outside the group, the page crumb. Escape with a member selected leaves at once and selects the group (criterion 13; one press, not the drawing page's "clear, then leave" of row "Entered group", T5). A press outside selects what it hits. Not a history step.
- Nested groups: a double-click on a group inside the entered group enters it, one level per double-click.
- Drawing pages: entering is as built (editable).

### 4.3 Clones and placed pages

- Clone rows in Layers have no chevron any more and no member rows (criterion 14). The clone row keeps its chips (`0101` 6.4). Inner rows of placed pages stay as built (Frage 213 A).
- A file with clone-member keys (Frage 212 A): nothing is shown, counted or noticed.

## 5. Operations view toggle (criteria 15, 16)

- One `button` per laser page in its canvas page label, after the Laser kind chip, 4px gap: 20 x 20px visual, `ListTree` 14px; hit area 24 x 24px (2px transparent padding on each side, WCAG 2.5.8). `rounded-[5px]`.
- Pressed (default): `--toolbar-icon-active-bg` fill, white glyph. Unpressed: transparent, `--toolbar-icon` glyph, 1px `--toolbar-icon` at 25 % border so it still reads as a button over the pasteboard. Hover unpressed `--editor-accent-hover`.
- Name and tooltip "Operations view", `aria-pressed` (constant name, pressed state). Tooltip `side="bottom"`, 400 ms.
- Keys: the label is DOM, so the button is a Tab stop in the page-label order of `PageFrames.tsx`; Space or Enter toggles. No shortcut (T6).
- State per laser page, per session; not saved, not a history step, no notice. Unpressed draws the drawing's own style, no chips, no Not assigned look (criterion 15).
- Narrow frames (row "Kind chip places"): the page name truncates first, the chip's word goes below 96px, the toggle stays as long as the label is drawn; below 24px the whole label goes and the toggle with it (its state is kept).
- The Operations tab loses the View radiogroup (criterion 16); "Cuts last" is the only control above the list (screen 03).

## 6. The unfolded row (criteria 17 to 24)

### 6.1 Fold and unfold

- A press on a row's glyph, name or line 2 selects it and unfolds its body directly under it. Any other unfolded row folds in the same frame (one at a time, Frage 197 A). A press on the unfolded row's own glyph, name or line 2 folds it and keeps it selected (T7). Enter or Space on the focused row does the same as the press. A double-click on the name renames (`0101` 4.4); the first press of the double-click has already unfolded the row, which stays unfolded.
- Folding is not a history step and is session state. A press on a Show, Output, trash, arrow or tick never unfolds or folds.
- Several rows selected (Ctrl or Shift press, `0028` M10): nothing unfolds; the multi-row material pick of `0028` 8.2 shows under the list, as built.
- The chevron at the right of line 1 in screen 03 is the Down arrow of the order buttons, not a disclosure. No disclosure glyph is added: the press on the row is the route (T7).
- The row keeps Show, Output, armed trash, arrows and rename in every state (criterion 17). Arming a trash of an unfolded row arms the row's line 1 and line 2 only; the body keeps its ground.
- Focus: Tab from the unfolded row enters its body (fields in visual order), then continues with the next row's list stop. Escape in a body field reverts the field; a second Escape returns focus to the row.
- Windowing: the unfolded row and its body are one window item of variable height; the item is measured once per unfold and per layout change, and is kept in the DOM while focused (`useScrollWindow`, `0101` section 12).
- The body is indented 16px (under the name, as screen 03) and carries the row's 3px `--accent` bar at the left down its whole height.

### 6.2 Upper part: the set (criteria 18, 19)

From top to bottom, 244px column less the 16px indent (228px), 4px between fields, 8px between groups:

1. Material pick (`MaterialPick`, inline list, `0028` 8.2, unchanged), 228 x 28px, `SwatchBook` 16px at the left of the value.
2. Power (112px, "%") and Passes (112px, "×") in one row.
3. Speed (228px, "mm/min").
4. Interval (124px, "mm", label inside; as built beside Passes, wrapping onto its own line where the two do not fit), Engrave and Engrave + Outline only (criterion 19): two decimals, 0.01 to 2.00, default 0.10. Engrave + Outline keeps its two sub-groups ("Hatch" and "Outline", `0028` 8.3); Interval sits in "Hatch".
5. State and "Differs" lines, "Material 60" under a typed field, "Reset to material", "Edit material", "No settings": all as built in `OperationSettingsBlock` (`0028` 8.3, 8.4). The draft's one-line "Differs from the material's set (30 %)" with a glyph button is not built; the built lines stay (criterion 18, section 14 M5).

In this slice Interval is the operation's own number: it gets no "Material" line and "Reset to material" leaves it (criterion 19).

### 6.3 Lower part: the operation-only settings (criteria 20, 21)

- A 1px hairline (section 2) 8px under the upper part, the lower part 8px under it.
- **Engrave, Engrave + Outline**, row 1: Crosshatch toggle (28 x 28, `Hash`), 4px, Bidirectional fill toggle (28 x 28, `ArrowRightLeft`), 4px, Scan angle field (164px, "°"). Row 2: Overscanning (128px, "%"); with a motorised Z axis, Z offset (128px, "mm") on row 3. (UX review 2026-11-12: with the label inside, "Overscanning 50 %" and "Z offset -20.0 mm" do not fit 112px, so the fields wrap instead of clipping.)
- **Cut, Score, Cut-out:** with a motorised Z axis, one row: Z offset (112px, "mm"). Without it the lower part and its hairline are absent (nothing to show).
- Toggles: the "Link corners toggle" look (pressed: active fill, white glyph; unpressed: transparent, ink glyph). Name and tooltip "Crosshatch" and "Bidirectional fill", `aria-pressed`. No caption (criterion 20). Defaults off and on.
- Fields: the bar number field (28px, label inside left 12px muted, unit inside right 12px). Labels "Scan angle", "Overscanning", "Z offset". Ranges and steps: Scan angle -90.0 to 90.0, one decimal, Up and Down by 1 (Shift 10); Overscanning 0 to 50, whole numbers (T8), default 2; Z offset -20.0 to 20.0, one decimal, Up and Down by 0.1 (Shift 1).
- Z offset is absent, not disabled, without the capability (criterion 21). A stored value stays and returns with the capability; nothing says so (T9).
- Profile capability (criterion 22): a row in the Machine block where the profile's other machine values are edited: label "Motorised Z axis", `PanelSwitch` at the right (`role="switch"`, no on/off text), default off. One undo step "Motorised Z axis on" / "Motorised Z axis off" on the page's profile copy (T10).

### 6.4 Commit and validation (criterion 24)

- Enter or leaving commits; Escape reverts; Up and Down step, committed on key-up. One undo step "Edit settings" per commit and per toggle press.
- Out of range: the typed text stays, 2px `--field-invalid`, `aria-invalid`, the validation chip 4px under the field (`aria-describedby`); nothing is written. Texts in section 10.
- A material pick, Use material, Reset to material, a refresh or a material edit never changes Interval or the lower part (criterion 23).

## 7. Generated rows (from `0112`) in this slice

- A test grid's rows are ordinary rows (screen 03 A). A kind button never assigns to them (criterion 6). Their unfolded body is described in `specs/0112-test-grid-and-materials/ux-notes.md` section 6.

## 8. Keyboard routes

| Action | Route |
|---|---|
| Kind button | Tab through the Select bar, Enter or Space; Shift+Enter or Shift+Space for a new operation |
| Assign to operation n | digits 1 to 9 on the canvas (`0101` section 9, unchanged) |
| Enter a group | double-click, or Enter with one group selected and the canvas focused |
| Leave a group | Escape (one press), the page crumb |
| Operations view | Tab to the toggle in the page label, Space or Enter |
| Unfold or fold a row | Enter or Space on the row (list keys of `0101` section 9) |
| Into the body | Tab from the unfolded row; Escape twice back to the row |
| Toggles in the body | Space or Enter |
| Fields | Enter commits, Escape reverts, Up and Down step (Shift 10 or the steps of 6.3) |
| Motorised Z axis | Tab to the switch, Space |

## 9. Undo steps

"Assign to <name>" (kind button, group, mixed tick), "Remove from <name>", "Edit settings" (every field commit and toggle of the unfolded row), "Use material", "Use own numbers", "Reset to material" (unchanged), "Motorised Z axis on" / "Motorised Z axis off". No step for: entering or leaving a group, the Operations view toggle, unfolding or folding, a row or canvas selection, a press that changes nothing.

## 10. Notices and status (each at most 70 characters)

| Text | Where | Role, time |
|---|---|---|
| "2 guides not assigned. Guides never run." (also "1 guide", and "dimensions", "fixtures" by the same pattern; mixed: "3 items not assigned. They never run.") | canvas notice slot | `role="status"`, 5 s |
| "Guides never run." (only guides selected; only dimensions or fixtures: silent) | canvas notice slot, refusal look | `role="alert"`, 8 s |
| "Scan angle is -90.0 to 90.0°." | validation chip | with `aria-invalid` |
| "Overscanning is 0 to 50 %." | validation chip | |
| "Z offset is -20.0 to 20.0 mm." | validation chip | |
| "Interval is 0.01 to 2.00 mm." | validation chip | |
| "Enter a number." / "Enter a whole number." | validation chip | |
| "Cut 3 added." / "Assigned to Engrave 1." | the tab's sr-only polite status region, after a kind press | polite, nothing visible |

No visible notice after a kind press, a toggle, an unfold, an entry or a leave (no success notice).

## 11. Accessible names

| Element | Name | State or description |
|---|---|---|
| Operation group in the bar | "Operation" (group) | |
| Kind button | "Cut", "Score", "Engrave", "Engrave + Outline", "Cut-out" | description "Assigns to <name>" or "Creates <Kind> <N>" |
| Operations view | "Operations view" | `aria-pressed` |
| Crumb | `nav` "Open groups"; the page crumb a button "<page name>"; the group `aria-current="location"`; "Read-only" text | |
| Unfolded row | the row (`listitem`) gets `aria-expanded="true"`; folded rows `aria-expanded="false"`; unknown operations none | |
| Body | `role="group"` "Settings of <name>"; description the state line (`0028` 8.4) | |
| Crosshatch, Bidirectional fill | "Crosshatch", "Bidirectional fill" | `aria-pressed` |
| Fields | "Interval, millimetres", "Scan angle, degrees", "Overscanning, percent", "Z offset, millimetres" | `aria-invalid` with the chip |
| Mixed tick | "<name>" (checkbox) | `aria-checked="mixed"`, description "Group 3 in part" |
| Motorised Z axis | "Motorised Z axis" (switch) | `aria-checked` |

## 12. Empty, error, locked, mixed and large states

- **No selection or not a laser page:** no Operation group in the bar.
- **No operation on the page:** the list line "No operations on this page." (`0101`); the summary as built. The first kind press creates the first row.
- **Only a test-grid or Unknown operation of a kind:** the press creates a new operation (criterion 6).
- **Locked objects** (Layers lock): a locked object can be selected through Layers; the kind buttons assign it (an assignment is not an edit of the object, T11).
- **Mixed:** the mixed tick and line 2 of section 4.1; a multi-row selection shows only the pick (`0028` 8.2).
- **Errors:** invalid values (6.4); a material not found or without a set (`0028` 8.4 states unchanged).
- **Damaged settings** (criterion 25): the state of `0028` criterion 28 as built.
- **Large:** kind press on 1,000 items, a group of 1,000 leaves, entering it and unfolding a row on a page with 50 operations each within 16 ms thread CPU (criterion 26); the unfolded row is one window item; the canvas look keeps the 500-chip cap of `0101` 13.4.

## 13. Decisions for veto (Proposal)

- **T1** Operation group right after the settings switches; inside an entered group the bar holds only it (P1).
- **T2** A kind press never switches the panel's tab.
- **T3** Two or more groups in part: " · 2 groups in part".
- **T4** Refused edits inside an entered group are silent; the dimming, the crumb's "Read-only" and the missing handles are the cue.
- **T5** Escape inside a laser group leaves in one press with the group selected (criterion 13), unlike the drawing page.
- **T6** No shortcut for the Operations view.
- **T7** The row press is the fold control; no chevron is added.
- **T8** Overscanning in whole percent.
- **T9** A Z offset hidden by the capability is kept without a sign.
- **T10** "Motorised Z axis" is a `PanelSwitch` row in the Machine block, one undo step on the project's profile copy.
- **T11** Locked objects take an assignment.
- **T12** Kind buttons are not toggles and show no pressed state.

## 14. Mismatches for the product owner and design-system changes

- **M1** Resolved: the tooltip line "Shift: new operation" is dropped (no operating tips); the Shift press stays as a shortcut with no tip text.
- **M2** Criterion 11 says "Esc: leave" in the crumb; the design system already shows it muted. Same text, no change.
- **M3** Row "Entered group" of the design system: on a laser page the root crumb is the page name, a "Read-only" segment with `Lock` follows, Escape leaves in one press, the Select bar shows only the Operation group (the "settings always shown" rule of "Select bar layout" has this one exception). Rows to amend at acceptance.
- **M4** Criterion 1 says the buttons sit "right after the settings switches", criterion 11 hides the switches inside a group; both kept (M3).
- **M5** Screen 03 B condenses the Differs line ("Differs from the material's set (30 %)" with a reset glyph); criterion 18 keeps `0028` unchanged, so the built lines and the "Reset to material" text button stay.
- **M6** Resolved: criterion 20 now says whole numbers (T8).
- **M7** Resolved: criterion 22 now names the undo step "Motorised Z axis on/off" (T10).
- **M8** `0101` ux-notes name a `--divider` token that `index.css` lacks; the hairline uses the bar divider (`--toolbar-icon` at 25 %).
- Design system rows to add at acceptance: "Operation group (`0111`)" in the Select bar, "Operations view toggle (`0111`)" in "Kind chip places", "Unfolded operation row (`0111`)"; remove "View switch" from the Operations tab rows. No token.

## 15. Implementer checklist

- Bar: Operation group after the switches, five 28px glyph buttons, hidden (not disabled) off laser pages and for out-of-context selections; tooltip is the kind word only; descriptions; Shift+Enter.
- Press rules of 3.3: reuse lowest non-generated, non-Unknown operation; create at section end otherwise; Shift always creates; no step when nothing changes; skip notices.
- Groups: group key plus member nodes with own assignments (criterion 9); mixed tick and " · Group 3 in part"; unticking removes group and member keys of that operation.
- Entered group: dim 35 %, accent outline, dashed member box, no handles, refused edits silent, crumb "<page> › <group> · Read-only · Esc: leave", bar with the five buttons only, Escape leaves in one press.
- Layers: clone rows without chevron or member rows; inner rows unchanged; old clone-member keys ignored silently.
- Page label: 20px `ListTree` toggle, 24px hit area, pressed by default, per page, session only, no step; the View radiogroup gone from the tab.
- Unfolded row: press unfolds, one at a time, press again folds; body indented 16px with the accent bar; pick, Power+Passes, Speed, Interval (Engrave), built pointing lines; hairline; toggles and fields of 6.3; Z offset only with the capability.
- Validation chips and texts of section 10; one "Edit settings" per commit or toggle; material actions never touch Interval or the lower part.
- Machine block: "Motorised Z axis" switch, one step.
- Windowing with one tall item; focus never to `body`; no new colour, no popup, no operating tip, no success notice.

## Links

Spec: `specs/0111-laser-operation-tools/specification.md`. Drafts: `docs/design-drafts/2026-11-laser-redesign/` screens 01 and 03, README P1, P2, P10, P11. Built notes: `specs/0101-laser-operations/ux-notes.md`, `specs/0028-material-test-library/ux-notes.md` section 8, `specs/0106-move-versus-edit/ux-notes.md`. Next: `specs/0112-test-grid-and-materials/ux-notes.md`, `specs/0113-fixture-sets-and-pins/ux-notes.md`.
