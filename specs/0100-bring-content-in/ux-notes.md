# 0100 Bring content in: UX notes

ux-engineer, 2026-11-09. Checked against `specification.md` (criteria 1 to 23), `specs/0099-sketch-pages/ux-notes.md` and its design-system section (row drag, "Drop badge", frame cue, "Place clone on"), sketches b, d and i of `specs/0063-manufacturing-workbenches/concept-sketches.html`, `specs/0060-snapping/ux-notes.md` section 10.2 (Alt), the "Linked clone marks" and clone block rows (`0049`, `0057`), `0098` sections 5 and 9, and `docs/design-system.md`. Customer rules applied: no popups, no operating tips or hint labels (2026-10-16), no notice after a success (2026-10-31), hidden not disabled. Every word below names a control or a state, or is a refusal the specification prescribes. No new colour token. Choices marked (Proposal) are vetoable at the demo.

## 1. Scope

Three surfaces: the pointer (canvas drag of objects, row drag of a drawing page), the panel (Place page, the clone block of group/text/image clones, a selected placed page) and the canvas look (cloning drag, placed page, clone marks). What an edit tool does to an object outside the active page's context is `0106`; nothing here changes it.

## 2. Dragging objects onto a laser page (criteria 1 to 6)

- **Source and start.** Select tool, a press on a selected or unselected object, 4px move: the drag of today. Nothing new to learn at the press; the badge decides at the release.
- **When the cloning look applies.** Per frame: the selection holds no member of a laser page (criterion 5) and the pointer is over the frame of a laser page whose extension is on. Both ways, per frame: leaving the page frame returns to the ordinary move look in that frame, re-entering restores the cloning look. Over a "Laser (off)" or "Unknown kind" page, a drawing page, a sketch page or the pasteboard: the ordinary move, no badge.
- **Four looks over a laser page** (one drag, the modifier state picks):

| Held | Badge (glyph, word) | Objects and ghost | Release |
|---|---|---|---|
| nothing | `Link2` "Linked clone" | The originals stay drawn as themselves where they started, no hollow ghost left behind (nothing of them moves). The "Drag ghost" outline of the selection (1.5px hollow `--drag-ghost` over white casing, fill none) follows the pointer at true size, showing where the clones land, with the snapping of a Select move | Place linked clones |
| Alt | `Move` "Move" | The ordinary move look: the objects drawn as themselves at the pointer, their committed outlines left behind as the ghost | Move |
| Ctrl | `Copy` "Copy" | The ordinary Ctrl copy look: originals stay as themselves, copies at the pointer | Copy |
| Ctrl + Alt | `Copy` "Copy" | As Ctrl | Copy (Alt still inverts snapping) |

  Over a drawing page, sketch page, pasteboard or other kind: no Drop badge; the Ctrl copy shows its circle-plus Modifier badge as built.
