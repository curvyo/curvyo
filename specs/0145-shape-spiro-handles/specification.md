# Shape loops: a Rounded handle on polygon, star, rectangle and ellipse that can be dragged past its normal range into looped, flower-like forms

Status: Ready for slice 1 (2026-11-23; slice 1 = polygon and star only). Slice 2 (rectangle, ellipse) and slice 3 (Randomized) stay Draft: slice 2 waits for the customer's trial of slice 1 and, for the rectangle part, for Frage 326; the ellipse part is unaffected by it. Fragen 324, 325, 326 and 308 to 311 open, every one has a default. `adrs.md` and `ux-notes.md` exist (2026-11-23). The golden Inkscape fixtures of criterion 4 are hand-computed from `adrs.md` decision 1 until the customer exports them from Inkscape 1.4 (`adrs.md` decision 7, step 7); criterion 4 is covered provisionally until then.
Priority: Should
Origin: Customer (2026-11-20, translated, with a screenshot of Inkscape's star/polygon tool): the polygon, star, rectangle (square) and circle/ellipse need the missing radius and rounding handles, and the maker can drag them beyond the normal range so the shape "jumps" into the Inkscape-style spiro/loop forms. The screenshot shows a wavy 8-point star, a ring of loops or petals, a rounded 6-point star with inward cusps, a large circle with small loops at each corner, a rosette of overlapping ellipses and a plain rectangle. This is also the polygon/star rounding handle the customer asked for on 2026-10-20 (never specified until now). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker I want to pull one handle on a star, polygon, rectangle or circle and see its corners go from sharp to soft, and past that into loops and petals, and keep the result as an editable shape, so that rosettes, flowers, wavy stars and soft badges take a drag, not a node-by-node build.

**What other tools do.** Inkscape's Star/Polygon tool has **Rounded** (tangent length of each corner, 0 sharp, positive soft, negative and large values loop; field range -10 to 10) and **Randomized** (each vertex moved by a seeded random amount); both are bar fields only, with no canvas handle, and the tool must be active. Affinity Designer's polygon has Curve and Smooth points (we have Curve, `0021`). LightBurn: none. We do better: a handle on the shape in the Select tool (as every parameter handle of `0009`), typed values in the bar, one undo step, and the result stays a live shape until the maker converts it.

## What is built and what is missing (reading of the code and specs on 2026-11-20, `main` at `format_version` 53)

| Shape | Built | Missing |
|---|---|---|
| Rectangle (`0013`) | Four corner-radius handles, four stored radii, radii clamped by the CSS rule `f` (criterion 9), "Link corners" | any over-range: a radius past the clamp is limited, there is no loop form |
| Ellipse (`0021`) | Two arc handles, one **Curve** handle (circular-arc sides, -400 % to 400 %, limited at `k_min` so the outline never crosses itself, criterion 38) | the Inkscape-style Rounded form and loops |
| Polygon (`0003`, `0021`, `0012`) | Points on the bar, resize and rotate handles, one **Curve** handle | Rounded handle and field |
| Star (`0003`, `0021`) | Points and Ratio, **inner-radius handle**, **Curve** handle | Rounded handle and field, Randomized |
| SVG import (`0024` criterion 118) | A polygon or star with `inkscape:rounded` or `inkscape:randomized` comes in as an ordinary path | a live primitive (Frage 310) |

`0021`'s Curve is a circular arc through two vertices and is deliberately limited so it never loops; Inkscape's Rounded is a Bezier tangent factor at the vertices and does loop. They are **two different parameters**. Decision (Proposal): this is a **new spec 0145**, not a revision: it adds a parameter and a format field; the limits of `0013` and `0021` stay as accepted behaviour (Frage 324 A). `0003` "Out of scope" listed star rounding and randomization; this spec lifts that for Rounded (Randomized: Frage 308). `0115` (Spiro paths of the Pen tool) is unrelated: the word "spiro" in the request means these looped shapes.

## Terms

- **Ring:** the vertices of a primitive in order: polygon N vertices, star 2N (outer and inner alternating), ellipse 4 (E, N, W, S in its own frame), rectangle 4 (its corners). Straight sides join them.
- **Rounded** `q`: a number, 0 = the shape as today. At every vertex the outline is a **smooth node** whose two handles lie along the tangent direction and have the length `|q|` times a base length derived from the vertex's neighbours (Inkscape's definition); **negative `q` flips the tangent direction**; large `|q|` makes the handles longer than the sides and the outline loops at each vertex. The **exact formula is pinned by the architect against Inkscape's own output** (Inkscape 1.4, the fixtures of criterion 4), because the customer's reference is what Inkscape draws; where this spec and Inkscape differ, Inkscape wins.
- **Normal range:** `q` from 0 to 1, the range the handle's guide marks (`q = 1` is the ring's centre). It is a marker, not a guarantee: some shapes start to loop before 1 (a hexagon near 1.2, a 5-point star at Ratio 0.3 near 0.95) and every `q < 0` loops (`adrs.md` finding A). **Over-range:** any other value, up to the stored limit of criterion 2.
- **Loop:** the outline crosses itself (a flattened segment pair intersects away from shared endpoints).

