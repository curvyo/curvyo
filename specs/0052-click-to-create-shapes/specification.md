# Click to create shapes: place a rectangle, ellipse or polygon/star with two clicks and a typed size

Status: Ready (architect decisions in `adrs.md` (2026-10-12): no new ADR, no new crate or dependency, no `format_version` change, one `curvyo-document-core` API addition; UX notes confirmed 2026-10-12; the open questions use their recorded defaults; criteria marked (Proposal) can be vetoed by the customer at the demo)
Priority: Should
Origin: Customer (request of 2026-10-12). Criteria marked (Proposal) are product-owner additions, built with the slice and vetoable at the demo.

## User value

As a maker who cuts parts to a dimension, I want to click once with the Rectangle, Ellipse or Polygon/Star tool, see the shape follow my pointer without holding the button, type its exact size and angle while it follows, and click again or press Enter to place it, so that a 40 x 20 mm slot or a 6 mm hole takes two clicks and a few keys instead of a drag followed by a trip to the size entry.

The customer's request (2026-10-12, German, paraphrased by the lead and translated): "With the Rectangle, Ellipse (circle) and Polygon/Star tools a plain click without dragging does nothing today. Instead a click should start the shape, and it follows the mouse without holding the button (it grows from the click point to the pointer); a second click places it; Esc cancels. While it follows the mouse I want to type the size: rectangle width, Tab to height, Tab to rotation; circle radius (Space switches between radius and diameter, radius by default); polygon or star radius the same way (point count and inner ratio stay where they are today). After the size, Tab goes to the rotation (0 by default, or what the mouse gives). As soon as a value is typed, the mouse changes nothing any more; the typed values rule until Enter or the second click; Esc cancels everything. Press, drag and release stays as it is."

**Field reference.**

- **Inkscape**: a click without a drag in the rectangle, ellipse or star tool creates nothing (it selects the object under the pointer, or nothing). Exact sizes are typed afterwards into the tool's W and H fields in the tool controls bar.
- **Illustrator**: a click without a drag opens a modal dialog (Width and Height; for the polygon Radius and Sides; for the star Radius 1, Radius 2 and Points) and creates the shape at the click point. The dialog has no preview and no rotation, and it blocks the canvas.
- **Blender**: its modal operations (grab, rotate, scale) follow the mouse without a held button; typing a number takes over from the mouse; Enter or a left click confirms; Escape or a right click cancels. This is the interaction the customer describes.
- **CAD sketchers** (Fusion 360's two-point rectangle, AutoCAD's dynamic input): click a corner, move, type the width, Tab, the height, Enter. The input boxes sit on the shape.
- **LightBurn**: shapes are drawn by dragging; exact sizes go into the numeric edit fields afterwards.

**Where we do better:** no modal dialog and no second step: the size is typed into a chip on the shape while it follows the pointer, and the shape on the canvas is the typed result before it is placed. The mouse, the keyboard and the modifiers that makers already know (Ctrl square or circle, Shift from the centre, the polygon angle snap) work in one gesture. The drag stays exactly as it is, the result is one undo step, and the angle is typed with the size instead of in a second chip.

## What exists today

Facts from the specs on `main` (2026-10-12); the implementer verifies against the code.

| Item | Today |
|---|---|
| A click with a creation tool | Creates nothing and changes nothing (`0003` criteria 1, 7, 11, 12: "a drag where A equals B creates nothing"; `0009` criterion 26). |
| A tiny drag | Any drag with A not equal to B creates a shape, even 1 px; there is no dead zone for creation (`0011` "Out of scope": "only A equal to B is refused"). A one-axis drag creates a shape with a zero side (`0011` criterion 12). |
| Modifiers | Rectangle and ellipse: Ctrl = 1:1 (square, circle), Shift = from the centre, both together = square or circle around the press point (`0011`). Polygon and star: press = centre, pointer = first vertex, the pointer's direction is the angle, Ctrl snaps the angle to the stops of 15° and 22.5° (`0010` criterion 4), Shift does nothing. Modifiers may change mid-drag with the pointer still (`0011` criterion 8). |
| Preview and readout | The final shape is drawn live during the drag; a readout at the pointer shows "W × H mm", "rx × ry mm", "r 12.0 mm, -15°" or "r 12.0 mm, ratio 0.50, -15°" (`0003` UX notes, `0011` criterion 11, `0010` criterion 6). |
| After a create-drag | One commit; the Select tool becomes active with the new shape as the only selected object (`0009` criterion 28). One history step "Draw rectangle", "Draw ellipse", "Draw polygon" or "Draw star" (`0020` table, labels `create_rect`, `create_ellipse`, `create_polygon`, `create_star`). In an entered group the shape becomes a child of that group, on top (`0023` criterion 19). |
| Exact size | Only after creation: S or a double-click on a resize handle opens the size chip (`0008`, `0010` criterion 57), R the angle chip. |
| Escape | During a create-drag it cancels the drag; the next Escape leaves for the Select tool (`0010` criteria 42, 44). |
| Polygon/Star bar | Mode, "New: Points" and "New: Ratio" are settings for the next shape (`0009` criterion 29). |

