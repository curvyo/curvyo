# Centre lines: filled line art becomes variable-width paths

Status: In progress: demo 2026-11-18 (Done (PR #366) once accepted)
Priority: Should
Origin: Customer, 2026-11-15 ("Center finding with variable path width"; the example was a line-art whale: black strokes of varying thickness, closed outlines, an eye with a filled pupil, mouth lines, branching). Items marked **(Proposal)** are the product owner's and vetoable at the demo.
Depends on: `0110-pen-modes-variable-stroke` and ADR 0053 (width profile), `0085-stroke-to-path` (the inverse operation, the check for criterion 5), `0025-raster-trace` and `0108-trace-library-extraction` (where such shapes come from), `0016-boolean-operations` (kernel tolerance 0.01 mm), `0020-undo-redo`, `0049-linked-clone`.

## User value

As a maker who traces or imports line art and stitches it on an embroidery machine, I want each black stroke turned into one path along its centre whose width follows the line, so that a satin stitch can run along it tight against the filled areas, instead of getting the outline of an area, which is right for a laser and wrong for a stitch (R-VEC-006, R-MFG-EMB-004).

## Field reference

Ink/Stitch's "Auto-Satin" and Inkscape's Centerline Trace (potrace `-c`) find a centre line of a bitmap. Both give uniform or hand-fixed width, no branch handling beyond splitting at junctions, and no work on existing vector shapes. Curvyo works on vector shapes (traced, imported or drawn), keeps the measured width as an editable profile (ADR 0053), and joins branches with a chosen overlap.

## Terms

- **Subject:** a selected object (at any depth in a selected group) that has a fill, is a closed path, compound path or primitive, and is visible and unlocked. A compound path's holes are the holes of the shape (the white regions of the whale).
- **Medial axis (skeleton):** the set of points inside the shape with two or more nearest boundary points; **local width** is twice the distance to the boundary.
- **Main path:** the single path through a shape's skeleton where direction and width change least (customer: CENTRE-JUNCTION A). **Branch:** every other part, an own path.
- **Overlap `p`:** at a junction a branch starts at distance `(0.5 - p) x w` from the centre line of the path it joins, `w` being that path's width there. `p = 0` touches the edge, `p = +20 %` (default) starts 20 % of `w` inside the edge, a negative `p` leaves a gap.
- **Tolerance `T(w)`** = max(0.05 mm, 3 % of `w`).
- **Thick parts (mm):** the largest local width a part of a shape may have and still be centre-lined; **(Proposal, CENTRE-THICK-SCOPE B, implementer 2026-11-18)** only the part wider than it stays filled, with the subject's own nodes, and the rest of the shape becomes lines (as specified first, A: a shape with any wider part stayed filled as a whole; a Single-colour trace is one compound path, so one fat spot kept the whole dolphin filled; ux-notes Frage 1). It replaces the blob ratio in the UI (customer 2026-11-18); the ratio (largest local width over the longest distance across the shape) may stay internal if the architect needs it.

## Acceptance criteria

### Entry and result

1. Given a selection with at least one subject, then **Find centre lines** is in the context menu and in the **Node tool's** bar, not in the Select bar (customer 2026-11-18; the place and the icon are set in `ux-notes.md`); otherwise it is dimmed.
2. Given a press, then every subject is replaced in its place in the stacking order by its results, in **one undo step**; undo and redo restore the exact previous document and the selection (the results are selected after the operation).
3. Given a result, then it is an ordinary path: Fill off, stroke colour = the subject's fill colour, Round cap and Round join (main paths and rings; branches have Butt caps on both ends, see criteria 8 and 10), base stroke width = the largest local width, a width profile of ADR 0053 holding the measured width as multipliers; it stays editable with the Node and Edit width tools (customer: CENTRE-MEANING A). A shape that gives more than one result is replaced by one group named "Centre lines of <subject name>", or "Centre lines" for an unnamed subject (coordinator 2026-11-16); a single result is not grouped **(Proposal)**.
4. Given a selection with no subject (open paths, stroke-only shapes, texts, images, guides), then nothing changes, no history entry is made and a message names the first reason ("Select filled shapes", "Convert text to path first"). Given a mixed selection, then the subjects are converted, the rest is left alone, and the notice counts converted, kept-as-filled (criterion 12) and skipped objects.

### Geometry

5. Given a stroke of constant width 2 mm along a known curve (straight, arc, S curve) with round ends, then the centre path lies within `T(w)` of the true axis, its width multiplied by the profile is within `T(w)` of 2 mm at 20 sample points, and the result painted (as by `0085`) differs from the subject by at most 4 % of the subject's area (CENTRE-DEFAULT A, 2026-11-17; it was 2 %).
6. Given a stroke tapering linearly from 3 mm to 0.5 mm, then at 20 sample points along it the profile width is within `T(w)` of the truth; a smooth taper needs no more profile points than are needed for that tolerance.
7. Given a closed ring (an annulus, the outline of a round eye), then the result is one **closed** path with a periodic profile and no seam step larger than `T(w)`.
8. Given an end of the shape, then the path ends there as measured from the shape: a rounded end (half circle of radius `w/2`) of a main path ends `w/2` before the tip with the Round cap; a free rounded end of a branch has a Butt cap and is drawn by a width taper to zero instead: the axis runs on by `r = w/2` to the tip and the width follows `2 sqrt(r^2 - s^2)` over that last `w/2`, so it paints the same round end and ends at the tip with width 0 (customer: CENTRE-CAPS A). The taper has four support points, smoothly interpolated, and its half-width stays within 0.08 r of the true semicircle (CENTRE-TIP A, 2026-11-17; 0.077 r measured); a tapering end reaches the tip with a width of 0. A flat end counts as rounded **(Proposal)**. Open strokes keep both of their ends.
9. Given a T shape (bar 40 x 2 mm, stem 20 x 2 mm), then the main path is the bar and the stem is a branch. Given a Y shape whose stem continues straight into a 2 mm arm and splits off a second arm at 60 degrees and 1 mm, then the main path takes the straight 2 mm arm. Equal candidates are broken the same way on every run (longer first, then the lower start position).
10. Given a junction and `p`, then the branch is its own path that starts at the distance defined under Terms, with a Butt cap on both ends (a path has one cap for both ends) and its own profile, so its width at the start equals the local width of the shape within `T(w)`. With `p = +20 %`, `0` and `-20 %` the start is measured at those distances within 0.01 mm. A loop with junctions gives branches attached at both ends. Branches of branches attach the same way. A spur shorter than its local width at the attachment is no branch; it only shapes the end of its parent **(Proposal)**.
11. Given a shape with holes (the whale's white regions), then the skeleton is that of the shape with its holes taken out; a hole is never filled in and no path crosses a hole; a stroke that closes a loop around a hole gives a closed path (criterion 7) or branches that meet at the junctions (criterion 10).
12. Given a shape with a part wider than **Thick parts** (slider in mm, default **(Proposal)** 6 mm, ux-notes Frage 2; range up to the page's scale set in `ux-notes.md`), then **(Proposal, CENTRE-THICK-SCOPE B)** that part is left as it is, a filled shape (the eye's pupil), and the rest of the shape becomes lines; it is counted in the notice (customer: CENTRE-BLOBS A, reworded 2026-11-18). The blob ratio is not shown. At the maximum, nothing stays filled except what no centre line can describe (a disc). Kept-filled shapes keep their nodes as they are (customer 2026-11-18, point 1).

### Inner areas

13. Given **Keep inner areas** off (default), then the holes are discarded. Given it on, then each hole of a subject also becomes a separate filled shape of the hole's outline moved inward by the offset (default 0.3 mm; a negative value moves it outward, under the stroke) within 0.01 mm, with Stroke off and a fill colour per Open question CENTRE-INNERFILL; a hole smaller than the offset gives none (customer: CENTRE-INNER A).
14. Given a selected shape inside a hole of another selected shape (the pupil inside the eye outline), then the inner area of the hole has that shape cut out, grown by the same offset.

### Preview, undo, input, limits

15. Given the options are open, then they are a bar in the top bar of the Node tool (the app has no popups): **Overlap, Thick parts, Optimise** and the **Offset** of Keep inner areas are the app's usual **sliders** with a value field (as Opacity), plus the **Keep inner areas** switch, an **i** button, Cancel and Apply; Enter applies, Escape cancels; only Apply writes the document. Options are shown with their defaults and remembered for the session (customer 2026-11-18, points 4 and 7).
15b. **Live preview** (customer 2026-11-18, point 5; reverses the interim of question 219 A): while a slider moves, the canvas shows the result of the current values in place of the subjects, computed in a worker so the UI stays responsive (a stale run is dropped when a newer value arrives); Cancel, Escape or deselecting restores the display and the document is unchanged; Apply commits the preview's result in one undo step (criterion 2) without recomputing on the main thread.
15c. Given the **i** button (as in Tracing), then the info page opens on the right and describes each value in plain words with what it does to the result; it lists no ratio.
15a. Given **Optimise** (slider 0 to 100 %, default 75 %; where nodes lie densely together it simplifies more aggressively: nodes closer than about half the local width merge, customer 2026-11-18, point 2; customer 2026-11-16 and CENTRE-DEFAULT A of 2026-11-17), then a higher setting never gives more nodes plus width points; at the default the result meets criteria 5 to 7; corners keep their detail at every setting: an anchor at every corner (a turn of more than 45 degrees within one local width), and within two local widths of a corner the curve keeps half of `T(w)`; at 100 % the centre line and the width stay within three and a half times `T(w)` of the truth on smooth stretches (ADR 0063 amendment).
16. Given a linked clone, then it is converted as the path it shows, and `0085` criterion 17 applies (the result is ordinary, clones of a converted original become ordinary paths); the notice says so.
17. Given the result, then no `format_version` change is needed: paths with width profiles exist since `0110`, and the result opens in the same build before and after saving. SVG export draws the visible result (`0085` criterion 11 test: outlines match within 0.5 % of pixels at 20 px per mm); SVG export does not claim profile round-trip (ADR 0053).
18. Given a traced image of 5,000 nodes over 20 shapes (the whale fixture), then Apply takes at most 1.0 s of thread CPU time and a value change in the preview at most 0.3 s; given 100,000 nodes, Apply takes at most 15 s and never freezes the UI thread for more than 100 ms between frames (work runs in the worker, results are committed in slices); above 100,000 nodes the operation is refused with the node count. The interim limit of 20,000 nodes (question 219 A) stays only if the worker is not enough, and then needs the customer's word. Budgets are **Proposals** for the architect to confirm; tests in `curvyo-editor-wasm/tests/cpu_time/` measure thread CPU time, never wall time.
19. Given the golden fixtures in `tests/fixtures/centre_lines/` (a whale-like drawing: constant and tapering strokes, a ring, an eye with a pupil, a T, a Y, a loop with a branch, a compound shape with two holes, a mouth line with round ends, and the customer's **dolphin** drawing of 2026-11-18 with its thick wave bands, which replaced the demo's orca image: the orca's licence does not allow it in the repository, so it is measured locally and never committed), then the expected centre nodes, width-profile points, branch starts and inner areas are stored as text and compared within 0.01 mm; the same fixtures run through criteria 2, 5 to 14 and 17.

## Customer decisions (2026-11-15, all A)

- CENTRE-MEANING A: an operation on the selection (context menu and tool bar), replaces the shapes in one undo step, results are paths with a width profile, still editable. A Trace checkbox comes later.
- CENTRE-BLOBS A: shapes too fat for a stroke (width-to-length ratio above an adjustable threshold) stay filled.
- CENTRE-JUNCTION A: main path where direction and width change least; branches are own paths starting at the main path; overlap as a percent of the local width, negative is a gap, default +20 %.
- CENTRE-INNER A: switch "Keep inner areas" (off), offset in mm (0.3, negative pushes under the stroke), inner areas as separate filled shapes.
- CENTRE-CAPS A: branches get Butt caps on both ends; a free round end of a branch is drawn by a width taper to zero (over the last `w/2`, ADR 0063). A path has one cap for both ends, and per-end caps would need a format bump, which criterion 17 rules out. The customer notes this may change: a later story may add separate caps per end (option C) with a `format_version` bump.
- CENTRE-SCOPE A: geometry only (paths with profile, branches, inner areas, preview, export); satin stitches come with the embroidery family.

20. Given the dolphin drawing with thick wave bands, then each thick band becomes a few smooth paths (not a mesh of short pieces) with a width profile, and each path has at most `2 + ceil(0.12 L)` nodes and `6 + 4 p + ceil(0.12 L)` width points, `L` its length in mm and `p` its pointed ends (0 to 2, ends whose width falls to 0: criterion 8 fixes four taper support points per such end) **(Proposal**, ADR 0063 amendment 2026-11-18, B, measured on the fixture; it replaces the first draft's 12 nodes plus 12 width points per 100 mm**)**; the painted result differs from the band by at most 4 % of its area. The fixture is stored in `tests/fixtures/centre_lines/` and checked in the golden test of criterion 19.

## Customer decisions (2026-11-17)

- CENTRE-DEFAULT A: criterion 5's painted-area deviation is relaxed to 4 % and Optimise defaults to 75 %, with an addition: corners are reduced less (an anchor at every real corner, the tighter tolerance within two local widths of it), so the looser fit applies to smooth stretches.
- CENTRE-TIP A: a branch's round free end has four support points instead of eight, smoothly interpolated; criterion 8 states its tolerance (0.08 r of the semicircle).

## Customer decisions (2026-11-18, after the whale and the orca)

1. Kept-filled blobs keep their many nodes; nothing to do (criterion 12).
2. Optimise is more aggressive where nodes lie densely together (criterion 15a).
3. Centre-lining thick waves worked badly and gave many nodes: the orca is a new acceptance case (criteria 19, 20).
4. The option inputs are the app's usual sliders (criterion 15).
5. A live preview is required to tune values; it reverses the interim of question 219 A, criteria 15b and 18 apply in this PR.
6. The operation moves into the Node tool, out of the Select bar (criterion 1).
7. An **i** button opens the info page on the right (criterion 15c); the blob ratio is replaced by **Thick parts** in mm (**Proposal**, criterion 12).

## Open questions (all have a default, none blocks the build)

- CENTRE-INNERFILL: fill colour of an inner area. A: a neutral light grey, the maker recolours. B: the subject's own colour. Recommendation and default: A, because B hides the stroke on the canvas.
- CENTRE-GROUP: results of one shape in a group (criterion 3) or as siblings. A: group when more than one. B: always siblings. Default: A.
- CENTRE-BLOBDEFAULT: Thick parts default **(Proposal)** 6 mm (criterion 12; ux-notes Frage 2: 3 mm kept every wave band filled and contradicted criterion 20). Default: 6 mm, adjust at the demo with the whale and the dolphin.
- CENTRE-THICK-SCOPE: only the too-wide part stays filled (B, built, **Proposal**) or the whole shape (A, as first specified). Default: B, decide at the demo (ux-notes Frage 1).

## Out of scope

Satin stitches, stitch order and direction, underlay, embroidery export (embroidery family). The Trace checkbox (a later story). Centre lines of open paths or strokes (already centre lines). V-carving and other uses of the skeleton (R-MFG-CNC-002). Non-uniform width in the shape's profile editing beyond what `0110` offers. Recognising text or shapes in the result (`0088`).

## Links

Requirements: R-VEC-006, R-MFG-EMB-004 (`docs/requirements.md`) · [ADR 0053](../../docs/adr/0053-editable-curves-and-width-profiles.md) · [0085](../0085-stroke-to-path/specification.md) · [0110](../0110-pen-modes-variable-stroke/specification.md) · [0025](../0025-raster-trace/specification.md) · [ADR 0063](../../docs/adr/0063-centre-lines.md) · PR #366
