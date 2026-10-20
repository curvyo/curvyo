# Arc and curve handles: ellipse arcs, and a Curve handle on ellipse, polygon and star

Status: Done (PR #153, merged 2026-10-19)
Priority: Should
Origin: Customer (Part A); Customer (Part B: reading confirmed 2026-10-06, extended to polygon and star)

## User value

As a maker I want to pull a circle or ellipse into an arc, a pie sector or a
segment with two handles, and pull the sides of a circle, ellipse, polygon or
star in or out with one more handle, so that a half circle cutout, a quarter
disc, a curved slot edge, a diamond, a four-pointed star (a sparkle), a
hexagon with bulging sides or a five-pointed star with concave sides is made
in a few drags from one shape instead of being built node by node.

The customer's words (translated, 2026-10-06): "The circle tool needs handles
to create a circular arc. It would also be practical to squash/stretch the
circle/ellipse at its imagined inner cross using nodes, analogous to Affinity,
into a [star]." The word before "squash" was unreadable; the customer then sent
two screenshots of Affinity Designer's Polygon tool (Sides: 4, option "Curve"),
which Part B is built from. He confirmed the reading and asked for Curve on
polygon and star right away: "polygon und stern auch gleich" (2026-10-06).

**Reference state (2026-10-17).** `main` has `CURRENT_FORMAT_VERSION` 14. The
primitives are `Shape::Rect`, `Shape::Ellipse { frame }`, `Shape::Polygon` and
`Shape::Star`, each with its own `rotation`; mirror and negative scale
(`specs/0056-mirror-and-negative-scale/`), the linked clone
(`specs/0049-linked-clone/`, `specs/0057-clone-overrides/`), click-to-create
(`specs/0052-click-to-create-shapes/`), layers (`specs/0050-layers-and-objects-panel/`)
and history branches (`specs/0042-history-branches/`) are specified or built
beside this slice. Where one of them is not merged when this slice is built,
the criteria that name it (53 to 66) are tested in the later of the two PRs.
**Customer rule (2026-10-16): a primitive stays a primitive wherever the result
is exact.** Arc and curve are parameters of the primitive, never a reason to
convert it (criterion 63).

**Depends on `specs/0009-unified-object-editing/`.** The arc handles and the curve
handle are parameter handles of the one edit mode, the Select tool, not a
feature of the Ellipse tool or the Polygon/Star tool. Part A can be built
without Part B. The curve on polygon and star (criteria 35 to 52) extends the
curve on the ellipse (criteria 17 to 34) and uses the same shared evaluation.

**Field reference.**

- Inkscape's Ellipse tool: Start and End angle fields and two round handles on
  the outline; buttons "Slice" (pie), "Arc" (open) and "Chord" (closed by a
  straight line) and "Make whole". Dragging a handle with the pointer inside
  the circle makes an arc, outside makes a slice. All of it needs the Ellipse
  tool. Inkscape has no curve parameter; a star with curved sides needs a path
  effect or manual nodes.
- Affinity Designer: the Ellipse tool can switch to a pie shape with Start and
  End angle in the context toolbar and a "Close pie" option. Its Polygon tool
  has "Sides" and "Curve" (help: "the convex or concave nature of the sides";
  0% is straight, a negative value is concave; the usable percentage depends on
  the number of sides) and "Smooth points" (only for curve above 0%). With
  Sides 4 and Curve 100% the shape is a circle with the usual eight box
  handles; with Curve about -100% it is a four-pointed star with concave
  sides. A small handle inside the shape, at the middle of one side, drags the
  value. Other side counts work the same way (Curve 100% is a circle for any
  number of sides); our definition for general N is under "Curve on polygon
  and star".
- Ink/Stitch, LightBurn: no equivalent; a LightBurn circle has no handles.

Where we do better: the arc handles and the curve handle are there whenever a
circle is selected, in the same tool as resizing and rotating; the kind (pie,
arc, chord) is an explicit setting, not a side effect of where the pointer is;
angles and the curve can be typed by double-click; the curve lives on the
circle or ellipse itself (stretch it to any aspect afterwards), where Affinity
needs a 4-sided polygon; the same handle works on our star, which Affinity's
Polygon tool cannot make.

## Part A: Arc handles

Terms. The **arc** of an ellipse is given by a start angle, an end angle and a
**kind**. Angles are the ellipse's own parametric angles, measured from its
own right-hand axis (3 o'clock in its own frame) and increasing clockwise on
screen, the product's outline direction. For a circle that is the ordinary
angle. The arc runs from the start angle to the end angle in the direction of
increasing angle. Kinds:

- **Pie** (sector): the arc plus a straight line from each end to the centre.
  A closed shape.
- **Arc** (open): the arc only. An open path with two ends.
- **Chord** (segment): the arc plus a straight line from its end to its start.
  A closed shape.

A start and an end angle that are equal (within 0.1°) mean the whole ellipse.

Part A concerns ellipses whose curve is 100% (a true ellipse, Part B). An
ellipse with a curve other than 100% has no arc (Part B criterion 31).

### Acceptance criteria

1. Given one selected circle or ellipse and handles drawn
   (`specs/0009-unified-object-editing/` criteria 6 to 8, so from a shorter side of
   72 px on screen, the parameter-handle threshold T), then two arc handles are
   shown, a start handle and an end handle, on the outline at the arc's start
   and end angles, together with the transform handles. They are shown on a
   whole ellipse as well, so the maker finds them without a manual. Their
   positions are the "Arc handle layout" of the UX notes: on a whole ellipse
   both rest at the seam of 45° (lower right on screen), 14 px apart along the
   outline, never on a resize handle; they are always distinguishable from
   each other (the end handle comes first, the start handle second, clockwise)
   and reachable. They use the parameter-handle rules of
   `specs/0009-unified-object-editing/` (UX notes sections 1 and 2): the "knob"
   glyph, a hit radius of 12 px, the built-in pointer cursor, not drawn during
   a move, resize, rotate or skew drag of the same object, and no drawn glyph
   closer than 4 px to another (an arc handle that would land within that
   distance of a resize or rotate glyph is drawn displaced along the outline,
   by the "Knob displacement" rule of the UX notes, never overlapping it). Below T the arc is set by the bar (criterion 3).
2. Given the maker drags an arc handle, then that handle's angle follows the
   pointer: with the pointer at (x, y) in the ellipse's own frame relative to
   its centre, the angle is atan2(y / ry, x / rx), whether the pointer is
   inside or outside the ellipse (for a circle, the plain direction from the
   centre to the pointer). The
   other angle and the kind do not change. The first drag of a handle on a
   whole ellipse starts an arc of the kind in
   criterion 3; the other angle then has the value both handles rest at on a
   whole ellipse, the seam of 45° (so the arc starts almost whole and the
   sweep is continuous from 360°). Escape cancels and
   writes nothing; a press and release without 3 screen pixels of movement
   writes nothing.
3. Given the Select tool and a selection that contains at least one ellipse
   (`specs/0009-unified-object-editing/` criterion 21: a control acts on
   exactly the objects of its kind), then the bar shows one ellipse group with
   always-present controls: a three-way choice "Pie / Arc / Chord", a button
   "Whole ellipse" and two number fields "Start" and "End" (degrees). Choosing
   a kind changes the arc's kind in one commit and keeps the angles; "Whole
   ellipse" makes every selected ellipse whole in one commit (the arc is no
   longer stored). Typing a value in "Start" or "End" sets that angle in one
   commit with the rules of criterion 8; on a whole ellipse the other angle is
   the resting seam of 45° and the new arc has the kind "Arc" (decided by the
   customer, 2026-10-17), the same result as dragging that handle (criterion 2). The
   kind choice and "Whole ellipse" are greyed (`aria-disabled`, still in the
   Tab order) while no selected ellipse has an arc; all four controls are
   greyed for an ellipse whose curve is not 100% (criterion 31) and skip such
   an ellipse in a mixed selection. "Start" and "End" show "Mixed" when the
   selected ellipses differ. A new arc, made by dragging a handle on a whole
   ellipse, has the kind "Arc".
4. Given an arc, then what is drawn and stroked is: Pie, the closed outline
   arc, line to the centre, line to the arc's start; Arc, only the arc, not
   closed; Chord, the arc and the straight chord, closed. Nothing of the rest of
   the ellipse is drawn. Given a fill (`specs/0007-stroke-and-fill-styling/`),
   then a Pie and a Chord are filled inside their outline, and an open Arc is
   filled as if closed by a straight line from its end to its start while only
   the arc itself is stroked (the SVG rule). The dash pattern, and the start
   and end markers of an open Arc (`specs/0018-stroke-markers/`, where
   primitives carry them), begin at the start angle and run in the direction
   of increasing angle, the same as on the converted path (criterion 13).
5. Given an arc whose sweep from start to end is S degrees, then S is between
   0 and 360 and is never negative: dragging a handle past the other handle
   gives the sweep the pointer's position implies (an end handle that crosses
   the start handle turns an almost whole ellipse into a sliver). A drag that
   ends within 0.1° of the other handle makes the ellipse whole.