## Terms

- **Click**: a press and a release of the primary button with the pointer moving less than 3 screen pixels between them (the dead zone of `0008`, `0010` and `0014`, `DRAG_THRESHOLD_PX`); for touch 8 px, as `0047` criterion 17.
- **A**: the press point of the click that starts a placement, in document millimetres.
- **Placement**: the state from that click until the shape is placed or the placement is cancelled.
- **Live shape**: the shape a create-drag from A to the current pointer position, with the current modifiers, would commit today. One function computes it for the drag and for the placement.
- **Placement chip**: the entry chip that belongs to a placement (UX notes, section 2).
- **Untouched field**: a field of the placement chip the maker has not edited. **Freeze**: the first edit of any field (criterion 14).
- **Direction**: for the rectangle and the ellipse, the sign of the pointer's offset from A on each axis; an axis with no offset counts as positive (`0011`, "The rule").
- **Display unit**: the document's unit for lengths (`0015` Part C: mm, cm or in; mm by default). Angles are in degrees, clockwise on screen, as the rotation readout.

## Changes to accepted behaviour

The lead names each at the demo; the owning specs are corrected in the PR that builds this slice.

| Spec and criterion | Today | After this spec |
|---|---|---|
| `0003` criteria 1, 7, 11, 12 ("a plain click with no movement creates nothing"); `0009` criterion 26 | A click creates nothing and changes nothing | A click starts a placement (criterion 1). Nothing is created until the placement is placed |
| `0011` "Out of scope" ("A drag threshold for shape creation: only A equal to B is refused") | A press and release 1 or 2 px apart creates a tiny shape | It is a click and starts a placement; a create-drag needs 3 px of movement (criterion 1) |
| `0010` "Out of scope" ("a default size or angle for a click without a drag") | Not done | Still no default size: the size comes from the pointer or the typed value |
| `0009` criterion 10, UX section 3 ("typed values do not preview") | Typed numbers never preview | The placement chip previews its typed values (criterion 15); the transform chips keep their rule |
| `0010` criterion 44 (Escape in a creation tool) | Escape cancels a drag, then leaves the tool | A placement is the operation in flight: Escape cancels it, the next Escape leaves the tool (criterion 7) |
| `0010` criterion 55 (the key gate) | Gated during a drag, with a chip open, with an unfinished Pen path | Also gated while a placement runs (criterion 8) |

## Acceptance criteria

All lengths are document millimetres in a document whose display unit is mm, X to the right, Y down, unless a criterion names another unit. Zoom 100 % (1 mm is about 3.78 CSS px) unless stated. Lengths are compared with the document's geometric tolerance. Shift and Ctrl are read as the host reports them (Ctrl is Cmd on macOS).

### Part A: click, follow, click (mouse)

