# Generator joint options: bend through the tabs, and your own number, size and pattern of tabs and fingers

Status: Slice 1 Done (PR #496); Slice 2 Done (PR #504); Slice 3 Done (PR #505); slice 4 Ready (Fragen 381 to 385 answered 2026-11-26; Frage 384 C)
Priority: Should
Origin: Customer (2026-11-25, after the visual check of PR #489, translated): "I only checked it visually, I still have to laser it. I suspect we will have trouble bending the material at the curves, because at the tabs no slits are cut. That should probably be possible as an option, otherwise it becomes a polygon. I'd also like to set the number and height of the tabs/fingers (as advanced). On some boxes it looks nicer with just one in the middle, or two in the middle closer together and fewer at the edges. It brings more calm, is easier to assemble and is often enough." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker I want the slit-bent walls of my rounded and path boxes to bend smoothly all the way round, also where a tab sits, and I want to choose how many tabs or fingers a joint has, how big they are and how they are spread, so that a laser-cut box really bends to a round shape instead of a polygon, and so that a calm box with one centred tab, or a tight pair in the middle, is quick to assemble.

**What other tools do.** Boxes.py (FlexBox family and the finger-joint boxes): `finger`/`space` settings give the finger and gap size, the number follows from the edge length, no centred pattern; the FlexBox tabs sit where the wall is flat. MakerCase: finger size by a choice (min/max), no pattern. Inkscape "Tabbed Box Maker": tab width and a "dimple" option, even spread only. LightBurn: no box generator. We do better: one option keeps the bend round over a tab, and the layout of a joint (count, size, pattern) is a few numbers on the live object, checked against the material so a layout that cannot hold is clamped with a notice.

## Relation to neighbouring specs

- **`0147` rounded-box** (slices 1 and 2 Done): the strip carries tab slots; on a bent span a tab stands in a flat window with no slit column (ADR 0082 §5, Frage 360 B). Part A changes that window, by option. **`0149` path-box** (Done) shares the slit pattern and the windows on curved spans (ADR 0083 §13). **`0141` box** (Done): finger joints; Part B only. The three generators keep one shared joint split (ADR 0082 §5: finger, tab and slot are runs of one split, `finger_count`, `tabs(span, count, width)`), so Part B is one change for all three.
- **`0065` / ADR 0080:** all new fields are parameters of the generated object (the Generator block); a file written before this story opens as stored.

## Slices

`adrs.md` (architect, 2026-11-25) cuts the story by what the customer can laser first. Each slice is one PR the customer can try.

- **Slice 1 (A0): Tighter tab windows.** Criteria 1 to 9. No field, no key. The unslit flat across a tab on a bent span shrinks from about pitch + 2t + 4t_w to the window length l. Frage 385 (JOINT-WINDOW) answered A on 2026-11-26. **Ready.**
- **Slice 2 (A1): Bend through tabs.** Criteria 10 to 17. Option Choice `bend_tabs` (`window` | `curve`): the true arc keeps its slits through the tab. Frage 381 answered A. **Ready.**
- **Slice 3 (B, tabs): Joint layout for tabs** (Rounded box, Path box, curved-top back tabs). Criteria 18 to 24. Fragen 382 A, 383 A, 384 C answered. **Done (PR #505).** Deviations (Proposals): the curved top shows no Advanced rows yet (slice 4); Joint gap and spread show with Centred, not "Count 2 or more" (no number condition in `ShownWhen`, no contract change).
- **Slice 4 (B, fingers): Joint layout for finger joints** (Box, curved-top plates). Criteria 25 to 29. Same Fragen. **Ready.**
- **Later (not slices of this story):** a layout per edge (Frage 383 B, C), free manual positions, different tab sizes inside one edge, patterns on path-box corner joints, joints for the gear or other generators.

## Acceptance criteria

Terms: t = the strip's thickness, t_w = the tab's plate thickness, w = the slit width, l = the window length = 2t + 2t_w (12 mm at 3 mm material; today's window chord, ADR 0082 §5). The **flat across a tab** is the unslit stretch along the strip between the facing edges of the two nearest slits on either side of a tab window on a bent span.

### Slice 1: Tighter tab windows (A0, Ready)

1. Given a Rounded box or Path box where a tab stands in a window on a bent span (60 x 60 mm footprint, radius 30, 3 mm material and the shapes of criterion 6), then the flat across every such tab is at most **l + 0.1 mm** (12.1 mm at 3 mm material). Before this slice it is about 25 mm (pitch + 2t + 4t_w) in the circle golden. The test measures it on the generated strip, for each tab window, for the Straight, Offset and Wave patterns.
2. Given a band of slit columns beside a window, then the band starts at the window's end (no further bridge beyond the chord, which already holds t_w each side of the tab) and the columns are laid **from the band ends**: the first and last column sit w/2 inside the band end, the columns between keep an even pitch of at least the radius's gap g = max(0.22 r, slit width + 1 mm) (N = max(2, floor(L / g)) columns, so the pitch L / N lies between g and 2 g; ADR 0082 §6, ADR 0083 §13; lead, revision 2), and every curved stretch keeps the minimum of two columns. In the circle golden (60 x 60, r 30) the count rises from 12 to between 18 and 24 columns; the implementer pins the exact figure in the golden.
3. Given any box built with this slice, then no band across a bend is left uncut: the test of `0147` criterion 4 ("every line of constant height across the strip on each bent span meets at least one slit or edge notch") holds, and no two contours are closer than the kerf.
4. **Strip length and mating are unchanged in rule:** the strip length equals the footprint perimeter on the neutral line with the window shortening of `0147` criterion 3, within 0.1 mm (as today); tab centres and slot centres agree, the engaged edges lie on one line within the kerf value plus the sag of the window chord (as `0147` criterion 5, five part thicknesses, the circle included). Only the slit columns move; the plates and the tab slots do not.
5. **Nothing else changes.** The default Rounded box (120 x 80 mm, radius 15), the curved top (`0147` slice 2) and every box whose tabs all stand on straights have no window on a bend and produce **byte-identical** output to before this slice (their goldens stay as they are). The Box (`0141`) is untouched. No field, no parameter key, no `format_version` change (ADR 0080 §5); a box saved earlier keeps its stored outline until its parameters are regenerated.
6. **Goldens re-blessed on purpose** (the guard of ADR 0083 §13.4 changes here): `tests/fixtures/rounded_box_circle.txt`, `path_box_circle.txt`, `path_box_heart.txt`, `path_box_shield.txt`, `path_box_ellipse.txt`. The PR description lists them, and the review of each golden shows only slit columns moved on spans with a window; plate outlines and slot positions are equal to the old golden.
7. Given a corner radius so small that a span holds fewer than two columns after the band is cut, then the box is built with the status notice of `0147` (the field and the limit), never an error dialog.
8. Given any parameter change, then the paths regenerate within 100 ms of thread CPU time for a box of 400 slots (as `0147` criterion 6).
9. Given the demo, then the customer cuts a circle box (60 x 60, 3 mm) and a heart and sees the wall bend round at the tabs with flats of about 12 mm instead of about 25 mm (the "demo check", on the customer's laser).

### Slice 2: Bend through tabs (A1, Ready; Frage 381 A)

10. Given a Rounded box or a Path box with a bent span, then a Choice **Bend through tabs** (`bend_tabs`: `window` | `curve`) exists in the Slits group next to Wall slits. A box saved before this slice has no key and **opens as stored, with `window`**; an unknown id reads `window`, as an unknown slit kind does (ADR 0080 §5). For a **new** box the value is `curve` (Frage 381 A, answered 2026-11-26; ADR 0089 `Param.initial`). The customer said old files do not matter (pre-release); the ADR keeps them as stored because it costs one optional field and keeps every existing golden valid. **(Proposal)** a later cleanup may let a missing key read `curve` too. The Finger-joint Box has no such field.
11. Given `curve`, then on every bent span the slit columns run on through the tab: **no flat window**. The plate keeps its true arc (within 0.01 mm), the tab is a **rectangle on the arc's normal at its centre** (root on the arc, tip the concentric arc r + t_w; Proposal of the architect), and the strip's tab slot sits between slit columns. A column near the slot is clipped, not dropped: it stops one bridge t_w from the slot's rows.
12. Given `curve`, then the test of criterion 3 also holds across every slot (the slot holds a rigid tab and is not counted as a bending cut), every clipped column keeps t_w from its slot, and no two contours are closer than the kerf.
13. Given the exact circle (square footprint at the maximum radius) with `curve`, then the strip length equals the footprint perimeter on the neutral line, `2(w - 2r) + 2(d - 2r) + pi (2r + t)`, within 0.1 mm, without the window shortening.
14. Given `curve`, then every plate mates with the strip: tab and slot centres agree by neutral-line arc length within c/2 (c the kerf), with no sag term; the island gap under the tab ends is at most (t + t_w)^2 / 2r (0.6 mm at r 30), inside the box, never an interference. Run at the circle and five thicknesses.
15. Given a curved-top box or a path-box curve, then the option applies wherever a tab meets a slit-bent span; where none does, `curve` output equals `window` output. With `window`, output equals slice 1 byte for byte (a golden pins one box).
16. Given a radius so small that a slot cannot sit between two columns with the minimum bridge, then the box is built with a status notice (fewer or narrower tabs on that span, then the window as a last resort), never an error dialog.
17. Given any parameter change, then regeneration stays within 100 ms of thread CPU time for 400 slots with `curve`; the demo check is a test cut (MDF and plywood) that the softer joint holds.

### Slice 3: Joint layout for tabs (Part B, Done (PR #505); Fragen 382 A, 383 A, 384 C)

18. Given a Rounded box or a Path box, then the Generator block has a group **Advanced**, **collapsed by default** (group state is not stored), holding the joint fields. A box with all Advanced fields at default equals the output before this slice, byte for byte (golden). The new keys are `joint_pattern`, `joint_count`, `joint_width`, `joint_gap`, `joint_spread`; no contract change; no `format_version` bump (**Proposal**, ADR 0080 §5; the architect confirms).
19. **Joint pattern** (choice): **Even** (default; today's layout) and **Centred** (Frage 384 C: Centred plus a Spread slider). Centred groups the tabs about the middle of the edge, the gaps widening toward the ends. Count 1 is the single centred tab; Count 2 the closer pair (gap = **Joint gap**, default 2 times the joint size, **Proposal**); Count 3 or more: each next gap outward adds the Joint gap. **Joint spread** (`joint_spread`, number 0 to 100 %, default 0 %, shown only for Centred with Count 2 or more; **Proposal**) moves every tab from its Centred place toward its Even place: 0 % is the Centred layout as defined above, 100 % equals Even, in between each centre is the linear blend (so no overlap and the margins hold, both ends being valid). At Count 1 it has no effect. Joint gap applies at 0 % and fades with the blend; a gap too large for the edge is lowered with a notice (criterion 23). An unknown id reads Even with a notice.
20. **Joint count** (default Auto, stored 0, muted used value as `0147` criterion 2) from 1 to the most that fit; on a path box a typed count is per side (a straight; the whole loop when there is none). **Joint size** (mm, default Auto = 2 times the thickness) is the tab width along the edge; its depth stays the thickness (Frage 382 A). **Joint gap** and **Joint spread** show only for Centred with Count 2 or more.
21. Given a layout, then tab and slot centres agree, no tab overlaps its neighbour, the engaged edges lie on one line within the kerf; at Count 1 the tab is within 0.01 mm of the edge's middle. The corner rule is unchanged (ADR 0082 §1): margins at both ends are at least the mating thickness, a joint never lies in the first or last piece of a straight.
22. Given the seam straight of a path box, then its count is even so the seam falls between two tabs; "one centred" there becomes two tabs mirrored about the seam (**Proposal**). Path-box corner joints (angled fingers, ADR 0083 §5) stay Even; patterns apply to straights only; bends keep the even rule (and, with slice 2, the bend rule).
23. Given values that cannot hold (size above the edge minus 2 times the thickness, count above what fits, a gap that pushes a joint into a corner piece, a spread outside 0 to 100 %, size below 1.5 times the kerf plus 1 mm), then they are clamped with a status notice naming the field and the limit, never an error. A typed count of at least two tabs per plate suppresses bend windows (default of the architect's question JOINT-COUNT).
24. Given a change, then the object updates within 100 ms of thread CPU time for 100 joints per edge, one undo step undoes it, Object to path equals the live object, and a stored file with the new keys opens intact (older Curvyo: as stored per ADR 0080 §5).

### Slice 4: Joint layout for finger joints (Part B, Ready)

25. Given a Box, then the same Advanced group holds the same fields; **Joint size** is the existing Finger width, shown there and not twice (`finger_width` keeps its key). Rounded box plates with finger joints (curved top) read the same fields. Defaults equal the output before this slice, byte for byte.
26. Given a pattern and count, then finger and slot centres agree, fingers do not overlap, the owner's pieces lie between, and at Count 1 the finger is centred within 0.01 mm of the edge's middle (the mating test of `0141` criterion 3), for every edge and every pattern.
27. Given Centred, then the rest of the edge outside the fingers is a plain butt or the corner owner's solid piece; one owner per corner; a test builds one box per pattern and asserts no contour closer than the kerf and every part closed.
28. Clamping with notice, live update within 100 ms, undo, Object to path and file round trip as criteria 23 and 24.
29. Given a box of 120 x 80 x 50 mm in 3 mm material with Joint pattern Centred, Count 1 on every edge, then lid, base and walls are cut-ready with exactly one tab or finger per edge, all parts closed (the demo check on the customer's laser).

## Out of scope

- A layout per edge or edge group (Frage 383 B, C): slices 3 and 4 are per box.
- Free manual positions, different joint sizes inside one edge, a joint depth other than the thickness (Frage 382 B).
- Dogbone, T-slot, hinge or snap joints; screws or glue marks.
- Changing the corner rule, the slit patterns or the wall thickness rules of `0141`, `0147`, `0149` (slice 1 moves slit columns only beside a tab window).
- Bend through tabs for a straight wall; patterns on path-box corner joints.

## Open questions for the customer

- **Frage 385 (JOINT-WINDOW), answered A on 2026-11-26:** the slit band starts at the window's end and the columns are laid from the band ends, so the flat across a tab shrinks from about pitch + 2t + 4t_w to the window length l. No key, no contract change. Circle, heart, shield and ellipse goldens are re-blessed on purpose; the default rounded rectangle and the curved top do not change.
- **Frage 381 (JOINT-BEND), slice 2, answered A on 2026-11-26 (old files do not matter, pre-release):** is Bend through tabs (`curve`) the default? A: `curve` for new boxes, boxes saved before open as `window`; the option stays. B: `window` by default, `curve` only when chosen. C: no option, always `curve`. Recommendation A. Default if unanswered: A (decided after a slice 1 test cut, if the flats of about 12 mm already bend well enough, B is reasonable).
- **Frage 382 (JOINT-SIZE), slices 3 and 4:** "height" of tabs and fingers. A: the size along the edge, depth stays the thickness. B: also a free depth. Recommendation A (B breaks the fit and the flush outside). **Answered A on 2026-11-26.**
- **Frage 383 (JOINT-SCOPE), slices 3 and 4:** A: one layout per box. B: plus one per edge group (width, depth, height edges). C: per edge. Recommendation A. **Answered A on 2026-11-26.**
- **Frage 384 (JOINT-PATTERNS), slices 3 and 4:** A: Even and Centred (Count 1 one centred, 2 the closer pair, 3 or more fewer at the edges; one field Joint gap). B: four named patterns, no gap. C: A plus a Spread slider. Recommendation A. **Answered C on 2026-11-26.**

## Decisions

Customer 2026-11-25: the two wishes above (Part A "as an option", Part B "as advanced"). Customer 2026-11-26: Frage 385 A (slice 1); Frage 381 JOINT-BEND A, Frage 382 JOINT-SIZE A, Frage 383 JOINT-SCOPE A, Frage 384 JOINT-PATTERNS C (Spread slider added). Architect 2026-11-25 (`adrs.md`): slicing A0, A1, B tabs, B fingers; `format_version` unchanged in every slice. Product owner (**Proposal**, vetoable at the demo): `curve` default for new boxes only after Frage 381; Advanced group collapsed; Joint gap default 2 times the joint size.

## UX notes

See `ux-notes.md` (short, Proposals): Bend through tabs next to Wall slits, the Advanced group, muted Auto values, Spread.

## Links

Requirements: R-EDIT-055. Related: `specs/0141-parametric-generators/`, `specs/0147-rounded-box/`, `specs/0149-path-box/`, `adrs.md`, ADR 0080, ADR 0082 §5, ADR 0083 §13. PR: -
