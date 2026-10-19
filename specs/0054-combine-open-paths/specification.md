# Combine open paths: compound paths with open, touching and crossing outlines

Status: In progress (`story/combine-open-paths`, no PR yet; plan in `plan.md`). Ready since 2026-10-12 (customer decisions applied, `adrs.md` re-checked by the architect, UX notes re-checked by the ux-engineer; Q1 and Q2 have recorded defaults and do not block). Amended 2026-10-18: criterion 27 (even-odd import is `0024`'s).
Priority: Should
Origin: Customer (2026-10-12: "ja, schreib die spec" to the product owner's proposal, then the answers of 2026-10-12 to the decisions below: Booleans close open outlines, closed outlines may touch and cross, compound paths become node-editable, no format bump). This is option B of `0035` Question 1 and option B of `0035` Question 3. Customer decisions that apply: primitives are converted to paths silently (`0047`); keyboard shortcuts are reviewed once at the end (`README.md`). The rules marked **PO** in "Decisions" are mine and the customer can change them.

## User value

As a maker who cuts, engraves or plots line work, I want to combine any paths (score lines, fold lines, a hatch, the strokes of a drawing, shapes that overlap) into one object, edit its nodes like any path, and take it apart again without losing a node, so that a plate and its engraving lines or a set of plotter strokes move, scale, style and stay together as one object.

**Customer principle (2026-10-12):** "Allow much so the operator does not have to puzzle out what they did wrong." Every command in this spec prefers a defined result plus a notice that says what happened over a refusal. A refusal stays only where no sensible result exists (a group as operand, a coordinate the kernel cannot read, nothing with two or more nodes, an operand with no area for a Boolean, a compound path for Cut, Outset of open outlines by customer decision 4).

**What the reference tools do** (as far as I know; check against the customer's versions):

- *Inkscape* Combine (Ctrl+K) accepts open and closed, touching and crossing paths and writes one `<path>` with several subpaths; only closed subpaths get a `Z`. Fill closes every open subpath with a straight line (SVG rule). The fill rule is a per-object style setting, and new shapes often carry even-odd, so a Combine of two overlapping shapes can turn the overlap into a hole. The Node tool edits every subpath. Its Boolean operations close open paths silently, so a "U" becomes a "D" without a word.
- *Illustrator* compound paths accept open and crossing paths; the fill closes open ones; the default fill rule is nonzero, with a "reverse path direction" toggle per subpath. The Direct Selection tool edits nodes of compound paths.
- *LightBurn* has no compound path object; lines are grouped. Its Boolean operations need closed shapes, and its fill engrave treats nested shapes alternately (even-odd like).

**What we do better:** Combine never refuses for touching, crossing or open, keeps every node, handle and direction of an open outline, and its notice says how many outlines are open and how many touch or cross. Overlapping shapes keep their look after Combine (the overlap stays filled), and a shape inside another becomes a hole even when it touches the edge. The Boolean operations close open outlines like Inkscape, but say so, and say which outlines enclosed no area and are gone. Every command states what it does with open and crossing outlines.

## Fact check (current behaviour, read this first)

- **Combine refuses open paths** (`0035` criterion 9, `curvyo-ui-core` `combine.rs`, `CombineRefusal::OpenPaths`), **touching or crossing outlines** (`0035` criteria 10, 10a) and **operands without area** (`0035` criterion 11). This spec removes all three refusals and the open-path notes of the Combine button.
- **A compound path holds closed, non-touching outlines only** (`0016` criterion 41, `0035` "Out of scope", `0048` "Out of scope"), but only because no command writes anything else. The stored form already has a `closed` register per outline (`curvyo-document-core` `subpath_codec.rs`, since format 8) and the readers honour it: renderer, hit test, selection box, Boolean availability and Break apart already handle an open outline of a compound path from a hand-edited file; Split groups it as if it were closed (criterion 29). The stored shape does not change and `format_version` stays (criterion 30, `adrs.md`; customer default 2026-10-12).
- **Fill of an open path already closes it with a straight line** (`0007`, `curvyo-render-core` and the point-in-fill test `interior.rs` in `curvyo-geometry-core`), also for an open outline of a compound path. The kernel's flattening (`flatten.rs`, used by the Boolean operations, the touch test and the depth test) closes every outline with the curve through the end handles instead; that changes in this slice (`adrs.md`). The fill is nonzero over all outlines in stored direction (`0016` `adrs.md`: no `fill_rule` register). This spec pins that rule with tests.
- **The Boolean operations refuse open paths** (`0016` criterion 15) and count a compound path with an open outline as open (`boolean.rs` `is_open`). Fracture and Flatten (`0037` criterion 2) and Offset (`0038` criterion 8) are specified, not built. **Cut refuses every compound path** (`0036` criterion 9). **The Node tool does not edit compound paths** (`0016` criteria 38, 38a), and the Pen ignores them (`0034` criteria 1, 12, 21). **No SVG import or export exists** (`0024` is not specified).
- `format_version` is 11 on `main` (`0023`).

## Words used below

- **Outline:** one contour of a path, open or closed. A compound path has two or more.
- **Open outline:** an outline whose first and last node are not joined by a segment. **Closed outline:** one whose last node is joined to its first.
- **Closing line:** the straight line from the last node of an open outline to its first. It is used for fill and for the region of the Boolean operations and Inset; it is never stroked, hit as a stroke, drawn by an overlay or output as a line.
- **Fill area** of a path: the area painted when Fill is on, by the nonzero rule over all its outlines, each in its stored direction, every open outline closed by its closing line (criterion 3). It is defined whether or not Fill is on.
- **Touch:** two outlines, or two parts of one outline, whose closest points are 0.001 mm or less apart, or that cross (`0048`, the kernel grid of `0016` criterion 39).
- **Contains:** closed outline A contains closed outline B when every point of B lies inside A or on it and at least one point of B lies strictly inside A. "Inside A" is the area A paints on its own by the nonzero rule (this matters for an A that crosses itself); "on it" is within 0.001 mm of A, the touch distance. Two coincident outlines do not contain each other. An outline with no area contains nothing.
- **Depth** of a closed outline: the number of other closed outlines of the same compound path that contain it. Even depth is a shape, odd depth is a **hole**. Where no closed outlines touch, this is the same depth as `0035` and `0048` compute (their first-node test); this spec replaces that test with "contains" so that touching and crossing outlines get a defined depth. Open outlines have no depth, are never holes and contain nothing.
- **Region, part:** as `0048` defines them, with depth as above and touch as above, so closed outlines that touch or cross are always in the same part.
- **Length** of an open outline: the arc length of its segments. Below 0.001 mm it is **zero-length**.

## Acceptance criteria

### The model, drawing and selection

1. Given a compound path, then each of its outlines is open or closed, independently of the others, and any mix is allowed: all closed, all open, or both; the first outline may be open. Any outlines may touch or cross each other, and any outline may touch or cross itself.
2. Given a compound path with an open outline and Stroke on, then the open outline is stroked from its first to its last node only, with the Cap setting at both ends and the Join setting at inner nodes; nothing is stroked along its closing line. Test: a compound path of a far-away closed square and an open "U" with nodes (0, 0), (0, 10), (10, 10), (10, 0), stroke 1 mm black, fill off: a pixel at (5, 10) is black, a pixel at (5, 0) is not.
3. Given a compound path with Fill on, then it paints its fill area: nonzero over all outlines, each in its stored direction, every open outline closed by its closing line (the SVG default rule; why nonzero and not even-odd: Decision 3). Test (the U of criterion 2, the square far away): a pixel at (5, 5) is filled. With the U inside a closed square (0, 0) to (40, 40) translated to (15, 15) to (25, 25): run in the same direction as the square, the pixel at (20, 20) is filled; run the other way, it is not. Both results are pinned in a test.
4. Given a compound path, then a click, marquee or lasso hits it where it is painted, by the rules of an ordinary path (`0014`): on the stroke of every outline, and inside the fill area when Fill is on. The selection box and the measured size include every outline; the closing line adds nothing beyond the nodes and curves it joins.
5. Given a compound path with open, touching or crossing outlines, then move, resize, rotate, skew (drag and typed), Duplicate, Delete, a Ctrl-copy move, Group, the Layers tab rows and every style edit work as on any compound path (`0016` criteria 35 to 36a); outlines keep their open or closed state and direction. The panel subject line reads "Compound path". The Markers block is not shown and no marker is drawn, as for every compound path (`0016` criterion 38b).

### Combine

6. Given two or more selected paths or shapes, open or closed, touching, crossing or apart, when the maker activates Combine, then all operands are replaced by one compound path that holds every outline of every operand in the order of `0035` criterion 3. Every node, handle and node kind is unchanged apart from the reversal of criterion 7. Combine is refused only for a group operand and for a coordinate beyond `MAX_COORDINATE_MM` (`0035` criterion 11, unchanged) and in the case of criterion 10. Test: two open lines and a circle (4 nodes) give one compound path with 3 outlines and 2 + 2 + 4 nodes.
7. Given criterion 6, then every closed outline gets its winding from its depth (definition above): even depth runs as the shapes of `0035` criterion 4, odd depth the opposite way, judged by the sign of the area it encloses. A closed outline whose enclosed area is zero within 0.001 mm² (a closed path of two nodes, or a figure eight whose lobes cancel) keeps its stored direction. Every open outline keeps its stored direction (its first node stays first). Tests:
   a. the concentric discs of `0035` criterion 4 still give a ring;
   b. a square (0, 0) to (40, 40) and a square (0, 10) to (20, 30) that touches its left edge: the inner square is a hole, the pixel at (10, 20) is not painted;
   c. squares (0, 0) to (20, 20) and (10, 10) to (30, 30), crossing: neither contains the other, both are shapes, the pixel at (15, 15) is painted and the painted area is 700 mm² within 1 mm², the same picture as before Combine;
   d. two coincident squares: both shapes, the square is painted;
   e. a closed figure eight: both lobes are painted;
   f. an open line from (0, 0) to (10, 0) combined with a circle and broken apart again: the line's first node is (0, 0);
   g. a square (0, 0) to (40, 40) with squares (5, 5) to (25, 25) and (15, 15) to (35, 35) inside it, crossing each other: both inner squares are holes (depth 1, reversed); the pixel at (10, 10) is not painted, the pixel at (20, 20) in their overlap is painted (winding −1); the notice has the touch sentence "2 closed outlines touch or cross." and the holes sentence of criterion 12.
8. Given criterion 6, then the refusals "Combine needs closed paths ..." (`0035` criterion 9), "Combine needs shapes that do not touch ..." (`0035` criterion 10), "Combine needs shapes that do not cross themselves ..." (`0035` criterion 10a) and the no-area refusal (`0035` criterion 11, second sentence) no longer exist. Test: each of these combines: a closed plate 0 to 100 mm with an open score line from (-10, 50) to (110, 50) across it; ten open lines forming a crossing grid; two overlapping closed squares plus an open line; a figure-eight path with two separate squares; a closed path of two nodes with a circle.
9. Given an open outline with at least 2 nodes that is zero-length (all its nodes at one point), then Combine keeps it unchanged in the result. Test: an open outline of two nodes both at (5, 5) and a circle combine into a compound path of 2 outlines.
10. Given an operand outline with fewer than 2 nodes (only a hand-edited or foreign file can hold one; this editor removes a path left with fewer than two nodes, `0002` criterion 13), then Combine leaves it out of the result, and the notice adds "1 outline with a single node was removed." (plural "2 outlines with a single node were removed."). When every outline of every operand has fewer than 2 nodes, Combine is refused with "Combine needs outlines with two or more nodes. Every selected outline has a single node. Nothing was changed." (no Refusal outline: a single node draws nothing to outline).
11. Given a rectangle, ellipse, polygon or star among the operands, then it contributes its outline as Object to path makes it, with no notice about the conversion (customer decision, `0047`).
12. Given a successful Combine, then the notice of `0035` criterion 7 counts holes (closed outlines of odd depth) and, when the result has open outlines, adds them after the hole count: "Combine: 3 objects became 1 compound path with 1 hole and 2 open outlines." ("with 2 open outlines" when there is no hole; "1 open outline" for one). When closed outlines of the result touch or cross another closed outline or themselves, one sentence follows: "4 closed outlines touch or cross." ("1 closed outline touches or crosses itself." for a single self-crossing one). Open outlines are not counted in it: a score line across a plate is the normal case. When two or more holes touch or cross each other, a second sentence follows: "2 holes touch or cross; where they overlap, it stays filled. Undo and Union them first for one hole." (the count is the number of holes that touch or cross another hole; no winding avoids the painted overlap, `adrs.md`). One commit, the label `combine_paths`, one undo step "Combine" as today.
13. Given the Combine button, then the notes are: dimmed with fewer than two selected, "Select two or more paths or shapes."; enabled, "Replaces the selection."; the note "Needs closed paths: 1 of 3 selected is open." is removed; the rule line "One object. Inner shapes become holes." stays. Test: with an open path and a circle selected the button is enabled and its note is "Replaces the selection.".
14. Given at least one operand with a start, end or node marker on (`0018`), then the notice adds "Markers are not drawn on compound paths." because the result draws none (criterion 5). The marker settings stay in the result's style (`0035` criterion 6), so Break apart brings them back for the outline that gets the bottom-most operand's style. (Customer accepted, 2026-10-12.)

### Split

15. Given a compound path with open, touching or crossing outlines, when the maker activates Split, then the parts of `0048` are formed from the closed outlines with depth and touch as defined above (closed outlines that touch or cross are in one part), and each open outline is added to a part by this rule, first match wins: (a) it touches or crosses an outline of a part, then it joins that part, and joins several parts into one when it touches outlines of several; (b) its first node lies in the painted area of a part (the area the part's closed outlines paint by the nonzero rule; a part of crossing shapes has several outer outlines), then it joins that part; (c) otherwise it is a part of its own. Open outlines that touch each other end in the same part. Tests: a plate with a hole, an engraving line inside the plate body and a separate open line far away give 2 objects, the plate with its hole and its line (a compound path) and the far line (an open path); a compound path of two crossing squares and a far circle gives 2 objects, the two squares (a compound path) and the circle.
16. Given criterion 15, then a part that is one open outline becomes an ordinary open path, a part that is one closed outline becomes an ordinary closed path (also one that crosses itself), and every other part becomes a compound path. Every node, handle, kind and direction is as before (Split reverses nothing); ids, style, order and selection follow `0048` criteria 3, 5 and 6.
17. Given a compound path that forms one part (for example a plate with score lines that cross its edge, or two crossing squares), then Split is refused with the sentence of `0048` criterion 7 when the part has a hole. When it has none, both halves change: "Nothing to split. The compound path is one piece. Use Break apart to separate every outline. Nothing was changed." (plural, none with a hole: "The 2 selected compound paths are one piece each."; any of them with a hole: the `0048` plural sentence). No Refusal outline, as in `0048`. This refusal stays: the result would be the same object.

### Break apart

18. Given a compound path with open, touching or crossing outlines, when the maker activates Break apart, then every outline becomes its own object as `0048` criterion 11 says: an open outline becomes an ordinary open path, a closed outline an ordinary closed path (a figure eight stays one closed path that crosses itself), each with its nodes, handles, kinds and direction unchanged; crossing outlines become objects that overlap. Outlines with fewer than 2 nodes are dropped with the sentence of criterion 10 ("... was removed."). The notice of `0048` criterion 12 is unchanged otherwise. Test: a compound path of a closed square, a square crossing it and two open lines gives 2 closed and 2 open paths.
19. Given Combine and then Break apart on open paths, then every open path comes back node for node and in its direction; closed paths come back node for node, reversed where criterion 7 reversed them. Given Split or Break apart and then Combine on the pieces, the result has the same outlines, open and closed, as before.

### Cut, the Boolean operations and the other path commands

20. Given a compound path among the operands of Cut, then Cut refuses it as it refuses every compound path (`0036` criterion 9: "Cut does not work on compound paths. Split or Break apart first. Nothing was changed.", the compound path outlined in red). Nothing else about Cut changes.
21. Given an operand of a Boolean operation (Union, Difference, Intersection, Exclusion, Reverse difference) that is an open ordinary path or a compound path with open outlines, then the operation is no longer refused for being open (`0016` criterion 15 is withdrawn): the operand's region is its fill area, whether Fill is on or off, so every open outline is closed by its closing line. The success notice of `0016` criterion 29 adds "1 open outline was closed with a straight line." (plural "3 open outlines were closed with a straight line."). Test: Union of the U of criterion 2 and a far square gives a compound path of 2 outlines; the U's outline is closed by a straight edge from (10, 0) to (0, 0) and encloses 100 mm²; the notice reads "Union: 2 objects became 1 compound path. 1 open outline was closed with a straight line."
22. Given criterion 21, then an open outline whose closed form encloses no area (all nodes on one line, or zero-length) adds nothing to the region and is not in the result; the notice adds "1 open outline enclosed no area and was removed." (plural "2 open outlines enclosed no area and were removed."; the wording of criterion 10), after the sentence of criterion 21, and counts it only there. Given an operand whose outlines together enclose no area (a single straight line, a set of straight score lines), then the no-area refusal of `0016` criterion 16 applies unchanged: there is no region to operate on. Test: a plate 0 to 100 mm combined with a straight score line, then Difference with a disc on top: the result is the plate minus the disc, the score line is gone, the notice has the sentence; Union of a straight open line and a square is refused by `0016` criterion 16.
23. Given the Boolean buttons, then the tooltip note "Needs closed paths: 1 of 3 selected is open." is removed; when the selection holds open outlines, the note reads "Closes 2 open outlines with a straight line." ("1 open outline"), so the maker knows before the press. The Boolean availability counts open outlines as usable operands.
24. Given an open path or a compound path with open outlines as an operand of Fracture or Flatten (`0037`), then criteria 21 and 22 apply with their names, and `0037` criterion 2's open-path refusal is withdrawn. Given it as an operand of Offset (`0038`): Inset works on the fill area (`0038` defines the region of a compound path as its fill area), closing every open outline, with the sentences of criteria 21 and 22 in its notice; Inset of an operand with no area stays refused (`0038` criterion 17). Outset of an open ordinary path grows a band as `0038` criterion 8 says, and Outset of a compound path with open outlines is refused: "Outset does not work on compound paths with open outlines. Break apart first. Nothing was changed." (customer decision 4), the compound path outlined in red; before Apply the Offset caption slot reads "Open outlines: Break apart first." A command that is not built when this slice merges takes these rules into its own spec.

### The Node tool and the Pen in this slice

25. Given the Node tool or the Pen and a compound path, then the Node tool behaves as today until `0055` `compound-path-node-editing` is built (`0016` criteria 38 and 38a, `0034` criteria 1, 12 and 21): no node overlay, the Node bar slot "Nodes of compound paths cannot be edited yet.", no Pen cue on its end nodes (Decision 7).

### SVG import and export (requirements for `0024`)

26. Given the SVG export (`0024`), then a compound path is written as one `<path>` whose `d` holds one subpath per outline in outline order, with `Z` after closed outlines only, and `fill-rule="nonzero"`. Written and read back, every outline has the same open or closed state, nodes and direction. This spec states the requirement; `0024` carries the golden-file test.
27. Given the SVG import (`0024`), then a `<path>` with several subpaths, open or closed, touching or crossing, becomes one compound path with the same outlines, open where the subpath has no `Z`, closed where it has one. How an imported even-odd fill is handled is decided by `0024` (re-wind by depth, no report). A subpath of a single `M` with no segment is dropped. This spec states the requirement; `0024` carries the golden-file test. *Amended 2026-10-18 (lead, after the `0024` customer decisions of 2026-10-16: re-wind even-odd fills by depth on import, no loss report, SVG import first): the even-odd sentence and the loss-report counts are `0024`'s.*

### Saving and the file

28. Given a project with a compound path that has an open outline and two crossing closed outlines is saved and reopened, then every outline has the same open or closed state, nodes, handles, direction and order.
29. Given criterion 28, then the stored shape does not change and `format_version` stays (customer default of 2026-10-12, architect `adrs.md` decision 1): an earlier build opens the file and draws, selects and breaks apart open and crossing outlines correctly; only its Split differs: it groups an open outline as if it were closed and judges the depth of crossing closed outlines by their first node, which can change the wording of its one-piece refusal; it keeps every node. A file from an earlier build opens unchanged and is not rewritten on open. A golden file `compound_open.curvyo` pins the format; it holds a compound path with a closed outline, an open outline, a zero-length open outline and a closed outline that crosses the first.
30. Given a file whose outline has no open or closed state where one is required, or a state that is not a boolean, then it is refused as damaged and the editor does not crash. Given a file with an outline of a single node, then it opens; the outline draws nothing and is not hit, and criteria 10 and 18 drop it when a command rewrites the object. The `document.json` view lists the open or closed state of every outline.

### Undo, performance and determinism

31. Given Combine, Split, Break apart or a Boolean operation on input with open, touching or crossing outlines, then each is one undo step with its existing operation name, and undo restores every operand with its ids, nodes, direction and open or closed state.
32. Given inputs generated by a deterministic generator inside the test (not stored as fixture files): (a) a closed rectangle with 2000 open lines of 2 nodes inside it, none touching; (b) 1000 horizontal and 1000 vertical open lines of 2 nodes that cross each other in a grid; (c) a compound path of (a); (d) a chain of 1000 closed squares, each crossing the next; then Combine of (a), (b) and (d), Split of (c) and Break apart of (c) each complete in under 2 s in a release build on the reference desktop, without panic or hang, and give the golden counts: Combine (a) 1 compound path of 2001 outlines, (b) 1 of 2000, (d) 1 of 1000 with no hole and the sentence "1000 closed outlines touch or cross."; Split (c) refused as one piece; Break apart (c) 1 closed path and 2000 open paths.
33. Given the same objects at the same stacking positions, then Combine, Split and Break apart give the same result however they were selected and on every platform to 1e-9 mm: no arithmetic changes a node beyond the reversal of criterion 7.

## Edge cases (where they are decided)

| Case | Result | Criterion |
|---|---|---|
| Open outline crossing a closed outline or itself | allowed, no notice | 1, 8, 12 |
| Two crossing closed outlines | allowed; both shapes, overlap painted; notice counts them | 7c, 12 |
| Closed outline touching the inside of another | hole | 7b |
| Two coincident closed outlines | both shapes, painted once | 7d |
| Closed figure eight | kept; lobes painted by nonzero | 7e, 18 |
| Two holes that cross each other inside one shape | both odd depth, both reversed; their overlap is painted (winding −1 under nonzero, and no winding choice avoids it); the touch sentence counts them and the holes sentence says "Undo and Union them first" | 7g, 12 |
| Closed outline with no area (two nodes) | kept, stroke only | 7, 8 |
| Zero-length open outline | kept by Combine; dropped by a Boolean with a sentence | 9, 22 |
| Single-node outline | dropped with a sentence | 10, 18 |
| Open outline in a Boolean | closed by its closing line, with a sentence | 21 |
| Straight open outline in a Boolean | gone from the result, with a sentence | 22 |
| Operand with no area at all in a Boolean | refused (`0016` criterion 16) | 22 |
| Compound path with open outlines in Outset | refused (customer decision 4) | 24 |
| Compound path in Cut | refused (`0036` criterion 9) | 20 |
| Compound path that is one part in Split | refused, same object | 17 |

Node-editing edge cases (a hole dragged outside its shape, the last nodes of an outline deleted, Join across objects) are in `0055`.

## Out of scope

- **A choice of fill rule** (even-odd) per object. The fill stays nonzero (Decision 3); how even-odd SVG input is handled is decided by `0024` (criterion 27).
- **A command to reverse an outline's direction**, and a notice when an open outline's fill depends on its direction (UX notes). A backlog item if makers meet it.
- **Markers on compound paths** (`0016` criterion 38b). Criterion 14 only says so in the notice.
- **Boolean operations that cut with an open line as a knife** (a score line splitting a plate in two). Cut (`0036`) is the command for that on ordinary paths.
- **Outset growing open outlines of a compound path** into bands (customer decision 4).
- **Cut on compound paths** (criterion 20).
- **What a job does with an open, zero-length, touching or crossing outline** (a dot burn on a laser, a pen-down on a plotter, a double cut along a shared edge). The output slices (`0029` and later) decide; this spec keeps the outlines and defines the painted area.
- **Node editing of compound paths**, and the Pen on their open outline ends: `0055` `compound-path-node-editing` (criterion 25).
- **SVG import and export themselves** (`0024`); criteria 26 and 27 are its requirements.
- **Keyboard shortcuts** for any command here. The customer reviews all shortcuts once at the end (`README.md`).

## Decisions

Customer answers of 2026-10-12 to the product owner's decisions, and the product owner's own rules (**PO**, vetoable).

1. **Booleans close open outlines** (criteria 21 to 23). Customer, 2026-10-12, overriding the product owner's refusal: close each open outline with a straight edge, as the fill does, and say so in a notice. Open outlines that enclose no area are dropped with a sentence (criterion 22); an operand with no area at all stays refused.
   **Q1, default pending customer (asked 2026-10-12):** the same closing applies to a single open ordinary path (withdrawing `0016` criterion 15 and `0037` criterion 2's open refusal) and to Inset (`0038` criterion 8, negative distance). Without it, a "U" on its own would be refused while a "U" inside a compound path is closed, which is the kind of rule the maker has to puzzle out. *Alternative:* keep the `0016` refusal for ordinary open paths. Default: yes, as criteria 21 to 24 are written.
2. **An open outline keeps its direction; the fill follows the stored direction** (criteria 3, 7). Customer, 2026-10-12: default stands.
3. **Outlines may touch and cross** (criteria 1, 7, 8, 12). Customer, 2026-10-12, extending the product owner's rule: open outlines may cross anything, and closed outlines may touch and cross each other and themselves; Combine's touch and self-crossing refusals go. **PO, the fill rule:** nonzero, as today. Why: (a) Combine keeps the picture: shapes that overlapped before Combine still paint their overlap, where even-odd would punch a hole into it; (b) holes still work, because Combine winds an outline contained in another the other way (criterion 7); (c) it is the SVG default and what the renderer, the hit test and the Boolean kernel already use, so no register and no format change; (d) one rule for every object, nothing to set. Even-odd would match LightBurn's fill engrave for crossing shapes but would need a stored setting and a format bump. **PO, depth by containment:** an outline touching the inside edge of another is a hole, crossing outlines are both shapes. **PO, Split and Break apart:** crossing closed outlines are one part for Split; Break apart makes them overlapping objects (criteria 15 to 18).
4. **Outset refuses a compound path with open outlines** (criterion 24). Customer, 2026-10-12: default stands.
   **Q2, default pending customer (asked 2026-10-12):** now that Inset works on the fill area with open outlines closed, Outset could do the same instead of refusing. *Alternative:* Outset grows the closed fill area. Default: keep the refusal, as criterion 24 is written.
5. **Split attaches an open outline to the part it touches or lies on** (criterion 15). Customer, 2026-10-12: default stands.
6. **Single-node outlines are dropped by the commands that rewrite an object** (criteria 10, 18), with a sentence. Zero-length outlines with 2 or more nodes are kept by Combine (criterion 9). Customer, 2026-10-12: default stands.
7. **Compound paths are node-editable.** Customer, 2026-10-12, overriding "not in this slice". Specified as its own slice, `0055` `compound-path-node-editing`, built right after `0054` (lead decision, 2026-10-12): it applies to every compound path, touches the Node tool, Join, Split at a node, Close path and the Pen rather than the commands of `0054`, and together the two would be one very large PR. This slice keeps the Node tool as today (criterion 25).
8. **The markers sentence** (criterion 14). Customer, 2026-10-12: accepted.
9. **No `format_version` bump** (criterion 29). Customer, 2026-10-12: no answer, default A of `adrs.md` applies.
10. **PO:** Combine's notice counts closed outlines that touch or cross (criterion 12), so a maker who expected separate shapes learns why the fill looks merged, without a refusal.

## Build position

After `0050` `layers-and-objects-panel` and before `0049` `linked-clone` (customer, 2026-10-12); `0055` `compound-path-node-editing` follows `0054` (lead, 2026-10-12). `0050` and `0049` bump `format_version`; `0054` does not (`adrs.md`), so the order is about shared code, not version numbers. Footprint: `curvyo-document-core` (outline state, format), `curvyo-geometry-core` (depth by containment, touch counting, the part rule of criterion 15, the closed region of open outlines for the Boolean kernel), `curvyo-ui-core` (`combine.rs` refusals and notes, the Split and Break apart plans, the Boolean availability and notices), `curvyo-render-core` (fill and stroke tests), the notice texts in `frontend`. It collides with `0049` (commands that read and remove paths) and with `0037` and `0038` if one is still open (refusal and notice sentences); it does not run beside them.

## UX notes

By `ux-engineer`, 2026-10-12. Adjusted by the product owner the same day to the customer's decisions (removed refusals, new sentences, node editing moved to `0055`); re-checked by the ux-engineer the same day against `docs/design-system.md` and the `0035`, `0048` texts (criterion 22 reworded to the "was removed" pattern of criterion 10, the holes sentence of criterion 12 added for 7g, the notice duration below). Rows in `docs/design-system.md`: "Path toolbox", "Path tooltips and notices", "Compound path in the UI", "Action notice", "Refusal outline", "Live preview outline", "Member box", "Offset section". No new component, token, glyph, placement or shortcut: the slice changes texts and what the existing overlays draw. The rail, notice and refusal rules of `0035` and `0048` apply unchanged.

### Texts, and where they appear

All Combine, Split and Break apart notices are the Path card's Action notice (left edge `--rail-right` + 12, level with Combine); Boolean, Fracture and Flatten notices and refusals are the Boolean card's (level with Union); Inset and Outset notices and refusals are under the Offset button and the Offset alert chip 4 px under Apply. Success `role="status"`, 3 s, 5 s above 70 characters, **8 s above 140 characters** (new, for the notices of three or more sentences this slice makes possible: about 17 characters a second); refusal `role="alert"`, 8 s, ending "Nothing was changed.". Every sentence is in the past tense and says what happened to the maker's outlines ("was closed", "was removed", "moved"); counts are digits, singular for 1.

- **Combine success** (criterion 12), sentences in this order: the result ("Combine: 3 objects became 1 compound path with 1 hole and 2 open outlines."), the touch count ("4 closed outlines touch or cross."), the holes sentence ("2 holes touch or cross; where they overlap, it stays filled. Undo and Union them first for one hole."), the removed single-node outlines (criterion 10), the style sentence of `0035` ("It uses the style of the lowest object."), the markers sentence (criterion 14). What changes the picture or was lost comes before what only looks different. **Crossing holes (7g), decided:** the notice says "Undo and Union them first" because it is the only way to one hole and the maker cannot see why the overlap is filled; it is a success notice, not a refusal (customer principle), and it is not shown for crossing shapes (7c), whose filled overlap is the picture they had before Combine. "Union them first" alone was rejected: after the press "first" is already past, so the sentence names the undo. The word "outline" is the one the Break apart rule line already uses ("Every outline becomes its own object."), so no new term reaches the maker.
- **Combine tooltip** (criterion 13): rule line unchanged; notes "Select two or more paths or shapes." / "Use the Select tool." / "Replaces the selection.". The open-path note, the open-path refusal, the touching refusal, the self-crossing refusal and the no-area refusal are deleted from the row, not left unreachable.
- **Combine refusals** left: group, coordinate bound (`0035` criterion 11) and the single-node refusal (criterion 10), which follows the "Combine needs ..." pattern.
- **Split** (criterion 17): the one-piece refusal drops "with its holes" and "release the holes" together when there is no hole; saying "release the holes" about a plate with only score lines would be wrong. Success notice unchanged.
- **Break apart** (criterion 18): unchanged apart from the removed-outline sentence, which goes after the count, before "Holes became filled shapes.".
- **Boolean, Fracture, Flatten** (criteria 21 to 24): no open-path refusal any more. The success notice adds, after the counts, "2 open outlines were closed with a straight line." and then "1 open outline enclosed no area and was removed.". The tooltip note "Closes 2 open outlines with a straight line." replaces "Needs closed paths: ...", so the note and the notice say the same thing before and after the press.
- **Inset, Outset** (criterion 24): Inset's notice carries the same two sentences. Outset of a compound path with open outlines: the caption slot "Open outlines: Break apart first." before Apply, the alert chip on Apply, as `0038` does for refusals. It points to Break apart, like Cut's "Split or Break apart first.": every refusal names the command to use instead.

### Refusal outline and the other overlays

- **Offender = the whole object.** For an Outset or Cut refusal the compound path is outlined as a whole (every outline, open and closed). Combine has no geometric refusal left, so it outlines nothing new.
- **The closing line is never drawn by an overlay.** The Refusal outline, the Live preview outline of a drag (move, resize, rotate, skew), the Offset preview and the hover box all draw an open outline open, from first to last node; the closing line exists only inside the fill and the Boolean region. The implementer builds overlay geometry from the stroke path, not from `build_fill_path`. Test: the U of criterion 2 refused by Outset shows no red line from (10, 0) to (0, 0).
- **Selection** shows what it shows for every compound path: the selection box over all outlines (criterion 4), member boxes in a multi-selection (none above 500 selected). No per-outline highlight; a plate with 2000 score lines looks the same selected as a plate alone, apart from its box.

### With Fill on, and the Node tool

- Fill paints each open outline closed by its closing line (criterion 3); the stroke stays open with Cap at both ends. A straight score line encloses nothing, so a plate with score lines looks the same with Fill on or off apart from the plate. A curved open outline inside a filled shape can show as a filled or an unfilled "D", depending on its stored direction (decision 2). No notice warns about it: the case is rare in line work, the canvas shows it at once, and Fill off removes it. A path-direction command would be the fix if makers meet it; a backlog item, not this slice.
- Crossing closed outlines paint their overlap (criterion 7c); the Combine notice's touch count is the only cue, the canvas shows the rest.
- **Node tool and Pen in this slice** (criterion 25): as today for every compound path. Node editing gets its UX notes in `0055`.

### Properties panel and Layers tab

- **Subject line stays "Compound path"** (criterion 5), with no count of open outlines. The subject slot is 80 to 120 px of 12 px text; "Compound path (2 open outlines)" is about 180 px and would be cut. It names the object's kind and shows no hole count either; the counts belong in the Combine notice. The Layers tab row reads the same.
- Style sections as for every compound path: Cap and Join act on the open outlines (they were already shown for compound paths), the Markers block is not shown (criterion 5; the criterion 14 sentence says why markers seem to vanish).

### Large documents, keyboard, accessibility

Criterion 32 inputs: the 2 s budget with `aria-busy` and the wait cursor on the Path card, member boxes off above 500 selected, at most 200 offenders outlined with the exact count in the sentence. Nothing new for the keyboard: the four Path buttons keep their roving Tab stop, Space and Enter, focus rules and accessible names. Every refusal cause is text, paired with the outline; no colour-only state, no motion.

### Design system

`docs/design-system.md` rows "Path tooltips and notices" (Combine and Boolean notes, removed refusals, new sentences; the dimmed Combine note becomes "Select two or more paths or shapes."), "Action notice" (8 s above 140 characters), "Refusal outline" (no closing line in any overlay; Combine no longer outlines offenders), "Path toolbox" (open paths and touching shapes no longer named among the refusals) and "Compound path in the UI" (open, touching and crossing outlines allowed; subject line without counts) are updated by `ux-engineer` in the slice's PR, when the behaviour lands.

## Links

Requirements: R-EDIT-023 (`docs/requirements.md`); related R-EDIT-003
Amends: `specs/0035-combine-and-break-apart/` (criteria 9, 10, 10a, the no-area part of 11, the Combine notes of criterion 2, depth definition; Questions 1 and 3 taken as B), `specs/0048-split-compound-path/` (Split and Break apart on open, touching and crossing outlines; depth by containment; "Out of scope: open paths" withdrawn), `specs/0016-boolean-operations/` (criterion 15 withdrawn; criterion 41 holds for Boolean results only), `specs/0037-fracture-and-flatten/` (criterion 2, open refusal withdrawn), `specs/0038-path-offset/` (criterion 8, Inset of open outlines)
Followed by: `specs/0055-compound-path-node-editing/` (node editing of every compound path)
Related: `specs/0036-cut-at-crossings/`, `specs/0007-stroke-and-fill-styling/` (fill of open paths), `specs/0031-segment-drag-bending/`, `specs/0053-node-marquee-selection/`, `specs/0050-layers-and-objects-panel/`, `specs/0049-linked-clone/` (build order), `0024` (SVG, not yet specified)
ADRs: `adrs.md` (architect, 2026-10-12, re-checked the same day after the customer decisions: no new ADR, no `format_version` bump)
PR: TBD
