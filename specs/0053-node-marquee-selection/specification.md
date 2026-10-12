# Node marquee selection: drag a box in the Node tool to select many nodes

Status: Draft (criteria complete; `adrs.md` lists the points for the architect, no new ADR expected; the UX notes are the product owner's draft and the ux-engineer confirms them before `Ready`; criteria marked (Proposal) are the product owner's additions and the customer can veto them at the demo)
Priority: Should
Origin: Customer (request of 2026-10-12). Criteria marked (Proposal) are product-owner additions, built with the slice and vetoable at the demo.

## User value

As a maker cleaning up a traced or imported outline, I want to drag a box around nodes in the Node tool to select all of them at once, and add to or take away from that selection with Shift and Ctrl as I do with objects in the Select tool, so that I can delete a run of thirty stray nodes in one go and join two end nodes that lie on top of each other without hunting for them one by one.

The customer's request (2026-10-12, translated): "In the Node tool I cannot select nodes with a rubber band ('Rahmen ziehen'), only by click and Shift+click. I want the rectangle drag as in the Select tool, to select several nodes at once; I need it for Join and for deleting several nodes. A lasso is not needed."

**Field reference.**

- **Inkscape** (Node tool): a drag on empty canvas draws a rubber band and selects the nodes inside it; Shift+drag adds to the node selection; Alt+drag draws a touch path that selects the nodes it crosses; Ctrl+A selects every node of the edited paths; Escape deselects.
- **Illustrator** (Direct Selection): a marquee selects the anchor points inside it; Shift+marquee toggles them. The Lasso is a separate tool.
- **Blender** (Edit Mode, box select): a drag replaces the selection, Shift+drag extends it, Ctrl+drag subtracts; A selects all. This is the scheme our Select tool uses since `0014` (Shift adds, Ctrl removes).
- **Affinity Designer** (Node tool): a marquee selects nodes, Shift adds to the selection.

**Where we do better:** one modifier rule for objects and nodes (Shift adds, Ctrl removes, Ctrl wins when both are held, as `0014`), where Inkscape and Illustrator use a different rule in each tool; the nodes that the release will select are marked while the box is drawn, with a count; the box works across every path the Node tool shows; the order in which Join reads the two end nodes is defined for a box selection, so a box around two coincident end nodes is always a valid Join; and the number of selected nodes is visible at all times (Proposal).

## What exists today

Facts from the specs and from `curvyo-ui-core/src/node_tool.rs` and `selection.rs` on `main` (2026-10-12); the implementer verifies.

| Item | Today |
|---|---|
| Node selection | A click selects a node (`0002` criterion 7); Shift+click toggles a node, across paths (`0002` criterion 10, `0006` criterion 7). The selection is an **ordered** list of (path, node); its order is Join's "first selected, second selected" (`0006` criterion 9). A node selection and a segment selection exclude each other (`NodeSelection`: nodes, or one segment, or nothing). |
| Which paths show nodes | Every path in the object selection when the Node tool is entered (`0006` criterion 6), every path leaf of a selected group (`0023` criterion 12). Compound paths show no node (`0016` criterion 38a), linked clones none (`0049`), primitives none (`0003` UX). This set is called the **editing set** below. The line "Multi-path node selection" in `0003` "Out of scope" was superseded by `0006` criteria 6 and 7. |
| A press on nothing | A press that hits no node, handle or segment of the editing set clears the node and segment selection **at the press**, unless Shift is held (`PointerDownOutcome::Missed`). Ctrl has no meaning. No box is drawn: rubber-band selection of nodes was deferred by `0002` and `0006` ("Out of scope"). |
| Delete | Delete or Backspace removes every selected node; each pair of nodes that becomes adjacent is joined by one segment from their own handles; a path left with fewer than two nodes is removed (`0002` criterion 13, `0010` criterion 61). History step "Delete nodes" (`0020`, label `delete_anchors`). |
| Join, Split | Join needs exactly two end nodes of open paths, not the two ends of a two-node path (`0006` criterion 8); Split needs exactly one node (`0006` criterion 12). Disabled buttons, no message (`0006` UX). |
| Escape | Cancels a drag, then clears the node selection, then leaves for the Select tool (`0010` criteria 42, 45). |
| Ctrl+A | Select tool only; ignored in the Node tool (`0044` criterion 3). |
| Count | The number of selected nodes is shown nowhere. |
| Select tool marquee | `0014` criteria 8 to 15: 3 px dead zone, green touch and red contain boxes, Shift adds, Ctrl removes strictly, Ctrl wins over Shift, Alt inverts the mode or arms the lasso, a legend at the pointer, the minus badge for Ctrl. |

