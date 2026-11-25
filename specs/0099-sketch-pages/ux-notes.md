# 0099 Sketch pages: UX notes

ux-engineer, 2026-11-06. Checked against `specification.md` (criteria 1 to 21, Fragen 157 A and 158 A assumed), `specs/0098-page-kinds/ux-notes.md`, the built `PagesBlock.tsx` (grip drag, row, trash), `SketchesPanel.tsx`, `CloneBlock.tsx`, sketch f and i of `specs/0063-manufacturing-workbenches/concept-sketches.html`, `docs/design-system.md` (rows "Drag ghost", "Modifier badge", "Linked clone marks", "Hidden, not disabled", "Page kinds", the `0049` clone block). Customer rules applied: no popups, no operating tips or hint labels (2026-10-16), no notice after a success, hidden not disabled. A label here names a control or a state; the two refusal badges and the trash text are state words the specification itself prescribes. No new colour token. Choices marked (Proposal) are vetoable at the demo.

## 1. The sketch page row

- **Layout.** The `0098` row (44px, grip, name cell, frame-switch column, trash; 3px `--editor-accent` bar when active). Line 1: name and Sketch chip as for every kind. Line 2 (11px tabular): **state glyph (12px), state word, " · ", size**. Example: "[check] Fully constrained · 120 × 80 mm". State first, because it changes while the size does not and the tail truncates; the whole line is the tooltip (`title`) as today.
- **State words and glyphs** (text and glyph, never colour alone; Lucide 12px, 1.5px stroke, `currentColor`):

| State | Glyph | Row text | Sketches tab says (built) |
|---|---|---|---|
| Empty | `CircleDashed` | Empty | no sketch yet |
| Fully constrained | `CircleCheck` | Fully constrained | Fully constrained · 0 DOF |
| N degrees of freedom | `Move` | 3 DOF | Under-constrained · 3 DOF |
| Conflict | `TriangleAlert` | Conflict | Conflicting |
| Solver failed | `CircleX` | Solver failed | error line |

  "Conflict" and "Solver failed" are drawn at full opacity (the rest of line 2 is the usual 65 %), the only emphasis; no red. The row updates in the frame the solver reports. Not announced when it changes (the Sketches tab status is the live place); read with the row.
- **Accessible name.** "<name>, Sketch page, <state>" ("Bracket, Sketch page, fully constrained", "Bracket, Sketch page, 3 degrees of freedom"); glyph and chip `aria-hidden`. The spoken form of DOF is spelled out; the visible one is short to fit 140px (see conflicts).
- **Used by N clones.** Not a third item on line 2 (no room at 244px). It is: in the tooltip of line 2 ("Fully constrained · 120 × 80 mm · used by 3 clones"), in the Sketches tab (section 4) and in the trash text (section 7). Nothing else in the row.
- **Frame switch.** The "Move frame only" switch is absent on a sketch row (criterion 5), its 28px column stays empty so the trash aligns with the other rows and nothing shifts on a state change. It leaves the row's Right-arrow sequence.
- **Grip.** Keeps its name "Reorder Bracket" and its place; it is also the drag source for placing a clone (section 2). No second grip, no change in name: the keyboard route is the button of section 3.

## 2. Dragging a sketch row out of the list (criteria 8 to 10)

- **Source.** Only the grip (as built; the name button keeps press, double-click rename, F2). A press and a 4px move start it. Tab does not reach the grip; nothing changes for the keyboard.
- **Two modes, one drag.**
  - Pointer inside the list: reorder exactly as built (insertion cue, row stays).
  - Pointer outside the list: place mode. Switching is per frame, both ways, so the pointer can come back and still reorder. The row never leaves the list; there is no row copy under the pointer.
  - Drawing and laser rows leave the list as built today (no place mode; `0100` adds theirs). Releasing outside the list for those rows keeps today's behaviour.
- **Targets under the pointer (place mode).**

| Under the pointer | Ghost | Badge | Frame cue | Release |
|---|---|---|---|---|
| Drawing page, laser page | yes, full strength | `Link2` "Linked clone" | the page frame gets a 2px `--editor-accent` outline, 2px outside the frame, no fill (on top of its normal edge; the active page keeps its stronger edge under it) | Place sketch clone (one step) |
| Pasteboard | yes | "Linked clone" | none | Place sketch clone; the active page stays |
| Sketch page (own or other) | yes, at 40 % | `Ban` "Not on a sketch page" | none | nothing created, no step |
| Empty sketch, over the canvas anywhere | no outline to draw: no ghost | `Ban` "Bracket has no sketch yet" (page name, truncated at 24 characters) | none | nothing created |
| Panel, rails, bar, status bar (not the list) | none | none | none | cancel, no step |
| Escape (any time) | gone in that frame | gone | gone | cancel, no step; the row never moved |

