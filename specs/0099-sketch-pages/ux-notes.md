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