One-line meaning (customer rule: put it to him before it is built): **Rounded:** "Pulls a soft curve out of every corner of the shape; past the middle of the shape the curves cross themselves into loops and petals."

## Acceptance criteria (as far as clear)

Slice 1: polygon and star (the screenshot's star forms). Slice 2: rectangle and ellipse (Frage 325; Ready only after the customer has tried slice 1, because the formula on a 4-vertex ring decides how those forms look).

1. Given one selected polygon or star with handles drawn (`0009` criteria 6 to 8), then one **Rounded handle** is shown with the other handles, on the ray from the centre through vertex 0, 15 px inside the vertex at `q = 0` (the corner-radius handle placement of `0013` criterion 1), draggable at 0; at rest and on hover it never lies within 4 px of the resize, rotate, Curve or inner-radius glyphs (`ux-notes.md` section 2 displaces it along its ray when it would); the dragged handle may pass over other glyphs during the drag. The threshold T of `0009` (72 px) is measured on the box of the shape at `q = 0`, so a drag into loops never makes handles appear or vanish. Below T the bar field is the way (criterion 3).
2. Given the maker drags the Rounded handle, then `q` changes **relative to its value at the press** by the pointer's displacement projected on the ray (inward positive, outward negative) divided by the unit `U = D - 15` px, where `D` is the on-screen distance from vertex 0 to the centre, so `q = 1` is exactly at the centre and the map is the same at every zoom; it is linear and **continues through the centre and past the opposite side without a stop**; the stored limit is -10 to 10 (Inkscape's field range, **Proposal**). A marker on the handle's guide shows where the normal range ends (`q = 1`, the centre; loops can start earlier, see Terms). Snaps with no modifier within 0.015 or 3 screen px, whichever is larger, of 0 and of 1; Ctrl snaps to steps of 0.25; Escape cancels and writes nothing; under 3 screen pixels of movement writes nothing; release writes one commit, one undo step "Change rounding".
3. Given a polygon or star selected, then the bar shows a **Rounded** number field (-10 to 10, two decimals, accessible name "Rounded"); a double-click on the handle opens the same entry; a value outside the range is limited to the nearest bound and the limited value is written. A selection of several shapes shows "Mixed" when they differ and writes all in one commit.
4. Given `q`, then the outline of a polygon or star is: `q = 0` exactly today's straight shape (every existing file renders as before, within 0.001 mm); otherwise the closed chain of cubic Beziers of the pinned Inkscape formula through every vertex. Tests: golden SVGs exported by Inkscape 1.4 for a 5-point star (Ratio 0.5) and a hexagon at `rounded` -1, -0.3, 0.3, 1, 3, compared to 0.001 mm per node and handle (`tests/fixtures/`, CLAUDE.md §5); the outline is the same after rotating the shape by 360/N degrees; every vertex of the ring lies on the outline within 0.001 mm.
5. Given Rounded and Curve (`0021`) on one shape, then the two are **mutually exclusive** (Frage 309 A): setting one to a non-zero value sets the other to 0 in the **same** commit; the Curve handle is still shown and is not greyed; one Ctrl+Z restores both.
6. Given `q` over-range (`q < 0` or `q > 1`), then the shape is **still a live polygon or star**: Points, Ratio, size, angle and the other handles keep working, the object is not turned into a path; the loops are only part of its outline (the customer rule "a primitive stays a primitive wherever the result is exact", `0021`). Test: a star at `q = 2.5` has its Points changed from 5 to 8 and `q` stays 2.5.
7. Given `q` not 0, then the **selection box and the marquee** use the exact bounds of the Bezier outline (extrema, not control points), because loops leave the circumscribed circle: the box contains 4096 sampled outline points and touches the extreme ones within 0.001 mm **wherever the outline leaves today's box** (the box is the union of today's box and the outline's exact box, `adrs.md` decision 5); at `q = 0` the box is exactly today's (accepted behaviour, finding C).
8. Given a **filled** looped shape, then the fill is nonzero like every fill of the product (`0035`): overlapping loops of one winding fill solid (Frage 311 A); a click inside any loop selects the shape; given an unfilled looped shape, then it is hit within max(half the stroke width, 3 screen px) of the outline, and a press on the handle wins over the body (`0009` criterion 5).
9. Given Convert to paths (Object to path, `0003` criterion 17), then the result is one path with M smooth nodes (polygon N, star 2N) and the cubic handles of criterion 4; the loops are kept as drawn (no boolean cleanup); `q = 0` converts to straight segments exactly as `0003` criterion 20 says. The maker cleans loops with Union (`0016`) afterwards.
10. Given a looped shape on a **laser or plotter page** (`0101`), then it is cut or engraved as drawn: no refusal and no warning chip (permit over refuse, memory "Permit over refuse"); the job preview shows the crossings, and a crossing is cut twice, as with any crossing path. Flattening for the job uses the job tolerance of `0029`; the test is that the job's polyline stays within that tolerance of the Bezier outline for `q` = -10, -1, 2.5, 10 on a 1024-point polygon.
11. Given save and reopen, then `q` is exactly as saved; the register is optional, so a file without it is a `q = 0` shape; an older build opens a file the new build saved with the "saved by a newer version" refusal (`OpenError::FormatTooNew`), never with wrong corners; `format_version` increases once, **next free at merge**, the number is the architect's (CLAUDE.md §4: a spec does not hard-code a version); a stored value that is not finite is damaged, outside -10 to 10 is limited to the bound on use (the "limited" tag of `0021`, nothing written back).
12. Given a drag, then the outline is recomputed within 2 ms of thread CPU time per pointer event for a star with 1024 points (2048 vertices) at `q` = 10 and with 5 points at `q` = -10 (`curvyo-editor-wasm/tests/cpu_time/`); the flatten step for drawing is part of the 2 ms.
13. Given N from 3 to 1024, Ratio from 0.01 to 0.99 and `q` from -10 to 10 (a property test over 1000 seeded combinations), then the outline is finite (no NaN), closed and the same at the same input on native and `wasm32-unknown-unknown` within 0.001 mm.

