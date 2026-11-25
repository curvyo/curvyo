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
- **Fields.** Directly under the pill, 8px apart, one **`ValueField`** per exposed parameter (0141 §15.1, §15.2: drag in the field, click, type, arrows with Shift x10 and Ctrl x0.1, one commit per gesture, `SliderTip`; slice 2 replaces Shift in this row, 12.9.3). Order: the Parameters list, then exposed dimensions in Sketches order. Label = the exposed name; unit mm, ° (degrees, also for a parameter the sketch shows in rad), none for Count; Count steps 1 (Shift 10, Ctrl nothing). A value that follows the original is drawn **muted** (the "automatic" look) and its `aria-valuetext` ends ", sketch's value"; an own value is full strength. The first edit while Original's makes the row Own in the same step (the `0135` rule). Step "Change parameter" (ADR 0080 §7).
- **Per-parameter Back.** The `ValueField` reset slot, shown only for an own value; it and Ctrl+Backspace remove the own value. Name and tooltip "Back to the sketch's value" (depth 1) / "Back to the original's value" (a chain, 12.6). Step "Back to the sketch's value".
- **Track.** A `ValueField` needs a track; a sketch parameter has only Min/Max (ADR 0080 §6). Rule: with both limits the track spans them; else a length runs log from Min (or 0) to 4 x the sketch's value (at least 10 mm), overshoot up to Max or unbounded; an angle linear 0 to 360° (or its limits); a count linear 0 to max(10, 2 x value), to 200. Typed values reach the limits; beyond them they are clamped with the 0141 §6 notice ("Width set to 120 mm, the maximum.").
- **Several clones** of one sketch: one list, Mixed where values differ, a typed value writes all. Clones of different sketches: the Parameters row shows the mixed state and no fields.

### 12.3 Handles on the canvas (criterion 25)

- **Which.** Only one selected clone, its box at least 72px on screen, as 0141 §9 (glyph, 12px hit, collisions, not during a transform drag). A **length, radius or diameter**: an exposed dimension's knob sits at its anchor (distance: the dimension line's second arrowhead end; radius and diameter: the point where the leader meets the circle), dragged along the dimension's direction; a diameter changes by twice the radial move. An exposed **named parameter** has a knob only when exactly one driving dimension's expression is that bare name; a formula or several uses: field only. **Angle**: the knob on its arc by "Arc handle layout", readout "<name> 30.0°", Ctrl (Cmd on macOS) snaps whole degrees. **Count**: field only.
- **Frame.** Anchor and direction follow the clone's matrix (rotated, mirrored, scaled). The value is in the sketch's units: on a 50 % clone a 10 mm pointer move changes a length by 20 mm; readout, field and limits show the parameter value, never the on-page size.
- **Drag.** Hover and drag draw the dimension's own extension lines and arrowheads in `--dim-guide`, no text; readout chip "Width 82.0 mm", "... max" / "... min" at a limit; **Ctrl (Cmd on macOS) snaps to whole mm** (revised 2026-11-25: Shift is the link rule of 12.9; Ctrl is already the canvas snap key, "45°" under Ctrl on the rotate readout and "Ctrl snaps to 15°" on the arc knobs; the knob has no fine mode, so Ctrl meaning "fine" inside a Value field and "snap" on a knob is the split rotate already has); preview blue new, black old; Escape restores and writes nothing; release = one step "Change parameter", the row turns Own, and the Original's / Own pill shows the state from before the gesture until that release (criterion 24). A linked value's knob: 12.9.4. A double-click on a knob opens the numeric entry chip (one field, the name); elsewhere on the clone a double-click stays Select original (criterion 13), the knob wins the hit. Knobs are not focusable; the field of the same name is their keyboard route.

### 12.4 No solution (criterion 26)

- **Canvas.** The clone draws the last values that solved, saved with the clone (Frage 372 A; without them its original's shape, else the sketch's), in its own style: the same after reopen, on every device, and what is cut (U4). Over it, screen-space only and never exported or cut, a 1.5px dashed `--destructive` outline (4 on, 3 off), shown selected or not: one look for "not as asked" on any page.
- **Panel.** Under the Parameters pill a state line: `TriangleAlert` 12px, "No solution for these values", 12px `--toolbar-icon` at full opacity, linked by `aria-describedby` from every field; announced once, politely, when it appears. Fields keep the values typed, no `aria-invalid` (permit over refuse). Status bar left: "Linked clone · Page 2 · no solution".
- **Drag into it.** The knob follows the pointer, the preview holds the last solvable geometry, the readout adds " · no solution"; the release still commits.

### 12.5 Sketch edits, Convert, Unlink (criteria 27, 29)

- A sketch edit updates every clone in that frame with its own values kept, no notice. Convert to paths and Unlink clone use the clone's own values (criterion 29); in the no-solution state they take the saved fallback, as the canvas showed it.