6. Given a drag of an arc handle, then the old shape stays in black and the
   new one is drawn in blue (`specs/0009-unified-object-editing/` criteria 10 to 13),
   and a readout at the pointer shows "start 30.0° · sweep 120.0°" for the
   dragged handle's angle and the resulting sweep.
7. Given Ctrl held during an arc-handle drag, then the dragged angle snaps to
   the nearest stop of the multiples of 15° and 22.5°
   (`specs/0008-object-transform-refinements/` criteria 33 and 34), measured from
   the ellipse's own right-hand axis (0°), positive and negative. Without
   Ctrl no snapping. Ctrl has no effect on a numeric entry.
8. Given a double-click on an arc handle, then a field with a "°" suffix opens
   next to it, pre-filled with that handle's angle (the readout's value, one
   decimal), with the cancel, invalid and untouched-Enter rules of
   `specs/0008-object-transform-refinements/` criteria 19 to 21. Enter writes the
   angle in one commit; no double-click switches a tool.
9. Given an arc, then its selection box is the tight oriented box around what
   is drawn (a half circle of radius 20 mm has a box of 40 × 20 mm; a pie
   includes its centre), in the ellipse's own rotated frame. Resize handles,
   the centre handle, the live size readouts, typed W and H, the marquee and
   the pivot all use that box (decided by the customer, 2026-10-17).
10. Given a resize of an arc by any transform handle and any modifier, then the
    ellipse's size and position change by the affine map the drag defines in
    the local frame and the angles do not change (parametric angles survive a
    non-uniform scale), so a half circle stays a half circle at any aspect.
    Stroke width follows the "Scale stroke width" switch and nothing else
    changes (`0005` criteria 8 and 26).
11. Given a rotation or a move, then the ellipse's rotation and position change
    as for any primitive and the arc angles are unchanged (they are relative to
    the ellipse's own axis). Skew handles are not shown (primitives,
    `specs/0008-object-transform-refinements/` criterion 50).
12. Given an arc, then hit-testing (click, hover, lasso, Alt-click cycle) uses
    the drawn outline only, including the straight lines of a pie or chord; the
    missing part of the ellipse is not clickable.
13. Given "Object to path" on an arc, then the result is a path of exactly the
    drawn outline: an Arc becomes an open path, a Pie and a Chord closed
    paths. The arc is split into `ceil(S / 90°)` cubic Bézier segments of equal
    sweep (the standard arc-to-cubic construction, deviating from the true
    ellipse by at most 0.1 % of its larger radius); nodes between two arc
    segments are Symmetric, the ends of an Arc and the corners of a Pie or
    Chord are Corner nodes. A Pie adds the centre as one Corner node. A whole
    ellipse converts as today (4 Symmetric nodes, `0003` criterion 19).
14. Given an arc, when the maker saves, closes and reopens the project, then
    start angle, end angle and kind are exactly as saved and the object is
    still an ellipse. A whole ellipse is stored exactly as before (no arc
    fields), so existing files are unchanged.
15. Given a project containing an arc, when an older build opens it, then it is
    refused with the "saved by a newer version" message and never shown as a
    whole ellipse; the `format_version` increases.
16. Given an ellipse created by the Ellipse tool (Shift and Ctrl rules of
    `specs/0011-shape-creation-from-center/` included), then it is a whole ellipse;
    arcs are made only with the arc handles.

### Document-model impact (Part A)

`Shape::Ellipse { frame }` gains an optional arc: `start`, `end` (angles) and
`kind` (three values). Absent means whole. This is a stored-field change and
a `format_version` increase, even though whole ellipses are written as before,
because an older build would otherwise show an arc as a whole ellipse
(criterion 15). Decided in `adrs.md` (2026-10-17): one list register `ellipse_arc`
`[start, end, kind]`, deleted for "whole"; validation on open (finite angles, a
kind outside the three values is damaged, an arc together with a curve is
damaged); the `format_version` is the next free one at merge; the tight box in
`oriented_bounds`; the one shared evaluation every consumer calls; golden
fixtures for all three kinds (`CLAUDE.md` §5).

## Part B: Curve handle, from circle to star

**Status of this part: the reading of the customer's request and the two
Affinity screenshots is confirmed by the customer (question 2, resolved
2026-10-06), including Curve on polygon and star from the start. Requirement,
not proposal. Criteria 17 to 34 define the ellipse; criteria 35 to 52 add
polygon and star.**

What we understood. A circle or ellipse has four **cross nodes** where its
imagined inner cross (the horizontal and vertical axis through the centre)
meets the outline: east, south, west, north. One handle lets the maker change
the curvature of the four sides between these nodes continuously: a circle at
100%, a diamond with straight sides at 0%, a four-pointed star with concave
sides and sharp tips at -100%. This is Affinity's "Curve" option of the Polygon
tool with 4 sides (screenshots: Curve about -100% gives the star with a small
handle inside near one side; Curve 100% gives a circle with a bounding box of
eight handles).

Considered and dropped (customer: none of these): a standing or lying oval
(already possible with the resize handles), an egg (one node pulled along its
axis), a rounder or squarer shape by node tension. The screenshots show one
curvature parameter, not node pulling.

Terms. **Curve** `k` is a value from -400% to 400% (decided 2026-10-19, each shape limited to its range, see the criteria); below, `k` is the same
value as a fraction (-4 to 4). The ellipse has centre `C`, half-axes `rx` and
`ry` in its own frame, and cross nodes E (`rx`, 0), S, W and N. A **side** joins
two neighbouring cross nodes (E to S, S to W, W to N, N to E). The **unit
shape** is the shape for `rx = ry = 1`; every ellipse is its image under the
scale `(x, y) → (rx·x, ry·y)`, so every statement below about the unit shape
holds on an ellipse with per-axis radii.

### Acceptance criteria

**Range, decided by the customer 2026-10-19 (CURVE-FINAL): ±400 %.** The stored curve
is a fraction from -4 to 4. Each shape is held to its own geometric range
`[k_min, k_max]` with `k_min = −A_min/(2·φ)` and `k_max = (360° − A_max)/(2·φ)`, both
within ±400 % (`A_min`, `A_max` the smallest and largest interior angle of the straight
shape, `φ = 180°/M`). At `k_min` the sharpest tip becomes a cusp; at `k_max` a corner's
interior angle reaches 360°; beyond either the outline would cross itself. So: ellipse
-100 % to 300 %; triangle -50 % to 250 %; square -100 % to 300 %; pentagon -150 % to
350 %; polygons from 6 sides -200 % (N = 6) to -400 % and up to 400 %; a star by its tip
and notch angles (5 points, ratio 0.382: about -100 % to 300 %). Above 100 % every corner
becomes an inward notch and the sides bulge past the circle (flowers and suns), every side
always bulging outward. Wherever a criterion below says -100 % or 100 % as a limit, it
reads as this range; the formulas (`d(k)`, the sagitta, the handle's inverse, the cubic
construction) hold unchanged over the whole range.

