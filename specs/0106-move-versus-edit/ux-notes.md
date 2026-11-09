# 0106 Move versus edit: UX notes

ux-engineer, 2026-11-09. Checked against `specification.md` (criteria 1 to 23, Fragen 164 to 166 at default A), `specs/0100-bring-content-in/ux-notes.md`, `specs/0099-sketch-pages/ux-notes.md`, `specs/0098-page-kinds/ux-notes.md`, `specs/0050-layers-and-objects-panel/`, sketch i of `specs/0063-manufacturing-workbenches/concept-sketches.html` and `docs/design-system.md` ("Page kinds", "Properties panel: Layers tab", "Linked clone marks", "Bring content in", "Hidden, not disabled"). Customer rules applied: no operating tips or hint labels (2026-10-16), no notice after a success (2026-10-31), hidden not disabled, no popups. Every word below names a control or a state, or is a text the specification prescribes. No new colour token, no new component. Choices marked (Proposal) are vetoable at the demo.

## 1. Principle

Moving needs no mode, so nothing about moving looks different. What an object is shown to be is what differs: an out-of-context object has everything a move needs and nothing an edit needs. The absence is the whole cue; the one sentence of criterion 11 and one double-click are the whole way out. Nothing is dimmed, tinted, locked-looking or marked on the canvas, because the object is not locked and not wrong.

## 2. A selected out-of-context object (criteria 2, 4, 5, 7)

- **Box and handles.** The ordinary selection box of `0019`, exactly as in context: 1px dashed `--shape-handle-stroke`, eight resize handles, four corner rotate handles, side skew handles on Shift, the centre move handle, the hover box (1px `--hover-box`). Same positions, sizes, hit areas, snapping marks, live readout and entry chips (double-click a handle types a value; `M`, `R`, `S`, `K`, `H`, `V` work). The box is not styled differently for context: a mark would ask the maker to learn a second box.
- **Absent.** Everything that edits what the object is: primitive parameter handles (rectangle corner radii, polygon/star radius and inner ratio, ellipse arc ends and Curve), the text-on-path position handle, the text edit frame and caret, Offset handles, nodes and segment hover. A resize or rotate still keeps a primitive a primitive (criterion 5, test).
- **Clone marks stay.** A selected clone on the pasteboard or on a laser page draws its trace, original trace, root trace and tag as built (`0049`, `0100` section 6): they show a relation, they do not edit.
- **Cursor.** The Select tool's own cursors over the box and handles; over the object the move cursor of today. No padlock, no `not-allowed` (the padlock is for locked objects, `0050`).
- **Hover.** An out-of-context object under the pointer gets the hover box like any object. No tooltip, no hover cue about its page.
- **Home page label (Proposal).** While the selection holds out-of-context objects whose home is a drawing, laser or sketch page, that page's canvas label (`0098` section 5) is semibold, the rule a selected placed page's name tag already follows. It names where the object belongs without a word. A pasteboard object has no label to emphasise.
- **Status bar left and announcer.** Status bar: "<object> · <home>" with the home written as the Style line writes it ("Rectangle · Drawing 2"; pasteboard keeps "Circle · pasteboard"). Selection announcer: the object name and size as built plus ", on Drawing 2" (", on the pasteboard").
- **A selection of several homes or a mix (Frage 165 A).** One box around everything, move-only as a whole. No part of the box or its handles hints at which member is in context.
- **Sketch page active (criteria 3, 8).** A drawing object over or beside the sketch gets the same box; the sketch geometry keeps its fixed place and no box. Pressing the other kind replaces the selection, silently.
- **Clone of a sketch on the pasteboard (criterion 7).** Box with handles; the Style tab holds the line of section 3 and the clone block of section 6.

## 3. The Style tab line (criterion 11)