### 12.6 Chains (criterion 28)

- A clone of a clone shows the value it gets (its own, else the nearest own value up the chain, else the sketch's) and draws an inherited value muted. Its reset says "Back to the original's value" and gives the upstream value. Original's on a chained clone drops only this clone's own values.

### 12.7 Accessibility and keys

- Tab order in the clone block: Placement, Parameters pill, the fields, Style. Fields: the Value field's own semantics, names "<clone name> <parameter>" ("Bracket clone width: 82 mm"). No new global key or chord; Ctrl+Backspace is the Value field's own. Slice 2 changes modifiers inside the Parameters row and on its knobs only (12.9.3, 12.9.4): Shift = only this value on a value in a link group (elsewhere Shift x10 as built), Ctrl (Cmd) snaps a knob, PageUp / PageDown step ten times. Contrast: tokens only (`--destructive` 6.5:1 on white; check the dark theme).

### 12.8 Rows for `docs/design-system.md` when Done

"Expose switch and limits" (parameter and dimension rows, entry box checkbox, exposed-dimension mark); "Parameters override" beside `0135`'s Clone override row (pill, muted inherited values, reset name, track rule for unbounded parameters); "Parameter handle" amended for sketch instances (anchor, matrix, diameter x2, bare-name rule); "No solution outline" (dashed `--destructive`, screen only) and its state line; the 0141 §14 item 4 reading (Params field) is void: both instances use the Value field. Slice 2 adds: a row **"Value link"** (group letter, link mark, the sketch's Link line and list, the Shift rule of 12.9.3); "Parameters override" amended (link marks in the field, Shift = only this, PageUp / PageDown, committed state during a gesture); "Parameter handle, sketch instance" amended (Ctrl / Cmd snaps, followers of a linked knob, the readout "Radius 1 = Radius 2 40.0 mm"); "Parameter handle" Followers rule widened from Link corners to linked values.

### 12.9 Linked values (slice 2, criteria 32 to 36; Fragen 373 B, 374 B)

ux-engineer, 2026-11-25. One rule from the customer: in the sketch the maker links exposed values on purpose; on a clone, changing one changes its partners to the same value; Shift changes only that one and unlinks it there; Back to the sketch's value links it again. Checked against the "Link corners toggle" and "Parameter handle" rows (Followers; Shift at the press inverts coupling), "Transform entry chip" (chain glyph between linked W and H), "Arc handle layout", "Value field", "Hidden, not disabled" and the "no popovers in the panel" rule.

- **Glyphs.** Lucide `Link` (closed chain) and `Unlink` (broken), 1.5px stroke: the design system's words for coupled values (Link corners couples four radii, the same meaning). `Link2` stays the clone glyph of the clone block and is never used here. On the **sketch page** no chain glyph is drawn: there `Unlink` in `--dim-guide` already means a vector dimension unlinked from its geometry (`0070`).
- **Group letter.** Groups are told apart by a letter, never a colour (no new token, never colour alone): A, B, C in the order of each group's first member in the exposed order (Parameters list, then Sketches order). Display only, not stored, so a letter may change when a group dissolves or rows move. Shown also when the sketch has one group, so the mark is the same everywhere **(Proposal)**.
- **Link mark** (panel and chips): the 12px glyph, 2px, the letter 11px medium tabular. Linked: `--panel-muted-fg`; unlinked on this clone: `--toolbar-icon` at full strength, so shape and strength both differ. `aria-hidden`; the state is in the control's text (12.9.5).

#### 12.9.1 Setting a link on the sketch page (criteria 32, 35)