17. Given an ellipse (or circle), then it has one curve value, from -100% to
    300% (the range above), 100% by default. A circle or ellipse made by the Ellipse tool has
    100% (criterion 34). Curve 100% is the ellipse as it exists today.
18. Given curve `k`, then every side of the unit shape is a circular arc
    through its two cross nodes whose tangent at each end makes an angle of
    `45° · |k|` with the chord between the two nodes, bulging away from the
    centre for `k > 0`, towards the centre for `k < 0`, and straight (no arc)
    for `k = 0`. (The arc then subtends `90° · |k|`.) The three fixed points:
    - `k = 100%`: each side is a quarter of the circle; the shape is the
      circle (the arc at 45° tangent angle is the circle's own arc).
    - `k = 0%`: each side is a straight line; the shape is a square turned by
      45° with its vertices at the cross nodes (for `rx ≠ ry` a rhombus). It is
      a diamond, not a rounded square.
    - `k = -100%`: each side is a quarter of a circle of radius 1 centred at
      the nearest corner of the bounding square (for example, the side E to N
      is the arc of the circle centred at (1, 1) through (1, 0) and (0, 1),
      concave); the shape is a four-pointed star with sharp tips at the cross
      nodes. It is not the mathematical astroid: the curve's middle is 0.293
      of a radius from the centre along the diagonal, the astroid's is 0.354.
19. Given curve `k`, then the middle of the side E to N (and of the other three
    by symmetry) lies on the diagonal of the box at `(rx·d, ry·d)` from the
    centre (towards E and N), with `d(k) = 0.5 · (1 + tan(22.5° · k))`. So
    `d(100%) = 0.7071`, `d(0%) = 0.5`, `d(-100%) = 0.2929`. Examples, circle of
    radius 20 mm: at 100% the side's middle is at 14.14 mm on both axes and the
    area is 1256.6 mm²; at 0% the shape is a diamond with 40 mm diagonals,
    middle at 10 mm, area 800.0 mm²; at -100% the middle is at 5.86 mm and the
    area is 343.4 mm² (a 40 × 40 mm square minus four quarter discs of
    radius 20 mm). Example, ellipse `rx = 30`, `ry = 10` mm at -100%: the
    middle of the side E to N is at (8.79, 2.93) mm.
20. Given any curve value, then the shape is one closed outline that does not
    cross itself and passes through the four cross nodes. Up to 100 % it stays
    inside the ellipse's bounding box, so its selection box is the box of the
    ellipse, `2·rx × 2·ry`, and typed W and H mean `2·rx` and `2·ry`. Above
    100 % the sides bulge past that box: the selection box, typed W and H and a
    resize use the tight box of the drawn outline, as for an arc (criteria 9
    and 10).
21. Given a circle, then the interior angle at each cross node is
    `90° · (1 + k/100%)`: 180° at 100% (smooth), 90° at 0% (corner), 0° at
    -100% (cusp, a needle tip). For an ellipse the angle at -100% is 0° and at
    100% 180° as well; between, it depends on the aspect ratio. Given a stroke
    (`specs/0007-stroke-and-fill-styling/`) and a fill, then the stroke joins
    at the cross nodes follow the object's join style like at any other
    corner, a mitre join falls back to a bevel by that style's mitre limit at
    sharp tips, and the fill is the closed outline. Nothing in this spec adds
    a stroke or offset rule; an offset of the shape is the offset of the path
    "Object to path" would give (criterion 29), and tips below 180° are what
    an offset has to handle for any path.
22. Given one selected ellipse and handles drawn (`specs/0009-unified-object-editing/`
    criteria 6 to 8), then one **curve handle** is shown inside or on the
    outline at the middle of the side E to N, the upper right side of the
    unrotated ellipse (position per criterion 19; at 100% it sits on the
    outline at 45°), together with the transform handles (and, on a whole
    ellipse, the arc handles of Part A). It is drawn with the same parameter-
    handle silhouette as the other parameter handles, and the hit order is that
    of `specs/0009-unified-object-editing/` criterion 5. The curve handle and the arc handles never
    meet: an ellipse with an arc has no curve handle and an ellipse with a
    curve other than 100% has no arc handles (criterion 31), and on a whole
    ellipse at 100% the curve handle (upper right, 315°) and the resting arc
    handles (seam 45°) are 90° apart.
    The curve handle follows the same parameter-handle rules as the arc handles
    (criterion 1: knob glyph, threshold T = 72 px, hit radius 12 px, the 4 px
    clearance, not drawn during other drags of the same object); the centre
    handle yields when it is within 20 px of it (`specs/0009-unified-object-editing/`
    criterion 8).
23. Given the maker drags the curve handle with the pointer at (x, y) in the
    ellipse's own frame relative to its centre, then with `u = x / rx` and
    `v = -y / ry` (screen y down, so `u`, `v` are positive towards the handle's
    corner) the curve becomes `k = atan((u + v) − 1) / 22.5°`, limited to
    the ellipse's range, -100% to 300% (`atan` in degrees; it is the exact inverse of criterion 19:
    the handle stays under the pointer until a limit is reached). Dragging
    towards the centre gives negative values, away from it towards the
    outline's diagonal point gives 100%, further out more. Within 1 percentage
    point of 100%, 0% or the limit the value snaps to exactly that value, with no modifier. With Ctrl
    held the value snaps to the nearest multiple of 25 percentage points
    (Ctrl has no effect on a numeric entry). Escape cancels and writes
    nothing; a press and release under 3 screen pixels of movement writes
    nothing; release writes one commit.
24. Given a double-click on the curve handle, then a field with a "%" suffix,
    accessible name "Curve", opens next to it, pre-filled with the current
    value (one decimal), and Enter writes the value in one commit. A value
    outside the shape's range is limited and the limited value is written; an
    empty or non-numeric value keeps the field open and marked invalid; Escape,
    an unedited Enter, a click elsewhere, a tool switch and a selection change
    close it and write nothing (`specs/0008-object-transform-refinements/`
    criteria 19 to 21 and 30 to 31). No double-click switches a tool.
25. Given a curve-handle drag, then the old shape stays in black and the shape
    the release would commit is drawn in blue, hollow
    (`specs/0009-unified-object-editing/` criteria 10 to 13), and a readout at the
    pointer shows "curve -35.0 %".
26. Given a selection that consists only of ellipses (one or more; polygons and
    stars join the selection in criterion 44), then the Select tool's top bar shows a "Curve" number field (-400 to 400, "%", each shape limited to its range),
    with the rules of `specs/0009-unified-object-editing/` criterion 21: it updates
    every selected ellipse live, commits once per interaction, shows "Mixed"
    when the values differ, and a typed value applies to all. It is greyed
    (the tooltip says why) for an ellipse that has an arc (criterion 31). Typing 100 makes
    the shape an ellipse again; there is no separate reset button.
27. Given a resize by any transform handle and any modifier, or a typed W and
    H, then `rx`, `ry` and position change by the affine map the drag defines
    in the local frame and the curve value does not change (the shape scales
    like any ellipse; a star made at -100% stays a star at any aspect). Stroke
    width follows the "Scale stroke width" switch and nothing else. Given a
    rotation or a move, then rotation and position change and the curve value
    is unchanged. Skew handles are not shown (`specs/0008-object-transform-
    refinements/` criterion 50).
28. Given any curve value, then hit-testing (click, hover, lasso, Alt-click
    cycle) and the marquee use the drawn outline and the selection box of
    criterion 20, by the same rules as for any closed shape.