1. Given the Rectangle, Ellipse or Polygon/Star tool and no placement running, when the maker presses at A and releases after moving less than 3 screen pixels, then nothing is created and a placement starts at A. When the pointer moves 3 px or more before the release, then it is a create-drag exactly as today (`0003`, `0011`, `0010` Part A), unchanged in every respect, and no placement starts. Examples at 100 %: press at (100, 50) and release at (100.5, 50) (about 1.9 px) starts a placement; press at (100, 50) and release at (101, 50) (about 3.8 px) creates a 1 x 0 mm rectangle as today.
2. Given a placement with no field edited, when the pointer moves with no button held, then the live shape is drawn as the preview, in the look of today's create-drag preview, updated in the frame of each pointer event; it is the exact shape a second click at that position would commit (one function for the drag, the preview and the commit). While the pointer is within 3 px of A, no outline is drawn (`0011` criterion 12). Example: Rectangle tool, A = (100, 50), pointer at (130, 40): the preview is the rectangle with origin (100, 40), width 30, height 10.
3. Given a placement with no field edited, then the modifiers work as in a create-drag and are read live, also with the pointer still: Rectangle and Ellipse, Ctrl = 1:1 and Shift = from the centre (`0011` criteria 1 to 8); Polygon/Star, Ctrl snaps the angle to the stops of `0010` criterion 4 and Shift does nothing. Ctrl or Shift may already be held at the first click. The preview and the chip's values change in the frame of the key event.
4. Given a placement with no field edited, when the maker presses the primary button on the canvas at B, at least 3 px away from A, then at that press the live shape for B and for the modifiers of that press event is committed, and the rest of that press (any movement, the release) does nothing. The press may land on an existing object; creation tools never select (`0009` criterion 25). Example: A = (100, 50), second press at (130, 40), no modifiers: one rectangle with origin (100, 40), width 30, height 10, corner radius 0, rotation 0, the same document state as a create-drag from (100, 50) to (130, 40).
5. Given a placement with no field edited, when the maker presses the primary button within 3 px of A (for example the second click of a double-click), then the press is ignored: nothing is written and the placement continues. A double-click with a creation tool therefore never creates a zero-size or sliver shape.
6. Given a placement is placed (by a second click, criteria 4 and 22, or by Enter, criterion 21), then exactly one object is written in one commit, with the stored fields a create-drag writes (no new field, `format_version` unchanged). It is one history step named as a create-drag of the same shape is named ("Draw rectangle", "Draw ellipse", "Draw polygon", "Draw star"; `0020` table, same commit labels). The Select tool becomes active with the new shape as the only selected object (`0009` criterion 28), the chip closes, and focus is on the canvas. In an entered group the shape becomes a child of that group, on top of its children (`0023` criterion 19).
7. Given a placement, when the maker presses Escape (with focus in the chip or on the canvas), then the placement ends, nothing is written, the preview, the chip and the anchor marker disappear in that frame, and the tool stays active; a second Escape switches to the Select tool (`0010` criteria 42 and 44; the placement is step 2, "an operation in flight"). The same cancel happens on a press with the secondary (right) mouse button on the canvas (Proposal, Blender's habit), on a tool switch by the rail or by any route, and on a press outside the canvas and the chip, except in the active tool's own bar (criterion 25). The window losing focus does not cancel; it releases Shift and Ctrl, so an unfrozen placement then follows the pointer with no modifier (`0014`'s rule for held modifiers).
8. Given a placement, then the key gate of `0010` criterion 55 treats it as an operation in flight: tool letters, M, R, S, K, Delete, Backspace, Ctrl+A, the group chords (Ctrl+G, Ctrl+Shift+G) and the file chords of `0051` (Ctrl+N, O, S, Shift+S, W, Q) have no effect on the document, the selection or the tool. A letter typed into a chip field is text (and refused by criterion 23). Ctrl+Z and Ctrl+Shift+Z with focus in a field act on the field's text only; no document step is undone while a placement runs.
9. Given a placement, when the maker zooms with the wheel or a pinch, or pans with the middle button, then A stays fixed in document space and the preview follows the pointer's document position. Space does not pan while the placement chip has focus (criterion 20 gives Space its meaning there). When the pointer leaves the viewport, the preview keeps its last shape; when it comes back, the preview follows again.
10. Given a placement and the pointer over the placement chip, then the preview keeps its last shape (the chip is not a canvas position), and a press on the chip focuses the field under it and is not a second click.

### Part B: typing the size and the angle

11. Given a placement starts, then the placement chip opens in the same frame with focus in its first field and that field's text selected, so typing replaces it. The fields, in Tab order, with their visible label, suffix and accessible name:
    - Rectangle: "W" (display unit, "Width"), "H" (display unit, "Height"), "Angle" ("°", "Rotation").
    - Ellipse: "r" (display unit, "Radius"; "⌀" and "Diameter" after Space, criterion 20), "Angle" ("°", "Rotation").
    - Polygon/Star: "r" (display unit, "Outer radius"; "⌀" and "Outer diameter" after Space), "Angle" ("°", "Rotation").
    The chip is a group named "New rectangle", "New ellipse", "New polygon" or "New star". Until the freeze it follows the pointer: its corner nearest the pointer sits 12 px from the pointer on the side away from A on each axis (an axis with no offset counts as right or down), with the flip and clamp rule of the design-system row "Placement chip". From the freeze it stays where it is on screen; on pan, zoom or window resize the same rule runs again from the pointer's document position at the freeze. It never covers A. The create-drag readout is not shown during a placement; the chip replaces it.
12. Given a placement with no field edited, then the fields show the live shape's values, updated in the frame of every pointer and key event: W and H the full width and height of the live box (also from the centre, `0011` criterion 11); for the ellipse, r the radius when the live shape is a circle (rx equal to ry), otherwise the field is empty with the muted placeholder "rx × ry" in numbers ("15.0 × 5.0"); for the polygon and star, r the distance from A to the pointer (the outer radius); Angle "0" for the rectangle and the ellipse and, for the polygon and star, the angle of `0010` criterion 1 for the live shape (after a Ctrl snap). Lengths use the decimals of the status bar for the display unit (mm 1, cm 2, in 3), the angle one decimal at most.
13. Given the chip, then Tab moves focus to the next field and wraps from the last to the first (rectangle: W, H, Angle, W; ellipse and polygon/star: r, Angle, r), Shift+Tab goes back, and a field that receives focus has its text selected. Tab and Shift+Tab do not freeze anything and do not leave the chip.
14. Given a placement, when the maker first edits any field (types a character, deletes, cuts or pastes), then the placement freezes: from that moment the pointer and the modifier keys change nothing. Every untouched field keeps the value it showed at that moment, and the direction, Shift's centred mode, Ctrl's 1:1 and the polygon's angle are kept as they were at that moment. The chip stays where it is. The muted hint line of the chip changes to say that a click places anywhere (UX notes, section 2). Example: Rectangle tool, A = (100, 50), pointer at (130, 40), no modifier; the maker types "40": W is 40, H stays 10, the direction stays right and up; moving the pointer to (0, 0) changes nothing.
15. Given a frozen placement, then the preview shows the shape the typed and kept values give, updated within one frame of each keystroke (an exception, for this chip only, to "typed values do not preview" of `0009` criterion 10). While a field holds text that is not valid (criterion 23), the preview keeps the last valid shape and the field is marked.
16. Given the Rectangle tool, then the result has a corner at A (or, with Shift kept, its centre at A), extends W along x and H along y in the kept direction, and is then turned by Angle about A, clockwise on screen. Examples, each from A = (100, 50) with the pointer at (130, 40) at the freeze: (a) "40", Tab, "20", Enter: origin (100, 30), width 40, height 20, rotation 0. (b) "40", Tab, "20", Tab, "90", Enter: the corners are (100, 50), (100, 90), (120, 90) and (120, 50); the stored rectangle has its centre at (110, 70), width 40, height 20, rotation 90°, and the Select tool shows 90°. (c) Shift held at the freeze, "40", Tab, "20", Enter: centre (100, 50), origin (80, 40), width 40, height 20.
17. Given Ctrl was held at the freeze of a rectangle placement, then W and H are linked: typing into either sets both, and a chain glyph between them shows the link (the linked fields of the size chip, `0008` criterion 29). Example: Ctrl held, A = (100, 50), pointer at (130, 40), "25", Enter: a 25 x 25 square with its corner at A, extending right and up: origin (100, 25).
18. Given the Ellipse tool, then a typed r (or ⌀ divided by 2) gives a circle, rx = ry = r, whose 2r x 2r box has a corner at A in the kept direction (or its centre at A with Shift kept), turned by Angle about A. An untouched r keeps the live rx and ry. Examples, from A = (50, 50) with the pointer at (70, 80): "10", Enter: centre (60, 60), r 10. Space, "10", Enter: ⌀ 10, centre (55, 55), r 5. Shift held at the freeze, "10", Enter: centre (50, 50), r 10. "10", Tab, "90", Enter: the box turns about A, so the centre is (40, 60), r 10, rotation 90° (see Question 4).
19. Given the Polygon/Star tool, then the result has its centre at A, outer radius r (or ⌀ divided by 2) and its first vertex at Angle (`0010` criterion 1); point count, mode and inner ratio come from the tool bar as today (`0003` criteria 10 and 12, `0009` criterion 29). An untouched Angle keeps the angle shown at the freeze. Examples with a hexagon, A = (100, 100), the pointer at (110, 100): "25", Enter: first vertex at (125, 100), shown angle 0°. "25", Tab, "-90", Enter: first vertex at (100, 75), shown angle -90°. In Star mode with ratio 0.5, "25", Enter: outer radius 25, inner radius 12.5.
20. Given the Ellipse or Polygon/Star chip, when the maker presses Space with focus in any of its fields, then the size field switches between radius and diameter: no space character is inserted and the canvas does not pan. Its label changes between "r" and "⌀" and its accessible name between "Radius" and "Diameter" ("Outer radius" and "Outer diameter"). An untouched size field is re-shown in the new mode so the shape does not change (r 12.0 becomes ⌀ 24.0); an edited field keeps its text and the text is read in the mode now current (the rule of the move chip's mode switch, `0010` criterion 19). Example: "20", Space, Enter gives a circle of diameter 20 (r 10). Space does not freeze. The size field's label is also a button: a click on it switches the mode the same way. The chip opens in radius mode every time; the mode is never stored. In the rectangle chip Space does nothing (no text, no pan).
21. Given a placement, when the maker presses Enter with focus in any field of the chip, then the shape is placed (criterion 6) with the typed and kept values; with no field edited it is the live shape, as a second click at the pointer's position would place it.
22. Given a frozen placement, when the maker presses the primary button anywhere on the canvas, then the shape is placed with the typed and kept values, wherever the press is; the press position is not used. Example: after "40", Tab, "20" of criterion 16 (a), a press at (300, 300) places the rectangle with origin (100, 30), width 40, height 20.
23. Given Enter or a second click, when a field is not valid, then nothing is written, the placement continues, focus goes to the first invalid field with its text selected, the field gets `aria-invalid` and a message line under it, and the message is one of: "Enter a number" (empty after editing, letters, more than one decimal separator, any other text); "Must be more than 0" (a size of 0, a negative size, or a size at or below the geometric tolerance, typed or kept); "Too large" (any coordinate of the result beyond ±1e7 mm, the coordinate limit of `0010` criterion 11). A decimal comma and a decimal point are accepted, a leading "+" is accepted, and the minus sign U+2212 is accepted as "-". Any finite angle is accepted and shown in the range -180° to 180° after placing. Examples: "abc" in W gives "Enter a number"; "0" or "-5" in r gives "Must be more than 0"; "20000000" in W gives "Too large"; a keyboard placement (criterion 26) with H never typed gives "Must be more than 0" on H.
24. Given a length field, then its value is in the display unit and the suffix shows that unit. A typed value may end in the display unit or in "mm", "cm" or "in" (any letter case, with or without a space before it); a value with another unit is converted to the display unit (Proposal). Any other trailing text is "Enter a number". The Angle field accepts a trailing "°". Examples in a document with display unit mm: "1in" is 25.4 mm, "2 cm" is 20 mm. In a document with display unit cm, the fields show cm with two decimals and "4" in W means 40 mm.
25. Given a placement with the Polygon/Star tool, when the maker changes the mode, "New: Points" or "New: Ratio" in the tool's bar, then the preview takes the new value in the next frame, the placement continues and the typed values are kept. A click in the tool's own bar is not a canvas press and does not cancel. Focus goes back to the chip with a click on a chip field.

### Part C: keyboard only (Proposal)

26. Given a creation tool, the canvas focused, no placement and no drag, when the maker presses Enter, then a placement starts at A = the pointer's document position if the pointer is over the viewport, otherwise at the centre of the viewport, with the chip open and focus in its first field. The direction is right and down, the polygon's angle 0. Example: pointer outside the viewport, viewport centre at (148.5, 105): Enter, "30", Tab, "20", Enter places a rectangle with origin (148.5, 105), width 30, height 20. Enter with an untouched size places nothing (criterion 23).
27. Given a placement started by Enter, then every criterion of Part B holds for it, Escape cancels it (criterion 7), and after it is placed focus is on the canvas, so the next Escape and the Select tool's keys work at once.

### Part D: feedback, accessibility, touch, speed

28. Given a placement, then the pivot marker of the design system is drawn at A for its whole duration (A is where the shape grows from and the point it turns about), in every modifier state, above the preview and below the chip (Proposal; `0011` shows it only while Shift is held during a drag).
29. Given the chip, then its last line is a muted hint: "Click or Enter: place. Esc: cancel." for the rectangle; "Space: r or ⌀. Click or Enter: place. Esc: cancel." for the ellipse and the polygon/star; after the freeze, "Click anywhere or Enter: place. Esc: cancel." The tooltips of the Rectangle, Ellipse and Polygon/Star rail buttons gain the line "Click, then type a size".
30. Given a placement, then a visually hidden live region (polite) says, at the start: "New rectangle. Type the width, Tab for height and angle. Enter places, Escape cancels." (ellipse: "New ellipse. Type the radius, Space for diameter, Tab for angle. Enter places, Escape cancels."; polygon and star likewise with "outer radius"); when it is placed: "Rectangle placed, 40 by 20 millimetres." / "Circle placed, radius 10 millimetres." / "Ellipse placed, 15 by 5 millimetres." / "Polygon placed, outer radius 25 millimetres." / "Star placed, outer radius 25 millimetres." (the display unit's word); when it is cancelled: "Cancelled." A field's error message is linked by `aria-describedby` and announced politely. The chip's fields are real text inputs with the accessible names of criterion 11.
31. Given a pen in range, then it moves the pointer and every rule above applies as for a mouse. Given touch (Proposal), then a tap is a click (8 px), a first tap starts a placement (the shape stays at zero size, since touch has no hover), and a second tap at least 8 px from A places the live shape for that tap point (two-tap creation); the chip is shown but takes focus only when a field is tapped, so no on-screen keyboard opens by itself; a touch drag creates as today.
32. Given a star with 1024 points (the maximum, `0003` criterion 10) in a placement, release build, the reference desktop the architect names, then the preview follows the pointer at 60 frames per second (no frame longer than 16.7 ms in a 5 s trace of continuous movement), and a keystroke in a frozen placement is shown within one frame. The gate is the one of `adrs.md` decision 7: native release benchmarks (the placement preview frame of a 1024-point star at most 1.1 times the create-drag frame of the same star; one placement result per keystroke under 1 ms), plus one frame-rate check in the real window on the reference desktop (the customer's Linux machine). A miss of the absolute 16.7 ms in that check is recorded under the debt item "Canvas performance on Linux/WebKitGTK" and does not block the slice.
33. Given a window of 800 x 600 px, then the chip lies fully inside the viewport (flipped and clamped), none of its fields is clipped for a value of up to 9 digits and a unit suffix, and it never covers A or its marker; a placement that starts within 40 px of a viewport edge is still usable.

### Part E: what does not change

34. Given a create-drag (3 px or more before the release), then everything of `0003`, `0011` and `0010` Part A holds unchanged: the readout, the modifiers, the release commit, Escape. The Pen (a click places a node), the Node tool and the Select tool are not changed by this spec.
35. Given a placement, then neither its start, nor typing, nor a cancel writes anything or adds a history step; a placed shape is one step, and Ctrl+Z after it removes the shape and the selection returns to what it was before (`0020`).
36. Given a project saved after a placement, then it holds only the existing fields of the shape kinds; a save is not possible while a placement runs (criterion 8), and no preview or chip state is ever written. `format_version` does not change.

## Out of scope

- Typing two radii for an ellipse (rx, Tab, ry), a corner radius, a point count or an inner ratio in the chip (point count and ratio stay in the tool bar, as the customer asked).
- Typing the position of A (X and Y fields); a negative size as a direction; a "from the centre" switch in the chip (Shift held at the freeze is the route); snapping of A or of the pointer.
- A default size for a click, remembering the last typed size, a "repeat last shape" command.
- Click-to-create for the Pen (a click already places a node), the Pencil (`0032`) or any other tool.
- Arithmetic in fields ("20+5"), and unit suffixes in the other chips (the move, size, angle and skew chips keep millimetres; Question 9).
- Auto-pan when the pointer nears the viewport edge during a placement.
- A modal dialog in the manner of Illustrator.
- Any change to the create-drag beyond criterion 1 (a press and release under 3 px is now a click).

## Open questions (defaults taken)

None blocks the build; each default is what the criteria say. The lead batches them in one German message.

1. **Confirm with Enter or the second click.** *A (default):* both (criteria 21, 22). *B:* Enter only. *C:* second click only. Recommendation A: the mouse hand and the keyboard hand each have their way.
2. **Tab order.** *A (default):* rectangle W, H, Angle; ellipse and polygon/star r, Angle, as the customer listed. *B:* Angle reachable only on request. Recommendation A.
3. **Where the typed angle turns the shape.** *A (default):* about the click point A, so the corner you clicked stays where you clicked (for the polygon and star, and with Shift, A is the centre anyway). *B:* about the shape's centre, so the clicked corner moves away from the click. Recommendation A.
4. **The angle of a circle.** With A, a circle placed from its box corner and turned moves around A (criterion 18 example), and a circle placed from its centre does not visibly change. *A (default):* keep the Angle field for the circle and store the rotation (one Tab order for all three tools; the turned box shows after placing and matters once the circle is stretched into an ellipse). *B:* no Angle field in the ellipse chip when the result is a circle. Recommendation A.
5. **What a typed value freezes.** *A (default, the customer's words):* the first typed character freezes everything; the mouse changes nothing more. *B:* only the typed field is fixed and the untouched ones keep following the mouse (Fusion 360's way). Recommendation A.
6. **Second click: press or release.** *A (default):* the shape is placed at the second press, so a tremor before the release cannot change it. *B:* at the release. Recommendation A.
7. **Right click cancels** (criterion 7, Proposal). *Default:* yes, as in Blender; no context menu exists in the creation tools.
8. **Enter starts a placement from the keyboard** (criteria 26, 27, Proposal) and **two-tap creation on touch** (criterion 31, Proposal). *Default:* both yes.
9. **Units.** The placement chip shows the display unit and converts "mm", "cm" and "in" (criterion 24, the conversion is a Proposal); the existing chips show millimetres. *Default:* this chip follows the document's unit now; the other chips get the same rule in a later small change if the customer wants it.

## UX notes

Status: **confirmed by the ux-engineer (2026-10-12)**, from the product owner's draft and `docs/design-system.md` (rows "Placement chip", "Transform pivot marker", "Keyboard concept", "Rail tooltip and notice placement" and the shortcut table, updated in the same change). Sizes are screen pixels, colours are tokens of the design system. The criteria were reworded to match these notes on 2026-10-12 (see "UX flags" below).

### 1. Preview, anchor, cursor

- **Preview:** the create-drag preview as built (row "Live preview outline": 1.5 px `--preview-new`, hollow, with its casing), unchanged, so a placement and a drag look the same while the shape grows. No dimming and no ghost of a final shape. Within 3 px of A nothing is drawn but the pivot marker.
- **Anchor:** the pivot marker (row "Transform pivot marker": 6 px, `--accent` at 60 %) at A from the first click to the end of the placement, in every modifier state, above the preview and below the chip (criterion 28). It says "the shape grows from here and turns about here", which the typed Angle needs.
- **Cursor:** the creation tool's crosshair, unchanged, also after the freeze (a click still places). Over the chip: the text cursor in a field, the arrow on the r/⌀ button. The pan cursors of Space and the middle button override it as everywhere.
- **No motion:** nothing in a placement animates (no fade of the chip, preview or marker); appearing and disappearing happen in one frame, so `prefers-reduced-motion` needs no special case.

### 2. Placement chip

Full row: "Placement chip" in the design system. In short:

- **Surface:** the "Transform entry chip" surface, shared with the move chip as one component (adrs decision 1, frontend). Row 1 the fields, 28 px high, 4 px apart, labels inside the field's left padding at 12 px as in the size chip, the display-unit suffix inside the right edge. Widths hold 9 characters at 14 px tabular figures (criterion 33): W, H and r 124 px; Angle 132 px (the label "Angle" needs a 44 px left padding). Between W and H a 16 px slot, always reserved, that shows the 12 px `Link2` glyph only while Ctrl's link is kept (criterion 17), so H never moves. Row 2 the hint, 12 px `--panel-muted-fg` (5.0:1 on `--toolbar-bg`), one line that never wraps; the chip is as wide as its widest row (rectangle 424 px, ellipse and polygon/star about 292 px).
- **r and ⌀:** a 24 x 28 px text button over the size field's left padding (which grows to 28 px), showing "r" or "⌀" at 12 px; accessible name "Diameter", `aria-pressed` true in diameter mode, `tabindex="-1"` (Space from any field is the keyboard route), tooltip "Radius or diameter (Space)". A click switches the mode and leaves focus in the field it came from.
- **Ellipse placeholder:** while the live ellipse is not a circle the r field is empty and shows "15.0 × 5.0" as its placeholder in `--panel-muted-fg` (about 6.9:1 on white, so it is readable, not decorative).
- **Error:** a refused value marks the field (2 px `--field-invalid`, `aria-invalid`) and adds a message row between the fields and the hint: 12 px `--field-invalid` text with the 12 px alert glyph, linked by `aria-describedby`, announced politely, cleared on the next keystroke. The chip grows on the side away from A, so the field row does not move under the eye.
- **Place:** the chip's corner nearest the pointer sits 12 px from the pointer **on the side away from A on each axis** (the outward rule of the transform entry chip; an axis with no offset counts as down or right, as "Direction"). It therefore never covers A, and for the rectangle and the ellipse never the live shape. Where the outward side has no room in the viewport, the chip goes to the other side of A on that axis (12 px clear of the marker), then clamps. The vertical axis always has room at 800 x 600 (the chip is at most 104 px high with a message), so A is never covered. Until the freeze the chip moves with every pointer event and changes sides when the pointer crosses A's horizontal or vertical line. From the freeze its anchor is the pointer's document position at the freeze: it stays put on screen and the same rule runs again on pan, zoom and window resize.
- **Why not 12 px up and right like the readout** (the product owner's draft): the readout is one short line; this chip is up to 424 px wide. Up and right of the pointer it covers the shape in every placement towards the lower left, and near A it covers the marker.
- **Focus:** the first field has focus and its text selected in the frame the placement starts (mouse, pen and Enter; on touch only when a field is tapped, criterion 31). Tab and Shift+Tab cycle inside the chip. Every end of the placement (placed, Escape, cancel) returns focus to the canvas. The chip is rendered inside the canvas container so modifier tracking keeps working (adrs decision 5). Keys the chip consumes: digits, separators, signs, unit letters, "°", Backspace, Delete, Tab, Shift+Tab, Enter, Escape, Space (all three chips; in the rectangle chip Space does nothing). Ctrl+Z and Ctrl+A in a field act on its text only.
- **Before the freeze the pointer cannot reach the chip** (it moves away with the pointer); Space is the route to ⌀. After the first keystroke the chip stands still and every part of it can be clicked (criterion 10).
- **Differences from the "Numeric entry chip" pattern,** on purpose and only here: the typed values preview (criterion 15); a press on the canvas places instead of cancelling (criteria 4, 22); window blur does not cancel (criterion 7); Escape ends the chip and the placement together, one step of the cascade (criterion 7).

### 3. Texts

- Hint lines, rail tooltip line and live-region sentences: criteria 29 and 30, as written. The live region is the polite one of the chip (always mounted, the text change is the announcement); the canvas notice slot is not used by a running placement.
- **Rail tooltips:** Rectangle and Ellipse get a third, muted line "Click, then type a size" under "Shift: from center. Ctrl: square or circle"; Polygon/star, which has no second line today, gets it as its second line. The design system's rail row is amended: creation tools may have three lines.

### 4. Keyboard and gate

- Enter on the focused canvas with a creation tool starts a placement (criterion 26); Space switches r and ⌀ inside the chip; both are in the shortcut table now.
- A placement is "an operation in flight" in the keyboard concept's gate (criterion 8) and step 2 of the Escape cascade (criterion 7).
- **Space pan during a placement:** not available while the chip has focus (Space means r/⌀). Wheel, pinch and the middle button pan and zoom as usual (criterion 9). Accepted: a placement is short, and the alternative (Space pans, another key for ⌀) breaks the customer's own wording.

### 5. Other panels and slices

- **`0051` sidebar, `0050` Layers panel, Properties panel, rail:** a press on any of them is a press outside the canvas and the chip and cancels the placement before its own action runs (adrs decision 6). The only exception is the active tool's own bar (criterion 25). The maker sees the preview, marker and chip vanish in that frame; the live region says "Cancelled."
- **`0050`, locked or hidden active layer:** the first click (or Enter) starts no placement: no chip, no marker, and the notice of `0050` criterion 23 in the canvas notice slot ("Layer Cut is locked. Unlock it or choose another layer."), the same as for a drag. The check runs again at the place (a peer may lock the layer meanwhile): then nothing is written, the placement ends and the same notice shows.
- **Large documents:** the preview is one shape; the frame cost does not depend on the document (criterion 32's gate).

### UX flags to the product owner (no customer question)

1. **Criterion 11, chip place:** reword "12 px up and right, with the readout's flip rules" to "12 px from the pointer on the side away from A on each axis, with the flip rule of the design-system row 'Placement chip'". Default: these notes. *Resolved 2026-10-12: criterion 11 reworded.*
2. **Criterion 33:** fine as written; it is why the fields are 124 and 132 px, not the size chip's 100 px. *Resolved 2026-10-12: no change needed.*

## Links

Requirements: R-EDIT-033 (new, `docs/requirements.md`), R-EDIT-002
Builds on: `specs/0003-primitive-shapes/`, `specs/0009-unified-object-editing/` (criteria 25 to 29), `specs/0010-edit-interaction-polish/` (criteria 4, 19, 42 to 49, 55; `adrs.md` decision 3 for the entry chip parts), `specs/0011-shape-creation-from-center/`, `specs/0015-document-size-and-rulers/` (display unit)
Touches: `specs/0020-undo-redo/` (step names, unchanged), `specs/0023-groups/` (criterion 19), `specs/0051-sidebar-and-no-menu/` (chord gate)
PR: (set when opened)
