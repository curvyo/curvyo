# Pen angle snap and typed values: Ctrl locks the angle of the next node, a key opens a Length and Angle entry

Status: Ready (2026-10-19; adrs.md of the architect and the PO draft of the UX notes below; the UX notes are to be confirmed by the ux-engineer; the four open questions are decided, see "Decided by the customer 2026-10-19"; criteria marked (Proposal) can be vetoed by the customer at the demo)
Priority: Should (pulled forward at the customer's request, 2026-10-19)
Origin: Customer (request of 2026-10-19: Ctrl snaps the angle in the Pen; typed values while drawing). The entry key (Tab) is accepted; the drag readout and the notices are Proposals, marked as such.

## User value

As a maker who draws parts for a laser cutter, I want to hold Ctrl in the Pen so the next segment locks to horizontal, vertical, 45 degrees and the other angle stops, and I want to type the length and the angle of the next segment, so that a 40 mm horizontal edge or a 25 mm line at 30 degrees is exactly what I meant, without drawing it by eye and fixing it afterwards.

The customer's request (2026-10-19, German, translated by the lead): "In the Pen tool, Ctrl must snap the angle, otherwise one cannot draw horizontal and vertical lines reliably. It would also be great to enter values directly, maybe by pressing 'e' (or similar) while dragging. I want this story refined fully and pulled forward."

**Field reference.**

- **Inkscape**: in the Pen (Bezier) tool Ctrl snaps the angle of the new segment, and of a dragged handle, to steps of 15 degrees (the step is a preference). Nothing can be typed while drawing; exact values are typed later in the XML editor or the node fields.
- **Illustrator**: Shift constrains a new anchor to multiples of 45 degrees; no numeric entry while drawing (the Transform panel comes afterwards).
- **LightBurn**: lines are drawn with the Pen and edited with the numeric edit fields afterwards; no angle lock in the Pen that this product's makers would call reliable.
- **CAD sketchers** (Fusion 360, AutoCAD dynamic input): after the first point, Tab opens Length and Angle boxes on the line, Tab switches, Enter places the point and the next segment starts. This is the interaction the customer describes.

**Where we do better:** the lock uses the angle stops the maker already knows from rotating and skewing (`0008`), including 22.5 degrees, in one table for the whole product. The typed entry is the same kind of chip as everywhere else (`0052`), shows its unit in every field, previews the segment before it is placed, and leaves the Pen drawing, so a polyline of exact segments is "Tab, 40, Tab, 0, Enter, Tab, 25, Tab, -90, Enter, ...".

## What exists today

Facts from the specs and the code on `main` (2026-10-19); the implementer verifies against the code.

| Item | Today |
|---|---|
| Pen, Shift and Ctrl | No new meaning in the Pen (`0010` criterion 39). Since `0034` Shift is the alternative at three targets (continue, join, close); Ctrl and Alt have no meaning at a target (`docs/design-system.md`, shortcut table). `0034` keeps Ctrl "for angle constraint in the Pen, which the product does not have yet". |
| Placing a node | A click places a corner node at the pointer; a click-drag places a node with symmetric handles along the drag (`0002` criteria 1, 2). The rubber band runs from the last node to the pointer. The path in progress is not in the document until it is finished (`0002` criterion 3, `0034` criterion 5). |
| Angle stops | One table: the union of the multiples of 15 degrees and of 22.5 degrees through all quadrants, nearest stop wins (`0008` criteria 33, 34; `curvyo-ui-core/src/angle_snap.rs`, `snap_angle`). Used by the Ctrl rotate, the Ctrl skew, the polygon and star create angle (`0010` criterion 4) and the arc handles (`0021` criterion 7). |
| Angle convention | Degrees, clockwise on screen, 0 degrees = straight right, 90 degrees = straight down, -90 degrees = straight up, shown in -180 to 180 (`0010` criteria 1 to 3, ADR 0002 section 4: Y grows down). |
| Entry chip | The placement chip of `0052`: label inside the field, unit suffix inside the field, Tab cycles, Enter places, Escape cancels, the first edit freezes the pointer, typed values preview, a length accepts the unit suffixes mm, cm and in (`PlacementChip.tsx`, `shape_placement.rs`, `parse_length`). |
| Key gate | Letters are ignored while the Pen has an unfinished path; Enter finishes the Pen path and is never gated; Ctrl+Z is ignored with an unfinished path (`docs/design-system.md`, "Keyboard concept"). |

## Terms

- **Last node L**: the most recently placed node of the path in progress, new or continued (`0034`: for a continuation the end node E until a new node is placed). The Pen has a last node as soon as one node is placed.
- **Raw point**: the pointer's document position.
- **Segment angle**: the angle of the vector from L to a point, in the convention of the table above (0 degrees right, clockwise positive, Y down).
- **Stops**: the table of `snap_angle`, measured from the x axis (absolute), not from the previous segment.
- **Constrained point**: L plus the vector of length |raw point - L| at the stop nearest to the raw segment angle. The length is kept, only the angle changes.
- **Target**: as `0034` (a continue, join or close target within 16 px of the raw pointer).
- **Entry key**: the key that opens the Pen entry chip. Tab (customer decision 2026-10-19).
- **Pen entry chip**: the chip of Part B. **Freeze**: the first edit of any field, as in `0052` criterion 14.
- **Display unit**: the document's length unit (`0015` Part C); angles are degrees.

## Changes to accepted behaviour

The lead names each at the demo; the owning specs are corrected in the PR that builds this slice.

| Spec and criterion | Today | After this spec |
|---|---|---|
| `0010` criterion 39 ("Shift and Ctrl have no new meaning" in the Pen) | Ctrl does nothing in the Pen | Ctrl locks the segment angle once the path in progress has a last node (criterion 1). Shift keeps its meaning at the three targets of `0034` and none elsewhere |
| `0034` Out of scope ("Ctrl is kept for angle constraint in the Pen") and the shortcut-table sentence "Alt and Ctrl have no meaning at a target" | Reserved | The reservation is used; "no meaning at a target" stays true (criterion 5) |
| Keyboard concept: Pen with an unfinished path ignores keys | Tab is not handled by the canvas | While the Pen has an unfinished path and the canvas has focus, Tab is consumed: it opens the Pen entry chip, or does nothing with a button held (criteria 11, 14) |
| Right-click in the Pen with a path in progress | The press goes to the Pen like a left press | Finishes the path as Enter does, places no node (criterion 19a, customer 2026-10-19) |
| Escape cascade (`0010` criterion 42) | The Pen path is step 3 | An open Pen entry chip is step 1 and closes alone (criterion 22) |

## Acceptance criteria

All lengths are document millimetres in a document whose display unit is mm, X to the right, Y down, unless a criterion names another unit. Zoom 100 % unless stated. Lengths and angles are compared with the document's geometric tolerance. Ctrl is Cmd on macOS. Angles in examples are in the convention of the table above.

### Part A: Ctrl locks the angle

1. Given the Pen with a path in progress that has a last node L, no target under the pointer, and Ctrl held, when the pointer moves, then the rubber band runs from L to the constrained point, in the frame of each pointer event. Examples at 100 %, L = (100, 50): pointer (130, 52) (raw angle 3.8 degrees) gives the constrained point (130.07, 50.00), segment angle 0; pointer (100.4, 80) (raw angle 89.2 degrees) gives (100.00, 80.00), segment angle 90; pointer (112, 38) (raw -45 degrees to within 0.1 degrees) gives angle -45 and a point whose x offset and y offset from L have equal size within the tolerance; pointer (140, 62) (raw 16.7 degrees) gives the stop 15 degrees. Without Ctrl the rubber band ends at the pointer as today.
2. Given criterion 1, when the maker presses the primary button without dragging and releases (a click) with Ctrl held in the press event, then the node is placed at the constrained point of the press, as a corner node, exactly as a click at that point places it. Example: L = (100, 50), Ctrl-click at (130, 52): the new node is at (130.07, 50.00) and the segment from L is horizontal (both ends have the same y within the tolerance). Three Ctrl-clicks drawn "right, down, left" with pointers off by up to 3 degrees give a path whose first and second segments are exactly horizontal and vertical and whose third is horizontal.
3. Given a click-drag in the Pen, then the node's position is fixed at the press: it is the constrained point if Ctrl is held in the press event, the raw point otherwise. While the button is held, Ctrl is read live and snaps the **handle direction**: the angle of the vector from the new node N to the pointer (the symmetric handles' direction) is replaced by the nearest stop, absolute, and the handle length stays |pointer - N|. Releasing Ctrl mid-drag returns the handles to the pointer and never moves N; pressing Ctrl mid-drag snaps the handles only. The committed handles are those of the last pointer event, the same function as the preview. Example: N = (130, 50), pointer during the drag at (150, 52) with Ctrl held: the handle points straight right (angle 0), its length is 20.10, so the outgoing handle ends at (150.10, 50.00) and the incoming one at (109.90, 50.00).
4. Given the Pen, Ctrl held, and a path in progress, when the maker presses or releases Ctrl with the pointer at rest, then the rubber band changes in the next frame (a live read, `0014`). The press reads Ctrl from its own event, and the cue is redrawn in the press frame. On WebKitGTK the Ctrl key-up may not be delivered (the Shift case of `0014` plan.md): the rubber band then keeps the snap until the next pointer event, and a press reads the state of its own event, so the node is placed where the preview last showed it or, if Ctrl was released, at the pointer. Known, not a blocker.
5. Given a target under the raw pointer (a continue, join or close target of `0034`, within 16 px), then Ctrl changes nothing there: the rubber band ends on the target's centre, the cue, the chip, the cursor and the press are those of `0034`, with or without Ctrl, and a Ctrl-click on the first node closes the path. The target test uses the raw pointer, before the snap. Given Shift held over a continue or join target (the plain node alternative of `0034` criterion 7), then the node is placed as anywhere else, so Ctrl snaps it. Over the close target Shift keeps flipping sharp and smooth (`0034` criterion 15) and Ctrl has no effect.
6. Given Ctrl and Shift held together away from every target, then the result is that of Ctrl alone. Given Ctrl held and Shift pressed or released over a target, then `0034` criteria 1, 7 and 16 apply unchanged (the chip follows Shift, Ctrl is ignored).
7. Given the Pen with no node placed yet (the first node of a new path), then Ctrl has no effect: the click places a node at the pointer. Given a continuation (`0034`), then L is the end node E, and Ctrl snaps the first new node relative to E. Given a click that is the first node and Ctrl was held, then the next node is snapped relative to it as in criterion 2.
8. Given a click-drag in the Pen (criterion 3), then a readout at the pointer shows the length and the angle of the handle (the vector from N to the pointer), in the display unit and degrees, with units: "30.1 mm, 0°" (the formatter and placement of the create-drag readout of `0010` criterion 6: one decimal for mm, two for cm, three for in; the angle one decimal at most, "22.5°"). It exists only while the button is held, with or without Ctrl, shows the snapped values when Ctrl is held, is text only and takes no focus. With the button up there is no readout: Ctrl at rest changes only the rubber band, and the numbers of the segment are in the entry chip (criterion 15).
9. Given a pen on a drawing tablet, a mouse or touch input, then the pointer kind changes nothing in this part: the modifier comes from the keyboard as for a mouse (`0032` criterion 5: a pen in range moves the pointer as a mouse does). A maker with a tablet and no keyboard has no route to Ctrl; there is no on-screen switch and no snap toggle (customer decision).
10. Given the Pen is idle with 5000 open paths in the document, then the hover cost of `0034` criterion 24 (under 2 ms per pointer move) is unchanged; the snap adds no more than 0.1 ms to a pointer move with a path in progress, measured in a release build.
11. Given the Select, Node, shape and Pencil tools, then Ctrl keeps the meaning it has there; nothing outside the Pen changes.