- **Ghost.** The sketch's solved outline at true size for the current zoom, centred under the pointer; line and casing exactly the "Drag ghost" row (1.5px hollow `--drag-ghost` over white casing), no fill, constructions, dimensions and symbols not drawn. It shows where the clone's box centre will land, including the snapping of a Select move (guides drawn as for a move). Clipped to the canvas. An outline under 6px on screen draws as an 8px ring at the pointer. No animation: release and Escape remove it in that frame; a refused release does not travel back to the row (the row never left).
- **Badge.** New pattern "Drop badge": DOM, `aria-hidden`, `pointer-events: none`; a pill 20px high, `--toolbar-bg`, 1px `--toolbar-icon` border at 25 %, 8px radius, 6px side padding, a 12px glyph, 4px, 12px `--toolbar-icon` word. Placed with its top-left 16px right of and 16px below the hotspot (the "Modifier badge" is left-below and the readout chip up-right, so the three never meet); mirrored to the left or above at a viewport edge. A refusal also sets `cursor: not-allowed`. The word is the whole message; no second line.
- **Active page and focus.** Dragging a row does not activate the source page. A successful release activates the page under the pointer (pasteboard: nothing changes), selects the clone, picks the new kind's Select tool if the current tool is missing there (`0098` section 7), and leaves focus on the canvas. Status bar and header follow in the same frame (`0098`). No notice.
- **Undo.** One step "Place sketch clone". Ctrl+Z removes the clone and clears the selection; the active page stays where the drop put it (activation is not a history step). Redo places it again at the same spot.
- **Many pages and far targets.** No auto-pan during the drag. A target outside the viewport is reached with section 3.
- **Pointer only.** A touch or pen drag works the same through the grip (36px on a coarse pointer); a keyboard has section 3.

## 3. Place clone on (criterion 11)

- **Where.** Sketches tab, in the new "Use" group (section 4). Hidden (not disabled) while the sketch is empty or the project has no drawing or laser page.
- **Button.** Full width (244 x 28px, 36px coarse), the outline "Fit to content" look, label **Place clone on**, `ChevronDown` 14px at the right (`ChevronUp` while open, no rotation animation), `aria-expanded`, `aria-controls`. Tooltip: none beyond the name.
- **List.** Opens inline directly under the button, pushes the rest down, closes again, no popup: `role="listbox"` named "Place clone on", `--toolbar-bg`, 4px padding. Rows 36px (44px coarse): the page name (truncates, tooltip) and the kind chip at the right. Drawing and laser pages in list order; sketch pages are absent. At most 6 rows, then it scrolls inside itself.
- **Keys.** Focus moves to the first row on open. Up, Down, Home, End move (do not choose). Enter or Space chooses; Escape closes, changes nothing, focus back to the button. Tab leaves the list and closes it without choosing. A press on a row chooses.
- **Result.** Places the clone at that page's centre with the rules of criterion 8, activates the page (revealed by panning, never zooming), selects the clone, closes the list. The step and undo are those of the drag. **Focus goes to the canvas**, not into the panel: the Sketches tab no longer exists on the new page, and the next thing the maker does is with the clone (arrow keys, Delete, Ctrl+Z). This differs from the `0049` key rule (first control of the new body) because the body is gone; see section 8.
- **Presses.** One page: button, row (2, plus Enter). Pointer: 2 clicks.

## 4. The Use group in the Sketches tab (criterion 12)

