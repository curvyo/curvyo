# Measure and dimension: measure on the canvas and leave thin green dimensions that are guides

Status: Draft (a sketch for the customer; not Ready: it builds on the guide flag of the reworked `0046` and on text for the numbers)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "A measuring tool: I want to measure and dimension easily. Dimensions are drawn thin green by default and not printed or exported. It is however a guide line and can also be converted back to a normal path."

## User value

As a maker I want to check how long, how far apart or how big something is, and to leave the numbers on the canvas while I work, without those marks ending up in the cut or the print. When I want a mark as real geometry (an arrow, a measure line to be engraved on a template), I convert it back to a normal path.

**What other tools do.**
- **Inkscape**: a Measure tool that shows lengths and angles for one drag, with an option to convert the measurement to items; no persistent dimensions in core (the Dimensions extension is a Python script).
- **Illustrator**: Measure tool (Info panel only); no dimension objects except plugins.
- **LightBurn**: a measure tool (a ruler line with length and angle in the status area); no persistent dimensions.
- **SolveSpace, FreeCAD, Fusion**: dimensions are real constraints (see `0066`); a "reference" dimension only shows a value.
- **Affinity**: Measure tool with a guide-like overlay; keeps nothing.

## Words (reuses the guide of `0046`)

- **Guide:** a node with the register `guide` (ADR 0019), drawn in the marking style (green, dashed 2 px on / 2 px off) and not output. See the reworked `0046`.
- **Dimension:** a guide object made of a measure line (with end ticks or arrows), extension lines, and a number. It is **thin green** (the dimension style: solid, 1 screen pixel, green token `--guide`, not dashed, so a dimension reads differently from a guide outline; decided).
- **Measurement:** what the tool shows while the pointer drags; nothing is stored.

## Acceptance criteria (as far as clear)

1. Given the Measure tool (a symbol in the tool rail), then a drag from one point to another shows a live measurement at the pointer: length, angle and the x and y distances, in the document unit (`0015`), and does not change the document.
2. Given the Measure tool, then the top bar has: kind of dimension (Linear, Aligned, Angle, Radius, Diameter) as symbol buttons, "Leave dimension" (switch: a plain measurement is not stored, a left dimension is), decimals (number field), and units. Nothing appears in the right panel.
3. Given **Leave dimension**, then a click or a drag creates one dimension as a guide object (a group with the guide flag: lines and a text); it follows undo in one step "Dimension".
4. Given snapping (`0060`), then the ends snap to nodes, centres and midpoints of the shapes; holding Alt turns it off for the gesture.
5. Given a dimension kind:
   - Linear: horizontal or vertical distance between two points; Aligned: the true distance along the line between them.
   - Angle: the angle between two lines or three points, in degrees.
   - Radius or Diameter: on a click on a circle, an arc, or a rounded corner (`0013`); the label reads R or the diameter sign.
6. Given the number, then it is drawn with the font of the UI (Proposal; a single built-in font, not the maker's text fonts), at a size set in the top bar in millimetres, and placed at the middle of the measure line, draggable along the line. If text objects exist by then (`0062`), the number is one; if not, a numeral renderer with the digits, the decimal point, the minus, degree and diameter signs, as plain paths in the core, is enough (Question 2).
7. Given the default style, then a dimension is thin green and is **not** printed or exported: it is a guide and is ignored in SVG export (`0024`), G-code, job preview and PNG. A switch in the export settings that includes guides is not planned (no speculative options).
8. Given **Convert to path**, then the same Guide button of `0046` (the Select bar) switches a dimension to ordinary objects: the lines become paths with the stroke style of the guide and the number becomes outlines or a text object; one step. Press Guide again on a path to make it a guide: round trip.
9. Given the dimension is edited, then moving an end node (Node tool) moves the dimension's measured point but does not update the number unless it was left linked to the measured points (Question 1).
10. Given the units, then the number is rounded for display only; the true value is stored in millimetres (`CLAUDE.md` §5 units as types).
11. Given a hidden or locked layer, then the dimension follows it like any guide.
12. Given an object scaled afterwards, then a linked dimension updates; an unlinked one keeps its number.

## Relation to `0066`

In a constraint sketch a **driving dimension** (typed number moves the geometry) is the same dimension object with a constraint attached; a **reference dimension** is the measuring dimension of this entry. So this entry's dimension object must carry (1) two or three anchor points, (2) a kind, (3) an optional "driving" flag that `0066` adds. No solver here.

## Out of scope

- Tolerances, leader notes, tables and drawing sheets.
- Dimension styles (arrow shapes, text positions) beyond thin green and a text size.
- Automatic dimensions for the whole drawing.
- Showing dimensions in the output (see criterion 7).

## Decided (customer, 2026-10-17)

- **DIM-GREEN:** exactly as proposed: thin solid green for dimensions, dashed green for guides.

## Open questions (customer; each has a default)

Frage 1 (DIM-LINK): Does a dimension follow the measured objects?
- A (default, recommended): yes, linked to the nodes it measured (it updates when they move; unlinked when a node is deleted). B: no, a dimension is a snapshot of numbers and lines, as plain guides are.
- Default if unanswered: B for the first slice (nothing else changes), A when `0066` comes.

Frage 2 (DIM-TEXT): How is the number drawn before text objects exist?
- A (default): a small built-in numeral renderer in the core (about 16 glyphs as paths), replaced by the text object of `0062` later. B: wait for `0062`.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-045 (new); R-EDIT-041 (guides)
Depends on: `0046-document-shapes` (the reworked version: guide flag, Guide button, PR #141), `0060-snapping`, `0015-document-size-and-rulers` (units)
Related: `0066-constraint-sketcher`, `0062-fonts`, `0013-rectangle-corner-radii`
ADRs: ADR 0019 (the `guide` register) is extended by a dimension group; no new ADR
PR: -