### Part B: typed length and angle

12. Given the Pen, the canvas focused, a path in progress with a last node L and no mouse button held, when the maker presses the entry key (Tab), then the Pen entry chip opens in the same frame, the key's default action is prevented, focus is in the Length field and its text is selected. Key repeat opens nothing. Tab with Ctrl, Alt or Cmd is not handled. With no path in progress, or with no node placed, the key is not consumed and does what it does without the Pen (no chip, no notice).
13. Given the chip, then it has two fields in Tab order, each with its unit visible inside the field at all times: "Length" with the display unit as suffix ("mm", "cm" or "in") and "Angle" with "°" as suffix. Accessible names: "Length, millimetres" (the unit word of the document) and "Angle, degrees". The chip is a group named "Next node". It is the surface and the field style of the placement chip of `0052`, not a new style, with no hint line (customer decision 2026-10-16).
14. Given the entry key is pressed with a mouse button held (a node drag in progress), then nothing opens, the key is consumed (focus does not leave the canvas) and the drag continues unchanged. Typed handle values are out of scope (customer decision).
15. Given the chip is open and no field is edited, then the fields show the values of the live segment from L to the pointer, as the rubber band draws it (so with Ctrl held they show the constrained values), updated in the frame of every pointer and key event; the chip follows the pointer as the placement chip does. Length uses the decimals of criterion 8, Angle one decimal at most. Example: L = (100, 50), pointer (130, 40): Length "31.6", Angle "-18.4".
16. Given the chip is open, when the maker first edits any field (types, deletes, cuts or pastes), then the chip freezes, as the placement chip does (`0052` criterion 14): the pointer and Ctrl change nothing from then on, an untouched field keeps the value it showed at that moment, and the chip stays where it is on screen (the same placement rule as the row "Placement chip", with L in the role of A, so the chip never covers L, its ring or the rubber band, flipping and clamping inside the viewport; recomputed on pan, zoom and window resize). Example: pointer (130, 40), the maker types "40" in Length: Length is 40, Angle stays -18.4, and moving the pointer to (0, 0) changes nothing.
17. Given a frozen or unfrozen chip, then the rubber band is drawn from L to the point the chip's values give, updated within one frame of each keystroke (the Pen is, with `0052`, the second place where typed values preview). The point is L + Length x (cos Angle, sin Angle) in the convention of the table (y down, so Angle 90 points straight down). Examples from L = (100, 50): Length 30, Angle 0 gives (130, 50); Length 30, Angle 90 gives (100, 80); Length 30, Angle -90 gives (100, 20); Length 30, Angle 45 gives (121.21, 71.21); Length 30, Angle 180 gives (70, 50).
18. Given the chip is open, when the maker presses Tab, then focus moves to the next field and wraps from Angle to Length, Shift+Tab goes back, and the field that receives focus has its text selected. Tab does not freeze and does not leave the chip.
19. Given the chip with valid values, when the maker presses Enter in a field, then one node is added to the path in progress at the point of criterion 17, as a corner node with no handles, exactly as a click at that point adds it (so the segment from L is a curve if L has an outgoing handle). The chip closes, the new node is the last node, the rubber band follows the pointer again from it, and the Pen goes on drawing. Enter in the chip never finishes the path; Enter on the canvas with the chip closed finishes it as today (`0002` criterion 3). With no field edited, Enter adds the node at the live segment's end (the point a click would place, with Ctrl's constraint if held). Example: Tab, "40", Tab, "0", Enter from L = (100, 50) adds a node at (140, 50); Tab, "25", Tab, "90", Enter then adds (140, 75); Tab, "40", Tab, "180", Enter adds (100, 75).
19a. (customer 2026-10-19) Given the Pen with a path in progress, when the maker presses the secondary (right) mouse button on the canvas, then the path is finished exactly as Enter finishes it (`0002` criterion 3): no node is placed at the press, no context menu opens, and an open Pen entry chip closes without adding a node. Given the Pen with no path in progress, a right press keeps its behaviour from before this story.
20. Given the chip is frozen, when the maker presses the primary button anywhere on the canvas, then the node is added at the point of the typed values, wherever the press is (the rule of `0052` criterion 22), and the rest of that press does nothing. Given the chip is open and not frozen, when the maker presses, then the chip closes and the press is an ordinary Pen press (a node at the pointer with Ctrl's constraint, or the target action of `0034`).
21. Given a length field, then a typed value may end in the display unit or in "mm", "cm" or "in" (any letter case, with or without a space) and is converted to the display unit; any other trailing text is not a number (criterion 24). The Angle field accepts a trailing "°". Decimal comma and decimal point, a leading "+" and the minus sign U+2212 are accepted, as everywhere (`parse_entry_number`, `parse_length`). Ctrl has no effect on the typed values. Examples in a document with display unit mm: "1in" is 25.4 mm, "2 cm" is 20 mm. In a document with display unit cm the fields show cm with two decimals and "4" in Length means 40 mm.
22. Given the chip is open, when the maker presses Escape, then the chip closes, nothing is added, the path in progress is exactly as it was, and the Pen keeps drawing with the rubber band back on the pointer. A second Escape discards the whole path (`0002` criterion 4), a third switches to the Select tool (`0010` criteria 42, 43). The chip also closes without adding when the maker switches tool, presses a rail button, the sidebar or the Properties panel, or finishes the path by double-click; the unfinished path then ends as `0034` criterion 26 says. A window blur does not close the chip and releases Ctrl.
23. Given the chip is open, then while it has focus the letters, Delete, Backspace and the chords of the key gate do not reach the canvas, Ctrl+Z and Ctrl+A act on the field's text, and no document step is undone. The keys the chip consumes are digits, separators, signs, the unit letters, "°", Backspace, Delete, Tab, Shift+Tab, Enter and Escape.

### Part C: permit over refuse (notices, edge cases)

24. Given a field holds text that is not a number when Enter is pressed (the typed text is empty after an edit, or letters, or two decimal separators, or an unknown unit), then the node is added using that field's value from the moment of the freeze, and a notice says so: "Not a number. Used 31.6 mm." or "Not a number. Used -18.4°." (the field's frozen value, with its unit). An emptied field never blocks Enter and never shows an error row.
25. Given the typed Length is negative, then the node is added at the point of the absolute value in the opposite direction (angle + 180 degrees), and the notice says "Negative length: drawn in the opposite direction." Example: L = (100, 50), Length -30, Angle 0 adds a node at (70, 50).
26. Given the typed Length is 0, or at or below the geometric tolerance, then no node is added, the chip stays open with the Length text selected, and the notice says "Length must be more than 0." This is the one case with no placeable result: a second node on the same spot would be a zero-length segment. Given the Length is untouched and the live length is at or below the tolerance (the pointer is on L), then Enter does the same.
27. Given the typed Angle is any finite number (for example 450, -270 or 3600), then it is taken modulo 360 degrees and shown after placing in -180 to 180; the result is the same point as the reduced angle (450 gives the point of 90). No notice. Examples: L = (100, 50), Length 30, Angle 450 adds (100, 80); Angle -270 adds (100, 80).
28. Given a typed point with any coordinate beyond 10 million mm (the coordinate limit of `0010` criterion 11), then the Length is reduced to the largest value that keeps the point inside the limit along that angle, the node is added there, and the notice says "Too far away. Length limited to the document limit." Example: Length 20000000, Angle 0, L = (100, 50): the node is at (10000000, 50).
29. Given any notice of this part, then it is shown in the canvas notice slot (`role="status"`, 3 s, text only, instant in and out; the row "Action notice" of `docs/design-system.md`) and is at most 70 characters in the English text. No notice contains a hint about keys.
30. Given the first node of a new path, then there is no segment and the Pen has no Length or Angle to show: the entry key opens nothing (criterion 12), Ctrl does nothing (criterion 7), and the click places a node at the pointer. Typing the position of the first node is out of scope.
31. Given a continuation (`0034`) from the first node of an existing path, then the typed node is prepended in the stored order exactly as a clicked node is (`0034` criterion 4); the point is still computed from E.
32. Given the Pen entry chip and the document, then nothing is written to the document while the chip is open, while typing, or on Escape. The nodes added by Enter belong to the path in progress and are written, together with the clicked nodes, by the one commit that finishes the path (`0002` criterion 3, `0034` criterion 5; the step is named as today). Ctrl+Z stays ignored while a path is unfinished (hint as today) and, with the chip open, acts on the field text only. No format field is added; `format_version` does not change.
33. Given a window of 800 x 600 px, then the chip lies fully inside the viewport (flipped and clamped), neither field is clipped for a value of up to 9 digits and its unit suffix, and it never covers L. A path whose last node is within 40 px of a viewport edge still gets a usable chip.

