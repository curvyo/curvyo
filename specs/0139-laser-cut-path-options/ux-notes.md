# 0139 Laser cut path options: UX notes

ux-engineer, 2026-11-26. Written from `specification.md` (revision 2026-11-26, criteria 1 to 25, Fragen 279 A, 280 A, 281 A, 414 default A), `docs/design-system.md` ("Interaction conventions": no operating tips, no success notice, hidden not disabled, no popups in the panel, "Numeric entry chip", "Parameter handles", "Blue new, black old", "Canvas notice slot"; rows "Parameter handle", "Position handle", "Transform entry chip", "Live transform readout", "Select bar layout", "Override rows (`0057`)", "Bend a segment" for Alt on Linux) and the built notes `specs/0111-laser-operation-tools/ux-notes.md` (unfolded row, lower part, Operation group), `specs/0101-laser-operations/ux-notes.md` 13 (canvas look, `--cut`, budget) and `specs/0029-laser-job-preview-and-output/ux-notes.md` 4 and 6.4 (Toolpath look, Checks). Code read: `OperationBody.tsx`, `OperationOptionsPart.tsx`, `lib/operationOptionRows.ts`, `OperationKindGroup.tsx`, `CloneBlock.tsx` (`OverrideRow`). Customer rules applied: no invented colour (red `--cut` and ink only on the laser look, `--accent` for what is selected), symbols with names as tooltip, units inside fields, plain fields and no sliders, no popup in the panel, hidden not disabled, no operating tip and no success notice. Items marked (Proposal) are vetoable at the demo; section 14 lists them as C1 to C14.

Terminology: "unfolded row", "lower part" and "ink" (`--toolbar-icon`) as in `0111`. "Item part" is the new block for the selected items in an unfolded row. "Handle" is a tab handle or a start marker on the canvas. "Current handle" is the one handle that keys act on.

## Built state, slice 1 (reviewed 2026-11-27)

Slices 1 (tabs) and 2 (path options) are built. Where the notes below and the two built-state blocks differ, the blocks are the built truth. The slice 1 review ran in Chromium at 1440 x 900, 1920 x 1080 and 800 x 640 on a laser page with a rectangle, a circle and a plate with a hole in one Cut operation; the slice 2 review (block at the end of this section) added an open line.

Built as written: the Tabs group of the unfolded Cut and Cut-out row (heading with glyph, Count 96 px, Width 128 px, placed line, Skip inner shapes switch; absent on every other kind); ranges, steps and chip texts of section 2 and 10; the Tabs toggle in the Operation group (glyph, `aria-pressed`, hidden without a Cut item selected and in the Toolpath view); the item part's Tabs pill "Operation's | Own" with readout ("4 tabs", "4 to 8 tabs", "Own tabs (5)"); tab handles on the selected item, drag along the path with node snap and the 1 mm stop (readout "42.30 mm", " · limit"); Delete on a current tab (the object stays, the next tab becomes current); the Operations view gap and ink bar; the Toolpath view as moves with arrowheads at every piece start and no handle or toggle; undo names "Edit settings", "Add tab", "Move tab", "Delete tab", "Tab width", "Own tabs", "Back to the operation's"; no hint or tip text.

Built differently (kept for the demo, vetoable):