Slice 2: rectangle and ellipse (Frage 325)

14. Given (rectangle part: only if Frage 326 is answered A or C; Frage 326 default B means Rounded on the ellipse only) one selected rectangle or ellipse with handles drawn, then it gets the same **Rounded handle** and bar field (criteria 1 to 3) on its ring of 4 vertices: for a rectangle on the ray from its top-left corner to the centre with its zero point 29 px from the corner (the corner-radius handle rests at 15 px, `ux-notes.md` section 9), for an ellipse on the ray through its right-hand vertex E. At `q = 0` the shape is today's rectangle (its four radii as stored) or ellipse; a non-zero `q` makes the corner radii (rectangle) or the Curve (ellipse) mutually exclusive with it by criterion 5, rectangle radii being kept in the file while `q` is not 0 so setting `q` back to 0 and one Ctrl+Z restore them.
15. Given an ellipse, then the value `q_c` at which the 4-vertex ring is closest to the circle (the pinned formula determines it) draws a true ellipse within 0.05 % of `rx` and `ry`, the handle snaps to it within 2 percent (`q_c`, like the 100 % snap of Curve), **the ellipse is looked at by the customer before slice 2 is built.** On the rectangle's corner ring `q_c` is the ellipse through the four corners, not a soft square (`adrs.md` decision 9, finding B); what the rectangle gets is Frage 326.
16. Given a rectangle or ellipse with a Rounded value, then criteria 6 to 13 hold for it (live primitive, bounds, fill, Convert to paths, laser, file, budget); each of slice 2 bumps `format_version` again (`adrs.md` decision 4).
19. Given an ellipse that has an arc (Start and End not a whole ellipse), then **Rounded is not available**: the bar field is greyed with "Not available while the ellipse has an arc" and the handle is not drawn, as for Curve (the arc wins, `0021`; `adrs.md` decision 9).
20. **(Proposal)** Given a rectangle with `q` not 0, then "Remove rounding" (`0013`) also sets `q` to 0 (it is enabled while `q` is not 0), in the same commit; a radius drag sets `q` to 0 likewise (criterion 14). Vetoable at the demo; it touches an accepted `0013` control.

Randomized and import (Frage 308, 310)

17. Given **Randomized** (Frage 308 A; slice 3), then polygon and star have a field Randomized (-10 to 10 like Inkscape) and a **New seed** button; each vertex moves by a pseudo-random offset up to `|r|` times a base length from a stored 32-bit **seed** (our own generator, `adrs.md` decision 8), so the same file draws the same shape on every platform; the shape stays a live primitive with its Points and Ratio (`r = 0`: no change), Convert to paths gives the moved vertices; `format_version` gets the seed and the value in the same bump as criterion 11 only if built together. Inkscape's generator cannot be reproduced from a stored seed, so a randomized shape **exports as a path** with the shadow, never as `inkscape:randomized` not 0, and an Inkscape star with `inkscape:randomized` not 0 **imports as a path** (criterion 18).
18. Given an SVG with `inkscape:rounded` not 0 on a polygon or star (Frage 310 A), then it imports as a **live star or polygon with that `q`** if the exported `d` equals the pinned formula's outline within 0.001 mm at the stored `sodipodi:r1`, `r2`, `arg1`, `arg2`, `sodipodi:sides`; otherwise (the numbers differ, or `inkscape:randomized` is not 0, in every slice) it stays an ordinary path as `0024` criterion 118 says today. Export of a polygon or star with Curve 0 and Randomized 0 writes `inkscape:rounded` and the polygon/star attributes, so Inkscape reads a Curvyo star with `q` back; the old shadow forms without `q` still read as `q = 0` (`adrs.md` decision 6).