29. Given "Object to path", then the result is a closed path of 4 nodes in the
    order and direction a whole ellipse's conversion has today
    (`0003` criterion 19), one cubic Bézier per side. In the unit shape the
    side E (1, 0) to N (0, 1) has the control points
    `P1 = E + L·(cos α, sin α)` and `P2 = N + L·(sin α, cos α)` (the mirror
    image of `P1` across the diagonal), with `α = 135° − 45°·k` and `L = (√2 / 3) / cos²(22.5° · |k|)`, and the
    other three sides are the same rotated or mirrored; control points are then
    scaled by `rx`, `ry` and placed in the ellipse's frame. Check values: at
    100% `P1 = (1, 0.5523)`, at -100% `P1 = (0.4477, 0)`, at 0% the control
    points are on the chord and the segment is written as a straight line
    without handles. Each Bézier deviates from its true circular arc by at
    most 0.1% of the larger radius. Node kinds: Symmetric at 100%, Corner at
    every other value (including -100%, where the two handles of a node point
    the same way). A shape at 100% converts exactly as a whole ellipse does today.
30. Given "Object to path" on a selection of several objects, then each is
    converted independently and atomically, as `specs/0009-unified-object-editing/`
    criterion 22 defines; the curve value is not kept (a path has none).
31. Given an ellipse with a curve other than 100%, then no arc handles, no
    "Pie / Arc / Chord" choice and no "Whole ellipse" button are shown for it,
    and no arc can be made; given an ellipse with an arc, then no curve handle
    is shown and the bar's "Curve" field is disabled. To combine the two the
    maker makes one of them whole first (arc: "Whole ellipse"; curve: type
    100). Arc and curve never coexist on one object. (Excluded to keep the
    angle definition of Part A, which assumes a true ellipse; combining them is
    a possible later story.)
32. Given an ellipse with a curve other than 100%, when the maker saves, closes
    and reopens the project, then the curve value is exactly as saved and the
    object is still an ellipse with the same size, rotation and other
    parameters. An ellipse with curve 100% is stored exactly as before (no new
    field), so existing projects are unchanged and not rewritten on open.
33. Given a project containing a curve other than 100%, when an older build
    opens it, then it is refused with the "saved by a newer version" message
    and never shown as a plain ellipse; the `format_version` increases (once
    for Part A and Part B if they ship in one PR). Given a file with a curve
    that is not finite or outside -400% to 400%, or with a curve and an arc on
    one ellipse, then it is refused as damaged with a named error, not clamped.
34. Given an ellipse made by the Ellipse tool (Shift and Ctrl rules of
    `specs/0011-shape-creation-from-center/` included), then its curve is 100%;
    the curve is changed only with the handle or the bar field.

### Curve on polygon and star

The customer asked for polygon and star in the same slice (2026-10-06). The
ellipse is the special case: an ellipse is a 4-sided shape whose first vertex
is at its right-hand cross node E, scaled to `rx` and `ry`; the rule below
reduces to criteria 18, 19, 23 and 29 for it (criterion 51 makes that a test).

Terms. A polygon with N sides has N **vertices** on its circumscribed circle of
radius `R` (its outer radius). A star with N points has 2N vertices, N outer
(radius `R`) and N inner (radius `ρ·R`, `ρ` the ratio), alternating. The
vertices form a closed **ring** of `M` vertices (`M = N` for a polygon, `M = 2N`
for a star, `M = 4` for the ellipse); a **side** joins two neighbouring
vertices; its chord has length `c` (polygon `c = 2·R·sin(180°/N)`; star
`c = R·√(1 + ρ² − 2·ρ·cos(180°/N))`). The **full angle** is `φ = 180°/M`
(polygon `180°/N`, star `90°/N`, ellipse 45°). **Curve** `k` is a fraction from
-4 to 4 as in Part B above. **Vertex 0** is the shape's first vertex: for a
polygon the vertex the creation drag put at B, for a star the first outer
vertex (the one the inner-radius handle's first inner vertex follows, clockwise
on screen), for the ellipse E.

35. Given a polygon or a star, then it has one curve value within its range
    (at most -400% to 400%),
    **0% by default** (straight sides: every polygon and star made so far, and
    every one the Polygon/Star tool makes). The default differs from the
    ellipse's 100% on purpose, so no existing shape changes its look. A shape
    made by the Polygon/Star tool has 0%; the curve is changed only with the
    handle or the bar field.