## Changes to accepted behaviour

| Spec | Today | After this spec |
|---|---|---|
| `0002` and `0006` "Out of scope" (rubber-band selection of nodes) | Deferred | Delivered (criteria 1 to 17) |
| `NodeTool::pointer_down` on nothing | Clears the selection at the press | Clears at the release of a click (criterion 12); a drag becomes a marquee. A plain click looks the same as before |
| Ctrl+click on empty canvas in the Node tool | Clears the selection | Leaves it unchanged, as in the Select tool (`0014` criterion 8) |
| `0044` criterion 3 ("Ctrl+A in the Node tool is ignored") | Ignored | Selects every node of the editing set (criterion 22, Proposal) |

## Acceptance criteria

Document millimetres, X to the right, Y down; zoom 100 % unless stated. "The editing set" is defined in "What exists today". Shift, Ctrl and Alt are read as the host reports them (Ctrl is Cmd on macOS). Lengths are compared with the document's geometric tolerance.

### Part A: the marquee

1. Given the Node tool, when the maker presses at a point where no node, handle or segment of the editing set is hit (the Node tool's hit tolerances, unchanged), including over the fill of a path, over an object outside the editing set and on the pasteboard, and the pointer moves 3 screen pixels or more before the release, then a node marquee starts. A release before the pointer has moved 3 px is a click (criterion 12).
2. Given a node marquee, then a rectangle with one corner at the press point and the opposite corner at the pointer is drawn in every frame, axis-aligned on screen, in the look of the Select tool's marquee box (border and 12 % fill, design-system rows "Marquee/lasso stroke weight" and `--marquee-contain-fill`) in `--marquee-contain` (red) whatever the drag direction, and the cursor is `crosshair` from the press to the release. The box is never part of the document.
3. Given the release of a node marquee, then the **result** is every node of the editing set whose anchor position lies inside the rectangle or on its edge. Handles are never selected, and a handle inside the box does not select its node; segments are never selected; only the anchor position counts, whatever part of the curve lies inside. Example: an open path with anchors (0, 0), (10, 0), (20, 0), (30, 0), the handle of (0, 0) pointing to (6, 0); a marquee from (5, -5) to (25, 5) gives the result (10, 0) and (20, 0); (0, 0) is not in it.
4. Given several paths in the editing set, then the result holds nodes of all of them that lie in the box, across paths, open and closed alike (every anchor of a closed path once). Nodes of paths outside the editing set are never in the result, also when they lie in the box. Compound paths, linked clones and primitives contribute no node. Once `0050` is built, nodes of hidden or locked paths are never in the result.
5. Given the modifiers held at the release, then the result combines with the current node selection by this table (read from the release event; the legend of criterion 10 shows the state live):

   | Held at release | Effect |
   |---|---|
   | none | **Replace**: the selection becomes exactly the result |
   | Shift | **Add**: the result is added |
   | Ctrl | **Remove**: the result is removed, strictly (never added) |
   | Shift and Ctrl | **Remove** (Ctrl wins, as `0014` criterion 13) |
   | Alt, alone or with the above | No effect of its own (no lasso, no touch or contain mode: for a point the two are the same) |

6. Given Replace, then the node selection is exactly the result, and an empty result leaves no node and no segment selected. A segment selection is dropped.
7. Given Add, then nodes already selected stay selected and keep their place in the order, and nodes of the result that were not selected are appended (criterion 9). With a segment selected and a result that is not empty, the segment selection is replaced by the result. An empty result changes nothing.
8. Given Remove, then each node of the result that is selected is deselected, every other selected node stays, and nothing is added. A segment selection is left as it is. An empty result changes nothing.
9. Given nodes that a marquee (or Ctrl+A, criterion 22) adds, then they are appended to the selection order in document order: path by path in stacking order from the bottom to the top, and inside a path from its first anchor to its last. Join reads this order as "first selected" and "second selected" (`0006` criterion 9). Example: open path P1 (lower) from (0, 0) to (10, 0) and open path P2 (upper) from (10, 0) to (20, 0), both in the editing set; a marquee from (8, -2) to (12, 2) selects P1's last node, then P2's first node; Join then gives one open path with the nodes (0, 0), (10, 0), (20, 0), the joined node a Corner at (10, 0), and P2 removed (`0006` criteria 9 and 11).
10. Given a node marquee in progress, then every node of the editing set inside the box is drawn with the hover ring (design-system "Node glyph" hover ring) in the frame of each pointer and key event, so the maker sees the result before the release; and a legend by the pointer (the surface, place and flip rules of the row "Marquee/lasso modifier-state legend") reads "<n> nodes · Replace", "<n> nodes · +Add" or "<n> nodes · −Remove", where n is the count of nodes inside the box ("1 node", "0 nodes"). While Ctrl is held the minus badge of the row "Modifier badge" shows by the pointer, as for the Select tool's Ctrl marquee. No other hover state is shown during the drag.
11. Given a node marquee in progress, when the maker presses Escape, then the marquee is cancelled: the box, the rings, the legend and the badge disappear in that frame, the node or segment selection is exactly what it was before the press (the same nodes, in the same order), and the release that follows does nothing. A second Escape follows the cascade of `0010` criteria 42 and 45 (it clears the node selection).
12. Given a press on nothing (criterion 1) released before the pointer has moved 3 px, then with no modifier the node and segment selection is cleared, at the release; with Shift or Ctrl held at the release it is left unchanged. The object selection, the editing set and the tool never change on such a click.
13. Given a press on a node, a handle or a segment of the editing set, then everything is as today (select, Shift toggle, node drag of every selected node, handle drag, segment select, segment bend of `0031`); a marquee never starts there. Alt has no effect on any press in the Node tool.
14. Given a node marquee in progress, when the maker zooms (wheel, pinch) or pans (middle button, Space held), then the press corner stays fixed in document space, the other corner is the pointer's current document position, the box is redrawn in the next frame, and the result is computed in document coordinates. Example: press at (0, 0), zoom to 400 %, release with the pointer over (10, 10): the box is (0, 0) to (10, 10).
15. Given a node marquee in progress, when the pointer leaves the viewport or the window, then the drag goes on (pointer capture), the box follows the pointer's document position and is drawn clipped to the viewport, and a release outside the window ends the marquee as a release there. The canvas does not pan by itself (auto-pan is out of scope).
16. Given hostile input, then: a box of zero width or height after the dead zone (a purely horizontal drag) selects the nodes that lie on that line within the geometric tolerance; a pointer event with a coordinate that is not finite is ignored and the box keeps its last valid corner; a box that reaches beyond ±1e7 mm selects as if it were cut at that limit. Nothing panics, nothing is written.
17. Given a marquee, a click or Ctrl+A in the Node tool, then only view state changes: no commit, no history step, no change to the saved file and no unsaved-changes marker (`0051`). Undo and redo restore the node selection of a step as they do today (`0020`).

### Part B: what the selection is for

18. Given nodes selected in one or more paths (by marquee, click or Ctrl+A), when the maker presses Delete or Backspace (gate of `0010` criterion 55) or uses the Delete node button, then every selected node is removed in **one** commit, one history step "Delete nodes" (`0020` table); in each path the rule of `0002` criterion 13 applies (each pair of nodes that becomes adjacent is joined by one segment from their own handles; a path left with fewer than two nodes is removed); a group left empty is removed in the same commit (`0023` criterion 14); afterwards no node is selected and the Node tool stays active. Examples: an open straight path (0, 0), (10, 0), (20, 0), (30, 0), (40, 0), a marquee from (5, -1) to (35, 1), Delete: a two-node path from (0, 0) to (40, 0). A marquee around every node of a path, Delete: the path is removed. Nodes of two paths selected, Delete: one step, both paths changed.
19. Given Join, then its rule is unchanged (`0006` criteria 8 to 11): it is enabled exactly when the selection is two end nodes of open paths (not the two ends of a two-node path), and the marquee is one way to make that selection, with the order of criterion 9. (Proposal) While Join is disabled with at least one node selected, its tooltip gains a muted line that names why: "Needs exactly two end nodes of open paths. 5 nodes selected." or "Needs end nodes of open paths. 1 of 2 selected nodes is not an end node."
20. Given a multi-node selection made by a marquee, then every other node action works on it as on a Shift+click selection: Make corner, Make symmetric and Make asymmetric act on every selected node (`0006`), Split stays disabled unless exactly one node is selected (`0006` criterion 12), and a drag of any selected node moves all selected nodes (`0002` criterion 10). Every selected node shows its handles (`0002` criterion 7).

### Part C: keyboard, count, accessibility

21. Given the Node tool, then Escape and Delete keep their meaning (`0010` criteria 42, 45, 61), and no letter key is added by this spec.
22. (Proposal) Given the Node tool, the canvas focused and the gate of `0010` criterion 55 passed, when the maker presses Ctrl+A (Cmd+A on macOS), then every node of every path in the editing set is selected, in the order of criterion 9, replacing the selection (a segment selection is dropped); the key is consumed (`preventDefault`) also when the editing set has no node; a key repeat does nothing more. This amends `0044` criterion 3.
23. (Proposal) Given the Node tool with an editing set that has nodes, then the Node bar shows at its start a status text in a fixed-width slot: "<k> of <n> nodes" (k selected, n in the editing set: "1 of 4 nodes", "5 of 120 nodes"), "<n> nodes" when none is selected, and "Segment, <n> nodes" when a segment is selected. It changes in the frame of every selection change and is not a live region. Where the Node bar slot holds the text-only pill for compound paths or clones (design-system rows "Compound path in the UI", "Linked clone in the UI"), the pill stays and no count is shown.
24. Given the release of a node marquee or Ctrl+A, then a visually hidden live region (polite) says the count of selected nodes after the change: "5 nodes selected.", "1 node selected.", "No nodes selected." A click on a node does not announce (unchanged).

### Part D: speed and size

25. Given an editing set with 10,000 nodes (for example 10 paths of 1,000 nodes), release build, the reference desktop the architect names, then during a node marquee each frame (box, rings and legend count) is drawn within 16.7 ms (60 frames per second over a 5 s trace of continuous movement with the box over half of the nodes), and from the release, or from Ctrl+A, to the frame that shows the new selection takes at most 100 ms.
26. Given a window of 800 x 600 px with the Properties panel open, then the legend stays fully inside the viewport (its flip rules), and the Node bar with the count slot wraps by whole groups as it does today; the count is never clipped for up to 6 digits on each side ("123456 of 999999 nodes").

### Part E: what does not change

27. Given the Select tool, the Pen and the creation tools, then nothing in them changes: the Select tool's marquee and lasso keep `0014`, the Pen's targets keep `0034`.
28. Given a project saved after node marquees, then nothing new is stored and `format_version` does not change.

## Out of scope

- A lasso or a touch path for nodes (the customer: not needed; Inkscape's Alt+drag). Alt stays without meaning in the Node tool.
- Selecting handles or segments with a box; selecting nodes of paths outside the editing set (adding a path to the editing set from inside the Node tool, `0006` "Out of scope").
- Toggle semantics for a modifier (Illustrator's Shift+marquee); a touch or contain mode, or a colour by drag direction (Question 2).
- Join of more than two end nodes (Inkscape pairs them by distance), Join with a new segment (`0006` "Out of scope"); Split at several nodes.
- Shape-preserving delete (Inkscape refits the curve; `0002` "Out of scope").
- Auto-pan when the pointer nears the viewport edge, here and in the Select tool's marquee (`0014` "Out of scope"); both could come together later.
- Invert the node selection, grow or shrink it, select nodes of the same type, Ctrl+Shift+A.
- Moving nodes with the arrow keys (`0044` criterion 13).
- Nodes of compound paths (`0016` criterion 38a).
- Touch-specific gestures; touch and pen follow the mouse rules where the platform delivers them as pointer events.

## Open questions (defaults taken)

None blocks the build; each default is what the criteria say. The lead batches them in one German message.

1. **Which paths a box reaches.** *A (default):* every path whose nodes the Node tool shows (the editing set), across paths, as Shift+click already does since `0006`. *B:* only the path of the last clicked node. Recommendation A: the main use, joining two end nodes of two paths, needs it.
2. **Colour of the box.** *A (default):* always red (`--marquee-contain`), because a node is selected only when it lies inside the box, whatever the direction. *B:* green when dragged to the left and red to the right, as in the Select tool, with the same result either way. Recommendation A: a colour that changes without changing the result would teach a rule that is not there.
3. **Ctrl.** *A (default):* Ctrl removes strictly and wins over Shift, as in the Select tool (the customer's rule of 2026-10-05). *B:* toggle (Illustrator). Recommendation A.
4. **Ctrl+A in the Node tool** (criterion 22, Proposal). *Default:* yes, every node of the editing set.
5. **The node count in the Node bar** (criterion 23, Proposal). *Default:* yes.
6. **The reason line on a disabled Join** (criterion 19, Proposal). *Default:* yes; `0006` chose no message, and the marquee makes "why is Join grey" a frequent question.
7. **Join of more than two end nodes.** *Default:* not in this slice; a small follow-up if the customer wants a box around several gaps to close them all.

## UX notes

Status: **draft by the product owner (2026-10-12)** from `docs/design-system.md`; the ux-engineer confirms or changes it before `Ready`.

- **Box:** the Select marquee box exactly (border weight, device-pixel snapping, 12 % fill), always in `--marquee-contain`. No new token.
- **Pre-selection rings:** the existing node hover ring (18 px, `--accent-hover`) on every node inside the box, drawn under the box border. A node already selected keeps its filled glyph and gets the ring too, so in Remove mode the maker sees which selected nodes will go. Risk: at 20 % the ring is faint on a fill; the ux-engineer may use the 65 % `--hover-box` tone instead.
- **Legend:** the marquee legend surface and placement; text "<n> nodes · Replace", "· +Add", "· −Remove" (U+2212 in "−Remove", as the Select legend).
- **Badge:** the minus badge while Ctrl is held, from the press of a marquee to the release, and wherever a Ctrl press would start one (a press on nothing in the Node tool), exactly as for the Select tool.
- **Count slot (Proposal):** the first item of the Node bar, 12 px `--panel-muted-fg` text, tabular figures, a fixed width measured for "999999 of 999999 nodes", right-aligned in its slot so the buttons after it never move; a 1 px 20 px divider after it.
- **Cursor:** the Node tool's arrow, `crosshair` during a marquee (the Select tool's rule).
- **Rows for the design system:** new "Node marquee" (box colour, rings, legend texts, count slot); changed "Keyboard concept" and the shortcut table (Ctrl+A in the Node tool).

## Links

Requirements: R-EDIT-034 (new, `docs/requirements.md`), R-EDIT-001, R-EDIT-014
Builds on: `specs/0002-path-node-editing/` (criteria 7, 10, 13), `specs/0006-path-merge-split-and-node-types/` (criteria 6 to 12), `specs/0014-advanced-selection/` (criteria 8, 12 to 14, UX notes), `specs/0010-edit-interaction-polish/` (criteria 42, 45, 55, 61)
Touches: `specs/0044-editing-quick-wins/` (criterion 3), `specs/0023-groups/` (criteria 12, 14), `specs/0020-undo-redo/` (step "Delete nodes"), `specs/0050-layers-and-objects-panel/` (hidden and locked)
PR: (set when opened)
