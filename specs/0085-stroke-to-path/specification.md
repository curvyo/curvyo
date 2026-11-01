# Stroke to path: the area a stroke covers becomes a filled outline

Status: Done (#203; was Ready 2026-10-26; customer answers of 2026-10-26: Frage 45 A, Frage 46 A, Frage 47 A; `adrs.md` and UX notes the same day)
Priority: Should
Origin: Customer (backlog note of 2026-10-23 "Stroke to path (turn a stroke into a filled outline)"; chosen by the customer on 2026-10-26, "C vorher", built before `0055`). Items marked **(Proposal)** or **PO** are the product owner's and vetoable at the demo.
Depends on: `0007-stroke-and-fill-styling` and `0017-style-panel-rework` (width, dash, join, cap, miter limit 4), `0018-stroke-markers`, `0016-boolean-operations` (kernel grid and tolerance, region, refusal sentences), `0054-combine-open-paths` (open outlines in compound paths, stroked open), `0038-path-offset` (exact parallel curves), `0074-text-objects` and `0076-single-line-fonts` (Object to path of texts), `0023-groups`, `0049-linked-clone`, `0046-document-shapes` (guides), `0020-undo-redo`.

## Frage 45 (STP-DEFINITION): what Stroke to path means

**One-line definition for the customer to confirm:** *Stroke to path replaces each selected object's stroke with a new closed, filled path whose area is exactly the area the stroke paints on the canvas (width, caps, joins, dashes and markers included), so that the line can be cut, welded, offset or filled like any shape.*

Options: **A** confirmed as written. **B** confirmed with changes (the customer names them). Recommendation: A. Default if unanswered: none; the build does not start before this is confirmed (lesson of Weld, 2026-10-20).

**Decided 2026-10-26: A** (confirmed as written).

## User value

As a maker who cuts, engraves or plots line drawings and single-line lettering, I want to turn a stroke into a real shape in one press, so that a 2 mm line becomes a 2 mm wide piece my laser or plotter cuts around, and so that Weld, Offset and the Boolean operations, which work only on areas, can work on it.

## Field reference

| Tool | What it does | Where it falls short |
|---|---|---|
| Inkscape 1.x, Path > Stroke to Path (Ctrl+Alt+C) | Outlines the stroke incl. markers; an object with fill and stroke becomes a group of two paths (fill, stroke). | Many nodes on curves; the group surprises users; dashes give overlapping pieces in older versions; text must be converted first by hand. |
| Illustrator, Object > Path > Outline Stroke | Stroke becomes a filled shape, fill kept as a separate object in a group. Arrowheads expand with "Expand Appearance". | Two commands for strokes with arrowheads; inside/outside alignment differs. |
| Affinity Designer, Expand Stroke | Stroke becomes a filled curve; fill is discarded unless the object is duplicated first. | The fill is lost silently. |
| LightBurn | No stroke concept; lines are cut on their centre. Offset of an open line gives a closed band. | No caps, joins, dashes; no visual stroke to start from. |

**What we do better.** One button next to Object to path in the Select bar, on any selection: paths, compound paths, primitives, texts (single-line texts too, converted in the same step), groups at any depth. The result looks exactly as before (Frage 47 A), dashes and markers included, as one non-overlapping region per object, with few smooth Bézier segments instead of hundreds of corners. The fill part of a shape keeps its kind, so a rectangle stays a rectangle (Frage 46 A).

## Terms

- **Subject:** every selected leaf that draws a stroke (Stroke on, width above 0.002 mm, stroke alpha above 0), and every such leaf inside a selected group at any depth. A text counts as the leaves Object to path gives it (`0074` criterion 22, `0076` criterion 17). A linked clone counts as the path it shows (`0049`, own placement and own style of `0057` included).
- **Stroke area** of a subject: the region the canvas paints for its stroke: the stroke swept along every outline (open outlines stay open, `0054`), with its width, join (miter limit 4, `0007` criterion 11), cap at every open end and every dash end, the dash pattern, and the markers of `0018`. Alignment is always centre (the only one the product has). Where the stroke overlaps itself, the area counts once (nonzero union).
- **Stroke outline:** the new path or compound path whose region is the stroke area.
- **Fill part:** the subject itself after the conversion, with Stroke off (Frage 46 A).
- Kernel tolerance 0.01 mm and grid 0.001 mm as `0016` criteria 25 and 39.

## Acceptance criteria

### Entry point

1. Given the Select tool with a selection, then the Select bar shows a text button **Stroke to path** directly after **Object to path** and before **Linked clone** (row "Select bar" of `docs/design-system.md`). It is shown whenever anything is selected and enabled when the selection holds at least one subject; otherwise it is dimmed with the Select bar's look (the bar's rule "enabled when it would change something").
2. Given a press, then nothing opens: no dialog, no chip, no options. One press, one result, a notice (criterion 22). The focus returns to the canvas after a mouse press and stays on the button after Space or Enter.
3. **(Proposal)** Given the canvas has the focus and the selection holds a subject, when the maker presses Ctrl+Alt+C (Inkscape's key), then the same command runs. The ux-engineer checks the key against the shortcut table and the OS before Ready; if it collides, there is no shortcut.

### Geometry of the stroke outline

4. Given an open straight path (0, 0) to (100, 0) with width 2 mm and Butt cap, then the stroke outline is one closed path of 4 corner nodes, (0, -1), (100, -1), (100, 1), (0, 1), area 200 mm² within 0.01 mm². With Square cap it is 102 x 2 mm (area 204 mm²); with Round cap it is a stadium with two half circles of radius 1 mm, area 203.14 mm² within 0.05 mm², each half circle drawn with at most 2 cubic segments.
5. Given a closed square path 20 x 20 mm with width 2 mm and Miter join, then the result is one compound path of two outlines: an outer square 22 x 22 mm and a hole 18 x 18 mm, area 160 mm² within 0.01 mm², 4 corner nodes each, the hole wound as a hole. With Round join the outer corners are quarter arcs of radius 1 mm and the hole stays sharp; with Bevel join the outer corners are cut by a straight line.
6. Given an open path (0, 0), (50, 0), (0, 5) (a corner of about 6 degrees) with width 2 mm and Miter join, then the corner is drawn as a bevel (miter limit 4, `0007` criterion 11), as the canvas draws it.
7. Given a closed circle of radius 10 mm with width 2 mm, then the result is two outlines, radius 11 mm and radius 9 mm, every point within 0.01 mm of the exact circles, at most 8 nodes per outline, every node Smooth.
8. Given an open path with width 1 mm and the custom dash `5 5` (5 mm on, 5 mm off, `0017` criterion 29) from (0, 0) to (100, 0), Butt cap, then the result is one compound path of 10 outlines, each a 5 x 1 mm rectangle starting at x = 0, 10, ..., 90. With Round cap every dash gets its two round ends. A dash that runs around a corner stays one outline with the corner's join.
9. Given an open path whose stroke crosses itself (a figure eight, width 3 mm), then the result is the union: no outline of the result lies inside the stroke area, and every crossing is one region, not two overlapping ones.
10. Given an open path with a 0018 arrow at the end and a dot at the start, then the arrow and the dot are part of the stroke outline: one region where they overlap the line (no inner line between line and arrow), a separate outline where a marker does not touch the line.
11. Given any subject, then rendered at 20 px per mm, the stroke outline filled in the stroke colour and the original stroke differ in at most 0.5 % of the stroke's pixels, and no differing pixel lies farther than 1 px from an edge. Test: the shapes of criteria 4 to 10 plus a 12-node Bézier path with sharp and smooth nodes, at each join and cap.
    *Note (lead decision 2026-10-26, Proposal, vetoable in the PR):* the canvas draws an object whose dashes number more than 2,000 (over all its outlines, `DashPattern::MAX_DASHES_PER_OBJECT`, counted as `curvyo-render-core` counts them) solid, so Stroke to path outlines such an object solid too, following Frage 45 literally. See `docs/technical-debt.md`.
12. Given any result, then its nodes obey `0016` criteria 24a and 25 (straight segments exact, curves within 0.01 mm of the exact stroke edge, no near-duplicate or collinear nodes), and nodes are Smooth where the tangents agree within 0.1 degree (as `0038` criterion 33). A part that cannot be rebuilt from exact pieces may be a polyline of the 0.001 mm grid (as `0038` criterion 39).

### What each kind of object gives

13. Given a compound path, then every outline is stroked (`0016` criterion 32), open outlines with caps (`0054`), and the stroke areas of all outlines form one stroke outline. Markers are not drawn on compound paths (`0016` criterion 38b), so none are in the result.
14. Given a rectangle, ellipse (arc, pie, chord of `0021` included), polygon or star with a stroke, then its stroke outline is a path or compound path; the primitive itself is the fill part and stays the same primitive kind with its parameters (Frage 46 A).
15. Given a selected text with a stroke (outline font or single-line font of `0076`), then it is converted as by Object to path and each glyph's stroke is outlined, in the same history step, without asking. A single-line text gives a group named as by `0076` criterion 17 that holds one filled stroke outline per glyph: this is how single-line lettering becomes cuttable as a band (PO decision, permit over refuse; `0076` criterion 19 refuses only area commands, and Stroke to path is not one).
16. Given a selected group, then every subject inside it at any depth is converted in place; the group, its name and its other members stay. Inside an entered group the results stay in that group.
17. Given a selected linked clone, then its stroke outline is made from the path and the style it shows, and the clone becomes the fill part as an ordinary path (it is unlinked, `0049`); its original is unchanged. Given a selected original that has clones, then before the conversion every clone becomes an ordinary path that keeps its current look (as `0049` does when an original is deleted), and the notice says so; nothing on the canvas jumps.
18. Given a selection that also holds objects without a stroke, guides (`0046`), or texts with Stroke off, then those are left as they are and the notice counts them. Guides are never converted (they are never output). Hidden and locked objects are never subjects.

### The result

19. Given a success (Frage 46 A), then each subject becomes two objects in the same layer or group: the fill part (the subject, same id, Stroke off) and, directly above it in the stacking order, the stroke outline. A subject with Fill off becomes only the stroke outline, which takes the subject's place and is the only object left of it. The paint follows Frage 47 A: the stroke outline has Fill on in the stroke's colour and alpha, Stroke off, and the subject's object opacity; dash, join, cap and markers are not carried over.
20. Given a success, then the new stroke outlines and the fill parts are selected, the tool stays Select, and the Properties panel names their kinds as for any selection.
21. Given a success, then exactly one commit `stroke_to_path` is written, shown as "Stroke to path" in History; Ctrl+Z restores every subject with its id, kind, style, stacking order and clone links, texts editable, and the selection as before; Ctrl+Shift+Z gives the same result again.
22. Given a success, then the notice reads "Stroke to path: 3 strokes became filled outlines." ("1 stroke became a filled outline."), followed when they apply, in this order, by: "2 objects have no stroke and were left as they are."; "1 text was converted to paths." ("2 texts were ..."); "2 clones were unlinked." Notices follow the row "Action notice" (at most 140 characters per sentence set; 70 for the first sentence). **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no success notice after Stroke to path; refusals stay.**
23. Given a stroke outline saved and reopened, then outlines, node kinds, holes and paint are unchanged. The slice writes only existing kinds: no `format_version` change.
24. Given a stroke outline, then Weld, Union, Offset, Combine and the other commands treat it as the closed path or compound path it is, with no special case. Test: the stroke outline of criterion 4 offset by 1 mm outward is 102 x 4 mm with round corners of radius 1 mm.

### Refusals

25. Given a selected text whose face is missing, then the command is refused with the notice of `0074` criterion 29, the text outlined in red, nothing changed (all or nothing, as `0076` criterion 22).
26. Given a subject with a coordinate that is not finite or beyond `MAX_COORDINATE_MM` (stroke width included), then the command is refused with the sentence of `0016` criterion 17a with "Stroke to path" as the name, offenders outlined in red, nothing changed.
27. Given a subject whose stroke area vanishes on the 0.001 mm grid (for example a path of one node with Butt cap), then it is left as it is and counted in the "no stroke" sentence; nothing is refused for it.

### Determinism, budgets

28. Given the same objects and styles, then the result is node for node the same however they were selected, and golden files in `tests/fixtures/stroke_to_path/` agree to 1e-6 mm on Linux, Windows, macOS and the browser build.
29. Given a release build on the CI runner (browser build up to twice as long), then: (a) the shapes of criteria 4 to 10 take under 50 ms each; (b) 500 characters of Hershey Sans 1-stroke at Size 10 mm, width 0.5 mm, Round cap and join, take under 2 s; (c) "Annabelle" in Great Vibes at 20 mm with a 1 mm stroke takes under 500 ms; (d) a path of 5,000 nodes with the dash `2 1` takes under 2 s. Each matches its golden result.

## Out of scope

- Stroke alignment inside or outside (the product strokes on the centre only).
- Brushes and width profiles of `0033` and the pressure strokes of `0032`; a brush gets its own Expand.
- A live (non-destructive) outline that follows later edits of the line; Ctrl+Z is the way back.
- Options of any kind (keep original, tolerance, join override); no dialog, no chip.
- Outline of the fill (that is Offset, `0038`) and outline of text without its stroke (that is Object to path).
- Kerf compensation, bridges, cut order: manufacturing slices.
- What a stroke outline means for a machine (cut its edge or engrave its area): `0027 manufacturing-roles`.
- Images (`0058`) and their frames.
- Refitting curves of the grid fallback (`0016` follow-up).

## Decided by the product owner (vetoable at the demo)

- Placement in the Select bar after Object to path: the tool rail is at its cap of 22 buttons (`0068` UX notes, "Placement"), and Stroke to path is the sibling of Object to path in every field tool.
- Texts, single-line texts and groups are accepted and converted in the same step (permit over refuse).
- Markers are part of the stroke area: what the maker sees becomes the shape. `0018` Question 5 (markers are appearance only in a job) is untouched, because here the maker converts on purpose.
- Clones of a converted original are unlinked so that nothing jumps (criterion 17).
- Objects without a stroke in the selection are skipped with a count, not refused.

## Notes for the architect (`adrs.md`)

1. Engine: Weld does **not** expand strokes (the stroke band of PR #167 was dropped, `0068` "Out of scope"), and Offset refuses open paths (`0038` criterion 55). The candidates in the project are the exact parallel curves of `offset_pieces` (two-sided, with caps and the dash split added) and `i_overlay`'s stroke builder in the version already pinned, as a fallback; the union is the Boolean grid's. No new dependency without a note.
2. The stroke area must match `render-core`'s drawing (miter limit, dash phase, marker geometry), criterion 11.
3. Clone handling in one commit (criterion 17) and text conversion in the same commit (criterion 15).

## Customer answers (2026-10-26)

**Frage 45 (STP-DEFINITION): confirm the meaning.** Decided: A. See the top of this spec.

**Frage 46 (STP-FILL): what happens to the fill of an object that has fill and stroke?**
- **A** Two objects: the object stays as it is with its stroke switched off (a rectangle stays a rectangle), the stroke outline lies directly above it. Both are selected. (Illustrator and Inkscape do this, but without forcing a group.)
- **B** One object: the fill area and the stroke area united into one silhouette path. The primitive is lost.
- **C** Only the stroke outline; the fill is removed.
- Recommendation: A, because nothing is lost, the primitive stays editable, and B is one Union press away. Default: A.
- **Decided: A** (two objects).

**Frage 47 (STP-PAINT): how is the stroke outline painted?**
- **A** Filled in the stroke's colour, no stroke: the drawing looks exactly as before the press.
- **B** Line art as Weld (Fill off, Stroke on in the stroke's colour at the default width): you see the cut lines, the band looks hollow.
- Recommendation: A, because the result must look like what you converted; whether an edge is cut is decided by the manufacturing role later (`0027`), not by the paint. Default: A.
- **Decided: A** (filled in the stroke colour).

## UX notes

UX notes: [`ux-notes.md`](ux-notes.md) (Select bar placement and wrap, dimmed state, no glyph, notice, selection after, the shortcut check of criterion 3: Ctrl+Alt+C is free).

## Links

Requirements: R-EDIT-004 (Object to path family); related R-EDIT-005, R-EDIT-007, R-EDIT-023, R-FONT-009
Depends on: `specs/0007-stroke-and-fill-styling/`, `specs/0018-stroke-markers/`, `specs/0016-boolean-operations/`, `specs/0054-combine-open-paths/`, `specs/0038-path-offset/`, `specs/0074-text-objects/`, `specs/0076-single-line-fonts/`, `specs/0049-linked-clone/`
ADRs: ADR 0003 (kernel); none new expected
PR: #203