- **Handle look:** the round "Parameter handle" knob, not the capsule of 5.1 (C13 drops to the start marker only). It reads as a knob next to the transform handles and is the one silhouette that does not tell tab from parameter; revisit with the capsule when slice 2 adds the start triangle.
- **Width of the current tab** is the item part's field "Width" (128 px, "mm", name "Tab width, millimetres") under the pill readout, with the "Operation's width" reset button after it while the tab has an own width, instead of the canvas chip of 5.1. The message chip hangs from the field's left edge and the part keeps 28 px below the field while the field shows. Double-click and Enter on a handle do nothing. Refusal texts as in section 10.
- **Add:** with the Tabs toggle on, a press within 6 px of a selected cut path adds a tab at the press (snapped to a node within 8 px). No ghost handle, no crosshair cursor, no 3 px dead zone, no drag-to-move-the-object from that spot.
- **Keys:** Tab, arrows and Insert do not act on handles (C9 not built); Delete and Escape do. Escape: clears the current handle, else turns the toggle off, else the canvas cascade. No screen-reader announcements of the current handle.
- **Which handles:** the item's first Cut or Cut-out in run order, not the unfolded row's operation (C2 half built; the item part of the unfolded row still edits that operation's records).
- **No heading** in the item part (3.1): the group is named "Selected items in <operation>" for assistive tech only.
- **Copies** (Duplicate, Ctrl copy, Paste) keep no tab records.
- **Tabs toggle** stays pressed when the Toolpath view is pressed and released again (4.2 says the Toolpath view turns it off).
- **Start markers** were not drawn in slice 1 (slice 2 draws them, see its block); a start may not sit inside a tab (spec reconciliation, ADR issue 3), which overrides 5.2.

Open findings at review time (lead decides): a plain click on a tab handle moves the tab to the pointer and writes "Move tab" and makes the tabs Own; Escape during a tab drag does not cancel it (the press made the tab current, so `tabs_escape` runs before `cancel_drag_in_flight`); the Count field has no `aria-describedby` for the placed line; the operation's Width and the item part's Width share one accessible name.

### Built state, slice 2 (reviewed 2026-11-27)

Reviewed in Chromium at 1440 x 900 (zoomed), 800 x 640 and 1920 x 1080 on a laser page with a rectangle, a circle, a plate with a hole and an open line in one Cut operation.

Built as written: the Path options group of the unfolded Cut and Cut-out row (sub-heading with the Lead glyph, Lead-in and Lead-out rows with the None | Line | Arc pill and the 84 px length field that is absent on None and starts at 1 mm, Overcut 128 px; Up/Down step 0.1; Escape reverts; "Lead-in is 0.10 to 20.00 mm." chip with `aria-invalid`; one step "Edit settings" per commit and per segment; absent on every other kind); the item part rows (Path options pill "Operation's | Own" with the summary line "Lead-in Arc 1 mm · Lead-out None · Overcut 0.5 mm" or the three own rows, "Own" copies the operation's values; Start readout "Automatic", "Set", "Set on 1 of 2 outlines" with the button "Automatic start" while any is set; Direction pill only when an open path is selected, Automatic by default, one Tab stop, arrows set and wrap); undo names "Own path options", "Back to the operation's", "Move start point", "Automatic start", "Direction"; start markers (set: solid, drawn whenever the item is selected; Automatic: hollow, only while the Tabs toggle is on, resting where the job enters, a rectangle's bottom-left corner; one per closed outline, a plate with a hole has two); drag along the path with node snap (readout "Start · 40.06 mm · node"), Alt suspends the snap, release is one step, Escape during the drag cancels without a step, a click without a drag writes nothing, Delete on a current set marker is "Automatic start"; the marker cannot sit inside a tab (it stops at the tab's edge, readout flat, and hops to the far edge once the pointer passes the tab's centre); the marker takes the press before the corner scale handle that sits on the same point; the Operations view marks (lead dashes with the arc and line shapes, overcut over the red line, ink chevron at a set start and at the first end of a fixed direction, nothing for Automatic); the Normal view (nothing but handles and markers); the Toolpath view (leads as burn pieces with arrowheads, no marker, no Tabs toggle); the warn line "1 lead crosses or touches a cut." with the selecting text button "1 lead" (it selected the plate); a lead that leaves the bed adds "1 item lies outside the 400 × 300 mm bed." as a block and disables Export job file (the same text as for an item outside the bed; without the lead the block is absent); no hint or tip text.

Built differently (kept for the demo, vetoable):

- **Direction** sits under the muted label "Direction" as a full-width three-segment pill (not a 56 px label with a pill beside it); "Automatic" is pressed when nothing is fixed.
- **Start marker** is a plain triangle drawn from triangles in the session's draw list; its hover look is its resting look (no render-core glyph). At a bounding-box corner (a rectangle's Automatic start) it sits under the white corner scale handle: the press still reaches the marker, but the triangle is hard to see there.
- **Which markers:** the item's first Cut or Cut-out in run order, as slice 1's handles; the item part edits the unfolded row's operation.
- **Copies** (Duplicate, Ctrl copy, Paste) keep no tab, own path option, start or direction records. Spec criteria 11 and 22 say copies keep them.
- **Keys:** Tab, arrows and Insert do not act on markers (C9 half built); Delete and Escape do. No screen-reader announcements.
- **Summary line** of the Operation's pill wraps at 228 px ("... Overcut 0 / mm").

Open findings at review time (lead decides):

1. **Blocker, page freeze:** a Lead-in of 5 mm Line on a rectangle, 4.5 mm on a circle, and 8 or 12 mm lead the render loop into `Session::operation_looked_artwork` calling `fmod` and never returning (a paused stack was read through the browser's debugger; 1, 3 and 4 mm and the job build in `curvyo-laser-core` are fine). The loop is the dash walker `dashed` in `curvyo-editor-wasm/src/session/operations/tab_look.rs`: when `(walked + t) % period` lands a hair under `on`, `t + on - phase` equals `t` in floating point and `t` never advances.
2. **Item part below the fold, press jumps the list:** the unfolded row's list is capped at 520 px at every window size (1920 x 1080 included), so the item part is always out of sight until the maker scrolls the nested list; every press in it returns the focus to the row and sets `scrollTop` from 127 back to 0 (the part leaves the view after each click). With the own rows shown, the list cannot be scrolled to the end: the last 16 to 40 px stay hidden ("Overcut" half cut at 1440 x 900, the Direction pill cut at 800 x 640, "Start" unreachable).
3. **Arc lead-in in a hole:** Lead-in Arc 4 mm raises "1 lead crosses or touches a cut." (the plate with a hole is selected by the button); Line 4 mm and an overcut alone do not. The arc hugs the hole's wall for about 1.5 mm before it leaves it. Whether this is the 0.01 mm rule of criterion 18 is for the tester to confirm.
4. After a click on the Tabs toggle the focus stays on the toggle, and Escape does not turn it off (it needs the canvas focus).

## 0. What is reused

| Part | Reused from (code) | Change in this slice |
|---|---|---|
| Lower part of the unfolded row | `OperationOptionsPart.tsx`, `lowerPart()` in `operationOptionRows.ts` | Cut and Cut-out get the Tabs and Path options groups (section 2); the part is no longer empty without a Z axis |
| Fields | `EntryField` through `OptionFieldView` (inside label, unit, Up/Down step, chip) | new `OptionField` keys; whole-number Count |
| Sub-heading | `SetHeading` look of `OperationSettingsBlock.tsx` (24px row, 12px semibold ink, 14px glyph) | headings "Tabs" and "Path options" |
| Switch row | `PanelSwitch` (the "Motorised Z axis" row of `0111` 6.3) | "Skip inner shapes" |
| Override pill | `OverrideRow` (`CloneBlock.tsx`, `0057`) | rows "Tabs" and "Path options" of the item part; a three-segment variant for Direction |
| Operation group | `OperationKindGroup.tsx` | a sixth control, the Tabs toggle (section 4) |
| Handles, chip, readout | "Parameter handle" behaviour, "Transform entry chip", "Live transform readout", the slide-along-a-carrier rule of "Position handle" | new silhouettes (section 5) |
| Checks | `OutputChecks.tsx` (`0029` 6.4) | two warn lines (section 10) |

## 1. Principle

The operation says how every part is held and entered; the canvas shows exactly where, and the maker moves it there. The operation's numbers sit in the unfolded row with the other operation-only settings. The item's own choices sit under them in the same body, so "the operation's" and "this item's" are read in one place and the way back is one press. On the canvas a selected item shows its tabs and its start as handles that slide along the path; nothing opens a dialog. The Toolpath view stays a read-only picture of the moves.

## 2. The operation's options in the unfolded row (criteria 1, 2, 3, 13)

Only on Cut and Cut-out rows. Score, Engrave, Engrave + Outline, Image, Unknown and damaged rows have neither group (hidden, not disabled). Order in the lower part (228px column, 4px between fields, 8px between groups), under the existing hairline:

1. **Z offset** row, unchanged (`0111` 6.3), only with a motorised Z axis.
2. **"Tabs"** sub-heading with the Tabs glyph (section 4.1) at 14px.
   - Row: **Count** (96px, label "Count" inside, no unit; whole number 0 to 20; Up/Down 1, Shift 5) and **Width** (128px, "Width", "mm"; 0.10 to 20.00, two decimals; Up/Down 0.1, Shift 1). Defaults 0 and 0.50.
   - **Placed line** (C1), 12px `--panel-muted-fg`, only while Count is above 0 and at least one path of the operation gets fewer than Count: "Placed: 3 per path" when every short path gets the same number, else "Placed: 2 to 4 per path"; "Placed: none" when every path is too short. It is a readout, no control, not a Tab stop; it is the field's `aria-describedby`.
   - **Skip inner shapes**: `PanelSwitch` row, 228 x 28px, label left 12px ink, switch right, default off.
3. **"Path options"** sub-heading (Lead glyph, section 4.1, 14px).
   - **Lead-in** row: visible label "Lead-in" (12px ink, 56px box), a `role="radiogroup"` "Lead-in" of three 28 x 28px icon segments None, Line, Arc (84px, the Offset chip's pill look: 1.5px ink ring, active segment `--toolbar-icon-active-bg` / `-fg`), 4px, then the length field (84px, no inside label, "mm"; 0.10 to 20.00; Up/Down 0.1, Shift 1; default 1.00). With None the length field is absent; the pill keeps its place, nothing below moves.
   - **Lead-out** row: the same, label "Lead-out", glyphs mirrored.
   - **Overcut**: field 128px, "Overcut", "mm"; 0.00 to 10.00; Up/Down 0.1, Shift 1; default 0.00.

Commit, revert and refusal exactly as `0111` 6.4: Enter or leaving commits, Escape reverts, one undo step "Edit settings" per commit, per segment press and per switch; out-of-range text stays with 2px `--field-invalid`, `aria-invalid` and the chip of section 10, nothing is written. No caption, no explanation of what a tab or a lead is.

## 3. The item part (criteria 5, 19, 20, 21)

### 3.1 Where and when

- In the unfolded body of a Cut or Cut-out row, after the lower part, under a second hairline. Shown only while the canvas selection holds at least one item that runs in this operation (by the nearest rule); absent otherwise. A canvas selection never unfolds a row and never switches the panel's tab (as `0111` T2).
- Heading (the sub-heading look, no glyph): the item's name as Layers shows it ("Path 3", "Group 2", cut at 28 characters with the tooltip whole); several: "3 selected items".
- The part speaks for the selected items in **this** operation (C2, spec issue 5). Start point and direction are per item, so they show the same values in every operation's body.

### 3.2 Rows, top to bottom

1. **Tabs**: `OverrideRow`, label "Tabs", segments "Operation's" | "Own" (244 becomes 228px wide here). Under it a 12px muted readout: "4 tabs" (one item, the placed number); own: "Own tabs (3)"; several: "4 to 6 tabs", "Own tabs on 2 of 3 items". Mixed sources: no segment pressed, visually hidden "Mixed" (the `0057` rule).
   - "Own" copies what the item shows now (positions and the operation's width): the output does not change. Step "Own tabs" (C3, spec issue 4).
   - "Operation's" drops the own tabs and own widths: step "Back to the operation's".
   - A handle edit on an item with the operation's tabs makes them own in the same step (the edit's name, e.g. "Move tab"); the pill then shows Own.
2. **Path options**: `OverrideRow`, label "Path options", "Operation's" | "Own". With Operation's: one muted 12px line naming the operation's values, wrapping: "Lead-in Arc 1 mm · Lead-out None · Overcut 0.5 mm". With Own: the three rows of section 2.3 for the item (same sizes, same fields), the line gone. "Own" copies the operation's values: step "Own path options"; "Operation's": step "Back to the operation's". Edits of the item's own fields: "Edit settings". Shown only when a selected item has a closed path (leads and overcut apply to closed paths only).
3. **Start**: shown when a selected item has a closed outline. Label "Start" (12px ink, 56px box) and a readout: "Automatic" (muted); one outline set: "Set" (ink); several outlines: "Set on 2 of 3 outlines". While any is set, a text button **Automatic** (outline-button look of "Edit material", 28px, right-aligned; accessible name "Automatic start") returns every selected outline to Automatic, one step "Automatic start". While none is set the button is absent.
4. **Direction** (built as a muted label over a full-width pill, see the slice 2 block): shown when a selected item has an open path (Cut only; a Cut-out contour is closed). `OverrideRow` look with three segments "Automatic" | "As drawn" | "Reversed", 12px text, label "Direction", `role="radiogroup"`, one Tab stop, arrows set. One step "Direction". Mixed: none pressed.

## 4. The Tabs tool in the Select bar (criterion 5)

### 4.1 Glyphs (custom, 16px, 1.5px, in `OperationGlyphs.tsx`)

- **Tabs**: a horizontal line in `--cut` from x 2 to 6 and from 10 to 14 (a 4px gap), an ink bar across the gap at x 8 from y 5 to 11. It is the canvas tab look in small.
- **Lead None / Line / Arc** (ink, `currentColor`): a horizontal edge at y 6 from x 2 to 14; Line adds a vertical stub from (5, 13) up to the edge; Arc a quarter circle from (2, 13) to (6, 6) meeting the edge tangentially. Lead-out glyphs are mirrored (stub at the right). Names and tooltips "None", "Line", "Arc".

### 4.2 The toggle

- In the Operation group of the Select bar, after Cut-out, 4px gap: one 28 x 28px `Toggle` (36 coarse), the pressed look of the lower part's toggles (`--toolbar-icon-active-bg` fill, white glyph; the red half of the glyph turns white too). Name and tooltip "Tabs", `aria-pressed`; no tooltip line 2.
- Shown only while the selection holds at least one item that runs in a Cut or Cut-out operation and the page's Toolpath view is not pressed. Absent otherwise (hidden, not disabled). Inside an entered laser group it is part of the one group the bar shows (`0111` M3).
- On by a press (Space or Enter on the focused toggle). Off by a press, by Escape on the canvas when no handle is current (section 7), by a tool switch, by pressing the Toolpath view, and when the selection no longer holds such an item. Session state; no undo step, no notice.

### 4.3 While Tabs is on

- **Add:** the pointer within 6 screen px of a selected item's cut path shows a ghost tab handle (hover ground, section 5.1) at the nearest point, snapped to a node within 8 px; Alt held suspends the snap. A click (press and release within the 3px dead zone) adds one tab there: step "Add tab". Where the 1 mm rule leaves no room, the ghost is not drawn and the click changes nothing; no notice (the missing ghost is the feedback). A drag from there moves the object as the Select tool does.
- Cursor over an addable point: built-in `crosshair`; elsewhere the Select tool's cursors.
- A click on anything else does what the Select tool does (select, marquee, deselect).
- **Automatic start markers** show (section 5.2, C4 / Frage 415).
- The arrow keys, Tab and Insert act on handles (section 7) instead of nudging and z-order.

## 5. Handles on the canvas (criteria 5, 6, 19)

Drawn in the WebGL draw list, screen-constant, never exported, `aria-hidden`; only for selected items that run in a Cut or Cut-out operation; never in the Toolpath view (criterion 8 of `0029`) and never during a move, resize, rotate or skew drag of the same item (the "Parameter handle layout" rule). Which operation's tabs are shown when an item runs in two (C2): the operation of the unfolded row, else the first Cut or Cut-out in run order.

### 5.1 Tab handle

- A capsule 12 x 6px (3px radius) centred on the tab's centre, its long side along the path's tangent there (it turns with the path; nothing else on the canvas is a capsule). Ground white, ring 1.5px `--accent`; hover `--accent-hover` ground; current, dragged or chip open: solid `--accent`. Hit radius 12px; cursor `pointer`. Drawn in the parameter-handle layer, above the path and the selection box; in a tie with another handle the nearest centre wins.
- Not drawn while the path's screen length is under 48px (four capsules would touch); zoom in to edit. At most 50 selected leaves in the viewport carry handles, selected first in z order (C5); the rest keep their tabs, unchanged.
- **Drag:** the tab's centre follows the nearest point of the path to the pointer (the "Position handle" rule), along the path, wrapping on a closed path. Snaps to a node within 8 screen px; Alt held during the drag suspends the snap, read on every pointer move (Alt at the press is not relied on: Linux window managers take it, row "Bend a segment"). At the 1 mm rule (criterion 5) the handle stops and lags the pointer, the visible rule of a handle at a limit. Blue new, black old: the committed capsule stays drawn, the dragged one is the solid one. Release writes one step "Move tab"; Escape cancels.
- **Readout** ("Live transform readout", 12px up-right of the pointer): position along the path from its first stored point, "42.30 mm"; snapped " · node"; at the limit " · limit".
- **Current handle:** a press on a handle makes it current (solid), in or out of Tabs mode. It stays current until Escape, a press elsewhere, a tool switch or a selection change. Delete or Backspace with a current tab removes it ("Delete tab") and never deletes the object; focus stays on the canvas, the next tab along the path becomes current.
- **Width chip** (criterion 6): a double-click on a tab handle, or Enter with a current tab, opens a "Transform entry chip": one field "Width", 100px, "mm", prefilled with the effective width, text selected, accessible name "Tab width, millimetres"; while the tab has an own width, a 28px `RotateCcw` button "Operation's width" follows the field (`tabindex="-1"`, name and tooltip "Operation's width"), which returns the tab to the operation's width. Placement 10px clear of the capsule, outward from the path on the waste side, upright, clamped. Enter commits one step "Tab width" (also the reset); Escape and the other ends of "Numeric entry chip" write nothing. Refused: "Width is 0.10 to 20.00 mm.", the chip stays open. A width that would break the 1 mm rule is refused with "No room for this width." (C6).

### 5.2 Start marker (built, see the slice 2 block for the differences)

- One per closed outline of a selected item (a compound path or a group: one each). An isosceles triangle 10 x 8px whose base centre sits on the start point and whose tip points along the cut direction (it turns with the path); 1.5px `--accent` ring. **Set:** solid `--accent`, shown whenever the item is selected. **Automatic:** white ground, shown only while Tabs is on (Frage 415 default), resting where the job really enters the outline (C7, spec issue 1). Hover `--accent-hover`. Hit radius 12px; cursor `pointer`. A triangle is the one handle that shows a direction; parameter knobs are round, tab handles capsules.
- **Drag:** as the tab handle (nearest point, wrapping, node snap 8px, Alt suspends). Readout "Start · 12.50 mm" (" · node"). Release: step "Move start point"; from then the outline is Set. Escape cancels. The marker cannot be dropped inside a tab (it stops at the tab's edge); a tab added later over it moves the burn start to that tab's end (spec criterion 19, ADR issue 3).
- Delete or Backspace on a current set marker returns that outline to Automatic, step "Automatic start" (C8). A double-click does nothing.
- Open paths have no marker; their direction is the item part's Direction pick, shown on the canvas by the Operations view's arrowhead (section 6).

## 6. What the views draw (criteria 7, 8, 14, 15, 17)

### 6.1 Operations view (pressed, `0111` 5; two colours only)

- **Tab:** the red cut line stops at the exact gap ends (document units), and an ink bar 1.5px wide, 8px long crosses the path at the gap's centre, perpendicular, over the 1px white casing. The bar keeps its 8px at every zoom, so a 0.5 mm gap is visible when it is under a pixel. Cut-out: the dashed 2px line stops at the gap the same way.
- **Lead-in, lead-out:** the burn line's look thinned: 1px `--cut` dashed 3 on 2, with casing, exactly where the output burns them.
- **Overcut:** ink dashed 1.5px, 3 on 2, over the red line along its exact stretch.
- **Start:** a set start is an open 5px ink chevron at the start point pointing along the cut (the `0029` arrowhead silhouette, in ink), with its white casing. An Automatic start draws nothing here (the Toolpath view shows the job's entry). A fixed Direction of an open path draws the same chevron at its first end; Automatic draws none.
- Positions of leads, overcut and the burn start come from the job's moves, so an Automatic path shows its lead where the job enters it (spec issue 2).
- **Not drawn** on Score, Engrave, Engrave + Outline, Image, fixtures, test grids and registration marks; Output off and Not assigned items keep their `0101` look with no tab, lead or start marks.
- Budget: marks follow the `0101` 13.4 scope (leaves meeting the viewport); bars and chevrons count toward no chip cap; a path under 12 screen px draws no chevron.

### 6.2 Normal view (Operations view not pressed)

- The drawing's own style; no tab bar, lead, overcut or chevron. Only a selected item's handles (section 5).

### 6.3 Toolpath view (read-only)

- Exactly the moves (`0029` 4): a gap is the travel look, 1px `--lane-line` dashed 4 on 3, between two pieces; leads and overcut are burn lines in the kind look (Cut 1px solid `--cut`, Cut-out 2px dashed 6 on 3); every burn piece starts with the arrowhead (a path with 4 tabs: 4 arrowheads; the first piece's arrowhead sits at the start of its lead-in); the numbered ring stays at the run's start. No handle, marker, capsule or Tabs toggle. The selection box and the transform handles still draw (`0029` 3).
- The play bar's time and the row times follow the moves; nothing else changes in the bar.

## 7. Keyboard and focus

| Action | Route |
|---|---|
| Fields, segments, switch in the body | `0111` 8: Tab in visual order, Enter commits, Escape reverts, a second Escape back to the row; Up/Down step (section 2); radiogroups one stop, arrows set |
| Tabs toggle | Tab through the Select bar, Space or Enter |
| Current handle (Tabs on, canvas focused) | Tab / Shift+Tab go to the next / previous handle of the selected items (tabs and start markers in path order, outline by outline, item by item in z order); past the last or before the first, Tab leaves the canvas as usual (no trap) (C9) |
| Slide the current handle | Right or Up forward along the path, Left or Down back: 1 mm, Shift 10 mm (the Nudge numbers); node snap does not apply; each key-up is one step ("Move tab", "Move start point") |
| Add a tab | Insert: one tab at the middle of the longest burnt piece of the current handle's path (or the first selected cut path); nothing where there is no room |
| Delete | Delete or Backspace on a current tab or set marker (section 5) |
| Width chip | Enter with a current tab (placed as if double-clicked) |
| Escape | clear the current handle; else turn Tabs off; else the canvas Escape cascade |

- Out of Tabs mode the arrows nudge, Page Up and Down reorder and Tab leaves the canvas, as today; a pointer-made current handle still takes Delete and Enter.
- Screen readers: the canvas is not in the accessibility tree. The Operations tab's sr-only polite region (`0111` 10) says the current handle on each Tab and step: "Tab 2 of 4, 0.50 mm wide, at 25.00 mm", "Start point, set, at 12.50 mm", "Start point, automatic"; and "Tab added." / "Tab deleted.". Nothing visible.
- Focus never goes to `body`: a removed handle hands current to the next; the chip returns focus to the canvas; a body control that disappears (lead length on None, the Automatic button) hands focus to the control before it.

## 8. Undo steps

"Edit settings" (every field, segment and switch of the operation's groups and of own path options), "Add tab", "Move tab", "Delete tab", "Tab width", "Own tabs", "Own path options", "Back to the operation's", "Move start point", "Automatic start", "Direction". A handle edit on the operation's tabs makes them own inside the same step. No step for: the Tabs toggle, a current handle, a hover ghost, a chip opened and closed without a change, unfolding.

## 9. Accessible names

| Element | Name | State or description |
|---|---|---|
| Tabs toggle | "Tabs" | `aria-pressed` |
| Count, Width | "Tab count", "Tab width, millimetres" | `aria-invalid` with the chip; Count described by the placed line |
| Skip inner shapes | "Skip inner shapes" (switch) | `aria-checked` |
| Lead pills | "Lead-in", "Lead-out" (radiogroup); radios "None", "Line", "Arc" | `aria-checked` |
| Lead lengths, Overcut | "Lead-in length, millimetres", "Lead-out length, millimetres", "Overcut, millimetres" | |
| Item part | `role="group"` "Selected items in <operation name>" | |
| Tabs, Path options pills | "Tabs", "Path options"; radios "Operation's", "Own" | mixed: description "Mixed" |
| Start | readout text; button "Automatic start" | |
| Direction | "Direction"; radios "Automatic", "As drawn", "Reversed" | |
| Width chip | field "Tab width, millimetres"; button "Operation's width" | |

## 10. Texts (each at most 70 characters)

| Text | Where |
|---|---|
| "Count is 0 to 20." / "Enter a whole number." | validation chip |
| "Width is 0.10 to 20.00 mm." | validation chip (field and entry chip) |
| "Lead-in is 0.10 to 20.00 mm." / "Lead-out is 0.10 to 20.00 mm." | validation chip |
| "Overcut is 0.00 to 10.00 mm." / "Enter a number." | validation chip |
| "No room for this width." | entry chip (C6) |
| "Placed: 3 per path" / "Placed: 2 to 4 per path" / "Placed: none" | muted readout |
| "2 paths are covered by tabs and cut nothing." / "1 path is covered by tabs and cuts nothing." | Checks, warn (criterion 10) |
| "3 leads cross or touch a cut." / "1 lead crosses or touches a cut." | Checks, warn (criterion 18) |

In both warn lines the count with its noun ("2 paths", "3 leads") is the text button of `0029` 6.4: it selects the items concerned on the canvas. No visible notice after an add, a move, a delete or a pill press (no success notice).

## 11. Empty, error, mixed and large states

- **Count 0, no own tabs:** no handle; the item part reads "0 tabs" with Operation's pressed; Tabs mode can still add (that makes own tabs).
- **Path too short** for any tab: placed line "Placed: none"; no handle; no ghost.
- **Damaged settings:** the `0028` damaged state; the lower part and the item part are absent.
- **Mixed selection** (items in this operation and others, open and closed): the part speaks for the items in this operation; rows that apply to none are absent.
- **Locked items** (Layers lock): handles drawn and editable; tabs and start are output, not geometry (as `0111` T11).
- **Entered laser group:** handles show on the selected member and act on the record of its assigned item (C10, spec issue 9).
- **Large:** 1,000 selected items: handles on at most 50 leaves in the viewport (5.1); the redraw per pointer move within 16 ms thread CPU (criterion 25).

## 12. Implementer checklist

- Lower part for Cut and Cut-out: Z offset (capability), "Tabs" heading, Count 96 + Width 128, placed line, Skip switch; "Path options" heading, Lead-in and Lead-out rows (label 56, pill 84, length 84, length absent on None), Overcut 128. Steps and ranges of section 2; chips of section 10.
- Item part after a hairline, only with a selected item in this operation: heading, Tabs pill and readout, Path options pill with summary or own rows, Start readout and Automatic, Direction pill.
- Tabs toggle in the Operation group, custom glyph, hidden off Cut items and in the Toolpath view; mode ends as 4.2.
- Capsule tab handles, triangle start markers (set solid, automatic hollow only in Tabs mode), 12px hit, `pointer`, drag along path with node snap 8px and Alt suspend, 1 mm stop, readout, current handle, Delete, width chip with reset.
- Operations view marks of 6.1 from the job's moves; Toolpath view of 6.3; nothing in the normal view but handles.
- Keys of section 7; sr-only announcements; undo names of section 8; warn lines with selecting text buttons.

## 13. Design-system rows to add at acceptance

"Tab handle", "Start marker", "Tabs toggle (`0139`)" in the Select bar, "Tab, lead and start look (`0139`)" beside the `0101` operation look, "Item part of the unfolded row (`0139`)". No new token, no new colour.

## 14. Decisions for veto (Proposal)

- **C1** The placed number is a per-operation range line, shown only when some path gets fewer.
- **C2** Own tabs and own path options are per item and operation; the item part sits in that operation's body; the unfolded row decides whose handles show.
- **C3** The `0057` pill "Operation's | Own" enters and leaves own tabs and own path options; new step "Own tabs".
- **C4** Automatic start markers only while Tabs is on (Frage 415).
- **C5** Handles on at most 50 selected leaves in the viewport and only on paths of 48 screen px or more.
- **C6** A typed width that leaves a piece under 1 mm is refused.
- **C7** The Automatic marker rests where the job enters, not at the first stored point.
- **C8** Delete on a set start marker is "Automatic start".
- **C9** Tab, arrows and Insert act on handles while Tabs is on.
- **C10** Handles are editable on locked items and inside an entered laser group.
- **C11** Lead shape as a three-icon pill; no icon reads "None" better than an edge alone.
- **C12** The Tabs toggle lives in the Operation group, not in a group of its own.
- **C13** Capsule for a tab, triangle for a start: two new silhouettes, both in `--accent`.
- **C14** In the Operations view an overcut is ink dashes over the red line, a lead a thinned red dash.

## Links

Spec: `specs/0139-laser-cut-path-options/specification.md`. Built notes: `specs/0111-laser-operation-tools/ux-notes.md`, `specs/0101-laser-operations/ux-notes.md`, `specs/0029-laser-job-preview-and-output/ux-notes.md`, `specs/0057-clone-overrides/` (override pill). Design system: `docs/design-system.md`.