- **Place.** First and only content of the Style tab for a selection that is entirely out of context (Fill, Stroke, Markers, Spread colours, Placement, opacity and Guide hidden, not disabled). It sits where the first section starts, 8px below the panel header, no group border, no icon, no Tab stop.
- **Look.** One line of 12px `--panel-muted-fg` text (the look of the usage line in `0099` "Use group"), left-aligned, ellipsis, tooltip with the whole text (the page name in full), `role="note"`, not a live region (the announcer carries the selection change). Contrast as the existing muted lines. Dark theme: token only.
- **Words (at most 70 characters, the page name cut at 24 with an ellipsis).**

  | Selection | Line |
  |---|---|
  | all out of context, one home | "On Drawing 2. Double-click to edit." |
  | all out of context, several homes | "On 3 pages. Double-click to edit." |
  | in-context and out-of-context mixed, one other home | "Includes objects on Drawing 2. Double-click to edit." |
  | mixed, several other homes | "Includes objects on 3 pages. Double-click to edit." |
  | pasteboard, laser or sketch page active | "On Pasteboard. Double-click to edit." |
  | own item or clone of a laser page | "On Laser 1. Double-click to edit." |

  The longest case is 67 characters with a 24-character name. "Home" is the page whose activation would put the object in context (criterion 1; criterion 14 for the pasteboard, whose line still says "Pasteboard"). The second sentence is identical in every row: the gesture never changes.
- **Mixed selection.** The Style sections of the in-context objects are hidden too (Frage 165 A); the line is the only content. This is the one case where the line says "Includes".
- **Placed page and clone blocks.** A placed page is in context only on its laser page, so its block ("Placed page of Coaster.", `0100`) appears there and never beside this line. A selection that holds out-of-context clones shows the line first, then the "Linked clone" block reduced to the name row, Select original, Select root original and Select clones (they select, they do not edit); Placement, the Style rows and Unlink clone are hidden (Proposal).
- **Tab states.** The Style tab stays enabled for such a selection (it is not dimmed; it has content). With nothing selected nothing changes.
- **Select bar.** For a selection that holds an out-of-context object: the move commands of criterion 5 stay in their places (the groups keep their order, so a double-click that brings the edit buttons back moves nothing that was already there). Shown: typed X, Y, W, H and angle where the bar has them, Align, Distribute (with Gap), Flip, Linked clone, Place on…, and the "Scale stroke width" toggle. Hidden: the Guide toggle, the Link corners toggle, Radius, Points, Ratio and Curve fields, Object to path, Stroke to path, the Boolean, Path and Grouping toolboxes' buttons that sit in the bar; the Arrange in a grid group stays, because criterion 5 lists it as a move (lead, 2026-11-09). Nothing else on the bar is dimmed.

## 4. Switching context: double-click and Enter (criteria 13 to 16)

- **Gesture.** A double-click on the object, with Select or with any edit tool active. Hit tolerance, double-click timing and slop as built. The first press of the pair only selects (criterion 2); the second activates.
- **What happens in that press.** The home page becomes active in that frame: rail, tabs, header chip, canvas label, status bar and Style tab change in one render (`0098` "Page activation"), the panel tab follows `0098` criterion 9, the object stays selected and its Style tab now shows the sections of its kind. The tool stays; if the new rail lacks it, the new kind's Select tool, as built. The view does not pan or zoom. Nothing else: no group entered, no text edit, no node edit, no chip. Not a history step. Instant, no animation, nothing for reduced motion to change.
- **Cue.** The muted line of section 3 ("Double-click to edit.") is the only prompt, plus the semibold home label of section 2. No cursor change, no hover tooltip, no badge: the gesture is the one the maker already knows for entering a group. After the switch the line disappears because the object is in context. This is the whole of the "discoverable" requirement under the no-hints rule; if the customer wants more, that is a question for the lead.
- **The second double-click** behaves as built: group entered, text edited, Node tool shows nodes.
- **Focus and announcer.** Focus goes to the canvas. Polite announcement "Drawing 2, drawing page, 2 of 5" (`0098`), then the selection as built.
- **Pasteboard object (criterion 14).** With a laser or sketch page active, the last active drawing page becomes active. No drawing page in the project: no activation and the notice "There is no drawing page to edit this on." in the canvas notice slot, the refusal alert look (8s or until the next action, once per gesture). With a drawing page already active the object is in context and nothing here applies.
- **Own item or clone of a laser page L (criterion 15).** L becomes active. A clone double-clicked while L is active keeps `0049`/`0066` (Select original) and `0099` criterion 13 (a clone of a sketch opens its sketch page).
- **Placed page of D while L is active (criterion 15).** D becomes active, the topmost source object under the pointer is selected with the view change of Select original (none if it is in view, a pan to centre it if not, never a zoom). An empty frame area: D active, selection cleared. No notice (this replaces the `0093` notice).
- **Drawing object over a sketch page, sketch page active (criterion 16).** Its drawing page becomes active; rail, tabs and the Sketches tab follow `0098`.
- **Keyboard equivalent.** **Enter** with the canvas focused, the Select tool (or any tool in which Enter has no function of its own for the selection) and a selection that is out of context with exactly one home: it does what the double-click does, activates the home page and nothing else. The next Enter then acts as built (enter a group, edit a text). Same gates as every Enter: not in a text field, chip or drag. A selection with several homes: nothing is activated and the refusal notice of section 6 is shown. A mixed selection with one other home: that home is activated. It is the only keyboard route; `Ctrl+Page Down`/`Page Up` (`0098`) also activate pages but select nothing. The page row press and the Layers tree's rows are the other routes (section 5).
- **Escape.** Unchanged; it never undoes an activation.

