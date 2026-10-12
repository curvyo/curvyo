# Path offset: Outset and Inset by dragging a handle, with curves kept as curves

Status: Ready (rework 2026-10-12 after customer review of #115)
Priority: Should
Origin: Customer (Inset and Outset, request of 2026-10-09; rework decisions of 2026-10-12). R-EDIT-007 (outline/offset). Amended by `0020-undo-redo`: Ctrl+Z takes the step back.

## Rework history

The first build (PR #115, now a draft) had a Select bar button "Offset", a section at the top of the Properties panel, an Apply step and a polygon result. The customer rejected it on 2026-10-12: the panel section appeared out of nowhere and pushed the Style rows down, offset is an operation and not a property of the object, and the rounded results were made of many small segments (a circle and a rectangle gave "a disaster"). The customer accepted the rework concept with all defaults on 2026-10-12. This specification replaces the first one. Sources: `ux-rework.md` (UX), `geometry-rework.md` (architect).

## User value

As a maker I want to make a bigger or smaller copy of a shape by dragging a handle on the canvas, or by typing an exact distance, and to see the result before I let go, so that borders, seam allowances, inlays and kerf allowances (2 mm outside a part for a kiss-cut border, 0.15 mm inside it for a kerf allowance, 3 mm around a line for a cut-out slot) take seconds, and so that a circle stays a circle, a rectangle stays a rectangle and a curve stays a curve I can edit and a machine can run smoothly.

**Reference tools (as I understand them).**

- Inkscape: Path > Outset and Inset replace the path by a fixed step from Preferences, no preview. Dynamic Offset has one handle on the outline that is dragged; it converts the path into a special object type. We take the handle, not the conversion.
- Illustrator: Object > Path > Offset Path, a dialog with Offset, Joins, Miter limit, Preview; adds a new path, keeps the original.
- LightBurn: Offset Shapes, a dialog with Distance, Corner style, Direction (outward, inward, both), "Delete original". We take Both and the replace choice, on the canvas and as modifier keys.
- Affinity: the Contour tool.

What we do better: the distance is dragged on the canvas with the result shown live and a number at the pointer, or typed when it must be exact; both sides in one drag (Shift); rectangles, circles, polygons and stars stay what they are; curved shapes give few smooth Bézier segments, not hundreds of corners; the rules for corners, open paths, holes and vanishing shapes are written down with numbers.

## Decisions

Customer decisions of 2026-10-12 (the rework concept, all defaults accepted):

1. A new rail card **"Path tools"** holds the **Offset tool** (only that tool for now), key **O**.
2. Modifier keys during a drag: **Shift** = both sides; **Ctrl** = change the original instead of making a new object; **Shift+Ctrl** = the original takes the dragged side and a copy is made for the other side.
3. A bigger copy goes **below** its original in the stacking order, a smaller copy **above** it.
4. A non-circular ellipse becomes an **exact path of few cubics**, not an approximate ellipse.
5. **No precision setting.** The result is exact to the kernel tolerance of 0.01 mm.
6. The **ADR 0003 §4 amendment is approved**: offset results are curves rebuilt from exact pieces, and the offset library only decides topology (what survives, holes, vanishing parts). The polyline stays as the fallback. No new dependency.
7. **UX rule:** the right-hand panel never grows sections for an operation. The Offset panel section and the Select bar Offset button of the first build are removed.

Earlier decisions that still hold: `i_overlay` `=9.0.1` is the topology kernel, no new dependency, no `Cargo.toml` change (lead, 2026-10-10); miter limit fixed at 4 with `miter-clip` at the limit (2026-10-10); Round is the default corner and end; values of the bar are session state, never saved in the file; commit label `offset_path`.

Proposals of the product owner that the customer accepted with "all defaults": the 0.01 mm to 1000 mm typed range, the snap series, the 2,000-node preview limit, the segment budgets below.

## Words used below

- **Operand:** a selected closed ordinary path, compound path, rectangle, ellipse, polygon or star, or an open ordinary path. The **region** of a closed operand is the area its outline(s) enclose under the nonzero rule, as the canvas paints a fill, whether or not Fill is on (`0016` criteria 4 and 5). For an open path the region to grow is the path itself, a line of no area.
- **Distance d:** a length in millimetres, unsigned in the bar. The **side** is Out (grow, Outset), In (shrink, Inset) or Both. Internally d is signed: positive grows, negative shrinks.
- **Corners (join):** how a corner is treated where the offset outline opens a gap (at convex corners when growing, at concave corners when shrinking): **Round** (an arc of radius d), **Miter** (the two edges extended to their meeting point, clipped at 4 x d from the corner when the point is farther away), **Bevel** (a straight cut across the gap). Default Round.
- **Miter limit:** fixed at 4, as SVG's: a miter is kept while its tip is at most 4 x d from the corner, which is a corner angle of at least about 28.96 degrees. A sharper corner is **clipped at the limit**: the tip is cut off by a straight edge whose two nodes lie 4 x d from the corner, symmetric about the corner's bisector (SVG 2 `miter-clip`, what the offset library does). It is not a setting.
- **Ends (cap):** how the ends of an open path are finished when it is grown: **Round**, **Flat**, **Square**. Default Round. Only open paths have ends.
- **Kernel tolerance:** 0.01 mm, as `0016`.
- **Segment:** one line or one cubic Bézier of a path.
- **Stays a primitive:** the result is the same kind of rectangle, ellipse, polygon or star object, with its own fields (size, corner radii, point count, inner ratio) and its rotation, not a path.

## What the result is: exact pieces, curves kept

The result is built from the exact offset of each piece of the operand, and the offset library only decides which parts survive:

- A straight edge gives one parallel line. A cubic gives a few cubics that follow its exact offset. A Round corner gives a true circular arc (one cubic per 90 degrees or less). A Miter corner gives the extended edges and, past the limit, the clip line. A Bevel gives a line. An open path also gets its ends.
- The library's topology (survivors, holes, vanishing parts, self-crossing cleanup) is mapped back onto these pieces. Where a piece cannot be matched within the kernel tolerance, **that outline** is written as a polyline, as in the first build (the fallback). The result is never worse than the first build.
- Nodes where two pieces meet with the same tangent are smooth nodes (with handles); nodes where the tangents differ are corner nodes.
- The original stays as it is unless Change original is chosen, so its curves are not lost either way.

## Acceptance criteria

### The tool

1. Given the app, then the tool rail column A has a toolbox card named "Path tools" 8 px under the Group card, with one button, the **Offset tool** (40 x 40 px, glyph: a solid rounded square inside a dashed concentric outline, accessible name "Offset tool"). The tooltip reads "Offset tool (O)" with a muted second line "Drag a corner handle. Shift: both sides. Ctrl: change the original.". The card has `role="toolbar"` named "Path tools", is one Tab stop and follows the rail keyboard model; the Tab order is Tools, Group, Path tools, Path, Boolean. At a window of 800 x 600 the card is fully visible.
2. Given the Offset tool button or the key **O** (focus not in a text field), then the Offset tool becomes active with the active-tool look; exactly one tool is active across the Tools card and the Path tools card. The selection is kept when the tool changes. The Path card and the Boolean card stay commands and never show the active look.
3. Given the app, then no Offset button exists in the Select bar and no Offset section exists in the Properties panel, and while the Offset tool is active, in use or finished, the Properties panel shows no section that was not there before: the panel is unchanged.

### The Offset bar

4. Given the Offset tool is active, with or without a selection, then a bar is shown at the top of the canvas (the overlay row, centred like the Node bar) with these controls, always present and never added or removed: **Distance** (value field, `mm` suffix, unsigned), **Direction** (radio group "Direction": Out, In, Both; default Out), **Corners** (radio group "Corners": Round, Miter, Bevel; default Round), **Ends** (radio group: Round, Flat, Square; default Round), **Result** (radio group "Result": New object, Change original; default New object), **Apply** (text button, accessible name "Apply offset") and a **status** text (polite live region, fixed 200 px box).
5. Given a control does not apply, then it is greyed (`aria-disabled`) and the status text names the reason: with nothing selected all controls except the radio groups' display are greyed and the status reads "Select one or more shapes."; Ends is greyed unless an open path is selected; Apply is greyed with nothing selected. With a group alone selected the handles are not drawn, the bar is greyed and the status reads "Groups cannot be offset. Ungroup first.".
6. Given the Distance field, then it accepts 0.01 to 1000 mm (a value outside is clamped to the range with the field marked invalid until corrected); the default is 1 mm on first use, then the last value used in the session, whether dragged or typed. A drag on the canvas writes its value into the field live.
7. Given the bar values (Distance, Direction, Corners, Ends, Result), then they are session state: not saved in the file, not part of undo. A change in the bar after a drag does not recompute the last result; it affects the next drag or Apply only.
8. Given a window of 800 x 600, then the bar wraps by whole groups and no control is cut off or unreachable.

### The handles and the drag

9. Given the Offset tool and a selection of at least one operand, then four round handles are drawn, one per corner of the selection box, each 12 px outside the corner on the box's outward diagonal (box: the selection box of one rotated object in its own rotated frame, otherwise the axis-aligned box around all selected objects; an axis smaller than 24 px on screen is widened to 24 px for placing the handles only). The handle is 10 px with a hit area of 16 px, in the accent colour. A drag on an object itself does not move it: only handles act in this tool.
10. Given a drag of a handle, then d is the pointer's movement projected onto the handle's outward diagonal, `d = (dx * sx + dy * sy) / 2` in document millimetres with (sx, sy) the corner's outward direction (each plus or minus 1 in the box frame). Moving the bottom right handle 1 mm right and 1 mm down makes an Outset of 1 mm; moving the top left handle 1 mm left and 1 mm up does the same. Outward is Outset, inward is Inset; passing through the start point switches live; at 0 there is no preview and a release writes nothing. The Direction in the bar is set from the drag (outward Out, inward In, with Shift Both) and stays so. d is limited to 1000 mm.
11. Given a drag, then d snaps to a step of at least 2 screen pixels from the series 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5 mm (1 mm at 100 % zoom, 0.2 mm at 400 %), and snaps to the bar's Distance value when the pointer is within 6 px of it. No key turns snapping off.
12. Given an Inset drag whose d would make the whole selection vanish, then the handle stops at the last d where something is left; the readout says "Nothing is left" in the invalid colour; a release there writes nothing. Given only some operands vanish, then the drag goes on and the readout adds "1 of 3 shapes vanishes".
13. Given a double-click on a handle, then focus moves to the Distance field of the bar.

### Preview, release, cancel

14. Given a drag, then the result is shown as a blue (`--preview-new`) 1.5 px hollow outline with a white casing over the unchanged original, every outline of the result (holes included), no fill, solid (not dashed); with Both, two outlines. The preview is never stored. It updates once per animation frame. The original operands stay black.
15. Given a drag, then a readout chip near the pointer has two lines. Line 1: "Outset 2.5 mm", "Inset 0.8 mm" or "Both ±2.5 mm". Line 2 (muted): "New object", "Changes the original" (Ctrl), or "Changes the original, adds the inner copy" / "Changes the original, adds the outer copy" (Shift+Ctrl, naming the side that is the copy). The bar status also says "Stays a rectangle." or "Becomes a path." (and likewise for the other primitives) while dragging. With Ctrl not held, the plus badge of the copy-move is shown beside the pointer.
16. Given a drag with d not 0 and no Escape, when the maker releases the pointer, then the result is written as **one commit** stored as `offset_path`, labelled "Offset" in the history, which adds, changes or both for all operands atomically (`0016` criterion 28). There is no second Apply step after a drag. Ctrl+Z takes the whole step back.
17. Given a drag, when the maker presses Escape (the first step of the cascade of `0010` criterion 42) or presses the other mouse button, then the preview disappears, nothing is written and the handles return.
18. Given an operand of 200 nodes, then the preview updates within 50 ms of each pointer move on the reference desktop in a release build for an outset of 2 mm and an inset of 1 mm. A larger operand may update later, but the pointer, the handles and the readout never block (they follow the pointer at frame rate) and a stale preview is dropped. Given more than 2,000 operand nodes, then the preview is replaced by the grown selection box (dashed `--preview-new`), the readout says "Preview off: large selection", and the release computes the real result. What is written is exactly what the kernel computes for the final d.

### Modifier keys and what is created

19. Given the table below, then the release (the key state at release counts; keys may be pressed before or during the drag; the preview, readout and bar follow within one frame of a key change) gives:

| Held | Result |
|---|---|
| none | one side from the drag direction: a **new object**, the original unchanged |
| Shift | both sides at the dragged distance: **two new objects** (outer and inner), the original unchanged |
| Ctrl | one side: **the original is changed**, no new object |
| Shift+Ctrl | both sides: the original takes the **dragged** side (drag outward: it becomes the outset), a **new object** is made for the other side |

With the Result choice in the bar set to Change original, the default is inverted and Ctrl gives New object; the bar shows the effective choice while Ctrl is held. The release reads the modifier state from its own event (Linux/WebKitGTK reports a late Shift key-up, `0014`).

20. Given a new object, then it is a copy of its operand's **complete style**, placed directly **below** its operand in the stacking order when it is an Outset and directly **above** it when it is an Inset. With Both, the outer copy is below, the inner above, the original between them. A filled outset therefore never covers the part.
21. Given Change original, then the original keeps its place in the stacking order, its name and its id (linked clones follow it); only its geometry (or, for a primitive, its fields) changes.
22. Given several operands, then each is offset on its own with the same d, and each gets its own result; results are never merged with each other (Union merges them when wanted).
23. Given a release, then the tool stays active and the selection is the objects the drag produced or changed (the new objects, the changed originals, or with Shift+Ctrl the changed original and its copy); unchanged originals are not selected. A second drag therefore offsets the result and gives concentric copies.
24. Given a release, then a notice (`role="status"`, bottom centre of the canvas, 3 seconds) names what happened: "Offset: added 1 shape, 2.5 mm outward.", "Offset: changed 1 shape, 2.5 mm inward." or "Offset: changed 1 shape and added 1, 2.5 mm both sides.". Given some operands vanished, it adds "1 of 3 shapes vanished".

### Keyboard route

25. Given the Offset tool, a selection, and no pointer use, then the maker can do the whole task by keyboard: **O** selects the tool; Tab reaches the bar in the order Distance, Direction, Corners, Ends, Result, Apply; arrows move inside each radio group (one Tab stop each); in Distance the arrows change the value by 0.1 mm, Shift by 1 mm, Ctrl by 0.01 mm. While focus is in the bar and a selection exists, the preview for the bar values is shown. Enter in Distance commits a typed value and shows its preview (no preview while typing, `0017` criterion 42); Enter again or Apply writes it as one commit (criterion 16); focus stays in the bar. Escape in the bar restores a typed value and then returns focus to the canvas. The accessible names are "Offset tool", "Distance, millimetres", "Direction", "Corners", "Ends", "Result", "Apply offset". Arrow keys on the canvas nudge as in the Select tool and do not offset.
26. Given the Apply route with Result and Direction from the bar, then the modifier table of criterion 19 is expressed by the bar: Both is Direction Both, Change original is Result Change original; Direction Both together with Change original gives the Shift+Ctrl behaviour with the Out side as the original's side.

### Selecting in the tool

27. Given the Offset tool, then a click on an object selects it, Shift+click adds or removes it, a press on empty canvas clears the selection, the marquee and Ctrl+A work as in the Select tool.

### The result (kernel behaviour, kept from the first specification)

28. Given a closed operand and d, then the result's region is the operand's region grown by d (every point within d of the region) or shrunk by d (every point of the region farther than d from its boundary), with the Corners rule at corners. Holes behave as parts of the boundary: growing a ring shrinks its hole, shrinking it grows the hole. A result with one outline and no hole is an ordinary closed path, otherwise one compound path (`0016` criterion 20).
29. Given an Inset that makes parts vanish, then the parts that vanish are gone from the result. Given every part of every operand vanishes at release or Apply, then it is refused with a chip "Inset is empty: nothing is left at 10 mm. Nothing was changed." (the number is d), nothing written (`0016` criterion 17). A result of no area counts as empty. Given some operands vanish and others not, the survivors get their results (criterion 24).
30. Given an open path with Out or Both, then the result is a closed outline of all points within d of the path: both sides at distance d, joined by the Corners rule at its corners and finished by the Ends rule at its ends; one closed path (a compound path if the path crosses itself and leaves a hole). Both on an open path is the same outline as Out. Given an open path with In, then the handle stops at 0 when only open paths are selected and the readout says "Inset needs closed paths"; with a mixed selection the open ones are left out and the notice names them ("Inset needs closed paths: 1 of 2 is open."); with Apply the same sentence appears in the status and the open ones are outlined in red as `0016` criterion 15, and nothing is written when no closed operand remains.
31. Given a self-crossing operand, then its region is the area the canvas paints (nonzero), as `0016` criterion 7, and the result is a clean outline without self-crossings.
32. Given a group among the operands, then a release or Apply is refused with "Offset does not work on groups. Ungroup first. Nothing was changed." and the group outlined (`0023-groups` criterion 28); a group alone draws no handles (criterion 5). Members of an entered group can be offset. A coordinate that is not finite, an operand beyond `MAX_COORDINATE_MM`, or a result beyond it, is refused as `0016` criterion 17a. A closed operand that encloses no area (all nodes on one line) is refused as `0016` criterion 16.

### Curves stay curves

33. Given a result path, then it consists of lines and cubic Béziers; a node where the neighbouring pieces have the same tangent (within 0.1 degree) is a smooth node, any other node is a corner node; no node lies on the straight line between two straight neighbours (`0016` criterion 24); no two consecutive nodes are closer than 0.001 mm. Winding follows `0016` criterion 41. A path result is rotation 0 (`0016` criterion 27).
34. Given any operand, then the result outline lies within 0.01 mm of the **exact offset of the original curves** (not of a flattened copy), for every Corners choice. A straight edge of the operand gives one segment; a Round corner or end gives one cubic per 90 degrees of turn or less (45 degree pieces when d is above 37 mm, so that the arc stays within 0.01 mm).
35. Given a circle (4 cubic nodes, as a path), Outset 2 mm Round, then the result is one closed path of **4 cubic segments**, all nodes smooth. Given the same circle as a circle primitive, criterion 40 applies.
36. Given a rounded rectangle as a path (4 lines and 4 quarter-arcs), Outset 2 mm Round, then the result is a closed path of **8 segments** (4 lines and 4 cubics). Given a rectangle of 20 x 20 mm as a path with sharp corners, Outset 2 mm Round, then the result has 8 segments (4 lines, 4 cubics); with Miter it has 4 lines.
37. Given a non-circular ellipse as a path (4 cubic nodes), Outset or Inset, then the result is a path of at most 8 cubic segments (smooth nodes), within 0.01 mm of the exact offset.
38. Given the 200-node wavy test shape of `tests/boolean_performance.rs` (the "gear"), Outset 2 mm Round, then the result has at most 400 segments (estimated 200 to 300; the architect's spike may correct this number on purpose, not silently) and at most as many segments as the polyline of the first build for the same input.
39. Given any outline for which the pieces cannot be matched to the library's topology within 0.01 mm, then that outline is written as a polyline within 0.01 mm of the exact offset, with no more nodes than the first build gave. A budget test prints how often the fallback fires on the test set; none of the numeric tests of criteria 45 to 51 may end in the fallback.

### Primitives stay primitives

40. Given a single primitive operand, then the result stays the same kind of primitive, with its rotation and centre kept and its own fields set, according to this table (r is the effective corner radius after CSS clamping, d the distance, N the point count, R the circumradius, h the distance of an edge line from the centre; every "stays" result is exact to the kernel tolerance; every other result is a path or compound path under criteria 33 to 39):

| Operand | Out | In |
|---|---|---|
| Rectangle, sharp corners | Miter: rectangle +d on each side, radii 0. Round: rectangle +d on each side, every radius d. Bevel: a path of 8 lines | rectangle -d on each side, radii 0, every Corners choice |
| Rectangle, corner radii r_i | per corner: r_i > 0 gives r_i + d for every Corners choice; r_i = 0 follows the Corners choice as above (Bevel at a sharp corner gives a path) | rectangle -d on each side, radius max(r_i - d, 0) per corner |
| Circle (rx = ry) | circle R + d, every Corners choice | circle R - d |
| Ellipse (rx not ry) | a path (criterion 37), except that it stays an ellipse when the largest deviation of the ellipse (rx + d, ry + d) from the exact offset, sampled deterministically, is at most 0.01 mm | a path (always, when d is above ry squared / rx the ends have sharp points), same exception |
| Regular polygon | Miter: polygon with R + d / cos(pi / N), same N. Round: a path of N lines and N arcs. Bevel: a path of 2N lines | polygon with R - d / cos(pi / N), every Corners choice |
| Star | Miter, while no tip is clipped by the limit: the star scaled about its centre by (h + d) / h, same N and ratio. Otherwise, and for Round and Bevel: a path | Miter, while no inner corner is clipped: scaled by (h - d) / h. Otherwise, and for Round and Bevel: a path |

41. Given a primitive whose inset removes it (rectangle: d at least half the shorter side; circle: d at least R; polygon and star: d at least the apothem or h), then it vanishes under criterion 29. A rotated rectangle, ellipse, polygon or star keeps its rotation in a "stays" result; a path result is rotation 0.
42. Given Change original on a primitive that stays a primitive, then it keeps its kind, id and place and only its fields change. Given one that would become a path, then it is converted to a path with the same id and place, and the readout says "Becomes a path".
43. Given Both on a primitive, then each side follows the table on its own (an outset Round rectangle and an inset rectangle are both rectangles).

### Determinism

44. Given the same operands and bar values, then the result is node for node the same, with the same segment count, however they were selected and on every platform, to 1e-6 mm (`0016` criterion 43). The cross-target determinism check of `0016` includes curved offsets (circle path, ellipse path, rounded rectangle path, the gear).

### Numeric tests

Areas are measured on the exact curve result (cubics integrated); tolerances allow for the cubic approximation of arcs (error up to 2.7e-4 x radius). "Path" means the operand is an ordinary path, not a primitive.

45. A square path of side 20 mm with its corner at (0, 0): Outset 5 mm with Miter gives a square from (-5, -5) to (25, 25) of 4 nodes. With Round the area is 400 + 4 x 20 x 5 + pi x 25 = 878.54 mm² within 0.05 mm², in 8 segments; with Bevel it is 400 + 400 + 4 x 12.5 = 850 mm² (exact to 0.01 mm²). Inset 5 mm gives the square (5, 5) to (15, 15), area 100 mm², with every Corners choice. Inset 10 mm is refused as empty, and Inset 9.99 mm gives a square of side 0.02 mm.
46. A ring path (a disc of radius 20 mm minus a concentric disc of radius 10 mm, a compound path): Outset 2 mm Round gives outer radius 22 and hole radius 8 (area pi x (484 - 64) = 1319.5 mm² within 3 mm²); Inset 2 mm gives outer 18 and hole 12 (area pi x (324 - 144) = 565.5 mm² within 3 mm²); Inset 5 mm: the hole radius 15 equals the outer radius 15, the ring vanishes (refused as empty).
47. An open line from (0, 0) to (20, 0): Outset 5 mm gives a closed path of area 20 x 10 + pi x 25 = 278.54 mm² with the Round end (within 0.1 mm²), 200 mm² with the Flat end (the rectangle (0, -5) to (20, 5)), and 300 mm² with the Square end (from (-5, -5) to (25, 5)). Inset of the line is refused (criterion 30).
48. A concave corner: an L-shaped closed path, Outset 2 mm: the convex corners follow the Corners choice; the one reflex corner stays a sharp corner, since the offset lines overlap there and no gap opens. Inset 2 mm: the reflex corner follows the Corners choice (a gap opens), the convex corners stay sharp.
49. Miter limit: a square path (90 degree corners), Outset 5 mm, Miter: the tips lie 5 x sqrt(2) = 7.07 mm from the corners, below the limit of 20 mm, so they are sharp points (criterion 45). A triangle path with one corner of 20 degrees, Outset 5 mm, Miter: the tip would lie 5 / sin(10 degrees) = 28.8 mm from that corner, above 20 mm, so the corner is clipped at the limit: it gets two nodes, each 20 mm (4 x 5 mm) from the original corner within 0.01 mm and symmetric about its bisector, and no node 28.8 mm from it. The other two corners (80 degrees each) stay sharp.
50. A circle primitive of radius 20 mm: Outset 2 mm, any Corners choice, gives a circle primitive of radius 22 mm (same centre, same style); Inset 2 mm gives radius 18 mm; Inset 20 mm is refused as empty. A rectangle primitive of 100 x 50 mm with corner radius 5 mm, Outset 2 mm, any Corners choice, gives a rectangle of 104 x 54 mm with radius 7 mm; the same rectangle rotated by 30 degrees gives the same sizes and still rotated by 30 degrees. A sharp rectangle of 20 x 20 mm, Outset 2 mm Round, gives a rectangle of 24 x 24 mm with radius 2 mm; with Miter, 24 x 24 mm with radius 0. A regular hexagon of circumradius 10 mm, Outset 2 mm Miter, gives a hexagon of circumradius 10 + 2 / cos(30 degrees) = 12.309 mm (within 0.001 mm), same rotation.
51. An ellipse primitive of 40 x 20 mm (rx = 20, ry = 10), Outset 2 mm: the result is a path (not an ellipse of 44 x 24 mm, which deviates by about 0.1 mm) of at most 8 cubic segments, every point within 0.01 mm of the exact offset curve.

## Out of scope

- A **live offset that stays linked** to its source (Inkscape's Dynamic and Linked Offset, Affinity's Contour), and "adjust the last offset" after the drag.
- **Several offsets at once** (a count, concentric copies in one drag, hatching) and a **variable distance**. Repeated drags give concentric copies (criterion 23).
- A **miter limit setting** (fixed at 4), a **precision setting** (decision 5; if ever wanted it is an app preference "Curve precision", not an Offset field), caps for closed paths.
- **Offsetting strokes** (a stroke's width is not part of any region), **text**, **images**; **groups** as operands (criterion 32).
- **A second path tool** in the "Path tools" card (a knife, a width tool). Whether a second button fits at 800 x 600 is measured when it is proposed.
- **Refitting curves of Boolean results** (separate, `0016` follow-up; the technical-debt entry "Boolean results are polylines").
- **V-carving and CNC tool-radius compensation** (R-MFG-CNC-001, R-MFG-CNC-002): these use offsetting in the job generator and are specified with the CNC slices.
- **Document units other than mm** in the Distance field (`0017` criterion 48).
- Alt as a modifier (window managers on Linux take Alt+drag), and a key that turns snapping off.

## Open questions

None. All questions of the rework concept were answered by the customer on 2026-10-12 with the defaults (see "Decisions"). The questions of the first specification (keep or replace the original, name, Round default, miter limit 4, ranges) are answered by the tool: Ctrl and the Result choice, the tool name "Offset", the defaults of criteria 4 and 6.

## UX notes

The interaction design is `ux-rework.md` in this folder (accepted by the customer on 2026-10-12): the "Path tools" card (section 1), the Offset bar (section 2), the handles and the readout (section 3), modifier keys (section 4), z-order and selection after (section 5), primitives (section 6), multi-selection, groups and large documents (section 8), the keyboard route (section 9). It replaces the UX notes of the first specification. Mockup: `ux-rework-mockup.html`. Design-system rows to change when built: "Command with parameters" and "Offset section" (removed), "Tool rail architecture" (column A gets the Path tools card), new rows "Offset bar" and "Offset handles".

## Links

Requirements: R-EDIT-007 (`docs/requirements.md`)
Builds on: `specs/0016-boolean-operations/` (kernel, compound paths, region, refusals, notices, determinism), `specs/0009-unified-object-editing/` (blue-new/black-old), `specs/0010-edit-interaction-polish/` (Escape cascade), `specs/0013-*` (corner radii of rectangles), `specs/0017-style-panel-rework/` (value fields)
Related: `specs/0023-groups/` (groups refused), `specs/0037-fracture-and-flatten/`, `specs/0033-stroke-brushes/`, `docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md` §3, §4
Design documents: `ux-rework.md`, `geometry-rework.md` in this folder
ADRs: `adrs.md` (ADR 0003 §4 amendment of 2026-10-12, approved by the customer)
PR: https://github.com/curvyo/curvyo/pull/115 (draft, to be reworked)