- **Where.** A **Link line** at the end of the exposed block of a row (Parameters tab parameter row, Sketches tab dimension row), under Min / Max, 8px above it. In the tree only while the value is exposed, is a length or an angle (a radius and a diameter are lengths) and at least one other exposed value of the same kind exists; never for a Count. **Hidden, not disabled:** a value of another kind is simply not offered, so there is no off control with a tooltip saying why (this replaces that wording of criterion 32; to align with the product owner).
- **Not linked.** A 244 x 28px outline button (the Place clone on look), label **Link**, `ChevronDown` 14px at the right (`ChevronUp` while open), `aria-expanded`, `aria-controls`, name "Link Radius 1". No tooltip beyond the name.
- **The list.** Opens inline under the button and pushes the rest down, no popup (section 3's list, keys and sizes): `role="listbox"` named "Link Radius 1 with". One option per existing group of the same kind first (the mark, then the member names: "A Radius 3, Radius 4"; choosing it joins that group), then one per ungrouped exposed value of the same kind ("Radius 2"; choosing it forms a new group of two). Both tabs' values are in one list, because partners may live in different tabs (a parameter and a dimension); that is why there is no "press Link here, then press the partner row" pick mode. Choosing closes the list, one step **"Link parameters"**, focus to the row's Unlink button. Escape closes, nothing changes, focus back to Link. A group member is never offered to another group's row while it is in a group (criterion 32: one group per value); to move it, Unlink it first.
- **Linked.** The button gives way to a line in the same 28px: the link mark, 4px, "Linked with Radius 2" (12px `--toolbar-icon`; more members "Linked with Radius 2, Radius 3"; truncates, full text as tooltip), and at the right a 28px text button **Unlink** (outline, 14px label, name "Unlink Radius 1"). Unlink is one step **"Unlink parameters"**, no confirm (undoable); a group left with one member dissolves (both rows show Link again) and clones' unlinked states of it go with it (35). Focus then goes to the row's Link button, else (no partner left of that kind) to the Expose switch.
- **Stop exposing a linked member** (35). The Expose switch's tooltip names the link first: "Leaves the link with Radius 2. 3 clones go back to the sketch's value." (the consequence rule of 12.1). Same for the parameter's Remove ×: the armed text of 12.1 is unchanged (it already names the clones); the link goes in the same step and Undo restores both.
- **Entry box** (`0066` §3): no Link there **(Proposal)**: it is a short-lived chip for typing a dimension; a list does not fit it. The route is the Sketches row.
- **On the sketch canvas.** A linked exposed dimension's text pill shows the group letter, 10px in its text colour, right after the 6px hollow circle ("○A 40"); its accessible name adds ", linked with Radius 2" **(Proposal)**. The sketch's own values are not changed by a Link, so nothing moves on the sketch page.

#### 12.9.2 The clone's Parameters row (criteria 33, 34, 36)

- **Place in the field.** The link mark sits **right after the label**, 4px from it, before the value; the label truncates first, the mark never. The value's place (32px from the right) and the always-reserved 24px reset slot do not move.

| State on the selected clone(s) | Mark | Tooltip of the mark (400ms) |
|---|---|---|
| Not in a group | none | none |
| Linked (also: inherited value up a chain, 36) | `Link` + letter, muted | "Linked with Radius 2" |
| Unlinked by this clone | `Unlink` + letter, full strength | "Unlinked from Radius 2" |
| Several clones, states differ | none | none (the accessible text says it) |

- **No press on the mark.** It is not a button. An unlinked value always holds an own value, so its reset slot is always shown, and **Back to the sketch's value** (name unchanged) is the relink route the customer named: it drops the own value, the field turns muted with the `Link` mark, the partners are not touched (34). A second control that relinks while keeping the value would almost do the same thing and is a trap; not offered.
- **Original's** drops every own value and every unlinked state of the selection in one step: all marks read `Link`.
- **Chains** (36): a clone shows `Unlink` only for values it holds unlinked itself; a value it inherits from an upstream clone's unlinked own value shows `Link`, because its own edits couple.

#### 12.9.3 Changing a linked value in a field (criteria 33, 34)

- **Coupled (no Shift).** The partners' fields show the same number and bar live in every frame; their border takes the hover strength (`--toolbar-icon` 100 %) as followers **(Proposal)**, as partner knobs do on the canvas. One step "Change parameter"; Escape restores all.
- **Committed state during a gesture.** The pill, the muted or full look and the marks of the edited field and of every partner show the state from before the gesture; only number and bar move. At the commit, in one frame: pill Own, partners full strength, and (with Shift) the `Unlink` mark. This is criterion 24's pill fix, widened to the partners.
- **Shift = only this value, decided at the press** (pointer down, or the key-down of an arrow), like Link corners: pressing or releasing Shift later in the gesture changes nothing. A gesture that ends on the old value, and Escape, unlink nothing (34).
- **Shift is not "coarse" on a value in a link group** **(Proposal)**, whether it is linked or already unlinked on this clone (it has partners either way; on an unlinked one Shift changes nothing beyond what the edit already does). Two meanings on one key would make every "only this" drag ten times coarser, so a one-off value could not be set finely. While a Shift gesture on such a value runs, the field's centred modifier word reads **"only this"** (muted 12px, `aria-hidden`, the place of "coarse"). A value **not in a group keeps Shift x10 and "coarse" exactly as built** in slice 1 (accepted behaviour, PR #482). Ctrl (Cmd) stays fine (x0.1, "fine") everywhere. Ten-times steps by key on any Parameters field: **PageUp / PageDown**, the spinbutton standard **(Proposal)**; on a grouped value Shift+Arrow is one normal step for that value only.
- **A new Link moves nothing on clones.** Clones whose own values of the new members differ keep them until their next coupled edit (criterion 33); nothing is equalised at link time.
- **Typing.** Enter in a linked field commits coupled; Shift+Enter does the same as Enter (criterion 34: no Shift on a typed commit). In an unlinked field typing changes only that value. To type a one-off value into a linked field: Shift+Arrow (unlinks it with one step), then type. There is no field menu "Change only this": the panel has no menus or popovers.
- **Limits.** A coupled drag stops at the tightest limit of all members (33): the bar stops, and the `SliderTip` shows the value with " max" / " min". A typed value beyond it is clamped with the 12.2 notice naming the member whose limit held: "Radius 1 set to 30 mm, the maximum of Radius 2." (own limit: the 12.2 text unchanged).
- **Several clones.** A typed or dragged value writes every selected clone, each coupled inside itself, except where that clone holds it unlinked (then only it). The marks show the committed mixed reading.

#### 12.9.4 Dragging a linked value's knob on the canvas (criteria 25, 33, 34)

- **Hover.** Over a linked value's knob, its partners' knobs take the follower ground (`--accent-hover`, the Parameter handle Followers rule of Link corners): it says who will move. With Shift held during the hover they stay idle.
- **Coupled drag.** The dragged knob has the dragging look; partner knobs keep the follower ground and move live to their anchors for the new value; the extension lines and arrowheads of **every** member's dimension are drawn in `--dim-guide`. **One** readout chip at the pointer names all: `Link` 12px, 4px, "Radius 1 = Radius 2 40.0 mm" (three or more: "Leg 1 = Leg 2 + 1 more 40.0 mm"); " max" / " min" when a member's limit holds. No second chip at the partner: it may be off screen, and two chips crowd the knob.
- **Shift at the press** (only this). Partner knobs idle, their lines not drawn, the partners stay; the chip reads `Unlink` 12px, "Radius 1 40.0 mm". An already unlinked value's drag looks the same. This chip glyph is how the knob shows the unlinked state (criterion 34 "at its field and handle"); at rest and on hover the canvas draws no chain glyph beside a knob **(Proposal)**: the canvas stays clean, the field carries the state.
- **Snap.** Ctrl (Cmd) snaps whole mm or whole degrees (12.3); with Shift as well: only this, snapped.
- **Entry chip** (double-click on a knob). A linked value's chip shows the link mark after its label (the "Transform entry chip" chain glyph); Enter commits coupled; an unlinked value's chip shows the `Unlink` mark and writes only it.
- **Escape** restores every member's value and knob; the pill and marks never changed (12.9.3).

#### 12.9.5 Accessibility and keys

- **Sketch side.** Link: button with `aria-expanded`, `aria-controls`; options' names "Radius 2", "Group A: Radius 3, Radius 4". Linked line: plain text (no Tab stop), then the Unlink button. Tab order in a row: Expose, Name, Min, Max, Link or Unlink.
- **Clone side.** `aria-valuetext` in this order: value, ", sketch's value" when muted, then ", linked with Radius 2" / ", unlinked from Radius 2" / ", linked on some clones": "40 millimetres, sketch's value, linked with Radius 2". `aria-keyshortcuts` adds "PageUp PageDown". A partner's changed value is not announced (its field text carries it; the edited field already says it is linked).
- **No new global key.** Modifiers change in the Parameters row and on its knobs only (12.7).
- **Contrast.** The muted mark's letter is 11px text over the fill bar (`--value-fill`) when the label sits on the bar: check it reaches 4.5:1 there in both themes, else draw the letter at `--toolbar-icon`.

#### 12.9.6 Steps

"Link parameters", "Unlink parameters", "Change parameter" (coupled and Shift alike, one per gesture), "Back to the sketch's value" (relinks), "Back to the sketch's values" (Original's), and Stop exposing / Remove in their existing steps with the link change inside.

### 12.10 Review checklist for slice 2

Knob drag with Ctrl (Cmd) snaps whole mm and degrees, Shift no longer snaps; the pill and all marks keep their pre-gesture state for a field drag, an arrow key held, a knob drag and Escape, and change in the frame of the commit; coupled drag: partner field and knob move each frame, partner knobs in the follower ground, every member's extension lines drawn, one chip "A = B 40.0 mm"; Shift at the press: partner idle and unchanged, chip with `Unlink`, `Unlink` mark at commit, pressing Shift mid-drag changes nothing, a Shift gesture back to the old value and Escape unlink nothing; Shift on a grouped value is never coarse and shows "only this", on an ungrouped value it stays x10 with "coarse"; PageUp / PageDown step ten times; Shift+Enter commits coupled; Back on an unlinked value shows the muted sketch's value with `Link` and leaves the partners alone; the Link line is absent on Count rows and without a same-kind partner, the list offers only same-kind values and groups, across both tabs; Unlink of a group of two shows Link on both rows; Stop exposing names the link in its tooltip; mark never truncated at a 244px field with a long name; muted letter contrast on the fill bar; `aria-valuetext` order as 12.9.5; 200 clones: a coupled knob step within 16 ms (criterion 37).
