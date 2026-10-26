# Compound path node editing: the Node tool, Join, Split at a node, Close path and the Pen on every compound path

Status: In progress (`story/compound-path-node-editing-v2`, no PR yet). Ready since 2026-10-12 (criteria moved from `0054`; `adrs.md` by the architect and UX notes by the ux-engineer the same day; the product owner's rules stay vetoable)
Priority: Should
Origin: Customer decision 7 of 2026-10-12 on `0054` ("doch das soll gehen": compound paths must be node-editable with the Node tool, and the Pen continues open outline ends if sensible), with the customer principle "allow much so the operator does not have to puzzle out what they did wrong": a defined result plus a notice instead of a refusal. Split out of `0054` as its own slice (lead decision, 2026-10-12). The rules marked **PO** in "Decisions" are mine and the customer can change them.

## User value

As a maker I want to edit the nodes of a compound path (a plate with holes, a Boolean result, a plate with score lines) like any path, so that I can fix one corner of a hole, extend a score line or close an outline without breaking the object apart and combining it again.

**What the reference tools do** (as far as I know): *Inkscape*'s Node tool edits every subpath of a path, joins and breaks nodes within it, and its Pen continues any open subpath end. *Illustrator*'s Direct Selection tool edits nodes of compound paths; joining ends of two different compound paths needs a release first. *LightBurn* has no compound path; its node editing works on single shapes.

**What we do better:** every node command works on compound paths with a stated result instead of a refusal: Join of ends in two different objects moves the other object's outlines in (with a notice), deleting the last nodes of an outline removes only that outline, and Close path on a plate with 2000 score lines closes only the outlines the maker selected.

## Fact check (current behaviour)

- The Node tool shows a compound path without nodes and the Node bar reads "Nodes of compound paths cannot be edited yet." (`0016` criteria 38, 38a). Join, Split at a node (`0006`), segment bending (`0031`) and Close path (`0034` criterion 21) skip compound paths. The Pen has no cue on their end nodes (`0034` criteria 1, 12).
- `0054` lets compound paths hold open, touching and crossing outlines and defines their fill (nonzero, open outlines closed by a straight line). This spec builds on those words: outline, open outline, closing line, depth (`0054` "Words used below").
- The stored form holds every outline with its own anchors and ids since format 8; this slice writes nothing new (criterion 12).

## Acceptance criteria

These apply to every compound path: Combine results, Boolean results with holes, and outlines that are open, touching or crossing.

1. Given one or more compound paths in the Node tool's editing set (selected and `N`, a double-click with the Select tool, or a multi-selection as `0006` criterion 6), then the nodes, handles and segments of every outline are shown and editable as on an ordinary path. The Node bar slot "Nodes of compound paths cannot be edited yet." and the double-click hint chip are removed (`0016` criteria 38, 38a withdrawn).
2. Given criterion 1, then click, Shift-click and the node marquee (`0053`, when built) select nodes across outlines and across objects (`0006` criterion 7), and a click on a segment selects it (`0002` criterion 14).
3. Given selected nodes, handles or segments of a compound path, then moving nodes and handles (`0002` criteria 8 to 10), node kind conversion (`0006` criteria 1 to 5, 16), Make line and Make curve, segment bending (`0031`) and inserting a node by double-click on a segment (`0002` criterion 12) work as on an ordinary path, on the outline that holds them. Nodes of several outlines and objects move together in one drag and one commit. The object keeps its id, style and place; every other outline and every anchor id is unchanged. Outlines are never re-wound by an edit and may come to touch or cross anything; the fill follows `0054` criterion 3. Test: in a ring, drag the hole's four nodes 50 mm to the right, outside the ring: the moved outline, still running the hole's way, is painted (nonzero winding -1), and the ring's centre is painted again.
4. Given selected nodes of a compound path, when the maker deletes them, then each outline loses those nodes as `0002` criterion 13 says for a path; an outline left with fewer than 2 nodes is removed from the object; a compound path left with one outline becomes an ordinary path with the same id, style and place (the Markers block appears again and markers draw); a compound path left with none is removed. One commit. Test: a compound path of three open lines of 2 nodes each; deleting one node of the first line leaves a compound path of 2 outlines; deleting one node of the second leaves an ordinary open path with the compound path's id.
5. Given exactly two selected end nodes of open outlines (of compound paths or ordinary open paths), then Join (`0006` criteria 8 to 11) is offered: (a) both ends of one outline with 3 or more nodes: that outline closes, as `0006` criterion 10; (b) ends of two outlines of one object: they become one outline in that object, in the place of the first-selected node's outline, by the order rule of `0006` criterion 9; (c) ends in two objects: the object of the first-selected node survives with its id, style and place, the joined outline stands where its outline stood, the other object's remaining outlines are appended in their order, the other object is removed, and the survivor is a compound path if it now holds two or more outlines. In case (c), when outlines moved besides the joined one, a notice says "Join: 2 more outlines moved into this compound path." ("1 more outline moved"). Test: a plate with a score line (compound) and a separate open line whose end the maker joins to the score line's end: one compound path of 2 outlines, with the plate's style.
6. Given one selected node of a compound path, then Split at a node (`0006` criteria 12 to 15) is offered: an interior node of an open outline splits it into two open outlines of the same object, in its place, first part first; a node of a closed outline opens that outline as `0006` criterion 14, in its place. The object stays one object (Break apart separates it). This is how an outline is opened.
7. Given the Close path group (`0034` criteria 18 to 20) and compound paths in the editing set, then their open outlines with 3 or more nodes are closed too: the outlines that have a selected node, or, when no node of that compound path is selected, all of its open outlines. Sharp and Smooth as `0034` criterion 15; direction unchanged; one commit `close_path`. The notice counts outlines of compound paths apart from paths: "Closed 1 path and 3 outlines." ("Closed 3 outlines."; skipped: "1 outline has fewer than 3 nodes and was not closed."). `0034` criterion 21 is amended for compound paths; the buttons are dimmed only when the editing set holds no open path or open outline with 3 or more nodes.
8. Given the Pen and an end node of an open outline of a compound path, then it is a continue target as for an open path (`0034` criteria 1 to 6, the same cue and chip): the new nodes extend that outline; the object keeps its id, style, place and every other outline; one commit `extend_path`. Its other end is the close target (`0034` criterion 13).
9. Given the Pen continuing or drawing, when it ends on an end node of an open outline of another object (`0034` criteria 7 to 11), then the surviving object is the one `0034` criterion 8 names, the two outlines become one in it, and the other object's remaining outlines move into it as criterion 5 (c) says, with the same notice. `0034` criterion 12 is amended: a compound path's open outline ends are connect targets.
10. Given a selection of nodes across a compound path and other paths, then a node move, Delete, Join, Split at a node, Close path and a Pen extension each make one commit and one undo step with their existing names, and undo restores ids, nodes, directions and open or closed states.
11. Given a compound path generated in the test as `0054` criterion 32 (c) (a closed rectangle with 2000 open lines of 2 nodes inside, 4004 nodes) in the Node tool, in a release build on the reference desktop, then the node overlay is drawn within 500 ms of entering the tool, a drag of one node redraws at 30 frames per second or more, and deleting 2000 selected nodes completes within 1 s.
12. Given a compound path edited in the Node tool and saved, then it reopens with the same outlines, open or closed states, directions and anchor ids. The stored shape and `format_version` do not change.

## Edge cases (where they are decided)

| Case | Result | Criterion |
|---|---|---|
| Hole dragged outside its shape | painted, not re-wound | 3 |
| Edit makes outlines cross | allowed, no notice | 3 |
| Outline left with one node | outline removed | 4 |
| Last but one outline deleted | becomes an ordinary path, same id | 4 |
| All nodes deleted | object removed | 4 |
| Join of ends of one 2-node outline | not offered (`0006` criterion 8) | 5 |
| Join of ends in two compound paths | the first-selected object survives, the other's outlines move in, notice | 5 |
| Split at a node of a closed outline | outline opens, object stays one | 6 |
| Close path with no node selected in a compound path | all its open outlines with 3 or more nodes close | 7 |
| Pen ends on another compound path's outline end | outlines move into the survivor, notice | 9 |

## Out of scope

- **Moving one outline into another object**, or out of it into its own object, other than by Join, Break apart and Split (`0054`).
- **Selecting a whole outline with one click** (a modifier-click on a segment). A follow-up if makers ask.
- **A command to reverse an outline's direction**, and re-winding holes after an edit (criterion 3).
- **Shape-preserving delete** (`0002` "Out of scope").
- **Markers on compound paths** (`0016` criterion 38b).
- **Keyboard shortcuts** for any command here. The customer reviews all shortcuts once at the end (`README.md`).

## Decisions

1. **Compound paths are node-editable, and the Pen continues and connects their open outline ends.** Customer, 2026-10-12 (decision 7 on `0054`).
2. **PO: Split at a node and Join within one object keep one object** (criteria 5 b, 6). Break apart and Split (`0054`) are the commands that separate objects. *Alternative:* split off the piece as its own object, as an ordinary path does.
3. **PO: Join of ends in two objects moves the other object's outlines into the survivor, with a notice** (criteria 5 c, 9), instead of refusing. *Alternative:* refuse and say "Break apart first".
4. **PO: Close path on a compound path closes the outlines with a selected node, or all open outlines when none of its nodes is selected** (criterion 7), so a plate with 2000 score lines is not closed wholesale by one press while nodes are selected elsewhere. *Alternative:* always every open outline, as `0034` does for ordinary paths.
5. **PO: edits never re-wind an outline** (criterion 3): a hole dragged outside its shape paints as a filled shape, as in Illustrator. *Alternative:* re-wind by depth after every edit, which moves node order under the maker's hands.

## Build position

Right after `0054` (lead decision, 2026-10-12). Footprint: `curvyo-ui-core` (node editing session, Join, Split at a node, Close path, the Pen's targets), `curvyo-editor-wasm`, the Node bar and notices in `frontend`. No format change. It does not run beside `0053` (node selection) or any slice that changes the Node tool or the Pen.

## UX notes

By `ux-engineer`, 2026-10-12. Rows in `docs/design-system.md`: "Node glyph", "Handle endpoint", "Segment hover highlight", "Node hit-test radius", "Close path group", "Pen target cue", "Pen hint chip", "Pen cursors", "Action notice", "Edit hint chip", "Compound path in the UI", and "Node marquee" once `0053` lands. **No new component, token, glyph, cursor or shortcut:** a compound path in the Node tool looks and behaves like several ordinary paths that share one style. The texts follow `0054` (the word "outline", past tense, counts in digits, the notice durations of `0054` UX notes).

### Nodes across outlines

- **Every outline shows the overlay of an ordinary path:** node glyphs by kind, handles of selected nodes and their neighbours, segment hover band, segment selection. The closing line of an open outline is never drawn, hovered or hit (`0054` "Closing line"): an open outline ends visibly at its two end nodes, as an open path does.
- **Which outline a node belongs to needs no extra cue.** The node sits on its outline, and the segment hover band shows the segment it starts. Inkscape and Illustrator add none either. Where the outline matters, the command says it before the press (Close path note, Pen chip) or after it (notices below). Rejected: tinting each outline (no colour is free that is not selection, hover, preview or refusal) and a whole-outline highlight on node hover (4004 nodes on a plate with score lines would flicker the plate on every pass).
- **End nodes of open outlines** look like every other node, as end nodes of open paths do today. They are found by the Pen's continue cue and by Join's enabled state, both unchanged in look.
- **Coincident nodes** (two outlines touching at a node, an end node on another outline's node): a click takes the nearest node within the 16 px hit radius; on a tie the selected one, else the one drawn last (later outline, upper object). Shift-click on the same spot then toggles that one only; the node marquee (`0053`) takes both, which is how two coincident ends are selected for Join.
- **Fill while editing:** a hole dragged outside its shape paints (criterion 3, decision 5) and outlines that come to cross paint by nonzero. No notice: the canvas shows it in the frame of the drag, and Ctrl+Z restores it.

### Node bar, notices and texts

- **The Node bar** is the bar of an ordinary path. The pill "Nodes of compound paths cannot be edited yet." and the double-click hint chip are removed. With linked clones in the selection, the clone pill of the row "Linked clone in the UI" stays; it now says nothing about compound paths.
- **Delete** (criterion 4): no notice. An outline that disappears and a subject line that changes from "Compound path" to "Path" (the Markers block comes back) are visible where the maker looks.
- **Join** (criterion 5): Action notice 4 px under the Join button, right-aligned, `role="status"`, only for case (c) with outlines moved: "Join: 2 more outlines moved into this compound path." ("1 more outline moved"). When the survivor had a start, end or node marker on and is a compound path after the Join, "Markers are not drawn on compound paths." follows (the sentence of `0054` criterion 14). Cases (a) and (b) say nothing, as Join on paths does today. Join's tooltip and the disabled-reason line of `0053` criterion 19 say "end nodes of open paths or outlines" instead of "end nodes of open paths".
- **Split at a node** (criterion 6): Action notice under the Split button, because the result differs from Split on a path (one object, not two) and looks the same: "Split: 1 outline became 2. They stay in this compound path." and, for a closed outline, "Split: 1 closed outline was opened.". Break apart (`0054`) is the command that separates them; the notice does not repeat that.
- **Close path** (criterion 7): the notes and the notice of the row "Close path group" count paths and outlines apart, paths first: disabled note "Select an open path or outline with 3 or more nodes."; notes "Closes 3 outlines." / "Closes 1 path and 3 outlines." with a skip "Closes 3 of 4 open outlines. 1 has fewer than 3 nodes."; notice "Closed 1 path and 3 outlines." with "1 outline has fewer than 3 nodes and was not closed.". Because the selected-node rule of decision 4 cannot be seen, **hovering or focusing Sharp or Smooth previews the closing segment of every outline and path the press would close** (the Pen's close preview: 1px dashed `--accent`, the closing node as the hollow glyph of its resolved kind), at most 200 drawn, the note's count exact; gone on leave, blur or press. This applies to ordinary paths too; it reads the same function that decides the note, so preview, note and result never differ.

### Pen on outline ends

- **Continue and close** (criterion 8): the cue, cursor and chip of an open path, unchanged ("Continue path"; the other end of the same outline gives the close cue).
- **Connect to another object's outline end** (criterion 9): the join cue and cursor unchanged. When the removed object has outlines besides the joined one, the chip adds a muted line "2 more outlines move into the result." after the style line of `0034` (the chip may then have four lines). After the commit the Join notice sentence shows in the "Edit hint chip" surface at the joined node, 3 s, `aria-live="polite"`, because the Pen has no button to anchor an Action notice to.
- Targets follow `0050`: no cue on outlines of hidden or locked objects.

### With the node marquee (`0053`)

Whichever of the two lands second amends the other: the marquee, Ctrl+A and the count slot take every node of every outline of a compound path in the editing set (`0053` criterion 4 loses "Compound paths ... contribute no node"); document order is outline by outline in outline order, first anchor to last (`adrs.md`), so "first selected" for Join stays predictable; the count slot counts all outlines' nodes ("5 of 4004 nodes") and is shown for compound paths now that their pill is gone. A marquee over a plate with 2000 score lines rings every node inside the box within the `0053` frame budget (criterion 11 here, criterion 25 there).

### Large documents, keyboard, accessibility

Criterion 11: the overlay of 4004 nodes is drawn within 500 ms (the implementer may skip nodes outside the viewport); Close path preview capped at 200 segments. Keyboard: nothing new. Join, Split, the kind group and Close path keep their Node bar Tab stops, Space and Enter; Delete and Backspace delete; Escape follows the cascade; focus does not move on a notice. Notices are `role="status"`, never colour only; the preview is a dashed line, not a colour change.

### Design system

Updated by `ux-engineer` in the slice's PR, when the behaviour lands: "Compound path in the UI" (Node tool edits every outline; pill and hint chip removed), "Close path group" (outline counts, hover preview), "Pen hint chip" (the outlines line), "Path tooltips and notices" or a Node bar row (Join and Split notices for compound paths), and "Node marquee" (`0053`) for the compound path counts.

## Links

Requirements: R-EDIT-023, R-EDIT-001 (`docs/requirements.md`)
Amends: `specs/0016-boolean-operations/` (criteria 38, 38a withdrawn), `specs/0006-path-merge-split-and-node-types/` (Join and Split at a node on outlines of compound paths), `specs/0034-pen-path-extension/` (criteria 1, 12, 21)
Related: `specs/0054-combine-open-paths/` (words, fill rule, outline states), `specs/0002-path-node-editing/`, `specs/0031-segment-drag-bending/`, `specs/0053-node-marquee-selection/`
ADRs: `adrs.md` (architect, to come)
PR: TBD