- **Order in the tab.** Status (as built), **Use group**, Fixed origin, Geometry, relationships, **Convert to paths** (last, as built). The group is a `role="group"` named "Use", no heading text, 8px between its items, 16px above and below as the tab spaces its sections.
- **Usage line.** 12px `--panel-muted-fg` text: "Not used on other pages", "Used by 1 linked clone on 1 page", "Used by 3 linked clones on 2 pages". Plain text, no glyph, no Tab stop, updates in the frame a clone appears or goes. Shown also while the sketch is empty (clones can outlive their geometry).
- **Select clones.** Full-width outline button, label **Select clones** ("Select clone" for one), in the tree only while a clone exists, directly under the line; then Place clone on. Selects the clones on the first page (list order) that has one and activates it. Focus to the canvas (the tab is gone), mouse and key alike. Clones on later pages are reached by returning to the sketch page and the row of that page, or by Select original from a clone: accepted limit (the specification asks for the first page only). No notice, no count.
- **Hidden or locked clones.** The `0049` refusals apply unchanged ("The clones are hidden or locked. Show or unlock them in the Layers tab. Nothing was changed."; the partial status line), anchored under the pressed button, and then no page is activated.

## 5. Reaching the sketch from a clone (criterion 13)

- **Double-click** a clone with Select, or **Select original** in the `0049` clone block (label unchanged): the sketch page becomes active in that frame with the sketch rail and tabs; the view pans to the page only if no part of it is visible (never zooms).
- **Selection.** No sketch geometry is selected. The clone's own selection is cleared (Proposal): the sketch tools cannot act on it, and a selected object with no box on the active page would look like an error. This is the one exception to "selection is kept across activation" of `0098` section 7; a press on a page keeps it.
- **Tab.** The `0098` rule (tab last used on that kind, else Document). The panel does not jump to Sketches.
- **Focus.** Canvas, after the double-click (it stays) and after the button (the button's tab is gone). Not a history step.
- **Unlink clone** on a clone of a sketch: as built (`0049`): the notice "Unlinked 3 clones." anchored under the button, focus to the first control of the Style area; the page does not change.
- **Mixed selection.** If the selection holds clones of two sketches, Select original selects both originals' pages as `0049` does for originals; only one page can be active, so the first in list order is activated and no selection is made on the other: not a case for notes beyond this line (Proposal).

## 6. Drawing objects over a sketch page (criteria 1 to 4)

- **Looks.** Drawn as drawn, in their stacking place, over the sketch. No notice, no tint, no label on the page. The page label and chip are unchanged.
- **Where it shows.** The status bar left says where the object lies when it is selected: "Circle · pasteboard" (sketch i, item 7), the same words as for any pasteboard object.
- **Press table.**

| Active page | Press (Select, Shift+press, marquee start) | Result |
|---|---|---|
| Drawing | on the object | object selected, page stays active, edited and moved as a pasteboard object |
| Drawing | in the sketch frame, on no object | the sketch page activates (`0098`); rail and tabs change |
| Drawing | in the sketch frame, on sketch geometry, no object over it | same: activates |
| Drawing or pasteboard marquee | crossing the frame | takes drawing objects only; the sketch is never part of it |
| Sketch | on a drawing object over it | passes through: hits the sketch geometry beneath, else empty canvas (marquee of sketch geometry) |
| Sketch | marquee, Ctrl+A, Delete | sketch geometry only |
| Laser | on a drawing object over a sketch page | as built in `0098`; move-only selection is `0100` |

- **Cursor and hover.** With a sketch page active the cursor and the hover cue are those of what lies beneath the pointer; the object gets no hover box. With a drawing page active the object has its normal hover box and the sketch geometry beneath gets none.
- **Equal hits.** Where an object and sketch geometry are both under the pointer and a drawing page is active, the object wins (it is drawn above).
- **Select all.** Ctrl+A on a drawing page selects drawing objects, the sketch never; no count of what was skipped.

## 7. The sketch is fixed; frame move, size, Convert, Delete

- **No box, no handles.** The sketch never gets a selection box, handles, hover box or marquee mark in any context. The Document tab's X and Y of the page move the frame and the sketch together (criterion 5); there are no object fields for the sketch.
- **Frame move.** The sketch outline moves with the frame during the drag in the same frames; clones stay put; drawing objects stay put. One step "Move page". The row has no frame-only switch.
- **Size change.** The sketch stays; the frame resizes around it.

### Convert to paths (Frage 157 A)

- **Control.** The existing **Convert to paths** button, now full width (244 x 28px), outline look, last item of the Sketches tab; hidden while the sketch is empty. No tooltip, no confirm: reversible, one step.
- **Frame of the press.** In one render: the paths exist and are selected; the row chip, header chip, canvas label and status bar read Drawing; the sketch rail becomes the vector rail with the Select tool; the tab strip loses Sketches and Parameters; the open tab becomes the tab last used on a drawing page, else Document; the row's state words go, its frame-switch column fills. The page frame and all clones keep their place and look. The page-kind announcement reads "Bracket, drawing page, 3 of 5" (the `0098` activation words), politely; the selection announcer speaks the paths.
- **Focus.** The pressed button is gone, so focus goes to the canvas (the existing `focusCanvas`).
- **Undo.** One step "Convert sketch to paths". Ctrl+Z gives back the sketch page, sketch rail, Sketches and Parameters tabs, constraints, dimensions, parameters and the state line, with the drawing Select tool replaced by the sketch Select tool and the selection empty. Focus stays on the canvas (key) or in the History tab (row press). The tab shown is the one last used on a sketch page.

### Delete page (Frage 158 A)

- **Control.** The row's trash button, unchanged place. Tooltip (400ms, `side="left"`) and accessible name: "Delete Bracket and its sketch; 3 clones become paths"; with one clone "1 clone becomes a path"; none: "Delete Bracket and its sketch". Count is live. Other kinds keep "Delete page: <name>". The Delete key on a focused row does the same without the tooltip; undo covers it.
- **Press.** No dialog, no inline confirm (the specification's Proposal; one undo step). The last page stays undeletable (the trash is absent, as built).
- **Result.** The page and sketch go; each clone is ordinary paths in its place with the same look; objects over the page stay where they are, now plainly on the pasteboard. A selected clone stays selected as its paths (Proposal; if the implementation cannot carry the ids, the selection clears). If the page was active, the neighbour page activates as for any delete (`0059`).
- **Focus.** As built: the row now at the deleted index, else the last row (mouse and key).
- **Undo.** One step "Delete sketch page": page at its list place, sketch with constraints and parameters, the clones linked again. Active page and selection return to what they were before the delete; the restored page does not steal activation unless it was active. Focus stays where Ctrl+Z was pressed.

## 8. Focus rule for commands that change the page

A command that activates another page (Place clone on, Select clones, Select original onto a sketch, Convert to paths) leaves focus on the canvas. Reason: the focused panel control belongs to a tab the new page does not have, and the next action is on the canvas. It overrides the `0049` key rule (first control of the new body) for the page-changing case only; commands that stay on the page keep theirs.

## 9. Keyboard map (additions)

| Action | Key | Notes |
|---|---|---|
| Place clone on | button in Sketches; Enter or Space opens, Up, Down, Home, End, Enter choose, Escape closes | section 3 |
| Cancel a row drag | Escape | section 2 |
| Delete page (sketch row) | Delete on the focused row | same step as the trash |
| Select original of a clone | double-click, or the button | section 5 |

No new global chord. The pointer drag has exactly one keyboard route: the Place clone on button.

## 10. Empty, error and large states

- **Empty sketch.** Row says Empty; drag shows the refusal badge; Place clone on, Convert to paths hidden; the usage line stays.
- **Solver failed or conflict.** Clones follow the last valid solve (`0057`); drag, Place clone on and Convert work on it; the row shows the state at full opacity.
- **No target page.** No drawing or laser page: Place clone on hidden; the drag can still end on the pasteboard.
- **Large documents.** Ghost redraw is the 16ms budget of criterion 20; the ghost draws the solved outline only. Page list with 200 pages: the drag does not auto-scroll the list; reorder is as built.
- **Dark theme.** Tokens only (`--drag-ghost`, `--toolbar-bg`, `--toolbar-icon`, `--editor-accent`).

## 11. Review checklist for the implementer

Row at 244px: the longest state ("Fully constrained · 120 × 80 mm") truncates its tail, never the glyph or word; trash aligns across kinds; drag out and back into the list switches mode in the same frame and Escape leaves the list order unchanged; badge never over the ghost's own centre cross and never off-viewport; refusal over a sketch page, empty-sketch refusal and a release over a panel all add no history step; focus lands on the canvas after drop, Place clone on, Select clones, Select original and Convert; the list of Place clone on has no sketch pages and scrolls at 7 or more rows; contrast of the 40 % ghost on `--canvas-bg` stays at least 3:1 (check, else 50 %); press table of section 6 covered by a test per row; screen reader reads the row state, the trash consequence and the Convert result.

## 12. Revision 2026-11-19: sketch instances (criteria 23 to 31)

ux-engineer, 2026-11-25. Checked against criteria 23 to 31 (Frage 274 A), `adrs.md` (ADR 0080 §6, §7), `specs/0141-parametric-generators/ux-notes.md` §6, §9 and **§15** (revision 2: the Generator block's numbers are the panel `ValueField`, built in `GeneratorBlock.tsx`), `curvyo-ui-core/src/instance*.rs`, `CloneBlock.tsx` (the `extraRow` slot), `SketchParameters.tsx`, `0066` UX notes §3 (entry box), `docs/design-system.md` (rows "Value field", "Parameter handle", "Arc handle layout", "Live preview outline", "Live transform readout", "Vector dimension look", "Clone override" of `0135`, "Action notice", "Hidden, not disabled", "Inline delete confirm"). One look for both parametric instances: what 0141 says for a generated object holds for a sketch clone unless named here. No hint labels, no success notice, no new token. Everything not a criterion is **(Proposal)**.

### 12.1 Expose on the sketch page (criterion 23)

- **Parameter row** (Parameters tab): a 28px `PanelSwitch` row **Expose** after the value line, off by default. Shown for Length, Angle and Count parameters; not for Scalar (U3). On: two fields appear under it, side by side (118px each, 8px apart, the `SketchField` look, label inside "Min" / "Max", unit of the kind); empty means unlimited and reads "None" muted (a state word). Off: the fields leave the tree and the limits (and a dimension's name) are not kept; Undo brings them back (U1). One step "Expose parameter" / "Stop exposing parameter"; a name or limit change "Change exposed parameter" (U2).
- **Driving dimension** (Sketches tab row): the same switch and Min/Max, plus a **Name** field above them (prefilled "Distance 1", "Radius 1", "Diameter 1", "Angle 1", numbered per kind in list order; the name is the clone field's label; must be unique among exposed names, else `--field-invalid` with "Name in use" in the row). A reference dimension has no switch (hidden): it drives nothing.
- **Entry box** (`0066` §3): row 2 gets a checkbox **Expose** after Reference, the same look (`tabindex="-1"`, click only); Reference checked hides it. The keyboard route is the Sketches row's switch. Accessible names "Expose Width", "Expose Distance 1".
- **Mark on the sketch page.** An exposed dimension's text pill starts with a 6px hollow circle in its text colour (the Parameter handle silhouette, `aria-hidden`; the dimension's accessible name adds ", exposed"). It says which values clones may change, so it is a state, not a hint.
- **Stop exposing / Remove while clones hold own values.** Undoable, so no arm on the switch: its tooltip and accessible description name the consequence ("3 clones go back to the sketch's value"); own values are dropped (criterion 27) and come back with Undo. The row's Remove × is a row delete and gets the Inline delete confirm (customer rule) only while clones hold own values: armed name "Press again to remove Width. 3 clones lose their own value."; otherwise it removes at once as built.

### 12.2 The Parameters override row and the value fields (criterion 24)

- **Place.** The clone block's `extraRow` slot (between Placement and Style, where an image clone's Adjustments sits). In the tree only while the selected clones' sketch exposes at least one parameter. Label "Parameters" 12px muted, the 244 x 28px two-segment pill **Original's / Own**, `role="radiogroup"`, one Tab stop (the 0057 look). State: Own when any parameter has an own value; mixed selection: no segment pressed, hidden "Mixed".
- **Segments.** Original's drops every own value of the selection (one step "Back to the sketch's values"); there is no separate "all" button. Own writes the current values as own values for every exposed parameter, so nothing moves (one step "Own parameter values").
- **Fields.** Directly under the pill, 8px apart, one **`ValueField`** per exposed parameter (0141 §15.1, §15.2: drag in the field, click, type, arrows with Shift x10 and Ctrl x0.1, one commit per gesture, `SliderTip`). Order: the Parameters list, then exposed dimensions in Sketches order. Label = the exposed name; unit mm, ° (degrees, also for a parameter the sketch shows in rad), none for Count; Count steps 1 (Shift 10, Ctrl nothing). A value that follows the original is drawn **muted** (the "automatic" look) and its `aria-valuetext` ends ", sketch's value"; an own value is full strength. The first edit while Original's makes the row Own in the same step (the `0135` rule). Step "Change parameter" (ADR 0080 §7).
- **Per-parameter Back.** The `ValueField` reset slot, shown only for an own value; it and Ctrl+Backspace remove the own value. Name and tooltip "Back to the sketch's value" (depth 1) / "Back to the original's value" (a chain, 12.6). Step "Back to the sketch's value".
- **Track.** A `ValueField` needs a track; a sketch parameter has only Min/Max (ADR 0080 §6). Rule: with both limits the track spans them; else a length runs log from Min (or 0) to 4 x the sketch's value (at least 10 mm), overshoot up to Max or unbounded; an angle linear 0 to 360° (or its limits); a count linear 0 to max(10, 2 x value), to 200. Typed values reach the limits; beyond them they are clamped with the 0141 §6 notice ("Width set to 120 mm, the maximum.").
- **Several clones** of one sketch: one list, Mixed where values differ, a typed value writes all. Clones of different sketches: the Parameters row shows the mixed state and no fields.

### 12.3 Handles on the canvas (criterion 25)

- **Which.** Only one selected clone, its box at least 72px on screen, as 0141 §9 (glyph, 12px hit, collisions, not during a transform drag). A **length, radius or diameter**: an exposed dimension's knob sits at its anchor (distance: the dimension line's second arrowhead end; radius and diameter: the point where the leader meets the circle), dragged along the dimension's direction; a diameter changes by twice the radial move. An exposed **named parameter** has a knob only when exactly one driving dimension's expression is that bare name; a formula or several uses: field only. **Angle**: the knob on its arc by "Arc handle layout", readout "<name> 30.0°", Shift snaps whole degrees. **Count**: field only.
- **Frame.** Anchor and direction follow the clone's matrix (rotated, mirrored, scaled). The value is in the sketch's units: on a 50 % clone a 10 mm pointer move changes a length by 20 mm; readout, field and limits show the parameter value, never the on-page size.
- **Drag.** Hover and drag draw the dimension's own extension lines and arrowheads in `--dim-guide`, no text; readout chip "Width 82.0 mm", "... max" / "... min" at a limit; Shift snaps to whole mm; preview blue new, black old; Escape restores and writes nothing; release = one step "Change parameter", the row turns Own. A double-click on a knob opens the numeric entry chip (one field, the name); elsewhere on the clone a double-click stays Select original (criterion 13), the knob wins the hit. Knobs are not focusable; the field of the same name is their keyboard route.

### 12.4 No solution (criterion 26)

- **Canvas.** The clone draws the last values that solved, saved with the clone (Frage 372 A; without them its original's shape, else the sketch's), in its own style: the same after reopen, on every device, and what is cut (U4). Over it, screen-space only and never exported or cut, a 1.5px dashed `--destructive` outline (4 on, 3 off), shown selected or not: one look for "not as asked" on any page.
- **Panel.** Under the Parameters pill a state line: `TriangleAlert` 12px, "No solution for these values", 12px `--toolbar-icon` at full opacity, linked by `aria-describedby` from every field; announced once, politely, when it appears. Fields keep the values typed, no `aria-invalid` (permit over refuse). Status bar left: "Linked clone · Page 2 · no solution".
- **Drag into it.** The knob follows the pointer, the preview holds the last solvable geometry, the readout adds " · no solution"; the release still commits.

### 12.5 Sketch edits, Convert, Unlink (criteria 27, 29)

- A sketch edit updates every clone in that frame with its own values kept, no notice. Convert to paths and Unlink clone use the clone's own values (criterion 29); in the no-solution state they take the saved fallback, as the canvas showed it.

### 12.6 Chains (criterion 28)

- A clone of a clone shows the value it gets (its own, else the nearest own value up the chain, else the sketch's) and draws an inherited value muted. Its reset says "Back to the original's value" and gives the upstream value. Original's on a chained clone drops only this clone's own values.

### 12.7 Accessibility and keys

- Tab order in the clone block: Placement, Parameters pill, the fields, Style. Fields: the Value field's own semantics, names "<clone name> <parameter>" ("Bracket clone width: 82 mm"). No new key or chord; Ctrl+Backspace is the Value field's own. Contrast: tokens only (`--destructive` 6.5:1 on white; check the dark theme).

### 12.8 Rows for `docs/design-system.md` when Done

"Expose switch and limits" (parameter and dimension rows, entry box checkbox, exposed-dimension mark); "Parameters override" beside `0135`'s Clone override row (pill, muted inherited values, reset name, track rule for unbounded parameters); "Parameter handle" amended for sketch instances (anchor, matrix, diameter x2, bare-name rule); "No solution outline" (dashed `--destructive`, screen only) and its state line; the 0141 §14 item 4 reading (Params field) is void: both instances use the Value field.