36. Given curve `k` and a side of the ring, then the side is a circular arc
    through its two vertices whose tangent at each end makes the angle
    `θ = |k|·φ` with the chord, bulging to the outside of the straight shape
    (the side of the chord on which the shape's interior is not) for `k > 0`,
    to the inside for `k < 0`, and straight for `k = 0`. The arc subtends
    `2θ`. All sides of one shape use the same `k`. Fixed points:
    - `k = 0%`: the shape as it is today (straight sides).
    - polygon, `k = 100%`: each side is an arc of the circumscribed circle; the
      shape is that circle, for every N from 3 to 1024. (A 4-sided polygon at
      100% is the circle of criterion 18.) A star at 100% is no circle: its
      outer and inner vertices are on two circles; its 2N arcs bulge outward and
      the shape is a pillow-edged star.
    - `k = -100%` (where the limit of criterion 38 allows it): each side is the
      mirror image of the 100% arc across its chord, concave. A 4-sided polygon
      at -100% is the four-pointed star of criterion 18; a hexagon is a
      six-pointed star with concave sides; a star made by the Star tool gets
      concave sides and keeps its points.
37. Given a polygon with outer radius `R` and N sides, then the middle of a side
    lies on the perpendicular bisector of its chord, at the distance
    `m(k) = R·(cos(180°/N) + sin(180°/N)·tan(k·90°/N))` from the centre. The
    bulge of the middle from its chord is the sagitta
    `s = (c/2)·tan(k·φ/2)`, signed (positive outward), and the same formula
    gives the middle of a star's sides. Examples, `R = 20` mm: hexagon at 100%:
    20.00 mm (on the circle, area 1256.6 mm²), at 0%: 17.32 mm (area
    1039.2 mm²), at -100%: 14.64 mm; triangle at 0%: 10.00 mm, at -50%
    (its limit): 5.36 mm; square (N = 4) at 0%: 14.14 mm, at 100%: 20.00 mm.
38. Given a shape, then the lowest allowed curve is `k_min = max(-4, −A/(2·φ))`
    with `A` the smallest interior angle of the straight shape (polygon:
    `180° − 360°/N`; star: the smaller of its outer and inner interior angle).
    At `k_min` the sharpest tips become cusps (interior angle 0°); below it the
    outline would cross itself. So: a polygon with N sides has
    `k_min = −(N/2 − 1)`, at most -400% (a square -100%, a hexagon -200%);
    a triangle has -50%; a star with N = 3, ρ = 0.5 (hexagram) has -100%; a
    star with N = 5, ρ = 0.382 (pentagram) has -100%; a star with N = 5,
    ρ = 0.2 has about -44.4% (tip angle 15.97°, `φ = 18°`). The value written by
    a drag, a typed entry or the bar field is limited to `[k_min, k_max]` for that
    shape (`k_max` from the range above) and the limited value is written. A
    stored value outside it (the maker later changes Points or the ratio) is
    valid and kept as saved; the **effective** curve is the stored value held
    to `[k_min, k_max]`, the field shows it with the "limited" tag, and everything drawn, shown
    and converted uses the effective curve. Restoring the Points or ratio
    brings the stored value back. For every N from 3 to 1024, every ratio from
    0.01 to 0.99 and every effective curve value the outline is one closed
    curve without self-crossings (touching only at cusps at `k_min` and at the
    notches at `k_max`).
39. Given a polygon or star at curve `k`, then the interior angle at every
    vertex is the straight shape's interior angle plus `2·k·φ`, so polygon
    corners are smooth (180°) at 100% and a star's outer tips widen as `k`
    grows. Strokes and fills follow the rules of criterion 21 (join style at
    every vertex, mitre limit at sharp tips, the fill is the closed outline).
40. Given one selected polygon or star and handles drawn
    (`specs/0009-unified-object-editing/` criteria 6 to 8), then one **curve handle**
    is shown at the middle of one side together with the transform handles (and,
    on a star, its inner-radius handle). The side is the one on the
    counter-clockwise (on screen) side of vertex 0 (for the unrotated ellipse
    that is E to N, criterion 22; for a star it is the side on the other side of
    vertex 0 from the inner-radius handle's vertex). The handle sits at the
    middle of that side at the effective curve (criterion 37: on the straight
    side's middle at 0%, outside it for `k > 0`, inside for `k < 0`); it follows
    rotation. It has the parameter-handle silhouette of criterion 22; hit order
    as in `specs/0009-unified-object-editing/` criterion 5; no two glyphs closer
    than 4 px, with the threshold T of 72 px and the knob glyph of
    `specs/0009-unified-object-editing/` (a star's inner-radius handle and this one
    are two knobs that must also clear each other). A polygon had no parameter
    handle before this; the curve handle is its only one.
41. Given the maker drags the curve handle, then the pointer is projected onto
    the perpendicular bisector of that side's chord (in the shape's own frame,
    unaffected by rotation), `t` is its signed distance from the chord's middle
    (positive outward) and the curve becomes `k = 2·atan(2·t/c) / φ`, limited
    to `[k_min, k_max]` (it is the exact inverse of criterion 37: the handle
    stays under the pointer's projection until a limit is reached; for the
    ellipse it equals criterion 23). Snapping and cancelling are as in
    criterion 23: within 1 percentage point of 100%, 0% or `k_min` the value
    snaps to it with no modifier, Ctrl snaps to multiples of 25 percentage
    points (the limit `k_min` is also a stop), Escape cancels and writes
    nothing, under 3 screen pixels of movement writes nothing, release writes
    one commit.
42. Given a double-click on the curve handle of a polygon or star, then the
    field of criterion 24 opens ("%" suffix, accessible name "Curve",
    pre-filled with the effective value to one decimal); a value outside
    `[k_min, k_max]` is limited to the nearest bound and the limited value is
    written; every other rule is criterion 24's.
43. Given a curve-handle drag on a polygon or star, then the preview and the
    readout are criterion 25's: the old shape stays in black, the shape the
    release would commit is drawn in blue, hollow, and a readout at the pointer
    shows "curve -35.0 %".
44. Given a selection that consists only of ellipses, polygons and stars in any
    mix, then the Select tool's top bar shows the "Curve" field of criterion 26, in a
    group of its own right after the kind groups (the polygon/star group with
    "Points" and "Ratio" comes before it). A typed value or a drag
    of the field is limited per shape to that shape's own range (a triangle in
    the selection stops at -50% and 250% while a hexagon goes to -200% and 400%); the field shows
    the effective value and "Mixed" when the effective values differ; typing
    100 gives an ellipse its ellipse shape back and a polygon its circle. For a
    selection that contains an ellipse with an arc the field is greyed with
    the tooltip of criterion 26 and skips that ellipse. There is no reset button; 0 resets a polygon or
    star to straight sides.
45. Given a resize of a polygon or star (always uniform and about the centre,
    `specs/0009-unified-object-editing/` criterion 16) or a typed r, then the outer
    radius `R` changes (r is `R`, as today) and the curve value, ratio, Points
    and orientation do not change; the curved sides scale with the shape
    (`k` is scale-free). Rotation and move change rotation and position only.
    Stroke width follows the "Scale stroke width" switch. No skew handles.
46. Given a change of Points N or of the ratio while a curve is set, then the
    stored curve value is kept and the shape is re-evaluated with the new
    `φ`, `c` and limit (criterion 38). The curve is a fraction of the
    full angle, so 100% on a polygon is a circle for every N. The curve handle
    and the "Curve" field show the effective value.
47. Given a polygon or star at any curve value, then its selection box is the
    tight oriented box around the drawn outline, by the rule the box has for
    these shapes today applied to the curved outline (it grows when sides bulge
    outward, up to the circle's box for a polygon at 100% and past it above,
    and is never smaller than the straight shape's box when `k > 0`), and the resize
    handles, centre handle, readouts, marquee and pivot use that box. The
    pivot of a rotation and of a resize stays the shape's centre
    (`specs/0009-unified-object-editing/` criterion 16). Hit-testing (click, hover,
    lasso, Alt-click cycle) uses the drawn outline, by the rules for any closed
    shape.
48. Given "Object to path" on a polygon or star, then the result is a closed
    path with `M` nodes (polygon N, star 2N) in the order and direction of
    today's conversion (`0003` criterion 20), one segment per side: at 0% N or
    2N Corner nodes joined by straight segments exactly as today, otherwise one
    cubic Bézier per side from vertex `V` to the next vertex `W`, with control
    points `V + L·d` and `W + L·d'`, where `L = (c/3)/cos²(θ/2)`, `d` is the
    unit vector of the chord `V→W` turned towards the outside by `θ` for
    `k > 0` (towards the inside for `k < 0`), and `d'` its mirror image across
    the chord's perpendicular bisector. (For the ellipse's unit shape this is
    criterion 29's `L` and angles.) A side whose arc subtends more than 90°
    (only a triangle with an effective curve above 75%) is split into two equal
    arcs by one extra Symmetric node at its middle, so a triangle at 100%
    gives 6 nodes. Each Bézier deviates from its true circular arc by at most
    0.1% of `R`. The effective curve is the one converted, and the curve value
    is not kept (criterion 30 applies).
49. Given "Object to path" on a polygon or star with a curve other than 0%,
    then a node is **Symmetric** where the two tangents at it are collinear
    (interior angle 180° within 0.01°; for a polygon exactly at 100%) and
    **Corner** at every other vertex (including every vertex at `k_min`, where
    the two handles of a node point the same way). Nodes at 0% are Corner, as
    today.
50. Given a polygon or star with a curve other than 0%, when the maker saves,
    closes and reopens the project, then the curve value is exactly as saved
    and Points, ratio, outer radius, rotation and every other parameter are
    unchanged. A polygon or star at 0% is stored exactly as before (no new
    field), so existing projects are unchanged and not rewritten on open.
    Given a project that contains a curve on a polygon or star, when an older
    build opens it, then it is refused with the "saved by a newer version"
    message and never shown with straight sides; the `format_version` is the
    one increase shared with the ellipse register of criterion 33 (once for
    Part A and Part B if they ship in one PR). Given a file with a curve that is
    not finite or outside -400% to 400%, then it is refused as damaged with a
    named error, not clamped; a value outside that shape's `[k_min, k_max]` is not damaged
    (criterion 38).
51. Given a 4-sided polygon with vertex 0 at the ellipse's E, or a circle, then
    at every curve value from -100% to 100% (steps of 12.5 percentage points
    included) the polygon's outline of criteria 36 and 37 and the circle's of
    criteria 18 and 19 are the same curve within 0.001 mm for `R = 20` mm, and
    their converted paths of criteria 29 and 48 have the same control points.
52. Given a selection that contains a path, a rectangle or any other object that
    is not an ellipse, polygon or star, then the "Curve" field is not shown and
    no curve handle exists for that object (a rectangle's rounded corners are
    `specs/0013-rectangle-corner-radii/`, not a curve).

### Interaction with the rest of the product (added for Ready)

Criteria 53 to 67 apply to arcs (Part A), to the curve of all three shapes
(Part B) or to both, as each says. They add no new feature; they pin down what
happens where this slice meets the specs that were written or built after it
was drafted.

53. **Rotated ellipse.** Given an ellipse with an arc and a rotation (a circle
    too: its rotation, invisible on a whole circle, now turns the arc), then
    the drawn arc, the two arc handles, the selection box and the hit area are
    the unrotated ones turned about the centre; a handle drag reads the
    pointer in the ellipse's own frame (criterion 2), so the handle stays under
    the pointer at any rotation. Rotating or moving writes only rotation and
    centre (criterion 11).
54. **Mirror and negative scale (`0056`).** Given a flip, a half turn, or a
    resize drag or typed W or H that passes through zero, applied to an ellipse
    with an arc, then it stays an ellipse with an arc whose drawn outline is
    the exact mirror image of the old one: centre, `rx`, `ry` and `rotation`
    are what `0056` criterion 21 gives, the kind and the sweep are unchanged,
    and a reflection reverses the direction of increasing angle, so the new
    start is the image of the old end and the new end the image of the old
    start. Angles are written in [0°, 360°). Example in the ellipse's own
    frame: a mirror along its own horizontal axis gives `start' = −end`,
    `end' = −start`; along its own vertical axis `start' = 180° − end`,
    `end' = 180° − start`; a document-axes flip negates the rotation and uses
    the first formula for a vertical flip and the second for a horizontal one.
    Test: the mirrored arc equals the mirror image of the old one at 64 equally
    spaced samples within 1e-9 mm, for a flip twice (identity within 1e-9° and
    1e-9 mm), and for rotations of 0°, 30° and 90°. A curve is scale-free and
    the ring of every curved shape is symmetric, so a mirror leaves the curve
    value unchanged on an ellipse, a polygon and a star (the drawn outline is
    the mirror image, as `0056` criterion 22 requires for the straight shape).
55. **Linked clone (`0049`, `0057`).** Given a selected ellipse with an arc or
    a curve, or a polygon or star with a curve, when the maker makes a linked
    clone, then the primitive is converted to a path in the same commit, with
    no notice (`0049` criterion 2), and the path is the one "Object to path"
    gives (criteria 13, 29, 48), so the clone shows the arc or the curved
    shape. A clone has no arc or curve field of its own, no handles of this
    spec, and the Select bar's ellipse group and "Curve" field are not shown
    for a clone; `0057`'s overrides change style and placement only.
56. **Stretch, skew and conversion (`0019`, `0047`; customer, 2026-10-17).**
    Given a non-uniform scale (squash or stretch) of an ellipse with an arc,
    including a group stretch in a multi-selection and a stretch of a rotated
    ellipse, then it stays an ellipse with an arc; nothing is converted. The
    image of an ellipse under any affine map is an ellipse, so this is exact
    and needs no new parametrisation: with the ellipse's frame `F = R(rotation)
    * diag(rx, ry)` and the map's linear part `M`, take the singular value
    decomposition `M F = U S V^T`; the new rotation is that of `U`, the new
    `rx`, `ry` are the singular values `S`, the new centre is the image of the
    old one, and a point at parameter `t` goes to parameter `t - beta` where
    `V = R(beta)`. So the start and end angles both move by `-beta` and the
    sweep and the kind are unchanged. If `M` reflects (negative determinant),
    the angle rule of criterion 54 is applied on top. Singular values equal
    within 1e-9 (a circle result) are written with rotation 0 and the same
    shift. Test: stretch a 30° rotated half circle by (2, 0.5) in the document
    axes; the new arc equals the image of the old one at 64 equally spaced
    samples within 1e-9 mm; a stretch by (2, 0.5) then (0.5, 2) restores the
    angles within 1e-9°. Given a **skew** (also in a multi-selection), then
    the ellipse becomes a path of the drawn outline, as criterion 13 and
    `0047` criterion 2 say, the arc control and the Select bar's ellipse group
    no longer apply to it, and the result is not an arc again. A curved
    ellipse (curve other than 100%), a polygon and a star stay as `0047`
    criterion 7 says (a stretch converts them): their ring is not the image of
    the old ring under a general affine map. **Proposal (vetoable):** a whole
    ellipse at curve 100% follows the same rule, so `0047` criterion 7 no
    longer converts a rotated whole ellipse on stretch; the same function
    handles both. A single ellipse resized in its own frame (criteria 10 and
    27) is not converted and is not changed by this criterion. A uniform scale,
    a rotation, a move and a mirror never convert (criterion 63).
57. **Copies.** Given Duplicate, a Ctrl-drag copy or a copy made by a transform
    entry, then the copy has the same arc or curve as the original (the whole
    parameter set is copied) and is the same kind of primitive.
58. **Layers (`0050`).** Given an ellipse, polygon or star that is hidden, on a
    hidden layer, locked or on a locked layer, then it is never in the
    selection, so it gets no handle of this spec and no bar control writes to
    it (`0050`'s selection filter); the bar fields skip it in a selection that
    reaches it through a group. Its arc or curve is drawn, hit-tested (when
    visible and unlocked) and converted like any other.
59. **History (`0020`, `0041`, `0042`).** Given any change of this spec, then it
    is one commit with one undo step and one entry in the History tab named
    "Change arc" (angles, kind, "Whole ellipse") or "Change curve" (handle,
    field, double-click entry). A bar field drag or typed value on a selection
    of several objects is one commit and one step for all of them. The object
    history of `0041` lists the same names. A branch, a Preview and a clone of
    an earlier state (`0042`) draw the arc or curve by the same evaluation;
    going to a state from before the arc reads a whole ellipse without any
    notice.
60. **Other consumers of the outline.** Given an ellipse with an arc, a curved
    ellipse, or a curved polygon or star as an operand of a Boolean operation
    (`0016`), Offset (`0038`), Combine (`0035`, `0054`), Cut at crossings
    (`0036`) or Fracture and Flatten (`0037`), then the operand is its drawn
    outline, converted as a primitive is today; an open Arc is handled as any
    open path is by that feature. This spec adds no rule to any of them.
61. **Determinism.** Given the same shape parameters, then the shared evaluation
    returns the same outline, box, handle positions and converted path every
    time and in any order of calls: a pure function, no clock, no random
    number, no map iteration, no global. Native and `wasm32-unknown-unknown`
    agree within 1e-9 mm on the golden fixtures (the trigonometric functions
    may differ in the last bits; the tests compare with that tolerance, and the
    stored values are compared exactly after a save and open).
62. **Budgets** (native release build, `#[ignore]` benchmarks on the build
    host, as `0019` criteria 48 and 49). (a) One evaluation of an ellipse with
    an arc, a curved ellipse, or a curved hexagon, including the box and the
    converted path: 50 µs at most. (b) A curved polygon with 1024 sides: 2 ms
    at most. (c) One frame of a handle drag (apply, outline, preview, readout)
    on a 1024-sided curved polygon: 4 ms at most, so the drag keeps 60 frames
    per second with the renderer's share. (d) A typed "Curve" over a selection
    of 5,000 mixed ellipses, polygons and stars in one commit: 250 ms at most.
    (e) Opening a file with 5,000 curved shapes adds at most 100 ms to the open
    time of the same file without curves.
63. **Primitives stay primitives.** Given any operation of this spec or of a
    transform whose result is exact, then the object stays the primitive it
    is: a curve of 100% on a polygon stays a polygon (it draws a circle) and a
    curve of 0% on an ellipse stays an ellipse (it draws a diamond); a pie of
    360° is a whole ellipse (criterion 5), not a path; a mirror, half turn,
    uniform scale, rotation, move and typed size never convert. Only "Object to
    path", a linked clone (criterion 55) and a group stretch (criterion 56)
    convert.
64. **Angle input.** Given a typed or dragged angle, then it is written in
    [0°, 360°): −90 becomes 270, 360 becomes 0, 450 becomes 90. An input that
    makes start and end equal within 0.1° makes the ellipse whole
    (criterion 5), one commit.
65. **Keyboard and screen reader.** Given the canvas handles are pointer
    controls, then every value they set is also reachable by keyboard in the
    bar: the kind choice, "Whole ellipse", "Start", "End" and "Curve" have a
    Tab stop, an accessible name and a tooltip; a release or a bar commit
    announces the new value in a polite live region ("Arc start 30.0°, end
    150.0°, pie"; "Curve −35.0%"). No new shortcut key is added (Proposals
    below).
66. **No hint text.** Given the handles and bar controls of this spec, then
    there is no hint chip, tip line or explanatory label by default (customer
    decision 2026-10-16); the pointer readout (criteria 6, 25) and the
    tooltips of the bar controls are the only text. A greyed control keeps its
    tooltip, which names the reason ("Not available while the curve is not
    100%").
67. **Rendering and export tolerance.** Given the renderer and every exporter
    (now: none; later `0024`, machine output), then an arc and a curved side
    are flattened with the document's kernel tolerance of 0.01 mm at most and
    never from the cubic of "Object to path" alone: the true circular arc is
    flattened, so the drawn outline and the converted path differ by at most
    the 0.1% bound of criteria 13, 29 and 48.

### Document-model impact (Part B)

`Shape::Ellipse { frame }` gains an optional `curve` register (a unit-carrying
newtype, never a bare `f64`; absent means 100%), next to the optional arc of
Part A. The polygon and the star gain the same kind of optional `curve`
register (the same newtype; absent means 0% there, criterion 35), so one
stored field and one `format_version` increase cover all three shapes, which
the ellipse register and the polygon/star register share (criterion 50). The
stored value is raw and clamped on evaluation to `[k_min, 100%]` for polygon
and star (criterion 38), like the rectangle's radius; validation on open only
rejects non-finite values and values outside -400% to 400% (decided 2026-10-19). A shared ring
evaluation (vertices, `φ`, `c`, side arc, middle, cubic control points,
`k_min`) is called by the ellipse and by polygon and star, so criterion 51
holds by construction; it lives in `curvyo-document-core` (`adrs.md`), with the property
test of criterion 38 (no self-crossing for N from 3 to 1024, ratio from 0.01 to
0.99, any allowed curve value). The half-axes `rx`,
`ry` are unchanged, so nothing about the frame or `oriented_bounds` changes for
Part B (criterion 20). Decided in `adrs.md`: one flat `curve` register per shape
(a fraction from -4 to 4, decided 2026-10-19), deleted when the value returns to the shape's
default; validation on open (finite, within the range, not together with an
arc); a live merge that yields both an arc and a curve is not refused (the arc
wins and the curve is ignored until the arc is made whole), a file with both is
damaged; the one evaluation function every consumer (outline, render, hit
test, handle layout, "Object to path") calls, so the drawn outline and the
converted path agree; the true-arc-to-cubic construction of criterion 29;
golden fixtures for
curve 100%, 50%, 0%, -50% and -100% on a circle and one ellipse, and for the
polygon and star on a triangle (limit -50%), a hexagon and a pentagram star at
100%, 50%, 0%, -50% and the limit, plus the triangle at 100% (6 nodes)
(`CLAUDE.md` §5).

A four-pointed star with straight sides still comes from the Star tool
(4 points and a ratio); the curve is the way to concave or convex sides on
any of the three shapes.

## Out of scope

- "Smooth points" (Affinity: smoothing of the corners above 0%), a separate
  curve per side or per node, a different curve for a star's outer and inner
  sides, a true astroid or any curve family other than circular sides, curve
  and arc on the same object (criterion 31), a Curve on a rectangle, path or
  text, a curve field in the Polygon/Star tool's own bar (it only creates;
  the Select tool edits).
- A donut or ring (inner radius, Affinity's "Donut"), Inkscape's "pointer
  inside or outside decides the kind", moving both arc handles together with
  one drag, arcs on rectangles, stars or polygons (a polygon's curve is not an
  arc), elliptical arcs tilted against the shape's own rotation, rotating the
  arc inside the ellipse by a modifier (two drags or typing do it).
- Arc or curve in the Ellipse tool or in the click-to-create chip of `0052`: a
  new shape is whole (criteria 16, 34); `0052`'s chip stays r and angle.
- SVG: `0024` imports `<ellipse>` and arc commands as paths, as it imports
  every shape; an exported arc or curve is its path outline, and a round trip
  gives a path, not an arc. Native SVG arcs as primitives are a later story.
- A keyboard route to the handles on the canvas (the bar fields are the
  keyboard route, criterion 65), new shortcut keys (Proposals below), an
  animation of the handles.
- Skewing primitives (`specs/0009-unified-object-editing/` question 5),
  keyboard nudging beyond `0044`. Undo and redo are in scope as one step per
  change (criterion 59), not as a feature of their own.
- Stroke offsetting and boolean operations on the shape (their own slices; the
  shape is an ordinary closed path through "Object to path", criterion 60).

## Decided by the product owner

Change any of these if you disagree; none of them blocks the build.

- Priority Should; one slice: Parts A and B on the branch `story/ellipse-arcs-and-shaping`
  as milestones (Part A, Part B ellipse, Part B polygon and star), **one pull
  request** when the whole slice can be tried (customer rule 2026-10-09), one
  `format_version` increase.
- Angles run clockwise on screen from the ellipse's own right-hand axis; equal
  angles within 0.1° are a whole ellipse; angles are stored in [0°, 360°)
  (criteria 5 and 64).
- On a whole ellipse the arc handles rest at the seam of 45° so they never sit
  on a resize handle and the first drag is continuous (criteria 1, 2).
- The ellipse group of the Select bar is always complete while an ellipse is
  selected: the controls are greyed where they do not apply and never appear
  or vanish (customer rules on the bar and on permitting over refusing), with
  "Start" and "End" fields as the keyboard route (criteria 3, 65).
- Polygon and star default to curve 0%, ellipse to 100% (criteria 17, 35); the
  stored curve is kept when Points or Ratio change (criterion 46).
- Budgets of criterion 62 are my estimates from the other specs' benchmarks;
  the implementer may propose other numbers with a measurement.
- No hint chip, tip line or Apply button anywhere in this slice (criteria 66,
  8, 24).
- Licence and dependencies: no new dependency, nothing to check with
  `cargo deny`; the circular-arc-to-cubic construction is written here from the
  standard formulas and is our own code.

## Decided (customer, 2026-10-17)

The customer answered the three open questions. The wording of the questions is
kept short below so the reasons stay readable.

- **ARC-KIND = A.** A new arc, made by dragging an arc handle on a whole
  circle, is an open arc; a wedge is one click on "Pie". (Rejected: B pie as
  the first kind, C open arc only.)
- **ARC-BOX = A.** The selection box of an arc is the box around the drawn
  part only (a half circle of radius 20 mm is 40 x 20 mm); the pivot of a
  rotation moves to the middle of that part. (Rejected: B the whole circle.)
- **ARC-VS-CURVE: the customer's reply addressed skew and scaling, not the
  Curve control.** The reply (translated): "I assume you mean skew: on skew we
  convert to a path, after that an arc of course no longer works. If you mean
  squashed or stretched by scaling: that should simply work; it should be
  solvable through the transformation." Result, see criterion 56:
  - A non-uniform scale (squash or stretch), also after a rotation, keeps an
    ellipse with an arc an ellipse with an arc; the arc follows the transform.
  - A skew converts the ellipse to a path (as 0047 already does for every
    primitive); the arc control does not apply to a path.
  - The Curve-versus-Arc interplay stays at the default **A**: the controls of
    the other are greyed with a tooltip (confirmed by the customer on
    2026-10-06; criterion 31). The customer's reply did not ask for B, so B
    (permit and set the other to whole) is not built.

Question 2 (CURVE-READING) was resolved on 2026-10-06 (criteria 17 to 52).

## Proposals (vetoable at the demo, no question)

- No keyboard shortcut for arc or curve; nothing is bound by this slice. A key
  for "Whole ellipse" is a later Proposal if the maker asks.
- The tooltip of the curve handle's bar field names the three stops
  ("100% circle, 0% diamond, -100% star"); it is the only place that says so.

## UX notes

Written to `docs/design-system.md` (rows "Parameter handle", "Parameter handle
layout", "Select bar layout", "Bar number field", "Limited tag") and the
customer rules of 2026-10-06 to 2026-10-16; the implementer adds the new rows
named below to the design system in the same PR. No new colour token, glyph
shape or font size: arc and curve add a knob position and a readout, as the
design system says for a new parameter kind.

**Arc handle layout.**
- Two knobs, the standard 10 px parameter handle, both on the outline at the
  start and end angle, in the ellipse's own rotated frame, drawn above the
  transform handles; shown from a shorter box side of 72 px (`T`).
- Whole ellipse: both rest at the seam of 45° (lower right on screen), the end
  knob 7 px before the seam and the start knob 7 px after it along the outline
  (14 px apart, the 4 px clearance rule). Dragging either takes the pointer's
  angle at once; the other stays at 45° (criterion 2).
- Arc: the knobs sit on the true angles. A pie or chord shows the same two
  knobs; the straight edges have no handle.
- The "Knob displacement" rule below keeps a knob off a resize
  square and off the other knob.
- Hover: the knob takes the hover ground; the cursor is the built-in `pointer`;
  no hint chip. During a drag the readout "start 30.0° · sweep 120.0°"
  (criterion 6) sits at the pointer by the existing readout rule; the old shape
  stays black, the new arc is blue and hollow, over any fill with the white
  casing of the "Live preview outline" row.

**Knob displacement (one rule for the arc, curve and inner-radius knobs).** A
knob keeps its true position unless a drawn glyph (resize square, another knob,
the centre handle where it does not yield) is closer than 14 px to its centre.
It is then drawn displaced along its own travel line (the outline for an arc
knob, the perpendicular bisector of the side towards the centre for a curve
knob) by the shortfall, at most 14 px, away from the other glyph; if both ways
are blocked, towards the shape's centre. The value, the hit area and the
readout follow the drawn position; the value is only recomputed from the
pointer once the drag starts, so no jump beyond the 14 px. Two knobs closer
than 14 px (a sliver arc) are pushed apart symmetrically about their middle.

**Curve handle.** One knob at the middle of the side counter-clockwise of
vertex 0 (criteria 22 and 40), at the effective curve; on an ellipse at 100%
it sits on the outline at 315° (upper right). A star's curve knob and its
inner-radius knob lie on opposite sides of vertex 0 and are separated by the
displacement rule. At small curve values or on a small shape the curve knob can
be near the centre handle; the centre handle yields within 20 px
(`0009` criterion 8).

**Select bar.** Groups follow the existing order: settings, Flip, kind groups,
then the curve group, then "Object to path" and "Linked clone".
- Ellipse group (shown while the selection holds an ellipse), two 36 px rows
  because the group is wider than the 324 px bar that an 800 px window with the
  panel open leaves: row 1 the "Arc kind" segmented group (three 28 x 28 px
  icon buttons Pie, Arc, Chord; 16 px glyph, 1.5 px stroke, `currentColor`;
  `role="radiogroup"` named "Arc kind", `aria-pressed` on the active one by
  fill and glyph, not by colour alone), 12 px gap, then the "Whole ellipse"
  icon button (a full circle outline, 28 x 28 px). Row 2 the number fields
  "Start" and "End", each 104 px, 4 px apart, the label inside the left edge
  (12 px `--toolbar-icon` at 70%), the "°" suffix inside the right edge, one
  decimal. Accessible names "Arc start" and "Arc end". The row heights of the
  bar are measured at the build, as the Flip and rectangle groups were, and
  recorded in the design-system row.
- Tooltips (Radix, 400 ms, bottom, name semibold, one muted line): "Pie" /
  "Arc" / "Chord" with "The arc and straight lines to the centre." / "The arc
  only. Open." / "The arc closed by a straight line."; "Whole ellipse" with
  "Remove the arc."; "Start" and "End" with "Angle from the right-hand axis,
  clockwise." A greyed control adds a second muted line with the reason.
  Greyed = `aria-disabled`, glyph at 40%, no hover, Tab stop and tooltip kept
  (the Flip rule), never hidden.
- Curve group (shown while the selection holds an ellipse, polygon or star):
  one field "Curve", 96 px, the label "Curve" inside the left edge, "%" suffix,
  right-aligned one-decimal value up to ±400 (decided 2026-10-19), "Mixed" when the effective values differ,
  the "limited" tag and its tooltip of the Limited tag row when a polygon's
  or star's stored value is below its limit ("Stored -80 %, limited to -44.4 %
  by Points and Ratio; changing them brings it back."). Greyed with a tooltip
  for an ellipse with an arc. Tooltip line: "100 % circle, 0 % diamond, -100 %
  star." (Proposal 2.)
- Fields: Enter commits once; Escape or a press elsewhere restores and writes
  nothing; invalid input is marked by the 2 px `--field-invalid` border and the
  12 px message line "Enter a number" under the field (the Radius field's
  rule).
- Entry chips (double-click on a knob): the existing entry chip surface with
  one field ("Arc start", "Arc end", "Curve"), the "°" or "%" suffix, no
  Apply button, Enter commits, Escape closes (criteria 8, 24, 42).

**Build notes (2026-10-19, after the architect, tester and UX reviews).**
- Kind buttons: a `role="group"` named "Arc kind" of toggle buttons with `aria-pressed`,
  not a `radiogroup` (which needs `role="radio"` children); replaces the `radiogroup`
  wording above.
- Knob displacement: a knob is moved by the shortest distance that clears, at most two
  pitches (28 px), not at most 14 px: one pitch cannot clear a knob lying exactly on a
  corner glyph. Observed about 15 px; the 28 px bound did not occur. An arc's end knobs
  move only into the drawn arc. The resting knobs on a whole ellipse are exactly 14 px
  apart (chord), symmetric in angle about 45°.
- The 72 px threshold of an arc's knobs is measured on its whole ellipse (`2·rx`,
  `2·ry`), not on the box of the drawn part.
- Bar layout: the kind buttons and the fields "Start", "End", "Curve" are two groups, so
  the kinds sit beside Flip. Start and End 93 px, Curve 99 px. Measured: 4 rows (144 px)
  at the 324 px bar of 800 x 600 (a rectangle takes 172 px there), 2 rows (72 px) at
  1280 x 800. The Curve tooltip line of Proposal 2 is not shown until the customer
  decides.

**Not present, by rule.** No hint chip or tip line on the canvas or in the bar
(customer 2026-10-16); no section in the right panel (the panel gets nothing
new; an arc or curve is not a style); no new toolbar button in the rail; no
tool switch; no Apply or Reset button (typing 100 or 0, or "Whole ellipse",
resets); nothing appears that the selection did not already imply (the
ellipse group is shown for any ellipse, whole or not).

**Accessibility and contrast.** The knob and bar controls reuse the tokens of
the existing parameter handle and Flip buttons, which meet 3:1 (glyph) and 4.5:1
(text) already; the tooltips and the "limited" tag are 12 px text at
`--toolbar-icon` on the bar ground as in the Limited tag row. Focus order in
the bar: kind group, Whole ellipse, Start, End, Curve.

**Open for the build review** (measured, not decided here): the real row
heights of the bar with an ellipse, a polygon and a mixed selection at
800 x 600; the 7 px offset of the resting knobs against hover target overlap;
the look of the old shape under a partly drawn blue arc on a filled shape.

## Links
Requirements: R-EDIT-002 (`docs/requirements.md`)
Builds on: `specs/0009-unified-object-editing/` (must come first),
`specs/0003-primitive-shapes/` (criteria 7 to 9, 10 to 15, 19, 20),
`specs/0008-object-transform-refinements/` (criteria 19 to 21, 33 and 34)
Meets: `specs/0013-rectangle-corner-radii/` (the register and bar pattern),
`specs/0049-linked-clone/` and `specs/0057-clone-overrides/` (criterion 55),
`specs/0050-layers-and-objects-panel/` (58), `specs/0056-mirror-and-negative-scale/` (54),
`specs/0042-history-branches/` (59), `specs/0052-click-to-create-shapes/`, `specs/0053-node-marquee-selection/`
and `specs/0055-compound-path-node-editing/` (nodes of the converted path),
`specs/0058-bitmap-import/` (no interaction)
Decisions: `adrs.md`
PR:
