# Fracture and Flatten: cut overlapping shapes into pieces, or remove the hidden parts

Status: Done (PR #160, merged 2026-10-20). Ready 2026-10-10 (`adrs.md` and the UX notes exist; Flatten means trim to the visible part, decided 2026-10-10; criterion 7 reworded the same day). Amended by `0020-undo-redo` (Ctrl+Z takes the step back) and by `0054-combine-open-paths` criterion 24 (open outlines are closed with a straight line, not refused). UX notes re-checked against `main` on 2026-10-19 (after `0038`, `0051`, `0052`, `0054`): placement unchanged, tooltips aligned with the built Boolean buttons, no customer question open.
Priority: Should
Origin: Customer asked for "Fracture" (cut overlapping shapes into non-overlapping pieces, like Affinity's Divide) and "Flatten" (request of 2026-10-09), and left the exact meaning to me. **The definitions below are my proposals** (Question 1 asks about Flatten in particular).

## User value

As a maker I want to take a stack of overlapping shapes and either (Fracture) cut it into the separate pieces the overlaps make, or (Flatten) trim every shape to what is actually visible, so that the laser never cuts the hidden part of a lower shape twice, a multi-part design becomes real separate parts, and an inlay or marquetry pattern can be cut piece by piece from the shapes I drew over each other.

Both are one idea. Take all outlines of the selected shapes and cut the plane along them. Each resulting **cell** is covered by some of the shapes; the **owner** of a cell is the topmost shape covering it.

- **Fracture** keeps every cell as its own object, painted with its owner's style. Two overlapping circles give three pieces: the left crescent, the lens, the right crescent. Nothing is lost and nothing is merged.
- **Flatten** gives every shape only the cells it owns, joined into one object per shape. The picture on the canvas does not change (fills), but nothing overlaps any more. Two overlapping circles give two objects: the lower one as a crescent, the upper one whole. A shape that is completely hidden has no cell and is removed.

**Reference tools.** Illustrator's Pathfinder has Divide (our Fracture, except that it keeps the pieces grouped and fills them), Trim (our Flatten, except that it also drops strokes) and Merge, in a panel of ten small icons. Affinity's Divide cuts all selected shapes into pieces. Inkscape has Division and Cut Path for exactly two paths (the top one cuts the bottom one and is used up) and nothing that does Fracture or Flatten on a stack; makers use Division repeatedly or a chain of Differences. LightBurn welds and subtracts but does not divide a stack. Where we do better: both commands take any number of shapes in one step, are named in plain words with a tooltip that states the rule, an untouched shape stays exactly as it was (its curves, ids and style), and the notice says how many pieces came out and what was removed.

## Words used below

- **Operand:** a selected closed ordinary path, compound path, rectangle, ellipse, polygon or star. **Region** of an operand: the area its outline(s) enclose under the nonzero rule, as the canvas paints a fill, **whether or not its Fill is on** (`0016` criteria 4 and 5). Paint plays no part.
- **Cell:** a connected area of the plane covered by exactly the same non-empty set of operands.
- **Owner** of a cell: the operand with the highest stacking position among those covering it. Stacking order, never selection order (`0016` criterion 9).
- **Piece:** a cell turned into an object. A cell with holes (another cell inside it, or a hole of an operand) is a compound path (`0016`).
- **Kernel tolerance and grid:** 0.01 mm and 0.001 mm, as `0016` (criteria 39, 25).

## Acceptance criteria

### Entry points and refusals

1. Given any state of the app, then Fracture and Flatten are two commands whose buttons follow the rules of `0016` criteria 1, 1a, 2 and 3 for the Boolean buttons (always rendered; dimmed when fewer than two objects are selected or the tool is not Select; still focusable; a three-line tooltip; no shortcut; the tool never changes). They are the sixth and seventh buttons of the Boolean toolbox (Fracture, then Flatten), which takes the same selection under the same dimming rule. Tooltips: "Fracture" / "Cuts overlapping shapes into pieces." / "Each piece keeps the style of the shape on top. Replaces the selection." and "Flatten" / "Trims each shape to what shows." / "Hidden shapes are removed. Fill does not matter. Strokes follow the trim."
2. Given an operand that is an open path, then the command is refused with the notice of `0016` criterion 15 ("Fracture needs closed paths. 1 of 3 selected objects is open. Nothing was changed.") and the red outline on the offender. An operand that encloses no area, a coordinate that is not finite or beyond `MAX_COORDINATE_MM`: refused as `0016` criteria 16 and 17a. An operand that is a group: "Fracture does not work on groups. Ungroup first. Nothing was changed." (`0023-groups` criterion 28).
3. Given no two operands overlap (their regions share no area; touching along an edge or at a point is not an overlap), then the command is refused with "Fracture: the selected shapes do not overlap. Nothing was changed." (Flatten likewise) and nothing changes.
4. Given the result would have more than 5,000 objects, then the command is refused with "Fracture would make more than 5000 pieces. Nothing was changed."
5. Given a refusal, then everything of `0016` criterion 18 holds (no dialog, buttons stay usable, focus rules).

### Fracture

6. Given two or more operands that overlap somewhere, when the maker activates Fracture, then the operands that take part in an overlap are removed and replaced by one object per piece. An operand that overlaps no other operand is **left exactly as it is** (same object, same id, curves kept) and stays selected.
7. Given criterion 6, then the pieces are exactly the cells of the participating operands: their areas add up to the area of the union of the participating regions (within the bound of `0016` criterion 14), no two pieces share area, and two pieces that share an edge have different sets of covering operands. Two separate areas with the same covering set are two pieces, since a cell is connected (example: a bar laid across both arms of a U overlaps it in two places and gives two overlap pieces; reworded 2026-10-10, `adrs.md`). Test: two squares of side 20 mm, the second offset by (10, 10) mm, give three pieces: the first square without the overlap (an L, area 300 mm², 6 nodes), the overlap (10 x 10 mm, area 100 mm², 4 nodes), the second square without the overlap (an L, 300 mm²). Test: a disc of radius 20 mm and a disc of radius 10 mm in the same centre give two pieces, a ring (a compound path, area π(400 − 100) mm²) and a disc of radius 10 mm. With the small disc on top, the ring has the large disc's style and the inner disc the small disc's. With the small disc below, both pieces have the large disc's style (the picture is unchanged, cut in two).
8. Given criterion 6, then each piece's complete style is a copy of its owner's style, and the piece sits at its owner's place in the stacking order (consecutively with that owner's other pieces), so objects that were not selected and lie between operands keep covering and being covered as before. Pieces of one owner are ordered by the position of their first node, left to right then top to bottom.
9. Given criterion 6, then all pieces and all untouched operands are selected afterwards, the tool is unchanged, one commit is made, stored as `fracture`, atomic as `0016` criterion 28, and a notice names the counts ("Fracture: 3 objects became 7 pieces."). **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no success notice after Fracture.**

### Flatten

10. Given two or more operands that overlap somewhere, when the maker activates Flatten, then each operand O that is covered by an operand above it is replaced by one object holding its **visible region**: the region of O minus the union of the regions of all operands above O. A visible region of several separate areas, or with holes, is one compound path. An operand with nothing above it that overlaps it is **left exactly as it is** (same object, same id, curves kept, same style). Test: the two squares of criterion 7: the first becomes the L of 300 mm² (6 nodes, one closed path); the second is untouched.
11. Given criterion 10, then an operand whose visible region is empty is removed, and the notice says so. Test: a disc of radius 10 mm below a disc of radius 20 mm in the same centre: the small disc is removed and the large one is untouched; the notice reads "Flatten: 2 objects became 1. 1 hidden object was removed." **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): the removal of an empty operand needs no notice.**
12. Given criterion 10, then each result keeps its operand's complete style and its place in the stacking order, so the picture of fills does not change except where noted in criterion 13. The result is selected together with the untouched operands.
13. Given a trimmed operand with a stroke, then the stroke is drawn along its new outline, so a stroke that was hidden under an upper shape is now visible along the trim edge (and stroke is not part of the visible region, as it is not part of any region in this product). The Flatten tooltip is the only warning; stroke-heavy drawings are better fractured or flattened after the stroke is converted (a later feature).
14. Given criterion 10, then one commit is made, stored as `flatten`, atomic, with the notice of criterion 11's form. **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no success notice after Flatten.**

### Result geometry and robustness

15. Given an operand with curves, then a trimmed or cut part of it is straight segments within 0.01 mm of the true result and with the node budget of `0016` criteria 25 and 26 (this is the cost of using the boolean kernel; Combine, Break apart, Split and Cut keep curves). All nodes of a result are Corner nodes without handles, no two consecutive nodes are closer than 0.001 mm, and no node lies on the straight line between its neighbours (`0016` criterion 24). Winding follows `0016` criterion 41.
16. Given the same objects at the same stacking positions, then the result is node for node the same however they were selected, and the same on every platform to 1e-6 mm (`0016` criterion 43).
17. Given the fixtures (in `tests/fixtures/`, golden): (a) 20 squares of side 10 mm at x = 5 i for i = 0 to 19, each overlapping its left neighbour by 5 mm, (b) 2,000 such squares, (c) the fixtures (a) to (i) of `0016` criterion 40, then each command completes in under 2 s without panic or hang and gives a valid result or one of the refusals. Expected for (a), with the highest index on top: Fracture gives 21 pieces of 5 x 10 mm (the 5 mm columns, owner the upper square; the left-most and right-most columns have one covering square); Flatten gives 19 objects of 5 x 10 mm and the top square unchanged (10 x 10 mm). Expected for (b): Fracture 2,001 pieces.
18. Given a result, then the Node tool edits each piece like any path; a piece with holes is a compound path whose nodes the Node tool does not edit yet (`0016` criterion 38).

## Out of scope

- **Merging pieces of equal style** (Illustrator's Merge) and **joining adjacent pieces into one**: Union (`0016`).
- **Keeping curves** in the result. Refitting curves after a boolean is a follow-up of `0016`.
- **Paint-aware behaviour**: a shape with Fill None hides what is below it, as the Boolean operations treat it (Question 2).
- **Strokes**: a shape's stroke is not part of its region; there is no "outline stroke" (stroke to path) here.
- **Groups** (`0023-groups` criterion 28), **open paths**, text.
- **Using one shape as a cutter that is used up** (Inkscape's Division): fracture, then delete the pieces not wanted.
- **Hover preview of the result** (see "Specified but not built" in `specs/README.md`).

## Open questions

Each has a default; nothing blocks.

1. **What Flatten means. Decided by the customer on 2026-10-10: A.** *A (default):* trim every shape to its visible part (criterion 10). *B:* "flatten hierarchy": ungroup everything recursively (needs `0023-groups`). *C:* merge shapes of the same colour that touch. Please say if you meant B or C; A needs no groups and is what a laser maker asks for. If you meant Illustrator's "Flatten Transparency", that has no meaning here (no transparency effects exist).
2. **Fill None hides.** *A (default):* an operand hides whatever is below it whatever its paint, because regions are geometry (`0016`). *B:* operands whose Fill is None do not hide anything. Reason for B: a drawing of fill-less cut lines would not lose its lower lines. Reason for A: one rule for all geometry commands, and the lower line is a second cut of the same area. Recommendation: A, since Flatten is a geometry command.
3. **Order of pieces in the stacking order (criterion 8).** Default: at the owner's place. Option: all at the topmost operand's place.
4. **Limit of 5,000 objects.** Default as written.
5. **Names.** Default: "Fracture" and "Flatten" (your words). Option: "Divide" and "Trim" (Illustrator's), which say more.

Decided by the product owner (change if you disagree): cells and owners as defined; untouched operands stay exactly as they were; hidden operands are removed with a notice; commit labels `fracture` and `flatten`; the 5,000 limit; piece order by first node.

## UX notes" (up to "## Links")

## UX notes

By `ux-engineer`, 2026-10-10, re-checked against `main` 2026-10-19. Rows in `docs/design-system.md`: "Tool rail architecture", "Boolean toolbox", "Boolean tooltip", "Command glyphs", "Action notice", "Refusal outline". Illustrator's Pathfinder and Affinity keep Divide and Trim with the Boolean operations; so do we.

### Placement

Buttons six and seven of the Boolean card, appended after Reverse difference: **Union, Difference, Intersection, Exclusion, Reverse difference, Fracture, Flatten**. Same selection, same dimming, results in straight segments like the Boolean operations (criterion 15); the Path card keeps curves. The Boolean card is already the second card of column B on `main` (it moved with `0035`/`0023`); nothing else moves. The card grows from 224 to 312 px, column B from 412 to 500 px and ends 536 px below the viewport top (40 px clear of the status bar at 800 x 600, measured 2026-10-19; the browser build scrolls the column as the row "Tool rail architecture" says). The toolbar keeps the name "Boolean operations".

**Rail budget after this release:** 20 buttons (column A: Tools 6, Group 2, Path tools 1; column B: Path 4, Boolean 7). Column B is full. A later command for the selection (for example Weld of `0068`) goes to the Select bar, not to this card.

### States

| Situation | Fracture, Flatten |
|---|---|
| Fewer than two objects selected, or another tool than Select | dimmed (as the five Boolean buttons) |
| Two or more objects, open outlines among them | enabled; open outlines are closed with a straight line (`0054` criteria 21, 22, 24) |
| A group, an object without area, a non-finite or out-of-range coordinate | enabled; pressing refuses with the sentences of criterion 2, offenders in the red outline |
| Closed shapes that do not overlap | enabled; pressing refuses (criterion 3), no outline |

Dimming is the rail's: `aria-disabled="true"`, 40 % opacity, focusable, tooltip stays open, activation does nothing.

### Tooltips

The three lines of the built Boolean buttons, with the same functions (`booleanText.ts`): name, rule line, state note.

- Fracture: "Fracture" / "Cuts overlapping shapes into pieces." / state note
- Flatten: "Flatten" / "Trims each shape to what shows." / state note

The state note is the shared one: "Select two or more objects." (plus " Use the Select tool." with another tool), "Closes 2 open outlines with a straight line." with open outlines, else "Replaces the selection.". The rule line describes the command, as the five Boolean rule lines do; it is a name-level description, not an operating tip. The earlier extra sentences ("Each piece keeps the style of the shape on top.", "Hidden shapes are removed. Fill does not matter. Strokes follow the trim.") are dropped (no hint labels, 2026-10-16): the notice reports what happened instead. This replaces the tooltip texts of criterion 1 and the sentence "The Flatten tooltip is the only warning" of criterion 13.

### Interaction and notices

One press, no dialog. All pieces and untouched shapes are selected; the tool stays Select; one undo step. The notice is the Boolean card's (level with Union), `role="status"`, 3 s, 5 s above 70 characters, 8 s above 140 (`0054`). It reports counts and consequences, in this order, each sentence only when it applies:

- "Fracture: 3 objects became 7 pieces." / "Flatten: 2 objects became 1."
- "1 hidden object was removed." (Flatten)
- "Strokes follow the trimmed outlines." (Flatten, when a trimmed object has a stroke; this carries the warning of criterion 13)
- the open-outline sentences of `0054` criteria 21 and 22

Fill None hides what is below (Question 2, A); the removal sentence makes that visible. A piece with holes is a compound path; the Node tool's rule for compound paths applies as built.

### Glyphs

Fracture: three solid pieces separated by gaps. Flatten: the upper square whole, the lower one trimmed to its visible L, a gap along the trim edge. Union has no gap; Exclusion has a hole.

### Keyboard, accessibility, contrast

No shortcut. The Boolean card stays one Tab stop; Up and Down reach the new buttons; Space or Enter presses; focus stays on the button after a key press and returns to the canvas after a mouse press. Accessible names "Fracture" and "Flatten"; the rule line is the description. Contrast as the Boolean card (refusal text 6.5:1, red outline 5.3:1, notice 8.3:1); counts in the sentence, so colour is never the only cue.

### Large documents

Fixture (b) gives 2,001 selected pieces: group box only, no member boxes (above 500, row "Member box"). The kernel runs on the UI thread: `aria-busy` on the card and the wait cursor, as for the Boolean operations; both commands finish within 2 s (criterion 17).

No customer question: the names stay "Fracture" and "Flatten" (his words), one card as Illustrator and Affinity do.

## Links

Requirements: R-EDIT-023 (`docs/requirements.md`); related R-EDIT-003
Builds on: `specs/0016-boolean-operations/` (the kernel, compound paths, region, refusals, determinism; must be merged first)
Related: `specs/0035-combine-and-break-apart/`, `specs/0036-cut-at-crossings/`, `specs/0048-split-compound-path/`, `specs/0038-path-offset/`, `specs/0023-groups/`
ADRs: `adrs.md`
PR: https://github.com/curvyo/curvyo/pull/160
