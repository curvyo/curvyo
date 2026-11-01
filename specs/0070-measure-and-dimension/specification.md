# Measure and dimension: measure on the canvas and leave green dimensions that are guides

Status: In build (story/0070-measure-and-dimension). UX notes and `adrs.md` exist; ADR 0027 Accepted (customer, 2026-11-01). Builds on the guide flag of the reworked `0046`.
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "A measuring tool: I want to measure and dimension easily. Dimensions are drawn thin green by default and not printed or exported. It is however a guide line and can also be converted back to a normal path." Customer, 2026-11-01: "nice, UX/UI consistent and above all working". Items marked (Proposal) are the product owner's or the UX engineer's ideas, vetoable at the demo.

## User value

As a maker I want to check how long, how far apart or how big something is, and to leave the numbers on the canvas while I work, without those marks ending up in the cut or the print. When I want a mark as real geometry (an arrow, a measure line to be engraved on a template), I convert it back to a normal path. Dimensions look and behave like the Sketch dimensions of `0066`, so I learn one thing.

**What other tools do.**
- **Inkscape**: a Measure tool (key M) that shows lengths and angles for one drag, with an option to convert the measurement to items; no persistent dimensions in core (the Dimensions extension is a Python script).
- **Illustrator**: Measure tool (Info panel only); no dimension objects except plugins.
- **LightBurn**: a measure tool (a ruler line with length and angle in the status area); no persistent dimensions.
- **SolveSpace, FreeCAD, Fusion**: dimensions are real constraints (see `0066`); a "reference" dimension only shows a value.
- **Affinity**: Measure tool with a guide-like overlay; keeps nothing.

We do better: dimensions stay linked to the geometry (they follow when it moves or scales), look like the Sketch dimensions, and turn into plain paths in one click.

## Words (reuses the guide of `0046`)

- **Guide:** a node with the register `guide` (ADR 0019), drawn in the marking style (green, dashed 2 px on / 2 px off) and not output. See the reworked `0046`.
- **Dimension:** a dimension object (a guide by kind, ADR 0027) with the look of the Sketch dimensions of `0066` (extension lines, a dimension line with arrowheads, the text breaking the line; "R 25 mm", "⌀ 50 mm", "45°"; sizes constant on screen at any zoom), drawn in guide green (token `--dim-guide`, an alias of `--guide`), solid, never output.
- **Measurement:** what the tool shows while the pointer drags, and the pinned result after the second pick; nothing is stored.

## Acceptance criteria