- **Ghost content.** One outline per drawn leaf of the selection (a group of 200 paths draws 200 outlines, as the group selection preview does at any count); an image draws its frame rectangle, a text its glyph outlines. A selection that holds only guides and dimensions: no ghost, no badge, no frame cue (they are not cloned). Guides and dimensions in a mixed selection: not in the ghost.
- **Badge.** The "Drop badge" of `0099` (DOM, `aria-hidden`, `pointer-events: none`, 20px pill, 12px glyph and word, top-left 16px right of and 16px below the hotspot, mirrored at viewport edges). It replaces the circle-plus "Modifier badge" for the length of the drag over a laser page, so one copy cue shows, never two. The Shift lock badge stays (it sits left-below; the Drop badge is right-below; the readout chip is up-right: the three never meet). A badge word is the whole message; glyph and word, never colour alone.
- **Frame cue.** While a Linked clone, Move, Copy or Placed page badge shows, the laser page frame gets the `0099` cue (2px `--editor-accent` outline, 2px outside, no fill, over the page's normal edge). Gone in the frame the pointer leaves the page, on release and on Escape. No cue for a refusal.
- **Modifier timing.** The badge, ghost and cue follow Alt and Ctrl from window-level key events, in the frame the key goes down or up, with the pointer at rest (as the Modifier badge does). The release is decided from the modifier flags of the pointer-up event, not from the last key event, so the badge and the result cannot disagree. On WebKitGTK a key-up can be missed (the Shift note in "Bend readout and badge"): the badge is corrected at the next pointer move and the release still follows the pointer-up flags. `preventDefault` on Alt key events during the drag, so no browser or webview menu activates on an Alt tap (Windows, Firefox).
- **Alt and snapping (criterion 3).** The two meanings of Alt are kept together as the specification proposes: Alt is Move and inverts snapping (`0060` section 10.2), in the same frame. Consequences, stated once so the review checks them: (a) Alt is pressed after the move has begun (Alt at the press arms the lasso and cycles, `0060`); (b) with snapping on, a Move is placed unsnapped, with snapping off it snaps; (c) the badge says Move whatever the snapping state, the snap marker and guides say the snapping state, as they do today; (d) a maker who wants a clone with snapping inverted has no key for it: switch snapping in the Snapping tab. Nothing in the UI explains this (no hint). Review at the demo; the customer may want a different Move key.
- **Escape.** Any time: ghost, badge, cue gone in that frame, nothing written, originals untouched.
- **After the release (Linked clone).** The laser page becomes active in that frame (rail, tabs, status bar and header follow, `0098`); the clones are selected and the Select tool is kept (picked for the new kind if missing); focus goes to the canvas. The view does not pan or zoom (the pointer was over the page). Select bar shows its Linked clone state for the clones. No notice. After Move and Copy the same activation and selection rules apply (the objects or copies are selected on the laser page).
- **Undo.** "Place linked clones" (criterion 1e): Ctrl+Z removes the clones, the originals were never touched, the selection clears, the active page stays on the laser page (activation is not a history step). Redo places them again at the same spot. Move and Copy undo with their own labels.
- **Count refusals.** Depth 8 (criterion 6): objects are not created; the release gives the `0049` criterion 52 refusal in the canvas notice slot; no ghost travels back (originals never left). Guides and dimensions left out: Action notice, status, 3s, canvas notice slot, "2 guides were not cloned. Guides never run." (criterion 6; "1 guide was not cloned. Guides never run.", "1 dimension was not cloned. ..." with the right noun; mixed: "3 guides and dimensions were not cloned."). This one is a partial result, so it is kept despite the no-success-notice rule.
- **Many objects.** The ghost budget is criterion 22 (16 ms); past 2,000 leaves the ghost draws the selection box outline only (Proposal, as "Many and small" for marks); the badge is unaffected.

## 3. Dragging a drawing page row onto a laser page (criteria 13, 14)

- **Source.** The grip of a drawing page row (`0099` section 2), the same two modes: pointer inside the list reorders as built; outside the list is place mode, per frame, both ways. The row never leaves the list. Laser and "Unknown kind" rows only reorder; releasing them outside the list is today's behaviour. Sketch rows keep `0099`.
- **Targets under the pointer (place mode).**

| Under the pointer | Ghost | Badge | Frame cue | Release |
|---|---|---|---|---|
| Laser page (extension on) | yes, full strength | `Frame` "Placed page" | yes | Place page, one step |
| Drawing page, sketch page, pasteboard, "Laser (off)", "Unknown kind" page | yes, at 40 % | `Ban` "Only on a laser page", cursor `not-allowed` | none | nothing created, no step |
| Panel, rails, bar, status bar (not the list) | none | none | none | cancel, no step |
| Escape | gone in that frame | gone | gone | cancel, no step |

- **Ghost.** The source page's frame as a hollow rectangle plus the outlines of its content (the "Drag ghost" look), at true size for the current zoom, scale 1, rotation 0, its centre on the pointer, snapped as a Select move snaps the frame box (guides drawn as for a move). Content outlines past 2,000 leaves: the frame only. An empty page: the frame only. Under 6px on screen: an 8px ring at the pointer. No fixtures yet, so no slot cue and no fit-by-size snap (`0102` adds them).
- **Badge.** `Frame` is the placed-page glyph throughout (badge, label, list). Same Drop badge as section 2; "Only on a laser page" is the specification's refusal word.
- **After the release.** The placed page is created above the laser page's earlier placed pages, selected; the laser page is active; focus on the canvas; no notice; one step "Place page". Ctrl+Z removes it and clears the selection; the drawing page and its objects were never touched. The source page does not activate during the drag.
- **Many placements of one page.** Allowed (criterion 13 does not forbid it); each one is a separate item with its own label.

## 4. The Place page button (criterion 15)

- **Where.** The laser page's Document tab, one button directly under the Machine block (after its Origin row, before Fixtures); it needs no group of its own. Hidden (not disabled) when the project has no drawing page. Same pattern as "Place clone on" (`0099` section 3), not a new component.
- **Button.** Full width (244 x 28px, 36px coarse), outline "Fit to content" look, label **Place page**, `ChevronDown` 14px at the right (`ChevronUp` while open), `aria-expanded`, `aria-controls`.
- **List.** Opens inline under the button, pushes the rest down, `role="listbox"` named "Place page", `--toolbar-bg`, 4px padding. Rows 36px (44px coarse): `Frame` 12px is not repeated; the drawing page name (truncates, tooltip) at the left and its size in 12px `--panel-muted-fg` tabular at the right ("125 × 75 mm"). The specification asks for the kind chip; every row would read "Drawing", so the size, which tells pages apart, takes its place (Proposal; see the report). Drawing pages only, in list order. At most 6 rows, then it scrolls inside itself.
- **Keys.** Enter or Space opens; focus moves to the first row. Up, Down, Home, End move; Enter, Space or a press chooses; Escape closes, changes nothing, focus back to the button; Tab leaves and closes without choosing.
- **Result.** Places the page at the laser page's centre with the rules of section 3, selects it, closes the list, keeps the laser page active. If the placed page's frame is not fully in view the view pans to centre it, never zooms. **Focus goes to the canvas** (the `0099` section 8 rule: the next action is on the object). One step "Place page". Placing again puts a second page exactly on the first; the new one is on top and selected, the maker moves it. Presses: button, row (2, plus Enter). This is the only keyboard route for placed pages.

## 5. A placed page on a laser page (criteria 16 to 19)

- **Look, unselected.**
  - **Content:** drawn live as the source page draws its members, under the placed page's matrix, in the stacking place of the placed page; a rectangle edited on the source page changes in the next frame; a page larger than the work area is drawn in full, past the laser frame.
  - **Frame:** the source page's frame under the matrix (a rotated rectangle when turned), 1px solid line, `--toolbar-icon` at 65 %, constant screen width, no fill, no casing. It is a mark, not output: not exported, not a production line (criterion 20 a), and it stays visible for an empty placed page, so that page can be found. It reads as a boundary beside the laser page's own edge (`--canvas-bg` rectangle look, which has a fill and a shadow) and unlike the violet dashed clone trace, which exists only while a clone is selected.
  - **Name tag:** the canvas page-label look (`0098` section 5): `Frame` 12px glyph, 4px, name in 12px `--toolbar-icon`, upright at any rotation, `aria-hidden`, `pointer-events: none`. Placed above the top-left corner of the screen bounds of the transformed frame, 6px clear; the name is semibold while selected. Narrow frames follow `0098`: never wider than the frame's screen width, the name truncates first, the whole label goes below a frame width of 24px. A label that would meet the tool rail, the bar or the viewport edge moves below the bounds, left-aligned (Proposal; same rule as the page labels). Not a chip: a placed page is a page in a page, and a chip shape belongs to clone marks and badges.
- **Hit and hover.** The whole frame area hits (criterion 17), as an image is hit by its frame. A press selects the placed page, never a source object. The hover box (1px `--accent` at 65 %) is the transformed frame box. Stacking decides against other items: see the report (open question). Marquee takes a placed page when it wholly contains its box.
- **Selection.** The ordinary box and handles of `0019` on the transformed frame: move, scale, rotate, skew, Flip, typed X/Y/W/H/angle in the Select bar, arrow keys, align and distribute. All readouts and badges as for any object. Nothing writes the source page.
- **Style tab (Proposal; the specification is silent).** A selected placed page replaces the Style sections with a block: `role="group"` named "Placed page", row 1 (14px `--toolbar-icon`, wraps) `Frame` 16px `aria-hidden`, 4px, "Placed page of Coaster." (name cut at 40 characters, tooltip whole); several: "3 placed pages.". No buttons: editing the source page is `0106` (double-click). Mixed with other objects: the block comes first, then the others' sections. About 24px; the Select bar's other controls follow their usual rules.
- **Status bar left and announcer.** Status bar: "Coaster · placed page" (the "Circle · pasteboard" form). Selection announcer: "Placed page Coaster selected, 90.0 by 90.0 mm. Source page: Coaster." (the box size after the matrix). After a placement the announcer speaks this; the page activation is announced as `0098` does ("Laser 1, laser page, 3 of 5").
- **Move that ends fully off the laser page (criterion 18).**
  - **During the drag:** while the box has no overlap with the laser frame, the placed page is drawn at 40 % and the cursor is `not-allowed`; no badge, no text. Overlap again restores full strength in that frame. The overlap test is the transformed box's bounds against the laser frame, tolerance 1e-9 mm.
  - **At the release:** the placed page is back at its start in that frame, no animation, nothing written, no history step. Refusal alert (the validation chip look, 8s or until the next action), canvas notice slot: "A placed page stays on its laser page."
  - **Other routes (Proposal):** a nudge by arrow key, a typed X/Y, align and a numeric transform that would end with no overlap are refused the same way: nothing moves, the typed field reverts, the same notice. A repeated key shows the notice once until another action. Dragging onto another laser page is the same case (there is no badge for a placed page).
- **Duplicate and Ctrl copy.** Ctrl+D (as built) and the Ctrl copy of a move create the second placed page above the first, selected; a copy-drag shows the circle-plus Modifier badge (the pointer stays on one page, no Drop badge). Delete removes the placed page; the source page and its objects stay. No notice. Undo restores selection and stacking place.
- **Group.** The Group button is hidden (not disabled) while the selection holds a placed page; Ctrl+G does nothing visible except the canvas notice "A placed page cannot be grouped." (Proposal; a silent key looks broken). Ungroup is unchanged.
- **Double-click inside.** The notice of `0093` criterion 7 as built until `0106`.
- **Source page changes.** Moved or deleted objects follow in the next frame. Deleting a drawing page that has placed pages: not in the specification (see the report).

## 6. Clones of groups, texts and images (criteria 8 to 12)

- **Select bar.** The Linked clone button is in the tree when the selection holds a group, a text or an image besides paths and primitives (hidden for a layer-only selection, as built). Its look and place do not change. Result as `0049`: clones selected, directly above, Follows original; no notice.
- **Marks (the "Linked clone marks" rules, extended).** When the selection holds a clone whose root is a group, text or image:
  - **Clone trace:** the oriented bounds of what it draws (the original's box under the clone's matrix, a rotated rectangle when turned), 2px dashed 6 on / 3 off `--clone-link` over the white casing. Not one outline per leaf: a clone of a group of 100 paths would mark 10,000 outlines and hide the picture (the 200-trace cap counts clones, not leaves).
  - **Original trace:** the original's oriented bounds, 2px solid. **Root trace** for a chain: 1px solid, same bounds rule.
  - **Tag, placement, many-and-small, off-screen, hide-while-dragging, draw order:** unchanged. Words "Clone", "Original", "Root", "Original (12 clones)". A path clone keeps its outline trace.
  - An image clone's box equals its outline, so the trace is the frame rectangle.
- **Clone on a laser page.** Marked the same; nothing says "Not assigned" yet (`0101`). A clone drawn on the laser page looks like what it draws, in the stacking place of its matrix; the marks exist only while it is selected.
- **Style tab block.** The existing clone block (`role="group"` named "Linked clone", `Link2`, row 1) with the names "Linked clone of Group 3f9a1c.", "Linked clone of Text 3f9a1c.", "Linked clone of Image 3f9a1c." Select original, Unlink clone, chain rows and the origin line as built. Unlink tooltip by root kind: group "Turns the clone into an ordinary group. Content and style stay."; text "... ordinary text. Text, place and style stay."; image "... ordinary image. Picture and place stay."; a mixed selection "Turns each clone into an ordinary object. Shape and style stay."
- **Override rows.** **Placement** (Follows original / Own) is in the tree for every clone. **Style** (Original's / Own), the greyed Stroke, Fill and Markers rows with their muted summaries, and **Spread colours** are in the tree only while every selected clone is a path clone (Proposal: a clone of a group draws its leaves in their own styles, so a summary or a pill would have nothing to say). Mixed selection with a group clone: the three are absent, nothing explains why. A style edit with such a clone selected writes to the others only; the status notice of `0049` ("2 clones show their original's style and were not changed.") names the clones left alone, group clones counted in.
- **Node tool.** No nodes, with the sentence of `0049` criterion 26 as built.
- **Layers tab.** A group clone is one row with `Link2`, not expandable (its leaves are the original's). The laser page's own Layers tree is `0106`.
- **Unlink clone and delete of the original (criteria 10, 11).** The results are independent copies; they keep their place in the stacking order and stay selected after Unlink. Deleting the original gives no success notice (the 2026-10-31 rule; the existing text-clone notice of `0080` stays as built). Focus after Unlink: as built (first control of the Style area).

## 7. The refused cycle move (criterion 12)

- **From a pointer drag in the Layers tab.** While the dragged rows include a clone of G (or a group holding one), a drop target inside G or inside a group within G shows no insertion cue and the cursor is `not-allowed` (the pointer state, no text). A release there writes nothing and shows the refusal alert (8s) in the canvas notice slot, because a drag has no button to anchor to: "A clone cannot go inside its own original."
- **Other commands** (any command that moves nodes into a group): the same text, in the notice place of that command (4px under a panel button, the canvas notice slot for a key). Nothing changes, no history step, the selection is kept.
- **Refusal outline.** The clone and G are not outlined (no geometry is wrong); the sentence is the whole message.
- **A file with such a clone inside its original.** Opens without a notice of its own; the clone is repaired as an orphan by the existing `0049` criterion 35 notice.

## 8. Keyboard and focus

| Action | Route | Notes |
|---|---|---|
| Place a drawing page on a laser page | Document tab, **Place page**: Enter or Space opens, Up, Down, Home, End, Enter chooses, Escape closes | section 4; the only key route |
| Cancel a canvas or row drag | Escape | sections 2, 3 |
| Make a linked clone of the selection | the Select bar **Linked clone** button (Tab to it) | clone lands directly above the original |
| Put a clone on the laser page by key | Linked clone, then the typed X and Y or the arrow keys (`0019`) | the clone becomes an own item of the laser page by position (155 A). See the report: a gap |
| Move a placed page | arrow keys, typed X/Y/W/H/angle, align | section 5; fully-off result is refused |
| Duplicate a placed page | Ctrl+D | as for objects |
| Delete a placed page | Delete | reference only |

- **Focus after:** a canvas drop, Place page, a nudge, a refusal: the canvas. Unlink clone: first control of the Style area (as built). Escape from the Place page list: the button.
- **Tab order of the Document tab (laser):** page list, Unit, Machine block, **Place page**, Fixtures, Background.
- **No new global chord.** Modifier meanings during the canvas drag (Alt, Ctrl) are the pointer-only route; a keyboard user has the rows above.
- **Reduced motion:** nothing animates (badge, ghost, snap-back are instant).

## 9. Empty, error and large states

- **No laser page:** Linked clone drag has no target; the ordinary move applies; no badge.
- **No drawing page:** Place page hidden; page-row drag has no source.
- **Empty drawing page:** can be placed; ghost is the frame only; the placed frame and label show.
- **Laser page larger than the view / far away:** no auto-pan in a drag (out of scope); Place page pans to the placed result; a drag target off screen is reached with Place page or by moving the view first.
- **Large documents:** ghost outlines are the only per-move cost in a drag (criterion 22); the clones themselves draw within the budgets of `0049`.
- **Dark theme:** tokens only (`--drag-ghost`, `--toolbar-bg`, `--toolbar-icon`, `--editor-accent`, `--clone-link`).
- **Contrast:** the 65 % placed frame is 2.7:1 on `--canvas-bg` (informational, like the hover box; the name tag at 8.3:1 carries the meaning); the 40 % ghost must stay at least 3:1 on `--canvas-bg` as in `0099` (check; else 50 %).

## 10. Review checklist for the implementer

Cloning look switches per frame at the laser page edge and back; Alt and Ctrl change badge and ghost with the pointer at rest; a release with a missed key-up follows the pointer-up flags; Alt tap does not open a menu; the circle-plus badge is replaced, not doubled, over a laser page and returns off it; Shift lock badge, Drop badge and readout chip never overlap; frame cue only for non-refusals; the originals draw unchanged during a cloning drag; Escape leaves no history step; guide-only selection shows nothing; focus on the canvas after every drop and placement; Place page list has drawing pages only, scrolls at 7 or more rows, Escape returns focus to the button; placed page hit anywhere in its frame, name tag upright at 90 degrees and never over the rail; fully-off drag shows 40 % and `not-allowed`, the release snaps back with one notice and no history step; typed and arrow routes refuse alike; group clone marks are bounds not leaves; Style row, greyed rows and Spread colours absent for group, text and image clones; the Layers drag into G shows `not-allowed` and the notice; contract test on Rust-produced payloads for every badge kind (architecture review finding 3); contrast of the 65 % frame and the 40 % ghost on both canvases; screen reader reads the clone, the placed page and each refusal.
