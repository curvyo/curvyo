# Path offset: UX rework concept (for discussion)

Status: Draft for the customer, 2026-10-12, by `ux-engineer`. Replaces the "UX notes" of
`specification.md` once the customer agrees; the criteria that change are listed at the end.
Mockup: `ux-rework-mockup.html` in this folder (open it in a browser).

## Why the built slice was rejected

Customer feedback on PR #115 (2026-10-12, translated): the Offset section appeared at the top of
the Properties panel out of nowhere and pushed the Style rows down. In the right panel nothing
appears or disappears: sections are collapsed, expanded or greyed out. Offset is not a property of
the object, it is an operation, and in the panel the maker cannot see when it takes effect.

What follows from that, as rules for this concept:

1. **Offset is a tool**, like the Node tool: a rail button, its options in the bar at the top, its
   own handles on the canvas. The Properties panel is not involved at all.
2. **It takes effect on release.** The drag shows the result in blue; releasing the pointer writes
   it; Escape during the drag writes nothing. There is no second "Apply" step after a drag.
3. **Nothing appears out of nowhere** in the bar either: every control is always there and greyed
   when it does not apply (the Node bar's rule), never added or removed.

## Reference tools (as far as I know them)

- **Inkscape:** Path > Outset / Inset (Ctrl+) and Ctrl+() replace the path by a fixed step set in
  Preferences. **Dynamic Offset** (Ctrl+J) turns the path into an offset object with one handle on
  the outline that is dragged to set the distance; **Linked Offset** does the same on a linked copy.
  The handle on the canvas is the idea we take; the conversion into a special object type is not.
- **Illustrator:** Object > Path > Offset Path, a dialog (Offset, Joins, Miter limit, Preview).
  Adds a new path, keeps the original. We take "keep the original" as default.
- **LightBurn:** Offset Shapes, a dialog with Distance, Corner style, Direction (Outward, Inward,
  Both) and "Delete original". We take Direction "Both" and the replace choice, but on the canvas
  and as modifier keys instead of a dialog.
- **What we do better:** the distance is dragged on the canvas with a live result and a number at
  the pointer, typed in the bar when it must be exact, both sides in one drag (Shift), and
  rectangles and circles stay rectangles and circles.

## 1. Where the tool lives: a "Path tools" toolbox

| | |
|---|---|
| Card | A new toolbox card **"Path tools"** in rail column A, **8 px under the Group card** (top 432 px, card 48 px high with one button, ends 480 px below the viewport top; 32 px clear at 800 x 600). The look of every toolbox card (row "Toolbox card"). |
| Button | **Offset tool**, 40 x 40 px, a 20 px glyph: a solid rounded square inside a dashed concentric outline (the result around the source). Tooltip "Offset tool (O)", second line muted "Drag a corner handle. Shift: both sides. Ctrl: change the original." |
| Kind | These are **tools**, not commands: the button shows the solid `--toolbar-icon-active-bg` of the active tool and belongs to the same set as the Tools card (exactly one tool is active, in either card). The Path card in column B (Combine, Split, Break apart, Cut) and the Boolean card stay **commands**: one press, no mode, never the active look. |
| Letter | **O** (free; Inkscape uses it for the connector tool, which Curvyo does not have). A tool letter like N, B, R, E, P. |
| Keyboard | `role="toolbar"`, name "Path tools", one Tab stop, the rail keyboard model. Tab order: Tools, Group, Path tools, Path, Boolean. |

**How it relates to the Path card.** The difference the maker sees: a press on a Path card button
does something at once to the selection; a press on a Path tools button changes what the pointer
does on the canvas and shows a bar at the top. The two cards have different names ("Path tools",
"Path operations") and the active-tool blue only ever appears in column A.

**Why a new card and not the Tools card:** the Tools card holds the tools that create or select;
an eighth button would push the Group card down 44 px (the rule of the rail allows it, but it moves
a card under the hand). A card of its own leaves every built button where it is and gives later
path tools a home (candidates: a knife tool that cuts along a dragged line, a width tool for
variable strokes). **Space:** with one button the card fits at 800 x 600. A second button (92 px
card, ending 524 px) fits only if the viewport is taller since `0051` removed the menu bar; this is
measured at build. If it does not fit, a second path tool is a customer decision (see question 1).

**What goes away:** the "Offset" text button in the Select bar and the Offset section of the
Properties panel. The kernel, the preview renderer and the plan code of the built slice are reused.

## 2. The bar at the top (Offset bar)

The bar is in the overlay row, centred like the Node and shape bars, a pill with 36 px rows and
28 px controls. It is there whenever the Offset tool is active, also with nothing selected; then
every control is greyed and the status text says "Select one or more shapes.".

| # | Group | Control | Default | Notes |
|---|---|---|---|---|
| 1 | Distance | label "Distance", the bar value field (80 px, `mm` suffix, unsigned) | 1 mm, then the last value used in the session (dragged or typed) | Typed 0.01 to 1000 mm. A drag on the canvas writes its value here live. |
| 2 | Direction | segmented text items **Out**, **In**, **Both** (44 px each, `role="radiogroup"` "Direction") | Out | A canvas drag sets it from the drag (outward Out, inward In, Shift Both) and it stays so. |
| 3 | Corners | three icon items **Round**, **Miter**, **Bevel** (the stroke Join glyphs, 32 px each, `role="radiogroup"` "Corners") | Round | Label "Corners", not "Join": the maker's word. |
| 4 | Ends | three icon items **Round**, **Flat**, **Square** (the stroke Cap glyphs) | Round | Greyed unless an open path is selected. Always in the bar. |
| 5 | Result | segmented text items **New object**, **Change original** (`role="radiogroup"` "Result") | New object | Ctrl during a drag inverts it for that drag only; the bar shows the inverted choice while Ctrl is held. |
| 6 | Apply | text button **Apply** (the filled look) | | Applies the bar values to the selection. Greyed with nothing selected. The keyboard route; the mouse route is the drag. |
| 7 | Status | muted 12 px text at the end, fixed 200 px box, polite live region | empty | The reason a control is greyed or a drag is refused: "Select one or more shapes.", "Groups cannot be offset. Ungroup first.", "Inset needs closed paths: 1 of 2 is open." |

Groups are separated by the bar's 1 px 20 px divider and 12 px gap. About 860 px wide; one row
from a window of about 1330 px with the panel open (or about 1050 px with it collapsed); at
800 x 600 (324 px overlay row) it wraps by whole groups to four rows, as the Node and Pencil bars
wrap. The bar values are session state, never saved in the file.

**Not in the bar:**

- **Miter limit.** Fixed at 4 (SVG's), as in the spec. A field that only matters for Miter would be
  greyed most of the time; add it if a maker asks.
- **Precision.** Not a maker setting: the result must be exact to 0.01 mm and keep curves as curves
  (section 7). A precision field would hand the kernel's problem to the maker.
- **Count / spacing** (several concentric copies). A follow-up; repeated drags already do it
  (after a drag the result is selected, so the next drag offsets the result).

## 3. The handles on the canvas

**Look.** Four handles, one per corner of the selection box, each sitting **12 px outside the
corner on the box's outward diagonal**, so it never covers a rectangle's corner node or a transform
handle position. Glyph: the "Parameter handle" (10 px circle, 1.5 px `--accent` ring, white ground,
4 px `--accent` centre dot), hover `--accent-hover`, being dragged `--accent` ground; hit area 16 px.
The box is drawn as the selection box (1 px dashed `--accent`). One rotated object: its own rotated
box (as the transform handles); several objects: the axis-aligned box around all of them. A box
smaller than 24 px on screen in an axis (a straight horizontal line) is widened to 24 px in that
axis for placing the handles only.

**What a drag maps to.**

- The distance d is the pointer's movement **projected onto the handle's outward diagonal**, in
  document millimetres, as the amount the box grows on each side: `d = (dx * sx + dy * sy) / 2`,
  where `(sx, sy)` is the corner's outward direction (each ±1 in the box frame). Moving the handle
  1 mm right and 1 mm down from the bottom right corner is an outset of 1 mm.
- **Outward (away from the box centre) = Outset, inward = Inset.** Passing through the start point
  switches between them live. At 0 the preview is gone and a release writes nothing.
- The handle stays on the diagonal and moves with the grown box, so it is always at the corner of
  the result's box (exact for Round corners; with Miter a sharp corner may reach a bit further).
- **Inset is limited:** the handle stops where the whole selection would vanish; at that point the
  readout says "Nothing is left" in `--field-invalid` and a release there writes nothing. If only
  some shapes vanish, the drag goes on and the readout adds "1 of 3 shapes vanishes".
- **Snapping:** d snaps to a step that is at least 2 screen pixels, from the series 0.01, 0.02,
  0.05, 0.1, 0.2, 0.5, 1, 2, 5 mm (1 mm at 100 % zoom, 0.2 mm at 400 %). It also snaps to the value
  in the bar's Distance field when the pointer is within 6 px of it, so a typed 2 mm can be dragged
  to exactly. No key turns snapping off (Ctrl is taken, Alt is unreliable on Linux): zoom in for a
  finer step, or type the value.

**Preview.** While dragging, the result is drawn as the "Live preview outline": a hollow 1.5 px
`--preview-new` line with the white casing over the unchanged original, every outline of the result
(holes included), no fill. With Both, two outlines. The original stays as it is drawn ("blue new,
black old"). Solid, not dashed: dashed is the selection box. Updated once per frame.

**Readout chip** (the "Live transform readout", 12 px up and right of the pointer), two lines:

- Line 1: "Outset 2.5 mm", "Inset 0.8 mm", "Both ±2.5 mm".
- Line 2, muted: "New object", "Changes the original" (Ctrl), "Changes the original, adds the inner
  copy" (Shift+Ctrl, naming the side that is the copy).
- With Ctrl not held and a new object being made, the plus badge of the copy-move is shown beside
  the pointer (row "Modifier badge"), so the move's "plus = a new object" holds here too.

**Release and Escape.** Release writes the result as **one undo step** ("Offset"). Escape during
the drag (the first step of the cascade) or a press of the other mouse button cancels: the preview
goes, nothing is written, the handles return. A double-click on a handle focuses the Distance field
of the bar (the convention "double-click a handle to type its value").

**After the release:** the tool stays active; the result is the selection (rules in section 5), so
its handles are ready for the next drag. A notice in the canvas notice slot (bottom centre, 3 s,
`role="status"`): "Offset: added 1 shape, 2.5 mm outward." / "Offset: changed 1 shape, 2.5 mm
inward." / "Offset: changed 1 shape and added 1, 2.5 mm both sides."

## 4. Modifier keys

| Held during the drag | Effect | Result |
|---|---|---|
| none | one side, from the drag direction | a new object, the original unchanged |
| **Shift** | **both sides** at the dragged distance | two new objects (outer and inner), the original unchanged |
| **Ctrl** | one side | **the original is changed**, no new object |
| **Shift+Ctrl** | both sides | the original takes the **dragged** side (drag outward: it becomes the outset), a new object is made for the other side |

Keys may be pressed before the press or during the drag; the state at release counts; the preview,
the readout and the bar follow in the frame a key changes (the move's rule). On Linux/WebKitGTK the
Shift key-up may arrive late (known, `0014`); the release reads the modifier from its own event.

**Conflicts with the meanings elsewhere, and why they are acceptable:**

- **Shift elsewhere means "about the centre" or "one axis"** (resize, create, move). Here it means
  "both sides of the outline": also the symmetric choice, so the meaning is close. There is no axis
  to lock in an offset.
- **Ctrl elsewhere means "copy" for a move and "keep proportions / snap angles" for a resize or
  rotate.** Here the default already makes a copy, so Ctrl gives the other choice: change the
  original. The common rule is "Ctrl = the other result than the default", and the readout's second
  line and the plus badge say which one the release gives. This is the one real inversion
  (question 2). Ctrl cannot also mean "snap" here; snapping is always on (section 3).
- **Alt** is not used (window managers on Linux take Alt+drag).

## 5. What is created, z-order, selection after

- **Style:** each result copies its original's complete style.
- **Each object is offset on its own.** Several selected objects give one result each, never
  merged (as in the spec). Overlapping outsets of neighbours stay separate objects; Union merges
  them when wanted.
- **Z-order, proposal (question 3): the bigger one goes below.** An outset copy goes directly
  **below** its original, an inset copy directly **above** it. Then a filled border never covers
  the part, and an inner contour is visible on a filled part. The built slice puts every result
  above its original.
- **With Both:** outer copy below, inner copy above, the original in between.
- **With Change original (Ctrl):** the original keeps its place in the stacking order, its name,
  its id (and so its linked clones follow it), only its geometry changes.
- **Selection after the release:** the objects that the drag produced or changed: the new copies,
  or the changed originals, or with Shift+Ctrl the changed original and its copy. The unchanged
  originals are not selected. So a second drag offsets the result, which gives concentric copies.

## 6. Rectangles, circles, polygons, stars: stay primitives where the result is exact

The customer asked for this, and it is a real advantage: the result keeps its radius knobs, its
size fields and its exact curves. **Rule: the result stays the same kind of primitive whenever the
exact offset is that kind; otherwise it becomes a path (with curves, section 7).** The bar's status
says which: "Stays a rectangle." / "Becomes a path." while dragging (second readout line too).

| Original | Out (outset) | In (inset) |
|---|---|---|
| **Rectangle**, any corner radii, rotated or not | **Round:** rectangle w+2d, h+2d, every corner radius r+d (a sharp corner gets radius d). **Miter:** rectangle w+2d, h+2d, sharp corners stay sharp, rounded corners get r+d. **Bevel:** a rectangle only if every corner is already rounded (r+d); with a sharp corner it becomes a path (Curvyo's rectangle has no chamfer). | Every corner style: rectangle w−2d, h−2d, each radius max(r−d, 0). Vanishes when d reaches half the shorter side. |
| **Circle** (rx = ry) | circle r+d, every corner style | circle r−d; vanishes at d ≥ r |
| **Ellipse** (rx ≠ ry) | **a path.** The exact offset of an ellipse is not an ellipse (an ellipse with rx+d, ry+d is off by about 0.08 mm for a 40 x 20 mm ellipse at 2 mm and about 0.5 mm for an 80 x 20 mm one at 5 mm, far above the 0.01 mm tolerance). | a path; a narrow ellipse inset further than its tightest curve radius gets pointed ends |
| **Polygon** (regular) | **Miter:** polygon of the same count and angle, outer radius R + d / cos(π/n) (no corner of a regular polygon is sharp enough to reach the miter limit). Round or Bevel: a path. | Every style: polygon with outer radius R − d / cos(π/n). |
| **Star** | **Miter:** a star with the same points and ratio, scaled about its centre (all edge lines of a regular star are at the same distance from its centre, so the offset is a scaling), as long as no point is clipped by the miter limit 4; otherwise a path. Round or Bevel: a path. | Miter: the same scaled star unless an inner corner is clipped; otherwise a path. |
| Path, compound path, open path | path / compound path (open path: a closed outline around it, with the Ends style) | path / compound path; open paths are refused |

With **Both**, each side follows the table on its own (an outset Round rectangle and an inset
rectangle are both rectangles). With **Change original**, a rectangle that stays a rectangle keeps
its kind; one that would become a path is converted, and the readout says "Becomes a path".

The architect confirms the formulas and the star case (open question for the architect, A1). The
spec's criteria 6, 10, 11 and 20 change for primitives.

## 7. Output quality (requirement from UX, geometry by the architect)

A curved original must give a result that a maker can edit and a machine can run smoothly:
**curves stay curves** (Bézier segments, smooth nodes), and the node count grows with the original's
node count, not with its size. A circle outset is a circle (section 6); a path with 4 Bézier nodes
gives a result with a comparable number of curve nodes, not hundreds of corners. Exactness 0.01 mm
as before. How (curve offset, refitting the polyline, arcs) is the architect's decision (A2). Until
that exists, the built polyline is a known limitation and must not ship as the final result,
because it is what the customer saw.

## 8. Multi-selection, groups, large documents

- **Multi-selection:** one box, four handles, one d for all; each object offset on its own;
  primitives and paths mixed (each by its own rule in section 6). One undo step for all.
- **Groups:** refused as in the spec: the handles are not drawn, the bar is greyed, the status says
  "Groups cannot be offset. Ungroup first.". A group together with other objects: the handles are
  drawn; release is refused with the existing sentence and the group outlined (row "Refusal
  outline"). Offsetting group members inside an entered group works.
- **Open paths:** In is refused: the handle stops at 0 for a selection of only open paths and the
  readout says "Inset needs closed paths"; in a mixed selection the open ones are left out and named
  in the notice. Both on an open path is the same outline as Out.
- **Selecting in the tool:** a click on an object selects it, Shift+click adds or removes, a press
  on empty canvas clears, the marquee works as in the Select tool, Ctrl+A selects all. A drag on an
  object does **not** move it (only handles act), so the tool cannot move things by accident.
- **Large documents:** the handles and the readout always follow the pointer at frame rate (they do
  not depend on the geometry). The preview is computed after the frame and a stale one is dropped;
  over 2,000 operand nodes it is replaced by the grown box (dashed `--preview-new`) and the readout
  says "Preview off: large selection"; the release computes the real result. Never block the
  pointer.

## 9. Keyboard-only route

1. **O** selects the Offset tool (or Tab to the "Path tools" toolbox, Space).
2. Select with the keyboard as before (Ctrl+A, or select in the Select tool first; the selection is
   kept when the tool changes).
3. Tab into the bar: Distance (type, or arrows: 0.1 mm, Shift 1 mm, Ctrl 0.01 mm, the panel field
   rule), Direction, Corners, Ends, Result (arrows inside each group), Apply.
4. **While focus is in the bar and a selection exists, the preview is shown** for the bar values,
   so the keyboard user sees what Apply does. Enter in Distance commits a typed value; Enter again
   (or Apply) applies. One undo step. Focus stays in the bar for the next value.
5. Escape in the bar restores a typed value, then returns focus to the canvas.

Accessible names: "Offset tool", "Distance, millimetres", "Direction", "Corners", "Ends",
"Result", "Apply offset". The handles are pointer-only; the bar is their equivalent. Every
greyed control is `aria-disabled` with the reason in the status text. Contrast as the existing
bar (Apply white on `--toolbar-icon-active-bg` 4.5:1, status `--panel-muted-fg` 5.0:1).

## 10. Deliberately not there

- A live, linked offset that follows its source (Inkscape's Dynamic and Linked Offset). Possible
  later as an offset in the object's history or as a linked clone kind; not now.
- "Adjust the last offset" (Blender's redo panel: change the bar after a drag and the last result
  is recomputed). Bar changes affect the next drag or Apply only, so it is always clear when
  something is written.
- Miter limit and precision fields, several copies at once, a variable distance, offsetting strokes
  or text, document units other than mm.
- Arrow keys on the canvas do not offset (they nudge as in the Select tool); the typed route is the
  bar.

## 11. Questions for the customer

1. **Where does the tool go?** *A (recommended, default):* a new card "Path tools" in column A under
   Group, with Offset only; later path tools join it. *B:* the same card, and the **Node tool moves
   into it** (it is a path tool too; the Tools card gets shorter, and a button the hands know moves
   once). *C:* Offset as the eighth button of the Tools card (Group moves down 44 px).
2. **Ctrl = change the original** (your proposal) inverts the move, where Ctrl = make a copy.
   *A (recommended, default):* keep your mapping; the default already copies, Ctrl gives the other
   result, and the readout says which. *B:* default changes the original, Ctrl makes a copy (same
   as the move, but the frequent case, a second contour for cutting, then always needs Ctrl).
3. **Z-order of the result.** *A (recommended, default):* outset copy below the original, inset
   copy above it. *B:* every copy above its original (as built).
4. **Ellipses that are not circles.** *A (recommended, default):* the result becomes a path with
   curves, exact to 0.01 mm. *B:* it stays an ellipse with rx+d and ry+d, editable but off by about 0.1 mm on a
   40 x 20 mm ellipse and up to 0.5 mm on narrow ones.
5. **Tool letter O.** Default O; say if you want another key.

## 12. Questions for the architect

- **A1:** Confirm the primitive rules of section 6 (rectangle radii r ± d with effective radii,
  polygon R ± d / cos(π/n), star as a scaling about its centre and the miter-limit check, circle).
  Is there a case where the "stays a primitive" result differs from the kernel's result by more
  than 0.01 mm?
- **A2:** How the result of curves keeps curves (section 7), and whether that is part of this slice
  or a fix before it.

## What changes in `specification.md` if this is accepted

Criterion 1 (entry: the tool, the bar, the handles instead of the Select bar button and the panel
section), 2 and 3 (preview during a drag and with focus in the bar), 4 (cancel: Escape during the
drag, a press of the other button), 5 (written on release, or Apply), 6 and 11 (primitives stay
primitives, curves stay curves), 10 (z-order, Change original, Both), 13 (the notice texts and
place), "Out of scope" (both directions and replacing the original are now in scope), Question 1
and 5 of the spec (answered by the tool), and the UX notes (replaced by this document). Design
system rows that change: "Command with parameters" and "Offset section" (removed), "Tool rail
architecture" (column A gets the Path tools card), a new row "Offset bar" and "Offset handles".