## 5. The laser page's own Layers tree (criteria 17 to 20, Frage 166 A)

Same tab, same toolbar position, same list, same row component (`0050`); only the rows differ. Document layers, "Initial Layer" and layer rows do not appear on a laser page.

- **Root row.** The page: name (14px, semibold), the Laser chip (`Zap` glyph and word, 11px, as the page row), no eye, no lock, no chevron, not selectable, not draggable. It is the grid's first row and is not scrolled out (pinned); the rows below are the virtualised list.
- **Row order.** The order the canvas draws, topmost first: placed pages and own items interleaved, then the **Fixtures** row last, always.
- **Own item and group rows.** As `0050`: kind glyph, name, opacity readout, eye, lock, row action, rename (F2 or double-click on the name), drag reorder inside the page, Delete. A group is expandable with its members as built.
- **Clone rows.** `Link2` glyph, name "Linked clone of <original name>", the `Locate` "Select original" row action, eye, lock, not expandable (`0100`). The Delete and drag rules of own items.
- **Placed page rows.** Glyph `Frame` 16px, name "Placed page of <name>" (the source name cut at 40 characters, tooltip whole, second muted tooltip line "Live, read-only"), chevron. No eye, no lock, no opacity readout, no row action; the three slots stay empty so the columns align (nothing reflows). Delete removes the reference. The row cannot be dragged to another laser page and not reordered against own items (criterion 19; the cursor is `not-allowed` over a refused target, no text, no notice).
- **Expanded placed page.** Inner rows are the source page's members, live, one indent step deeper, glyph and name at 60 % (`--panel-muted-fg` name), no eye, no lock, no action, no drag, no rename. They are not selectable as such: a press on an inner row selects the placed page and scrolls it into view, the inner row gets no selected ground (only the placed page row does). Hover on an inner row: the hover ground, no cursor change. Deep groups inside a source page are expandable as usual, the same dimming. A source page with no members expands to one muted row "Empty" (14px `--panel-muted-fg`, no glyph, not focusable).
- **Fixtures row.** `Grid3x3` 16px, name "Fixtures", the eye only, the same state as the Fixtures block eye of the Document tab (`0098` criterion 13); no lock, no chevron, no selection ground, no rename, no drag, no Delete. A press on the row selects nothing and does nothing but take focus; it is the row `0102` will fill.
- **Selection marks.** A selection on the canvas marks its rows (`--value-fill` ground, 3px `--accent` bar) and expands to them, collapsed containers open and the first marked row scrolls into view, the `0050` rule.
- **Layer state.** The eye and lock of the document layer an object lies on still apply. The row shows them as inherited state: calm for the object's own, a `EyeOff` or `Lock` in `--panel-muted-fg` inside the 1px dotted ring with the description "Hidden by layer Cut" when the layer decides (Proposal, `0050` "Eye, lock"). No layer name appears anywhere on a row.
- **Rows of another page on a drawing page (criterion 17).** A drawing page's Layers tab lists them as built; the row looks like any row, its tooltip gets a muted second line "On Laser 1". A press on such a row selects the object as out-of-context (criterion 2); no page change. A double-click on its name renames, as built (a double-click for context stays a canvas gesture; F2 and rename of an out-of-context object are an edit command, so they are refused with the notice of section 6, Proposal).
- **Toolbar on a laser page.** `Copy` "Duplicate", `ArrowUp` "Move up", `ArrowDown` "Move down", `Trash2` "Delete". "New layer" (`LayersPlus`) is hidden, not dimmed: there is no layer to add. Buttons that do not apply to the selection stay dimmed with their Tab stop, per `0050`. The Opacity slot stays at the bottom, 64px, reserved; it applies to own items and clones (caption "3 objects"), is empty for a placed page (no opacity) and for an empty selection.
- **Empty page.** No placed pages and no own items: between the root row and the Fixtures row one muted line, "Nothing on this page yet." (14px `--panel-muted-fg`, no glyph, not focusable). The Fixtures row always shows, so the list is never blank.
- **Semantics.** `role="treegrid"` named "Layers and objects" (unchanged), one Tab stop, roving focus; `aria-level` true depth (root 1, rows 2, inner rows 3 and deeper), `aria-posinset`, `aria-setsize`, `aria-rowindex`, `aria-rowcount` of the whole tree, `aria-selected`, `aria-expanded`. Row names: "Rectangle, own item", "Linked clone of Path 3f9a1c", "Placed page of Coaster, 50 objects, read-only", inner "Rectangle, in placed page, read-only", "Fixtures, shown". Inner rows are `aria-readonly="true"`. Keys as `0050`; Alt+arrows reorder own items and clones only; Enter or Space on a placed page row selects it. At most 60 row elements in the DOM, the focused row's element kept when it scrolls out, scrollbar exact from the first frame, also with all 20 placed pages expanded. The tree is built from the ADR's page-tree read; the first rows paint within 100 ms (criterion 20).
- **Large sources.** Expanding a placed page of 50 objects adds 50 virtual rows, never 50 elements.

