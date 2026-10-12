# Combine open paths: compound paths with open outlines

Status: Draft
Priority: Should
Origin: Customer (2026-10-12: "ja, schreib die spec" to the product owner's proposal: compound paths may hold open outlines, fill closes them like SVG, stroke stays open, Combine accepts open paths, and Split, Break apart, Cut, the Boolean operations, the Node tool and SVG import and export each have a stated behaviour; build after `0050`, before `0049`). This is option B of `0035` Question 1, which the customer now takes. Customer decisions that apply: primitives are converted to paths silently (`0047`); keyboard shortcuts are reviewed once at the end (`README.md`). The rules in "Decided by the product owner" and the one criterion marked **Proposal** are mine; the customer can change them.

## User value

As a maker who cuts or plots line work, I want to combine open paths (score lines, fold lines, a hatch, the strokes of a drawing) into one object, alone or together with closed shapes, so that a plate and its engraving lines or a set of plotter strokes move, scale, style and stay together as one object, and I can take them apart again without losing a node.

**What the reference tools do** (as far as I know; check against the customer's versions):

- *Inkscape* Combine (Ctrl+K) accepts open and closed paths and writes one `<path>` with several subpaths; only closed subpaths get a `Z`. Fill closes every open subpath with a straight line, by the SVG rule. Its Boolean operations close open paths silently, which turns a "U" into a "D" without a word (`0016`, "Closed paths only").
- *Illustrator* compound paths accept open paths; the fill closes them the same way.
- *LightBurn* has no compound path object; lines are grouped. Its Boolean operations need closed shapes.

**What we do better:** Combine keeps every node, handle and direction of an open path, so Split or Break apart gives it back exactly. Open lines may cross each other and the shapes they lie on (a hatch, a score line across a plate) without a refusal. The Boolean operations refuse an open outline and name the object, as they already do for open paths, instead of closing it silently. Every notice still says what happened and how many.

## Fact check (current behaviour, read this first)

- **Combine refuses open paths** (`0035` criterion 9, built in `curvyo-ui-core` `combine.rs`, `CombineRefusal::OpenPaths`): "Combine needs closed paths. 1 of 3 selected objects is open. Nothing was changed.", the open paths outlined in red. Its dimmed note reads "Select two or more closed shapes." and the enabled note warns "Needs closed paths: 1 of 3 selected is open.". This spec removes that refusal and those notes.
- **A compound path holds closed outlines only** (`0016` criterion 41, `0035` "Out of scope", `0048` "Out of scope"), but only because no command writes an open one. The stored form already has a `closed` register per outline (`curvyo-document-core` `subpath_codec.rs`, since format 8), Combine and the Boolean operations write it as true, and the reader reads it: the renderer, the hit test, the selection box, the Boolean availability, Split and Break apart already handle an open outline of a compound path (for example from a hand-edited file). The stored shape does not change and `format_version` stays (criterion 26, `adrs.md`).
- **Fill of an open path already closes it with a straight line** (`0007`, built in `curvyo-render-core` and `curvyo-geometry-core`), and the same code already does so for an open outline of a compound path. This spec pins that rule with tests.
- **The Boolean operations refuse open paths** (`0016` criterion 15) and already count a compound path with an open outline as open, with the short sentence (`curvyo-ui-core` `boolean.rs`, `is_open`). Fracture and Flatten (`0037` criterion 2) and Offset (`0038`) are specified, not built. **Cut refuses every compound path** (`0036` criterion 9). **The Node tool does not edit compound paths** (`0016` criteria 38, 38a). **No SVG import or export exists** (`0024` is not specified).
- `format_version` is 11 on `main` (`0023`).

## Words used below

- **Outline:** one contour of a path, open or closed. A compound path has two or more.
- **Open outline:** an outline with a first and a last node that are not joined by a segment. **Closed outline:** one whose last node is joined to its first.
- **Closing line:** the straight line from the last node of an open outline to its first. It is used for fill only; it is never stroked, hit as a stroke, or output as a line.
- **Depth, region, touch, part:** as `0035` and `0048` define them, computed over the **closed** outlines only. Open outlines do not enclose anything and are never holes.
- **Length** of an open outline: the arc length of its segments. Below 0.001 mm (the kernel grid, `0016` criterion 39) it is **zero-length**.

## Acceptance criteria

### The model, drawing and selection

1. Given a compound path, then each of its outlines is open or closed, independently of the others, and any mix is allowed: all closed (today's compound path), all open, or both. The first outline may be open.
2. Given a compound path with an open outline and Stroke on, then the open outline is stroked from its first to its last node only, with the Cap setting at both ends and the Join setting at inner nodes; nothing is stroked along its closing line. Test: a compound path of a far-away closed square and an open "U" with nodes (0, 0), (0, 10), (10, 10), (10, 0), stroke 1 mm black, fill off: a pixel at (5, 10) is black, a pixel at (5, 0) is not.
3. Given a compound path with an open outline and Fill on, then the fill area is computed as if every open outline were closed by its closing line, with the nonzero rule over all outlines and every outline in the direction it is stored (the SVG rule; decision 2). Test (the U of criterion 2, fill on, the square far away): a pixel at (5, 5) is filled. With the U inside the body of a closed square (0, 0) to (40, 40) translated so it lies at (15, 15) to (25, 25): run in the same direction as the square, the pixel at (20, 20) is filled; run the other way, it is not (it shows what is behind the object). Both results are pinned in a test.
4. Given a compound path with open outlines, then a click, marquee or lasso hits it where it is painted, by the rules of an ordinary open path (`0014`): on the stroke of every outline, and inside the fill area of criterion 3 when Fill is on. The selection box and the measured size include every outline, open or closed; the closing line adds nothing to the box beyond the nodes and curves it joins.
5. Given a compound path with open outlines, then move, resize, rotate, skew (drag and typed), Duplicate, Delete, a Ctrl-copy move, Group, the Layers tab rows and every style edit work as on any compound path (`0016` criteria 35 to 36a); open outlines stay open and keep their direction. The panel subject line reads "Compound path". The Markers block is not shown and no marker is drawn, as for every compound path (`0016` criterion 38b).

### Combine

6. Given two or more selected objects of which at least one is an open ordinary path or a compound path with an open outline, when the maker activates Combine, then Combine is no longer refused for being open: all operands are replaced by one compound path that holds every outline of every operand, open or closed, in the order of `0035` criterion 3. Every node, handle and node kind is unchanged. Test: two open lines and a circle (4 nodes) give one compound path with 3 outlines and 2 + 2 + 4 nodes.
7. Given criterion 6, then the closed outlines get their winding from their depth as `0035` criterion 4 says, with depth counted over the closed outlines only, and every open outline keeps the direction it had (its first node stays first). Test: Combine an open line from (0, 0) to (10, 0) with a circle, then Break apart: the line's first node is (0, 0).
8. Given criterion 6, then an open outline may touch or cross any other outline, open or closed, and itself; Combine is refused for touching only when two **closed** outlines touch or cross (`0035` criterion 10), and for self-crossing only when a **closed** outline crosses itself (`0035` criterion 10a). Test: a closed plate 0 to 100 mm with an open score line from (-10, 50) to (110, 50) across it combines; ten open lines forming a crossing grid combine; two overlapping closed squares plus an open line are refused with the touching sentence of `0035`, and only the two squares are outlined in red.
9. Given an open outline with at least 2 nodes that is zero-length (all its nodes at one point), then Combine accepts it like any open outline and it is kept, unchanged, in the result. The no-area refusal (`0016` criterion 16) applies to closed outlines only. Test: an open outline of two nodes both at (5, 5) and a circle combine into a compound path of 2 outlines.
10. Given an operand outline with fewer than 2 nodes (only a hand-edited or foreign file can hold one; this editor removes a path left with fewer than two nodes, `0002` criterion 13), then Combine leaves it out of the result, and the notice adds "1 outline with a single node was removed." (plural "2 outlines with a single node were removed."). When every outline of every operand has fewer than 2 nodes, Combine is refused with "Combine needs outlines with two or more nodes. Every selected outline has a single node. Nothing was changed." (no Refusal outline: a single node draws nothing to outline).
11. Given a rectangle, ellipse, polygon or star among the operands, then it contributes its outline as Object to path makes it, with no notice about the conversion (customer decision, `0047`).
12. Given a successful Combine, then the notice of `0035` criterion 7 counts holes among the closed outlines only, and, when the result has open outlines, adds them after the hole count: "Combine: 3 objects became 1 compound path with 1 hole and 2 open outlines." ("with 2 open outlines" when there is no hole; "1 open outline" for one). One commit, the label `combine_paths`, one undo step "Combine" as today.
13. Given the Combine button, then the notes change as follows: dimmed with fewer than two selected, "Select two or more paths or shapes."; the note "Needs closed paths: 1 of 3 selected is open." is removed; the rule line "One object. Inner shapes become holes." stays. Test: with an open path and a circle selected the button is enabled and its note is "Replaces the selection.".
14. **Proposal.** Given at least one operand with a start, end or node marker on (`0018`), then the notice adds "Markers are not drawn on compound paths." because the result draws none (criterion 5). The marker settings stay in the result's style (`0035` criterion 6), so Break apart brings them back for the outline that gets the bottom-most operand's style.

### Split

15. Given a compound path with open outlines, when the maker activates Split, then the parts of `0048` are formed from the closed outlines as today, and each open outline is added to a part by this rule, first match wins: (a) it touches or crosses an outline of a part (touch as in `0048`), then it joins that part, and joins several parts into one when it touches outlines of several; (b) its first node lies in the painted area of a region (inside the region's outer outline and outside its holes), then it joins that region's part; (c) otherwise it is a part of its own. Open outlines that touch each other end in the same part. Test: a plate with a hole and an engraving line inside the plate body, and a separate open line far away: Split gives 2 objects, the plate with its hole and its line (a compound path), and the far line (an open path).
16. Given criterion 15, then a part that is one open outline becomes an ordinary open path, a part that is one closed outline becomes an ordinary closed path, and every other part becomes a compound path. Every node, handle, kind and direction is as before; ids, style, order and selection follow `0048` criteria 3, 5 and 6.
17. Given a compound path that forms one part (for example a plate with score lines that cross its edge), then Split is refused with the sentence of `0048` criterion 7 when the part has a hole. When it has none, both halves change: "Nothing to split. The compound path is one piece. Use Break apart to separate every outline. Nothing was changed." (plural, none with a hole: "The 2 selected compound paths are one piece each."; any of them with a hole: the `0048` plural sentence). No Refusal outline, as in `0048`.

### Break apart

18. Given a compound path with open outlines, when the maker activates Break apart, then every outline becomes its own object as `0048` criterion 11 says: an open outline becomes an ordinary open path with its nodes, handles, kinds and direction unchanged, a closed outline an ordinary closed path. Outlines with fewer than 2 nodes are dropped with the sentence of criterion 10 ("... was removed."). The notice of `0048` criterion 12 is unchanged otherwise. Test: a compound path of a closed square and two open lines gives 1 closed and 2 open paths.
19. Given Combine and then Break apart on open paths, then every open path comes back node for node and in its direction; given open paths and closed shapes that Combine accepts, Combine and then Split gives the parts of criterion 15, node for node. Given Split or Break apart and then Combine on the pieces, the result has the same outlines, open and closed, as before.

### Cut, the Boolean operations and the other path commands

20. Given a compound path with open outlines among the operands of Cut, then Cut refuses it as it refuses every compound path (`0036` criterion 9: "Cut does not work on compound paths. Split or Break apart first. Nothing was changed.", the compound path outlined in red). Nothing else about Cut changes.
21. Given a compound path with at least one open outline among the operands of a Boolean operation (Union, Difference, Intersection, Exclusion, Reverse difference), then the operation is refused as for an open path (`0016` criterion 15) and the compound path counts as open: "Union needs closed paths. 1 of 3 selected objects is open or has an open outline. Nothing was changed." (plural "2 of 3 selected objects are open or have open outlines."; the shorter `0016` sentence stays when only ordinary open paths are refused), each offender outlined in red. The tooltip note counts it as open and uses the same long form when it counts a compound path: "Needs closed paths: 1 of 3 selected is open or has an open outline." (the note may wrap). Nothing is closed silently (decision 1).
22. Given the same compound path as an operand of Fracture or Flatten (`0037`), then they refuse it with the sentence of criterion 21 and their own name. Given it as an operand of Offset (`0038`), then Inset refuses it with "Inset needs closed paths. 1 of 2 selected objects is open or has an open outline. Nothing was changed." and Outset refuses it with "Outset does not work on compound paths with open outlines. Break apart first. Nothing was changed." (decision 4), the compound path outlined in red. Before Apply, the Offset caption slot (`0038` UX notes) reads "Needs closed paths: 1 of 2 selected is open." for Inset (the compound path counted as open) and "Open outlines: Break apart first." for Outset. A command that is not built when this slice merges takes the rule into its own spec.

### The Node tool and the Pen

23. Given a compound path with open outlines and the Node tool, then it behaves as every compound path does today (`0016` criteria 38 and 38a): the outlines are drawn, no node, handle or segment overlay is shown, the Node bar slot reads "Nodes of compound paths cannot be edited yet.", and Join, Split at a node, segment bending (`0031`) and Close path (`0034` criterion 21) are not offered for it and refuse if called. The open outlines have no end nodes for the Pen's continue and connect (`0034` criterion 12). Nothing changes.

### SVG import and export (requirements for `0024`)

24. Given the SVG export (`0024`), then a compound path is written as one `<path>` whose `d` holds one subpath per outline in outline order, with `Z` after closed outlines only, and `fill-rule="nonzero"`. An open outline written and read back is still open, with the same nodes and direction. This spec states the requirement; `0024` carries the golden-file test.
25. Given the SVG import (`0024`), then a `<path>` with several subpaths, open or closed in any mix, becomes one compound path with the same outlines, open where the subpath has no `Z`, closed where it has one. A subpath of a single `M` with no segment is dropped and counted in the loss report of `0024`. This spec states the requirement; `0024` carries the golden-file test.

### Saving and the file

26. Given a project with a compound path that has an open outline is saved and reopened, then every outline has the same open or closed state, nodes, handles and order. The stored shape does not change and `format_version` stays (architect, `adrs.md` decision 1; the customer may choose a bump instead): an earlier build opens the file and draws, selects and breaks apart open outlines correctly; only its Split groups an open outline as if it were closed, keeping every node. A file from an earlier build opens unchanged and is not rewritten on open. A golden file `compound_open.curvyo` pins the format; it holds a compound path with a closed outline, an open outline and a zero-length open outline.
27. Given a file whose outline has no open or closed state where one is required, or a state that is not a boolean, then it is refused as damaged and the editor does not crash. Given a file with an outline of a single node, then it opens; the outline draws nothing and is not hit, and criteria 10 and 18 drop it when a command rewrites the object. The `document.json` view lists the open or closed state of every outline.

### Undo, performance and determinism

28. Given Combine, Split or Break apart on input with open outlines, then each is one undo step with its existing operation name, and undo restores every operand with its ids, nodes, direction and open or closed state.
29. Given inputs generated by a deterministic generator inside the test (not stored as fixture files): (a) a closed rectangle with 2000 open lines of 2 nodes inside it, none touching; (b) 1000 horizontal and 1000 vertical open lines of 2 nodes that cross each other in a grid; (c) a compound path of (a), then Combine of (a) and of (b), Split of (c) and Break apart of (c) each complete in under 2 s in a release build on the reference desktop, without panic or hang, and give the golden counts: Combine (a) 1 compound path of 2001 outlines, Combine (b) 1 of 2000, Split (c) refused as one piece, Break apart (c) 1 closed path and 2000 open paths.
30. Given the same objects at the same stacking positions, then Combine, Split and Break apart give the same result however they were selected and on every platform to 1e-9 mm: no arithmetic happens on open outlines beyond copying nodes.

## Out of scope

- **Node editing of compound paths**, open or closed (`0016` Question 7). The Node tool shows them as today (criterion 23).
- **Markers on compound paths** (`0016` criterion 38b). Criterion 14 only says so in the notice.
- **Boolean operations, Fracture, Flatten and Offset that use open outlines** (closing them, or cutting with them as a knife). They refuse (criteria 21, 22).
- **Cut on compound paths** (criterion 20).
- **Closing an open outline of a compound path** by a command. The maker breaks it apart, closes the path and combines again.
- **A choice of fill rule** (even-odd). The fill stays nonzero.
- **What a job does with an open outline or a zero-length outline** (a dot burn on a laser, a pen-down on a plotter). The output slices (`0029` and later) decide; this spec only keeps the outline.
- **Joining open outlines of one compound path to each other**, or to another path, with Join or the Pen.
- **SVG import and export themselves** (`0024`); criteria 24 and 25 are its requirements.
- **Keyboard shortcuts** for any command here. The customer reviews all shortcuts once at the end (`README.md`).

## Decided by the product owner

Change any of these if you disagree. Each has the alternative next to it.

1. **Boolean operations refuse open outlines** (criterion 21). *Alternative:* treat every open outline as closed by its closing line, as Inkscape does and as the fill paints it. Refusing keeps the rule `0016` already has for ordinary open paths, which the customer accepted, and never turns a "U" into a "D" without a word.
2. **An open outline keeps its direction; the fill follows the stored direction** (criteria 3, 7). *Alternative:* Combine orients open outlines by depth like closed ones, so the fill of an open outline inside a shape is always a hole. Keeping the direction makes Combine and Break apart exact inverses and keeps start and end where the maker drew them; the cost is that, with Fill on, the fill of an open outline inside a shape depends on its direction, as in SVG and Inkscape. Most open line work has Fill off.
3. **Open outlines may touch and cross anything** (criterion 8). *Alternative:* the touch refusal of `0035` applies to every outline. Refusing would block the common cases (a score line across a plate, a hatch).
4. **Outset refuses a compound path with open outlines** (criterion 22). *Alternative:* grow every outline, the open ones into closed bands as `0038` criterion 8 does for an open path. That is a bigger change to `0038` and can follow when a maker asks.
5. **Split attaches an open outline to the part it touches or lies on** (criterion 15). *Alternative:* every open outline is a part of its own unless it touches. Lying on a plate counts as overlapping, which is the customer's wording for Split.
6. **Single-node outlines are dropped by the commands that rewrite an object** (criteria 10, 18), with a sentence in the notice. Zero-length outlines with 2 or more nodes are kept (criterion 9).

## Build position

After `0050` `layers-and-objects-panel` and before `0049` `linked-clone` (customer, 2026-10-12). `0050` and `0049` bump `format_version`; `0054` does not (`adrs.md`), so the order is about shared code, not version numbers. Footprint: `curvyo-document-core` (outline state, format), `curvyo-geometry-core` (depth and touch over closed outlines, the part rule of criterion 15), `curvyo-ui-core` (`combine.rs` refusals and notes, the Split and Break apart plans, the Boolean availability), `curvyo-render-core` (fill and stroke of open outlines in a compound path), the notice texts in `frontend`. It collides with `0049` (commands that read and remove paths) and with `0037` and `0038` if one is still open (refusal sentences); it does not run beside them.

## UX notes

By `ux-engineer`, 2026-10-12. Rows in `docs/design-system.md`: "Path toolbox", "Path tooltips and notices", "Compound path in the UI", "Action notice", "Refusal outline", "Live preview outline", "Member box", "Offset section". No new component, token, glyph, placement or shortcut: the slice changes texts and what the existing overlays draw. The rail, notice and refusal rules of `0035` and `0048` apply unchanged.

### Texts, and where they appear

All Combine, Split and Break apart notices are the Path card's Action notice (left edge `--rail-right` + 12, level with Combine); Boolean, Fracture and Flatten refusals are the Boolean card's (level with Union); Inset and Outset refusals are the Offset alert chip 4 px under Apply. Success `role="status"`, 3 s, 5 s above 70 characters; refusal `role="alert"`, 8 s, ending "Nothing was changed.".

- **Combine success** (criterion 12), sentences in this order: the result ("Combine: 3 objects became 1 compound path with 1 hole and 2 open outlines."), the removed single-node outlines (criterion 10), the style sentence of `0035` ("It uses the style of the lowest object."), the markers sentence (criterion 14). What was lost comes before what only looks different. The word "outline" is the one the Break apart rule line already uses ("Every outline becomes its own object."), so no new term reaches the maker.
- **Combine tooltip** (criterion 13): rule line unchanged; notes "Select two or more paths or shapes." / "Use the Select tool." / "Replaces the selection.". The open-path note and the open-path refusal are deleted from the row, not left unreachable.
- **Combine refusals:** touching and self-crossing keep their `0035` sentences; only closed outlines are counted and outlined (criterion 8). The single-node refusal (criterion 10) follows the "Combine needs ..." pattern of every other Combine refusal.
- **Split** (criterion 17): the one-piece refusal drops "with its holes" and "release the holes" together when there is no hole; saying "release the holes" about a plate with only score lines would be wrong. Success notice unchanged.
- **Break apart** (criterion 18): unchanged apart from the removed-outline sentence, which goes after the count, before "Holes became filled shapes.".
- **Boolean, Fracture, Flatten** (criteria 21, 22): "is open or has an open outline" only when a compound path is among the offenders, so the built sentence for plain open paths does not change. The tooltip note uses the same form, so the note and the refusal never disagree about why.
- **Inset, Outset** (criterion 22): the caption slot says it before Apply, the alert chip on Apply, as `0038` does for open paths. Outset points to Break apart, like Cut's "Split or Break apart first.": every refusal names the command to use instead.

### Refusal outline and the other overlays

- **Offender = the whole object.** For a Boolean, Fracture, Flatten, Inset, Outset or Cut refusal the compound path is outlined as a whole (every outline, the closed ones and the open ones), not only its open outlines: the fix is Break apart on the object, and node editing of compound paths does not exist (criterion 23). The Combine touch refusal outlines only the objects whose closed outlines touch (criterion 8).
- **The closing line is never drawn by an overlay.** The Refusal outline, the Live preview outline of a drag (move, resize, rotate, skew), the Offset preview and the hover box all draw an open outline open, from first to last node; the closing line exists only inside the fill. The implementer builds overlay geometry from the stroke path, not from `build_fill_path`. Test: the U of criterion 2 refused by Union shows no red line from (10, 0) to (0, 0).
- **Selection** shows what it shows for every compound path: the selection box over all outlines (criterion 4), member boxes in a multi-selection (none above 500 selected). No per-outline highlight; a plate with 2000 score lines looks the same selected as a plate alone, apart from its box.

### With Fill on, and the Node tool

- Fill paints each open outline closed by its closing line (criterion 3); the stroke stays open with Cap at both ends. A straight score line encloses nothing, so a plate with score lines looks the same with Fill on or off apart from the plate. A curved open outline inside a filled shape can show as a filled or an unfilled "D", depending on its stored direction (decision 2). No notice warns about it: the case is rare in line work, the canvas shows it at once, and Fill off removes it. A path-direction command (none exists yet) would be the fix if makers meet it; that is a backlog item, not this slice.
- **Node tool and Pen** (criterion 23): no node, handle or segment overlay, the Node bar pill "Nodes of compound paths cannot be edited yet.", the hint chip on a double-click, as for every compound path. The Pen shows no target cue (row "Pen target cue") over the end nodes of an open outline of a compound path, because a press would not continue it.

### Properties panel and Layers tab

- **Subject line stays "Compound path"** (criterion 5), with no count of open outlines. The subject slot is 80 to 120 px of 12 px text; "Compound path (2 open outlines)" is about 180 px and would be cut. It names the object's kind and shows no hole count either; the counts belong in the Combine notice. The Layers tab row reads the same.
- Style sections as for every compound path: Cap and Join act on the open outlines (they were already shown for compound paths), the Markers block is not shown (criterion 5; the criterion 14 sentence says why markers seem to vanish).

### Large documents, keyboard, accessibility

Criterion 29 inputs: the 2 s budget with `aria-busy` and the wait cursor on the Path card, member boxes off above 500 selected, at most 200 offenders outlined with the exact count in the sentence. Nothing new for the keyboard: the four Path buttons keep their roving Tab stop, Space and Enter, focus rules and accessible names. Every refusal cause is text, paired with the outline; no colour-only state, no motion.

### Design system

`docs/design-system.md` rows "Path tooltips and notices" (Combine note, open-path refusal removed, new sentences), "Refusal outline" (compound path with open outlines as an offender; no closing line in any overlay) and "Compound path in the UI" (open outlines allowed; subject line without counts) are updated by `ux-engineer` in the slice's PR, when the behaviour lands.

## Links

Requirements: R-EDIT-023 (`docs/requirements.md`); related R-EDIT-003
Amends: `specs/0035-combine-and-break-apart/` (criterion 9, the Combine notes of criterion 2, Question 1 taken as B), `specs/0048-split-compound-path/` (Split and Break apart on open outlines; "Out of scope: open paths" withdrawn), `specs/0016-boolean-operations/` (criterion 41: compound paths may hold open outlines; criterion 15 extended)
Related: `specs/0036-cut-at-crossings/`, `specs/0037-fracture-and-flatten/`, `specs/0038-path-offset/`, `specs/0007-stroke-and-fill-styling/` (fill of open paths), `specs/0050-layers-and-objects-panel/`, `specs/0049-linked-clone/` (build order), `0024` (SVG, not yet specified)
ADRs: `adrs.md` (architect, 2026-10-12: no new ADR, no `format_version` bump)
PR: TBD