## Out of scope

- Changing the limits of `0013` (rectangle clamp) and `0021` (`k_min`): they stay (Frage 324 A).
- A warning for self-intersecting outlines, automatic loop removal (Union is the maker's step), an even-odd fill rule (Frage 311).
- Inkscape arcs, spirals and elliptic rects as primitives (backlog "SVG Inkscape shapes", after the MVP).
- Rounded on a path, on a Spiro path (`0110`, `0115`), per-vertex Rounded values (one value for the whole ring).
- Animating or stepping `q` over several copies (a Repeat of `0066` can do that).

## Questions for the customer

Defaults apply if unanswered; nothing blocks slice 1.

**Frage 324 (LOOP-HOME): Where do the loops live?**
- A: In a new parameter Rounded; the Curve of `0021` and the rectangle clamp of `0013` stay as they are.
- B: Lift the limits of those two (Curve below `k_min`, radius past the clamp) so they themselves loop.
Recommendation: A (Curve is exact circular arcs, Inkscape's loops are Bezier tangent loops; B would change accepted behaviour and still not give the Inkscape forms). Default: A.

**Frage 325 (LOOP-SHAPES): Which shapes in which slice?**
- A: Polygon and star first (slice 1), rectangle and ellipse second (slice 2) on the same parameter.
- B: All four in one PR.
- C: Polygon and star only.
Recommendation: A (the formula on a 4-vertex ring decides what the soft square looks like; see it on the star first). Default: A.

**Frage 326 (LOOP-RECT): What does Rounded do on a rectangle?** (`adrs.md` finding B; the rectangle part of slice 2 waits for this, the ellipse part does not)
- A: Rounded on the rectangle's four corners as the formula gives it: every `q > 0` bulges the sides out of the box (a pillow), and the value closest to a circle is the ellipse through the four corners, not a soft square.
- B: Rounded only on the ellipse (and polygon, star); the rectangle keeps its corner radii of `0013`.
- C: A squircle on the rectangle as its own parameter (its own spec).
Recommendation: B (A gives no soft square, which is what the screenshot's rectangle suggested; C is a different feature). Default: B.

**Frage 308 (LOOP-RANDOM): Randomized (seeded random vertices)?**
- A: Yes, as slice 3 with a stored seed.
- B: Not at all.
Recommendation: A, it makes the wavy stars of the screenshot and costs one field and one seed in the file. Default: A (slice 3).

**Frage 309 (LOOP-CURVE): Rounded and Curve on one shape?**
- A: Mutually exclusive: setting one resets the other in the same undo step.
- B: Both combine (Rounded on the curved ring).
- C: Replace the Curve on polygon and star by Rounded.
Recommendation: A (B is hard to read and test, C removes accepted behaviour you asked for on 2026-10-06). Default: A.

**Frage 310 (LOOP-IMPORT): Inkscape stars with Rounded import as live shapes?**
- A: Yes, when the outline equals ours within 0.001 mm, otherwise as paths.
- B: Always as paths as today (`0024` criterion 118).
Recommendation: A (a file with your Inkscape stars keeps its handle). Default: A.

**Frage 311 (LOOP-FILL): Filled loops: nonzero only?**
- A: Nonzero only (overlapping loops fill solid; Inkscape's default even-odd shows holes, so some Inkscape rosettes look filled here).
- B: A per-object fill-rule setting (Nonzero, Even-odd), which `0035` ruled out so far.
Recommendation: A for this spec; B is its own story if you want holes. Default: A.

## UX notes

See `ux-notes.md`: Rounded knob on the vertex-0 ray with `q = 1` at the centre, guide ticks at 0 and 1, relative drag and snaps, the Drag-ghost preview, the Rounded field after Curve, slice 2 placement, and the criterion changes it needs (section 11).

## Links

Requirements: R-EDIT-057 (new)
Depends on: `0003`, `0009`, `0013`, `0021` (all done)
Related: `0012`, `0024` (criterion 118 changes with Frage 310 A), `0016`, `0029`, `0035`, `0101`
PR: none yet