## 6. Edit tools on foreign objects (criteria 9, 10)

- **Rule.** An edit tool draws nothing on an out-of-context object and creates only in the active page's context. No tool is switched away, no message, no cursor change.
- **Node tool.** No nodes, handles or segment hover on the object. A press on its outline selects it as Select does (the box with handles shows; the Node tool stays active). The Node bar keeps its frame but shows none of its node commands for this selection (hidden, as for a selection with no editable path). Hover: the object hover box, not the node/segment hover. A clone on the active laser page keeps the `0049` criterion 26 sentence as built.
- **Text tool.** A press on an out-of-context text is a press on empty canvas: it starts a new text at the pointer, in the active page's context (Proposal in the spec). No caret on the foreign text, no hover cursor that suggests editing it (the Text tool's I-beam is the tool's own cursor everywhere).
- **Pen, shape tools, Pencil.** Create in the active page's context. A press near an out-of-context path's end node does not join, extend or show the end-node target (the open-path hover ring of the Pen is not drawn); node snapping to it stays and shows the snap marker as `0060`.
- **Offset, Trace, Tiling, Measure.** An out-of-context object is not an operand: Offset shows no corner handles and no entry chip on it; Trace and Tiling take no operand from it; the Measure tool picks points as today. Their bars show their usual empty state.
- **Chords reachable with an out-of-context selection.** Ctrl+G, Ctrl+Shift+G, Ctrl+Alt+C, Home, End, Page Up, Page Down, F2 in the Layers list, Enter with several homes: no change, no history step, and one notice (below). Buttons that carry these commands are hidden (section 3), so the notice exists for keys only. Delete, arrows, Ctrl+D, Ctrl+C, Ctrl+X and Ctrl+V are move commands and work as in context. Ctrl+Z is as built.
- **Refusal notice.** Canvas notice slot, the refusal alert look of `0100` (validation chip look, `role="alert"`, 8s or until the next action), at most 70 characters, the home written as the Style line writes it:

  | Case | Text |
  |---|---|
  | all out of context, one home | "Not editable here: on Drawing 2." |
  | several homes | "Not editable here: on 3 pages." |
  | mixed, one other home | "Not editable here: includes objects on Drawing 2." |
  | mixed, several homes | "Not editable here: includes objects on 3 pages." |

  Shown once for a run of repeated keys (a held key, a second chord) until another action or selection change; never by a press, a hidden button or a tool (criterion 11 of the specification keeps the muted line as the only text for tools). It does not tell the maker what to do: the Style line does. The notice of criterion 14 ("There is no drawing page to edit this on.") is the other text of this slice. No success notice anywhere: a switch, a move and a Layers press say nothing.