### Part D: accessibility and feedback

34. Given the chip opens, then a visually hidden polite live region says "Next node." and, after Enter, "Node added, 40 millimetres at 0 degrees." (the document's unit word). The live region names the chip and the result and gives no instructions. The notices of Part C are read by their own live region (criterion 29). The fields are real text inputs with the names of criterion 13.
35. Given the drag readout (criterion 8) and the chip, then every number on screen has its unit next to it: the chip suffixes (inside the field, always visible, also when the field is empty) and the readout text. No field or readout shows a bare number.
36. Given the rail tooltip of the Pen tool, then it gets no new line ("Ctrl: angle", "Tab: type values"): no operating hints (customer decision 2026-10-16). Both functions are listed in the shortcut table of `docs/design-system.md` in the PR that builds this slice.

## Out of scope

- **Typed handle values while the button is held** (handle length and angle). Reason: the node is already placed at the press, the other hand is on the mouse, and a symmetric handle is rarely a round number; the same chip cannot be opened without freezing the drag. Decided by the customer 2026-10-19.
- **Typing the position of the first node** (X and Y fields), and **closing the path from the chip**: the maker closes by clicking the first node, which a frozen chip does not allow (a press places the typed node); Escape closes the chip first.
- **A chip that stays open for the next segment** ("keep going" mode, as in AutoCAD). Tab opens it again; one key is cheap and Enter keeps one meaning.
- **Angle relative to the previous segment** (turn angle). Decided by the customer 2026-10-19: the typed angle is absolute.
- **Snap to the intersection of two angle lines** (for example the fourth corner of a rectangle aligned with the third and the first node), snap to the grid, to nodes, to guides or to bounding boxes (`0060-snapping` is a draft and decides how an object snap and this angle lock combine; until then there is no grid and no object snap in the Pen).
- **An on-screen angle-lock switch** for a tablet without a keyboard, or any Pen bar. Decided by the customer 2026-10-19.
- **Ctrl angle lock in the Node tool** (segment bending, handle drags), the Pencil, or **Alt** in the Pen.
- **Polar tracking guides** drawn across the canvas, an angle grid, and a preference for the step (the table of `0008` is the one table).
- **Backspace to remove the last placed node**, arithmetic in fields ("20+5"), and a remembered last length.
- **Undo of single nodes** (there is none: the path in progress is one step, `0034` criterion 27).

## Decided by the customer 2026-10-19

- **Entry key: Tab** (PEN-ENTRY-KEY A). No letter key is used, so the letter gate is unchanged. While a Pen path is unfinished Tab is taken from the browser's focus order; Enter or Escape gives it back.
- **Typed angle is absolute** (PEN-ANGLE-REF A): from the x axis, 0 degrees right, 90 down, -90 up, clockwise on screen; the same number the Ctrl lock uses and the readout shows. No turn angle.
- **No typed handle values** (PEN-HANDLE-TYPED A): the entry key with a button held does nothing (criterion 14).
- **No angle-lock toggle for keyboard-less use** (PEN-SNAP-TOGGLE A): Ctrl and Tab come from the keyboard; no Pen bar.

Decided by the product owner (change if you disagree): the stops are the existing table of 15 and 22.5 degrees (a 15-degree-only table would be a second table; horizontal, vertical and 45 degrees are in both); the lock keeps the length and changes the angle (Inkscape's behaviour); the lock is absolute, not relative to the previous segment; targets win over the snap; a handle snap leaves the node where it was placed; a typed node is a corner node as a click is; Enter in the chip never finishes the path; zero length is the one refusal; the chip does not stay open.

## UX notes

Draft of the product owner on 2026-10-19 from `docs/design-system.md` (rows "Placement chip", "Pen target cue", "Pen hint chip", "Action notice", "Keyboard concept" and the shortcut table). **To be confirmed by the ux-engineer before the build starts**; the numbers below are the rows' numbers, not new ones.

- **No new entry-box style.** The Pen entry chip is the "Placement chip" surface (`PlacementChip.tsx`): fields 28 px high, 4 px apart, 14 px tabular figures, the label inside the left padding at 12 px, the unit suffix inside the right edge. Two fields: "Length" (154 px, 48 px label padding) and "Angle" (150 px, 44 px label padding as in the placement chip, "°"); each keeps 72 px for the text, 9 digits at 14 px (criterion 33; widened from 132 px in the UX review). No r/diameter button, no modifier checks, no link slot, no hint row. The chip is as wide as its fields (320 px; fits the 336 px right of the tool rail at 800 x 600).
- **Place.** Until the freeze the chip's corner nearest the pointer is 12 px from the pointer on the side away from L on each axis (the Placement chip rule with L as the anchor); from the freeze it stays put on screen and the rule runs again on pan, zoom and resize. It never covers L, L's ring or the rubber band's start. No transition.
- **Preview.** The existing rubber band, dashed as in the Pen, runs to the typed point; no extra marker (L already carries the permanent ring of the most recent node, `0002`). With Ctrl held and no chip, only the existing rubber band changes (it is the constrained one); nothing new appears at rest. While a node is dragged, a readout "30.1 mm, 0°" sits 12 px up and right of the pointer (the create-drag readout's placement and formatter, units included) and is gone on release. The only previews are the rubber band, the handles while dragging, and the chip's typed point while the chip is open.
- **Cursor.** The Pen nib throughout, also with the chip open and frozen (a press still adds the node). Over the chip: the text cursor in a field.
- **Focus.** The Length field is focused with its text selected in the frame the chip opens. Every end of the chip (Enter, Escape, a press, a tool switch) returns focus to the canvas. The chip is rendered inside the canvas container so Ctrl tracking keeps working (`0052` adrs decision 5).
- **Notices.** The canvas notice slot, bottom centre, 3 s, `role="status"`, as the keyboard actions of `0020` and `0044`. No error row in the chip: the one refusal (zero length) is a notice, the chip stays open.
- **Shortcut table rows to add** (same PR): "Lock the angle of the next node" (hold Ctrl in the Pen with a path in progress; read live; also the handle direction while dragging; targets win) and "Type the next node" (Tab in the Pen with a path in progress and no button held; Tab switches Length and Angle; Enter applies and keeps drawing; Escape closes the chip; Tab accepted by the customer).
- **No new right-panel section, no new bar, no toolbar glyph, no hint label, no popup, no Apply or Cancel button, no snap toggle, no handle fields.** Enter applies, Escape cancels. The only new visible elements are the chip (opened by Tab) and the drag readout (only while the button is held). The UX notes were confirmed by the ux-engineer on 2026-10-19 against the rows "Placement chip", "Action notice" and "Interaction conventions".

## Delivery

One PR on `story/pen-angle-snap-and-typed-values`, as one slice (customer rule 2026-10-09). Size: **medium**, about 4 to 5 working days with the tests and the UX review, no new crate, no dependency, no format change. Milestones (commits on the branch, none a PR):

1. **Ctrl angle lock** (criteria 1 to 11): the constraint function in `curvyo-ui-core` on top of `snap_angle` (tests first, with the examples), the Ctrl flag passed through the Pen's move, down and up, the rubber band and handle preview, the Ctrl readout. The customer can try it at once in the app.
2. **Typed entry in the core** (criteria 15 to 17, 19, 21, 24 to 28, 30 to 32): the Pen entry state (live, frozen, result) as a pure module next to `shape_placement.rs`, reusing `parse_length` and `parse_entry_number`, with the polar resolution and the permit rules; the wasm calls.
3. **The chip** (criteria 12 to 14, 18, 20, 22, 23, 33 to 36): the Tab key in the key table, the chip (`PenEntryChip.tsx` on the shared `EntryField`), the Escape step, the live region, the notices. The slice is complete here.
4. **Docs and review**: the design-system rows, the shortcut table, the corrections named in "Changes to accepted behaviour", the tester pass and the UX review once on the finished slice, the architect's check of the key-table change.

Builds on code already on `main`: `angle_snap.rs`, `shape_placement.rs`, `display_unit_text.rs` (`parse_length`), `PlacementChip.tsx`. Collides with `0055` (`pen_tool.rs`) and `0032` (key gate, tool rail); not built beside them.

## Links

Requirements: R-EDIT-047 (new, `docs/requirements.md`), R-EDIT-001, R-EDIT-022
Builds on: `specs/0002-path-node-editing/` (Pen, handles, finish and Escape), `specs/0034-pen-path-extension/` (targets, Shift, continuation), `specs/0008-object-transform-refinements/` (criteria 33, 34: the angle stops), `specs/0010-edit-interaction-polish/` (criterion 39, Escape cascade, key gate, readout), `specs/0014-advanced-selection/` (live modifier read), `specs/0052-click-to-create-shapes/` (the entry chip, freeze, units), `specs/0015-document-size-and-rulers/` (display unit)
Related: `specs/0021-ellipse-arcs-and-shaping/` (criterion 7: Ctrl has no effect on a typed value), `specs/0047-transform-polish/` (typed move), `specs/0060-snapping/` (object snap, later), `specs/0055-compound-path-node-editing/` (same Pen file)
ADRs: `adrs.md` (architect)
PR: (set when opened)