1. Given the Measure tool (a button in the tool rail, criterion 2), when I drag from one point to another (or click one point, then another), then a live readout chip at the pointer shows length and angle on the first line and the x and y distances on the second, in the document unit (`0015`), and the document does not change (undo history length unchanged). After the second pick the measurement stays pinned until the next press, Escape, a tool change, an undo or redo, or any document edit; pan and zoom keep it.
2. Given the tool rail, then Measure is the second button of the Path-tools card (below Offset; the card's accessible name becomes "More tools"), with the tooltip "Measure tool (M)" ("Measure tool (Esc, M)" while Select has a selection), and the letter **M** activates it in every state except Select with a selection (there M keeps opening the move entry). Given the Measure tool is active, then there is **no top bar, no hint line and nothing in the right panel**. The tool name is the pressed rail button; before the first pick the pointer is a plain crosshair with no chip or label, like the Rectangle tool (customer, 2026-11-01); after the first pick a mode label at the pointer names the current kind (as in `0066`). A right click does nothing in the Measure tool. Values show with up to 3 decimals, trailing zeros trimmed, in the document unit; there is no decimals field and no unit picker.
3. Given the tool, then it is **one smart tool**: the kind is inferred from the picks and the pointer placement, never chosen in a bar. The kinds match `0066`: a curve pick (circle, arc, rounded corner `0013`) gives **Radius**; two points give **Aligned**, **Horizontal** or **Vertical** by where the pointer is relative to the box around the two points (above or below: Horizontal; left or right: Vertical; inside or in a corner quadrant: Aligned; a pure function of the two points and the pointer, unit-tested for its eight regions); one segment pick then placement gives the **Aligned** length of that segment; a segment pick followed by a second, non-parallel segment pick gives **Angle** (the sector under the pointer chooses which of the four angles). The mode label names the kind before the commit.
4. Given the tool, when I hold **Shift at the pick of the second anchor** (the release of the drag or the second click; for a curve pick, the pick of the curve), then the gesture leaves a dimension: the dimension follows the pointer, and the next click places it as one dimension object (a guide by kind); undo removes it in one step "Dimension". Without Shift the gesture stays a measurement (criterion 1). Shift is read live from the first pick on: pressing or releasing it swaps the blue measure drawing and the readout chip for the green dimension preview and back in the same frame, and the label shows "Dimension, " plus the kind. Escape cancels the gesture. Shift never locks an axis here; Ctrl locks the direction from A to B to the Pen's angle stops. Pointers without a keyboard cannot leave a dimension in this slice.
5. Given exactly one eligible object is selected before the tool is used (selection first, as in `0066`): a circle or arc (Radius), or a straight segment or two-node open path (Aligned length), and the pointer is over empty canvas, then the dimension preview follows the pointer and a click places the dimension at once, without picking anchors and without Shift; Enter places it at the default position (24 px offset). A press on a pick source (a snap point, a curve, a segment) starts a normal pick instead. The selection does not change.
6. Given snapping (`0060`), then the anchors snap to nodes, centres and midpoints of the shapes. **Alt has two jobs separated by phase.** While picking an anchor or curve, holding Alt turns snapping off for the gesture (and on a curve near a node picks the outline instead of the node). While placing a Radius or Diameter, pressing Alt (keydown, once per press; not with Ctrl, Cmd or Shift down; not when Alt was already held at the pick) switches between Radius and Diameter; the default is Radius. In any other placement Alt does nothing. The placement of a dimension line is never snapped.
7. Given a dimension of any kind, then it is drawn like a Sketch dimension of `0066`: two extension lines, a dimension line with arrowheads, and the text breaking the line; texts read "12.5 mm" (in the document unit), "R 25 mm", "⌀ 50 mm", "45°"; line weight 1 px, arrowhead size and text size are constant on screen at every zoom; all in `--dim-guide`, lines solid. A Vector dimension is never hidden by the small-span rule of `0066`. A dimension of the same measured value drawn by `0066` (as a reference) and by this tool differs only in colour.
8. Given the text, then it is drawn with the shared dimension text formatter and label rendering that the Sketch dimensions of `0066` use (DIM-TEXT A); there is no separate numeral renderer. The text is placed at the middle of the dimension line and can be dragged along the line and across it (the offset) with the Select tool; one history step "Move dimension".
9. Given a dimension, then it is **not** printed or exported: it is ignored in SVG export (`0024`), G-code, job preview and PNG; the SVG and G-code of a document with dimensions equal those of the same document without. No export switch that includes guides (no speculative options).
10. Given a dimension is selected, then the Guide button of `0046` (the Select bar) shows pressed; pressing it **converts the dimension to ordinary objects** in one undo step named "Convert to path": lines and arrowheads become paths in guide green with a 1 screen-pixel line width at the current zoom, the text becomes an editable text object (not outlines; Convert to path of a text already exists), and the new objects are selected. Pressing Guide again on those paths makes them plain guides ("Guide"): round trip. The round trip returns plain guides; it does not rebuild the linked dimension.
11. Given a dimension is linked (DIM-LINK, decided), then it stores the nodes or geometry it measured; when one of them moves, or the object is scaled, rotated or edited with the Node tool, the dimension line and its number update in the same frame as the geometry (live during the drag), without a click, as part of that gesture's one history step. 200 dimensions follow a drag at the frame rate of a 200-object move (acceptance test).
12. Given a measured node or object is deleted (Delete, Cut, Boolean, Node delete, join, Break apart, Convert to path of a shape, clone unlink; or a non-uniform scale of a measured circle), then the dimension becomes **unlinked** in the same history step: it stays where it is and keeps its last number; one undo restores object, link and dimension. An unlinked dimension is drawn with its extension and dimension lines dashed 4 on / 3 off and a 10 px `Unlink` glyph before the text; the text stays "12.5 mm" without parentheses; colour is unchanged (dash and glyph carry the state); its accessible name ends "not linked".
13. Given an unlinked dimension, then it does not update when other geometry moves. Moving its own kept points with the Node tool does not update the number (the number is the last measured value); that move is the history step "Edit nodes".
14. Given the units, then the number is rounded for display only (3 decimals, trailing zeros trimmed); the true value is stored in millimetres with unit newtypes (`CLAUDE.md` §5).
15. Given a hidden or locked layer, then the dimension follows it like any guide. Locked objects can still be measured; hidden objects and existing dimensions are not pick targets.
16. Given the model (ADR 0027, pending DIM-STORE), then (1) the dimension object carries two or three anchor links, each a reference to a node and anchor or shape feature, or a free point; (2) a kind; (3) the kept last number, refreshed whenever the geometry commits. A Vector dimension and a reference dimension of `0066` **share kinds, layout, look and text**; there is no driving flag until a dimension drives plain geometry (that needs a new ADR). No solver here. The file format version becomes 31 (provisional) and older builds refuse such files.
17. Given the Layers panel, then a dimension has a row "Dimension 12.5 mm" (with "not linked" appended when unlinked) that can be selected and deleted from the keyboard; the selection announcer says the same. Delete of a selected dimension is the history step "Delete".

## Relation to `0066`

In a constraint sketch a **driving dimension** (typed number moves the geometry) is a constraint in the sketch graph (ADR 0026); a **reference dimension** shows a value. Vector dimensions are a separate object kind (ADR 0027) and do not reference sketch entities. Look, kind names, snapping rules, number formatting and label rendering are shared (criteria 3, 7, 8). The on-canvas entry box of `0066` is not needed here because nothing is driven.

## Out of scope

- Tolerances, leader notes, tables and drawing sheets.
- Dimension styles (arrow shapes, text positions) beyond the `0066` look; a text size setting.
- Automatic dimensions for the whole drawing.
- Showing dimensions in the output (see criterion 9).
- Typing a number to drive geometry (that is `0066`); a driving flag on Vector dimensions.
- A decimals field, a unit picker, a top bar or a hint line for the tool.
- A mode toggle for leaving a dimension (Shift only, see MEAS-KEEP).
- Rebuilding a linked dimension from converted paths.
- Touch or keyboard-less ways to leave a dimension (to be recorded in `docs/technical-debt.md` if accepted).

## Decided (customer, 2026-10-17 and 2026-10-31/11-01)

- **DIM-GREEN:** thin green for dimensions, dashed green for guides. The earlier detail "1 px solid, end ticks" is replaced by the `0066` look (DIM-LOOK).
- **DIM-LINK: A.** A dimension is linked to the nodes/geometry it measured and updates when they move or scale; it becomes unlinked (keeps its last number) when a measured node is deleted.
- **DIM-LOOK: A.** Vector dimensions look like the Sketch dimensions of `0066`, in guide green, never output.
- **DIM-TEXT: A.** The text uses the shared rendering of the `0066` Sketch dimensions; no separate numeral renderer.
- Consistency: kinds and interactions follow `0066` (customer: "nice, UX/UI consistent and above all working").

## Decided (customer, 2026-11-01): DIM-STORE, MEAS-RAIL, MEAS-KEEP

- **DIM-STORE: A.** A dedicated `dimension` object kind that references the measured nodes and resolves on read, with the kept last number (file format 31, ADR 0027).
- **MEAS-RAIL: A.** Second button of the Path-tools card, key M; the rail column scrolls under Tools in the smallest window.
- **MEAS-KEEP: A.** Hold Shift at the second pick; no toggle control.

## Open questions (defaults apply if unanswered)

- Proposals not yet accepted: the placement rule of criterion 3 (SolidWorks-style, eight regions), 3 decimals trimmed in the readout, and Enter to place on a selection. Default if unanswered: as written; vetoable at the demo.

## UX notes

See `ux-notes.md` (ux-engineer, 2026-11-01): rail entry and icon `RulerDimensionLine`, gesture anatomy, readout chip, mode label, Shift, snapping cues, linked and unlinked look, Alt phases, keyboard and history names, tokens (`--dim-guide`).

## Links

Requirements: R-EDIT-045 (new); R-EDIT-041 (guides)
Depends on: `0046-document-shapes` (the reworked version: guide flag, Guide button, PR #141), `0060-snapping`, `0015-document-size-and-rulers` (units), `0066-constraint-sketcher` (dimension look, text rendering, kind names)
Related: `0062-fonts`, `0013-rectangle-corner-radii`
ADRs: ADR 0027 linked dimension objects (Accepted, customer 2026-11-01), `docs/adr/0027-linked-dimension-objects.md`; ADR 0019 (guide filters) and ADR 0015/0022 (references) are reused; see `adrs.md`
PR: -