## 7. Keyboard and focus

| Action | Route | Notes |
|---|---|---|
| Select an object in any context | press, Shift+press, marquee (`0007`); Layers tree: arrows then Enter or Space | selection never changes the active page (criterion 2) |
| Select all | Ctrl+A | in-context objects of the active page only (criterion 3) |
| Move an out-of-context object | arrow keys, Shift+Arrow, typed X, Y, W, H, angle, Align, Distribute, Flip | as in context; focus stays on the canvas |
| Switch to the object's context | Enter (canvas focused), or double-click | section 4; activates the home page, nothing else |
| Activate a page without an object | page row press, `Ctrl+Page Down`, `Ctrl+Page Up` | `0098`, unchanged |
| Cancel a drag | Escape | unchanged |

- **Focus after:** a context switch, a refusal, a move: the canvas. A Layers tree press: stays in the list (selecting does not switch the page). A press on a placed page row selects and keeps focus in the list.
- **Tab order of the Layers tab (laser):** toolbar, tree (one stop), Opacity field. Document tab and the rest as `0098`/`0100`.
- **No new global chord.** Enter is the existing key of "Enter a group" and "Edit a text", used the same way (activate first, act second).
- **Reduced motion:** nothing animates in this slice.

## 8. Empty, error and large states

- **No selection:** nothing of this slice shows.
- **No drawing page:** a pasteboard object double-clicked with a laser page active gives the criterion 14 notice and nothing else.
- **Objects hidden or locked:** a locked object cannot be selected by a press as built (`0050`); a hidden one is not there. A locked object in a selection (via the tree) is not moved, as built.
- **Large documents:** classification is session-side (criterion 22): no per-object work in the render frame beyond the box; the Style line is computed once per selection change. 1,000 out-of-context objects selected show one box and one line. Marquee over page borders selects across contexts and ends in a move-only box; no per-object feedback is drawn during the marquee.
- **Dark theme:** tokens only (`--shape-handle-stroke`, `--hover-box`, `--panel-muted-fg`, `--value-fill`, `--accent`, `--toolbar-icon`). Contrast: the 12px muted line uses `--panel-muted-fg` on the panel ground, as the existing usage and caption lines; the 60 % inner rows must stay at least 4.5:1 for the name, otherwise 70 % (check both themes).
- **Touch (`pointer: coarse`):** rows 40px, hit areas as `0050`; a double-click is two taps within the system interval; Enter is not available without a keyboard, so touch has the double-tap and the Layers tree only (no hint).

## 9. Review checklist for the implementer

An out-of-context selection draws the ordinary box and handles and no parameter, text or node affordance; Style tab holds only the line (and the reduced clone block), at most 70 characters, name cut at 24, "On"/"Includes" forms per table; Select bar keeps the move groups in place and hides (not dims) the edit buttons including Guide and Link corners; first double-click activates only (group not entered, text not edited), second behaves as built; Enter does the same; the activation is not a history step and not a view change; pasteboard double-click with no drawing page gives its notice; home label semibold while selected; Node tool shows no nodes on a foreign object and its bar is empty of node commands; Text tool starts a new text over a foreign text; Pen never offers a join target on a foreign path; keys Ctrl+G, Ctrl+Alt+C, Home, End, Page Up and Down, F2 post the refusal once and write nothing; the laser Layers tree has the pinned root row, interleaved placed pages and own items topmost first, Fixtures last with only an eye; placed page rows have no eye, lock or action and expand to dimmed read-only rows whose press selects the placed page; inherited layer eye/lock shown dotted; "New layer" hidden on laser pages; 60 row elements at most with 20 expanded placed pages; contrast of the muted line and the 60 % rows in both themes; screen reader reads the line, the home, the tree levels and each refusal.
